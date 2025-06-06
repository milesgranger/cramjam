//! Module holds native Rust objects exposed to Python, or objects
//! which wrap native Python objects to provide additional functionality
//! or tighter integration with de/compression algorithms.
//!
use std::convert::TryFrom;
use std::fs::{File, OpenOptions};
use std::io::{copy, Cursor, Read, Seek, SeekFrom, Write};
use std::mem;
use std::os::raw::c_int;

use crate::exceptions::CompressionError;
use crate::BytesType;
use pyo3::exceptions::{self, PyBufferError};
use pyo3::ffi;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3::IntoPyObjectExt;
use std::path::PathBuf;

pub(crate) trait AsBytes {
    fn as_bytes(&self) -> &[u8];
    fn as_bytes_mut(&mut self) -> PyResult<&mut [u8]>;
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

impl AsBytes for RustyFile {
    fn as_bytes(&self) -> &[u8] {
        unimplemented!(
            "Converting a File to bytes is not supported, as it'd require reading the \
        entire file into memory; consider using cramjam.Buffer"
        )
    }
    fn as_bytes_mut(&mut self) -> PyResult<&mut [u8]> {
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
    #[pyo3(signature = (path, read = None, write = None, truncate = None, append = None))]
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
    #[pyo3(signature = (n_bytes=None))]
    pub fn read<'a>(&mut self, py: Python<'a>, n_bytes: Option<usize>) -> PyResult<Bound<'a, PyBytes>> {
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
    #[pyo3(signature = (position, whence=None))]
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

    fn __repr__(&self) -> PyResult<String> {
        let path = match self.path.as_path().to_str() {
            Some(path) => path.to_string(),
            None => self.path.to_string_lossy().to_string(),
        };
        let repr = format!("cramjam.File<path={}, len={:?}>", path, self.len()?);
        Ok(repr)
    }
    fn __bool__(&self) -> PyResult<bool> {
        Ok(self.len()? > 0)
    }
    fn __len__(&self) -> PyResult<usize> {
        self.len()
    }
}

/// Internal wrapper to PyBuffer, not exposed thru API
/// used only for impl of Read/Write
// Inspired from pyo3 PyBuffer<T>, but here we don't want or care about T
pub struct PythonBuffer {
    pub(crate) inner: std::pin::Pin<Box<ffi::Py_buffer>>,
    pub(crate) pos: usize,
    #[cfg(any(PyPy, Py_GIL_DISABLED))]
    pub(crate) owner: Py<PyAny>,
}
// PyBuffer is thread-safe: the shape of the buffer is immutable while a Py_buffer exists.
// Accessing the buffer contents is protected using the GIL.
unsafe impl Send for PythonBuffer {}
unsafe impl Sync for PythonBuffer {}

impl PythonBuffer {
    /// Reset the read/write position of cursor
    pub fn reset_position(&mut self) {
        self.pos = 0;
    }
    /// Explicitly set the position of the cursor
    pub fn set_position(&mut self, pos: usize) {
        self.pos = pos;
    }
    /// Get the current position of the cursor
    pub fn position(&self) -> usize {
        self.pos
    }
    /// Is the Python buffer readonly
    pub fn readonly(&self) -> bool {
        self.inner.readonly == 1
    }
    /// Get the underlying buffer as a slice of bytes
    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.buf_ptr() as *const u8, self.len_bytes()) }
    }
    /// Get the underlying buffer as a mutable slice of bytes
    pub fn as_slice_mut(&mut self) -> PyResult<&mut [u8]> {
        #[cfg(any(PyPy, Py_GIL_DISABLED))]
        {
            Python::with_gil(|py| {
                let is_memoryview = unsafe { ffi::PyMemoryView_Check(self.owner.as_ptr()) } == 1;
                if is_memoryview || self.owner.bind(py).is_instance_of::<PyBytes>() {
                    #[cfg(PyPy)]
                    {
                        Err(pyo3::exceptions::PyTypeError::new_err(
                            "Cannot create mutable reference to `bytes` or `memoryview` on PyPy. See issue pypy/pypy#4918",
                        ))
                    }
                    #[cfg(Py_GIL_DISABLED)]
                    {
                        Err(pyo3::exceptions::PyTypeError::new_err(
                            "Cannot create mutable reference to `bytes` or `memoryview` on the free-threaded build. See issue milesgranger/cramjam#213"
                        ))
                    }
                } else {
                    Ok(())
                }
            })?;
        }
        Ok(unsafe { std::slice::from_raw_parts_mut(self.buf_ptr() as *mut u8, self.len_bytes()) })
    }
    /// If underlying buffer is c_contiguous
    pub fn is_c_contiguous(&self) -> bool {
        unsafe { ffi::PyBuffer_IsContiguous(&*self.inner as *const ffi::Py_buffer, b'C' as std::os::raw::c_char) == 1 }
    }
    /// Dimensions for buffer
    pub fn dimensions(&self) -> usize {
        self.inner.ndim as usize
    }
    /// raw pointer to buffer
    pub fn buf_ptr(&self) -> *mut std::os::raw::c_void {
        self.inner.buf
    }
    /// length of buffer in bytes
    pub fn len_bytes(&self) -> usize {
        self.inner.len as usize
    }
    /// the buffer item size
    pub fn item_size(&self) -> usize {
        self.inner.itemsize as usize
    }
    /// number of items in buffer
    pub fn item_count(&self) -> usize {
        (self.inner.len as usize) / (self.inner.itemsize as usize)
    }
}

