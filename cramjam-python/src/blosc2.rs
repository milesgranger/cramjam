//! snappy de/compression interface
use std::io::{self, BufReader, Cursor};

use crate::exceptions::{CompressionError, DecompressionError};
use crate::io::{AsBytes, RustyBuffer};
use crate::BytesType;
use libcramjam::blosc2::blosc2::schunk::{Chunk, SChunk, Storage};
use libcramjam::blosc2::blosc2::{CLevel, CParams, Codec, DParams, Filter};
use pyo3::exceptions::{self, PyRuntimeError};
use pyo3::prelude::*;
use pyo3::types::PySlice;
use pyo3::wrap_pyfunction;
use pyo3::PyResult;

pub(crate) fn init_py_module(m: &PyModule) -> PyResult<()> {
    libcramjam::blosc2::blosc2::init();

    let ncores = std::thread::available_parallelism().map(|v| v.get()).unwrap_or(1);
    libcramjam::blosc2::blosc2::set_nthreads(ncores);

    m.add_function(wrap_pyfunction!(compress, m)?)?;
    m.add_function(wrap_pyfunction!(decompress, m)?)?;

    m.add_function(wrap_pyfunction!(compress_into, m)?)?;
    m.add_function(wrap_pyfunction!(decompress_into, m)?)?;

    m.add_function(wrap_pyfunction!(compress_chunk, m)?)?;
    m.add_function(wrap_pyfunction!(decompress_chunk, m)?)?;

    m.add_function(wrap_pyfunction!(compress_chunk_into, m)?)?;
    m.add_function(wrap_pyfunction!(decompress_chunk_into, m)?)?;

    // extra functions helpful when using blosc2
    m.add_function(wrap_pyfunction!(get_version, m)?)?;
    m.add_function(wrap_pyfunction!(get_nthreads, m)?)?;
    m.add_function(wrap_pyfunction!(set_nthreads, m)?)?;
    m.add_function(wrap_pyfunction!(max_compressed_len, m)?)?;

    m.add_class::<Compressor>()?;
    m.add_class::<Decompressor>()?;

    m.add_class::<PySChunk>()?;
    m.add_class::<PyChunk>()?;
    m.add_class::<PyFilter>()?;
    m.add_class::<PyCLevel>()?;
    m.add_class::<PyCodec>()?;

    Ok(())
}

/// Compress into SChunk
#[pyfunction]
#[allow(unused_variables)]

pub fn compress(
    py: Python,
    input: BytesType,
    output_len: Option<usize>,
    typesize: Option<usize>,
    clevel: Option<PyCLevel>,
    filter: Option<PyFilter>,
    codec: Option<PyCodec>,
    nthreads: Option<usize>,
) -> PyResult<RustyBuffer> {
    if input.is_empty() {
        return Ok(RustyBuffer::from(vec![]));
    }

    let mut cparams = CParams::from_typesize(typesize.unwrap_or_else(|| input.itemsize()))
        .set_codec(codec.map_or_else(Codec::default, Into::into))
        .set_clevel(clevel.map_or_else(CLevel::default, Into::into))
        .set_filter(filter.map_or_else(Filter::default, Into::into))
        .set_nthreads(nthreads.unwrap_or_else(libcramjam::blosc2::blosc2::get_nthreads));
    let mut dparams = DParams::default().set_nthreads(nthreads.unwrap_or_else(libcramjam::blosc2::blosc2::get_nthreads));

    let storage = Storage::default()
        .set_contiguous(true)
        .set_cparams(&mut cparams)
        .set_dparams(&mut dparams);

    let mut schunk = SChunk::new(storage);
    io::copy(&mut BufReader::new(input), &mut schunk)?;
    schunk
        .into_vec()
        .map(RustyBuffer::from)
        .map_err(CompressionError::from_err)
}

