#![warn(missing_docs)]
//! Core cramjam internals, specifically the implementation of:
//!
//!  - [`cramjam_core.File`](io/struct.RustyFile.html)
//!  - [`cramjam_core.Buffer`](io/struct.RustyBuffer.html)
//!  - [`cramjam_core.RustyNumpyArray`](io/struct.RustyNumpyArray.html)
//!  - [`cramjam_core.RustyPyByteArray`](io/struct.RustyPyByteArray.html)
//!  - [`cramjam_core.RustyPyBytes`](io/struct.RustyPyBytes.html)
//!

pub mod exceptions;
pub mod io;
use pyo3::prelude::*;

pub use crate::io::{AsBytes, RustyBuffer, RustyFile, RustyNumpyArray, RustyPyByteArray, RustyPyBytes};
pub use exceptions::{CompressionError, DecompressionError};
use std::io::{Read, Seek, SeekFrom, Write};

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

impl<'a> AsBytes for BytesType<'a> {
    fn as_bytes(&self) -> &[u8] {
        match self {
            BytesType::Bytes(b) => b.as_bytes(),
            BytesType::ByteArray(b) => b.as_bytes(),
            BytesType::NumpyArray(b) => b.as_bytes(),
            BytesType::RustyBuffer(b) => {
                let py_ref = b.borrow();
                let bytes = py_ref.as_bytes();
                unsafe { std::slice::from_raw_parts(bytes.as_ptr(), bytes.len()) }
            }
            BytesType::RustyFile(b) => {
                let py_ref = b.borrow();
                let bytes = py_ref.as_bytes();
                unsafe { std::slice::from_raw_parts(bytes.as_ptr(), bytes.len()) }
            }
        }
    }
    fn as_bytes_mut(&mut self) -> &mut [u8] {
        match self {
            BytesType::Bytes(b) => b.as_bytes_mut(),
            BytesType::ByteArray(b) => b.as_bytes_mut(),
            BytesType::NumpyArray(b) => b.as_bytes_mut(),
            BytesType::RustyBuffer(b) => {
                let mut py_ref = b.borrow_mut();
                let bytes = py_ref.as_bytes_mut();
                unsafe { std::slice::from_raw_parts_mut(bytes.as_mut_ptr(), bytes.len()) }
            }
            BytesType::RustyFile(b) => {
                let mut py_ref = b.borrow_mut();
                let bytes = py_ref.as_bytes_mut();
                unsafe { std::slice::from_raw_parts_mut(bytes.as_mut_ptr(), bytes.len()) }
            }
        }
    }
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
    /// Refers to the length of bytes for the underlying variant.
    pub fn len(&self) -> usize {
        self.as_bytes().len()
    }
}

impl<'a> IntoPy<PyObject> for BytesType<'a> {
    fn into_py(self, py: Python) -> PyObject {
        match self {
            Self::Bytes(bytes) => bytes.inner.into(),
            Self::ByteArray(byte_array) => byte_array.inner.into(),
            Self::RustyFile(file) => file.to_object(py),
            Self::RustyBuffer(buffer) => buffer.into_py(py),
            Self::NumpyArray(array) => array.to_object(py),
        }
    }
}
