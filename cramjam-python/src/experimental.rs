//! Experimental and unstable implementations.
//! This module makes no effort to maintain SemVer between
//! releases.
use pyo3::prelude::*;
use pyo3::PyResult;

pub(crate) fn init_py_module(m: &PyModule) -> PyResult<()> {
    Python::with_gil(|py| add_experimental_modules(py, m))?;
    Ok(())
}
fn add_experimental_modules(_py: Python, m: &PyModule) -> PyResult<()> {
    use crate::blosc2;

    blosc2::init_py_module(m)?;
    Ok(())
}
