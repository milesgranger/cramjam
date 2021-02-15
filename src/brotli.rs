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

/// Brotli decompression.
///
/// Python Example
/// --------------
/// ```python
/// >>> cramjam.brotli.decompress(compressed_bytes, output_len=Optional[int])
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

/// Brotli compression.
///
/// Python Example
/// --------------
/// ```python
/// >>> cramjam.brotli.compress(b'some bytes here', level=9, output_len=Option[int])  # level defaults to 11
/// ```
#[pyfunction]
pub fn compress<'a>(
    py: Python<'a>,
    data: BytesType<'a>,
    level: Option<u32>,
    output_len: Option<usize>,
) -> PyResult<BytesType<'a>> {
    let level = level.unwrap_or_else(|| 11);
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
    use brotli2::read::{BrotliDecoder, BrotliEncoder};
    use std::io::prelude::*;
    use std::io::Error;

    /// Decompress via Brotli
    pub fn decompress<'a>(data: &[u8], output: Output<'a>) -> Result<usize, Error> {
        let mut decoder = BrotliDecoder::new(data);
        match output {
            Output::Slice(slice) => decoder.read(slice),
            Output::Vector(v) => decoder.read_to_end(v),
        }
    }

    /// Compress via Brotli
    pub fn compress<'a>(data: &'a [u8], output: Output<'a>, level: u32) -> Result<usize, Error> {
        let mut encoder = BrotliEncoder::new(data, level);
        match output {
            Output::Slice(slice) => encoder.read(slice),
            Output::Vector(v) => encoder.read_to_end(v),
        }
    }
}
