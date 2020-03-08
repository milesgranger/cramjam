use snap::raw::{Decoder, Encoder};

/// Decompress snappy data
pub fn decompress(data: &[u8]) -> Vec<u8> {
    let mut decoder = Decoder::new();
    decoder.decompress_vec(data).unwrap()
}

/// Compress snappy data
pub fn compress(data: &[u8]) -> Vec<u8> {
    let mut encoder = Encoder::new();
    encoder.compress_vec(data).unwrap()
}
