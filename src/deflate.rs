use crate::Output;
use flate2::read::{DeflateDecoder, DeflateEncoder};
use flate2::Compression;
use std::io::prelude::*;
use std::io::Error;

/// Decompress gzip data
pub fn decompress<'a>(data: &[u8], output: Output<'a>) -> Result<usize, Error> {
    let mut decoder = DeflateDecoder::new(data);
    match output {
        Output::Slice(slice) => decoder.read(slice),
        Output::Vector(v) => decoder.read_to_end(v),
    }
}

/// Compress gzip data
pub fn compress<'a>(data: &'a [u8], output: Output<'a>, level: u32) -> Result<usize, Error> {
    let mut encoder = DeflateEncoder::new(data, Compression::new(level));
    match output {
        Output::Slice(slice) => encoder.read(slice),
        Output::Vector(v) => encoder.read_to_end(v),
    }
}
