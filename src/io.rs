//! Module holds native Rust objects exposed to Python, or objects
//! which wrap native Python objects to provide additional functionality
//! or tighter integration with de/compression algorithms.
//!
use std::fs::{File, OpenOptions};
use std::io::{copy, Cursor, Read, Seek, SeekFrom, Write};

use crate::exceptions::CompressionError;
use crate::BytesType;
use numpy::PyArray1;
use pyo3::class::buffer::PyBufferProtocol;
use pyo3::prelude::*;
use pyo3::types::{PyByteArray, PyBytes};
use pyo3::{ffi, PySequenceProtocol};
use pyo3::{AsPyPointer, PyObjectProtocol};
use std::path::PathBuf;

pub(crate) trait AsBytes {
    fn as_bytes(&self) -> &[u8];
    fn as_bytes_mut(&mut self) -> &mut [u8];
}

/// Internal wrapper for `numpy.array`/`PyArray1`, to provide Read + Write and other traits
pub struct RustyNumpyArray<'a> {
    pub(crate) inner: &'a PyArray1<u8>,
    pub(crate) cursor: Cursor<&'a mut [u8]>,
}
impl<'a> AsBytes for RustyNumpyArray<'a> {
    fn as_bytes(&self) -> &[u8] {
        self.cursor.get_ref().as_ref()
    }
    fn as_bytes_mut(&mut self) -> &mut [u8] {
        self.cursor.get_mut()
    }
}
impl<'a> RustyNumpyArray<'a> {
    pub(crate) fn as_bytes(&self) -> &[u8] {
        unsafe { self.inner.as_slice().unwrap() }
    }
}
impl<'a> From<&'a PyArray1<u8>> for RustyNumpyArray<'a> {
    fn from(inner: &'a PyArray1<u8>) -> Self {
        Self {
            inner,
            cursor: Cursor::new(unsafe { inner.as_slice_mut().unwrap() }),
        }
    }
}
impl<'a> FromPyObject<'a> for RustyNumpyArray<'a> {
    fn extract(ob: &'a PyAny) -> PyResult<Self> {
        let pybytes: &PyArray1<u8> = ob.extract()?;
        Ok(Self::from(pybytes))
    }
}
impl<'a> ToPyObject for RustyNumpyArray<'a> {
    fn to_object(&self, py: Python<'_>) -> PyObject {
        self.inner.to_object(py)
    }
}
impl<'a> Write for RustyNumpyArray<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.cursor.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.cursor.flush()
    }
}
impl<'a> Read for RustyNumpyArray<'a> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.cursor.read(buf)
    }
}

impl<'a> Seek for RustyNumpyArray<'a> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.cursor.seek(pos)
    }
}

/// Internal wrapper for `bytes`/`PyBytes`, to provide Read + Write and other traits
pub struct RustyPyBytes<'a> {
    pub(crate) inner: &'a PyBytes,
    pub(crate) cursor: Cursor<&'a mut [u8]>,
}
impl<'a> AsBytes for RustyPyBytes<'a> {
    fn as_bytes(&self) -> &[u8] {
        self.inner.as_bytes()
    }
    fn as_bytes_mut(&mut self) -> &mut [u8] {
        self.cursor.get_mut()
    }
}
impl<'a> From<&'a PyBytes> for RustyPyBytes<'a> {
    fn from(inner: &'a PyBytes) -> Self {
        let ptr = inner.as_bytes().as_ptr();
        Self {
            inner,
            cursor: Cursor::new(unsafe { std::slice::from_raw_parts_mut(ptr as *mut _, inner.as_bytes().len()) }),
        }
    }
}
impl<'a> FromPyObject<'a> for RustyPyBytes<'a> {
    fn extract(ob: &'a PyAny) -> PyResult<Self> {
        let pybytes: &PyBytes = ob.extract()?;
        Ok(Self::from(pybytes))
    }
}
impl<'a> ToPyObject for RustyPyBytes<'a> {
    fn to_object(&self, py: Python<'_>) -> PyObject {
        self.inner.to_object(py)
    }
}
impl<'a> Read for RustyPyBytes<'a> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.cursor.read(buf)
    }
}
impl<'a> Write for RustyPyBytes<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.cursor.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.cursor.flush()
    }
}
impl<'a> Seek for RustyPyBytes<'a> {
    fn seek(&mut self, style: SeekFrom) -> std::io::Result<u64> {
        self.cursor.seek(style)
    }
}

