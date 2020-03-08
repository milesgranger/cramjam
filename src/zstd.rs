
/// Decompress gzip data
pub fn decompress(data: &[u8]) -> Vec<u8> {
    zstd::stream::decode_all(data).unwrap()
}

/// Compress gzip data
pub fn compress(data: &[u8], level: i32) -> Vec<u8> {
    zstd::stream::encode_all(data, level).unwrap()
}
