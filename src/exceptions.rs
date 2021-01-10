use pyo3::create_exception;
use pyo3::exceptions::PyException;

create_exception!(cramjam, CompressionError, PyException);
create_exception!(cramjam, DecompressionError, PyException);
