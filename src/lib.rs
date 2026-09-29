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

pub mod exceptions;
pub mod experimental;
pub mod io;

#[cfg(any(feature = "blosc2", feature = "blosc2-static", feature = "blosc2-shared"))]
pub mod blosc2;
#[cfg(feature = "brotli")]
pub mod brotli;
#[cfg(feature = "bzip2")]
pub mod bzip2;
#[cfg(any(feature = "deflate", feature = "deflate-static", feature = "deflate-shared"))]
pub mod deflate;
#[cfg(any(feature = "gzip", feature = "gzip-static", feature = "gzip-shared"))]
pub mod gzip;
#[cfg(all(
    any(feature = "ideflate", feature = "ideflate-static", feature = "ideflate-shared"),
    target_pointer_width = "64"
))]
pub mod ideflate;
#[cfg(all(
    any(feature = "igzip", feature = "igzip-static", feature = "igzip-shared"),
    target_pointer_width = "64"
))]
pub mod igzip;
#[cfg(all(
    any(feature = "izlib", feature = "izlib-static", feature = "izlib-shared"),
    target_pointer_width = "64"
))]
pub mod izlib;
#[cfg(feature = "lz4")]
pub mod lz4;
#[cfg(feature = "snappy")]
pub mod snappy;
#[cfg(any(feature = "xz", feature = "xz-static", feature = "xz-shared"))]
pub mod xz;
#[cfg(any(feature = "zlib", feature = "zlib-static", feature = "zlib-shared"))]
pub mod zlib;
#[cfg(feature = "zstd")]
pub mod zstd;

use io::{PythonBuffer, RustyBuffer};
use pyo3::prelude::*;

use crate::io::RustyFile;
use exceptions::{CompressionError, DecompressionError};
use std::io::{Read, Seek, SeekFrom, Write};
use std::ops::{Deref, DerefMut};

/// Any possible input/output to de/compression algorithms.
/// Typically, as a Python user, you never have to worry about this object. It's exposed here in
/// the documentation to see what types are acceptable for de/compression functions.
pub enum BytesType<'a> {
    /// [`cramjam.Buffer`](io/struct.RustyBuffer.html)
    RustyBuffer(Bound<'a, RustyBuffer>),
    /// [`cramjam.File`](io/struct.RustyFile.html)
    RustyFile(Bound<'a, RustyFile>),
    /// `object` implementing the Buffer Protocol
    PyBuffer(PythonBuffer),
}

// Hand-written rather than derived: the derive tries each variant in turn and builds a
// formatted Python exception for every miss, ~1.7 µs per call for the most common inputs
// (bytes, bytearray, numpy), which all fall through to the last variant.
impl<'a, 'py> FromPyObject<'a, 'py> for BytesType<'py> {
    type Error = PyErr;

    fn extract(obj: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        if let Ok(buffer) = obj.cast::<RustyBuffer>() {
            return Ok(Self::RustyBuffer(buffer.to_owned()));
        }
        if let Ok(file) = obj.cast::<RustyFile>() {
            return Ok(Self::RustyFile(file.to_owned()));
        }
        PythonBuffer::try_from(&*obj).map(Self::PyBuffer).map_err(|cause| {
            let expected = "expected Buffer, File or a C-contiguous bytes-like object";
            let msg = match obj.get_type().name() {
                Ok(name) => format!("{expected}, not '{name}'"),
                Err(_) => expected.to_string(),
            };
            let err = pyo3::exceptions::PyTypeError::new_err(msg);
            err.set_cause(obj.py(), Some(cause));
            err
        })
    }
}

/// Bytes borrowed from a [`BytesType`]. For `Buffer` this holds the PyCell borrow, so the
/// underlying Vec can't be resized/freed (e.g. `set_len` from another thread) while in use,
/// including while the GIL is released.
pub(crate) enum BytesRef<'a> {
    Buffer(PyRef<'a, RustyBuffer>),
    Slice(&'a [u8]),
}

impl Deref for BytesRef<'_> {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        match self {
            Self::Buffer(b) => b.inner.get_ref(),
            Self::Slice(s) => s,
        }
    }
}

