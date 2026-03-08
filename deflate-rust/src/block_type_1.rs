use std::io::{Result, BufReader, Read};
use std::fs::File;
use std::cmp::{min, max};
use crc32fast::Hasher;

use crate::bitstream::BitStream;

use crate::deflate::{get_prefix_codes, encode_byte_stream};


fn get_code_lens(len_list: &[(u32, u32, u32)], list_length: usize) -> Vec<u32>{

    let mut code_lens = vec![0u32; list_length];

    for (from_label, to_label, num_bits) in len_list {
        for label in *from_label..=*to_label {
            code_lens[label as usize] =  *num_bits;
        }
    }

    return code_lens; 
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
    encode_byte_stream(in_stream, out_stream, num_unparsed_bytes, hasher, &ll_codes, &ll_lens, &dist_codes, &dist_lens)?;
    
    // add block terminatin symbol
    BitStream::append_reverse(out_stream, ll_codes[256 as usize], ll_lens[256 as usize])?;
    BitStream::flush(out_stream)?;
    
    Ok(block_size)
}

