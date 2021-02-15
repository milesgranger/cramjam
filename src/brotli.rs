use crate::Output;
use brotli2::read::{BrotliDecoder, BrotliEncoder};
use std::io::prelude::*;
use std::io::Error;

/// Decompress via Brotli
pub fn decompress<'a>(data: &[u8], output: Output<'a>) -> Result<usize, Error> {
    let mut decoder = BrotliDecoder::new(data);
    match output {
        Output::Slice(slice) => decoder.read(slice),
        Output::Vector(v) => decoder.read_to_end(v),
    }
}

/// Compress via Brotli
pub fn compress<'a>(data: &'a [u8], output: Output<'a>, level: u32) -> Result<usize, Error> {
    let mut encoder = BrotliEncoder::new(data, level);
    match output {
        Output::Slice(slice) => encoder.read(slice),
        Output::Vector(v) => encoder.read_to_end(v),
    }
}
