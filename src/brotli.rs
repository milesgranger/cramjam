use brotli2::read::{BrotliDecoder, BrotliEncoder};
use std::io::prelude::*;

/// Decompress via Brotli
pub fn decompress(data: &[u8]) -> Vec<u8> {
    let mut decoder = BrotliDecoder::new(data);
    let mut buf = vec![];
    decoder.read_to_end(&mut buf).unwrap();
    buf
}

/// Compress via Brotli
pub fn compress(data: &[u8], level: u32) -> Vec<u8> {
    let mut encoder = BrotliEncoder::new(data, level);
    let mut buf = vec![];
    encoder.read_to_end(&mut buf).unwrap();
    buf
}