/// Internal wrapper for `bytearray`/`PyByteArray`, to provide Read + Write and other traits
pub struct RustyPyByteArray<'a> {
    pub(crate) inner: &'a PyByteArray,
    pub(crate) cursor: Cursor<&'a mut [u8]>,
}
impl<'a> AsBytes for RustyPyByteArray<'a> {
    fn as_bytes(&self) -> &[u8] {
        self.cursor.get_ref()
    }
    fn as_bytes_mut(&mut self) -> &mut [u8] {
        self.cursor.get_mut()
    }
}
impl<'a> From<&'a PyByteArray> for RustyPyByteArray<'a> {
    fn from(inner: &'a PyByteArray) -> Self {
        Self {
            inner,
            cursor: Cursor::new(unsafe { inner.as_bytes_mut() }),
        }
    }
}
impl<'a> FromPyObject<'a> for RustyPyByteArray<'a> {
    fn extract(ob: &'a PyAny) -> PyResult<Self> {
        let pybytes: &PyByteArray = ob.extract()?;
        Ok(Self::from(pybytes))
    }
}
impl<'a> ToPyObject for RustyPyByteArray<'a> {
    fn to_object(&self, py: Python<'_>) -> PyObject {
        self.inner.to_object(py)
    }
}
impl<'a> Write for RustyPyByteArray<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if (self.cursor.position() as usize + buf.len()) > self.inner.len() {
            let previous_pos = self.cursor.position();
            self.inner.resize(self.cursor.position() as usize + buf.len()).unwrap();
            self.cursor = Cursor::new(unsafe { self.inner.as_bytes_mut() });
            self.cursor.set_position(previous_pos);
        }
        self.cursor.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        if self.inner.len() != self.cursor.position() as usize {
            let prev_pos = self.cursor.position();
            self.inner.resize(self.cursor.position() as usize).unwrap();
            self.cursor = Cursor::new(unsafe { self.inner.as_bytes_mut() });
            self.cursor.set_position(prev_pos);
        }
        Ok(())
    }
}
impl<'a> Read for RustyPyByteArray<'a> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.cursor.read(buf)
    }
}

impl<'a> Seek for RustyPyByteArray<'a> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.cursor.seek(pos)
    }
}

/// A native Rust file-like object. Reading and writing takes place
/// through the Rust implementation, allowing access to the underlying
/// bytes in Python.
///
/// ### Python Example
/// ```python
/// from cramjam import File
/// file = File("/tmp/file.txt")
/// file.write(b"bytes")
/// ```
///
/// ### Notes
/// Presently, the file's handle is managed by Rust's lifetime rules, in that
/// once it's garbage collected from Python's side, it will be closed.
///
#[pyclass(name = "File")]
pub struct RustyFile {
    pub(crate) path: PathBuf,
    pub(crate) inner: File,
}

#[pyproto]
impl PyObjectProtocol for RustyFile {
    fn __repr__(&self) -> PyResult<String> {
        let path = match self.path.as_path().to_str() {
            Some(path) => path.to_string(),
            None => self.path.to_string_lossy().to_string(),
        };
        let repr = format!("cramjam.File(path={}, len={:?})", path, self.len()?);
        Ok(repr)
    }
    fn __bool__(&self) -> PyResult<bool> {
        Ok(self.len()? > 0)
    }
}

#[pyproto]
impl PySequenceProtocol for RustyFile {
    fn __len__(&self) -> PyResult<usize> {
        self.len()
    }
}

impl AsBytes for RustyFile {
    fn as_bytes(&self) -> &[u8] {
        unimplemented!(
            "Converting a File to bytes is not supported, as it'd require reading the \
        entire file into memory; consider using cramjam.Buffer"
        )
    }
    fn as_bytes_mut(&mut self) -> &mut [u8] {
        unimplemented!(
            "Converting a File to bytes is not supported, as it'd require reading the \
        entire file into memory; consider using cramjam.Buffer"
        )
    }
}

