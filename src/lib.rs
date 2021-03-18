#![warn(missing_docs)]
//! CramJam documentation of python exported functions for (de)compression of bytes
//!
//! Although this documentation is built using Cargo/Rust toolchain, the examples and API represent
//! the usable _Python_ API
//!
//! In general, the API follows cramjam.`<<compression algorithm>>.compress` and cramjam.`<<compression algorithm>>.decompress`
//! as well as `compress_into`/`decompress_into` where it takes an input and output combination of any of the following:
//!  - `numpy.array` (dtype=np.uint8)
//!  - `bytes`
//!  - `bytearray`
//!  - [`cramjam.File`](io/struct.RustyFile.html)
//!  - [`cramjam.Buffer`](./io/struct.RustyBuffer.html)
//!
//! ### Simple Python Example:
//!
//! ```python
//! >>> data = b'some bytes here'
//! >>> compressed = cramjam.snappy.compress(data)
//! >>> decompressed = cramjam.snappy.decompress(compressed)
//! >>> assert data == decompressed
//! >>>
//! ```
//!
//! ### Example of de/compressing into different types.
//!
//! ```python
//! >>> import numpy as np
//! >>> from cramjam import snappy, Buffer
//! >>>
//! >>> data = np.frombuffer(b'some bytes here', dtype=np.uint8)
//! >>> data
//! array([115, 111, 109, 101,  32,  98, 121, 116, 101, 115,  32, 104, 101,
//!        114, 101], dtype=uint8)
//! >>>
//! >>> compressed = Buffer()
//! >>> snappy.compress_into(data, compressed)
//! 33  # 33 bytes written to compressed buffer
//! >>>
//! >>> compressed.tell()  # Where is the buffer position?
//! 33  # goodie!
//! >>>
//! >>> compressed.seek(0)  # Go back to the start of the buffer so we can prepare to decompress
//! >>> decompressed = b'0' * len(data)  # let's write to `bytes` as output
//! >>> decompressed
//! b'000000000000000'
//! >>>
//! >>> snappy.decompress_into(compressed, decompressed)
//! 15  # 15 bytes written to decompressed
//! >>> decompressed
//! b'some bytes here'
//! ```

pub mod brotli;
pub mod deflate;
pub mod exceptions;
pub mod gzip;
pub mod io;
pub mod lz4;
pub mod snappy;
pub mod zstd;

use pyo3::prelude::*;

use crate::io::{RustyBuffer, RustyFile, RustyNumpyArray, RustyPyByteArray, RustyPyBytes};
use exceptions::{CompressionError, DecompressionError};
use std::io::{Read, Seek, SeekFrom, Write};

#[cfg(feature = "mimallocator")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// Any possible input/output to de/compression algorithms.
/// Typically, as a Python user, you never have to worry about this object. It's exposed here in
/// the documentation to see what types are acceptable for de/compression functions.
#[derive(FromPyObject)]
pub enum BytesType<'a> {
    /// `bytes`
    #[pyo3(transparent, annotation = "bytes")]
    Bytes(RustyPyBytes<'a>),
    /// `bytearray`
    #[pyo3(transparent, annotation = "bytearray")]
    ByteArray(RustyPyByteArray<'a>),
    /// `numpy.array` with `dtype=np.uint8`
    #[pyo3(transparent, annotation = "numpy")]
    NumpyArray(RustyNumpyArray<'a>),
    /// [`cramjam.File`](io/struct.RustyFile.html)
    #[pyo3(transparent, annotation = "File")]
    RustyFile(&'a PyCell<RustyFile>),
    /// [`cramjam.Buffer`](io/struct.RustyBuffer.html)
    #[pyo3(transparent, annotation = "Buffer")]
    RustyBuffer(&'a PyCell<RustyBuffer>),
}

impl<'a> Write for BytesType<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let result = match self {
            BytesType::RustyFile(out) => out.borrow_mut().inner.write(buf)?,
            BytesType::RustyBuffer(out) => out.borrow_mut().inner.write(buf)?,
            BytesType::ByteArray(out) => out.write(buf)?,
            BytesType::NumpyArray(out) => out.write(buf)?,
            BytesType::Bytes(out) => out.write(buf)?,
        };
        Ok(result)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            BytesType::RustyFile(f) => f.borrow_mut().flush(),
            BytesType::RustyBuffer(b) => b.borrow_mut().flush(),
            BytesType::ByteArray(_) | BytesType::Bytes(_) | BytesType::NumpyArray(_) => Ok(()),
        }
    }
}
impl<'a> Read for BytesType<'a> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            BytesType::RustyFile(data) => data.borrow_mut().inner.read(buf),
            BytesType::RustyBuffer(data) => data.borrow_mut().inner.read(buf),
            BytesType::ByteArray(data) => data.read(buf),
            BytesType::NumpyArray(array) => array.read(buf),
            BytesType::Bytes(data) => data.read(buf),
        }
    }
}
impl<'a> Seek for BytesType<'a> {
    fn seek(&mut self, style: SeekFrom) -> std::io::Result<u64> {
        match self {
            BytesType::RustyFile(f) => f.borrow_mut().inner.seek(style),
            BytesType::RustyBuffer(b) => b.borrow_mut().inner.seek(style),
            BytesType::ByteArray(a) => a.seek(style),
            BytesType::NumpyArray(a) => a.seek(style),
            BytesType::Bytes(b) => b.seek(style),
        }
    }
}

