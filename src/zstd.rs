use std::io::Error;

/// Decompress gzip data
pub fn decompress(data: &[u8]) -> Result<Vec<u8>, Error> {
    zstd::stream::decode_all(data)
}

/// Compress gzip data
pub fn compress(data: &[u8], level: i32) -> Result<Vec<u8>, Error> {
    zstd::stream::encode_all(data, level)
}
