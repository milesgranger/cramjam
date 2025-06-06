//! LZ4 de/compression interface
use pyo3::prelude::*;

/// LZ4 de/compression interface
#[pymodule]
pub mod lz4 {

    use crate::exceptions::{CompressionError, DecompressionError};
    use crate::io::{AsBytes, RustyBuffer};
    use crate::BytesType;
    use libcramjam::lz4::lz4::{BlockMode, ContentChecksum};
    use pyo3::prelude::*;
    use pyo3::PyResult;
    use std::io::Cursor;
    use std::sync::Mutex;

    const DEFAULT_COMPRESSION_LEVEL: u32 = 4;

    /// LZ4 decompression.
    ///
    /// Python Example
    /// --------------
    /// ```python
    /// >>> # Note, output_len is currently ignored; underlying algorithm does not support reading to slice at this time
    /// >>> cramjam.lz4.decompress(compressed_bytes, output_len=Optional[int])
    /// ```
    #[pyfunction]
    #[pyo3(signature = (data, output_len=None))]
    pub fn decompress(py: Python, data: BytesType, output_len: Option<usize>) -> PyResult<RustyBuffer> {
        crate::generic!(py, libcramjam::lz4::decompress[data], output_len = output_len)
            .map_err(DecompressionError::from_err)
    }

    /// LZ4 compression.
    ///
    /// Python Example
    /// --------------
    /// ```python
    /// >>> # Note, output_len is currently ignored; underlying algorithm does not support reading to slice at this time
    /// >>> cramjam.lz4.compress(b'some bytes here', output_len=Optional[int])
    /// ```
    #[pyfunction]
    #[pyo3(signature = (data, level=None, output_len=None))]
    pub fn compress(
        py: Python,
        data: BytesType,
        level: Option<u32>,
        output_len: Option<usize>,
    ) -> PyResult<RustyBuffer> {
        crate::generic!(py, libcramjam::lz4::compress[data], output_len = output_len, level)
            .map_err(CompressionError::from_err)
    }

    /// Compress directly into an output buffer
    #[pyfunction]
    #[pyo3(signature = (input, output, level=None))]
    pub fn compress_into(py: Python, input: BytesType, mut output: BytesType, level: Option<u32>) -> PyResult<usize> {
        crate::generic!(py, libcramjam::lz4::compress[input, output], level).map_err(CompressionError::from_err)
    }

    /// Decompress directly into an output buffer
    #[pyfunction]
    pub fn decompress_into(py: Python, input: BytesType, mut output: BytesType) -> PyResult<usize> {
        crate::generic!(py, libcramjam::lz4::decompress[input, output]).map_err(DecompressionError::from_err)
    }

    /// LZ4 _block_ decompression.
    ///
    /// `output_len` is optional, it's the upper bound length of decompressed data; if it's not provided,
    /// then it's assumed `store_size=True` was used during compression and length will then be taken
    /// from the header, otherwise it's assumed `store_size=False` was used and no prepended size exists in input
    ///
    /// Python Example
    /// --------------
    /// ```python
    /// >>> cramjam.lz4.decompress_block(compressed_bytes, output_len=Optional[int])
    /// ```
    #[pyfunction]
    #[allow(unused_variables)]
    #[pyo3(signature = (data, output_len=None))]
    pub fn decompress_block(py: Python, data: BytesType, output_len: Option<usize>) -> PyResult<RustyBuffer> {
        let bytes = data.as_bytes();

        py.allow_threads(|| {
            match output_len {
                Some(n) => {
                    let mut buf = vec![0u8; n];
                    libcramjam::lz4::block::decompress_into(bytes, &mut buf, Some(false)).map(|_| buf)
                }
                None => libcramjam::lz4::block::decompress_vec(bytes),
            }
            .map_err(DecompressionError::from_err)
            .map(RustyBuffer::from)
        })
    }

    /// LZ4 _block_ compression.
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
    #[pyo3(signature = (data, output_len=None, mode=None, acceleration=None, compression=None, store_size=None))]
    pub fn compress_block(
        py: Python,
        data: BytesType,
        output_len: Option<usize>,
        mode: Option<&str>,
        acceleration: Option<i32>,
        compression: Option<i32>,
        store_size: Option<bool>,
    ) -> PyResult<RustyBuffer> {
        let bytes = data.as_bytes();
        py.allow_threads(|| {
            libcramjam::lz4::block::compress_vec(bytes, compression.map(|v| v as _), acceleration, store_size)
        })
        .map_err(CompressionError::from_err)
        .map(RustyBuffer::from)
    }

