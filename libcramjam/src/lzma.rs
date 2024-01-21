//! snappy de/compression interface
use std::io;
use std::io::{Read, Result, Write};
pub use xz2;
use xz2::read::{XzDecoder, XzEncoder};

/// Decompress snappy data framed
#[inline(always)]
pub fn decompress<W: Write + ?Sized, R: Read>(input: R, output: &mut W) -> Result<usize> {
    let mut decoder = XzDecoder::new(input);
    let n_bytes = io::copy(&mut decoder, output)?;
    Ok(n_bytes as usize)
}

/// Decompress snappy data framed
#[inline(always)]
pub fn compress<W: Write + ?Sized, R: Read>(data: R, output: &mut W, preset: Option<u32>) -> Result<usize> {
    let preset = preset.unwrap_or(6); // same as python default
    let mut encoder = XzEncoder::new(data, preset);
    let n_bytes = io::copy(&mut encoder, output)?;
    Ok(n_bytes as usize)
}
