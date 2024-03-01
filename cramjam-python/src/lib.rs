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
//! >>> assert bytes(data) == bytes(decompressed)
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
pub mod bzip2;
pub mod deflate;
pub mod exceptions;
pub mod experimental;
pub mod gzip;
pub mod io;
pub mod lz4;
pub mod snappy;
pub mod zstd;

use io::{PythonBuffer, RustyBuffer};
use pyo3::prelude::*;

use crate::io::{AsBytes, RustyFile};
use exceptions::{CompressionError, DecompressionError};
use std::io::{Read, Seek, SeekFrom, Write};

/// Any possible input/output to de/compression algorithms.
/// Typically, as a Python user, you never have to worry about this object. It's exposed here in
/// the documentation to see what types are acceptable for de/compression functions.
#[derive(FromPyObject)]
pub enum BytesType<'a> {
    /// [`cramjam.Buffer`](io/struct.RustyBuffer.html)
    #[pyo3(transparent, annotation = "Buffer")]
    RustyBuffer(&'a PyCell<RustyBuffer>),
    /// [`cramjam.File`](io/struct.RustyFile.html)
    #[pyo3(transparent, annotation = "File")]
    RustyFile(&'a PyCell<RustyFile>),
    /// `object` implementing the Buffer Protocol
    #[pyo3(transparent, annotation = "pybuffer")]
    PyBuffer(PythonBuffer),
}

impl<'a> AsBytes for BytesType<'a> {
    fn as_bytes(&self) -> &[u8] {
        match self {
            BytesType::RustyBuffer(b) => {
                let py_ref = b.borrow();
                let bytes = py_ref.as_bytes();
                unsafe { std::slice::from_raw_parts(bytes.as_ptr(), bytes.len()) }
            }
            BytesType::PyBuffer(b) => b.as_slice(),
            BytesType::RustyFile(b) => {
                let py_ref = b.borrow();
                let bytes = py_ref.as_bytes();
                unsafe { std::slice::from_raw_parts(bytes.as_ptr(), bytes.len()) }
            }
        }
    }
    fn as_bytes_mut(&mut self) -> PyResult<&mut [u8]> {
        match self {
            BytesType::RustyBuffer(b) => {
                let mut py_ref = b.borrow_mut();
                let bytes = py_ref.as_bytes_mut()?;
                Ok(unsafe { std::slice::from_raw_parts_mut(bytes.as_mut_ptr(), bytes.len()) })
            }
            BytesType::PyBuffer(b) => b.as_slice_mut(),
            BytesType::RustyFile(b) => {
                let mut py_ref = b.borrow_mut();
                let bytes = py_ref.as_bytes_mut()?;
                Ok(unsafe { std::slice::from_raw_parts_mut(bytes.as_mut_ptr(), bytes.len()) })
            }
        }
    }
}

impl<'a> Write for BytesType<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let result = match self {
            BytesType::RustyBuffer(out) => out.borrow_mut().inner.write(buf)?,
            BytesType::RustyFile(out) => out.borrow_mut().inner.write(buf)?,
            BytesType::PyBuffer(out) => out.write(buf)?,
        };
        Ok(result)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            BytesType::RustyBuffer(b) => b.borrow_mut().flush(),
            BytesType::RustyFile(f) => f.borrow_mut().flush(),
            BytesType::PyBuffer(_) => Ok(()),
        }
    }
}
impl<'a> Read for BytesType<'a> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            BytesType::RustyBuffer(data) => data.borrow_mut().inner.read(buf),
            BytesType::RustyFile(data) => data.borrow_mut().inner.read(buf),
            BytesType::PyBuffer(data) => data.read(buf),
        }
    }
}
impl<'a> Seek for BytesType<'a> {
    fn seek(&mut self, style: SeekFrom) -> std::io::Result<u64> {
        match self {
            BytesType::RustyBuffer(b) => b.borrow_mut().inner.seek(style),
            BytesType::RustyFile(f) => f.borrow_mut().inner.seek(style),
            BytesType::PyBuffer(buf) => buf.seek(style),
        }
    }
}

impl<'a> BytesType<'a> {
    fn len(&self) -> usize {
        self.as_bytes().len()
    }
}