impl Drop for PythonBuffer {
    fn drop(&mut self) {
        Python::with_gil(|_| unsafe { ffi::PyBuffer_Release(&mut *self.inner) })
    }
}

impl<'py> FromPyObject<'py> for PythonBuffer {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        Self::try_from(obj)
    }
}

impl<'a, 'py> TryFrom<&'a Bound<'py, PyAny>> for PythonBuffer {
    type Error = PyErr;
    fn try_from(obj: &'a Bound<'py, PyAny>) -> Result<Self, Self::Error> {
        let mut buf = Box::new(mem::MaybeUninit::uninit());
        let rc = unsafe { ffi::PyObject_GetBuffer(obj.as_ptr(), buf.as_mut_ptr(), ffi::PyBUF_CONTIG_RO) };
        if rc != 0 {
            return Err(exceptions::PyBufferError::new_err(
                "Failed to get buffer, is it C contiguous, and shape is not null?",
            ));
        }
        let buf = Box::new(unsafe { mem::MaybeUninit::<ffi::Py_buffer>::assume_init(*buf) });
        let buf = Self {
            inner: std::pin::Pin::from(buf),
            pos: 0,
            #[cfg(any(PyPy, Py_GIL_DISABLED))]
            owner: Python::with_gil(|py| obj.into_py_any(py).unwrap()),
        };
        // sanity checks
        if buf.inner.shape.is_null() {
            Err(exceptions::PyBufferError::new_err("shape is null"))
        } else if !buf.is_c_contiguous() {
            Err(PyBufferError::new_err("Buffer is not C contiguous"))
        } else {
            Ok(buf)
        }
    }
}

impl Read for PythonBuffer {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let slice = self.as_slice();
        if self.pos < slice.len() {
            let nbytes = (&slice[self.pos..]).read(buf)?;
            self.pos += nbytes;
            Ok(nbytes)
        } else {
            Ok(0)
        }
    }
}

