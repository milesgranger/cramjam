//! snappy de/compression interface
pub use snap;
use snap::read::{FrameDecoder, FrameEncoder};
use std::io;
use std::io::{Read, Result, Write};

/// Decompress snappy data framed
#[inline(always)]
pub fn decompress<W: Write + ?Sized, R: Read>(input: R, output: &mut W) -> Result<usize> {
    let mut decoder = FrameDecoder::new(input);
    let n_bytes = io::copy(&mut decoder, output)?;
    Ok(n_bytes as usize)
}

/// Decompress snappy data framed
#[inline(always)]
pub fn compress<W: Write + ?Sized, R: Read>(data: R, output: &mut W) -> Result<usize> {
    let mut encoder = FrameEncoder::new(data);
    let n_bytes = io::copy(&mut encoder, output)?;
    Ok(n_bytes as usize)
}

pub mod raw {
    use super::*;

    #[inline(always)]
    pub fn compress_vec(input: &[u8]) -> Result<Vec<u8>> {
        snap::raw::Encoder::new()
            .compress_vec(input)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
    }

    #[inline(always)]
    pub fn compress(input: &[u8], output: &mut [u8]) -> Result<usize> {
        snap::raw::Encoder::new()
            .compress(input, output)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
    }

    #[inline(always)]
    pub fn decompress_vec(input: &[u8]) -> Result<Vec<u8>> {
        snap::raw::Decoder::new()
            .decompress_vec(input)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
    }

    #[inline(always)]
    pub fn decompress(input: &[u8], output: &mut [u8]) -> Result<usize> {
        snap::raw::Decoder::new()
            .decompress(input, output)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
    }
}
