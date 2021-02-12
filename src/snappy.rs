use snap::raw::{Decoder, Encoder};
use snap::read::{FrameDecoder, FrameEncoder};
use std::io::{Error, Read};

/// Decompress snappy data raw
pub fn decompress_raw(data: &[u8]) -> Result<Vec<u8>, snap::Error> {
    let mut decoder = Decoder::new();
    decoder.decompress_vec(data)
}

/// Compress snappy data raw
pub fn compress_raw(data: &[u8]) -> Result<Vec<u8>, snap::Error> {
    let mut encoder = Encoder::new();
    encoder.compress_vec(data)
}

/// Decompress snappy data framed
pub fn decompress(data: &[u8], output: &mut [u8]) -> Result<(), Error> {
    let mut decoder = FrameDecoder::new(data);
    decoder.read(output)?;
    Ok(())
}

/// Decompress snappy data framed
pub fn compress(data: &[u8], output: &mut [u8]) -> Result<(), Error> {
    let mut encoder = FrameEncoder::new(data);
    encoder.read(output)?;
    Ok(())
}
