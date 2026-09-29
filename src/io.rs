//! Module holds native Rust objects exposed to Python, or objects
//! which wrap native Python objects to provide additional functionality
//! or tighter integration with de/compression algorithms.
//!
use std::convert::TryFrom;
use std::fs::{File, OpenOptions};
use std::io::{copy, Cursor, Read, Seek, SeekFrom, Write};
use std::ops::Deref;
use std::os::raw::c_int;

use crate::exceptions::CompressionError;
use crate::BytesType;
use pyo3::exceptions::{self, PyBufferError};
use pyo3::ffi;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
#[cfg(any(PyPy, Py_GIL_DISABLED))]
use pyo3::IntoPyObjectExt;
use std::path::PathBuf;
use std::sync::Arc;

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
        if self.readonly() {
            return Err(pyo3::exceptions::PyTypeError::new_err("Output buffer is read-only"));
        }
        #[cfg(any(PyPy, Py_GIL_DISABLED))]
        {
            Python::attach(|py| {
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
        Python::attach(|_| unsafe { ffi::PyBuffer_Release(&mut *self.inner) })
    }
}

impl<'a, 'py> FromPyObject<'a, 'py> for PythonBuffer {
    type Error = PyErr;

    fn extract(obj: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        Self::try_from(&obj.to_owned())
    }
}

impl<'a, 'py> TryFrom<&'a Bound<'py, PyAny>> for PythonBuffer {
    type Error = PyErr;
    fn try_from(obj: &'a Bound<'py, PyAny>) -> Result<Self, Self::Error> {
        let mut buf = Box::<ffi::Py_buffer>::new_uninit();
        let rc = unsafe { ffi::PyObject_GetBuffer(obj.as_ptr(), buf.as_mut_ptr(), ffi::PyBUF_CONTIG_RO) };
        if rc != 0 {
            // Chain the reason (e.g. "a bytes-like object is required") instead of leaving it pending.
            let err =
                exceptions::PyBufferError::new_err("Failed to get buffer, is it C contiguous, and shape is not null?");
            err.set_cause(obj.py(), PyErr::take(obj.py()));
            return Err(err);
        }
        let buf = unsafe { buf.assume_init() };
        let buf = Self {
            inner: std::pin::Pin::from(buf),
            pos: 0,
            #[cfg(any(PyPy, Py_GIL_DISABLED))]
            owner: Python::attach(|py| obj.into_py_any(py).unwrap()),
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

/// Bytes behind a [`RustyBuffer`]: owned, or a view (`copy=False`) that holds the source's
/// buffer-protocol export for the Buffer's lifetime, so the source can't resize or free them.
pub(crate) enum Storage {
    Owned(Vec<u8>),
    View(PythonBuffer),
}

impl Default for Storage {
    fn default() -> Self {
        Storage::Owned(vec![])
    }
}

impl Deref for Storage {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        match self {
            Storage::Owned(v) => v,
            Storage::View(view) => view.as_slice(),
        }
    }
}

impl AsRef<[u8]> for Storage {
    fn as_ref(&self) -> &[u8] {
        self
    }
}

impl PartialEq for Storage {
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}

impl Storage {
    /// Errors (`TypeError`) for a view of a read-only source, such as `bytes`.
    pub(crate) fn as_mut_slice(&mut self) -> PyResult<&mut [u8]> {
        match self {
            Storage::Owned(v) => Ok(v),
            Storage::View(view) => view.as_slice_mut(),
        }
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
/// NOTE: `copy=False` doesn't copy: the Buffer reads and writes the source object's memory
/// directly, holding its buffer export for the Buffer's lifetime. While the Buffer is alive
/// the source can't be resized (e.g. `bytearray` raises `BufferError`), the Buffer itself
/// can't grow past the source's length, and writing into a read-only source such as `bytes`
/// raises `TypeError`. This makes it a zero-copy file-like wrapper, e.g. for handing a numpy
/// array to an API that only accepts file-like objects, such as an S3 upload `Body`
/// (fsspec/s3fs#959).
///
/// `copy=False` is not supported on PyPy distributions
#[pyclass(subclass, name = "Buffer")]
#[derive(Default)]
pub struct RustyBuffer {
    pub(crate) inner: Cursor<Storage>,
    /// Each live buffer-protocol export (memoryview, numpy array, ...) holds a clone, dropped in
    /// `__releasebuffer__`. Exports point straight into `inner`'s allocation, so its length
    /// can't change while any exist.
    exports: Arc<()>,
}

impl From<Vec<u8>> for RustyBuffer {
    fn from(v: Vec<u8>) -> Self {
        Self {
            inner: Cursor::new(Storage::Owned(v)),
            exports: Default::default(),
        }
    }
}

impl RustyBuffer {
    fn is_view(&self) -> bool {
        matches!(self.inner.get_ref(), Storage::View(_))
    }

    /// The object a `copy=False` Buffer views, as a borrowed pointer.
    fn view_obj(&self) -> Option<*mut ffi::PyObject> {
        match self.inner.get_ref() {
            Storage::View(view) if !view.inner.obj.is_null() => Some(view.inner.obj),
            _ => None,
        }
    }

    /// Like `bytearray`, refuse to change the length while buffer-protocol exports exist.
    fn ensure_resizable(&self) -> PyResult<()> {
        if Arc::strong_count(&self.exports) > 1 {
            return Err(PyBufferError::new_err(
                "Existing exports of data: object cannot be re-sized",
            ));
        }
        Ok(())
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
    pub fn __init__(data: Option<Bound<'_, PyAny>>, copy: Option<bool>) -> PyResult<Self> {
        let Some(data) = data else {
            return Ok(Self::default());
        };
        if copy.unwrap_or(true) {
            let mut buf = vec![];
            data.extract::<BytesType<'_>>()?.read_to_end(&mut buf)?;
            return Ok(Self::from(buf));
        }
        if cfg!(PyPy) {
            return Err(exceptions::PyRuntimeError::new_err("copy=False not supported on PyPy"));
        }
        Ok(Self {
            inner: Cursor::new(Storage::View(PythonBuffer::try_from(&data)?)),
            exports: Default::default(),
        })
    }

    /// Get the PyObject this Buffer is referencing as its view,
    /// returns None if this Buffer owns its data.
    pub fn get_view_reference(&self, py: Python<'_>) -> Option<Py<PyAny>> {
        self.view_obj()
            .map(|obj| unsafe { Bound::from_borrowed_ptr(py, obj) }.unbind())
    }

    /// Get the PyObject reference count this Buffer is referencing as its view,
    /// returns None if this Buffer owns its data.
    pub fn get_view_reference_count(&self) -> Option<isize> {
        self.view_obj().map(|obj| unsafe { ffi::Py_REFCNT(obj) })
    }

    /// Length of the underlying buffer
    pub fn len(&self) -> usize {
        self.inner.get_ref().len()
    }

    /// Write some bytes to the buffer, where input data can be anything in [BytesType](../enum.BytesType.html)
    pub fn write(&mut self, mut input: BytesType) -> PyResult<usize> {
        // A view can't grow, so refuse up front rather than after a partial write.
        if self.is_view() {
            if input.len()? > self.inner.get_ref().len() - self.inner.position() as usize {
                return Err(exceptions::PyIOError::new_err("Too much to write on view"));
            }
        }
        let r = write(&mut input, self)?;
        Ok(r as usize)
    }
    /// Read from the buffer in its current position, returns bytes; optionally specify number of bytes to read.
    #[pyo3(signature = (n_bytes=None))]
    pub fn read<'a>(&mut self, py: Python<'a>, n_bytes: Option<isize>) -> PyResult<Bound<'a, PyBytes>> {
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
    #[pyo3(signature = (position, whence=None))]
    pub fn seek(&mut self, position: isize, whence: Option<usize>) -> PyResult<usize> {
        let pos = match whence.unwrap_or_else(|| 0) {
            0 => {
                if self.is_view() {
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
                if self.is_view() {
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
                if self.is_view() {
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
    pub fn tell(&self) -> usize {
        self.inner.position() as usize
    }
    /// Set the length of the buffer. If less than current length, it will truncate to the size given;
    /// otherwise will be null byte filled to the size.
    pub fn set_len(&mut self, size: usize) -> PyResult<()> {
        if self.is_view() {
            return Err(exceptions::PyIOError::new_err("Cannot set length on unowned buffer"));
        }
        if size != self.inner.get_ref().len() {
            self.ensure_resizable()?;
        }
        if let Storage::Owned(v) = self.inner.get_mut() {
            v.resize(size, 0);
        }
        Ok(())
    }
    /// Truncate the buffer
    pub fn truncate(&mut self) -> PyResult<()> {
        if self.is_view() {
            return Err(exceptions::PyIOError::new_err("Cannot truncate unowned buffer"));
        }
        if !self.inner.get_ref().is_empty() {
            self.ensure_resizable()?;
        }
        if let Storage::Owned(v) = self.inner.get_mut() {
            v.clear();
        }
        self.inner.set_position(0);
        Ok(())
    }

    fn __len__(&self) -> usize {
        self.len()
    }
    fn __contains__(&self, py: Python, x: BytesType) -> PyResult<bool> {
        let bytes: &[u8] = &x.as_bytes()?;
        Ok(py.detach(|| self.inner.get_ref().windows(bytes.len()).any(|w| w == bytes)))
    }
    fn __repr__(&self) -> String {
        format!("cramjam.Buffer<len={:?}>", self.len())
    }
    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
    fn __bool__(&self) -> bool {
        self.len() > 0
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

        let bytes: &[u8] = slf.inner.get_ref();

        (*view).buf = bytes.as_ptr() as *mut std::os::raw::c_void;
        (*view).len = bytes.len() as isize;
        // RustyBuffer may resize its Vec, so exported views cannot safely be writable.
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
        (*view).internal = Arc::into_raw(Arc::clone(&slf.exports)) as *mut std::os::raw::c_void;
        Ok(())
    }
    // Takes `&Bound` rather than `&self` so CPython's path needs no borrow: a failed borrow
    // would skip this, leaking the export and leaving the buffer non-resizable forever.
    unsafe fn __releasebuffer__(slf: &Bound<'_, Self>, view: *mut ffi::Py_buffer) {
        let export = (*view).internal as *const ();
        if !cfg!(PyPy) && !export.is_null() {
            drop(Arc::from_raw(export));
        } else if let Ok(this) = slf.try_borrow() {
            // PyPy passes a fresh Py_buffer with `internal` unset, so reach the Arc through
            // `self`; all clones share one allocation, so this releases exactly one export.
            // ponytail: if another thread holds a mutable borrow right now, the export leaks
            // (Buffer stays non-resizable; never unsound).
            Arc::decrement_strong_count(Arc::as_ptr(&this.exports));
        }
    }
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
        let position = self.inner.position();
        let pos = usize::try_from(position).unwrap_or(usize::MAX);
        let end = pos.saturating_add(buf.len());
        if end > self.inner.get_ref().len() {
            if self.is_view() {
                // A view's memory belongs to the source object; it can't grow.
                let err = exceptions::PyIOError::new_err("Too much to write on view");
                return Err(std::io::Error::other(err));
            }
            self.ensure_resizable().map_err(std::io::Error::other)?;
        }
        match self.inner.get_mut() {
            Storage::Owned(v) => {
                let mut cursor = Cursor::new(v);
                cursor.set_position(position);
                cursor.write_all(buf)?;
            }
            Storage::View(view) => view.as_slice_mut().map_err(std::io::Error::other)?[pos..end].copy_from_slice(buf),
        }
        self.inner.set_position(position + buf.len() as u64);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
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
