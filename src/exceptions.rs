use pyo3::create_exception;
use pyo3::exceptions::Exception;

create_exception!(cramjam, CompressionError, Exception);
create_exception!(cramjam, DecompressionError, Exception);
