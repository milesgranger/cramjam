use std::fs::{File, OpenOptions};
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

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

    pub fn write(&mut self, buf: &[u8]) -> PyResult<usize> {
        let r = Write::write(self, buf)?;
        Ok(r)
    }
    pub fn read<'a>(&mut self, py: Python<'a>, n_bytes: Option<usize>) -> PyResult<&'a PyBytes> {
        match n_bytes {
            Some(n) => {
                let mut buf = vec![0; n];
                self.inner.read(buf.as_mut_slice())?;
                Ok(PyBytes::new(py, buf.as_slice()))
            }
            None => {
                let mut buf = vec![];
                self.inner.read_to_end(&mut buf)?;
                Ok(PyBytes::new(py, buf.as_slice()))
            }
        }
    }
    pub fn seek(&mut self, position: usize) -> PyResult<usize> {
        let r = Seek::seek(self, SeekFrom::Start(position as u64))?;
        Ok(r as usize)
    }
    pub fn seekable(&self) -> bool {
        true
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

    pub fn write(&mut self, buf: &[u8]) -> PyResult<usize> {
        let r = Write::write(self, buf)?;
        Ok(r)
    }
    pub fn read<'a>(&mut self, py: Python<'a>, n_bytes: Option<usize>) -> PyResult<&'a PyBytes> {
        match n_bytes {
            Some(n) => {
                let mut buf = vec![0; n];
                self.inner.read(buf.as_mut_slice())?;
                Ok(PyBytes::new(py, buf.as_slice()))
            }
            None => {
                let mut buf = vec![];
                self.inner.read_to_end(&mut buf)?;
                Ok(PyBytes::new(py, buf.as_slice()))
            }
        }
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
    pub fn set_len(&mut self, size: usize) -> PyResult<()> {
        self.inner.get_mut().resize(size, 0);
        Ok(())
    }
    pub fn truncate(&mut self) -> PyResult<()> {
        self.inner.get_mut().truncate(0);
        Ok(())
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