/// Compress into output
#[pyfunction]
pub fn compress_into(
    _py: Python,
    input: BytesType,
    mut output: BytesType,
    typesize: Option<usize>,
    clevel: Option<PyCLevel>,
    filter: Option<PyFilter>,
    codec: Option<PyCodec>,
    nthreads: Option<usize>,
) -> PyResult<usize> {
    if input.is_empty() {
        return Ok(0);
    }

    let mut cparams = CParams::from_typesize(typesize.unwrap_or_else(|| input.itemsize()))
        .set_codec(codec.map_or_else(Codec::default, Into::into))
        .set_clevel(clevel.map_or_else(CLevel::default, Into::into))
        .set_filter(filter.map_or_else(Filter::default, Into::into))
        .set_nthreads(nthreads.unwrap_or_else(libcramjam::blosc2::blosc2::get_nthreads));
    let mut dparams = DParams::default().set_nthreads(nthreads.unwrap_or_else(libcramjam::blosc2::blosc2::get_nthreads));

    let storage = Storage::default()
        .set_contiguous(true)
        .set_cparams(&mut cparams)
        .set_dparams(&mut dparams);

    if let BytesType::RustyFile(_file) = output {
        return Err(pyo3::exceptions::PyNotImplementedError::new_err(
            "Output to File w/ blosc2 not implemented yet.",
        ));
        // storage = storage
        //     .set_urlpath(&file.borrow().path)
        //     .map_err(CompressionError::from_err)?;
    }

    let mut schunk = SChunk::new(storage);
    io::copy(&mut BufReader::new(input), &mut schunk)?;

    let nbytes = schunk.frame().map_err(CompressionError::from_err)?.len();
    match output {
        BytesType::RustyFile(_) => Ok(nbytes),
        _ => {
            let schunk_buf = schunk.into_vec().map_err(CompressionError::from_err)?;
            io::copy(&mut Cursor::new(schunk_buf), &mut output)?;
            Ok(nbytes)
        }
    }
}

/// Decompress a SChunk into buffer
#[pyfunction]
#[allow(unused_variables)]
pub fn decompress(py: Python, input: BytesType, output_len: Option<usize>) -> PyResult<RustyBuffer> {
    if input.is_empty() {
        return Ok(RustyBuffer::from(vec![]));
    }
    return crate::generic!(py, libcramjam::blosc2::decompress[input], output_len = output_len)
        .map_err(DecompressionError::from_err);
}

/// decompress into output
#[pyfunction]
pub fn decompress_into(py: Python, input: BytesType, mut output: BytesType) -> PyResult<usize> {
    if input.is_empty() {
        return Ok(0);
    }
    crate::generic!(py, libcramjam::blosc2::decompress[input, output]).map_err(DecompressionError::from_err)
}

/// Blosc2 decompression.
///
/// Python Example
/// --------------
/// ```python
/// >>> cramjam.blosc2.decompress(compressed_bytes, output_len=Optional[None])
/// ```
#[pyfunction]
#[allow(unused_variables)]
pub fn decompress_chunk(py: Python, data: BytesType, output_len: Option<usize>) -> PyResult<RustyBuffer> {
    let bytes = data.as_bytes();
    let buf = py
        .allow_threads(|| libcramjam::blosc2::decompress_chunk(bytes))
        .map(RustyBuffer::from)?;
    Ok(buf)
}

/// Decompress a Chunk into output
#[pyfunction]
pub fn decompress_chunk_into(py: Python, input: BytesType, mut output: BytesType) -> PyResult<usize> {
    let bytes = input.as_bytes();
    let out = output.as_bytes_mut()?;
    let nbytes = py.allow_threads(|| libcramjam::blosc2::decompress_chunk_into(bytes, out))?;
    Ok(nbytes)
}

/// Blosc2 compression, chunk format
///
/// Python Example
/// --------------
/// ```python
/// >>> _ = cramjam.blosc2.compress(b'some bytes here', typesize=1, clevel=CLevel.Nine, filter=Filter.Shuffle, codec=Codec.BloscLz)
/// ```
#[pyfunction]
#[allow(unused_variables)]
pub fn compress_chunk(
    py: Python,
    data: BytesType,
    typesize: Option<usize>,
    clevel: Option<PyCLevel>,
    filter: Option<PyFilter>,
    codec: Option<PyCodec>,
) -> PyResult<RustyBuffer> {
    let bytes = data.as_bytes();
    py.allow_threads(|| {
        let clevel = clevel.map(Into::into);
        let filter = filter.map(Into::into);
        let codec = codec.map(Into::into);
        libcramjam::blosc2::blosc2::compress(bytes, typesize, clevel, filter, codec)
    })
    .map_err(CompressionError::from_err)
    .map(RustyBuffer::from)
}

