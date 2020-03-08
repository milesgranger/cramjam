pub mod brotli;
pub mod lz4;
pub mod snappy;

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3::wrap_pyfunction;

#[pyfunction]
fn snappy_decompress<'a>(py: Python<'a>, data: &'a [u8]) -> PyResult<&'a PyBytes> {
    let decompressed = snappy::decompress(data);
    Ok(PyBytes::new(py, &decompressed))
}

#[pyfunction]
fn snappy_compress<'a>(py: Python<'a>, data: &'a [u8]) -> PyResult<&'a PyBytes> {
    let compressed = snappy::compress(data);
    Ok(PyBytes::new(py, &compressed))
}

#[pyfunction]
fn brotli_decompress<'a>(py: Python<'a>, data: &'a [u8]) -> PyResult<&'a PyBytes> {
    let decompressed = brotli::decompress(data);
    Ok(PyBytes::new(py, &decompressed))
}

#[pyfunction]
fn brotli_compress<'a>(py: Python<'a>, data: &'a [u8], level: Option<u32>) -> PyResult<&'a PyBytes> {
    let level = level.unwrap_or_else(|| 11);
    let compressed = brotli::compress(data, level);
    Ok(PyBytes::new(py, &compressed))
}

#[pyfunction]
fn lz4_decompress<'a>(py: Python<'a>, data: &'a [u8]) -> PyResult<&'a PyBytes> {
    let decompressed = lz4::decompress(data);
    Ok(PyBytes::new(py, &decompressed))
}

#[pyfunction]
fn lz4_compress<'a>(py: Python<'a>, data: &'a [u8]) -> PyResult<&'a PyBytes> {
    let compressed = lz4::compress(data);
    Ok(PyBytes::new(py, &compressed))
}

#[pymodule]
fn cramjam(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_wrapped(wrap_pyfunction!(snappy_compress))?;
    m.add_wrapped(wrap_pyfunction!(snappy_decompress))?;

    m.add_wrapped(wrap_pyfunction!(brotli_compress))?;
    m.add_wrapped(wrap_pyfunction!(brotli_decompress))?;

    m.add_wrapped(wrap_pyfunction!(lz4_compress))?;
    m.add_wrapped(wrap_pyfunction!(lz4_decompress))?;

    Ok(())
}
