use std::io::{Result, BufReader, Error, ErrorKind, Read};
use std::io::{BufWriter};
use std::fs::File;
use std::usize;
use crc32fast::Hasher;
use std::collections::HashMap;
use std::cmp::max;

use crate::bitstream::BitStream;
use crate::block_type_0::write_block_type_0;
use crate::block_type_1::write_block_type_1;
use crate::block_type_2::write_block_type_2;

const USED_BLOCK_TYPE: u32 = 2;

const MAX_REF_DISTANCE: usize = 32768;
const MAX_REF_LEN: usize = 258;


// Compact representation of the len code value (257-285), len range and number
// of extra bits to use in LZ77 compression (See Section 3.2.5 of RFC 1951)
const LENGTH_CODE_RANGES: [(usize, u32, u32, u32); 29] = [ // (code, num bits, lower bound, upper bound)
    (257, 0,   3,   3),   (258, 0,   4,   4),   (259, 0,   5,   5),   (260, 0,   6,   6),   (261, 0,   7,   7),
    (262, 0,   8,   8),   (263, 0,   9,   9),   (264, 0,  10,  10),   (265, 1,  11,  12),   (266, 1,  13,  14),
    (267, 1,  15,  16),   (268, 1,  17,  18),   (269, 2,  19,  22),   (270, 2,  23,  26),   (271, 2,  27,  30),
    (272, 2,  31,  34),   (273, 3,  35,  42),   (274, 3,  43,  50),   (275, 3,  51,  58),   (276, 3,  59,  66),
    (277, 4,  67,  82),   (278, 4,  83,  98),   (279, 4,  99, 114),   (280, 4, 115, 130),   (281, 5, 131, 162),
    (282, 5, 163, 194),   (283, 5, 195, 226),   (284, 5, 227, 257),   (285, 0, 258, 258),
];

// Compact representation of the distance code value (0-31), distance range and number
// of extra bits to use in LZ77 compression (See Section 3.2.5 of RFC 1951)
const DISTANCE_CODE_RANGES: [(usize, u32, u32, u32); 30] = [ // (code, num bits, lower bound, upper bound)
    (0,0,1,1),         (1,0,2,2),          (2,0,3,3),           (3,0,4,4),           (4,1,5,6),
    (5,1,7,8),         (6,2,9,12),         (7,2,13,16),         (8,3,17,24),         (9,3,25,32),
    (10,4,33,48),      (11,4,49,64),       (12,5,65,96),        (13,5,97,128),       (14,6,129,192),
    (15,6,193,256),    (16,7,257,384),     (17,7,385,512),      (18,8,513,768),      (19,8,769,1024),
    (20,9,1025,1536),  (21,9,1537,2048),   (22,10,2049,3072),   (23,10,3073,4096),   (24,11,4097,6144),
    (25,11,6145,8192), (26,12,8193,12288), (27,12,12289,16384), (28,13,16385,24576), (29,13,24577,32768),
];

fn get_length_codes() -> [(usize, u32, u32); MAX_REF_LEN + 1] {
    let mut length_codes = [(0, 0, 0); MAX_REF_LEN + 1];
    for &(code, num_bits, lower, upper) in &LENGTH_CODE_RANGES {
        for i in 0..(upper-lower +1) as usize {
            length_codes[lower as usize + i ] = (code, num_bits, i as u32);
        }
    }

    return length_codes; 
}

fn get_distance_codes() -> [(usize, u32, u32); MAX_REF_DISTANCE + 1] {
    let mut distance_codes = [(0, 0, 0); MAX_REF_DISTANCE + 1];
    for &(code, num_bits, lower, upper) in &DISTANCE_CODE_RANGES {
        for i in 0..(upper-lower +1) as usize {
            distance_codes[lower as usize + i ] = (code, num_bits, i as u32);
        }
    }

    return distance_codes; 
}

// generic helper functions
pub fn get_prefix_codes(code_lens: &[u32], num_codes: usize) -> Vec<u32> {
    let mut bl_count : HashMap<u32, u32> = HashMap::new();
    let mut max_num_bits = 0; 

    for num_bits in code_lens {
        *bl_count.entry(*num_bits).or_insert(0) += 1;
        max_num_bits = max(max_num_bits, *num_bits); 
    }

    let mut code = 0; 
    let mut next_code: HashMap<u32, u32> = HashMap::new();

    for bits in 1..=max_num_bits {
        code += bl_count.get(&(bits - 1)).unwrap_or(&0);
        code <<= 1; 
        next_code.insert(bits, code); 
    }

    let mut prefix_codes = vec![0u32; num_codes];
    for (label, &tree_len) in code_lens.iter().enumerate() {
        if tree_len == 0 {
            continue;
        }
        prefix_codes[label] =  *next_code.get(&tree_len).unwrap();
        *next_code.entry(tree_len).or_insert(0) += 1;
    }

    return prefix_codes; 
}

fn find_reference(byte_buffer: &Vec<u8>, buffer_idx: usize) -> (usize, usize) {

    let mut ref_dist: usize = 3; 
    let mut ref_len: usize = 0;

    loop {

        if ref_dist > buffer_idx || ref_dist >= MAX_REF_DISTANCE {
            break;
        }

        ref_len = 0;

        while byte_buffer[buffer_idx + ref_len] == byte_buffer[buffer_idx - ref_dist + ref_len] {
            ref_len+= 1;
            if buffer_idx + ref_len >= byte_buffer.len() || ref_len >= MAX_REF_LEN {
                break;
            }
             
        }

        if ref_len >= 3 {
            break;
        }

        ref_dist += 1; 
        
    } 

    return (ref_dist,ref_len);
}