    /// LZ4 _block_ decompression into a pre-allocated buffer.
    ///
    /// Python Example
    /// --------------
    /// ```python
    /// >>> cramjam.lz4.decompress_block_into(compressed_bytes, output_buffer)
    /// ```
    #[pyfunction]
    #[pyo3(signature = (input, output, output_len=None))]
    pub fn decompress_block_into(
        py: Python,
        input: BytesType,
        mut output: BytesType,
        output_len: Option<usize>,
    ) -> PyResult<usize> {
        let bytes = input.as_bytes();

        // If output_len is not set, we assume size is stored in block
        let size_stored = output_len.is_none();

        // If we have output_len set, but the actual length of output
        // is less than output_len, we'll let the user know.
        if let Some(size) = output_len {
            if output.len() < size {
                let msg = format!("output_len set to {}, but output is less. ({})", size, output.len());
                return Err(DecompressionError::new_err(msg));
            }
        }

        let out_bytes = output.as_bytes_mut()?;
        py.allow_threads(
            || match libcramjam::lz4::block::decompress_into(bytes, out_bytes, Some(size_stored)) {
                Ok(r) => Ok(r),
                // Fallback and try negation of stored size, incase we/they got it wrong;
                // giving back original error if this also fails.
                Err(e) => libcramjam::lz4::block::decompress_into(bytes, out_bytes, Some(!size_stored)).map_err(|_| e),
            },
        )
        .map_err(DecompressionError::from_err)
        .map(|v| v as _)
    }

    /// LZ4 _block_ compression into pre-allocated buffer.
    ///
    /// The kwargs mostly follow the same definition found in [python-lz4 block.compress](https://python-lz4.readthedocs.io/en/stable/lz4.block.html#module-lz4.block)
    ///
    /// Python Example
    /// --------------
    /// ```python
    /// >>> cramjam.lz4.compress_block_into(
    /// ...     b'some bytes here',
    /// ...     output=output_buffer,
    /// ...     mode=Option[str],
    /// ...     acceleration=Option[int],
    /// ...     compression=Option[int],
    /// ...     store_size=Option[bool]
    /// ... )
    /// ```
    #[pyfunction]
    #[allow(unused_variables)]
    #[pyo3(signature = (data, output, mode=None, acceleration=None, compression=None, store_size=None))]
    pub fn compress_block_into(
        py: Python,
        data: BytesType,
        mut output: BytesType,
        mode: Option<&str>,
        acceleration: Option<i32>,
        compression: Option<i32>,
        store_size: Option<bool>,
    ) -> PyResult<usize> {
        let bytes = data.as_bytes();
        let out_bytes = output.as_bytes_mut()?;
        py.allow_threads(|| {
            libcramjam::lz4::block::compress_into(
                bytes,
                out_bytes,
                compression.map(|v| v as _),
                acceleration,
                store_size,
            )
        })
        .map_err(CompressionError::from_err)
        .map(|v| v as _)
    }

    /// Determine the size of a buffer which is guaranteed to hold the result of block compression, will error if
    /// data is too long to be compressed by LZ4.
    ///
    /// Python Example
    /// --------------
    /// ```python
    /// >>> cramjam.lz4.compress_block_bound(b'some bytes here')
    /// ```
    #[pyfunction]
    pub fn compress_block_bound(src: BytesType) -> PyResult<usize> {
        Ok(libcramjam::lz4::block::compress_bound(src.len(), Some(true)))
    }

    /// lz4 Compressor object for streaming compression
    #[pyclass]
    pub struct Compressor {
        inner: Mutex<Option<libcramjam::lz4::lz4::Encoder<Cursor<Vec<u8>>>>>,
    }

    #[pymethods]
    impl Compressor {
        /// Initialize a new `Compressor` instance.
        #[new]
        #[pyo3(signature = (level=None, content_checksum=None, block_linked=None))]
        pub fn __init__(
            level: Option<u32>,
            content_checksum: Option<bool>,
            block_linked: Option<bool>,
        ) -> PyResult<Self> {
            let inner = libcramjam::lz4::lz4::EncoderBuilder::new()
                .auto_flush(true)
                .level(level.unwrap_or(DEFAULT_COMPRESSION_LEVEL))
                .checksum(match content_checksum {
                    Some(false) => ContentChecksum::NoChecksum,
                    _ => ContentChecksum::ChecksumEnabled,
                })
                .block_mode(match block_linked {
                    Some(false) => BlockMode::Independent,
                    _ => BlockMode::Linked,
                })
                .build(Cursor::new(vec![]))?;
            Ok(Self {
                inner: Mutex::new(Some(inner)),
            })
        }

        /// Compress input into the current compressor's stream.
        pub fn compress(&mut self, input: &[u8]) -> PyResult<usize> {
            crate::io::stream_compress(&mut self.inner.lock().unwrap(), input)
        }

        /// Flush and return current compressed stream
        #[allow(mutable_transmutes)] // TODO: feature req to lz4 to get mut ref to writer
        pub fn flush(&mut self) -> PyResult<RustyBuffer> {
            crate::io::stream_flush(&mut self.inner.lock().unwrap(), |e| {
                let writer = e.writer();
                // no other mutations to buf b/c it'll be truncated and return immediately after this
                unsafe { std::mem::transmute::<&Cursor<Vec<u8>>, &mut Cursor<Vec<u8>>>(writer) }
            })
        }

        /// Consume the current compressor state and return the compressed stream
        /// **NB** The compressor will not be usable after this method is called.
        pub fn finish(&mut self) -> PyResult<RustyBuffer> {
            crate::io::stream_finish(&mut self.inner.lock().unwrap(), |inner| {
                let (cursor, result) = inner.finish();
                result.map(|_| cursor.into_inner())
            })
        }
    }

    mod _decompressor {
        use super::*;
        crate::make_decompressor!(lz4);
    }
    #[pymodule_export]
    use _decompressor::Decompressor;
}
