//! Experimental and unstable implementations.
//! This module makes no effort to maintain SemVer between
//! releases.
use pyo3::prelude::*;

#[pymodule]
pub mod experimental {

    #[pymodule_export]
    use crate::blosc2::blosc2;
}