/// Mutable counterpart of [`BytesRef`], holding the exclusive PyCell borrow for `Buffer`.
pub(crate) enum BytesRefMut<'a> {
    Buffer(PyRefMut<'a, RustyBuffer>),
    Slice(&'a mut [u8]),
}

impl Deref for BytesRefMut<'_> {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        match self {
            Self::Buffer(b) => b.inner.get_ref(),
            Self::Slice(s) => s,
        }
    }
}

impl DerefMut for BytesRefMut<'_> {
    fn deref_mut(&mut self) -> &mut [u8] {
        match self {
            Self::Buffer(b) => b.inner.get_mut().as_mut_slice().expect("checked in as_bytes_mut"),
            Self::Slice(s) => s,
        }
    }
}

const FILE_AS_BYTES_ERR: &str = "Converting a File to bytes is not supported, as it'd require reading the \
    entire file into memory; consider using cramjam.Buffer";

impl<'a> BytesType<'a> {
    /// Borrow the underlying bytes; the borrow is held for as long as the returned guard lives.
    pub(crate) fn as_bytes(&self) -> PyResult<BytesRef<'_>> {
        match self {
            BytesType::RustyBuffer(b) => Ok(BytesRef::Buffer(b.try_borrow()?)),
            BytesType::PyBuffer(b) => Ok(BytesRef::Slice(b.as_slice())),
            BytesType::RustyFile(_) => Err(pyo3::exceptions::PyTypeError::new_err(FILE_AS_BYTES_ERR)),
        }
    }
    /// Mutably borrow the underlying bytes; the borrow is held for as long as the returned guard lives.
    pub(crate) fn as_bytes_mut(&mut self) -> PyResult<BytesRefMut<'_>> {
        match self {
            BytesType::RustyBuffer(b) => {
                let mut buf = b.try_borrow_mut()?;
                buf.inner.get_mut().as_mut_slice()?; // a view of a read-only source errors here
                Ok(BytesRefMut::Buffer(buf))
            }
            BytesType::PyBuffer(b) => Ok(BytesRefMut::Slice(b.as_slice_mut()?)),
            BytesType::RustyFile(_) => Err(pyo3::exceptions::PyTypeError::new_err(FILE_AS_BYTES_ERR)),
        }
    }
}

impl<'a> Write for BytesType<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let result = match self {
            BytesType::RustyBuffer(out) => Write::write(&mut *out.borrow_mut(), buf)?,
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
    /// Length in bytes
    fn len(&self) -> PyResult<usize> {
        match self {
            BytesType::RustyFile(file) => file.try_borrow()?.len(),
            _ => Ok(self.as_bytes()?.len()),
        }
    }
    /// The item size, in bytes, that the buffer/bytes represent.
    #[allow(dead_code)]
    fn itemsize(&self) -> usize {
        match self {
            Self::PyBuffer(pybuffer) => pybuffer.inner.itemsize as _,
            _ => 1,
        }
    }
    /// Empty
    #[allow(dead_code)]
    fn is_empty(&self) -> PyResult<bool> {
        Ok(self.len()? == 0)
    }
}

