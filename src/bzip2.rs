//! bzip2 de/compression interface
use pyo3::prelude::*;

/// bzip2 de/compression interface
#[pymodule]
pub mod bzip2 {

    use crate::exceptions::{CompressionError, DecompressionError};
    use crate::io::RustyBuffer;
    use crate::{AsBytes, BytesType};
    use pyo3::prelude::*;
    use pyo3::PyResult;
    use std::io::Cursor;

    const DEFAULT_COMPRESSION_LEVEL: u32 = 6;

    /// bzip2 decompression.
    ///
    /// Python Example
    /// --------------
    /// ```python
    /// >>> cramjam.bzip2.decompress(compressed_bytes, output_len=Optional[int])
    /// ```
    #[pyfunction]
    #[pyo3(signature = (data, output_len=None))]
    pub fn decompress(py: Python, data: BytesType, output_len: Option<usize>) -> PyResult<RustyBuffer> {
        crate::generic!(py, libcramjam::bzip2::decompress[data], output_len = output_len)
            .map_err(DecompressionError::from_err)
    }

    /// bzip2 compression.
    ///
    /// Python Example
    /// --------------
    /// ```python
    /// >>> cramjam.bzip2.compress(b'some bytes here', level=6, output_len=Option[int])  # level defaults to 6
    /// ```
    #[pyfunction]
    #[pyo3(signature = (data, level=None, output_len=None))]
    pub fn compress(
        py: Python,
        data: BytesType,
        level: Option<u32>,
        output_len: Option<usize>,
    ) -> PyResult<RustyBuffer> {
        crate::generic!(py, libcramjam::bzip2::compress[data], output_len = output_len, level)
            .map_err(CompressionError::from_err)
    }

    /// Compress directly into an output buffer
    #[pyfunction]
    #[pyo3(signature = (input, output, level=None))]
    pub fn compress_into(py: Python, input: BytesType, mut output: BytesType, level: Option<u32>) -> PyResult<usize> {
        crate::generic!(py, libcramjam::bzip2::compress[input, output], level).map_err(CompressionError::from_err)
    }

    /// Decompress directly into an output buffer
    #[pyfunction]
    pub fn decompress_into(py: Python, input: BytesType, mut output: BytesType) -> PyResult<usize> {
        crate::generic!(py, libcramjam::bzip2::decompress[input, output]).map_err(DecompressionError::from_err)
    }

    /// bzip2 Compressor object for streaming compression
    #[pyclass]
    pub struct Compressor {
        inner: Option<libcramjam::bzip2::bzip2::write::BzEncoder<Cursor<Vec<u8>>>>,
    }

    #[pymethods]
    impl Compressor {
        /// Initialize a new `Compressor` instance.
        #[new]
        #[pyo3(signature = (level=None))]
        pub fn __init__(level: Option<u32>) -> PyResult<Self> {
            let level = level.unwrap_or_else(|| DEFAULT_COMPRESSION_LEVEL);
            let comp = libcramjam::bzip2::bzip2::Compression::new(level);
            let inner = libcramjam::bzip2::bzip2::write::BzEncoder::new(Cursor::new(vec![]), comp);
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
            crate::io::stream_finish(&mut self.inner, |inner| inner.finish().map(|c| c.into_inner()))
        }
    }

    mod _decompressor {
        use super::*;
        crate::make_decompressor!(bzip2);
    }
    #[pymodule_export]
    use _decompressor::Decompressor;
}
