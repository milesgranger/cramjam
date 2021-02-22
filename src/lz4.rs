use crate::exceptions::{CompressionError, DecompressionError};
use crate::{to_py_err, BytesType};
use pyo3::prelude::*;
use pyo3::types::{PyByteArray, PyBytes};
use pyo3::wrap_pyfunction;
use pyo3::{PyResult, Python};

pub fn init_py_module(m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(compress, m)?)?;
    m.add_function(wrap_pyfunction!(decompress, m)?)?;
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
    match data {
        BytesType::Bytes(input) => {
            let out = to_py_err!(DecompressionError -> self::internal::decompress(input.as_bytes()))?;
            Ok(BytesType::Bytes(PyBytes::new(py, &out)))
        }
        BytesType::ByteArray(input) => {
            let out = to_py_err!(DecompressionError -> self::internal::decompress(unsafe { input.as_bytes() }))?;
            Ok(BytesType::ByteArray(PyByteArray::new(py, &out)))
        }
    }
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
#[allow(unused_variables)]
pub fn compress<'a>(
    py: Python<'a>,
    data: BytesType<'a>,
    level: Option<u32>,
    output_len: Option<usize>,
) -> PyResult<BytesType<'a>> {
    match data {
        BytesType::Bytes(input) => {
            let out = to_py_err!(CompressionError -> self::internal::compress(input.as_bytes(), level))?;
            Ok(BytesType::Bytes(PyBytes::new(py, &out)))
        }
        BytesType::ByteArray(input) => {
            let out = to_py_err!(CompressionError -> self::internal::compress(unsafe { input.as_bytes() }, level))?;
            Ok(BytesType::ByteArray(PyByteArray::new(py, &out)))
        }
    }
}

pub(crate) mod internal {
    use std::error::Error;

    /// Decompress lz4 data
    pub fn decompress(data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
        lz_fear::framed::decompress_frame(data).map_err(|err| err.into())
    }

    /// Compress lz4 data
    // TODO: lz-fear does not yet support level
    pub fn compress(data: &[u8], level: Option<u32>) -> Result<Vec<u8>, Box<dyn Error>> {
        let _ = level.unwrap_or_else(|| 4);
        let mut buf = vec![];
        lz_fear::framed::CompressionSettings::default().compress(data, &mut buf)?;
        Ok(buf)
    }
}
