//! bzip2 de/compression interface
use std::io::prelude::*;
use std::io::Error;

const DEFAULT_COMPRESSION_LEVEL: u32 = 6;

pub use bzip2;
use bzip2::read::{BzEncoder, MultiBzDecoder};

/// Decompress via bzip2
#[inline(always)]
pub fn decompress<W: Write + ?Sized, R: Read>(input: R, output: &mut W) -> Result<usize, Error> {
    let mut decoder = MultiBzDecoder::new(input);
    let n_bytes = std::io::copy(&mut decoder, output)?;
    Ok(n_bytes as usize)
}

/// Compress via bzip2
#[inline(always)]
pub fn compress<W: Write + ?Sized, R: Read>(input: R, output: &mut W, level: Option<u32>) -> Result<usize, Error> {
    let level = level.unwrap_or_else(|| DEFAULT_COMPRESSION_LEVEL);
    let mut encoder = BzEncoder::new(input, bzip2::Compression::new(level));
    let n_bytes = std::io::copy(&mut encoder, output)?;
    Ok(n_bytes as usize)
}
