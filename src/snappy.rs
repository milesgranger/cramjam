//! snappy de/compression interface
use crate::exceptions::{CompressionError, DecompressionError};
use crate::io::{AsBytes, RustyBuffer};
use crate::BytesType;
use pyo3::prelude::*;
use pyo3::wrap_pyfunction;
use pyo3::PyResult;
use std::io::Cursor;

pub(crate) fn init_py_module(m: &PyModule) -> PyResult<()> {
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
    m.add_class::<Compressor>()?;
    m.add_class::<Decompressor>()?;
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
pub fn decompress(py: Python, data: BytesType, output_len: Option<usize>) -> PyResult<RustyBuffer> {
    crate::generic!(py, internal::decompress[data], output_len = output_len)
        .map_err(DecompressionError::from_err::<snap::Error>)
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
pub fn compress(py: Python, data: BytesType, output_len: Option<usize>) -> PyResult<RustyBuffer> {
    crate::generic!(py, internal::compress[data], output_len = output_len)
        .map_err(CompressionError::from_err::<snap::Error>)
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
#[allow(unused_variables)]
pub fn decompress_raw(py: Python, data: BytesType, output_len: Option<usize>) -> PyResult<RustyBuffer> {
    let bytes = data.as_bytes();
    py.allow_threads(|| snap::raw::Decoder::new().decompress_vec(bytes))
        .map_err(DecompressionError::from_err)
        .map(From::from)
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
#[allow(unused_variables)]
pub fn compress_raw(py: Python, data: BytesType, output_len: Option<usize>) -> PyResult<RustyBuffer> {
    let bytes = data.as_bytes();
    py.allow_threads(|| snap::raw::Encoder::new().compress_vec(bytes))
        .map_err(CompressionError::from_err)
        .map(From::from)
}

/// Compress directly into an output buffer
#[pyfunction]
pub fn compress_into(py: Python, input: BytesType, mut output: BytesType) -> PyResult<usize> {
    crate::generic!(py, internal::compress[input, output]).map_err(CompressionError::from_err)
}

/// Decompress directly into an output buffer
#[pyfunction]
pub fn decompress_into(py: Python, input: BytesType, mut output: BytesType) -> PyResult<usize> {
    crate::generic!(py, internal::decompress[input, output]).map_err(DecompressionError::from_err)
}

/// Compress raw format directly into an output buffer
#[pyfunction]
pub fn compress_raw_into(py: Python, input: BytesType, mut output: BytesType) -> PyResult<usize> {
    let bytes_in = input.as_bytes();
    let bytes_out = output.as_bytes_mut();
    py.allow_threads(|| snap::raw::Encoder::new().compress(bytes_in, bytes_out))
        .map_err(CompressionError::from_err)
}

/// Decompress raw format directly into an output buffer
#[pyfunction]
pub fn decompress_raw_into(py: Python, input: BytesType, mut output: BytesType) -> PyResult<usize> {
    let bytes_in = input.as_bytes();
    let bytes_out = output.as_bytes_mut();
    py.allow_threads(|| snap::raw::Decoder::new().decompress(bytes_in, bytes_out))
        .map_err(DecompressionError::from_err)
}

/// Get the expected max compressed length for snappy raw compression; this is the size
/// of buffer that should be passed to `compress_raw_into`
#[pyfunction]
pub fn compress_raw_max_len(data: BytesType) -> usize {
    snap::raw::max_compress_len(data.len())
}

/// Get the decompressed length for the given data. This is the size of buffer
/// that should be passed to `decompress_raw_into`
#[pyfunction]
pub fn decompress_raw_len(data: BytesType) -> PyResult<usize> {
    snap::raw::decompress_len(data.as_bytes()).map_err(DecompressionError::from_err)
}

/// Snappy Compressor object for streaming compression
#[pyclass]
pub struct Compressor {
    inner: Option<snap::write::FrameEncoder<Cursor<Vec<u8>>>>,
}

#[pymethods]
impl Compressor {
    /// Initialize a new `Compressor` instance.
    #[new]
    pub fn __init__() -> PyResult<Self> {
        let inner = snap::write::FrameEncoder::new(Cursor::new(vec![]));
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
        crate::io::stream_finish(&mut self.inner, |inner| inner.into_inner().map(|c| c.into_inner()))
    }
}

crate::make_decompressor!();

// /// Snappy Decompressor object for streaming compression
// /// **NB** This is mostly here for API complement to `Compressor`
// /// You'll almost always be statisfied with `de/compress` / `de/compress_into` functions.
// #[pyclass]
// pub struct Decompressor {
//     inner: Option<snap::read::FrameDecoder<Cursor<Vec<u8>>>>,
// }

// #[pymethods]
// impl Decompressor {
//     /// Initialize a new `Decompressor` instance.
//     #[new]
//     pub fn __init__() -> PyResult<Self> {
//         let inner = snap::read::FrameDecoder::new(Default::default());
//         Ok(Self { inner: Some(inner) })
//     }

//     /// Write contents of `BytesType` into inner buffer
//     // Note: This doesn't actually perform any decompression, that'll be done in flush/finish methods
//     pub fn decompress(&mut self, mut input: BytesType) -> PyResult<usize> {
//         match self.inner.as_mut() {
//             Some(ref mut inner) => std::io::copy(&mut input, inner.get_mut().get_mut())
//                 .map(|v| v as usize)
//                 .map_err(Into::into),
//             None => Err(DecompressionError::new_err(
//                 "Appears `finish()` was called on this instance",
//             )),
//         }
//     }

//     /// Flush and return current decompressed stream.
//     pub fn flush(&mut self, py: Python) -> PyResult<RustyBuffer> {
//         match self.inner.as_mut() {
//             Some(ref mut inner) => py
//                 .allow_threads(|| {
//                     let mut out = vec![];
//                     std::io::copy(inner, &mut out)?;
//                     Ok(out)
//                 })
//                 .map(RustyBuffer::from),
//             None => Err(DecompressionError::new_err(
//                 "Appears `finish()` was called on this instance",
//             )),
//         }
//     }

//     /// Consume the current Decompressor state and return the decompressed stream
//     /// **NB** The Decompressor will not be usable after this method is called.
//     pub fn finish(&mut self, py: Python) -> PyResult<RustyBuffer> {
//         match std::mem::take(&mut self.inner) {
//             Some(mut inner) => py
//                 .allow_threads(|| {
//                     let mut out = vec![];
//                     std::io::copy(&mut inner, &mut out)?;
//                     Ok(out)
//                 })
//                 .map(RustyBuffer::from),
//             None => Err(DecompressionError::new_err(
//                 "Appears `finish()` was called on this instance",
//             )),
//         }
//     }
// }

pub(crate) mod internal {
    use snap::read::{FrameDecoder, FrameEncoder};
    use std::io::{Error, Read, Write};

    /// Decompress snappy data framed
    #[inline(always)]
    pub fn decompress<W: Write + ?Sized, R: Read>(input: R, output: &mut W) -> Result<usize, Error> {
        let mut decoder = FrameDecoder::new(input);
        let n_bytes = std::io::copy(&mut decoder, output)?;
        Ok(n_bytes as usize)
    }

    /// Decompress snappy data framed
    #[inline(always)]
    pub fn compress<W: Write + ?Sized, R: Read>(data: R, output: &mut W) -> Result<usize, Error> {
        let mut encoder = FrameEncoder::new(data);
        let n_bytes = std::io::copy(&mut encoder, output)?;
        Ok(n_bytes as usize)
    }
}
