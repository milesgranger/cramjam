use crate::exceptions::{CompressionError, DecompressionError};
use crate::{to_py_err, BytesType, Output};
use numpy::PyArray1;
use pyo3::prelude::*;
use pyo3::types::{PyByteArray, PyBytes};
use pyo3::wrap_pyfunction;
use pyo3::{PyResult, Python};

pub fn init_py_module(m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(compress, m)?)?;
    m.add_function(wrap_pyfunction!(decompress, m)?)?;
    m.add_function(wrap_pyfunction!(compress_into, m)?)?;
    m.add_function(wrap_pyfunction!(decompress_into, m)?)?;
    Ok(())
}

/// Gzip decompression.
///
/// Python Example
/// --------------
/// ```python
/// >>> cramjam.gzip.decompress(compressed_bytes, output_len=Optional[int])
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

/// Gzip compression.
///
/// Python Example
/// --------------
/// ```python
/// >>> cramjam.gzip.compress(b'some bytes here', level=2, output_len=Optional[int])  # Level defaults to 6
/// ```
#[pyfunction]
pub fn compress<'a>(
    py: Python<'a>,
    data: BytesType<'a>,
    level: Option<u32>,
    output_len: Option<usize>,
) -> PyResult<BytesType<'a>> {
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

/// Compress directly into an output buffer
#[pyfunction]
pub fn compress_into<'a>(
    _py: Python<'a>,
    data: BytesType<'a>,
    array: &PyArray1<u8>,
    level: Option<u32>,
) -> PyResult<usize> {
    crate::de_compress_into(data.as_bytes(), array, |bytes, out| {
        self::internal::compress(bytes, out, level)
    })
}

/// Decompress directly into an output buffer
#[pyfunction]
pub fn decompress_into<'a>(_py: Python<'a>, data: BytesType<'a>, array: &'a PyArray1<u8>) -> PyResult<usize> {
    crate::de_compress_into(data.as_bytes(), array, self::internal::decompress)
}

mod internal {
    use crate::Output;
    use flate2::read::GzDecoder;
    use flate2::read::GzEncoder;
    use flate2::Compression;
    use flate2::Crc;
    use std::io::prelude::*;
    use std::io::Error;

    /// Decompress gzip data
    pub fn decompress<'a>(data: &'a [u8], output: Output<'a>) -> Result<usize, Error> {
        let mut decoder = GzDecoder::new(data);
        match output {
            Output::Slice(slice) => decoder.read(slice),
            Output::Vector(v) => decoder.read_to_end(v),
        }
    }

    /// Compress gzip data
    pub fn compress<'a>(data: &'a [u8], output: Output<'a>, level: Option<u32>) -> Result<usize, Error> {
        let level = level.unwrap_or_else(|| 6);
        match output {
            Output::Slice(slice) => {
                // GzEncoder::read does not output the 'tail' of the gzip encoding. So we need to
                // calculate the checksum and the data length manually.

                // compute checksum
                let mut crc = Crc::new();
                crc.update(&data);

                // Encode
                let mut encoder = GzEncoder::new(data, Compression::new(level));
                let n_bytes = encoder.read(slice)?;

                // insert checksum as bytes into output
                let mut checksum_bytes = crc.sum().to_le_bytes();
                slice[n_bytes..n_bytes + 4].swap_with_slice(&mut checksum_bytes);

                // insert data len as bytes into output
                let mut data_len_bytes = (data.len() as u32).to_le_bytes();
                slice[n_bytes + 4..n_bytes + 8].swap_with_slice(&mut data_len_bytes);

                // Ka-pow, total bytes affected output
                Ok(n_bytes + checksum_bytes.len() + data_len_bytes.len())
            }
            Output::Vector(v) => {
                let mut encoder = GzEncoder::new(data, Compression::new(level));
                encoder.read_to_end(v)
            }
        }
    }
}
