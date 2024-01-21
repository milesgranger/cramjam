//! Experimental and unstable implementations.
//! This module makes no effort to maintain SemVer between
//! releases.
use pyo3::prelude::*;
use pyo3::PyResult;

pub(crate) fn init_py_module(m: &PyModule) -> PyResult<()> {
    Python::with_gil(|py| add_experimental_modules(py, m))?;
    Ok(())
}
fn add_experimental_modules(py: Python, m: &PyModule) -> PyResult<()> {
    let lzma_module = PyModule::new(py, "lzma")?;
    lzma::init_py_module(lzma_module)?;
    m.add_submodule(lzma_module)?;
    Ok(())
}

pub mod lzma {
    //! lzma de/compression interface
    use crate::exceptions::{CompressionError, DecompressionError};
    use crate::io::{AsBytes, RustyBuffer};
    use crate::BytesType;
    use pyo3::exceptions::PyNotImplementedError;
    use pyo3::prelude::*;
    use pyo3::wrap_pyfunction;
    use pyo3::PyResult;
    use std::io::Cursor;

    pub(crate) fn init_py_module(m: &PyModule) -> PyResult<()> {
        m.add_function(wrap_pyfunction!(compress, m)?)?;
        m.add_function(wrap_pyfunction!(decompress, m)?)?;
        m.add_function(wrap_pyfunction!(compress_into, m)?)?;
        m.add_function(wrap_pyfunction!(decompress_into, m)?)?;
        m.add_class::<Compressor>()?;
        m.add_class::<Decompressor>()?;
        Ok(())
    }
    /// LZMA decompression.
    ///
    /// Python Example
    /// --------------
    /// ```python
    /// >>> # bytes or bytearray; bytearray is faster
    /// >>> cramjam.experimental.lzma.decompress(compressed_bytes, output_len=Optional[None])
    /// ```
    #[pyfunction]
    pub fn decompress(py: Python, data: BytesType, output_len: Option<usize>) -> PyResult<RustyBuffer> {
        crate::generic!(py, libcramjam::lzma::decompress[data], output_len = output_len)
            .map_err(DecompressionError::from_err)
    }

    /// LZMA compression.
    ///
    /// Python Example
    /// --------------
    /// ```python
    /// >>> _ = cramjam.experimental.lzma.compress(b'some bytes here')
    /// ```
    #[pyfunction]
    pub fn compress(
        py: Python,
        data: BytesType,
        preset: Option<u32>,
        output_len: Option<usize>,
    ) -> PyResult<RustyBuffer> {
        crate::generic!(
            py,
            libcramjam::lzma::compress[data],
            output_len = output_len,
            level = preset
        )
        .map_err(CompressionError::from_err)
    }

    /// Compress directly into an output buffer
    #[pyfunction]
    pub fn compress_into(py: Python, input: BytesType, mut output: BytesType, preset: Option<u32>) -> PyResult<usize> {
        crate::generic!(py, libcramjam::lzma::compress[input, output], level = preset)
            .map_err(CompressionError::from_err)
    }

    /// Decompress directly into an output buffer
    #[pyfunction]
    pub fn decompress_into(py: Python, input: BytesType, mut output: BytesType) -> PyResult<usize> {
        crate::generic!(py, libcramjam::lzma::decompress[input, output]).map_err(DecompressionError::from_err)
    }
    /// Snappy Compressor object for streaming compression
    #[pyclass]
    pub struct Compressor {
        inner: Option<libcramjam::lzma::xz2::write::XzEncoder<Cursor<Vec<u8>>>>,
    }

    #[pymethods]
    impl Compressor {
        /// Initialize a new `Compressor` instance.
        #[new]
        pub fn __init__(preset: Option<u32>) -> PyResult<Self> {
            let preset = preset.unwrap_or(5);
            let inner = libcramjam::lzma::xz2::write::XzEncoder::new(Cursor::new(vec![]), preset);
            Ok(Self { inner: Some(inner) })
        }

        /// Compress input into the current compressor's stream.
        pub fn compress(&mut self, input: &[u8]) -> PyResult<usize> {
            crate::io::stream_compress(&mut self.inner, input)
        }

        /// Flush and return current compressed stream
        pub fn flush(&mut self) -> PyResult<RustyBuffer> {
            Err(PyNotImplementedError::new_err(
                "`.flush` for LZMA not implemented, just use `.finish()` instead when your done.",
            ))
        }

        /// Consume the current compressor state and return the compressed stream
        /// **NB** The compressor will not be usable after this method is called.
        pub fn finish(&mut self) -> PyResult<RustyBuffer> {
            crate::io::stream_finish(&mut self.inner, |inner| inner.finish().map(|c| c.into_inner()))
        }
    }

    crate::make_decompressor!(lzma);
}
