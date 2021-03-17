//! cramjam specific Python exceptions
use pyo3::create_exception;
use pyo3::exceptions::PyException;

/// Raised during any error which occurs during compression
create_exception!(cramjam, CompressionError, PyException);

/// Raised during any error which occurs during decompression
create_exception!(cramjam, DecompressionError, PyException);
