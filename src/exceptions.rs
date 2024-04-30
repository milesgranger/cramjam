#![allow(missing_docs)]
//! cramjam specific Python exceptions
use pyo3::create_exception;
use pyo3::exceptions::PyException;

create_exception!(cramjam, CompressionError, PyException);
create_exception!(cramjam, DecompressionError, PyException);

impl CompressionError {
    // From<ToString> already impl
    pub fn from_err<T: ToString>(err: T) -> pyo3::PyErr {
        CompressionError::new_err(err.to_string()).into()
    }
}

impl DecompressionError {
    pub fn from_err<T: ToString>(err: T) -> pyo3::PyErr {
        DecompressionError::new_err(err.to_string())
    }
}