pub fn encode_byte_stream(
    in_stream: &mut BufReader<File>, 
    out_stream: &mut BitStream, 
    num_unparsed_bytes: usize,
    hasher: &mut Hasher, 
    ll_codes: &[u32],
    ll_lens: &[u32],
    dist_codes: &[u32],
    dist_lens: &[u32],
) -> Result<()> {

    // todo change for dynamic block sizes
    let block_size = num_unparsed_bytes; 

    //payload
    let mut parsed_bytes= 0; 
    let mut byte_buffer = vec![0u8; MAX_REF_DISTANCE + MAX_REF_LEN];
    
    let mut buffer_idx = 0;
    let mut buffer_max = in_stream.read(&mut byte_buffer)?; 

    let length_codes_table: [(usize, u32, u32); MAX_REF_LEN + 1] = get_length_codes(); 
    let dist_codes_table: [(usize, u32, u32); MAX_REF_DISTANCE + 1] = get_distance_codes(); 

    while parsed_bytes < block_size {

        while buffer_idx < buffer_max {

            let byte = byte_buffer[buffer_idx];

            let (ref_dist, ref_len) = find_reference(&byte_buffer, buffer_idx); 

            if ref_len >= 3 {

                let (len_label,  len_num_bits, len_offset) = length_codes_table[ref_len];
                let (dist_label,  dist_num_bits, dist_offset) = dist_codes_table[ref_dist];

                let total_ref_size = ll_lens[ref_len] + len_num_bits as u32 + dist_lens[ref_dist];

                if 8 * ref_len as u32 > total_ref_size {

                    //println!(
                    //    "ref_dist: {:3} | ref_len: {:3}",
                    //    ref_dist,
                    //    ref_len
                    //);

                    BitStream::append_reverse(out_stream, ll_codes[len_label], ll_lens[len_label])?;
                    BitStream::append(out_stream, len_offset, len_num_bits)?;

                    BitStream::append_reverse(out_stream, dist_codes[dist_label], dist_lens[dist_label])?;
                    BitStream::append(out_stream, dist_offset, dist_num_bits)?;

                    parsed_bytes += ref_len; 
                    buffer_idx += ref_len; 
                    continue;

                } 
            }

            BitStream::append_reverse(out_stream, ll_codes[byte as usize], ll_lens[byte as usize])?;

            //println!(
            //    "hex: 0x{:02X} | dec: {:3} | char: {}",
            //    byte,
            //    byte,
            //    byte as char
            //);

            buffer_idx += 1; 
            parsed_bytes += 1; 
        }

        hasher.update(&byte_buffer[0..buffer_idx]);

        byte_buffer.copy_within(buffer_idx..buffer_max, 0);
        buffer_max -= buffer_idx;
        buffer_idx = 0;

        let bytes_read = in_stream.read(&mut byte_buffer[buffer_max..])?;
        buffer_max += bytes_read;

    }

    Ok(())
}

fn build_gzip_header(bitstream: &mut BitStream) -> Result<()>{

    // header definition: https://www.ietf.org/rfc/rfc1952.txt, chapter: 2.3.1. Member header and trailer
    
    // header id1
    BitStream::append_numerical(bitstream, 0x1f, 1)?;
    // header id2
    BitStream::append_numerical(bitstream, 0x8b, 1)?;
    // header cm deflate
    BitStream::append_numerical(bitstream, 0x08, 1)?;
    // configuration flags
    BitStream::append(bitstream, 0x00, 8)?; 
    // header mtime
    BitStream::append_numerical(bitstream, 0x00, 4)?;
    //header xfl
    BitStream::append_numerical(bitstream, 0x00, 1)?;
    // header os = linux
    BitStream::append_numerical(bitstream, 0x03, 1)?;

    Ok(())
}


fn build_gzip_footer(out_stream: &mut BitStream, checksum: u32, file_size: usize) -> Result<()>{

    let isize_size = (file_size % (u32::MAX as usize)) as u32;

    BitStream::append_numerical(out_stream,  checksum, 4)?;
    BitStream::append_numerical(out_stream,  isize_size, 4)?;
    BitStream::flush(out_stream)?;
    Ok(())
    
}

pub fn deflate(in_file_path: &String, out_file_path: &String) -> Result<()>{
    
    let in_file = File::open(in_file_path)?;
    let out_file = File::create(out_file_path)?;

    // input file size; 
    let in_file_size = in_file.metadata().unwrap().len() as usize;
    if in_file_size > u32::MAX as usize {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "file too large (must fit into u32)",
        ));
    }

    //prepare streams
    let mut in_stream = BufReader::new(in_file);    
    let mut out_stream = BitStream::new(BufWriter::new(out_file));

    build_gzip_header(&mut out_stream)?;

    let mut num_unparsed_bytes = in_file_size; 
    let mut hasher = Hasher::new();

    let mut block_size; 
    while  num_unparsed_bytes > 0 {

        match USED_BLOCK_TYPE {
            // block 0
            0 => {
                block_size = write_block_type_0(&mut in_stream, &mut out_stream, num_unparsed_bytes, &mut hasher)?; 
            }
            1 => {
                block_size = write_block_type_1(&mut in_stream, &mut out_stream, num_unparsed_bytes, &mut hasher)?; 
            }
            2 => {
                block_size = write_block_type_2(&mut in_stream, &mut out_stream, num_unparsed_bytes, &mut hasher)?; 
            }
            _ => {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "invalid block type specified.",
                ));
            }
            
        }
        num_unparsed_bytes -= block_size;     

    }
    

    let checksum = hasher.finalize();
    build_gzip_footer(&mut out_stream, checksum, in_file_size)?; 
    
    Ok(())
}
