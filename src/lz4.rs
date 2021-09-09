//! lz4 de/compression interface
use crate::exceptions::{CompressionError, DecompressionError};
use crate::io::{AsBytes, RustyBuffer};
use crate::{to_py_err, BytesType};
use pyo3::prelude::*;
use pyo3::wrap_pyfunction;
use pyo3::PyResult;
use std::io::Cursor;

const DEFAULT_COMPRESSION_LEVEL: u32 = 4;

pub(crate) fn init_py_module(m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(compress, m)?)?;
    m.add_function(wrap_pyfunction!(decompress, m)?)?;
    m.add_function(wrap_pyfunction!(compress_block, m)?)?;
    m.add_function(wrap_pyfunction!(decompress_block, m)?)?;
    m.add_function(wrap_pyfunction!(compress_into, m)?)?;
    m.add_function(wrap_pyfunction!(decompress_into, m)?)?;
    m.add_class::<Compressor>()?;
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
pub fn decompress(data: BytesType, output_len: Option<usize>) -> PyResult<RustyBuffer> {
    crate::generic!(decompress(data), output_len = output_len)
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
pub fn compress(mut data: BytesType, level: Option<u32>, output_len: Option<usize>) -> PyResult<RustyBuffer> {
    crate::generic!(compress(&mut data), output_len = output_len, level = level)
}

/// Compress directly into an output buffer
#[pyfunction]
pub fn compress_into(mut input: BytesType, mut output: BytesType, level: Option<u32>) -> PyResult<usize> {
    let r = internal::compress(&mut input, &mut output, level)?;
    Ok(r)
}

/// Decompress directly into an output buffer
#[pyfunction]
pub fn decompress_into(input: BytesType, mut output: BytesType) -> PyResult<usize> {
    let r = internal::decompress(input, &mut output)?;
    Ok(r)
}

/// LZ4 _block_ decompression.
///
/// `output_len` is optional, it's the upper bound length of decompressed data; if it's not provided,
/// then it's assumed `store_size=True` was used during compression and length will then be taken
/// from the header.
///
/// Python Example
/// --------------
/// ```python
/// >>> cramjam.lz4.decompress_block(compressed_bytes, output_len=Optional[int])
/// ```
#[pyfunction]
pub fn decompress_block(data: BytesType, output_len: Option<usize>) -> PyResult<RustyBuffer> {
    use lz4::block;
    let out = to_py_err!(DecompressionError -> block::decompress(data.as_bytes(), output_len.map(|v| v as i32)))?;
    Ok(RustyBuffer::from(out))
}

/// lZ4 _block_ compression.
///
/// The kwargs mostly follow the same definition found in [python-lz4 block.compress](https://python-lz4.readthedocs.io/en/stable/lz4.block.html#module-lz4.block)
///
/// Python Example
/// --------------
/// ```python
/// >>> cramjam.lz4.compress_block(
/// ...     b'some bytes here',
/// ...     output_len=Optional[int],
/// ...     mode=Option[str],
/// ...     acceleration=Option[int],
/// ...     compression=Option[int],
/// ...     store_size=Option[bool]
/// ... )
/// ```
#[pyfunction]
#[allow(unused_variables)]
pub fn compress_block(
    data: BytesType,
    output_len: Option<usize>,
    mode: Option<&str>,
    acceleration: Option<i32>,
    compression: Option<i32>,
    store_size: Option<bool>,
) -> PyResult<RustyBuffer> {
    use lz4::{block, block::CompressionMode};

    let store_size = store_size.unwrap_or(true);
    let mode = match mode {
        Some(m) => match m {
            "default" => CompressionMode::DEFAULT,
            "fast" => CompressionMode::FAST(acceleration.unwrap_or(1)),
            "high_compression" => CompressionMode::HIGHCOMPRESSION(compression.unwrap_or(9)),
            _ => return Err(DecompressionError::new_err(format!("Unrecognized mode '{}'", m))),
        },
        None => CompressionMode::DEFAULT,
    };
    let out = to_py_err!(CompressionError -> block::compress(data.as_bytes(), Some(mode), store_size))?;
    Ok(RustyBuffer::from(out))
}

/// Snappy Compressor object for streaming compression
#[pyclass]
pub struct Compressor {
    inner: Option<lz4::Encoder<Cursor<Vec<u8>>>>,
}

#[pymethods]
impl Compressor {
    /// Initialize a new `Compressor` instance.
    #[new]
    pub fn __init__(level: Option<u32>) -> PyResult<Self> {
        let inner = lz4::EncoderBuilder::new()
            .auto_flush(true)
            .level(level.unwrap_or_else(|| DEFAULT_COMPRESSION_LEVEL))
            .build(Cursor::new(vec![]))?;
        Ok(Self { inner: Some(inner) })
    }

    /// Compress input into the current compressor's stream.
    pub fn compress(&mut self, input: &[u8]) -> PyResult<usize> {
        crate::io::stream_compress(&mut self.inner, input)
    }

    /// Flush and return current compressed stream
    #[allow(mutable_transmutes)] // TODO: feature req to lz4 to get mut ref to writer
    pub fn flush(&mut self) -> PyResult<RustyBuffer> {
        crate::io::stream_flush(&mut self.inner, |e| {
            let writer = e.writer();
            // no other mutations to buf b/c it'll be truncated and return immediately after this
            unsafe { std::mem::transmute::<&Cursor<Vec<u8>>, &mut Cursor<Vec<u8>>>(writer) }
        })
    }

    /// Consume the current compressor state and return the compressed stream
    /// **NB** The compressor will not be usable after this method is called.
    pub fn finish(&mut self) -> PyResult<RustyBuffer> {
        crate::io::stream_finish(&mut self.inner, |inner| {
            let (cursor, result) = inner.finish();
            result.map(|_| cursor.into_inner())
        })
    }
}

pub(crate) mod internal {
    use crate::lz4::DEFAULT_COMPRESSION_LEVEL;
    use lz4::{Decoder, EncoderBuilder};
    use std::io::{Error, Read, Seek, SeekFrom, Write};

    /// Decompress lz4 data
    pub fn decompress<W: Write + ?Sized, R: Read>(input: R, output: &mut W) -> Result<usize, Error> {
        let mut decoder = Decoder::new(input)?;
        let n_bytes = std::io::copy(&mut decoder, output)?;
        decoder.finish().1?;
        Ok(n_bytes as usize)
    }

    /// Compress lz4 data
    pub fn compress<W: Write + ?Sized + Seek, R: Read>(
        input: &mut R,
        output: &mut W,
        level: Option<u32>,
    ) -> Result<usize, Error> {
        let start_pos = output.seek(SeekFrom::Current(0))?;
        let mut encoder = EncoderBuilder::new()
            .auto_flush(true)
            .level(level.unwrap_or_else(|| DEFAULT_COMPRESSION_LEVEL))
            .build(output)?;

        // this returns, bytes read from uncompressed, input; we want bytes written
        // but lz4 only implements Read for Encoder
        std::io::copy(input, &mut encoder)?;
        let (w, r) = encoder.finish();
        r?;
        let ending_pos = w.seek(SeekFrom::Current(0))?;
        Ok((ending_pos - start_pos) as usize)
    }
}
