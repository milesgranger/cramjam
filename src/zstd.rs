use crate::exceptions::{CompressionError, DecompressionError};
use crate::{to_py_err, BytesType, Output};
use pyo3::prelude::*;
use pyo3::types::{PyByteArray, PyBytes};
use pyo3::wrap_pyfunction;
use pyo3::{PyResult, Python};

pub fn init_py_module(m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(compress, m)?)?;
    m.add_function(wrap_pyfunction!(decompress, m)?)?;
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
    match data {
        BytesType::Bytes(input) => match output_len {
            Some(len) => {
                let pybytes = PyBytes::new_with(py, len, |buffer| {
                    let output = Output::Slice(buffer);
                    to_py_err!(DecompressionError -> self::internal::decompress(input.as_bytes(), output))?;
                    Ok(())
                })?;
                Ok(BytesType::Bytes(pybytes))
            }
            None => {
                let mut buffer = Vec::with_capacity(data.len() / 10);
                let output = Output::Vector(&mut buffer);
                to_py_err!(DecompressionError -> self::internal::decompress(input.as_bytes(), output))?;
                Ok(BytesType::Bytes(PyBytes::new(py, &buffer)))
            }
        },
        BytesType::ByteArray(input) => match output_len {
            Some(len) => {
                let mut size = 0;
                let pybytes = PyByteArray::new_with(py, len, |buffer| {
                    let output = Output::Slice(buffer);
                    size = to_py_err!(DecompressionError -> self::internal::decompress(unsafe { input.as_bytes() }, output))?;
                    Ok(())
                })?;
                pybytes.resize(size)?;
                Ok(BytesType::ByteArray(pybytes))
            }
            None => {
                let mut buffer = Vec::with_capacity(data.len() / 10);
                let output = Output::Vector(&mut buffer);
                to_py_err!(DecompressionError -> self::internal::decompress(unsafe { input.as_bytes() }, output))?;
                Ok(BytesType::ByteArray(PyByteArray::new(py, &buffer)))
            }
        },
    }
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
    let level = level.unwrap_or_else(|| 0); // 0 will use zstd's default, currently 11

    match data {
        BytesType::Bytes(input) => match output_len {
            Some(len) => {
                let pybytes = PyBytes::new_with(py, len, |buffer| {
                    let output = Output::Slice(buffer);
                    to_py_err!(CompressionError -> self::internal::compress(input.as_bytes(), output, level))?;
                    Ok(())
                })?;
                Ok(BytesType::Bytes(pybytes))
            }
            None => {
                let mut buffer = Vec::with_capacity(data.len() / 10);
                let output = Output::Vector(&mut buffer);
                to_py_err!(CompressionError -> self::internal::compress(input.as_bytes(), output, level))?;
                Ok(BytesType::Bytes(PyBytes::new(py, &buffer)))
            }
        },
        BytesType::ByteArray(input) => match output_len {
            Some(len) => {
                let mut size = 0;
                let pybytes = PyByteArray::new_with(py, len, |buffer| {
                    let output = Output::Slice(buffer);
                    size = to_py_err!(CompressionError -> self::internal::compress(unsafe { input.as_bytes() }, output, level))?;
                    Ok(())
                })?;
                pybytes.resize(size)?;
                Ok(BytesType::ByteArray(pybytes))
            }
            None => {
                let mut buffer = Vec::with_capacity(data.len() / 10);
                let output = Output::Vector(&mut buffer);
                to_py_err!(CompressionError -> self::internal::compress(unsafe { input.as_bytes() }, output, level))?;
                Ok(BytesType::ByteArray(PyByteArray::new(py, &buffer)))
            }
        },
    }
}

mod internal {

    use crate::Output;
    use std::io::{Error, Read};

    /// Decompress gzip data
    pub fn decompress<'a>(data: &'a [u8], output: Output<'a>) -> Result<usize, Error> {
        let mut decoder = zstd::stream::read::Decoder::new(data)?;
        match output {
            Output::Slice(slice) => decoder.read(slice),
            Output::Vector(v) => decoder.read_to_end(v),
        }
    }

    /// Compress gzip data
    pub fn compress<'a>(data: &'a [u8], output: Output<'a>, level: i32) -> Result<usize, Error> {
        let mut encoder = zstd::stream::read::Encoder::new(data, level)?;
        match output {
            Output::Slice(slice) => encoder.read(slice),
            Output::Vector(v) => encoder.read_to_end(v),
        }
    }
}
