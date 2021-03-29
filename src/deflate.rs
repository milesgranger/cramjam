//! deflate de/compression interface
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

/// Deflate decompression.
///
/// Python Example
/// --------------
/// ```python
/// >>> cramjam.deflate.decompress(compressed_bytes, output_len=Optional[int])
/// ```
#[pyfunction]
pub fn decompress(data: BytesType, output_len: Option<usize>) -> PyResult<RustyBuffer> {
    crate::generic!(decompress(data), output_len = output_len)
}

/// Deflate compression.
///
/// Python Example
/// --------------
/// ```python
/// >>> cramjam.deflate.compress(b'some bytes here', level=5, output_len=Optional[int])  # level defaults to 6
/// ```
#[pyfunction]
pub fn compress(data: BytesType, level: Option<u32>, output_len: Option<usize>) -> PyResult<RustyBuffer> {
    crate::generic!(compress(data), output_len = output_len, level = level)
}

/// Compress directly into an output buffer
#[pyfunction]
pub fn compress_into(input: BytesType, mut output: BytesType, level: Option<u32>) -> PyResult<usize> {
    let r = internal::compress(input, &mut output, level)?;
    Ok(r)
}

/// Decompress directly into an output buffer
#[pyfunction]
pub fn decompress_into(input: BytesType, mut output: BytesType) -> PyResult<usize> {
    let r = internal::decompress(input, &mut output)?;
    Ok(r)
}

pub(crate) mod internal {

    use flate2::read::{DeflateDecoder, DeflateEncoder};
    use flate2::Compression;
    use std::io::prelude::*;
    use std::io::Error;

    /// Decompress gzip data
    pub fn decompress<W: Write + ?Sized, R: Read>(input: R, output: &mut W) -> Result<usize, Error> {
        let mut decoder = DeflateDecoder::new(input);
        let n_bytes = std::io::copy(&mut decoder, output)?;
        Ok(n_bytes as usize)
    }

    /// Compress gzip data
    pub fn compress<W: Write + ?Sized, R: Read>(input: R, output: &mut W, level: Option<u32>) -> Result<usize, Error> {
        let level = level.unwrap_or_else(|| 6);

        let mut encoder = DeflateEncoder::new(input, Compression::new(level));
        let n_bytes = std::io::copy(&mut encoder, output)?;
        Ok(n_bytes as usize)
    }
}
