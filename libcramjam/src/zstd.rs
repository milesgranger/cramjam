//! zstd de/compression interface
use std::io::{Error, Read, Write};
pub use zstd;

const DEFAULT_COMPRESSION_LEVEL: i32 = 0;

/// Decompress gzip data
#[inline(always)]
pub fn decompress<W: Write + ?Sized, R: Read>(input: R, output: &mut W) -> Result<usize, Error> {
    let mut decoder = zstd::stream::read::Decoder::new(input)?;
    let n_bytes = std::io::copy(&mut decoder, output)?;
    Ok(n_bytes as usize)
}

/// Compress gzip data
#[inline(always)]
pub fn compress<W: Write + ?Sized, R: Read>(input: R, output: &mut W, level: Option<i32>) -> Result<usize, Error> {
    let level = level.unwrap_or_else(|| DEFAULT_COMPRESSION_LEVEL); // 0 will use zstd's default, currently 3
    let mut encoder = zstd::stream::read::Encoder::new(input, level)?;
    let n_bytes = std::io::copy(&mut encoder, output)?;
    Ok(n_bytes as usize)
}
