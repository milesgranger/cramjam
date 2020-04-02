use snap::raw::{Decoder, Encoder};
use snap::read::{FrameDecoder, FrameEncoder};
use std::io::Read;

/// Decompress snappy data raw
pub fn decompress_raw(data: &[u8]) -> Vec<u8> {
    let mut decoder = Decoder::new();
    decoder.decompress_vec(data).unwrap()
}

/// Compress snappy data raw
pub fn compress_raw(data: &[u8]) -> Vec<u8> {
    let mut encoder = Encoder::new();
    encoder.compress_vec(data).unwrap()
}

/// Decompress snappy data framed
pub fn decompress(data: &[u8]) -> Vec<u8> {
    let mut buf = vec![];
    let mut decoder = FrameDecoder::new(data);
    decoder.read_to_end(&mut buf).unwrap();
    buf
}

/// Decompress snappy data framed
pub fn compress(data: &[u8]) -> Vec<u8> {
    let mut buf = vec![];
    let mut encoder = FrameEncoder::new(data);
    encoder.read_to_end(&mut buf).unwrap();
    buf
}
