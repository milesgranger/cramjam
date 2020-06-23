use flate2::read::{DeflateDecoder, DeflateEncoder};
use flate2::Compression;
use std::error::Error;
use std::io::prelude::*;

/// Decompress gzip data
pub fn decompress(data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut decoder = DeflateDecoder::new(data);
    let mut buf = vec![];
    decoder.read_to_end(&mut buf)?;
    Ok(buf)
}

/// Compress gzip data
pub fn compress(data: &[u8], level: u32) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut buf = vec![];
    let mut encoder = DeflateEncoder::new(data, Compression::new(level));
    encoder.read_to_end(&mut buf)?;
    Ok(buf)
}
