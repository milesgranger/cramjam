use brotli2::read::{BrotliDecoder, BrotliEncoder};
use std::error::Error;
use std::io::prelude::*;

/// Decompress via Brotli
pub fn decompress(data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut decoder = BrotliDecoder::new(data);
    let mut buf = vec![];
    decoder.read_to_end(&mut buf)?;
    Ok(buf)
}

/// Compress via Brotli
pub fn compress(data: &[u8], level: u32) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut encoder = BrotliEncoder::new(data, level);
    let mut buf = vec![];
    encoder.read_to_end(&mut buf)?;
    Ok(buf)
}
