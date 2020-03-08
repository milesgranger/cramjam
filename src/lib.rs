pub mod snappy;

use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

#[pyfunction]
fn snappy_decompress(data: &[u8]) -> PyResult<Vec<u8>> {
    Ok(snappy::decompress_snappy(data))
}

#[pyfunction]
fn snappy_compress(data: &[u8]) -> PyResult<Vec<u8>> {
    Ok(snappy::compress_snappy(data))
}

#[pymodule]
fn cramjam_rs(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_wrapped(wrap_pyfunction!(snappy_compress))?;
    m.add_wrapped(wrap_pyfunction!(snappy_decompress))?;
    Ok(())
}
