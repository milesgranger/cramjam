use flate2::read::{GzDecoder, GzEncoder};
use flate2::Compression;
use std::error::Error;
use std::io::prelude::*;

/// Decompress gzip data
pub fn decompress(data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut decoder = GzDecoder::new(data);
    let mut buf = vec![];
    decoder.read_to_end(&mut buf)?;
    Ok(buf)
}

/// Compress gzip data
pub fn compress(data: &[u8], level: u32) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut buf = vec![];
    let mut encoder = GzEncoder::new(data, Compression::new(level));
    encoder.read_to_end(&mut buf)?;
    Ok(buf)
}
