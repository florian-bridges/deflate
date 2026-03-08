use std::io::{Result, BufReader, Error, ErrorKind, Read};
use std::io::{BufWriter};
use std::fs::File;
use crc32fast::Hasher;
use std::collections::HashMap;
use std::mem::ManuallyDrop;
use std::cmp::{min, max};

use crate::bitstream::BitStream;
use crate::block_type_0::write_block_type_0;
use crate::block_type_1::write_block_type_1;

const USED_BLOCK_TYPE: u32 = 1;

const MAX_REF_DISTANCE: usize = 32768;
const MAX_REF_LEN: usize = 258;

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


pub fn encode_byte_stream(
    in_stream: &mut BufReader<File>, 
    out_stream: &mut BitStream, 
    num_unparsed_bytes: u64,
    hasher: &mut Hasher, 
    ll_codes: &[u32],
    ll_lens: &[u32],
    dist_codes: &[u32],
    dist_lens: &[u32],
) -> Result<()> {

    // todo change for dynamic block sizes
    let block_size = num_unparsed_bytes; 

    //payload
    let mut parsed_bytes = 0; 
    let mut byte_buffer = vec![0u8; MAX_REF_DISTANCE + MAX_REF_LEN];
    
    let mut buffer_idx = 0;
    let mut buffer_max = in_stream.read(&mut byte_buffer)?; 

    while parsed_bytes < block_size {

        while buffer_idx < buffer_max {

            let byte = byte_buffer[buffer_idx];
            BitStream::append_reverse(out_stream, ll_codes[byte as usize], ll_lens[byte as usize])?;

            println!(
                "hex: 0x{:02X} | dec: {:3} | char: {}",
                byte,
                byte,
                byte as char
            );

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


fn build_gzip_footer(out_stream: &mut BitStream, checksum: u32, file_size: u64) -> Result<()>{

    let isize_size = (file_size % (u32::MAX as u64)) as u32;

    BitStream::append_numerical(out_stream,  checksum, 4)?;
    BitStream::append_numerical(out_stream,  isize_size, 4)?;
    BitStream::flush(out_stream)?;
    Ok(())
    
}

pub fn deflate(in_file_path: &String, out_file_path: &String) -> Result<()>{
    
    let in_file = File::open(in_file_path)?;
    let out_file = File::create(out_file_path)?;

    // input file size; 
    let in_file_size = in_file.metadata().unwrap().len();
    if in_file_size > u32::MAX as u64 {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "file too large (must fit into u32)",
        ));
    }
    let in_file_size = in_file_size; 

    //prepare streams
    let mut in_stream = BufReader::new(in_file);    
    let mut out_stream = BitStream::new(BufWriter::new(out_file));

    build_gzip_header(&mut out_stream)?;

    let mut num_unparsed_bytes = in_file_size; 
    let mut hasher = Hasher::new();

    while  num_unparsed_bytes > 0 {

        let mut block_size = 0; 

        match USED_BLOCK_TYPE {
            // block 0
            0 => {
                block_size = write_block_type_0(&mut in_stream, &mut out_stream, num_unparsed_bytes, &mut hasher)?; 
            }
            1 => {
                block_size = write_block_type_1(&mut in_stream, &mut out_stream, num_unparsed_bytes, &mut hasher)?; 
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
