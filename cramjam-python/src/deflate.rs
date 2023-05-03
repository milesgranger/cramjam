//! deflate de/compression interface
use crate::exceptions::{CompressionError, DecompressionError};
use crate::io::RustyBuffer;
use crate::{AsBytes, BytesType};
use pyo3::prelude::*;
use pyo3::wrap_pyfunction;
use pyo3::PyResult;
use std::io::Cursor;

const DEFAULT_COMPRESSION_LEVEL: u32 = 6;

pub(crate) fn init_py_module(m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(compress, m)?)?;
    m.add_function(wrap_pyfunction!(decompress, m)?)?;
    m.add_function(wrap_pyfunction!(compress_into, m)?)?;
    m.add_function(wrap_pyfunction!(decompress_into, m)?)?;
    m.add_class::<Compressor>()?;
    m.add_class::<Decompressor>()?;
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
pub fn decompress(py: Python, data: BytesType, output_len: Option<usize>) -> PyResult<RustyBuffer> {
    crate::generic!(py, libcramjam::deflate::decompress[data], output_len = output_len)
        .map_err(DecompressionError::from_err)
}

/// Deflate compression.
///
/// Python Example
/// --------------
/// ```python
/// >>> cramjam.deflate.compress(b'some bytes here', level=5, output_len=Optional[int])  # level defaults to 6
/// ```
#[pyfunction]
pub fn compress(py: Python, data: BytesType, level: Option<u32>, output_len: Option<usize>) -> PyResult<RustyBuffer> {
    crate::generic!(
        py,
        libcramjam::deflate::compress[data],
        output_len = output_len,
        level = level
    )
    .map_err(CompressionError::from_err)
}

/// Compress directly into an output buffer
#[pyfunction]
pub fn compress_into(py: Python, input: BytesType, mut output: BytesType, level: Option<u32>) -> PyResult<usize> {
    crate::generic!(py, libcramjam::deflate::compress[input, output], level = level).map_err(CompressionError::from_err)
}

/// Decompress directly into an output buffer
#[pyfunction]
pub fn decompress_into(py: Python, input: BytesType, mut output: BytesType) -> PyResult<usize> {
    crate::generic!(py, libcramjam::deflate::decompress[input, output]).map_err(DecompressionError::from_err)
}

/// Deflate Compressor object for streaming compression
#[pyclass]
pub struct Compressor {
    inner: Option<libcramjam::deflate::flate2::write::DeflateEncoder<Cursor<Vec<u8>>>>,
}

#[pymethods]
impl Compressor {
    /// Initialize a new `Compressor` instance.
    #[new]
    pub fn __init__(level: Option<u32>) -> PyResult<Self> {
        let level = level.unwrap_or_else(|| DEFAULT_COMPRESSION_LEVEL);
        let compression = libcramjam::deflate::flate2::Compression::new(level);
        let inner = libcramjam::deflate::flate2::write::DeflateEncoder::new(Cursor::new(vec![]), compression);
        Ok(Self { inner: Some(inner) })
    }

    /// Compress input into the current compressor's stream.
    pub fn compress(&mut self, input: &[u8]) -> PyResult<usize> {
        crate::io::stream_compress(&mut self.inner, input)
    }

    /// Flush and return current compressed stream
    pub fn flush(&mut self) -> PyResult<RustyBuffer> {
        crate::io::stream_flush(&mut self.inner, |e| e.get_mut())
    }

    /// Consume the current compressor state and return the compressed stream
    /// **NB** The compressor will not be usable after this method is called.
    pub fn finish(&mut self) -> PyResult<RustyBuffer> {
        crate::io::stream_finish(&mut self.inner, |inner| inner.finish().map(|c| c.into_inner()))
    }
}

crate::make_decompressor!(deflate);
