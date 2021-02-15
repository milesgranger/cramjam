//! CramJam documentation of python exported functions for (de)compression of bytes
//!
//! The API follows `<<compression algorithm>>_compress` and `<<compression algorithm>>_decompress`
//!
//! Python Example:
//!
//! ```python
//! data = b'some bytes here'
//! compressed = cramjam.snappy_compress(data)
//! decompressed = cramjam.snappy_decompress(compressed)
//! assert data == decompressed
//! ```

// TODO: There is a lot of very similar, but slightly different code for each variant
// time should be spent perhaps with a macro or other alternative.
// Each variant is similar, but sometimes has subtly different APIs/logic.

// TODO: Add output size estimation for each variant, now it's just snappy
// allow for resizing PyByteArray if over allocated; cannot resize PyBytes yet.

// TODO: Convert to modules

mod brotli;
mod deflate;
mod exceptions;
mod gzip;
mod lz4;
mod snappy;
mod zstd;

use pyo3::prelude::*;
use pyo3::types::{PyByteArray, PyBytes};
use pyo3::wrap_pyfunction;
use snap::raw::decompress_len;
use snap::raw::max_compress_len;

use exceptions::{CompressionError, DecompressionError};

#[derive(FromPyObject)]
pub enum BytesType<'a> {
    #[pyo3(transparent, annotation = "bytes")]
    Bytes(&'a PyBytes),
    #[pyo3(transparent, annotation = "bytearray")]
    ByteArray(&'a PyByteArray),
}

impl<'a> BytesType<'a> {
    fn len(&self) -> usize {
        self.as_bytes().len()
    }
    fn as_bytes(&self) -> &'a [u8] {
        match self {
            Self::Bytes(b) => b.as_bytes(),
            Self::ByteArray(b) => unsafe { b.as_bytes() },
        }
    }
}

impl<'a> IntoPy<PyObject> for BytesType<'a> {
    fn into_py(self, py: Python) -> PyObject {
        match self {
            Self::Bytes(bytes) => bytes.to_object(py),
            Self::ByteArray(byte_array) => byte_array.to_object(py),
        }
    }
}

