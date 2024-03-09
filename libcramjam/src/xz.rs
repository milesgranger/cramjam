//! LZMA / XZ de/compression interface
//! Note this is still a bit of a work in progress, especially when it comes
//! to filter chain support.
use std::io::{self, BufRead, BufReader};
use std::io::{Read, Result, Write};
pub use xz2;
use xz2::read::{XzDecoder, XzEncoder};
use xz2::stream::{Check as xz2Check, Stream, TELL_ANY_CHECK};
pub use xz2::stream::{Filters, LzmaOptions, MatchFinder, Mode};

/// Possible formats
#[derive(Clone, Debug, Copy)]
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

/// Possible Check configurations
#[derive(Debug, Clone, Copy)]
pub enum Check {
    Crc64,
    Crc32,
    Sha256,
    None,
}

impl Into<xz2Check> for Check {
    fn into(self) -> xz2Check {
        match self {
            Self::Crc64 => xz2Check::Crc64,
            Self::Crc32 => xz2Check::Crc32,
            Self::Sha256 => xz2Check::Sha256,
            Self::None => xz2Check::None,
        }
    }
}

/// Decompress snappy data framed
#[inline(always)]
pub fn decompress<W: Write + ?Sized, R: Read>(input: R, output: &mut W) -> Result<usize> {
    let xz_magicbytes = b"\xfd7zXZ\x00";
    let mut input = BufReader::new(input);
    let stream = {
        let innerbuf = input.fill_buf()?;
        if innerbuf.len() >= xz_magicbytes.len() && &innerbuf[..xz_magicbytes.len()] == xz_magicbytes {
            Stream::new_auto_decoder(u64::MAX, TELL_ANY_CHECK)?
        } else {
            Stream::new_lzma_decoder(u64::MAX)?
        }
    };
    let mut decoder = XzDecoder::new_stream(input, stream);
    let n_bytes = io::copy(&mut decoder, output)?;
    Ok(n_bytes as usize)
}

/// Decompress snappy data framed
#[inline(always)]
pub fn compress<W: Write + ?Sized, R: Read>(
    data: R,
    output: &mut W,
    preset: Option<u32>,
    format: Option<impl Into<Format>>,
    check: Option<impl Into<Check>>,
    filters: Option<impl Into<Filters>>,
    options: Option<impl Into<LzmaOptions>>,
) -> Result<usize> {
    let preset = preset.unwrap_or(6); // same as python default
    let stream = match format.map(Into::into).unwrap_or_default() {
        Format::AUTO | Format::XZ => {
            let check = check.map(Into::into).unwrap_or(Check::Crc64); // default for xz
            let stream = Stream::new_easy_encoder(preset, check.into())?;
            stream
        }
        Format::ALONE => {
            let opts = match options {
                Some(opts) => opts.into(),
                None => LzmaOptions::new_preset(preset)?,
            };
            let stream = Stream::new_lzma_encoder(&opts)?;
            stream
        }
        Format::RAW => {
            let check = check.map(Into::into).unwrap_or(Check::None); // default for Alone and Raw formats
            let filters = filters.map(Into::into).unwrap_or_else(|| Filters::new());
            let stream = Stream::new_stream_encoder(&filters, check.into())?;
            stream
        }
    };
    let mut encoder = XzEncoder::new_stream(data, stream);
    let n_bytes = io::copy(&mut encoder, output)?;
    Ok(n_bytes as usize)
}
