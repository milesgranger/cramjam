//! zstd de/compression interface
use pyo3::prelude::*;

/// zstd de/compression interface
#[pymodule]
pub mod zstd {
    use crate::exceptions::{CompressionError, DecompressionError};
    use crate::io::RustyBuffer;
    use crate::{AsBytes, BytesType};
    use pyo3::prelude::*;
    use pyo3::PyResult;
    use std::io::Cursor;

    const DEFAULT_COMPRESSION_LEVEL: i32 = 0;

    /// ZSTD decompression.
    ///
    /// Python Example
    /// --------------
    /// ```python
    /// >>> cramjam.zstd.decompress(compressed_bytes, output_len=Optional[int])
    /// ```
    #[pyfunction]
    #[pyo3(signature = (data, output_len=None))]
    pub fn decompress(py: Python, data: BytesType, output_len: Option<usize>) -> PyResult<RustyBuffer> {
        crate::generic!(py, libcramjam::zstd::decompress[data], output_len = output_len)
            .map_err(DecompressionError::from_err)
    }

    /// ZSTD compression.
    ///
    /// Python Example
    /// --------------
    /// ```python
    /// >>> cramjam.zstd.compress(b'some bytes here', level=0, output_len=Optional[int])  # level defaults to 11
    /// ```
    #[pyfunction]
    #[pyo3(signature = (data, level=None, output_len=None))]
    pub fn compress(
        py: Python,
        data: BytesType,
        level: Option<i32>,
        output_len: Option<usize>,
    ) -> PyResult<RustyBuffer> {
        crate::generic!(py, libcramjam::zstd::compress[data], output_len = output_len, level)
            .map_err(CompressionError::from_err)
    }

    /// Compress directly into an output buffer
    #[pyfunction]
    #[pyo3(signature = (input, output, level=None))]
    pub fn compress_into(py: Python, input: BytesType, mut output: BytesType, level: Option<i32>) -> PyResult<usize> {
        crate::generic!(py, libcramjam::zstd::compress[input, output], level).map_err(CompressionError::from_err)
    }

    /// Decompress directly into an output buffer
    #[pyfunction]
    pub fn decompress_into<'a>(py: Python<'a>, input: BytesType<'a>, mut output: BytesType<'a>) -> PyResult<usize> {
        crate::generic!(py, libcramjam::zstd::decompress[input, output]).map_err(DecompressionError::from_err)
    }

    /// ZSTD Compressor object for streaming compression
    #[pyclass]
    pub struct Compressor {
        inner: Option<libcramjam::zstd::zstd::stream::write::Encoder<'static, Cursor<Vec<u8>>>>,
    }

    #[pymethods]
    impl Compressor {
        /// Initialize a new `Compressor` instance.
        #[new]
        #[pyo3(signature = (level=None))]
        pub fn __init__(level: Option<i32>) -> PyResult<Self> {
            let inner = libcramjam::zstd::zstd::stream::write::Encoder::new(
                Cursor::new(vec![]),
                level.unwrap_or(DEFAULT_COMPRESSION_LEVEL),
            )?;
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
            crate::io::stream_finish(&mut self.inner, |inner| inner.finish().map(|v| v.into_inner()))
        }
    }

    mod _decompressor {
        use super::*;
        crate::make_decompressor!(zstd);
    }
    #[pymodule_export]
    use _decompressor::Decompressor;
}
