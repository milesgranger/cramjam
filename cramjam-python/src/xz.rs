//! xz and lzma de/compression interface
use pyo3::prelude::*;
use pyo3::PyResult;

use crate::exceptions::{CompressionError, DecompressionError};
use crate::io::{AsBytes, RustyBuffer};
use crate::BytesType;
use pyo3::exceptions::PyNotImplementedError;
use pyo3::wrap_pyfunction;
use std::io::Cursor;

pub(crate) fn init_py_module(m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(compress, m)?)?;
    m.add_function(wrap_pyfunction!(decompress, m)?)?;
    m.add_function(wrap_pyfunction!(compress_into, m)?)?;
    m.add_function(wrap_pyfunction!(decompress_into, m)?)?;
    m.add_class::<Format>()?;
    m.add_class::<Check>()?;
    m.add_class::<Filter>()?;
    m.add_class::<FilterChainItem>()?;
    m.add_class::<FilterChain>()?;
    m.add_class::<Options>()?;
    m.add_class::<Mode>()?;
    m.add_class::<MatchFinder>()?;
    m.add_class::<Compressor>()?;
    m.add_class::<Decompressor>()?;
    Ok(())
}

/// LZMA compression.
///
/// Python Example
/// --------------
/// ```python
/// >>> _ = cramjam.xz.compress(b'some bytes here')
/// >>> # Defaults to XZ format, you can use the deprecated LZMA format like this:
/// >>> _ = cramjam.xz.compress(b'some bytes here', format=cramjam.xz.Format.ALONE)
/// ```
#[pyfunction]
pub fn compress(
    py: Python,
    data: BytesType,
    preset: Option<u32>,
    format: Option<Format>,
    check: Option<Check>,
    filters: Option<FilterChain>,
    options: Option<Options>,
    output_len: Option<usize>,
) -> PyResult<RustyBuffer> {
    crate::generic!(
        py,
        libcramjam::xz::compress[data],
        output_len = output_len,
        preset,
        format,
        check,
        filters,
        options
    )
    .map_err(CompressionError::from_err)
}

/// Compress directly into an output buffer
#[pyfunction]
pub fn compress_into(
    py: Python,
    input: BytesType,
    mut output: BytesType,
    preset: Option<u32>,
    format: Option<Format>,
    check: Option<Check>,
    filters: Option<FilterChain>,
    options: Option<Options>,
) -> PyResult<usize> {
    crate::generic!(py, libcramjam::xz::compress[input, output], preset, format, check, filters, options)
        .map_err(CompressionError::from_err)
}

/// LZMA decompression.
///
/// Python Example
/// --------------
/// ```python
/// >>> # bytes or bytearray; bytearray is faster
/// >>> cramjam.xz.decompress(compressed_bytes, output_len=Optional[None])
/// ```
#[pyfunction]
pub fn decompress(py: Python, data: BytesType, output_len: Option<usize>) -> PyResult<RustyBuffer> {
    crate::generic!(py, libcramjam::xz::decompress[data], output_len = output_len).map_err(DecompressionError::from_err)
}

/// Decompress directly into an output buffer
#[pyfunction]
pub fn decompress_into(py: Python, input: BytesType, mut output: BytesType) -> PyResult<usize> {
    crate::generic!(py, libcramjam::xz::decompress[input, output]).map_err(DecompressionError::from_err)
}
/// XZ Compressor object for streaming compression
#[pyclass]
pub struct Compressor {
    inner: Option<libcramjam::xz::xz2::write::XzEncoder<Cursor<Vec<u8>>>>,
}

#[pymethods]
impl Compressor {
    /// Initialize a new `Compressor` instance.
    #[new]
    pub fn __init__(preset: Option<u32>) -> PyResult<Self> {
        let preset = preset.unwrap_or(5);
        let inner = libcramjam::xz::xz2::write::XzEncoder::new(Cursor::new(vec![]), preset);
        Ok(Self { inner: Some(inner) })
    }

    /// Compress input into the current compressor's stream.
    pub fn compress(&mut self, input: &[u8]) -> PyResult<usize> {
        crate::io::stream_compress(&mut self.inner, input)
    }

