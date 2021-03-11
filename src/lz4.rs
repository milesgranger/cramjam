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

/// LZ4 compression.
///
/// Python Example
/// --------------
/// ```python
/// >>> # Note, output_len is currently ignored; underlying algorithm does not support reading to slice at this time
/// >>> cramjam.lz4.decompress(compressed_bytes, output_len=Optional[int])
/// ```
#[pyfunction]
#[allow(unused_variables)] // TODO: Make use of output_len for lz4
pub fn decompress<'a>(py: Python<'a>, data: BytesType<'a>, output_len: Option<usize>) -> PyResult<BytesType<'a>> {
    crate::generic!(decompress(data), py = py, output_len = output_len)
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
pub fn compress<'a>(
    py: Python<'a>,
    data: BytesType<'a>,
    level: Option<u32>,
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
    level: Option<u32>,
) -> PyResult<usize> {
    crate::generic_into!(compress(data -> array), level)
}

/// Decompress directly into an output buffer
#[pyfunction]
pub fn decompress_into<'a>(_py: Python<'a>, data: BytesType<'a>, array: &'a PyArray1<u8>) -> PyResult<usize> {
    crate::generic_into!(decompress(data -> array))
}

pub(crate) mod internal {
    use lz4::{Decoder, EncoderBuilder};
    use std::io::{Error, Read, Write, Cursor, Seek, SeekFrom};

    /// Decompress lz4 data
    pub fn decompress<W: Write + ?Sized, R: Read>(input: R, output: &mut W) -> Result<usize, Error> {
        let mut decoder = Decoder::new(input)?;
        let n_bytes = std::io::copy(&mut decoder, output)?;
        decoder.finish().1?;
        Ok(n_bytes as usize)
    }

    /// Compress lz4 data
    // TODO: lz-fear does not yet support level
    pub fn compress<W: Write + ?Sized + Seek, R: Read>(
        input: &mut R,
        output: &mut W,
        level: Option<u32>,
    ) -> Result<usize, Error> {
        let mut encoder = EncoderBuilder::new()
            .auto_flush(true)
            .level(level.unwrap_or_else(|| 4))
            .build(output)?;

        std::io::copy(input, &mut encoder)?;
        let (w, r) = encoder.finish();
        r?;
        let n_bytes = w.seek(SeekFrom::Current(0))?;
        Ok(n_bytes as usize)
    }
}
