use std::io::{Result, BufReader, Read};
use std::fs::File;
use std::cmp::{min, max};
use std::collections::HashMap;
use std::mem::ManuallyDrop;
use crc32fast::Hasher;

use crate::bitstream::BitStream;

const MAX_REF_DISTANCE: usize = 32768;
const MAX_REF_LEN: usize = 258;



fn get_code_lens(len_list: &[(u32, u32, u32)], list_length: usize) -> Vec<u32>{

    let mut code_lens = vec![0u32; list_length];

    for (from_label, to_label, num_bits) in len_list {
        for label in *from_label..=*to_label {
            code_lens[label as usize] =  *num_bits;
        }
    }

    return code_lens; 
}

fn get_prefix_codes(code_lens: &[u32], num_codes: usize) -> Vec<u32> {
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

const LL_CODE_LIST_LEN: usize = 288; 
fn get_default_ll_codes() -> ([u32; LL_CODE_LIST_LEN], [u32; LL_CODE_LIST_LEN]){

    const MAX_LEN_LIST: usize = LL_CODE_LIST_LEN; 

    let len_list = [
        (0, 143, 8),
        (144, 255, 9),
        (256, 279, 7),
        (280, 287, 8),
    ];

    let code_lens: [u32; LL_CODE_LIST_LEN] = get_code_lens(&len_list, MAX_LEN_LIST).try_into().unwrap(); 
    let prefix_codes: [u32; LL_CODE_LIST_LEN] = get_prefix_codes(&code_lens, MAX_LEN_LIST).try_into().unwrap(); 
    return (prefix_codes, code_lens); 

}

const DISTANCE_CODE_LIST_LEN: usize = 32769; 
fn get_default_distance_codes() -> ([u32; DISTANCE_CODE_LIST_LEN], [u32; DISTANCE_CODE_LIST_LEN]){

    let len_list = [
        (0, 32768, 5),
    ];

    let code_lens: [u32; DISTANCE_CODE_LIST_LEN] = get_code_lens(&len_list, DISTANCE_CODE_LIST_LEN).try_into().unwrap(); 
    let prefix_codes: [u32; DISTANCE_CODE_LIST_LEN] = get_prefix_codes(&code_lens, DISTANCE_CODE_LIST_LEN).try_into().unwrap(); 
    return (prefix_codes, code_lens); 

}

struct ByteWindow {
    bytes: [u8; MAX_REF_DISTANCE + MAX_REF_LEN],
    byte_idx: usize,
    len: u32,  
}

impl ByteWindow {
    fn new() -> ByteWindow {
        ByteWindow {
            bytes: [0; MAX_REF_DISTANCE + MAX_REF_LEN], 
            byte_idx: 0, 
            len:0,
        }
    }

    fn add(&mut self, new_byte: u8){
        self.bytes[self.byte_idx] = new_byte;

        self.byte_idx = (self.byte_idx + 1) % MAX_REF_DISTANCE;
        self.len = min(self.len + 1, MAX_REF_DISTANCE as u32);  
    }

    fn find(&mut self, find_byte: u8) -> usize {

        let mut find_idx = 0; 
        for idx in 3..(self.len as usize) {
            if self.bytes[(self.byte_idx - idx) % MAX_REF_DISTANCE] != find_byte {
                continue;
            }

            find_idx = idx;

        }

        return  find_idx;
    }
    
}

fn encode_byte_stream(
    in_stream: &mut BufReader<File>, 
    out_stream: &mut BitStream, 
    num_unparsed_bytes: u64,
    hasher: &mut Hasher, 
    ll_codes: HashMap<i32,i32>,
    ll_lens: HashMap<i32,i32>,
    dist_codes: HashMap<i32,i32>,
    dist_lens: HashMap<i32,i32>,
) -> Result<()> {

    let mut bytewindow = ByteWindow::new();
    let block_size = num_unparsed_bytes;
    
    let mut parsed_bytes = 0; 
    for byte in in_stream.bytes(){ 

            let byte = byte?;
            bytewindow.add(byte);


            BitStream::append(out_stream, byte as u32, 8)?;
            hasher.update(&[byte]);
            println!(
                "hex: 0x{:02X} | dec: {:3} | char: {}",
                byte,
                byte,
                byte as char
            );

            parsed_bytes += 1; 

            if parsed_bytes >= block_size {
                break;
            }

        }

    // TODO add block termination label 256
    BitStream::flush(out_stream)?;

    Ok(())
}


pub fn write_block_type_1(in_stream: &mut BufReader<File>, out_stream: &mut BitStream, num_unparsed_bytes: u64, hasher: &mut Hasher) -> Result<u64> {

    // todo change for dynamic block sizes
    let block_size = num_unparsed_bytes; 
    let is_last = true; 

    //header
    let block_type = 1;   
    
    
    BitStream::append(out_stream,  if is_last {1u32} else {0u32}, 1)?;
    BitStream::append(out_stream, block_type, 2)?;

    // get default codes
    let (ll_codes, ll_lens) = get_default_ll_codes();
    let (dist_codes, dist_lens) = get_default_distance_codes();

    //payload
    let mut parsed_bytes = 0; 
    for byte in in_stream.bytes(){
            
            let byte = byte?;

            BitStream::append_reverse(out_stream, ll_codes[byte as usize], ll_lens[byte as usize])?;
            hasher.update(&[byte]);
            println!(
                "hex: 0x{:02X} | dec: {:3} | char: {}",
                byte,
                byte,
                byte as char
            );

            parsed_bytes += 1; 

            if parsed_bytes >= block_size {
                break;
            }

        }
    
    // add block terminatin symbol
    BitStream::append_reverse(out_stream, ll_codes[256 as usize], ll_lens[256 as usize])?;
    BitStream::flush(out_stream)?;
    
    Ok(block_size)
}

