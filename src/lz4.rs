use crate::exceptions::{CompressionError, DecompressionError};
use crate::{to_py_err, BytesType};
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3::wrap_pyfunction;
use pyo3::{PyResult, Python};
use std::io::{Cursor, Seek, SeekFrom};

pub fn init_py_module(m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(compress, m)?)?;
    m.add_function(wrap_pyfunction!(decompress, m)?)?;
    m.add_function(wrap_pyfunction!(compress_into, m)?)?;
    m.add_function(wrap_pyfunction!(decompress_into, m)?)?;
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
#[allow(unused_variables)] // TODO: Make use of output_len for lz4
pub fn decompress<'a>(py: Python<'a>, data: BytesType<'a>, output_len: Option<usize>) -> PyResult<BytesType<'a>> {
    crate::generic!(decompress(data), py = py, output_len = output_len)
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
pub fn compress<'a>(
    py: Python<'a>,
    data: BytesType<'a>,
    level: Option<u32>,
    output_len: Option<usize>,
) -> PyResult<BytesType<'a>> {
    crate::generic!(compress(data), py = py, output_len = output_len, level = level)
}

/// Compress directly into an output buffer
#[pyfunction]
pub fn compress_into<'a>(
    _py: Python<'a>,
    mut input: BytesType<'a>,
    mut output: BytesType<'a>,
    level: Option<u32>,
) -> PyResult<usize> {
    // Unfortunately, lz4 only implements a the Read trait for its Encoder,
    // meaning we will only get the bytes read, when we want the bytes written.
    // it's too cumbersome to implement Seek for BytesType; as things like PyBytes doesn't
    // really have a position; so we match each arm and create Cursor for &[u8] and normal seek
    // for the internal rust types which have already implemented it.
    let r = match &mut output {
        BytesType::Bytes(pybytes) => {
            let starting_pos = to_py_err!(CompressionError -> pybytes.cursor.seek(SeekFrom::Current(0)))?;
            let _ = internal::compress(&mut input, pybytes, level)?;
            let ending_pos = to_py_err!(CompressionError -> pybytes.cursor.seek(SeekFrom::Current(0)))?;
            ending_pos - starting_pos
        }
        BytesType::ByteArray(pybytes) => {
            let starting_pos = to_py_err!(CompressionError -> pybytes.cursor.seek(SeekFrom::Current(0)))?;
            let _ = internal::compress(&mut input, pybytes, level)?;
            let ending_pos = to_py_err!(CompressionError -> pybytes.cursor.seek(SeekFrom::Current(0)))?;
            ending_pos - starting_pos
        }
        BytesType::NumpyArray(array) => {
            let starting_pos = to_py_err!(CompressionError -> array.cursor.seek(SeekFrom::Current(0)))?;
            internal::compress(&mut input, array, level)?;
            let ending_pos = to_py_err!(CompressionError -> array.cursor.seek(SeekFrom::Current(0)))?;
            ending_pos - starting_pos
        }
        BytesType::RustyFile(file) => {
            let mut file_ref = file.borrow_mut();
            let starting_pos = to_py_err!(CompressionError -> file_ref.inner.seek(SeekFrom::Current(0)))?;
            let _ = internal::compress(&mut input, &mut file_ref.inner, level)?;
            let ending_pos = to_py_err!(CompressionError -> file_ref.inner.seek(SeekFrom::Current(0)))?;
            (ending_pos - starting_pos) as u64
        }
        BytesType::RustyBuffer(buffer) => {
            let mut buf_ref = buffer.borrow_mut();
            let starting_pos = buf_ref.inner.seek(SeekFrom::Current(0))?;
            let _ = internal::compress(&mut input, &mut buf_ref.inner, level)?;
            let ending_pos = buf_ref.inner.seek(SeekFrom::Current(0))?;
            (ending_pos - starting_pos) as u64
        }
    };
    Ok(r as usize)
}

/// Decompress directly into an output buffer
#[pyfunction]
pub fn decompress_into<'a>(_py: Python<'a>, input: BytesType<'a>, mut output: BytesType<'a>) -> PyResult<usize> {
    let r = internal::decompress(input, &mut output)?;
    Ok(r)
}

pub(crate) mod internal {
    use lz4::{Decoder, EncoderBuilder};
    use std::io::{Error, Read, Seek, Write};

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
        let mut encoder = EncoderBuilder::new()
            .auto_flush(true)
            .level(level.unwrap_or_else(|| 4))
            .build(output)?;

        let n_bytes = std::io::copy(input, &mut encoder)?;
        let (_, r) = encoder.finish();
        r?;
        Ok(n_bytes as usize)
    }
}
