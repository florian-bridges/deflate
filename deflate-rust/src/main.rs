use std::env;
mod bitstream;
mod block_type_0;
mod block_type_1;

mod deflate;
use deflate::{deflate}; 

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();

    let in_file_path = &args[1];
    let out_file_path = &args[2];

    deflate(in_file_path, out_file_path)?;

    Ok(())

    
}