    /// Flush and return current compressed stream
    pub fn flush(&mut self) -> PyResult<RustyBuffer> {
        Err(PyNotImplementedError::new_err(
            "`.flush` for XZ/LZMA not implemented, just use `.finish()` instead when your done.",
        ))
    }

    /// Consume the current compressor state and return the compressed stream
    /// **NB** The compressor will not be usable after this method is called.
    pub fn finish(&mut self) -> PyResult<RustyBuffer> {
        crate::io::stream_finish(&mut self.inner, |inner| inner.finish().map(|c| c.into_inner()))
    }
}

crate::make_decompressor!(xz);

/// Available Filter IDs
#[derive(Clone, Debug)]
#[pyclass]
#[allow(missing_docs)]
pub enum Filter {
    Arm,
    ArmThumb,
    Ia64,
    Lzma1,
    Lzma2,
    PowerPC,
    Sparc,
    X86,
}
impl Default for Filter {
    fn default() -> Self {
        Self::Lzma2
    }
}

/// MatchFinder, used with Options.mf attribute
#[derive(Clone, Debug)]
#[pyclass]
#[allow(missing_docs)]
pub enum MatchFinder {
    HashChain3,
    HashChain4,
    BinaryTree2,
    BinaryTree3,
    BinaryTree4,
}

impl Into<libcramjam::xz::MatchFinder> for MatchFinder {
    fn into(self) -> libcramjam::xz::MatchFinder {
        match self {
            Self::HashChain3 => libcramjam::xz::MatchFinder::HashChain3,
            Self::HashChain4 => libcramjam::xz::MatchFinder::HashChain4,
            Self::BinaryTree2 => libcramjam::xz::MatchFinder::BinaryTree2,
            Self::BinaryTree3 => libcramjam::xz::MatchFinder::BinaryTree3,
            Self::BinaryTree4 => libcramjam::xz::MatchFinder::BinaryTree4,
        }
    }
}

/// MatchFinder, used with Options.mode attribute
#[derive(Clone, Debug)]
#[pyclass]
#[allow(missing_docs)]
pub enum Mode {
    Fast,
    Normal,
}
impl Into<libcramjam::xz::Mode> for Mode {
    fn into(self) -> libcramjam::xz::Mode {
        match self {
            Self::Fast => libcramjam::xz::Mode::Fast,
            Self::Normal => libcramjam::xz::Mode::Normal,
        }
    }
}

/// FilterChain, similar to the default Python XZ filter chain which is a list of
/// dicts.
#[derive(Debug, Clone)]
#[pyclass]
pub struct FilterChain(Vec<FilterChainItem>);

#[pymethods]
#[allow(missing_docs)]
impl FilterChain {
    #[new]
    pub fn __init__() -> Self {
        Self(vec![])
    }
    pub fn append_filter(&mut self, filter_chain_item: FilterChainItem) {
        self.0.push(filter_chain_item);
    }
}

impl Into<libcramjam::xz::Filters> for FilterChain {
    fn into(self) -> libcramjam::xz::Filters {
        let mut filters = libcramjam::xz::Filters::new();
        for filter in self.0 {
            match filter.filter {
                Filter::Lzma1 => filters.lzma1(&filter.try_into().unwrap()),
                Filter::Lzma2 => filters.lzma2(&filter.try_into().unwrap()),
                Filter::Arm => filters.arm(),
                Filter::ArmThumb => filters.arm_thumb(),
                Filter::Ia64 => filters.ia64(),
                Filter::PowerPC => filters.powerpc(),
                Filter::Sparc => filters.sparc(),
                Filter::X86 => filters.x86(),
            };
        }
        filters
    }
}

/// FilterChainItem. In Python's lzma module, this represents a single dict in the
/// filter chain list. To be added to the `FilterChain`
#[derive(Clone, Debug, Default)]
#[pyclass]
pub struct FilterChainItem {
    filter: Filter,
    options: Options,
}

#[pymethods]
impl FilterChainItem {
    #[new]
    #[allow(missing_docs)]
    pub fn __init__(filter: Filter, options: Option<Options>) -> Self {
        Self {
            filter,
            options: options.unwrap_or_default(),
        }
    }
}

