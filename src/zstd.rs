use crate::exceptions::{CompressionError, DecompressionError};
use crate::{to_py_err, BytesType, WriteablePyByteArray};
use numpy::PyArray1;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3::wrap_pyfunction;
use pyo3::{PyResult, Python};
use std::io::Cursor;

pub fn init_py_module(m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(compress, m)?)?;
    m.add_function(wrap_pyfunction!(decompress, m)?)?;
    m.add_function(wrap_pyfunction!(compress_into, m)?)?;
    m.add_function(wrap_pyfunction!(decompress_into, m)?)?;
    Ok(())
}

/// ZSTD decompression.
///
/// Python Example
/// --------------
/// ```python
/// >>> cramjam.zstd.decompress(compressed_bytes, output_len=Optional[int])
/// ```
#[pyfunction]
pub fn decompress<'a>(py: Python<'a>, data: BytesType<'a>, output_len: Option<usize>) -> PyResult<BytesType<'a>> {
    crate::generic!(decompress(data), py = py, output_len = output_len)
}

/// ZSTD compression.
///
/// Python Example
/// --------------
/// ```python
/// >>> cramjam.zstd.compress(b'some bytes here', level=0, output_len=Optional[int])  # level defaults to 11
/// ```
#[pyfunction]
pub fn compress<'a>(
    py: Python<'a>,
    data: BytesType<'a>,
    level: Option<i32>,
    output_len: Option<usize>,
) -> PyResult<BytesType<'a>> {
    crate::generic!(compress(data), py = py, output_len = output_len, level = level)
}

/// Compress directly into an output buffer
#[pyfunction]
pub fn compress_into<'a>(
    _py: Python<'a>,
    data: BytesType<'a>,
    array: &PyArray1<u8>,
    level: Option<i32>,
) -> PyResult<usize> {
    crate::generic_into!(compress(data -> array), level)
}

/// Decompress directly into an output buffer
#[pyfunction]
pub fn decompress_into<'a>(_py: Python<'a>, data: BytesType<'a>, array: &'a PyArray1<u8>) -> PyResult<usize> {
    crate::generic_into!(decompress(data -> array))
}

pub(crate) mod internal {

    use std::io::{Error, Write};

    /// Decompress gzip data
    pub fn decompress<W: Write + ?Sized>(input: &[u8], output: &mut W) -> Result<usize, Error> {
        let mut decoder = zstd::stream::read::Decoder::new(input)?;
        let n_bytes = std::io::copy(&mut decoder, output)?;
        Ok(n_bytes as usize)
    }

    /// Compress gzip data
    pub fn compress<W: Write + ?Sized>(input: &[u8], output: &mut W, level: Option<i32>) -> Result<usize, Error> {
        let level = level.unwrap_or_else(|| 0); // 0 will use zstd's default, currently 11
        let mut encoder = zstd::stream::read::Encoder::new(input, level)?;
        let n_bytes = std::io::copy(&mut encoder, output)?;
        Ok(n_bytes as usize)
    }
}
