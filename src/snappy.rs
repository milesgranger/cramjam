//! snappy de/compression interface
use crate::exceptions::{CompressionError, DecompressionError};
use crate::io::{AsBytes, RustyBuffer};
use crate::{to_py_err, BytesType};
use pyo3::prelude::*;
use pyo3::wrap_pyfunction;
use pyo3::PyResult;
use std::io::Cursor;

pub(crate) fn init_py_module(m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(compress, m)?)?;
    m.add_function(wrap_pyfunction!(decompress, m)?)?;
    m.add_function(wrap_pyfunction!(compress_raw, m)?)?;
    m.add_function(wrap_pyfunction!(decompress_raw, m)?)?;
    m.add_function(wrap_pyfunction!(compress_into, m)?)?;
    m.add_function(wrap_pyfunction!(decompress_into, m)?)?;
    m.add_function(wrap_pyfunction!(compress_raw_into, m)?)?;
    m.add_function(wrap_pyfunction!(decompress_raw_into, m)?)?;
    m.add_function(wrap_pyfunction!(compress_raw_max_len, m)?)?;
    m.add_function(wrap_pyfunction!(decompress_raw_len, m)?)?;
    m.add_class::<Compressor>()?;
    Ok(())
}

/// Snappy decompression.
///
/// Python Example
/// --------------
/// ```python
/// >>> # bytes or bytearray; bytearray is faster
/// >>> cramjam.snappy.decompress(compressed_bytes, output_len=Optional[None])
/// ```
#[pyfunction]
pub fn decompress(data: BytesType, output_len: Option<usize>) -> PyResult<RustyBuffer> {
    crate::generic!(decompress(data), output_len = output_len)
}

/// Snappy compression.
///
/// Python Example
/// --------------
/// ```python
/// >>> _ = cramjam.snappy.compress(b'some bytes here')
/// >>> _ = cramjam.snappy.compress(bytearray(b'this avoids double allocation in rust side, and thus faster!'))  # <- use bytearray where possible
/// ```
#[pyfunction]
pub fn compress(data: BytesType, output_len: Option<usize>) -> PyResult<RustyBuffer> {
    crate::generic!(compress(data), output_len = output_len)
}

/// Snappy decompression, raw
/// This does not use the snappy 'framed' encoding of compressed bytes.
///
/// Python Example
/// --------------
/// ```python
/// >>> cramjam.snappy.decompress_raw(compressed_raw_bytes)
/// ```
#[pyfunction]
#[allow(unused_variables)]
pub fn decompress_raw(data: BytesType, output_len: Option<usize>) -> PyResult<RustyBuffer> {
    let mut decoder = snap::raw::Decoder::new();
    let output = to_py_err!(DecompressionError -> decoder.decompress_vec(data.as_bytes()))?;
    Ok(RustyBuffer::from(output))
}

/// Snappy compression raw.
/// This does not use the snappy 'framed' encoding of compressed bytes.
///
/// Python Example
/// --------------
/// ```python
/// >>> cramjam.snappy.compress_raw(b'some bytes here')
/// ```
#[pyfunction]
#[allow(unused_variables)]
pub fn compress_raw(data: BytesType, output_len: Option<usize>) -> PyResult<RustyBuffer> {
    let mut encoder = snap::raw::Encoder::new();
    let output = to_py_err!(CompressionError -> encoder.compress_vec(data.as_bytes()))?;
    Ok(RustyBuffer::from(output))
}

/// Compress directly into an output buffer
#[pyfunction]
pub fn compress_into(input: BytesType, mut output: BytesType) -> PyResult<usize> {
    let r = internal::compress(input, &mut output)?;
    Ok(r)
}

/// Decompress directly into an output buffer
#[pyfunction]
pub fn decompress_into(input: BytesType, mut output: BytesType) -> PyResult<usize> {
    let r = internal::decompress(input, &mut output)?;
    Ok(r as usize)
}

/// Compress raw format directly into an output buffer
#[pyfunction]
pub fn compress_raw_into(input: BytesType, mut output: BytesType) -> PyResult<usize> {
    let mut encoder = snap::raw::Encoder::new();
    let output = encoder.compress(input.as_bytes(), output.as_bytes_mut());
    to_py_err!(CompressionError -> output)
}

/// Decompress raw format directly into an output buffer
#[pyfunction]
pub fn decompress_raw_into(input: BytesType, mut output: BytesType) -> PyResult<usize> {
    let mut decoder = snap::raw::Decoder::new();
    let output = decoder.decompress(input.as_bytes(), output.as_bytes_mut());
    to_py_err!(DecompressionError -> output)
}

/// Get the expected max compressed length for snappy raw compression; this is the size
/// of buffer that should be passed to `compress_raw_into`
#[pyfunction]
pub fn compress_raw_max_len(data: BytesType) -> usize {
    snap::raw::max_compress_len(data.len())
}

/// Get the decompressed length for the given data. This is the size of buffer
/// that should be passed to `decompress_raw_into`
#[pyfunction]
pub fn decompress_raw_len(data: BytesType) -> PyResult<usize> {
    to_py_err!(DecompressionError -> snap::raw::decompress_len(data.as_bytes()))
}

/// Snappy Compressor object for streaming compression
#[pyclass]
pub struct Compressor {
    inner: Option<snap::write::FrameEncoder<Cursor<Vec<u8>>>>,
}

#[pymethods]
impl Compressor {
    /// Initialize a new `Compressor` instance.
    #[new]
    pub fn __init__() -> PyResult<Self> {
        let inner = snap::write::FrameEncoder::new(Cursor::new(vec![]));
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
        crate::io::stream_finish(&mut self.inner, |inner| inner.into_inner().map(|c| c.into_inner()))
    }
}

pub(crate) mod internal {
    use snap::read::{FrameDecoder, FrameEncoder};
    use std::io::{Error, Read, Write};

    /// Decompress snappy data framed
    pub fn decompress<W: Write + ?Sized, R: Read>(input: R, output: &mut W) -> Result<usize, Error> {
        let mut decoder = FrameDecoder::new(input);
        let n_bytes = std::io::copy(&mut decoder, output)?;
        Ok(n_bytes as usize)
    }

    /// Decompress snappy data framed
    pub fn compress<W: Write + ?Sized, R: Read>(data: R, output: &mut W) -> Result<usize, Error> {
        let mut encoder = FrameEncoder::new(data);
        let n_bytes = std::io::copy(&mut encoder, output)?;
        Ok(n_bytes as usize)
    }
}