/// Compress a Chunk into output
#[pyfunction]
pub fn compress_chunk_into(
    py: Python,
    input: BytesType,
    mut output: BytesType,
    typesize: Option<usize>,
    clevel: Option<PyCLevel>,
    filter: Option<PyFilter>,
    codec: Option<PyCodec>,
) -> PyResult<usize> {
    let bytes = input.as_bytes();
    let out = output.as_bytes_mut()?;
    py.allow_threads(|| {
        let clevel = clevel.map(Into::into);
        let filter = filter.map(Into::into);
        let codec = codec.map(Into::into);
        libcramjam::blosc2::blosc2::compress_into(bytes, out, typesize, clevel, filter, codec)
    })
    .map_err(CompressionError::from_err)
}

/// A Compressor interface, using blosc2's SChunk
#[pyclass]
#[derive(Clone)]
pub struct Compressor(Option<SChunk>);

unsafe impl Send for Compressor {}

#[pymethods]
impl Compressor {
    /// Initialize a new `Compressor` instance.
    #[new]
    pub fn __init__(
        path: Option<String>,
        typesize: Option<usize>,
        clevel: Option<PyCLevel>,
        filter: Option<PyFilter>,
        codec: Option<PyCodec>,
        nthreads: Option<usize>,
    ) -> PyResult<Self> {
        let mut cparams = CParams::from_typesize(typesize.unwrap_or(1))
            .set_codec(codec.map_or_else(Codec::default, Into::into))
            .set_clevel(clevel.map_or_else(CLevel::default, Into::into))
            .set_filter(filter.map_or_else(Filter::default, Into::into))
            .set_nthreads(nthreads.unwrap_or_else(libcramjam::blosc2::blosc2::get_nthreads));
        let mut dparams =
            DParams::default().set_nthreads(nthreads.unwrap_or_else(libcramjam::blosc2::blosc2::get_nthreads));

        let mut storage = Storage::default()
            .set_contiguous(true)
            .set_cparams(&mut cparams)
            .set_dparams(&mut dparams);
        if let Some(pth) = path {
            storage = storage.set_urlpath(pth).map_err(CompressionError::from_err)?;
        }

        let schunk = SChunk::new(storage);
        Ok(Self(Some(schunk)))
    }

    /// Compress input into the current compressor's stream.
    pub fn compress(&mut self, input: BytesType) -> PyResult<usize> {
        match self.0.as_mut() {
            Some(schunk) => schunk
                .append_buffer(input.as_bytes())
                .map_err(CompressionError::from_err),
            None => Err(CompressionError::new_err("Compressor has been consumed")),
        }
    }

    /// Flush and return current compressed stream, if file-backed Schunk,
    /// then empty buf is returned
    pub fn flush(&mut self) -> PyResult<RustyBuffer> {
        match self.0.as_ref() {
            Some(schunk) => {
                let buf = schunk.frame().map_err(CompressionError::from_err)?;
                Ok(RustyBuffer::from(buf.to_vec()))
            }
            None => Err(CompressionError::new_err("Compressor has been consumed")),
        }
    }

    /// Consume the current compressor state and return the compressed stream
    /// **NB** The compressor will not be usable after this method is called.
    pub fn finish(&mut self) -> PyResult<RustyBuffer> {
        match std::mem::take(&mut self.0) {
            Some(schunk) => schunk
                .into_vec()
                .map_err(CompressionError::from_err)
                .map(RustyBuffer::from),
            None => Err(CompressionError::new_err("Compressor has been consumed")),
        }
    }
}

crate::make_decompressor!(blosc2);

/// Represents a single compressed 'chunk' of data. Analogous to Lz4 block or snappy's raw format in Blosc2
#[pyclass(name = "Chunk")]
pub struct PyChunk(Chunk);

#[pymethods]
impl PyChunk {
    /// Construct a Chunk from compressing
    #[classmethod]
    pub fn compress(
        _cls: &PyAny,
        src: BytesType,
        typesize: Option<usize>,
        clevel: Option<PyCLevel>,
        filter: Option<PyFilter>,
        codec: Option<PyCodec>,
    ) -> PyResult<Self> {
        let typesize = typesize.or_else(|| Some(src.itemsize()));
        let clevel = clevel.map(Into::into);
        let filter = filter.map(Into::into);
        let codec = codec.map(Into::into);
        let chunk =
            Chunk::compress(src.as_bytes(), typesize, clevel, filter, codec).map_err(CompressionError::from_err)?;
        Ok(Self(chunk))
    }

