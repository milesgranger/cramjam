use std::fs::{File, OpenOptions};
use std::io::{copy, Cursor, Read, Seek, SeekFrom, Write};

use crate::{BytesType, WriteablePyByteArray};
use pyo3::prelude::*;
use pyo3::types::PyBytes;

#[pyclass(name = "File")]
pub struct RustyFile {
    inner: File,
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
        let out = output.extract::<BytesType>()?;
        let r = readinto(self, out)?;
        Ok(r as usize)
    }
    pub fn seek(&mut self, position: usize) -> PyResult<usize> {
        let r = Seek::seek(self, SeekFrom::Start(position as u64))?;
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
    inner: Cursor<Vec<u8>>,
}

#[pymethods]
impl RustyBuffer {
    #[new]
    pub fn new(len: Option<usize>) -> Self {
        Self {
            inner: Cursor::new(vec![0; len.unwrap_or_else(|| 0)]),
        }
    }
    pub fn write(&mut self, data: &PyAny) -> PyResult<usize> {
        let mut input = data.extract::<BytesType>()?;
        let r = write(&mut input, self)?;
        Ok(r as usize)
    }
    pub fn read<'a>(&mut self, py: Python<'a>, n_bytes: Option<usize>) -> PyResult<&'a PyBytes> {
        read(self, py, n_bytes)
    }
    pub fn seek(&mut self, position: usize) -> PyResult<usize> {
        // TODO: Support SeekFrom from python side as in IOBase.seek definition
        let r = Seek::seek(self, SeekFrom::Start(position as u64))?;
        Ok(r as usize)
    }
    pub fn seekable(&self) -> bool {
        true
    }
    pub fn tell(&self) -> usize {
        self.inner.position() as usize
    }
    pub fn readinto(&mut self, output: &PyAny) -> PyResult<usize> {
        let out = output.extract::<BytesType>()?;
        let r = readinto(self, out)?;
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
        BytesType::ByteArray(data) => {
            let mut array = Cursor::new(unsafe { data.as_bytes() });
            copy(&mut array, output)?
        }
        BytesType::Bytes(data) => {
            let buffer = data.as_bytes();
            copy(&mut Cursor::new(buffer), output)?
        }
    };
    Ok(result)
}

fn read<'a, R: Read>(reader: &mut R, py: Python<'a>, n_bytes: Option<usize>) -> PyResult<&'a PyBytes> {
    match n_bytes {
        Some(n) => {
            let mut buf = vec![0; n];
            reader.read(buf.as_mut_slice())?;
            Ok(PyBytes::new(py, buf.as_slice()))
        }
        None => {
            let mut buf = vec![];
            reader.read_to_end(&mut buf)?;
            Ok(PyBytes::new(py, buf.as_slice()))
        }
    }
}

fn readinto<R: Read>(reader: &mut R, output: BytesType) -> std::io::Result<u64> {
    let result = match output {
        BytesType::RustyFile(out) => copy(reader, &mut out.borrow_mut().inner)?,
        BytesType::RustyBuffer(out) => copy(reader, &mut out.borrow_mut().inner)?,
        BytesType::ByteArray(out) => {
            let mut array = WriteablePyByteArray::from(out);
            copy(reader, &mut array)?
        }
        BytesType::Bytes(out) => {
            // TODO: official API support from PyO3 is probably better; coerce it into &mut [u8]
            let ptr = out.as_bytes().as_ptr();
            let mut buffer = unsafe { std::slice::from_raw_parts_mut(ptr as *mut _, out.as_bytes().len()) };
            copy(reader, &mut buffer)?
        }
    };
    Ok(result)
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
