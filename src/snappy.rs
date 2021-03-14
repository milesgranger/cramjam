use crate::exceptions::{CompressionError, DecompressionError};
use crate::{to_py_err, BytesType};
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3::wrap_pyfunction;
use pyo3::{PyResult, Python};
use std::io::Cursor;

pub fn init_py_module(m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(compress, m)?)?;
    m.add_function(wrap_pyfunction!(decompress, m)?)?;
    m.add_function(wrap_pyfunction!(compress_raw, m)?)?;
    m.add_function(wrap_pyfunction!(decompress_raw, m)?)?;
    m.add_function(wrap_pyfunction!(compress_into, m)?)?;
    m.add_function(wrap_pyfunction!(decompress_into, m)?)?;
    m.add_function(wrap_pyfunction!(compress_raw_into, m)?)?;
    m.add_function(wrap_pyfunction!(decompress_raw_into, m)?)?;
    m.add_function(wrap_pyfunction!(compress_raw_max_len, m)?)?;
    m.add_function(wrap_pyfunction!(decompress_raw_len, m)?)?;
    Ok(())
}

/// Snappy decompression.
///
/// Python Example
/// --------------
/// ```python
/// >>> # bytes or bytearray; bytearray is faster
/// >>> cramjam.snappy.decompress(compressed_bytes, output_len=Optional[None])
/// ```
#[pyfunction]
pub fn decompress<'a>(py: Python<'a>, data: BytesType<'a>, output_len: Option<usize>) -> PyResult<BytesType<'a>> {
    crate::generic!(decompress(data), py = py, output_len = output_len)
}

/// Snappy compression.
///
/// Python Example
/// --------------
/// ```python
/// >>> _ = cramjam.snappy.compress(b'some bytes here')
/// >>> _ = cramjam.snappy.compress(bytearray(b'this avoids double allocation in rust side, and thus faster!'))  # <- use bytearray where possible
/// ```
#[pyfunction]
pub fn compress<'a>(py: Python<'a>, data: BytesType<'a>, output_len: Option<usize>) -> PyResult<BytesType<'a>> {
    crate::generic!(compress(data), py = py, output_len = output_len)
}

/// Snappy decompression, raw
/// This does not use the snappy 'framed' encoding of compressed bytes.
///
/// Python Example
/// --------------
/// ```python
/// >>> cramjam.snappy.decompress_raw(compressed_raw_bytes)
/// ```
#[pyfunction]
pub fn decompress_raw<'a>(py: Python<'a>, data: BytesType<'a>, output_len: Option<usize>) -> PyResult<BytesType<'a>> {
    crate::generic!(decompress_raw(data), py = py, output_len = output_len)
}

/// Snappy compression raw.
/// This does not use the snappy 'framed' encoding of compressed bytes.
///
/// Python Example
/// --------------
/// ```python
/// >>> cramjam.snappy.compress_raw(b'some bytes here')
/// ```
#[pyfunction]
pub fn compress_raw<'a>(py: Python<'a>, data: BytesType<'a>, output_len: Option<usize>) -> PyResult<BytesType<'a>> {
    crate::generic!(compress_raw(data), py = py, output_len = output_len)
}

/// Compress directly into an output buffer
#[pyfunction]
pub fn compress_into<'a>(_py: Python<'a>, input: BytesType<'a>, mut output: BytesType<'a>) -> PyResult<usize> {
    let r = internal::compress(input, &mut output)?;
    Ok(r)
}

/// Decompress directly into an output buffer
#[pyfunction]
pub fn decompress_into<'a>(_py: Python<'a>, input: BytesType<'a>, mut output: BytesType<'a>) -> PyResult<usize> {
    let r = internal::decompress(input, &mut output)?;
    Ok(r as usize)
}

/// Compress raw format directly into an output buffer
#[pyfunction]
pub fn compress_raw_into<'a>(_py: Python<'a>, input: BytesType<'a>, mut output: BytesType<'a>) -> PyResult<usize> {
    let r = to_py_err!(CompressionError -> internal::compress_raw(input, &mut output))?;
    Ok(r)
}

/// Decompress raw format directly into an output buffer
#[pyfunction]
pub fn decompress_raw_into<'a>(_py: Python<'a>, input: BytesType<'a>, mut output: BytesType<'a>) -> PyResult<usize> {
    let r = to_py_err!(DecompressionError -> internal::decompress_raw(input, &mut output))?;
    Ok(r)
}