#[pymethods]
impl RustyFile {
    /// ### Example
    /// ```python
    /// from cramjam import File
    /// file = File("/tmp/file.txt", read=True, write=True, truncate=True)
    /// file.write(b"bytes")
    /// file.seek(2)
    /// file.read()
    /// b'tes'
    /// ```
    #[new]
    pub fn __init__(
        path: &str,
        read: Option<bool>,
        write: Option<bool>,
        truncate: Option<bool>,
        append: Option<bool>,
    ) -> PyResult<Self> {
        Ok(Self {
            path: PathBuf::from(path),
            inner: OpenOptions::new()
                .read(read.unwrap_or_else(|| true))
                .write(write.unwrap_or_else(|| true))
                .truncate(truncate.unwrap_or_else(|| false))
                .create(true) // create if doesn't exist, but open if it does.
                .append(append.unwrap_or_else(|| false))
                .open(path)?,
        })
    }
    /// Write some bytes to the file, where input data can be anything in [`BytesType`](../enum.BytesType.html)
    pub fn write(&mut self, mut input: BytesType) -> PyResult<usize> {
        let r = write(&mut input, self)?;
        Ok(r as usize)
    }
    /// Read from the file in its current position, returns `bytes`; optionally specify number of
    /// bytes to read.
    pub fn read<'a>(&mut self, py: Python<'a>, n_bytes: Option<usize>) -> PyResult<&'a PyBytes> {
        read(self, py, n_bytes)
    }
    /// Read from the file in its current position, into a [`BytesType`](../enum.BytesType.html) object.
    pub fn readinto(&mut self, mut output: BytesType) -> PyResult<usize> {
        let r = copy(self, &mut output)?;
        Ok(r as usize)
    }
    /// Seek to a position within the file. `whence` follows the same values as [IOBase.seek](https://docs.python.org/3/library/io.html#io.IOBase.seek)
    /// where:
    /// ```bash
    /// 0: from start of the stream
    /// 1: from current stream position
    /// 2: from end of the stream
    /// ```
    pub fn seek(&mut self, position: isize, whence: Option<usize>) -> PyResult<usize> {
        let pos = match whence.unwrap_or_else(|| 0) {
            0 => SeekFrom::Start(position as u64),
            1 => SeekFrom::Current(position as i64),
            2 => SeekFrom::End(position as i64),
            _ => {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "whence should be one of 0: seek from start, 1: seek from current, or 2: seek from end",
                ))
            }
        };
        let r = Seek::seek(self, pos)?;
        Ok(r as usize)
    }
    /// Whether the file is seekable; here just for compatibility, it always returns True.
    pub fn seekable(&self) -> bool {
        true
    }
    /// Give the current position of the file.
    pub fn tell(&mut self) -> PyResult<usize> {
        let r = self.inner.seek(SeekFrom::Current(0))?;
        Ok(r as usize)
    }
    /// Set the length of the file. If less than current length, it will truncate to the size given;
    /// otherwise will be null byte filled to the size.
    pub fn set_len(&mut self, size: usize) -> PyResult<()> {
        self.inner.set_len(size as u64)?;
        Ok(())
    }
    /// Truncate the file.
    pub fn truncate(&mut self) -> PyResult<()> {
        self.set_len(0)
    }
    /// Length of the file in bytes
    pub fn len(&self) -> PyResult<usize> {
        let meta = self
            .inner
            .metadata()
            .map_err(|e| pyo3::exceptions::PyOSError::new_err(e.to_string()))?;
        Ok(meta.len() as usize)
    }
}

/// A native Rust file-like object. Reading and writing takes place
/// through the Rust implementation, allowing access to the underlying
/// bytes in Python.
///
/// ### Python Example
/// ```python
/// >>> from cramjam import Buffer
/// >>> buf = Buffer(b"bytes")
/// >>> buf.read()
/// b'bytes'
/// ```
///
#[pyclass(name = "Buffer")]
#[derive(Default)]
pub struct RustyBuffer {
    pub(crate) inner: Cursor<Vec<u8>>,
}

impl AsBytes for RustyBuffer {
    fn as_bytes(&self) -> &[u8] {
        self.inner.get_ref().as_slice()
    }
    fn as_bytes_mut(&mut self) -> &mut [u8] {
        self.inner.get_mut().as_mut_slice()
    }
}

impl From<Vec<u8>> for RustyBuffer {
    fn from(v: Vec<u8>) -> Self {
        Self { inner: Cursor::new(v) }
    }
}

