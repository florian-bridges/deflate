use std::io::{BufWriter, Write, Result};
use std::fs::File;

pub struct BitStream {
    current_byte: u8,
    bit_pos: u8,
    writer: BufWriter<File>,
}

impl BitStream {
    pub fn new(writer: BufWriter<File>) -> BitStream {
        BitStream { current_byte: 0, bit_pos: 0, writer: writer}
    }

    fn append_bit(bitstream: &mut BitStream, new_bit: u8) -> Result<()> {
        if bitstream.bit_pos == 8 {
            BitStream::flush(bitstream)?;
        }

        bitstream.current_byte |= (new_bit & 1u8) << bitstream.bit_pos;
        bitstream.bit_pos += 1; 

        Ok(())
    }

    pub fn append(bitstream: &mut BitStream, mut bits: u32, num_bits: u32) -> Result<()>{
                
        for _ in 0..num_bits {
            BitStream::append_bit(bitstream, bits as u8 & 1u8)?;
            bits >>= 1; 
        }

        

        Ok(())
    }

    pub fn append_reverse(bitstream: &mut BitStream, bits: u32, num_bits: u32) -> Result<()>{

        fn reverse_bits(mut bits: u32, num_bits: u32) -> u32{
            let mut reversed_bits = 0u32;

            for _ in 0..num_bits {
                reversed_bits <<= 1;
                reversed_bits |= bits & 1u32;
                bits >>=1;
            }

            reversed_bits

        }

        let reversed_bits = reverse_bits(bits, num_bits);

        BitStream::append(bitstream, reversed_bits, num_bits)?;

        Ok(())

    }

    pub fn append_numerical(bitstream: &mut BitStream, value: u32, num_bytes: u32) -> Result<()> {
        for i in 0..num_bytes {
            BitStream::append(bitstream, (value >> (8 * i)) & 0xFF, 8)?;
        }
        
        Ok(())
    }

    pub fn flush(bitstream: &mut BitStream) -> Result<()>{

        if bitstream.bit_pos == 0 {
            return Ok(())
        }
        
        bitstream.writer.write(&[bitstream.current_byte])?;
        bitstream.current_byte = 0; 
        bitstream.bit_pos = 0; 
        Ok(())
    }

}

