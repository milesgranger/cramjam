use crate::Output;
use std::io::{Error, Read};

/// Decompress gzip data
pub fn decompress<'a>(data: &'a [u8], output: Output<'a>) -> Result<usize, Error> {
    let mut decoder = zstd::stream::read::Decoder::new(data)?;
    match output {
        Output::Slice(slice) => decoder.read(slice),
        Output::Vector(v) => decoder.read_to_end(v),
    }
}

/// Compress gzip data
pub fn compress<'a>(data: &'a [u8], output: Output<'a>, level: i32) -> Result<usize, Error> {
    let mut encoder = zstd::stream::read::Encoder::new(data, level)?;
    match output {
        Output::Slice(slice) => encoder.read(slice),
        Output::Vector(v) => encoder.read_to_end(v),
    }
}
