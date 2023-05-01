//! snappy de/compression interface
pub use snap;
use snap::read::{FrameDecoder, FrameEncoder};
use std::io::{Error, Read, Write};

/// Decompress snappy data framed
#[inline(always)]
pub fn decompress<W: Write + ?Sized, R: Read>(input: R, output: &mut W) -> Result<usize, Error> {
    let mut decoder = FrameDecoder::new(input);
    let n_bytes = std::io::copy(&mut decoder, output)?;
    Ok(n_bytes as usize)
}

/// Decompress snappy data framed
#[inline(always)]
pub fn compress<W: Write + ?Sized, R: Read>(data: R, output: &mut W) -> Result<usize, Error> {
    let mut encoder = FrameEncoder::new(data);
    let n_bytes = std::io::copy(&mut encoder, output)?;
    Ok(n_bytes as usize)
}
