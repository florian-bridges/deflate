use std::io::{Result, BufReader};
use std::fs::File;
use std::io::Read;
use crc32fast::Hasher;
use std::cmp::min;

use crate::bitstream::BitStream;

const BLOCK_0_MAX_SIZE: usize = u16::MAX as usize;

pub fn write_block_type_0(in_stream: &mut BufReader<File>, out_stream: &mut BitStream, num_unparsed_bytes: usize, hasher: &mut Hasher) -> Result<usize> {

    //header
    let block_type = 0; 
    
    let block_size = min(num_unparsed_bytes, BLOCK_0_MAX_SIZE);
    
    let is_last = block_size == num_unparsed_bytes;     
    BitStream::append(out_stream,  if is_last {1u32} else {0u32}, 1)?;
    BitStream::append(out_stream, block_type, 2)?;
    BitStream::append(out_stream, 0, 5)?;

    BitStream::append_numerical(out_stream, block_size as u32, 2)?;
    BitStream::append_numerical(out_stream, !(block_size as u32), 2)?;

    //payload
    let mut parsed_bytes = 0; 
    for byte in in_stream.bytes(){
            
            let byte = byte?;
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
    BitStream::flush(out_stream)?;

    Ok(block_size)
}