/// Macro for generating the implementation of de/compression against a variant interface
#[macro_export]
macro_rules! generic {
    // de/compress
    ($py:ident, $op:path[$input:expr], output_len = $output_len:ident $(, $args:ident)*) => {
        {
            use crate::io::RustyBuffer;

            let mut output: Vec<u8> = match $output_len {
                Some(len) => vec![0; len],
                None => vec![]
            };
            match $input {
                BytesType::RustyFile(f) => {
                    let borrowed = f.try_borrow()?;
                    let file = &borrowed.inner;
                    $py.detach(|| {
                        $op(file, &mut Cursor::new(&mut output) $(, $args)* )
                    })
                },
                _ => {
                    let bytes: &[u8] = &$input.as_bytes()?;
                    $py.detach(|| {
                        $op(bytes, &mut Cursor::new(&mut output) $(, $args)* )
                    })
                }
            }.map(|_| RustyBuffer::from(output))
        }
    };
    // de/compress_into
    ($py:ident, $op:path[$input:ident, $output:ident] $(, $args:ident)*) => {
        {
            match $input {
                BytesType::RustyFile(f) => {
                    let borrowed = f.try_borrow()?;
                    let f_in = &borrowed.inner;
                    match $output {
                        BytesType::RustyFile(f) => {
                            let mut borrowed = f.try_borrow_mut()?;
                            let mut f_out = &mut borrowed.inner;
                            $py.detach(|| {
                                $op(f_in, &mut f_out $(, $args)* )
                            })
                        },
                        BytesType::RustyBuffer(buffer) => {
                            let mut borrowed = buffer.try_borrow_mut()?;
                            let mut buf_out = &mut *borrowed;
                            $py.detach(|| {
                                $op(f_in, &mut buf_out $(, $args)* )
                            })
                        },
                        _ => {
                            let bytes_out: &mut [u8] = &mut $output.as_bytes_mut()?;
                            $py.detach(|| {
                                $op(f_in, &mut Cursor::new(bytes_out) $(, $args)* )
                            })
                        }
                    }
                },
                _ =>  {
                    let bytes_in: &[u8] = &$input.as_bytes()?;
                    match $output {
                        BytesType::RustyFile(f) => {
                            let mut borrowed = f.try_borrow_mut()?;
                            let mut f_out = &mut borrowed.inner;
                            $py.detach(|| {
                                $op(bytes_in, &mut f_out $(, $args)* )
                            })
                        },
                        BytesType::RustyBuffer(buffer) => {
                            let mut borrowed = buffer.try_borrow_mut()?;
                            let mut buf_out = &mut *borrowed;
                            $py.detach(|| {
                                $op(bytes_in, &mut buf_out $(, $args)* )
                            })
                        },
                        _ => {
                            let bytes_out: &mut [u8] = &mut $output.as_bytes_mut()?;
                            $py.detach(|| {
                                $op(bytes_in, &mut Cursor::new(bytes_out) $(, $args)*)
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
                            let mut borrowed = f.try_borrow_mut()?;
                            let f_in = &mut borrowed.inner;
                            py.detach(|| libcramjam::$codec::decompress(f_in, inner).map_err(Into::into))
                        }
                        _ => {
                            let bytes: &[u8] = &input.as_bytes()?;
                            py.detach(|| {
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
            fn __contains__(&self, py: Python, x: BytesType) -> PyResult<bool> {
                let bytes: &[u8] = &x.as_bytes()?;
                Ok(py.detach(|| {
                    self.inner
                        .as_ref()
                        .map(|c| c.get_ref().windows(bytes.len()).any(|w| w == bytes))
                        .unwrap_or_else(|| false)
                }))
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

#[pymodule]
mod cramjam {
    use super::*;

    #[pymodule_init]
    fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
        m.add("__version__", env!("CARGO_PKG_VERSION"))?;
        m.add_class::<crate::io::RustyFile>()?;
        m.add_class::<crate::io::RustyBuffer>()?;
        Ok(())
    }

    #[pymodule_export]
    use crate::CompressionError;

    #[pymodule_export]
    use crate::DecompressionError;

    #[cfg(feature = "snappy")]
    #[pymodule_export]
    use crate::snappy::snappy;

    #[cfg(feature = "zstd")]
    #[pymodule_export]
    use crate::zstd::zstd;

    #[cfg(feature = "lz4")]
    #[pymodule_export]
    use crate::lz4::lz4;

    #[cfg(any(feature = "brotli"))]
    #[pymodule_export]
    use crate::brotli::brotli;

    #[cfg(any(feature = "xz", feature = "xz-static", feature = "xz-shared"))]
    #[pymodule_export]
    use crate::xz::xz;

    #[cfg(feature = "bzip2")]
    #[pymodule_export]
    use crate::bzip2::bzip2;

    #[cfg(any(feature = "gzip", feature = "gzip-static", feature = "gzip-shared"))]
    #[pymodule_export]
    use crate::gzip::gzip;

    #[cfg(any(feature = "zlib", feature = "zlib-static", feature = "zlib-shared"))]
    #[pymodule_export]
    use crate::zlib::zlib;

    #[cfg(any(feature = "deflate", feature = "deflate-static", feature = "deflate-shared"))]
    #[pymodule_export]
    use crate::deflate::deflate;

    #[pymodule_export]
    use crate::experimental::experimental;
}
