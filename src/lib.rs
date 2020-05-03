//! CramJam documentation of python exported functions for (de)compression of bytes
//!
//! The API follows `<<compression algorithm>>_compress` and `<<compression algorithm>>_decompress`
//!
//! Python Example:
//!
//! ```python
//! data = b'some bytes here'
//! compressed = cramjam.snappy_compress(data)
//! decompressed = cramjam.snappy_decompress(compressed)
//! assert data == decompressed
//! ```

mod brotli;
mod deflate;
mod gzip;
mod lz4;
mod snappy;
mod zstd;

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3::wrap_pyfunction;

/// Snappy decompression.
///
/// Python Example
/// --------------
/// ```python
/// >>> snappy_decompress(compressed_bytes)
/// ```
#[pyfunction]
pub fn snappy_decompress<'a>(py: Python<'a>, data: &'a [u8]) -> PyResult<&'a PyBytes> {
    let decompressed = snappy::decompress(data);
    Ok(PyBytes::new(py, &decompressed))
}

/// Snappy compression.
///
/// Python Example
/// --------------
/// ```python
/// >>> snappy_compress(b'some bytes here')
/// ```
#[pyfunction]
pub fn snappy_compress<'a>(py: Python<'a>, data: &'a [u8]) -> PyResult<&'a PyBytes> {
    let compressed = snappy::compress(data);
    Ok(PyBytes::new(py, &compressed))
}

/// Snappy decompression, raw
/// This does not use the snappy 'framed' encoding of compressed bytes.
///
/// Python Example
/// --------------
/// ```python
/// >>> snappy_decompress_raw(compressed_raw_bytes)
/// ```
#[pyfunction]
pub fn snappy_decompress_raw<'a>(py: Python<'a>, data: &'a [u8]) -> PyResult<&'a PyBytes> {
    let decompressed = snappy::decompress_raw(data);
    Ok(PyBytes::new(py, &decompressed))
}

/// Snappy compression raw.
/// This does not use the snappy 'framed' encoding of compressed bytes.
///
/// Python Example
/// --------------
/// ```python
/// >>> snappy_compress_raw(b'some bytes here')
/// ```
#[pyfunction]
pub fn snappy_compress_raw<'a>(py: Python<'a>, data: &'a [u8]) -> PyResult<&'a PyBytes> {
    let compressed = snappy::compress_raw(data);
    Ok(PyBytes::new(py, &compressed))
}

/// Brotli decompression.
///
/// Python Example
/// --------------
/// ```python
/// >>> brotli_decompress(compressed_bytes)
/// ```
#[pyfunction]
pub fn brotli_decompress<'a>(py: Python<'a>, data: &'a [u8]) -> PyResult<&'a PyBytes> {
    let decompressed = brotli::decompress(data);
    Ok(PyBytes::new(py, &decompressed))
}

/// Brotli compression.
///
/// Python Example
/// --------------
/// ```python
/// >>> brotli_compress(b'some bytes here', level=9)  # level defaults to 11
/// ```
#[pyfunction]
pub fn brotli_compress<'a>(py: Python<'a>, data: &'a [u8], level: Option<u32>) -> PyResult<&'a PyBytes> {
    let level = level.unwrap_or_else(|| 11);
    let compressed = brotli::compress(data, level);
    Ok(PyBytes::new(py, &compressed))
}

/// LZ4 compression.
///
/// Python Example
/// --------------
/// ```python
/// >>> lz4_decompress(compressed_bytes)
/// ```
#[pyfunction]
pub fn lz4_decompress<'a>(py: Python<'a>, data: &'a [u8]) -> PyResult<&'a PyBytes> {
    let decompressed = lz4::decompress(data);
    Ok(PyBytes::new(py, &decompressed))
}

/// lZ4 compression.
///
/// Python Example
/// --------------
/// ```python
/// >>> lz4_compress(b'some bytes here')
/// ```
#[pyfunction]
pub fn lz4_compress<'a>(py: Python<'a>, data: &'a [u8]) -> PyResult<&'a PyBytes> {
    let compressed = lz4::compress(data);
    Ok(PyBytes::new(py, &compressed))
}

