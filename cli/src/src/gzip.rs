//! gzip de/compression interface
use crate::exceptions::{CompressionError, DecompressionError};
use crate::io::{AsBytes, RustyBuffer};
use crate::BytesType;
use pyo3::prelude::*;
use pyo3::wrap_pyfunction;
use pyo3::PyResult;
use std::io::Cursor;

const DEFAULT_COMPRESSION_LEVEL: u32 = 6;

pub(crate) fn init_py_module(m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(compress, m)?)?;
    m.add_function(wrap_pyfunction!(decompress, m)?)?;
    m.add_function(wrap_pyfunction!(compress_into, m)?)?;
    m.add_function(wrap_pyfunction!(decompress_into, m)?)?;
    m.add_class::<Compressor>()?;
    m.add_class::<Decompressor>()?;
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
pub fn decompress(py: Python, data: BytesType, output_len: Option<usize>) -> PyResult<RustyBuffer> {
    crate::generic!(py, internal::decompress[data], output_len = output_len).map_err(DecompressionError::from_err)
}

/// Gzip compression.
///
/// Python Example
/// --------------
/// ```python
/// >>> cramjam.gzip.compress(b'some bytes here', level=2, output_len=Optional[int])  # Level defaults to 6
/// ```
#[pyfunction]
pub fn compress(py: Python, data: BytesType, level: Option<u32>, output_len: Option<usize>) -> PyResult<RustyBuffer> {
    crate::generic!(py, internal::compress[data], output_len = output_len, level = level)
        .map_err(CompressionError::from_err)
}

/// Compress directly into an output buffer
#[pyfunction]
pub fn compress_into(py: Python, input: BytesType, mut output: BytesType, level: Option<u32>) -> PyResult<usize> {
    crate::generic!(py, internal::compress[input, output], level = level).map_err(CompressionError::from_err)
}

/// Decompress directly into an output buffer
#[pyfunction]
pub fn decompress_into(py: Python, input: BytesType, mut output: BytesType) -> PyResult<usize> {
    crate::generic!(py, internal::decompress[input, output]).map_err(DecompressionError::from_err)
}

/// GZIP Compressor object for streaming compression
#[pyclass]
pub struct Compressor {
    inner: Option<flate2::write::GzEncoder<Cursor<Vec<u8>>>>,
}

#[pymethods]
impl Compressor {
    /// Initialize a new `Compressor` instance.
    #[new]
    pub fn __init__(level: Option<u32>) -> PyResult<Self> {
        let level = level.unwrap_or(DEFAULT_COMPRESSION_LEVEL);
        let inner = flate2::write::GzEncoder::new(Cursor::new(vec![]), flate2::Compression::new(level));
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

crate::make_decompressor!();

pub(crate) mod internal {
    use crate::gzip::DEFAULT_COMPRESSION_LEVEL;
    use flate2::read::{GzEncoder, MultiGzDecoder};
    use flate2::Compression;
    use std::io::prelude::*;
    use std::io::{Cursor, Error};

    /// Decompress gzip data
    #[inline(always)]
    pub fn decompress<W: Write + ?Sized, R: Read>(input: R, output: &mut W) -> Result<usize, Error> {
        let mut decoder = MultiGzDecoder::new(input);
        let mut out = vec![];
        let n_bytes = decoder.read_to_end(&mut out)?;
        std::io::copy(&mut Cursor::new(out.as_slice()), output)?;
        Ok(n_bytes as usize)
    }

    /// Compress gzip data
    #[inline(always)]
    pub fn compress<W: Write + ?Sized, R: Read>(input: R, output: &mut W, level: Option<u32>) -> Result<usize, Error> {
        let level = level.unwrap_or_else(|| DEFAULT_COMPRESSION_LEVEL);
        let mut encoder = GzEncoder::new(input, Compression::new(level));
        let n_bytes = std::io::copy(&mut encoder, output)?;
        Ok(n_bytes as usize)
    }

    #[cfg(test)]
    mod tests {

        #[test]
        fn test_gzip_multiple_streams() {
            let mut out1 = vec![];
            let mut out2 = vec![];
            super::compress(b"foo".to_vec().as_slice(), &mut out1, None).unwrap();
            super::compress(b"bar".to_vec().as_slice(), &mut out2, None).unwrap();

            let mut out3 = vec![];
            out1.extend_from_slice(&out2);
            super::decompress(out1.as_slice(), &mut out3).unwrap();
            assert_eq!(out3, b"foobar".to_vec());
        }
    }
}
