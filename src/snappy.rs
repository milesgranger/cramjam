use snap::raw::{Decoder, Encoder};
use snap::read::{FrameDecoder, FrameEncoder};
use std::io::{Error, Read};

use crate::Output;

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
pub fn decompress<'a>(data: &'a [u8], output: Output<'a>) -> Result<usize, Error> {
    let mut decoder = FrameDecoder::new(data);
    match output {
        Output::Slice(slice) => decoder.read(slice),
        Output::Vector(v) => decoder.read_to_end(v),
    }
}

/// Decompress snappy data framed
pub fn compress<'a>(data: &'a [u8], output: Output<'a>) -> Result<usize, Error> {
    let mut encoder = FrameEncoder::new(data);
    match output {
        Output::Slice(slice) => encoder.read(slice),
        Output::Vector(v) => encoder.read_to_end(v),
    }
}