impl Write for PythonBuffer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let pos = self.position();
        let slice = self
            .as_slice_mut()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        let len = slice.len();

        if pos < slice.len() {
            let nbytes = std::cmp::min(len - pos, buf.len());
            slice[pos..pos + nbytes].copy_from_slice(&buf[..nbytes]);
            self.pos += nbytes;
            Ok(nbytes)
        } else {
            Ok(0)
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub(crate) enum BufferOwnership {
    Owned,
    #[allow(dead_code)]
    View(Py<PyAny>),
}

impl Default for BufferOwnership {
    fn default() -> Self {
        BufferOwnership::Owned
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
/// NOTE: Use `copy=False` responsibly! That is to say, it will not
/// copy the data, and will be referencing the underlying buffer during this
/// Buffer's lifetime. We make an attempt to realign each time when accessing
/// the buffer, but one should broadly take care to use locks where neccessary.
/// Internally we increment the PyObject ref count, so it **should** be free
/// from said buffer being garbage collected out from under us, but do try to
/// avoid any funny business. :)
///
/// `copy=False` is not supported on PyPy distributions
#[pyclass(subclass, name = "Buffer")]
#[derive(Default)]
pub struct RustyBuffer {
    pub(crate) inner: Cursor<Vec<u8>>,
    pub(crate) ownership: BufferOwnership,
}

impl Drop for RustyBuffer {
    fn drop(&mut self) {
        if let BufferOwnership::View(_) = &mut self.ownership {
            let mut cursor = Cursor::new(vec![]);
            mem::swap(&mut self.inner, &mut cursor);

            let buf = cursor.into_inner();
            mem::forget(buf);
        }
    }
}

impl AsBytes for RustyBuffer {
    fn as_bytes(&self) -> &[u8] {
        self.inner.get_ref().as_slice()
    }
    fn as_bytes_mut(&mut self) -> PyResult<&mut [u8]> {
        let slice = self.inner.get_mut().as_mut_slice();
        Ok(slice)
    }
}

impl From<Vec<u8>> for RustyBuffer {
    fn from(v: Vec<u8>) -> Self {
        Self {
            inner: Cursor::new(v),
            ownership: BufferOwnership::Owned,
        }
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
    #[pyo3(signature = (data=None, copy=None))]
    pub fn __init__(py: Python, mut data: Option<Py<PyAny>>, copy: Option<bool>) -> PyResult<Self> {
        if let Some(maybe_bytestype) = data.as_mut() {
            let mut bytestype = maybe_bytestype.extract::<BytesType<'_>>(py)?;
            if copy.unwrap_or(true) {
                let mut buf = vec![];
                bytestype.read_to_end(&mut buf)?;
                Ok(Self {
                    inner: Cursor::new(buf),
                    ownership: BufferOwnership::Owned,
                })
            } else {
                if cfg!(PyPy) {
                    return Err(exceptions::PyRuntimeError::new_err("copy=False not supported on PyPy"));
                }
                let reference = maybe_bytestype.clone_ref(py);
                let bytes = bytestype.as_bytes();
                let buf = unsafe { Vec::from_raw_parts(bytes.as_ptr() as *mut _, bytes.len(), bytes.len()) };
                Ok(Self {
                    inner: Cursor::new(buf),
                    ownership: BufferOwnership::View(reference),
                })
            }
        } else {
            Ok(Self {
                inner: Cursor::new(vec![]),
                ownership: BufferOwnership::Owned,
            })
        }
    }

    /// When the underlying buffer view has maybe changed, call this to
    /// realign it according to the object we're referencing.
    /// Has no effect if Buffer has owned data or if it's determined no
    /// change has occurred, by comparing pointer and length.
    #[inline(always)]
    pub(crate) fn ensure_aligned_view(&mut self, py: Python) -> PyResult<()> {
        match &mut self.ownership {
            BufferOwnership::Owned => Ok(()),
            BufferOwnership::View(obj) => {
                let bytestype = obj.extract::<BytesType<'_>>(py)?;
                let bytes = bytestype.as_bytes();

                // if the pointer has changed or the length, we need to realign our buffer view
                if bytes.as_ptr() != self.inner.get_ref().as_ptr() || bytes.len() != self.inner.get_ref().len() {
                    // updated view of buffer
                    let buf = unsafe { Vec::from_raw_parts(bytes.as_ptr() as *mut _, bytes.len(), bytes.len()) };

                    // swap out inner cursor, ensuring position isn't outside bounds of
                    // a potentially shortened new buffer
                    let mut cursor = Cursor::new(buf);
                    let pos = std::cmp::min(bytes.len() as u64, self.inner.position());
                    cursor.set_position(pos);
                    mem::swap(&mut cursor, &mut self.inner);

                    // forget the inner buffer, it was not managed by us.
                    let old_inner_buf = cursor.into_inner();
                    mem::forget(old_inner_buf);
                }
                Ok(())
            }
        }
    }

    /// Get the PyObject this Buffer is referencing as its view,
    /// returns None if this Buffer owns its data.
    pub fn get_view_reference(&self) -> Option<&Py<PyAny>> {
        match self.ownership {
            BufferOwnership::Owned => None,
            BufferOwnership::View(ref obj) => Some(obj),
        }
    }

    /// Get the PyObject reference count this Buffer is referencing as its view,
    /// returns None if this Buffer owns its data.
    pub fn get_view_reference_count(&self, py: Python) -> Option<isize> {
        self.get_view_reference().map(|obj| obj.get_refcnt(py))
    }

    /// Length of the underlying buffer
    pub fn len(&mut self, py: Python) -> PyResult<usize> {
        self.ensure_aligned_view(py)?;
        Ok(self.inner.get_ref().len())
    }

    /// Write some bytes to the buffer, where input data can be anything in [BytesType](../enum.BytesType.html)
    pub fn write(&mut self, py: Python, mut input: BytesType) -> PyResult<usize> {
        self.ensure_aligned_view(py)?;

        // TODO: combining conditions is unstable with if let
        if let BufferOwnership::View(_) = self.ownership {
            if input.len() > self.inner.get_ref().len() - self.inner.position() as usize {
                return Err(exceptions::PyIOError::new_err("Too much to write on view"));
            }
        }
        let r = write(&mut input, self)?;
        Ok(r as usize)
    }
    /// Read from the buffer in its current position, returns bytes; optionally specify number of bytes to read.
    #[pyo3(signature = (n_bytes=None))]
    pub fn read<'a>(&mut self, py: Python<'a>, n_bytes: Option<isize>) -> PyResult<Bound<'a, PyBytes>> {
        self.ensure_aligned_view(py)?;

        let n_bytes = n_bytes.map(|n| {
            let remaining_bytes = self.inner.get_ref().len() - self.inner.position() as usize;
            if n < 0 {
                // negative, read all remaining bytes
                remaining_bytes
            } else {
                // can cast to usize, as n is > 0 here
                std::cmp::min(n as usize, remaining_bytes)
            }
        });

        read(self, py, n_bytes)
    }
    /// Read from the buffer in its current position, into a [BytesType](../enum.BytesType.html) object.
    pub fn readinto(&mut self, py: Python, mut output: BytesType) -> PyResult<usize> {
        self.ensure_aligned_view(py)?;

        let r = copy(self, &mut output)?;
        Ok(r as usize)
    }
    /// Seek to a position within the buffer. whence follows the same values as IOBase.seek where:
    /// ```bash
    /// 0: from start of the stream
    /// 1: from current stream position
    /// 2: from end of the stream
    /// ```
    #[pyo3(signature = (position, whence=None))]
    pub fn seek(&mut self, py: Python, position: isize, whence: Option<usize>) -> PyResult<usize> {
        self.ensure_aligned_view(py)?;

        let pos = match whence.unwrap_or_else(|| 0) {
            0 => {
                if let BufferOwnership::View(_) = self.ownership {
                    let buf_len = self.inner.get_ref().len() as isize;
                    let desired_idx = position;
                    if desired_idx > buf_len || desired_idx < 0 {
                        let msg = format!("Bad seek: cannot seek outside bounds of unowned buffer, tried to seek from start by {} which would place it outside of the buffer which has length of {}.", desired_idx, buf_len);
                        return Err(exceptions::PyIOError::new_err(msg));
                    }
                }
                SeekFrom::Start(position as u64)
            }
            1 => {
                if let BufferOwnership::View(_) = self.ownership {
                    let buf_len = self.inner.get_ref().len() as isize;
                    let current_position = self.inner.position() as isize;
                    let desired_idx = current_position + position;
                    if desired_idx > buf_len || desired_idx < 0 {
                        let msg = format!("Bad seek: cannot seek outside bounds of unowned buffer, tried to seek from current position {} by {} which would place it outside of the buffer which has length of {}.", current_position, desired_idx, buf_len);
                        return Err(exceptions::PyIOError::new_err(msg));
                    }
                }
                SeekFrom::Current(position as i64)
            }
            2 => {
                if let BufferOwnership::View(_) = self.ownership {
                    let buf_len = self.inner.get_ref().len() as isize;
                    let desired_idx = buf_len + position;
                    if desired_idx > buf_len || desired_idx < 0 {
                        let msg = format!("Bad seek: cannot seek outside bounds of unowned buffer, tried to seek from end position by {} which would place it outside of the buffer which has length of {}.", position, buf_len);
                        return Err(exceptions::PyIOError::new_err(msg));
                    }
                }

                SeekFrom::End(position as i64)
            }
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
    pub fn tell(&mut self, py: Python) -> PyResult<usize> {
        self.ensure_aligned_view(py)?;
        Ok(self.inner.position() as usize)
    }
    /// Set the length of the buffer. If less than current length, it will truncate to the size given;
    /// otherwise will be null byte filled to the size.
    pub fn set_len(&mut self, size: usize) -> PyResult<()> {
        if let BufferOwnership::View(_) = self.ownership {
            return Err(exceptions::PyIOError::new_err("Cannot set length on unowned buffer"));
        }
        self.inner.get_mut().resize(size, 0);
        Ok(())
    }
    /// Truncate the buffer
    pub fn truncate(&mut self) -> PyResult<()> {
        if let BufferOwnership::View(_) = self.ownership {
            return Err(exceptions::PyIOError::new_err("Cannot truncate unowned buffer"));
        }
        self.inner.get_mut().truncate(0);
        self.inner.set_position(0);
        Ok(())
    }

    fn __len__(&mut self, py: Python) -> PyResult<usize> {
        self.len(py)
    }
    fn __contains__(&self, py: Python, x: BytesType) -> bool {
        let bytes = x.as_bytes();
        py.allow_threads(|| self.inner.get_ref().windows(bytes.len()).any(|w| w == bytes))
    }
    fn __repr__(&mut self, py: Python) -> PyResult<String> {
        Ok(format!("cramjam.Buffer<len={:?}>", self.len(py)?))
    }
    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
    fn __bool__(&mut self, py: Python) -> PyResult<bool> {
        Ok(self.len(py)? > 0)
    }
    unsafe fn __getbuffer__(slf: PyRefMut<Self>, view: *mut ffi::Py_buffer, flags: c_int) -> PyResult<()> {
        if view.is_null() {
            return Err(pyo3::exceptions::PyBufferError::new_err("View is null"));
        }

        if (flags & ffi::PyBUF_WRITABLE) == ffi::PyBUF_WRITABLE {
            return Err(pyo3::exceptions::PyBufferError::new_err("Object is not writable"));
        }

        (*view).obj = slf.as_ptr();
        ffi::Py_INCREF((*view).obj);

        let bytes = slf.inner.get_ref().as_slice();

        (*view).buf = bytes.as_ptr() as *mut std::os::raw::c_void;
        (*view).len = bytes.len() as isize;
        (*view).readonly = 0;
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
        Ok(())
    }
    unsafe fn __releasebuffer__(&self, _view: *mut ffi::Py_buffer) {}
}

fn write<W: Write>(input: &mut BytesType, output: &mut W) -> std::io::Result<u64> {
    let result = match input {
        BytesType::RustyBuffer(buf) => copy(&mut buf.borrow_mut().inner, output)?,
        BytesType::RustyFile(data) => copy(&mut data.borrow_mut().inner, output)?,
        BytesType::PyBuffer(buf) => copy(buf, output)?,
    };
    Ok(result)
}

fn read<'a, R: Read>(reader: &mut R, py: Python<'a>, n_bytes: Option<usize>) -> PyResult<Bound<'a, PyBytes>> {
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
impl Seek for PythonBuffer {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let len = self.len_bytes();
        let current = self.position();
        match pos {
            SeekFrom::Start(n) => self.set_position(n as usize),
            SeekFrom::End(n) => self.set_position((len as i64 - n) as usize),
            SeekFrom::Current(n) => self.set_position((current as i64 + n) as usize),
        }
        Ok(self.position() as _)
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
        Some(encoder) => std::io::copy(&mut Cursor::new(input), encoder)
            .map(|v| v as usize)
            .map_err(CompressionError::from_err),
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
            let result = into_vec(encoder).map_err(CompressionError::from_err)?;
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
            inner.flush().map_err(CompressionError::from_err)?;
            let cursor = cursor_mut_ref(inner);
            let buf = RustyBuffer::from(cursor.get_ref().clone());
            cursor.get_mut().truncate(0);
            cursor.set_position(0);
            Ok(buf)
        }
        None => Ok(RustyBuffer::from(vec![])),
    }
}
