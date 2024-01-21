//! snappy de/compression interface
pub use lzma;
use lzma::reader::LzmaReader;
use std::io;
use std::io::{Read, Result, Write};

/// Decompress snappy data framed
#[inline(always)]
pub fn decompress<W: Write + ?Sized, R: Read>(input: R, output: &mut W) -> Result<usize> {
    let mut decoder = LzmaReader::new_decompressor(input).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let n_bytes = io::copy(&mut decoder, output)?;
    Ok(n_bytes as usize)
}

/// Decompress snappy data framed
#[inline(always)]
pub fn compress<W: Write + ?Sized, R: Read>(data: R, output: &mut W, preset: Option<u32>) -> Result<usize> {
    let preset = preset.unwrap_or(6); // same as python default
    let mut encoder = LzmaReader::new_compressor(data, preset).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let n_bytes = io::copy(&mut encoder, output)?;
    Ok(n_bytes as usize)
}