/// Macro for generating the implementation of de/compression against a variant interface
#[macro_export]
macro_rules! generic {
    // de/compress
    ($py:ident, $op:path[$input:expr], output_len=$output_len:ident $(, level=$level:ident)?) => {
        {
            use crate::io::RustyBuffer;

            let mut output: Vec<u8> = match $output_len {
                Some(len) => vec![0; len],
                None => vec![]
            };
            match $input {
                BytesType::RustyFile(f) => {
                    let borrowed = f.borrow();
                    let file = &borrowed.inner;
                    $py.allow_threads(|| {
                        $op(file, &mut Cursor::new(&mut output) $(, $level)? )
                    })
                },
                _ => {
                    let bytes = $input.as_bytes();
                    $py.allow_threads(|| {
                        $op(bytes, &mut Cursor::new(&mut output) $(, $level)? )
                    })
                }
            }.map(|_| RustyBuffer::from(output))
        }
    };
    // de/compress_into
    ($py:ident, $op:path[$input:ident, $output:ident] $(, level=$level:ident)?) => {
        {
            match $input {
                BytesType::RustyFile(f) => {
                    let borrowed = f.borrow();
                    let f_in = &borrowed.inner;
                    match $output {
                        BytesType::RustyFile(f) => {
                            let mut borrowed = f.borrow_mut();
                            let mut f_out = &mut borrowed.inner;
                            $py.allow_threads(|| {
                                $op(f_in, &mut f_out $(, $level)? )
                            })
                        },
                        BytesType::RustyBuffer(buffer) => {
                            let mut borrowed = buffer.borrow_mut();
                            let mut buf_out = &mut borrowed.inner;
                            $py.allow_threads(|| {
                                $op(f_in, &mut buf_out $(, $level)? )
                            })
                        },
                        _ => {
                            let bytes_out = $output.as_bytes_mut()?;
                            $py.allow_threads(|| {
                                $op(f_in, &mut Cursor::new(bytes_out) $(, $level)? )
                            })
                        }
                    }
                },
                _ =>  {
                    let bytes_in = $input.as_bytes();
                    match $output {
                        BytesType::RustyFile(f) => {
                            let mut borrowed = f.borrow_mut();
                            let mut f_out = &mut borrowed.inner;
                            $py.allow_threads(|| {
                                $op(bytes_in, &mut f_out $(, $level)? )
                            })
                        },
                        BytesType::RustyBuffer(buffer) => {
                            let mut borrowed = buffer.borrow_mut();
                            let mut buf_out = &mut borrowed.inner;
                            $py.allow_threads(|| {
                                $op(bytes_in, &mut buf_out $(, $level)? )
                            })
                        },
                        _ => {
                            let bytes_out = $output.as_bytes_mut()?;
                            $py.allow_threads(|| {
                                $op(bytes_in, &mut Cursor::new(bytes_out) $(, $level)?)
                            })
                        }
                    }
                }
            }
        }
    }
}

/// Generate a `Decompressor` from a library's decompressor which implements Read
#[macro_export]
macro_rules! make_decompressor {
    ($codec:ident) => {
        /// Decompressor object for streaming decompression
        /// **NB** This is mostly here for API complement to `Compressor`
        /// You'll almost always be statisfied with `de/compress` / `de/compress_into` functions.
        #[pyclass]
        pub struct Decompressor {
            inner: Option<Cursor<Vec<u8>>>,
        }
        #[pymethods]
        impl Decompressor {
            /// Initialize a new `Decompressor` instance.
            #[new]
            pub fn __init__() -> PyResult<Self> {
                Ok(Self {
                    inner: Some(Default::default()),
                })
            }

            /// Length of internal buffer containing decompressed data.
            pub fn len(&self) -> usize {
                self.inner
                    .as_ref()
                    .map(|c| c.get_ref().len())
                    .unwrap_or_else(|| 0)
            }

            /// Decompress this input into the inner buffer.
            pub fn decompress(&mut self, py: Python, mut input: BytesType) -> PyResult<usize> {
                match self.inner.as_mut() {
                    Some(ref mut inner) => match &mut input {
                        BytesType::RustyFile(f) => {
                            let mut borrowed = f.borrow_mut();
                            let f_in = &mut borrowed.inner;
                            py.allow_threads(|| libcramjam::$codec::decompress(f_in, inner).map_err(Into::into))
                        }
                        _ => {
                            let bytes = input.as_bytes();
                            py.allow_threads(|| {
                                libcramjam::$codec::decompress(&mut Cursor::new(bytes), inner).map_err(Into::into)
                            })
                        }
                    },
                    None => Err(DecompressionError::new_err(
                        "Appears `finish()` was called on this instance",
                    )),
                }
            }

            /// Flush and return current decompressed stream.
            pub fn flush(&mut self) -> PyResult<RustyBuffer> {
                match self.inner.as_mut() {
                    Some(ref mut inner) => {
                        let mut out = vec![];
                        std::mem::swap(&mut out, inner.get_mut());
                        inner.set_position(0);
                        Ok(RustyBuffer::from(out))
                    }
                    None => Err(DecompressionError::new_err(
                        "Appears `finish()` was called on this instance",
                    )),
                }
            }

            /// Consume the current Decompressor state and return the decompressed stream
            /// **NB** The Decompressor will not be usable after this method is called.
            pub fn finish(&mut self) -> PyResult<RustyBuffer> {
                match std::mem::take(&mut self.inner) {
                    Some(inner) => Ok(RustyBuffer::from(inner.into_inner())),
                    None => Err(DecompressionError::new_err(
                        "Appears `finish()` was called on this instance",
                    )),
                }
            }

            fn __len__(&self) -> usize {
                self.len()
            }
            fn __contains__(&self, py: Python, x: BytesType) -> bool {
                let bytes = x.as_bytes();
                py.allow_threads(|| {
                    self.inner
                        .as_ref()
                        .map(|c| c.get_ref().windows(bytes.len()).any(|w| w == bytes))
                        .unwrap_or_else(|| false)
                })
            }
            fn __repr__(&self) -> String {
                format!("Decompressor<len={}>", self.len())
            }
            fn __bool__(&self) -> bool {
                self.inner.is_some() && self.len() > 0
            }
        }
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
    make_submodule!(py -> m -> bzip2);
    make_submodule!(py -> m -> lz4);
    make_submodule!(py -> m -> gzip);
    make_submodule!(py -> m -> deflate);
    make_submodule!(py -> m -> zstd);
    make_submodule!(py -> m -> experimental);

    Ok(())
}
