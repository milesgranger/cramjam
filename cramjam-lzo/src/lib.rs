//! lzo de/compression interface
use cramjam::{CompressionError, DecompressionError};
use cramjam::{AsBytes, RustyBuffer, to_py_err, BytesType};
use pyo3::prelude::*;
use pyo3::wrap_pyfunction;
use pyo3::PyResult;

/// LZO decompression
///
/// Python Example
/// --------------
/// ```python
/// >>> cramjam.lzo.decompress(compressed_raw_bytes)
/// ```
#[pyfunction]
#[allow(unused_variables)]
pub fn decompress(data: BytesType, output_len: Option<usize>) -> PyResult<RustyBuffer> {
    let output = to_py_err!(DecompressionError -> minilzo3::decompress_vec(data.as_bytes()))?;
    Ok(RustyBuffer::from(output))
}

/// LZO compression
///
/// This follows the header format of `python-lzo` where the first byte indicates if it's level 1
/// compression (default; and only one implemented here thus far) and the next four bytes are
/// u32 big endian formatted bytes indicating the length of the original input, before compression.
///
/// Python Example
/// --------------
/// ```python
/// >>> cramjam.lzo.compress(b'some bytes here')
/// ```
#[pyfunction]
#[allow(unused_variables)]
pub fn compress(py: Python, data: PyObject, output_len: Option<usize>) -> PyResult<RustyBuffer> {
    let data: BytesType = data.extract(py)?;
    let output = to_py_err!(CompressionError -> minilzo3::compress_vec(data.as_bytes(), true))?;
    Ok(RustyBuffer::from(output))
}

/// Compress raw format directly into an output buffer
#[pyfunction]
pub fn compress_into(input: BytesType, mut output: BytesType) -> PyResult<usize> {
    let output = minilzo3::compress(input.as_bytes(), output.as_bytes_mut(), true);
    to_py_err!(CompressionError -> output)
}

/// Decompress raw format directly into an output buffer
#[pyfunction]
pub fn decompress_into(input: BytesType, mut output: BytesType) -> PyResult<usize> {
    let output = minilzo3::decompress(input.as_bytes(), output.as_bytes_mut());
    to_py_err!(DecompressionError -> output)
}

#[pymodule]
fn cramjam_lzo(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add_class::<cramjam::RustyFile>()?;
    m.add_class::<cramjam::RustyBuffer>()?;
    m.add_function(wrap_pyfunction!(compress, m)?)?;
    m.add_function(wrap_pyfunction!(decompress, m)?)?;
    m.add_function(wrap_pyfunction!(compress_into, m)?)?;
    m.add_function(wrap_pyfunction!(decompress_into, m)?)?;
    Ok(())
}
