//! lz4 de/compression interface
pub use lz4;
use lz4::{Decoder, EncoderBuilder};
use std::io::{BufReader, Error, Read, Seek, SeekFrom, Write};

const DEFAULT_COMPRESSION_LEVEL: u32 = 4;

/// Decompress lz4 data
#[inline(always)]
pub fn decompress<W: Write + ?Sized, R: Read>(input: R, output: &mut W) -> Result<usize, Error> {
    let mut decoder = Decoder::new(input)?;
    let n_bytes = std::io::copy(&mut decoder, output)?;
    decoder.finish().1?;
    Ok(n_bytes as usize)
}

/// Compress lz4 data
#[inline(always)]
pub fn compress<W: Write + ?Sized + Seek, R: Read>(
    input: R,
    output: &mut W,
    level: Option<u32>,
) -> Result<usize, Error> {
    let start_pos = output.seek(SeekFrom::Current(0))?;
    let mut encoder = EncoderBuilder::new()
        .auto_flush(true)
        .level(level.unwrap_or_else(|| DEFAULT_COMPRESSION_LEVEL))
        .build(output)?;

    // this returns, bytes read from uncompressed, input; we want bytes written
    // but lz4 only implements Read for Encoder
    let mut buf = BufReader::new(input);
    std::io::copy(&mut buf, &mut encoder)?;
    let (w, r) = encoder.finish();
    r?;
    let ending_pos = w.seek(SeekFrom::Current(0))?;
    Ok((ending_pos - start_pos) as usize)
}
