//! Experimental and unstable implementations.
//! This module makes no effort to maintain SemVer between
//! releases.
use pyo3::prelude::*;
use pyo3::PyResult;

pub(crate) fn init_py_module(m: &PyModule) -> PyResult<()> {
    Python::with_gil(|py| add_experimental_modules(py, m))?;
    Ok(())
}
fn add_experimental_modules(py: Python, m: &PyModule) -> PyResult<()> {
    use crate::blosc2;
    let sub_mod = PyModule::new(py, "blosc2")?;
    blosc2::init_py_module(sub_mod)?;
    m.add_submodule(sub_mod)?;
    Ok(())
}
