use either::Either;
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
pub fn decompress(data: &[u8], output: Either<&mut [u8], &mut Vec<u8>>) -> Result<usize, Error> {
    let mut decoder = FrameDecoder::new(data);
    match output {
        Either::Left(slice) => decoder.read(slice),
        Either::Right(v) => decoder.read_to_end(v),
    }
}

/// Decompress snappy data framed
pub fn compress(data: &[u8], output: Either<&mut [u8], &mut Vec<u8>>) -> Result<usize, Error> {
    let mut encoder = FrameEncoder::new(data);
    match output {
        Either::Left(slice) => encoder.read(slice),
        Either::Right(v) => encoder.read_to_end(v),
    }
}
