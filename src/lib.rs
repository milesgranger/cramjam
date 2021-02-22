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
use std::io::Write;

#[cfg(feature = "mimallocator")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[derive(FromPyObject)]
pub enum BytesType<'a> {
    #[pyo3(transparent, annotation = "bytes")]
    Bytes(&'a PyBytes),
    #[pyo3(transparent, annotation = "bytearray")]
    ByteArray(&'a PyByteArray),
}

impl<'a> BytesType<'a> {
    #[allow(dead_code)]
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

/// A wrapper to PyByteArray, providing the std::io::Write impl
pub struct WriteablePyByteArray<'a> {
    array: &'a PyByteArray,
    position: usize,
}

impl<'a> WriteablePyByteArray<'a> {
    pub fn new(py: Python<'a>, len: usize) -> Self {
        Self {
            array: PyByteArray::new_with(py, len, |_| Ok(())).unwrap(),
            position: 0,
        }
    }
    pub fn into_inner(mut self) -> PyResult<&'a PyByteArray> {
        self.flush()
            .map_err(|e| pyo3::exceptions::PyBufferError::new_err(e.to_string()))?;
        Ok(self.array)
    }
}

impl<'a> Write for WriteablePyByteArray<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if (self.position + buf.len()) > self.array.len() {
            self.array.resize(self.position + buf.len()).unwrap()
        }
        let array_bytes = unsafe { self.array.as_bytes_mut() };

        //let mut wtr = Cursor::new(&mut array_bytes[self.position..]);
        //let n_bytes = wtr.write(buf).unwrap();
        let buf_len = buf.len();
        array_bytes[self.position..self.position + buf_len].copy_from_slice(buf);

        self.position += buf.len();
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        if self.array.len() != self.position {
            self.array.resize(self.position).unwrap();
        }
        Ok(())
    }
}

/// Expose de/compression_into(data: BytesType<'_>, array: &PyArray1<u8>) -> PyResult<usize>
/// functions to allow de/compress bytes into a pre-allocated Python array.
///
/// This will handle gaining access to the Python's array as a buffer for an underlying de/compression
/// function which takes the normal `&[u8]` and `Output` types
#[macro_export]
macro_rules! generic_into {
    ($op:ident($input:ident -> $output:ident) $(, $level:ident)?) => {
        {
            let mut array_mut = unsafe { $output.as_array_mut() };

            let buffer: &mut [u8] = to_py_err!(DecompressionError -> array_mut.as_slice_mut().ok_or_else(|| {
                pyo3::exceptions::PyBufferError::new_err("Failed to get mutable slice from array.")
            }))?;
            let mut cursor = Cursor::new(buffer);
            let size = to_py_err!(DecompressionError -> self::internal::$op($input.as_bytes(), &mut cursor $(, $level)?))?;
            Ok(size)
        }
    }
}

#[macro_export]
macro_rules! generic {
    ($op:ident($input:ident), py=$py:ident, output_len=$output_len:ident $(, level=$level:ident)?) => {
        {
            let bytes = $input.as_bytes();
            match $input {
                BytesType::Bytes(_) => match $output_len {
                    Some(len) => {
                        let pybytes = PyBytes::new_with($py, len, |buffer| {
                            let mut cursor = Cursor::new(buffer);
                            if stringify!($op) == "compress" {
                                to_py_err!(CompressionError -> self::internal::$op(bytes, &mut cursor $(, $level)? ))?;
                            } else {
                                to_py_err!(DecompressionError -> self::internal::$op(bytes, &mut cursor $(, $level)? ))?;
                            }
                            Ok(())
                        })?;
                        Ok(BytesType::Bytes(pybytes))
                    }
                    None => {
                        let mut buffer = Vec::new();
                        if stringify!($op) == "compress" {
                            to_py_err!(CompressionError -> self::internal::$op(bytes, &mut buffer $(, $level)? ))?;
                        } else {
                            to_py_err!(DecompressionError -> self::internal::$op(bytes, &mut buffer $(, $level)? ))?;
                        }

                        Ok(BytesType::Bytes(PyBytes::new($py, &buffer)))
                    }
                },
                BytesType::ByteArray(_) => {
                    let mut pybytes = WriteablePyByteArray::new($py, $output_len.unwrap_or_else(|| 0));
                    if stringify!($op) == "compress" {
                        to_py_err!(CompressionError -> self::internal::$op(bytes, &mut pybytes $(, $level)? ))?;
                    } else {
                        to_py_err!(DecompressionError -> self::internal::$op(bytes, &mut pybytes $(, $level)? ))?;
                    }
                    Ok(BytesType::ByteArray(pybytes.into_inner()?))
                }
            }
        }
    }
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

#[cfg(test)]
mod tests {

    use std::io::Cursor;

    // Default testing data
    fn gen_data() -> Vec<u8> {
        (0..1000000)
            .map(|_| "oh what a beautiful morning, oh what a beautiful day!!")
            .collect::<String>()
            .into_bytes()
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
                        crate::$variant::internal::compress(&data, &mut cursor $(, $level)?).unwrap()
                    } else {

                        crate::$variant::internal::compress(&data, &mut compressed $(, $level)?).unwrap()
                    };

                assert_eq!(compressed_size, $compressed_len);
                compressed.truncate(compressed_size);

                let mut decompressed = Vec::new();

                let decompressed_size = if stringify!($decompress_output) == "Slice" {
                        decompressed = (0..data.len()).map(|_| 0).collect::<Vec<u8>>();
                        let mut cursor = Cursor::new(decompressed.as_mut_slice());
                        crate::$variant::internal::decompress(&compressed, &mut cursor).unwrap()
                    } else {

                        crate::$variant::internal::decompress(&compressed, &mut decompressed).unwrap()
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
}
