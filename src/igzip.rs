//! igzip de/compression interface
use pyo3::prelude::*;

/// igzip de/compression interface
#[pymodule]
pub mod igzip {

    use crate::exceptions::{CompressionError, DecompressionError};
    use crate::io::{AsBytes, RustyBuffer};
    use crate::BytesType;
    use pyo3::prelude::*;
    use pyo3::PyResult;
    use std::io::Cursor;

    const DEFAULT_COMPRESSION_LEVEL: u32 = 6;

    /// IGzip decompression.
    ///
    /// Python Example
    /// --------------
    /// ```python
    /// >>> cramjam.gzip.decompress(compressed_bytes, output_len=Optional[int])
    /// ```
    #[pyfunction]
    #[pyo3(signature = (data, output_len=None))]
    pub fn decompress(py: Python, data: BytesType, output_len: Option<usize>) -> PyResult<RustyBuffer> {
        crate::generic!(py, libcramjam::igzip::decompress[data], output_len = output_len)
            .map_err(DecompressionError::from_err)
    }

    /// IGzip compression.
    ///
    /// Python Example
    /// --------------
    /// ```python
    /// >>> cramjam.gzip.compress(b'some bytes here', level=2, output_len=Optional[int])  # Level defaults to 6
    /// ```
    #[pyfunction]
    #[pyo3(signature = (data, level=None, output_len=None))]
    pub fn compress(
        py: Python,
        data: BytesType,
        level: Option<u32>,
        output_len: Option<usize>,
    ) -> PyResult<RustyBuffer> {
        crate::generic!(py, libcramjam::igzip::compress[data], output_len = output_len, level)
            .map_err(CompressionError::from_err)
    }

    /// Compress directly into an output buffer
    #[pyfunction]
    #[pyo3(signature = (input, output, level=None))]
    pub fn compress_into(py: Python, input: BytesType, mut output: BytesType, level: Option<u32>) -> PyResult<usize> {
        crate::generic!(py, libcramjam::igzip::compress[input, output], level).map_err(CompressionError::from_err)
    }

    /// Decompress directly into an output buffer
    #[pyfunction]
    pub fn decompress_into(py: Python, input: BytesType, mut output: BytesType) -> PyResult<usize> {
        crate::generic!(py, libcramjam::igzip::decompress[input, output]).map_err(DecompressionError::from_err)
    }

}
