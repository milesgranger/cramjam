//! brotli de/compression interface
use crate::exceptions::{CompressionError, DecompressionError};
use crate::io::RustyBuffer;
use crate::{to_py_err, BytesType};
use pyo3::prelude::*;
use pyo3::wrap_pyfunction;
use pyo3::PyResult;
use std::io::Cursor;

const DEFAULT_COMPRESSION_LEVEL: u32 = 11;

pub(crate) fn init_py_module(m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(compress, m)?)?;
    m.add_function(wrap_pyfunction!(decompress, m)?)?;
    m.add_function(wrap_pyfunction!(compress_into, m)?)?;
    m.add_function(wrap_pyfunction!(decompress_into, m)?)?;
    m.add_class::<Compressor>()?;
    Ok(())
}

/// Brotli decompression.
///
/// Python Example
/// --------------
/// ```python
/// >>> cramjam.brotli.decompress(compressed_bytes, output_len=Optional[int])
/// ```
#[pyfunction]
pub fn decompress(data: BytesType, output_len: Option<usize>) -> PyResult<RustyBuffer> {
    crate::generic!(decompress(data), output_len = output_len)
}

/// Brotli compression.
///
/// Python Example
/// --------------
/// ```python
/// >>> cramjam.brotli.compress(b'some bytes here', level=9, output_len=Option[int])  # level defaults to 11
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

/// Brotli Compressor object for streaming compression
#[pyclass]
pub struct Compressor {
    inner: Option<brotli2::write::BrotliEncoder<Cursor<Vec<u8>>>>,
}

#[pymethods]
impl Compressor {
    /// Initialize a new `Compressor` instance.
    #[new]
    pub fn __init__(level: Option<u32>) -> PyResult<Self> {
        let level = level.unwrap_or_else(|| DEFAULT_COMPRESSION_LEVEL);
        let inner = brotli2::write::BrotliEncoder::new(Cursor::new(vec![]), level);
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

pub(crate) mod internal {

    use crate::brotli::DEFAULT_COMPRESSION_LEVEL;
    use brotli2::read::{BrotliDecoder, BrotliEncoder};
    use std::io::prelude::*;
    use std::io::Error;

    /// Decompress via Brotli
    pub fn decompress<W: Write + ?Sized, R: Read>(input: R, output: &mut W) -> Result<usize, Error> {
        let mut decoder = BrotliDecoder::new(input);
        let n_bytes = std::io::copy(&mut decoder, output)?;
        Ok(n_bytes as usize)
    }

    /// Compress via Brotli
    pub fn compress<W: Write + ?Sized, R: Read>(input: R, output: &mut W, level: Option<u32>) -> Result<usize, Error> {
        let level = level.unwrap_or_else(|| DEFAULT_COMPRESSION_LEVEL);
        let mut encoder = BrotliEncoder::new(input, level);
        let n_bytes = std::io::copy(&mut encoder, output)?;
        Ok(n_bytes as usize)
    }
}