/// Gzip decompression.
///
/// Python Example
/// --------------
/// ```python
/// >>> gzip_decompress(compressed_bytes)
/// ```
#[pyfunction]
pub fn gzip_decompress<'a>(py: Python<'a>, data: &'a [u8]) -> PyResult<&'a PyBytes> {
    let decompressed = gzip::decompress(data);
    Ok(PyBytes::new(py, &decompressed))
}

/// Gzip compression.
///
/// Python Example
/// --------------
/// ```python
/// >>> gzip_compress(b'some bytes here', level=2)  # Level defaults to 6
/// ```
#[pyfunction]
pub fn gzip_compress<'a>(py: Python<'a>, data: &'a [u8], level: Option<u32>) -> PyResult<&'a PyBytes> {
    let level = level.unwrap_or_else(|| 6);
    let compressed = gzip::compress(data, level);
    Ok(PyBytes::new(py, &compressed))
}

/// Deflate decompression.
///
/// Python Example
/// --------------
/// ```python
/// >>> deflate_decompress(compressed_bytes)
/// ```
#[pyfunction]
pub fn deflate_decompress<'a>(py: Python<'a>, data: &'a [u8]) -> PyResult<&'a PyBytes> {
    let decompressed = deflate::decompress(data);
    Ok(PyBytes::new(py, &decompressed))
}

/// Deflate compression.
///
/// Python Example
/// --------------
/// ```python
/// >>> deflate_compress(b'some bytes here', level=5)  # level defaults to 6
/// ```
#[pyfunction]
pub fn deflate_compress<'a>(py: Python<'a>, data: &'a [u8], level: Option<u32>) -> PyResult<&'a PyBytes> {
    let level = level.unwrap_or_else(|| 6);
    let compressed = deflate::compress(data, level);
    Ok(PyBytes::new(py, &compressed))
}

/// ZSTD decompression.
///
/// Python Example
/// --------------
/// ```python
/// >>> zstd_decompress(compressed_bytes)
/// ```
#[pyfunction]
pub fn zstd_decompress<'a>(py: Python<'a>, data: &'a [u8]) -> PyResult<&'a PyBytes> {
    let decompressed = zstd::decompress(data);
    Ok(PyBytes::new(py, &decompressed))
}

/// ZSTD compression.
///
/// Python Example
/// --------------
/// ```python
/// >>> zstd_compress(b'some bytes here', level=0)  # level defaults to 11
/// ```
#[pyfunction]
pub fn zstd_compress<'a>(py: Python<'a>, data: &'a [u8], level: Option<i32>) -> PyResult<&'a PyBytes> {
    let level = level.unwrap_or_else(|| 0); // 0 will use zstd's default, currently 11
    let compressed = zstd::compress(data, level);
    Ok(PyBytes::new(py, &compressed))
}

#[pymodule]
fn cramjam(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_wrapped(wrap_pyfunction!(snappy_compress))?;
    m.add_wrapped(wrap_pyfunction!(snappy_decompress))?;
    m.add_wrapped(wrap_pyfunction!(snappy_compress_raw))?;
    m.add_wrapped(wrap_pyfunction!(snappy_decompress_raw))?;

    m.add_wrapped(wrap_pyfunction!(brotli_compress))?;
    m.add_wrapped(wrap_pyfunction!(brotli_decompress))?;

    m.add_wrapped(wrap_pyfunction!(lz4_compress))?;
    m.add_wrapped(wrap_pyfunction!(lz4_decompress))?;

    m.add_wrapped(wrap_pyfunction!(gzip_compress))?;
    m.add_wrapped(wrap_pyfunction!(gzip_decompress))?;

    m.add_wrapped(wrap_pyfunction!(deflate_compress))?;
    m.add_wrapped(wrap_pyfunction!(deflate_decompress))?;

    m.add_wrapped(wrap_pyfunction!(zstd_compress))?;
    m.add_wrapped(wrap_pyfunction!(zstd_decompress))?;

    Ok(())
}
