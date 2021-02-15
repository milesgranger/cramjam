//! CramJam documentation of python exported functions for (de)compression of bytes
//!
//! The API follows cramjam.`<<compression algorithm>>.compress` and cramjam.`<<compression algorithm>>.decompress`
//!
//! Python Example:
//!
//! ```python
//! data = b'some bytes here'
//! compressed = cramjam.snappy.compress(data)
//! decompressed = cramjam.snappy.decompress(compressed)
//! assert data == decompressed
//! ```

// TODO: There is a lot of very similar, but slightly different code for each variant
// time should be spent perhaps with a macro or other alternative.
// Each variant is similar, but sometimes has subtly different APIs/logic.

// TODO: Add output size estimation for each variant, now it's just snappy
// allow for resizing PyByteArray if over allocated; cannot resize PyBytes yet.

pub mod brotli;
pub mod deflate;
pub mod exceptions;
pub mod gzip;
pub mod lz4;
pub mod snappy;
pub mod zstd;

use pyo3::prelude::*;
use pyo3::types::{PyByteArray, PyBytes};

use exceptions::{CompressionError, DecompressionError};

#[derive(FromPyObject)]
pub enum BytesType<'a> {
    #[pyo3(transparent, annotation = "bytes")]
    Bytes(&'a PyBytes),
    #[pyo3(transparent, annotation = "bytearray")]
    ByteArray(&'a PyByteArray),
}

impl<'a> BytesType<'a> {
    fn len(&self) -> usize {
        self.as_bytes().len()
    }
    fn as_bytes(&self) -> &'a [u8] {
        match self {
            Self::Bytes(b) => b.as_bytes(),
            Self::ByteArray(b) => unsafe { b.as_bytes() },
        }
    }
}

impl<'a> IntoPy<PyObject> for BytesType<'a> {
    fn into_py(self, py: Python) -> PyObject {
        match self {
            Self::Bytes(bytes) => bytes.to_object(py),
            Self::ByteArray(byte_array) => byte_array.to_object(py),
        }
    }
}

/// Buffer to de/compression algorithms' output.
/// ::Vector used when the output len cannot be determined, and/or resulting
/// python object cannot be resized to what the actual bytes decoded was.
pub enum Output<'a> {
    Slice(&'a mut [u8]),
    Vector(&'a mut Vec<u8>),
}

#[macro_export]
macro_rules! to_py_err {
    ($error:ident -> $expr:expr) => {
        $expr.map_err(|err| PyErr::new::<$error, _>(err.to_string()))
    };
}

macro_rules! make_submodule {
    ($py:ident -> $parent:ident -> $submodule:ident) => {
        let sub_mod = PyModule::new($py, stringify!($submodule))?;
        $submodule::init_py_module(sub_mod)?;
        $parent.add_submodule(sub_mod)?;
    };
}

#[pymodule]
fn cramjam(py: Python, m: &PyModule) -> PyResult<()> {
    m.add("CompressionError", py.get_type::<CompressionError>())?;
    m.add("DecompressionError", py.get_type::<DecompressionError>())?;

    make_submodule!(py -> m -> snappy);
    make_submodule!(py -> m -> brotli);
    make_submodule!(py -> m -> lz4);
    make_submodule!(py -> m -> gzip);
    make_submodule!(py -> m -> deflate);
    make_submodule!(py -> m -> zstd);

    Ok(())
}
