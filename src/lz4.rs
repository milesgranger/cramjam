//! lz4 de/compression interface
use crate::exceptions::{CompressionError, DecompressionError};
use crate::io::RustyBuffer;
use crate::{to_py_err, BytesType};
use pyo3::prelude::*;
use pyo3::wrap_pyfunction;
use pyo3::PyResult;
use std::io::Cursor;

pub(crate) fn init_py_module(m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(compress, m)?)?;
    m.add_function(wrap_pyfunction!(decompress, m)?)?;
    m.add_function(wrap_pyfunction!(compress_into, m)?)?;
    m.add_function(wrap_pyfunction!(decompress_into, m)?)?;
    Ok(())
}

/// LZ4 compression.
///
/// Python Example
/// --------------
/// ```python
/// >>> # Note, output_len is currently ignored; underlying algorithm does not support reading to slice at this time
/// >>> cramjam.lz4.decompress(compressed_bytes, output_len=Optional[int])
/// ```
#[pyfunction]
pub fn decompress(data: BytesType, output_len: Option<usize>) -> PyResult<RustyBuffer> {
    crate::generic!(decompress(data), output_len = output_len)
}

/// lZ4 compression.
///
/// Python Example
/// --------------
/// ```python
/// >>> # Note, output_len is currently ignored; underlying algorithm does not support reading to slice at this time
/// >>> cramjam.lz4.compress(b'some bytes here', output_len=Optional[int])
/// ```
#[pyfunction]
pub fn compress(mut data: BytesType, level: Option<u32>, output_len: Option<usize>) -> PyResult<RustyBuffer> {
    crate::generic!(compress(&mut data), output_len = output_len, level = level)
}

/// Compress directly into an output buffer
#[pyfunction]
pub fn compress_into(mut input: BytesType, mut output: BytesType, level: Option<u32>) -> PyResult<usize> {
    let r = internal::compress(&mut input, &mut output, level)?;
    Ok(r)
}

/// Decompress directly into an output buffer
#[pyfunction]
pub fn decompress_into(input: BytesType, mut output: BytesType) -> PyResult<usize> {
    let r = internal::decompress(input, &mut output)?;
    Ok(r)
}

pub(crate) mod internal {
    use lz4::{Decoder, EncoderBuilder};
    use std::io::{Error, Read, Seek, SeekFrom, Write};

    /// Decompress lz4 data
    pub fn decompress<W: Write + ?Sized, R: Read>(input: R, output: &mut W) -> Result<usize, Error> {
        let mut decoder = Decoder::new(input)?;
        let n_bytes = std::io::copy(&mut decoder, output)?;
        decoder.finish().1?;
        Ok(n_bytes as usize)
    }

    /// Compress lz4 data
    pub fn compress<W: Write + ?Sized + Seek, R: Read>(
        input: &mut R,
        output: &mut W,
        level: Option<u32>,
    ) -> Result<usize, Error> {
        let start_pos = output.seek(SeekFrom::Current(0))?;
        let mut encoder = EncoderBuilder::new()
            .auto_flush(true)
            .level(level.unwrap_or_else(|| 4))
            .build(output)?;

        // this returns, bytes read from uncompressed, input; we want bytes written
        // but lz4 only implements Read for Encoder
        std::io::copy(input, &mut encoder)?;
        let (w, r) = encoder.finish();
        r?;
        let ending_pos = w.seek(SeekFrom::Current(0))?;
        Ok((ending_pos - start_pos) as usize)
    }
}