#[pyproto]
impl PyBufferProtocol for RustyBuffer {
    fn bf_getbuffer(slf: PyRefMut<Self>, view: *mut ffi::Py_buffer, flags: std::os::raw::c_int) -> PyResult<()> {
        if view.is_null() {
            return Err(pyo3::exceptions::PyBufferError::new_err("View is null"));
        }

        if (flags & ffi::PyBUF_WRITABLE) == ffi::PyBUF_WRITABLE {
            return Err(pyo3::exceptions::PyBufferError::new_err("Object is not writable"));
        }

        unsafe {
            (*view).obj = slf.as_ptr();
            ffi::Py_INCREF((*view).obj);
        }

        let bytes = slf.inner.get_ref().as_slice();

        unsafe {
            (*view).buf = bytes.as_ptr() as *mut std::os::raw::c_void;
            (*view).len = bytes.len() as isize;
            (*view).readonly = 1;
            (*view).itemsize = 1;

            (*view).format = std::ptr::null_mut();
            if (flags & ffi::PyBUF_FORMAT) == ffi::PyBUF_FORMAT {
                let msg = std::ffi::CStr::from_bytes_with_nul(b"B\0").unwrap();
                (*view).format = msg.as_ptr() as *mut _;
            }

            (*view).ndim = 1;
            (*view).shape = std::ptr::null_mut();
            if (flags & ffi::PyBUF_ND) == ffi::PyBUF_ND {
                (*view).shape = (&((*view).len)) as *const _ as *mut _;
            }

            (*view).strides = std::ptr::null_mut();
            if (flags & ffi::PyBUF_STRIDES) == ffi::PyBUF_STRIDES {
                (*view).strides = &((*view).itemsize) as *const _ as *mut _;
            }

            (*view).suboffsets = std::ptr::null_mut();
            (*view).internal = std::ptr::null_mut();
        }
        Ok(())
    }
    fn bf_releasebuffer(_slf: PyRefMut<Self>, _view: *mut ffi::Py_buffer) {}
}

#[pyproto]
impl PySequenceProtocol for RustyBuffer {
    fn __len__(&self) -> usize {
        self.len()
    }
    fn __contains__(&self, x: u8) -> bool {
        self.inner.get_ref().contains(&x)
    }
}

#[pyproto]
impl PyObjectProtocol for RustyBuffer {
    fn __repr__(&self) -> String {
        format!("cramjam.Buffer(len={:?})", self.len())
    }
    fn __bool__(&self) -> bool {
        self.len() > 0
    }
}

/// A Buffer object, similar to [cramjam.File](struct.RustyFile.html) only the bytes are held in-memory
///
/// ### Example
/// ```python
/// from cramjam import Buffer
/// buf = Buffer(b'start bytes')
/// buf.read(5)
/// b'start'
/// ```
#[pymethods]
impl RustyBuffer {
    /// Instantiate the object, optionally with any supported bytes-like object in [BytesType](../enum.BytesType.html)
    #[new]
    pub fn __init__(mut data: Option<BytesType<'_>>) -> PyResult<Self> {
        let mut buf = vec![];
        if let Some(bytes) = data.as_mut() {
            bytes.read_to_end(&mut buf)?;
        }
        Ok(Self {
            inner: Cursor::new(buf),
        })
    }

    /// Length of the underlying buffer
    pub fn len(&self) -> usize {
        self.inner.get_ref().len()
    }

    /// Write some bytes to the buffer, where input data can be anything in [BytesType](../enum.BytesType.html)
    pub fn write(&mut self, mut input: BytesType) -> PyResult<usize> {
        let r = write(&mut input, self)?;
        Ok(r as usize)
    }
    /// Read from the buffer in its current position, returns bytes; optionally specify number of bytes to read.
    pub fn read<'a>(&mut self, py: Python<'a>, n_bytes: Option<usize>) -> PyResult<&'a PyBytes> {
        read(self, py, n_bytes)
    }
    /// Read from the buffer in its current position, into a [BytesType](../enum.BytesType.html) object.
    pub fn readinto(&mut self, mut output: BytesType) -> PyResult<usize> {
        let r = copy(self, &mut output)?;
        Ok(r as usize)
    }
    /// Seek to a position within the buffer. whence follows the same values as IOBase.seek where:
    /// ```bash
    /// 0: from start of the stream
    /// 1: from current stream position
    /// 2: from end of the stream
    /// ```
    pub fn seek(&mut self, position: isize, whence: Option<usize>) -> PyResult<usize> {
        let pos = match whence.unwrap_or_else(|| 0) {
            0 => SeekFrom::Start(position as u64),
            1 => SeekFrom::Current(position as i64),
            2 => SeekFrom::End(position as i64),
            _ => {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "whence should be one of 0: seek from start, 1: seek from current, or 2: seek from end",
                ))
            }
        };
        let r = Seek::seek(self, pos)?;
        Ok(r as usize)
    }
    /// Whether the buffer is seekable; here just for compatibility, it always returns True.
    pub fn seekable(&self) -> bool {
        true
    }
    /// Give the current position of the buffer.
    pub fn tell(&self) -> usize {
        self.inner.position() as usize
    }
    /// Set the length of the buffer. If less than current length, it will truncate to the size given;
    /// otherwise will be null byte filled to the size.
    pub fn set_len(&mut self, size: usize) -> PyResult<()> {
        self.inner.get_mut().resize(size, 0);
        Ok(())
    }
    /// Truncate the buffer
    pub fn truncate(&mut self) -> PyResult<()> {
        self.inner.get_mut().truncate(0);
        self.inner.set_position(0);
        Ok(())
    }
}