    /// Decompress this chunk into bytes buffer
    pub fn decompress(&self) -> PyResult<RustyBuffer> {
        self.0
            .decompress()
            .map_err(DecompressionError::from_err)
            .map(RustyBuffer::from)
    }

    /// Get raw bytes of this Chunk
    pub fn raw(&self) -> PyResult<&[u8]> {
        self.0
            .as_slice()
            .map_err(|e| exceptions::PyBufferError::new_err(e.to_string()))
    }

    /// repr
    pub fn __repr__(&self) -> PyResult<String> {
        let ratio = self.0.compression_ratio().map_err(CompressionError::from_err)?;
        let info = self.0.info().map_err(CompressionError::from_err)?;
        let repr = format!(
            "Chunk<cbytes={} nbytes={} compression_ratio={ratio:.2}>",
            info.cbytes(),
            info.nbytes()
        );
        Ok(repr)
    }
}

/// SChunk interface
#[pyclass(name = "SChunk")]
pub struct PySChunk {
    schunk: SChunk,
    from_bytes_cb: Option<PyObject>,
    to_bytes_cb: Option<PyObject>,
}

unsafe impl Send for PySChunk {}

// Trampoline function from PySChunk, since generics not allowed.
// Call a function on PyObject which may be BytesType, or have a `converter` python function to convert
// the input PyObject `buf` into a `BytesType`, then call the intended operation on the bytes
#[inline]
fn try_to_bytes_with_op<T, F>(py: Python, buf: PyObject, converter: Option<&PyObject>, op: F) -> PyResult<T>
where
    F: FnOnce(&[u8]) -> PyResult<T>,
{
    match buf.extract::<BytesType>(py) {
        Ok(bt) => op(bt.as_bytes()),
        Err(_) => {
            if let Some(to_bytes_cb) = &converter {
                let obj = to_bytes_cb.call(py, (&buf,), None)?;
                let bytestype = obj.extract::<BytesType>(py)?;
                op(bytestype.as_bytes())
            } else {
                let msg = "Could not convert to variant of `BytesType` and no `to_bytes_cb` function set";
                return Err(CompressionError::new_err(msg));
            }
        }
    }
    .map_err(CompressionError::from_err)
}

/// Helper function to convert a RustyBuffer to some other PyObject
/// as defined by user callback converter function
#[inline]
fn maybe_convert_buffer(py: Python, buf: RustyBuffer, converter: Option<&PyObject>) -> PyResult<PyObject> {
    match converter {
        Some(convert) => convert.call(py, (buf,), None),
        None => Ok(buf.into_py(py)),
    }
}

#[pymethods]
impl PySChunk {
    /// Construct a new SChunk
    #[new]
    pub fn __init__(
        path: Option<String>,
        typesize: Option<usize>,
        clevel: Option<PyCLevel>,
        filter: Option<PyFilter>,
        codec: Option<PyCodec>,
        nthreads: Option<usize>,
        from_bytes_cb: Option<PyObject>,
        to_bytes_cb: Option<PyObject>,
    ) -> PyResult<Self> {
        let mut cparams = CParams::from_typesize(typesize.unwrap_or(1))
            .set_codec(codec.map_or_else(Codec::default, Into::into))
            .set_clevel(clevel.map_or_else(CLevel::default, Into::into))
            .set_filter(filter.map_or_else(Filter::default, Into::into))
            .set_nthreads(nthreads.unwrap_or_else(libcramjam::blosc2::blosc2::get_nthreads));
        let mut dparams =
            DParams::default().set_nthreads(nthreads.unwrap_or_else(libcramjam::blosc2::blosc2::get_nthreads));

        let mut storage = Storage::default()
            .set_contiguous(true)
            .set_cparams(&mut cparams)
            .set_dparams(&mut dparams);
        if let Some(pth) = path {
            storage = storage.set_urlpath(pth).map_err(CompressionError::from_err)?;
        }

        let schunk = SChunk::new(storage);
        Ok(Self {
            schunk,
            from_bytes_cb,
            to_bytes_cb,
        })
    }