///
#[derive(Clone, Debug, Default)]
#[pyclass]
pub struct Options {
    preset: Option<u32>,
    dict_size: Option<u32>,
    lc: Option<u32>,
    lp: Option<u32>,
    pb: Option<u32>,
    mode: Option<Mode>,
    nice_len: Option<usize>,
    mf: Option<MatchFinder>,
    depth: Option<usize>,
}

impl Into<libcramjam::xz::LzmaOptions> for FilterChainItem {
    fn into(self) -> libcramjam::xz::LzmaOptions {
        self.options.into()
    }
}

impl Into<libcramjam::xz::LzmaOptions> for Options {
    fn into(self) -> libcramjam::xz::LzmaOptions {
        let mut opts = libcramjam::xz::LzmaOptions::new_preset(self.preset.unwrap_or(6)).unwrap();
        self.dict_size.map(|dict_size| opts.dict_size(dict_size));
        self.lc.map(|lc| opts.literal_context_bits(lc));
        self.lp.map(|lp| opts.literal_position_bits(lp));
        self.pb.map(|pb| opts.position_bits(pb));
        self.mode.map(|mode| opts.mode(mode.into()));
        self.nice_len.map(|nice_len| opts.nice_len(nice_len as _));
        self.mf.map(|mf| opts.match_finder(mf.into()));
        self.depth.map(|depth| opts.depth(depth as _));
        opts
    }
}

#[pymethods]
#[allow(missing_docs)]
impl Options {
    #[new]
    pub fn __init__() -> Self {
        Self::default()
    }
    pub fn set_preset(&mut self, preset: u32) -> Self {
        self.preset = Some(preset);
        self.clone()
    }
    pub fn set_dict_size(&mut self, dict_size: u32) -> Self {
        self.dict_size = Some(dict_size);
        self.clone()
    }
    pub fn set_lc(&mut self, lc: u32) -> Self {
        self.lc = Some(lc);
        self.clone()
    }
    pub fn set_lp(&mut self, lp: u32) -> Self {
        self.lp = Some(lp);
        self.clone()
    }
    pub fn set_pb(&mut self, pb: u32) -> Self {
        self.pb = Some(pb);
        self.clone()
    }
    pub fn set_mode(&mut self, mode: Mode) -> Self {
        self.mode = Some(mode);
        self.clone()
    }
    pub fn set_nice_len(&mut self, nice_len: usize) -> Self {
        self.nice_len = Some(nice_len);
        self.clone()
    }
    pub fn set_mf(&mut self, mf: MatchFinder) -> Self {
        self.mf = Some(mf);
        self.clone()
    }
    pub fn set_depth(&mut self, depth: usize) -> Self {
        self.depth = Some(depth);
        self.clone()
    }
}

/// Possible formats
#[derive(Clone, Debug)]
#[pyclass]
pub enum Format {
    /// Auto select the format, for compression this is XZ,
    /// for decompression it will be determined by the compressed input.
    AUTO,
    /// The `.xz` format (default)
    XZ,
    /// Legacy `.lzma` format.
    ALONE,
    /// Raw data stream
    RAW,
}

impl Default for Format {
    fn default() -> Self {
        Format::XZ
    }
}
impl Into<libcramjam::xz::Format> for Format {
    fn into(self) -> libcramjam::xz::Format {
        match self {
            Self::AUTO => libcramjam::xz::Format::AUTO,
            Self::XZ => libcramjam::xz::Format::XZ,
            Self::ALONE => libcramjam::xz::Format::ALONE,
            Self::RAW => libcramjam::xz::Format::RAW,
        }
    }
}

/// Possible Check configurations
#[derive(Debug, Clone)]
#[pyclass]
#[allow(missing_docs)]
pub enum Check {
    Crc64,
    Crc32,
    Sha256,
    None,
}

impl Into<libcramjam::xz::Check> for Check {
    fn into(self) -> libcramjam::xz::Check {
        match self {
            Self::Crc64 => libcramjam::xz::Check::Crc64,
            Self::Crc32 => libcramjam::xz::Check::Crc32,
            Self::Sha256 => libcramjam::xz::Check::Sha256,
            Self::None => libcramjam::xz::Check::None,
        }
    }
}
