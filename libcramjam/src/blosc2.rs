//! snappy de/compression interface

use ::blosc2::CParams;
pub use blosc2;
use std::io::{self, BufReader, Read, Write};

// TODO: Could downcast to check for file, then use file-backed SChunk

/// Compress using Blosc2 SChunk
pub fn compress<R, W>(rdr: R, wtr: &mut W) -> io::Result<usize>
where
    R: Read,
    W: Write + ?Sized,
{
    let mut schunk = blosc2::schunk::SChunk::new(
        blosc2::schunk::Storage::default()
            .set_contiguous(true)
            .set_cparams(&mut CParams::default())
            .set_dparams(&mut Default::default()),
    );
    let mut rdr = BufReader::new(rdr);

    // stream compress into schunk
    io::copy(&mut rdr, &mut schunk)?;

    let buf = schunk.into_vec()?;
    wtr.write_all(&buf)?;
    Ok(buf.len())
}

/// Decompress, assumed reader will be giving a SChunk compatible input
pub fn decompress<R, W>(input: R, output: &mut W) -> io::Result<usize>
where
    R: Read,
    W: Write + ?Sized,
{
    // TODO: Avoid the double copy somehow
    let mut buf = vec![];
    io::copy(&mut BufReader::new(input), &mut buf)?;

    let mut schunk = blosc2::schunk::SChunk::from_vec(buf)?;
    let mut decoder = blosc2::schunk::SChunkDecoder::new(&mut schunk);
    io::copy(&mut decoder, output).map(|v| v as usize)
}

#[inline(always)]
pub fn compress_chunk<T: 'static>(input: &[T]) -> io::Result<Vec<u8>> {
    let buf = blosc2::compress(input, None, None, None, None)?;
    Ok(buf)
}

pub fn compress_chunk_into<T: 'static>(input: &[T], output: &mut [u8]) -> io::Result<usize> {
    let nbytes = blosc2::compress_into(input, output, None, None, None, None)?;
    Ok(nbytes)
}

#[inline(always)]
pub fn decompress_chunk(input: &[u8]) -> io::Result<Vec<u8>> {
    let buf = blosc2::decompress(input)?;
    Ok(buf)
}

pub fn decompress_chunk_into<T>(input: &[u8], output: &mut [T]) -> io::Result<usize> {
    let nbytes = blosc2::decompress_into(input, output)?;
    Ok(nbytes)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn test_compress() {
        let mut compressed = vec![];
        let data = b"bytes";
        assert!(compress(Cursor::new(data), &mut compressed).is_ok());
    }
}
