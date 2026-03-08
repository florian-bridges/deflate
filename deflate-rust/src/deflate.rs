use std::io::{Result, BufReader, Error, ErrorKind};
use std::io::{BufWriter};
use std::fs::File;
use crc32fast::Hasher;

use crate::bitstream::BitStream;
use crate::block_type_0::write_block_type_0;
use crate::block_type_1::write_block_type_1;

const USED_BLOCK_TYPE: u32 = 1;


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
