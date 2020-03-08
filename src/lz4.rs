/// Decompress lz4 data
pub fn decompress(data: &[u8]) -> Vec<u8> {
    lz4_compress::decompress(data).unwrap()
}

/// Compress lz4 data
pub fn compress(data: &[u8]) -> Vec<u8> {
    lz4_compress::compress(data)
}