/// Get the expected max compressed length for snappy raw compression; this is the size
/// of buffer that should be passed to `compress_raw_into`
#[pyfunction]
pub fn compress_raw_max_len<'a>(_py: Python<'a>, data: BytesType<'a>) -> usize {
    snap::raw::max_compress_len(data.len())
}

/// Get the decompressed length for the given data. This is the size of buffer
/// that should be passed to `decompress_raw_into`
#[pyfunction]
pub fn decompress_raw_len<'a>(_py: Python<'a>, data: BytesType<'a>) -> PyResult<usize> {
    to_py_err!(DecompressionError -> snap::raw::decompress_len(data.as_bytes()))
}

pub(crate) mod internal {
    use snap::raw::{decompress_len, max_compress_len, Decoder, Encoder};
    use snap::read::{FrameDecoder, FrameEncoder};
    use std::io::{Cursor, Error, Read, Write};

    pub(crate) struct RawEncoder<R: Read> {
        inner: R,
        buffer: Vec<u8>,             // raw data read from inner
        compressed: Cursor<Vec<u8>>, // compressed data.
        encoder: Encoder,
    }
    impl<R: Read> RawEncoder<R> {
        pub fn new(inner: R) -> Self {
            Self {
                inner,
                buffer: Vec::new(),
                compressed: Cursor::new(Vec::new()),
                encoder: Encoder::new(),
            }
        }
    }
    impl<R: Read> Read for RawEncoder<R> {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if self.buffer.is_empty() {
                let n = self.inner.read_to_end(&mut self.buffer)?;
                self.buffer.truncate(n);
                let len = max_compress_len(self.buffer.len());
                self.compressed.get_mut().resize(len, 0);
                let n = self
                    .encoder
                    .compress(self.buffer.as_slice(), self.compressed.get_mut())?;
                self.compressed.get_mut().truncate(n);
                self.compressed.set_position(0);
            }
            self.compressed.read(buf)
        }
    }

    pub(crate) struct RawDecoder<R: Read> {
        inner: R,
        buffer: Vec<u8>,               // raw data read from inner
        decompressed: Cursor<Vec<u8>>, // decompressed data.
        decoder: Decoder,
    }
    impl<R: Read> RawDecoder<R> {
        pub fn new(inner: R) -> Self {
            Self {
                inner,
                buffer: Vec::with_capacity(64000),
                decompressed: Cursor::new(Vec::with_capacity(64000)),
                decoder: Decoder::new(),
            }
        }
    }
    impl<R: Read> Read for RawDecoder<R> {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if self.buffer.is_empty() {
                let n = self.inner.read_to_end(&mut self.buffer)?;
                self.buffer.truncate(n);
                let len = decompress_len(&self.buffer)?;
                self.decompressed.get_mut().resize(len, 0);
                let n = self
                    .decoder
                    .decompress(self.buffer.as_slice(), self.decompressed.get_mut())?;
                self.decompressed.get_mut().truncate(n);
                self.decompressed.set_position(0);
            }
            self.decompressed.read(buf)
        }
    }

    /// Decompress snappy data raw into a mutable slice
    pub fn decompress_raw<W: Write + ?Sized, R: Read>(input: R, output: &mut W) -> std::io::Result<usize> {
        let mut decoder = RawDecoder::new(input);
        let n_bytes = std::io::copy(&mut decoder, output)?;
        Ok(n_bytes as usize)
    }

    /// Compress snappy data raw into a mutable slice
    pub fn compress_raw<W: Write + ?Sized, R: Read>(input: R, output: &mut W) -> std::io::Result<usize> {
        let mut encoder = RawEncoder::new(input);
        let n_bytes = std::io::copy(&mut encoder, output)?;
        Ok(n_bytes as usize)
    }

    /// Decompress snappy data framed
    pub fn decompress<W: Write + ?Sized, R: Read>(input: R, output: &mut W) -> Result<usize, Error> {
        let mut decoder = FrameDecoder::new(input);
        let n_bytes = std::io::copy(&mut decoder, output)?;
        Ok(n_bytes as usize)
    }

    /// Decompress snappy data framed
    pub fn compress<W: Write + ?Sized, R: Read>(data: R, output: &mut W) -> Result<usize, Error> {
        let mut encoder = FrameEncoder::new(data);
        let n_bytes = std::io::copy(&mut encoder, output)?;
        Ok(n_bytes as usize)
    }
}
