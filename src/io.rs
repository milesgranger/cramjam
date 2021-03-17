use std::fs::{File, OpenOptions};
use std::io::{copy, Cursor, Read, Seek, SeekFrom, Write};

use crate::BytesType;
use numpy::PyArray1;
use pyo3::prelude::*;
use pyo3::types::{PyByteArray, PyBytes};

// Internal wrapper for PyArray1, to provide Read + Write and other traits
pub struct RustyNumpyArray<'a> {
    pub(crate) inner: &'a PyArray1<u8>,
    pub(crate) cursor: Cursor<&'a mut [u8]>,
}
impl<'a> RustyNumpyArray<'a> {
    pub fn from_vec(py: Python<'a>, v: Vec<u8>) -> Self {
        let inner = PyArray1::from_vec(py, v);
        Self {
            inner,
            cursor: Cursor::new(unsafe { inner.as_slice_mut().unwrap() }),
        }
    }
    pub fn as_bytes(&self) -> &[u8] {
        unsafe { self.inner.as_slice().unwrap() }
    }
    pub fn as_slice(&self) -> &[u8] {
        self.as_bytes()
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

// Internal wrapper for PyBytes, to provide Read + Write and other traits
pub struct RustyPyBytes<'a> {
    pub(crate) inner: &'a PyBytes,
    pub(crate) cursor: Cursor<&'a mut [u8]>,
}
impl<'a> RustyPyBytes<'a> {
    pub fn as_bytes(&self) -> &[u8] {
        self.inner.as_bytes()
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

// Internal wrapper for PyByteArray, to provide Read + Write and other traits
pub struct RustyPyByteArray<'a> {
    pub(crate) inner: &'a PyByteArray,
    pub(crate) cursor: Cursor<&'a mut [u8]>,
}
impl<'a> RustyPyByteArray<'a> {
    pub fn new(py: Python<'a>, len: usize) -> Self {
        let inner = PyByteArray::new_with(py, len, |_| Ok(())).unwrap();
        Self {
            inner,
            cursor: Cursor::new(unsafe { inner.as_bytes_mut() }),
        }
    }
    pub fn into_inner(mut self) -> PyResult<&'a PyByteArray> {
        self.flush()
            .map_err(|e| pyo3::exceptions::PyBufferError::new_err(e.to_string()))?;
        Ok(self.inner)
    }
    pub fn as_bytes(&self) -> &[u8] {
        unsafe { self.inner.as_bytes() }
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

#[pyclass(name = "File")]
pub struct RustyFile {
    pub inner: File,
}

#[pymethods]
impl RustyFile {
    #[new]
    pub fn new(
        path: &str,
        read: Option<bool>,
        write: Option<bool>,
        truncate: Option<bool>,
        append: Option<bool>,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: OpenOptions::new()
                .read(read.unwrap_or_else(|| true))
                .write(write.unwrap_or_else(|| true))
                .truncate(truncate.unwrap_or_else(|| false))
                .create(true) // create if doesn't exist, but open if it does.
                .append(append.unwrap_or_else(|| false))
                .open(path)?,
        })
    }
    pub fn write(&mut self, data: &PyAny) -> PyResult<usize> {
        let mut input = data.extract::<BytesType>()?;
        let r = write(&mut input, self)?;
        Ok(r as usize)
    }
    pub fn read<'a>(&mut self, py: Python<'a>, n_bytes: Option<usize>) -> PyResult<&'a PyBytes> {
        read(self, py, n_bytes)
    }
    pub fn readinto(&mut self, output: &PyAny) -> PyResult<usize> {
        let mut out = output.extract::<BytesType>()?;
        let r = copy(self, &mut out)?;
        Ok(r as usize)
    }
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
    pub fn seekable(&self) -> bool {
        true
    }
    pub fn tell(&mut self) -> PyResult<usize> {
        let r = self.inner.seek(SeekFrom::Current(0))?;
        Ok(r as usize)
    }
    pub fn set_len(&mut self, size: usize) -> PyResult<()> {
        self.inner.set_len(size as u64)?;
        Ok(())
    }
    pub fn truncate(&mut self) -> PyResult<()> {
        self.set_len(0)
    }
}

#[pyclass(name = "Buffer")]
#[derive(Default)]
pub struct RustyBuffer {
    pub(crate) inner: Cursor<Vec<u8>>,
}

#[pymethods]
impl RustyBuffer {
    #[new]
    pub fn new(mut data: Option<BytesType<'_>>) -> PyResult<Self> {
        let mut buf = vec![];
        if let Some(bytes) = data.as_mut() {
            bytes.read_to_end(&mut buf)?;
        }
        Ok(Self {
            inner: Cursor::new(buf),
        })
    }
    pub fn write(&mut self, data: &PyAny) -> PyResult<usize> {
        let mut input = data.extract::<BytesType>()?;
        let r = write(&mut input, self)?;
        Ok(r as usize)
    }
    pub fn read<'a>(&mut self, py: Python<'a>, n_bytes: Option<usize>) -> PyResult<&'a PyBytes> {
        read(self, py, n_bytes)
    }
    pub fn seek(&mut self, position: i64, whence: Option<usize>) -> PyResult<usize> {
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
    pub fn seekable(&self) -> bool {
        true
    }
    pub fn tell(&self) -> usize {
        self.inner.position() as usize
    }
    pub fn readinto(&mut self, output: &PyAny) -> PyResult<usize> {
        let mut out = output.extract::<BytesType>()?;
        let r = copy(self, &mut out)?;
        Ok(r as usize)
    }
    pub fn set_len(&mut self, size: usize) -> PyResult<()> {
        self.inner.get_mut().resize(size, 0);
        Ok(())
    }
    pub fn truncate(&mut self) -> PyResult<()> {
        self.inner.get_mut().truncate(0);
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
