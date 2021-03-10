use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};

use pyo3::prelude::*;
use pyo3::types::{PyBytes};


#[pyclass(name="File")]
pub struct RustyFile {
    inner: File, // preferably, this is R: Read, but generic structs cannot be exposed to Python
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
        let r = self.inner.write(buf)?;
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
        let r = self.inner.seek(SeekFrom::Start(position as u64)).map(|r| r as usize)?;
        Ok(r)
    }
    pub fn set_len(&mut self, size: usize) -> PyResult<()> {
        self.inner.set_len(size as u64)?;
        Ok(())
    }
    pub fn truncate(&mut self) -> PyResult<()> {
        self.set_len(0)
    }

}