    /// Create a `SChunk` from `Compressor`
    #[classmethod]
    pub fn from_compressor(_cls: &PyAny, compressor: &PyAny) -> PyResult<Self> {
        let compressor: Compressor = compressor.extract()?;
        match compressor.0.as_ref() {
            Some(inner) => Ok(Self {
                schunk: inner.clone(),
                from_bytes_cb: None,
                to_bytes_cb: None,
            }),
            None => Err(exceptions::PyValueError::new_err(
                "Provided compressor has been consumed",
            )),
        }
    }

    /// Get a Compressor interface to this SChunk
    pub fn as_compressor(&self) -> Compressor {
        Compressor(Some(self.schunk.clone()))
    }

    /// Get a slice of decompressed data
    pub fn get_slice_buffer(&self, start: usize, stop: usize) -> PyResult<RustyBuffer> {
        self.schunk
            .get_slice_buffer(start, stop)
            .map(RustyBuffer::from)
            .map_err(CompressionError::from_err)
    }

    /// Get the typsize of the SChunk's items
    #[getter]
    pub fn typesize(&self) -> usize {
        self.schunk.typesize()
    }

    /// Number of uncompressed bytes
    #[getter]
    pub fn nbytes(&self) -> usize {
        self.schunk.nbytes()
    }

    /// Number of compressed bytes
    #[getter]
    pub fn cbytes(&self) -> usize {
        self.schunk.cbytes()
    }

    /// Get number of chunks in this SChunk
    #[getter]
    pub fn nchunks(&self) -> usize {
        self.schunk.n_chunks()
    }

    /// Current compression ratio
    #[getter]
    pub fn compression_ratio(&self) -> f32 {
        self.schunk.compression_ratio()
    }

    /// Get the SChunk file path, if any.
    #[getter]
    pub fn path(&self) -> Option<std::path::PathBuf> {
        self.schunk.path()
    }

    /// Append/compress a buffer into this SChunk, returning the new number of chunks
    pub fn append_buffer(&mut self, py: Python, buf: PyObject) -> PyResult<usize> {
        try_to_bytes_with_op(py, buf, self.to_bytes_cb.as_ref(), |bytes| {
            self.schunk.append_buffer(bytes).map_err(CompressionError::from_err)
        })
    }

    /// Decompress a specific chunk
    pub fn decompress_chunk(&mut self, py: Python, nchunk: usize) -> PyResult<PyObject> {
        self.schunk
            .decompress_chunk_vec(nchunk)
            .map_err(DecompressionError::from_err)
            .map(RustyBuffer::from)
            .and_then(|buf| maybe_convert_buffer(py, buf, self.from_bytes_cb.as_ref()))
    }

    /// Get a specific Chunk from this SChunk
    pub fn get_chunk(&self, nchunk: usize) -> PyResult<PyChunk> {
        self.schunk
            .get_chunk(nchunk)
            .map_err(CompressionError::from_err)
            .map(PyChunk)
    }

    /// Return the current _raw_ SChunk frame data
    pub fn frame(&self) -> PyResult<&[u8]> {
        self.schunk.frame().map_err(CompressionError::from_err)
    }

    /// Get a slice of SChunk (uncompressed)
    pub fn __getitem__(&self, py: Python, slice: &PySlice) -> PyResult<PyObject> {
        let indices = slice.indices(self.len() as _)?;
        self.schunk
            .get_slice_buffer(indices.start as _, indices.stop as _)
            .map(|buf| {
                buf.chunks_exact(self.typesize())
                    .step_by(indices.step as _)
                    .flatten()
                    .map(Clone::clone)
                    .collect::<Vec<u8>>()
            })
            .map(RustyBuffer::from)
            .map_err(DecompressionError::from_err)
            .and_then(|buf| maybe_convert_buffer(py, buf, self.from_bytes_cb.as_ref()))
    }