/// Buffer to de/compression algorithms' output.
/// ::Vector used when the output len cannot be determined, and/or resulting
/// python object cannot be resized to what the actual bytes decoded was.
pub enum Output<'a> {
    Slice(&'a mut [u8]),
    Vector(&'a mut Vec<u8>),
}

macro_rules! to_py_err {
    ($error:ident -> $expr:expr) => {
        $expr.map_err(|err| PyErr::new::<$error, _>(err.to_string()))
    };
}

/// Snappy decompression.
///
/// Python Example
/// --------------
/// ```python
/// >>> snappy_decompress(compressed_bytes)  # bytes or bytearray; bytearray is faster
/// ```
#[pyfunction]
pub fn snappy_decompress<'a>(py: Python<'a>, data: BytesType<'a>, output_len: Option<usize>) -> PyResult<BytesType<'a>> {
    let estimated_len = match output_len {
        Some(len) => len,
        None => to_py_err!(DecompressionError -> decompress_len(data.as_bytes()))?,
    };

    let result = match data {
        BytesType::Bytes(bytes) => {
            let pybytes = if output_len.is_some() {
                PyBytes::new_with(py, estimated_len, |buffer| {
                    to_py_err!(DecompressionError -> snappy::decompress(bytes.as_bytes(), Output::Slice(buffer)))?;
                    Ok(())
                })?
            } else {
                let mut buffer = Vec::with_capacity(estimated_len);

                to_py_err!(DecompressionError -> snappy::decompress(bytes.as_bytes(), Output::Vector(&mut buffer)))?;
                PyBytes::new(py, &buffer)
            };
            BytesType::Bytes(pybytes)
        }
        BytesType::ByteArray(bytes_array) => unsafe {
            let mut actual_len = 0;
            let pybytes = PyByteArray::new_with(py, estimated_len, |output| {
                actual_len =
                    to_py_err!(DecompressionError -> snappy::decompress(bytes_array.as_bytes(), Output::Slice(output)))?;
                Ok(())
            })?;
            pybytes.resize(actual_len)?;
            BytesType::ByteArray(pybytes)
        },
    };
    Ok(result)
}

/// Snappy compression.
///
/// Python Example
/// --------------
/// ```python
/// >>> _ = snappy_compress(b'some bytes here')
/// >>> _ = snappy_compress(bytearray(b'this avoids double allocation, and thus faster!'))  # <- use bytearray where possible
/// ```
#[pyfunction]
pub fn snappy_compress<'a>(py: Python<'a>, data: BytesType<'a>, output_len: Option<usize>) -> PyResult<BytesType<'a>> {
    // Prefer the user's output_len, fallback to estimate the output len
    let estimated_len = output_len.unwrap_or_else(|| max_compress_len(data.len()));

    let result = match data {
        BytesType::Bytes(bytes) => {
            // user provided the exact output len
            if output_len.is_some() {
                let pybytes = PyBytes::new_with(py, estimated_len, |buffer| {
                    to_py_err!(CompressionError -> snappy::compress(bytes.as_bytes(), Output::Slice(buffer)))?;
                    Ok(())
                })?;
                BytesType::Bytes(pybytes)

            // we can use the estimated length, but need to use buffer as we don't know for sure the length
            } else {
                let mut buffer = Vec::with_capacity(estimated_len);

                to_py_err!(CompressionError -> snappy::compress(bytes.as_bytes(), Output::Vector(&mut buffer)))?;

                let pybytes = PyBytes::new(py, &buffer);
                BytesType::Bytes(pybytes)
            }
        }
        BytesType::ByteArray(bytes_array) => {
            let bytes = unsafe { bytes_array.as_bytes() };
            let mut actual_len = 0;
            let pybytes = PyByteArray::new_with(py, estimated_len, |output| {
                actual_len = to_py_err!(CompressionError -> snappy::compress(bytes, Output::Slice(output)))?;
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
/// >>> snappy_decompress_raw(compressed_raw_bytes)
/// ```
#[pyfunction]
pub fn snappy_decompress_raw<'a>(py: Python<'a>, data: BytesType<'a>) -> PyResult<BytesType<'a>> {
    match data {
        BytesType::Bytes(input) => {
            let out = to_py_err!(DecompressionError -> snappy::decompress_raw(input.as_bytes()))?;
            Ok(BytesType::Bytes(PyBytes::new(py, &out)))
        }
        BytesType::ByteArray(input) => {
            let out = to_py_err!(DecompressionError -> snappy::decompress_raw(unsafe { input.as_bytes() }))?;
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
/// >>> snappy_compress_raw(b'some bytes here')
/// ```
#[pyfunction]
pub fn snappy_compress_raw<'a>(py: Python<'a>, data: BytesType<'a>) -> PyResult<BytesType<'a>> {
    match data {
        BytesType::Bytes(input) => {
            let out = to_py_err!(CompressionError -> snappy::compress_raw(input.as_bytes()))?;
            Ok(BytesType::Bytes(PyBytes::new(py, &out)))
        }
        BytesType::ByteArray(input) => {
            let out = to_py_err!(CompressionError -> snappy::compress_raw(unsafe { input.as_bytes() }))?;
            Ok(BytesType::ByteArray(PyByteArray::new(py, &out)))
        }
    }
}

/// Brotli decompression.
///
/// Python Example
/// --------------
/// ```python
/// >>> brotli_decompress(compressed_bytes)
/// ```
#[pyfunction]
pub fn brotli_decompress<'a>(py: Python<'a>, data: BytesType<'a>, output_len: Option<usize>) -> PyResult<BytesType<'a>> {
    match data {
        BytesType::Bytes(input) => match output_len {
            Some(len) => {
                let pybytes = PyBytes::new_with(py, len, |buffer| {
                    let output = Output::Slice(buffer);
                    to_py_err!(DecompressionError -> brotli::decompress(input.as_bytes(), output))?;
                    Ok(())
                })?;
                Ok(BytesType::Bytes(pybytes))
            }
            None => {
                let mut buffer = Vec::with_capacity(data.len() / 10);
                let output = Output::Vector(&mut buffer);
                to_py_err!(DecompressionError -> brotli::decompress(input.as_bytes(), output))?;
                Ok(BytesType::Bytes(PyBytes::new(py, &buffer)))
            }
        },
        BytesType::ByteArray(input) => match output_len {
            Some(len) => {
                let mut size = 0;
                let pybytes = PyByteArray::new_with(py, len, |buffer| {
                    let output = Output::Slice(buffer);
                    size = to_py_err!(DecompressionError -> brotli::decompress(unsafe { input.as_bytes() }, output))?;
                    Ok(())
                })?;
                pybytes.resize(size)?;
                Ok(BytesType::ByteArray(pybytes))
            }
            None => {
                let mut buffer = Vec::with_capacity(data.len() / 10);
                let output = Output::Vector(&mut buffer);
                to_py_err!(DecompressionError -> brotli::decompress(unsafe { input.as_bytes() }, output))?;
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
/// >>> brotli_compress(b'some bytes here', level=9)  # level defaults to 11
/// ```
#[pyfunction]
pub fn brotli_compress<'a>(
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
                    to_py_err!(CompressionError -> brotli::compress(input.as_bytes(), output, level))?;
                    Ok(())
                })?;
                Ok(BytesType::Bytes(pybytes))
            }
            None => {
                let mut buffer = Vec::with_capacity(data.len() / 10);
                let output = Output::Vector(&mut buffer);
                to_py_err!(CompressionError -> brotli::compress(input.as_bytes(), output, level))?;
                Ok(BytesType::Bytes(PyBytes::new(py, &buffer)))
            }
        },
        BytesType::ByteArray(input) => match output_len {
            Some(len) => {
                let mut size = 0;
                let pybytes = PyByteArray::new_with(py, len, |buffer| {
                    let output = Output::Slice(buffer);
                    size = to_py_err!(CompressionError -> brotli::compress(unsafe { input.as_bytes() }, output, level))?;
                    Ok(())
                })?;
                pybytes.resize(size)?;
                Ok(BytesType::ByteArray(pybytes))
            }
            None => {
                let mut buffer = Vec::with_capacity(data.len() / 10);
                let output = Output::Vector(&mut buffer);
                to_py_err!(CompressionError -> brotli::compress(unsafe { input.as_bytes() }, output, level))?;
                Ok(BytesType::ByteArray(PyByteArray::new(py, &buffer)))
            }
        },
    }
}

/// LZ4 compression.
///
/// Python Example
/// --------------
/// ```python
/// >>> lz4_decompress(compressed_bytes)
/// ```
#[pyfunction]
#[allow(unused_variables)] // TODO: Make use of output_len for lz4
pub fn lz4_decompress<'a>(py: Python<'a>, data: BytesType<'a>, output_len: Option<usize>) -> PyResult<BytesType<'a>> {
    match data {
        BytesType::Bytes(input) => {
            let out = to_py_err!(DecompressionError -> lz4::decompress(input.as_bytes()))?;
            Ok(BytesType::Bytes(PyBytes::new(py, &out)))
        }
        BytesType::ByteArray(input) => {
            let out = to_py_err!(DecompressionError -> lz4::decompress(unsafe { input.as_bytes() }))?;
            Ok(BytesType::ByteArray(PyByteArray::new(py, &out)))
        }
    }
}

/// lZ4 compression.
///
/// Python Example
/// --------------
/// ```python
/// >>> lz4_compress(b'some bytes here')
/// ```
#[pyfunction]
#[allow(unused_variables)]
pub fn lz4_compress<'a>(
    py: Python<'a>,
    data: BytesType<'a>,
    level: Option<u32>,
    output_len: Option<usize>,
) -> PyResult<BytesType<'a>> {
    let level = level.unwrap_or_else(|| 4);

    match data {
        BytesType::Bytes(input) => {
            let out = to_py_err!(CompressionError -> lz4::compress(input.as_bytes(), level))?;
            Ok(BytesType::Bytes(PyBytes::new(py, &out)))
        }
        BytesType::ByteArray(input) => {
            let out = to_py_err!(CompressionError -> lz4::compress(unsafe { input.as_bytes() }, level))?;
            Ok(BytesType::ByteArray(PyByteArray::new(py, &out)))
        }
    }
}

/// Gzip decompression.
///
/// Python Example
/// --------------
/// ```python
/// >>> gzip_decompress(compressed_bytes)
/// ```
#[pyfunction]
pub fn gzip_decompress<'a>(py: Python<'a>, data: BytesType<'a>, output_len: Option<usize>) -> PyResult<BytesType<'a>> {
    match data {
        BytesType::Bytes(input) => match output_len {
            Some(len) => {
                let pybytes = PyBytes::new_with(py, len, |buffer| {
                    let output = Output::Slice(buffer);
                    to_py_err!(DecompressionError -> gzip::decompress(input.as_bytes(), output))?;
                    Ok(())
                })?;
                Ok(BytesType::Bytes(pybytes))
            }
            None => {
                let mut buffer = Vec::with_capacity(data.len() / 10);
                let output = Output::Vector(&mut buffer);
                to_py_err!(DecompressionError -> gzip::decompress(input.as_bytes(), output))?;
                Ok(BytesType::Bytes(PyBytes::new(py, &buffer)))
            }
        },
        BytesType::ByteArray(input) => match output_len {
            Some(len) => {
                let mut size = 0;
                let pybytes = PyByteArray::new_with(py, len, |buffer| {
                    let output = Output::Slice(buffer);
                    size = to_py_err!(DecompressionError -> gzip::decompress(unsafe { input.as_bytes() }, output))?;
                    Ok(())
                })?;
                pybytes.resize(size)?;
                Ok(BytesType::ByteArray(pybytes))
            }
            None => {
                let mut buffer = Vec::with_capacity(data.len() / 10);
                let output = Output::Vector(&mut buffer);
                to_py_err!(DecompressionError -> gzip::decompress(unsafe { input.as_bytes() }, output))?;
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
/// >>> gzip_compress(b'some bytes here', level=2)  # Level defaults to 6
/// ```
#[pyfunction]
pub fn gzip_compress<'a>(
    py: Python<'a>,
    data: BytesType<'a>,
    level: Option<u32>,
    output_len: Option<usize>,
) -> PyResult<BytesType<'a>> {
    let level = level.unwrap_or_else(|| 6);
    match data {
        BytesType::Bytes(input) => match output_len {
            Some(len) => {
                let pybytes = PyBytes::new_with(py, len, |buffer| {
                    let output = Output::Slice(buffer);
                    to_py_err!(CompressionError -> gzip::compress(input.as_bytes(), output, level))?;
                    Ok(())
                })?;
                Ok(BytesType::Bytes(pybytes))
            }
            None => {
                let mut buffer = Vec::with_capacity(data.len() / 10);
                let output = Output::Vector(&mut buffer);
                to_py_err!(CompressionError -> gzip::compress(input.as_bytes(), output, level))?;
                Ok(BytesType::Bytes(PyBytes::new(py, &buffer)))
            }
        },
        BytesType::ByteArray(input) => match output_len {
            Some(len) => {
                let mut size = 0;
                let pybytes = PyByteArray::new_with(py, len, |buffer| {
                    let output = Output::Slice(buffer);
                    size = to_py_err!(CompressionError -> gzip::compress(unsafe { input.as_bytes() }, output, level))?;
                    Ok(())
                })?;
                pybytes.resize(size)?;
                Ok(BytesType::ByteArray(pybytes))
            }
            None => {
                let mut buffer = Vec::with_capacity(data.len() / 10);
                let output = Output::Vector(&mut buffer);
                to_py_err!(CompressionError -> gzip::compress(unsafe { input.as_bytes() }, output, level))?;
                Ok(BytesType::ByteArray(PyByteArray::new(py, &buffer)))
            }
        },
    }
}

/// Deflate decompression.
///
/// Python Example
/// --------------
/// ```python
/// >>> deflate_decompress(compressed_bytes)
/// ```
#[pyfunction]
pub fn deflate_decompress<'a>(
    py: Python<'a>,
    data: BytesType<'a>,
    output_len: Option<usize>,
) -> PyResult<BytesType<'a>> {
    match data {
        BytesType::Bytes(input) => match output_len {
            Some(len) => {
                let pybytes = PyBytes::new_with(py, len, |buffer| {
                    let output = Output::Slice(buffer);
                    to_py_err!(DecompressionError -> deflate::decompress(input.as_bytes(), output))?;
                    Ok(())
                })?;
                Ok(BytesType::Bytes(pybytes))
            }
            None => {
                let mut buffer = Vec::with_capacity(data.len() / 10);
                let output = Output::Vector(&mut buffer);
                to_py_err!(DecompressionError -> deflate::decompress(input.as_bytes(), output))?;
                Ok(BytesType::Bytes(PyBytes::new(py, &buffer)))
            }
        },
        BytesType::ByteArray(input) => match output_len {
            Some(len) => {
                let mut size = 0;
                let pybytes = PyByteArray::new_with(py, len, |buffer| {
                    let output = Output::Slice(buffer);
                    size = to_py_err!(DecompressionError -> deflate::decompress(unsafe { input.as_bytes() }, output))?;
                    Ok(())
                })?;
                pybytes.resize(size)?;
                Ok(BytesType::ByteArray(pybytes))
            }
            None => {
                let mut buffer = Vec::with_capacity(data.len() / 10);
                let output = Output::Vector(&mut buffer);
                to_py_err!(DecompressionError -> deflate::decompress(unsafe { input.as_bytes() }, output))?;
                Ok(BytesType::ByteArray(PyByteArray::new(py, &buffer)))
            }
        },
    }
}

/// Deflate compression.
///
/// Python Example
/// --------------
/// ```python
/// >>> deflate_compress(b'some bytes here', level=5)  # level defaults to 6
/// ```
#[pyfunction]
pub fn deflate_compress<'a>(
    py: Python<'a>,
    data: BytesType<'a>,
    level: Option<u32>,
    output_len: Option<usize>,
) -> PyResult<BytesType<'a>> {
    let level = level.unwrap_or_else(|| 6);
    match data {
        BytesType::Bytes(input) => match output_len {
            Some(len) => {
                let pybytes = PyBytes::new_with(py, len, |buffer| {
                    let output = Output::Slice(buffer);
                    to_py_err!(CompressionError -> deflate::compress(input.as_bytes(), output, level))?;
                    Ok(())
                })?;
                Ok(BytesType::Bytes(pybytes))
            }
            None => {
                let mut buffer = Vec::with_capacity(data.len() / 10);
                let output = Output::Vector(&mut buffer);
                to_py_err!(CompressionError -> deflate::compress(input.as_bytes(), output, level))?;
                Ok(BytesType::Bytes(PyBytes::new(py, &buffer)))
            }
        },
        BytesType::ByteArray(input) => match output_len {
            Some(len) => {
                let mut size = 0;
                let pybytes = PyByteArray::new_with(py, len, |buffer| {
                    let output = Output::Slice(buffer);
                    size =
                        to_py_err!(CompressionError -> deflate::compress(unsafe { input.as_bytes() }, output, level))?;
                    Ok(())
                })?;
                pybytes.resize(size)?;
                Ok(BytesType::ByteArray(pybytes))
            }
            None => {
                let mut buffer = Vec::with_capacity(data.len() / 10);
                let output = Output::Vector(&mut buffer);
                to_py_err!(CompressionError -> deflate::compress(unsafe { input.as_bytes() }, output, level))?;
                Ok(BytesType::ByteArray(PyByteArray::new(py, &buffer)))
            }
        },
    }
}

/// ZSTD decompression.
///
/// Python Example
/// --------------
/// ```python
/// >>> zstd_decompress(compressed_bytes)
/// ```
#[pyfunction]
pub fn zstd_decompress<'a>(py: Python<'a>, data: BytesType<'a>, output_len: Option<usize>) -> PyResult<BytesType<'a>> {
    match data {
        BytesType::Bytes(input) => match output_len {
            Some(len) => {
                let pybytes = PyBytes::new_with(py, len, |buffer| {
                    let output = Output::Slice(buffer);
                    to_py_err!(DecompressionError -> zstd::decompress(input.as_bytes(), output))?;
                    Ok(())
                })?;
                Ok(BytesType::Bytes(pybytes))
            }
            None => {
                let mut buffer = Vec::with_capacity(data.len() / 10);
                let output = Output::Vector(&mut buffer);
                to_py_err!(DecompressionError -> zstd::decompress(input.as_bytes(), output))?;
                Ok(BytesType::Bytes(PyBytes::new(py, &buffer)))
            }
        },
        BytesType::ByteArray(input) => match output_len {
            Some(len) => {
                let mut size = 0;
                let pybytes = PyByteArray::new_with(py, len, |buffer| {
                    let output = Output::Slice(buffer);
                    size = to_py_err!(DecompressionError -> zstd::decompress(unsafe { input.as_bytes() }, output))?;
                    Ok(())
                })?;
                pybytes.resize(size)?;
                Ok(BytesType::ByteArray(pybytes))
            }
            None => {
                let mut buffer = Vec::with_capacity(data.len() / 10);
                let output = Output::Vector(&mut buffer);
                to_py_err!(DecompressionError -> zstd::decompress(unsafe { input.as_bytes() }, output))?;
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
/// >>> zstd_compress(b'some bytes here', level=0)  # level defaults to 11
/// ```
#[pyfunction]
pub fn zstd_compress<'a>(
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
                    to_py_err!(CompressionError -> zstd::compress(input.as_bytes(), output, level))?;
                    Ok(())
                })?;
                Ok(BytesType::Bytes(pybytes))
            }
            None => {
                let mut buffer = Vec::with_capacity(data.len() / 10);
                let output = Output::Vector(&mut buffer);
                to_py_err!(CompressionError -> zstd::compress(input.as_bytes(), output, level))?;
                Ok(BytesType::Bytes(PyBytes::new(py, &buffer)))
            }
        },
        BytesType::ByteArray(input) => match output_len {
            Some(len) => {
                let mut size = 0;
                let pybytes = PyByteArray::new_with(py, len, |buffer| {
                    let output = Output::Slice(buffer);
                    size = to_py_err!(CompressionError -> zstd::compress(unsafe { input.as_bytes() }, output, level))?;
                    Ok(())
                })?;
                pybytes.resize(size)?;
                Ok(BytesType::ByteArray(pybytes))
            }
            None => {
                let mut buffer = Vec::with_capacity(data.len() / 10);
                let output = Output::Vector(&mut buffer);
                to_py_err!(CompressionError -> zstd::compress(unsafe { input.as_bytes() }, output, level))?;
                Ok(BytesType::ByteArray(PyByteArray::new(py, &buffer)))
            }
        },
    }
}

#[pymodule]
fn cramjam(py: Python, m: &PyModule) -> PyResult<()> {
    m.add("CompressionError", py.get_type::<CompressionError>())?;
    m.add("DecompressionError", py.get_type::<DecompressionError>())?;

    m.add_wrapped(wrap_pyfunction!(snappy_compress))?;
    m.add_wrapped(wrap_pyfunction!(snappy_decompress))?;
    m.add_wrapped(wrap_pyfunction!(snappy_compress_raw))?;
    m.add_wrapped(wrap_pyfunction!(snappy_decompress_raw))?;

    m.add_wrapped(wrap_pyfunction!(brotli_compress))?;
    m.add_wrapped(wrap_pyfunction!(brotli_decompress))?;

    m.add_wrapped(wrap_pyfunction!(lz4_compress))?;
    m.add_wrapped(wrap_pyfunction!(lz4_decompress))?;

    m.add_wrapped(wrap_pyfunction!(gzip_compress))?;
    m.add_wrapped(wrap_pyfunction!(gzip_decompress))?;

    m.add_wrapped(wrap_pyfunction!(deflate_compress))?;
    m.add_wrapped(wrap_pyfunction!(deflate_decompress))?;

    m.add_wrapped(wrap_pyfunction!(zstd_compress))?;
    m.add_wrapped(wrap_pyfunction!(zstd_decompress))?;

    Ok(())
}
