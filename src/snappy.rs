use crate::exceptions::{CompressionError, DecompressionError};
use crate::{to_py_err, BytesType, WriteablePyByteArray};
use numpy::PyArray1;
use pyo3::prelude::*;
use pyo3::types::{PyByteArray, PyBytes};
use pyo3::wrap_pyfunction;
use pyo3::{PyResult, Python};
use std::io::Cursor;

pub fn init_py_module(m: &PyModule) -> PyResult<()> {
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
pub fn decompress<'a>(py: Python<'a>, data: BytesType<'a>, output_len: Option<usize>) -> PyResult<BytesType<'a>> {
    crate::generic!(decompress(data), py = py, output_len = output_len)
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
pub fn compress<'a>(py: Python<'a>, data: BytesType<'a>, output_len: Option<usize>) -> PyResult<BytesType<'a>> {
    crate::generic!(compress(data), py = py, output_len = output_len)
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
pub fn decompress_raw<'a>(py: Python<'a>, data: BytesType<'a>) -> PyResult<BytesType<'a>> {
    let output_len = to_py_err!(DecompressionError -> snap::raw::decompress_len(data.as_bytes()))?;

    match data {
        BytesType::Bytes(_) => {
            let pybytes = PyBytes::new_with(py, output_len, |output| {
                to_py_err!(DecompressionError -> self::internal::decompress_raw_into(data.as_bytes(), &mut Cursor::new(output)))?;
                Ok(())
            })?;
            Ok(BytesType::Bytes(pybytes))
        }
        BytesType::ByteArray(_) => {
            let pybytes = PyByteArray::new_with(py, output_len, |output| {
                to_py_err!(DecompressionError -> self::internal::decompress_raw_into(data.as_bytes(), &mut Cursor::new(output)))?;
                Ok(())
            })?;
            Ok(BytesType::ByteArray(pybytes))
        }
    }
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
pub fn compress_raw<'a>(py: Python<'a>, data: BytesType<'a>) -> PyResult<BytesType<'a>> {
    let output_len = snap::raw::max_compress_len(data.len());

    match data {
        BytesType::Bytes(_) => {
            let mut output = vec![0; output_len];
            let n_bytes = to_py_err!(CompressionError -> self::internal::compress_raw_into(data.as_bytes(), &mut Cursor::new(output.as_mut_slice())))?;
            output.truncate(n_bytes);
            Ok(BytesType::Bytes(PyBytes::new(py, &output)))
        }
        BytesType::ByteArray(_) => {
            let mut actual_size = 0;
            let pybytes = PyByteArray::new_with(py, output_len, |output| {
                let mut cursor = Cursor::new(output);
                actual_size =
                    to_py_err!(CompressionError -> self::internal::compress_raw_into(data.as_bytes(), &mut cursor))?;
                Ok(())
            })?;
            pybytes.resize(actual_size)?;
            Ok(BytesType::ByteArray(pybytes))
        }
    }
}

/// Compress directly into an output buffer
#[pyfunction]
pub fn compress_into<'a>(_py: Python<'a>, data: BytesType<'a>, array: &PyArray1<u8>) -> PyResult<usize> {
    crate::generic_into!(compress(data -> array))
}

/// Decompress directly into an output buffer
#[pyfunction]
pub fn decompress_into<'a>(_py: Python<'a>, data: BytesType<'a>, array: &'a PyArray1<u8>) -> PyResult<usize> {
    crate::generic_into!(decompress(data -> array))
}

/// Compress raw format directly into an output buffer
#[pyfunction]
pub fn compress_raw_into<'a>(_py: Python<'a>, data: BytesType<'a>, array: &PyArray1<u8>) -> PyResult<usize> {
    crate::generic_into!(compress_raw_into(data -> array))
}

/// Decompress raw format directly into an output buffer
#[pyfunction]
pub fn decompress_raw_into<'a>(_py: Python<'a>, data: BytesType<'a>, array: &PyArray1<u8>) -> PyResult<usize> {
    crate::generic_into!(decompress_raw_into(data -> array))
}

/// Get the expected max compressed length for snappy raw compression; this is the size
/// of buffer that should be passed to `compress_raw_into`
#[pyfunction]
pub fn compress_raw_max_len<'a>(_py: Python<'a>, data: BytesType<'a>) -> usize {
    snap::raw::max_compress_len(data.len())
}

/// Get the decompressed length for the given data. This is the size of buffer
/// that should be passed to `decompress_raw_into`
#[pyfunction]
pub fn decompress_raw_len<'a>(_py: Python<'a>, data: BytesType<'a>) -> PyResult<usize> {
    to_py_err!(DecompressionError -> snap::raw::decompress_len(data.as_bytes()))
}

pub(crate) mod internal {
    use snap::raw::{Decoder, Encoder};
    use snap::read::{FrameDecoder, FrameEncoder};
    use std::io::{Cursor, Error, Write};

    /// Decompress snappy data raw into a mutable slice
    pub fn decompress_raw_into(input: &[u8], output: &mut Cursor<&mut [u8]>) -> Result<usize, snap::Error> {
        let mut decoder = Decoder::new();
        let buffer = output.get_mut();
        decoder.decompress(input, *buffer)
    }

    /// Compress snappy data raw into a mutable slice
    pub fn compress_raw_into(input: &[u8], output: &mut Cursor<&mut [u8]>) -> Result<usize, snap::Error> {
        let mut encoder = Encoder::new();
        let buffer = output.get_mut();
        encoder.compress(input, buffer)
    }

    /// Decompress snappy data framed
    pub fn decompress<W: Write + ?Sized>(input: &[u8], output: &mut W) -> Result<usize, Error> {
        let mut decoder = FrameDecoder::new(input);
        let n_bytes = std::io::copy(&mut decoder, output)?;
        Ok(n_bytes as usize)
    }

    /// Decompress snappy data framed
    pub fn compress<W: Write + ?Sized>(data: &[u8], output: &mut W) -> Result<usize, Error> {
        let mut encoder = FrameEncoder::new(data);
        let n_bytes = std::io::copy(&mut encoder, output)?;
        Ok(n_bytes as usize)
    }
}
