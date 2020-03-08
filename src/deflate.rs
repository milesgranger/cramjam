use flate2::read::{DeflateDecoder, DeflateEncoder};
use flate2::Compression;
use std::io::prelude::*;

/// Decompress gzip data
pub fn decompress(data: &[u8]) -> Vec<u8> {
    let mut decoder = DeflateDecoder::new(data);
    let mut buf = vec![];
    decoder.read_to_end(&mut buf).unwrap();
    buf
}

/// Compress gzip data
pub fn compress(data: &[u8], level: u32) -> Vec<u8> {
    let mut buf = vec![];
    let mut encoder = DeflateEncoder::new(data, Compression::new(level));
    encoder.read_to_end(&mut buf).unwrap();
    buf
}
