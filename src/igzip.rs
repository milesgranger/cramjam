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

    /// IGZIP Compressor object for streaming compression
    #[pyclass(unsendable)] // TODO: make sendable
    pub struct Compressor {
        inner: Option<libcramjam::igzip::isal::write::GzipEncoder<Cursor<Vec<u8>>>>,
    }

    #[pymethods]
    impl Compressor {
        /// Initialize a new `Compressor` instance.
        #[new]
        #[pyo3(signature = (level=None))]
        pub fn __init__(level: Option<u32>) -> PyResult<Self> {
            let level = level.unwrap_or(DEFAULT_COMPRESSION_LEVEL);
            let inner = libcramjam::igzip::isal::write::GzipEncoder::new(
                Cursor::new(vec![]),
                libcramjam::igzip::isal::CompressionLevel::try_from(level as isize)
                    .map_err(CompressionError::from_err)?,
            );
            Ok(Self { inner: Some(inner) })
        }

        /// Compress input into the current compressor's stream.
        pub fn compress(&mut self, input: &[u8]) -> PyResult<usize> {
            crate::io::stream_compress(&mut self.inner, input)
        }

        /// Flush and return current compressed stream
        pub fn flush(&mut self) -> PyResult<RustyBuffer> {
            crate::io::stream_flush(&mut self.inner, |e| e.get_ref_mut())
        }

        /// Consume the current compressor state and return the compressed stream
        /// **NB** The compressor will not be usable after this method is called.
        pub fn finish(&mut self) -> PyResult<RustyBuffer> {
            crate::io::stream_finish(&mut self.inner, |inner| inner.finish().map(|c| c.into_inner()))
        }
    }

    mod _decompressor {
        use super::*;
        crate::make_decompressor!(gzip);
    }
    #[pymodule_export]
    use _decompressor::Decompressor;
}
