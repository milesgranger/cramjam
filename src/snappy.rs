use crate::exceptions::{CompressionError, DecompressionError};
use crate::{to_py_err, BytesType, Output};
use numpy::PyArray1;
use pyo3::prelude::*;
use pyo3::types::{PyByteArray, PyBytes};
use pyo3::wrap_pyfunction;
use pyo3::{PyResult, Python};
use snap::raw::max_compress_len;

pub fn init_py_module(m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(compress, m)?)?;
    m.add_function(wrap_pyfunction!(decompress, m)?)?;
    m.add_function(wrap_pyfunction!(compress_raw, m)?)?;
    m.add_function(wrap_pyfunction!(decompress_raw, m)?)?;
    m.add_function(wrap_pyfunction!(compress_into, m)?)?;
    m.add_function(wrap_pyfunction!(decompress_into, m)?)?;
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
    let result = match data {
        BytesType::Bytes(bytes) => {
            let pybytes = match output_len {
                Some(len) => PyBytes::new_with(py, len, |output| {
                    to_py_err!(DecompressionError -> self::internal::decompress(bytes.as_bytes(), Output::Slice(output)))?;
                    Ok(())
                })?,
                None => {
                    let mut output = Vec::with_capacity(data.len());

                    to_py_err!(DecompressionError -> self::internal::decompress(bytes.as_bytes(), Output::Vector(&mut output)))?;
                    PyBytes::new(py, &output)
                }
            };
            BytesType::Bytes(pybytes)
        }
        BytesType::ByteArray(bytes_array) => {
            let bytes = unsafe { bytes_array.as_bytes() };
            match output_len {
                Some(len) => {
                    let mut actual_len = 0;
                    let pybytes = PyByteArray::new_with(py, len, |output| {
                        actual_len =
                            to_py_err!(DecompressionError -> self::internal::decompress(bytes, Output::Slice(output)))?;
                        Ok(())
                    })?;
                    pybytes.resize(actual_len)?;
                    BytesType::ByteArray(pybytes)
                }
                None => {
                    let mut output = Vec::with_capacity(data.len());
                    to_py_err!(DecompressionError -> self::internal::decompress(bytes, Output::Vector(&mut output)))?;
                    let pybytes = PyByteArray::new(py, &output);
                    BytesType::ByteArray(pybytes)
                }
            }
        }
    };
    Ok(result)
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
    // Prefer the user's output_len, fallback to estimate the output len
    let estimated_len = output_len.unwrap_or_else(|| max_compress_len(data.len()));

    let result = match data {
        BytesType::Bytes(bytes) => {
            // user provided the exact output len
            if output_len.is_some() {
                let pybytes = PyBytes::new_with(py, estimated_len, |buffer| {
                    to_py_err!(CompressionError -> self::internal::compress(bytes.as_bytes(), Output::Slice(buffer)))?;
                    Ok(())
                })?;
                BytesType::Bytes(pybytes)

                // we can use the estimated length, but need to use buffer as we don't know for sure the length
            } else {
                let mut buffer = Vec::with_capacity(estimated_len);

                to_py_err!(CompressionError -> self::internal::compress(bytes.as_bytes(), Output::Vector(&mut buffer)))?;

                let pybytes = PyBytes::new(py, &buffer);
                BytesType::Bytes(pybytes)
            }
        }
        BytesType::ByteArray(bytes_array) => {
            let bytes = unsafe { bytes_array.as_bytes() };
            let mut actual_len = 0;
            let pybytes = PyByteArray::new_with(py, estimated_len, |output| {
                actual_len = to_py_err!(CompressionError -> self::internal::compress(bytes, Output::Slice(output)))?;
                Ok(())
            })?;
            pybytes.resize(actual_len)?;
            BytesType::ByteArray(pybytes)
        }
    };
    Ok(result)
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
    match data {
        BytesType::Bytes(input) => {
            let out = to_py_err!(DecompressionError -> self::internal::decompress_raw(input.as_bytes()))?;
            Ok(BytesType::Bytes(PyBytes::new(py, &out)))
        }
        BytesType::ByteArray(input) => {
            let out = to_py_err!(DecompressionError -> self::internal::decompress_raw(unsafe { input.as_bytes() }))?;
            Ok(BytesType::ByteArray(PyByteArray::new(py, &out)))
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
    match data {
        BytesType::Bytes(input) => {
            let out = to_py_err!(CompressionError -> self::internal::compress_raw(input.as_bytes()))?;
            Ok(BytesType::Bytes(PyBytes::new(py, &out)))
        }
        BytesType::ByteArray(input) => {
            let out = to_py_err!(CompressionError -> self::internal::compress_raw(unsafe { input.as_bytes() }))?;
            Ok(BytesType::ByteArray(PyByteArray::new(py, &out)))
        }
    }
}

/// Compress directly into an output buffer
#[pyfunction]
pub fn compress_into<'a>(_py: Python<'a>, data: BytesType<'a>, array: &PyArray1<u8>) -> PyResult<usize> {
    crate::de_compress_into(data.as_bytes(), array, self::internal::compress)
}

/// Decompress directly into an output buffer
#[pyfunction]
pub fn decompress_into<'a>(_py: Python<'a>, data: BytesType<'a>, array: &'a PyArray1<u8>) -> PyResult<usize> {
    crate::de_compress_into(data.as_bytes(), array, self::internal::decompress)
}

pub(crate) mod internal {
    use snap::raw::{Decoder, Encoder};
    use snap::read::{FrameDecoder, FrameEncoder};
    use std::io::{Error, Cursor};

    use crate::Output;

    /// Decompress snappy data raw
    pub fn decompress_raw(data: &[u8]) -> Result<Vec<u8>, snap::Error> {
        let mut decoder = Decoder::new();
        decoder.decompress_vec(data)
    }

    /// Compress snappy data raw
    pub fn compress_raw(data: &[u8]) -> Result<Vec<u8>, snap::Error> {
        let mut encoder = Encoder::new();
        encoder.compress_vec(data)
    }

    /// Decompress snappy data framed
    pub fn decompress<'a>(data: &'a [u8], output: Output<'a>) -> Result<usize, Error> {
        let mut decoder = FrameDecoder::new(data);
        match output {
            Output::Slice(slice) => {
                let mut wtr = Cursor::new(slice);
                let n_bytes = std::io::copy(&mut decoder, &mut wtr)?;
                Ok(n_bytes as usize)
            }
            Output::Vector(v) => {
                let mut wtr = Cursor::new(v);
                let n_bytes = std::io::copy(&mut decoder, &mut wtr)?;
                Ok(n_bytes as usize)
            },
        }
    }

    /// Decompress snappy data framed
    pub fn compress<'a>(data: &'a [u8], output: Output<'a>) -> Result<usize, Error> {
        let mut encoder = FrameEncoder::new(data);
        match output {
            Output::Slice(slice) => {
                let mut wtr = Cursor::new(slice);
                let n_bytes = std::io::copy(&mut encoder, &mut wtr)?;
                Ok(n_bytes as usize)
            }
            Output::Vector(v) => {
                let mut wtr = Cursor::new(v);
                let n_bytes = std::io::copy(&mut encoder, &mut wtr)?;
                Ok(n_bytes as usize)
            },
        }
    }
}
