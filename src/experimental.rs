//! Experimental and unstable implementations.
//! This module makes no effort to maintain SemVer between
//! releases.
use pyo3::prelude::*;

/// Experimental and unstable implementations.
/// This module makes no effort to maintain SemVer between
/// releases.
#[pymodule]
pub mod experimental {

    #[cfg(any(feature = "blosc2", feature = "blosc2-static", feature = "blosc2-shared"))]
    #[pymodule_export]
    use crate::blosc2::blosc2;

    #[cfg(all(
        any(feature = "ideflate", feature = "ideflate-static", feature = "ideflate-shared"),
        target_pointer_width = "64"
    ))]
    #[pymodule_export]
    use crate::ideflate::ideflate;

    #[cfg(all(
        any(feature = "igzip", feature = "igzip-static", feature = "igzip-shared"),
        target_pointer_width = "64"
    ))]
    #[pymodule_export]
    use crate::igzip::igzip;

    #[cfg(all(
        any(feature = "izlib", feature = "izlib-static", feature = "izlib-shared"),
        target_pointer_width = "64"
    ))]
    #[pymodule_export]
    use crate::izlib::izlib;
}
