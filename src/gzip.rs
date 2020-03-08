use flate2::read::{GzDecoder, GzEncoder};
use flate2::Compression;
use std::io::prelude::*;

/// Decompress gzip data
pub fn decompress(data: &[u8]) -> Vec<u8> {
    let mut decoder = GzDecoder::new(data);
    let mut buf = vec![];
    decoder.read_to_end(&mut buf).unwrap();
    buf
}

/// Compress gzip data
pub fn compress(data: &[u8], level: u32) -> Vec<u8> {
    let mut buf = vec![];
    let mut encoder = GzEncoder::new(data, Compression::new(level));
    encoder.read_to_end(&mut buf).unwrap();
    buf
}