    /// Set a slice of the SChunk (will compress data given)
    pub fn __setitem__(&self, py: Python, slice: &PySlice, buf: PyObject) -> PyResult<()> {
        let indices = slice.indices(self.len() as _)?;
        if indices.step != 1 {
            return Err(CompressionError::new_err(
                "Setting with a step other than 1 not implemented",
            ));
        }
        try_to_bytes_with_op(py, buf, self.to_bytes_cb.as_ref(), |bytes| {
            self.schunk
                .set_slice_buffer(indices.start as _, indices.stop as _, bytes)
                .map_err(CompressionError::from_err)
        })
    }

    /// Length (in items size) of SChunk
    pub fn len(&self) -> usize {
        self.schunk.len()
    }

    /// Numer of items in this SChunk
    pub fn __len__(&self) -> usize {
        self.len()
    }

    /// Repr for SChunk
    pub fn __repr__(&self) -> String {
        let len = self.schunk.len();
        let ratio = self.schunk.compression_ratio();
        let nchunks = self.schunk.n_chunks();
        let nbytes = self.schunk.nbytes();
        let cbytes = self.schunk.cbytes();
        format!("SChunk<nitems={len} nchunks={nchunks} nbytes={nbytes} cbytes={cbytes} compression_ratio={ratio:.2}>")
    }
}

#[pyclass(name = "Filter")]
#[allow(missing_docs)]
#[derive(Clone)]
pub enum PyFilter {
    NoFilter,
    Shuffle,
    BitShuffle,
    Delta,
    TruncPrec,
    LastFilter,
    LastRegisteredFilter,
}

impl Into<Filter> for PyFilter {
    #[inline]
    fn into(self) -> Filter {
        match self {
            Self::NoFilter => Filter::NoFilter,
            Self::Shuffle => Filter::Shuffle,
            Self::BitShuffle => Filter::BitShuffle,
            Self::Delta => Filter::Delta,
            Self::TruncPrec => Filter::TruncPrec,
            Self::LastFilter => Filter::LastFilter,
            Self::LastRegisteredFilter => Filter::LastRegisteredFilter,
        }
    }
}

#[pyclass(name = "CLevel")]
#[allow(missing_docs)]
#[derive(Clone)]
pub enum PyCLevel {
    Zero,
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
}

impl Into<CLevel> for PyCLevel {
    #[inline]
    fn into(self) -> CLevel {
        match self {
            Self::Zero => CLevel::Zero,
            Self::One => CLevel::One,
            Self::Two => CLevel::Two,
            Self::Three => CLevel::Three,
            Self::Four => CLevel::Four,
            Self::Five => CLevel::Five,
            Self::Six => CLevel::Six,
            Self::Seven => CLevel::Seven,
            Self::Eight => CLevel::Eight,
            Self::Nine => CLevel::Nine,
        }
    }
}

#[pyclass(name = "Codec")]
#[allow(missing_docs)]
#[derive(Clone)]
pub enum PyCodec {
    BloscLz,
    LZ4,
    LZ4HC,
    ZLIB,
    ZSTD,
    LastCodec,
    LastRegisteredCodec,
}

impl Into<Codec> for PyCodec {
    #[inline]
    fn into(self) -> Codec {
        match self {
            Self::BloscLz => Codec::BloscLz,
            Self::LZ4 => Codec::LZ4,
            Self::LZ4HC => Codec::LZ4HC,
            Self::ZLIB => Codec::ZLIB,
            Self::ZSTD => Codec::ZSTD,
            Self::LastCodec => Codec::LastCodec,
            Self::LastRegisteredCodec => Codec::LastRegisteredCodec,
        }
    }
}

/// Set number of threads, returning previous number
#[pyfunction]
pub fn set_nthreads(n: usize) -> usize {
    libcramjam::blosc2::blosc2::set_nthreads(n)
}

/// get current number of threads set
#[pyfunction]
pub fn get_nthreads() -> usize {
    libcramjam::blosc2::blosc2::get_nthreads()
}

/// Print the blosc2 library version
#[pyfunction]
pub fn get_version() -> PyResult<String> {
    let version =
        libcramjam::blosc2::blosc2::get_version_string().map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
    Ok(format!("{}", version))
}

/// Get the max compressed size of some raw input length in bytes.
#[pyfunction]
pub fn max_compressed_len(len_bytes: usize) -> usize {
    libcramjam::blosc2::blosc2::max_compress_len_bytes(len_bytes)
}
