//! gzip de/compression interface
pub use flate2;
use flate2::read::{GzEncoder, MultiGzDecoder};
use flate2::Compression;
use std::io::prelude::*;
use std::io::{Cursor, Error};

const DEFAULT_COMPRESSION_LEVEL: u32 = 6;

/// Decompress gzip data
#[inline(always)]
pub fn decompress<W: Write + ?Sized, R: Read>(input: R, output: &mut W) -> Result<usize, Error> {
    let mut decoder = MultiGzDecoder::new(input);
    let mut out = vec![];
    let n_bytes = decoder.read_to_end(&mut out)?;
    std::io::copy(&mut Cursor::new(out.as_slice()), output)?;
    Ok(n_bytes as usize)
}

/// Compress gzip data
#[inline(always)]
pub fn compress<W: Write + ?Sized, R: Read>(input: R, output: &mut W, level: Option<u32>) -> Result<usize, Error> {
    let level = level.unwrap_or_else(|| DEFAULT_COMPRESSION_LEVEL);
    let mut encoder = GzEncoder::new(input, Compression::new(level));
    let n_bytes = std::io::copy(&mut encoder, output)?;
    Ok(n_bytes as usize)
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_gzip_multiple_streams() {
        let mut out1 = vec![];
        let mut out2 = vec![];
        super::compress(b"foo".to_vec().as_slice(), &mut out1, None).unwrap();
        super::compress(b"bar".to_vec().as_slice(), &mut out2, None).unwrap();

        let mut out3 = vec![];
        out1.extend_from_slice(&out2);
        super::decompress(out1.as_slice(), &mut out3).unwrap();
        assert_eq!(out3, b"foobar".to_vec());
    }
}
