//! brotli de/compression interface
use std::io::Write;

const DEFAULT_COMPRESSION_LEVEL: u32 = 11;
const BUF_SIZE: usize = 1 << 17; // Taken from brotli kCompressFragementTwoPassBlockSize
const LGWIN: u32 = 22;

pub use brotli;
use std::io::prelude::*;
use std::io::Error;

/// Decompress via Brotli
#[inline(always)]
pub fn decompress<W: Write + ?Sized, R: Read>(input: R, output: &mut W) -> Result<usize, Error> {
    let mut decoder = brotli::Decompressor::new(input, BUF_SIZE);
    let n_bytes = std::io::copy(&mut decoder, output)?;
    Ok(n_bytes as usize)
}

/// Compress via Brotli
#[inline(always)]
pub fn compress<W: Write + ?Sized, R: Read>(input: R, output: &mut W, level: Option<u32>) -> Result<usize, Error> {
    let level = level.unwrap_or_else(|| DEFAULT_COMPRESSION_LEVEL);
    let mut encoder = brotli::CompressorReader::new(input, BUF_SIZE, level, LGWIN);
    let n_bytes = std::io::copy(&mut encoder, output)?;
    Ok(n_bytes as usize)
}