impl<'a> BytesType<'a> {
    #[allow(dead_code)]
    fn len(&self) -> usize {
        self.as_bytes().len()
    }
    fn as_bytes(&self) -> &'_ [u8] {
        match self {
            Self::Bytes(b) => b.as_bytes(),
            Self::ByteArray(b) => b.as_bytes(),
            _ => unimplemented!("Converting Rust native types to bytes is not supported"),
        }
    }
}

impl<'a> IntoPy<PyObject> for BytesType<'a> {
    fn into_py(self, py: Python) -> PyObject {
        match self {
            Self::Bytes(bytes) => bytes.to_object(py),
            Self::ByteArray(byte_array) => byte_array.to_object(py),
            Self::RustyFile(file) => file.to_object(py),
            Self::RustyBuffer(buffer) => buffer.to_object(py),
            Self::NumpyArray(array) => array.to_object(py),
        }
    }
}

/// Macro for generating the implementation of de/compression against a variant interface
#[macro_export]
macro_rules! generic {
    ($op:ident($input:expr), py=$py:ident, output_len=$output_len:ident $(, level=$level:ident)?) => {
        {
            use crate::io::{RustyPyBytes, RustyPyByteArray, RustyNumpyArray};

            match $input {
                BytesType::Bytes(_) => {
                    match $output_len {
                        Some(len) => {
                            let pybytes = PyBytes::new_with($py, len, |buffer| {
                                let mut cursor = Cursor::new(buffer);
                                if stringify!($op) == "compress" {
                                    to_py_err!(CompressionError -> self::internal::$op($input, &mut cursor $(, $level)? ))?;
                                } else {
                                    to_py_err!(DecompressionError -> self::internal::$op($input, &mut cursor $(, $level)? ))?;
                                }
                                Ok(())
                            })?;
                            Ok(BytesType::Bytes(RustyPyBytes::from(pybytes)))
                        }
                        None => {
                            let mut buffer = Vec::new();
                            if stringify!($op) == "compress" {
                                to_py_err!(CompressionError -> self::internal::$op($input, &mut Cursor::new(&mut buffer) $(, $level)? ))?;
                            } else {
                                to_py_err!(DecompressionError -> self::internal::$op($input, &mut Cursor::new(&mut buffer) $(, $level)? ))?;
                            }

                            Ok(BytesType::Bytes(RustyPyBytes::from(PyBytes::new($py, &buffer))))
                        }
                    }
                },
                BytesType::ByteArray(_) => {
                    let mut pybytes = RustyPyByteArray::new($py, $output_len.unwrap_or_else(|| 0));
                    if stringify!($op) == "compress" {
                        to_py_err!(CompressionError -> self::internal::$op($input, &mut pybytes $(, $level)? ))?;
                    } else {
                        to_py_err!(DecompressionError -> self::internal::$op($input, &mut pybytes $(, $level)? ))?;
                    }
                    Ok(BytesType::ByteArray(pybytes))
                },
                BytesType::NumpyArray(_) => {
                    let mut output = Vec::new();
                    if stringify!($op) == "compress" {
                        to_py_err!(CompressionError -> self::internal::$op($input, &mut Cursor::new(&mut output) $(, $level)? ))?;
                    } else {
                        to_py_err!(DecompressionError -> self::internal::$op($input, &mut Cursor::new(&mut output) $(, $level)? ))?;
                    }
                    Ok(BytesType::NumpyArray(RustyNumpyArray::from_vec($py, output)))
                },
                BytesType::RustyFile(_) => {
                    let mut output = crate::io::RustyBuffer::default();
                    if stringify!($op) == "compress" {
                        to_py_err!(CompressionError -> self::internal::$op($input, &mut output $(, $level)? ))?;
                    } else {
                        to_py_err!(DecompressionError -> self::internal::$op($input, &mut output $(, $level)? ))?;
                    }
                    Ok(BytesType::RustyBuffer(PyCell::new($py, output).unwrap()))
                },
                BytesType::RustyBuffer(_) => {
                    let mut output = crate::io::RustyBuffer::default();
                    if stringify!($op) == "compress" {
                        to_py_err!(CompressionError -> self::internal::$op($input, &mut output $(, $level)? ))?;
                    } else {
                        to_py_err!(DecompressionError -> self::internal::$op($input, &mut output $(, $level)? ))?;
                    }
                    Ok(BytesType::RustyBuffer(PyCell::new($py, output).unwrap()))
                }
            }
        }
    }
}

