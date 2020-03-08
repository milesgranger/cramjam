pub mod snappy;

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3::wrap_pyfunction;

#[pyfunction]
fn snappy_decompress<'a>(py: Python<'a>, data: &'a [u8]) -> PyResult<&'a PyBytes> {
    let decompressed = snappy::decompress_snappy(data);
    Ok(PyBytes::new(py, &decompressed))
}

#[pyfunction]
fn snappy_compress<'a>(py: Python<'a>, data: &'a [u8]) -> PyResult<&'a PyBytes> {
    let compressed = snappy::compress_snappy(data);
    Ok(PyBytes::new(py, &compressed))
}

#[pymodule]
fn cramjam(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_wrapped(wrap_pyfunction!(snappy_compress))?;
    m.add_wrapped(wrap_pyfunction!(snappy_decompress))?;
    Ok(())
}