fn write<W: Write>(input: &mut BytesType, output: &mut W) -> std::io::Result<u64> {
    let result = match input {
        BytesType::RustyFile(data) => copy(&mut data.borrow_mut().inner, output)?,
        BytesType::RustyBuffer(data) => copy(&mut data.borrow_mut().inner, output)?,
        BytesType::ByteArray(data) => copy(data, output)?,
        BytesType::NumpyArray(array) => copy(array, output)?,
        BytesType::Bytes(data) => {
            let buffer = data.as_bytes();
            copy(&mut Cursor::new(buffer), output)?
        }
    };
    Ok(result)
}

fn read<'a, R: Read>(reader: &mut R, py: Python<'a>, n_bytes: Option<usize>) -> PyResult<&'a PyBytes> {
    match n_bytes {
        Some(n) => PyBytes::new_with(py, n, |buf| {
            reader.read(buf)?;
            Ok(())
        }),
        None => {
            let mut buf = vec![];
            reader.read_to_end(&mut buf)?;
            Ok(PyBytes::new(py, buf.as_slice()))
        }
    }
}

impl Seek for RustyBuffer {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(pos)
    }
}
impl Seek for RustyFile {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(pos)
    }
}
impl Write for RustyBuffer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
impl Write for RustyFile {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
impl Read for RustyBuffer {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}
impl Read for RustyFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}

// general stream compression interface. Can't use associated types due to pyo3::pyclass
// not supporting generic structs.
#[inline(always)]
pub(crate) fn stream_compress<W: Write>(encoder: &mut Option<W>, input: &[u8]) -> PyResult<usize> {
    match encoder {
        Some(encoder) => {
            let result = std::io::copy(&mut Cursor::new(input), encoder).map(|v| v as usize);
            crate::to_py_err!(CompressionError -> result)
        }
        None => Err(CompressionError::new_err(
            "Compressor looks to have been consumed via `finish()`. \
            please create a new compressor instance.",
        )),
    }
}

// general stream finish interface. Can't use associated types due to pyo3::pyclass
// not supporting generic structs.
#[inline(always)]
pub(crate) fn stream_finish<W, F, E>(encoder: &mut Option<W>, into_vec: F) -> PyResult<RustyBuffer>
where
    W: Write,
    E: ToString,
    F: Fn(W) -> Result<Vec<u8>, E>,
{
    // &mut encoder is part of a Compressor, often the .finish portion consumes
    // the struct; which cannot be done with pyclass. So we'll swap it out for None
    let mut detached_encoder = None;
    std::mem::swap(&mut detached_encoder, encoder);

    match detached_encoder {
        Some(encoder) => {
            let result = crate::to_py_err!(CompressionError -> into_vec(encoder))?;
            Ok(RustyBuffer::from(result))
        }
        None => Ok(RustyBuffer::from(vec![])),
    }
}

// flush inner encoder data out
#[inline(always)]
pub(crate) fn stream_flush<W, F>(encoder: &mut Option<W>, cursor_mut_ref: F) -> PyResult<RustyBuffer>
where
    W: Write,
    F: Fn(&mut W) -> &mut Cursor<Vec<u8>>,
{
    match encoder {
        Some(inner) => {
            crate::to_py_err!(CompressionError -> inner.flush())?;
            let cursor = cursor_mut_ref(inner);
            let buf = RustyBuffer::from(cursor.get_ref().clone());
            cursor.get_mut().truncate(0);
            cursor.set_position(0);
            Ok(buf)
        }
        None => Ok(RustyBuffer::from(vec![])),
    }
}