/// Macro to convert an error into a specific Python exception.
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
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("CompressionError", py.get_type::<CompressionError>())?;
    m.add("DecompressionError", py.get_type::<DecompressionError>())?;
    m.add_class::<crate::io::RustyFile>()?;
    m.add_class::<crate::io::RustyBuffer>()?;
    make_submodule!(py -> m -> snappy);
    make_submodule!(py -> m -> brotli);
    make_submodule!(py -> m -> lz4);
    make_submodule!(py -> m -> gzip);
    make_submodule!(py -> m -> deflate);
    make_submodule!(py -> m -> zstd);

    Ok(())
}

#[cfg(test)]
mod tests {

    use std::io::Cursor;

    // Default testing data
    fn gen_data() -> Vec<u8> {
        (0..1000000)
            .map(|_| b"oh what a beautiful morning, oh what a beautiful day!!".to_vec())
            .flat_map(|v| v)
            .collect()
    }

    // Single test generation
    macro_rules! round_trip {
        ($name:ident($compress_output:ident -> $decompress_output:ident), variant=$variant:ident, compressed_len=$compressed_len:literal, $(level=$level:tt)?) => {
            #[test]
            fn $name() {
                let data = gen_data();

                let mut compressed = Vec::new();

                let compressed_size = if stringify!($decompress_output) == "Slice" {
                        compressed = (0..data.len()).map(|_| 0).collect::<Vec<u8>>();
                        let mut cursor = Cursor::new(compressed.as_mut_slice());
                        crate::$variant::internal::compress(&mut Cursor::new(data.as_slice()), &mut cursor $(, $level)?).unwrap()
                    } else {
                        crate::$variant::internal::compress(&mut Cursor::new(data.as_slice()), &mut Cursor::new(&mut compressed) $(, $level)?).unwrap()
                    };

                assert_eq!(compressed_size, $compressed_len);
                compressed.truncate(compressed_size);

                let mut decompressed = Vec::new();

                let decompressed_size = if stringify!($decompress_output) == "Slice" {
                        decompressed = (0..data.len()).map(|_| 0).collect::<Vec<u8>>();
                        let mut cursor = Cursor::new(decompressed.as_mut_slice());
                        crate::$variant::internal::decompress(&mut Cursor::new(&compressed), &mut cursor).unwrap()
                    } else {
                        crate::$variant::internal::decompress(&mut Cursor::new(&compressed), &mut decompressed).unwrap()
                    };
                assert_eq!(decompressed_size, data.len());
                if &decompressed[..decompressed_size] != &data {
                    panic!("Decompressed and original data do not match! :-(")
                }
            }
        }
    }

    // macro to generate each variation of Output::* roundtrip.
    macro_rules! test_variant {
        ($variant:ident, compressed_len=$compressed_len:literal, $(level=$level:tt)?) => {
         #[cfg(test)]
         mod $variant {
            use super::*;
            round_trip!(roundtrip_compress_via_slice_decompress_via_slice(Slice -> Slice), variant=$variant, compressed_len=$compressed_len, $(level=$level)? );
            round_trip!(roundtrip_compress_via_slice_decompress_via_vector(Slice -> Vector), variant=$variant, compressed_len=$compressed_len, $(level=$level)? );
            round_trip!(roundtrip_compress_via_vector_decompress_via_slice(Vector -> Slice), variant=$variant, compressed_len=$compressed_len, $(level=$level)? );
            round_trip!(roundtrip_compress_via_vector_decompress_via_vector(Vector -> Vector), variant=$variant, compressed_len=$compressed_len, $(level=$level)? );
         }
        }
    }

    test_variant!(snappy, compressed_len = 2572398,);
    test_variant!(gzip, compressed_len = 157192, level = None);
    test_variant!(brotli, compressed_len = 729, level = None);
    test_variant!(deflate, compressed_len = 157174, level = None);
    test_variant!(zstd, compressed_len = 4990, level = None);
    test_variant!(lz4, compressed_len = 303278, level = None);
}
