//! lz4 de/compression interface
pub use lz4;
use std::io::{BufReader, Cursor, Error, Read, Write};

pub const DEFAULT_COMPRESSION_LEVEL: u32 = 4;
pub const LZ4_ACCELERATION_MAX: u32 = 65537;

#[inline(always)]
pub fn make_write_compressor<W: Write>(output: W, level: Option<u32>) -> Result<lz4::Encoder<W>, Error> {
    let comp = lz4::EncoderBuilder::new()
        .level(level.unwrap_or(DEFAULT_COMPRESSION_LEVEL))
        .auto_flush(true)
        .favor_dec_speed(true)
        .build(output)?;
    Ok(comp)
}

/// Decompress lz4 data
#[inline(always)]
pub fn decompress<W: Write + ?Sized, R: Read>(input: R, output: &mut W) -> Result<usize, Error> {
    let mut decoder = lz4::Decoder::new(input)?;
    let n_bytes = std::io::copy(&mut decoder, output)?;
    decoder.finish().1?;
    Ok(n_bytes as usize)
}

#[inline(always)]
pub fn compress_bound(input_len: usize, level: Option<u32>) -> usize {
    let mut prefs: std::mem::MaybeUninit<lz4::liblz4::LZ4FPreferences> = std::mem::MaybeUninit::zeroed();
    let prefs_ptr = prefs.as_mut_ptr();
    unsafe {
        std::ptr::write(
            std::ptr::addr_of_mut!((*prefs_ptr).compression_level),
            level.unwrap_or(DEFAULT_COMPRESSION_LEVEL),
        )
    };

    let n = unsafe { lz4::liblz4::LZ4F_compressBound(input_len, prefs.as_ptr()) };
    unsafe { std::ptr::drop_in_place(std::ptr::addr_of_mut!((*prefs_ptr).compression_level)) };
    n
}

/// Compress lz4 data
#[inline(always)]
pub fn compress<W: Write + ?Sized, R: Read>(input: R, output: &mut W, level: Option<u32>) -> Result<usize, Error> {
    // Can add an additional constraint to `Seek` for output but that is not great for API
    // so very unfortunately, we have an intermediate buffer to get bytes written to output
    // as lz4::Encoder is Write only
    let out_buffer = vec![];
    let mut encoder = make_write_compressor(out_buffer, level)?;

    // this returns, bytes read from uncompressed, input; we want bytes written
    // but lz4 only implements Read for Encoder
    let mut buf = BufReader::new(input);
    std::io::copy(&mut buf, &mut encoder)?;
    let (w, r) = encoder.finish();
    r?;

    // Now copy bytes from temp output buffer to actual output, returning number of bytes written to 'output'.
    let nbytes = std::io::copy(&mut Cursor::new(w), output)?;
    Ok(nbytes as _)
}

pub mod block {
    use lz4::block::CompressionMode;
    use std::io::Error;

    const PREPEND_SIZE: bool = true;

    #[inline(always)]
    pub fn compress_bound(input_len: usize, prepend_size: Option<bool>) -> usize {
        match lz4::block::compress_bound(input_len) {
            Ok(len) => {
                if prepend_size.unwrap_or(true) {
                    len + 4
                } else {
                    len
                }
            }
            Err(_) => 0,
        }
    }

    /// Decompress into Vec. Must have been compressed with prepended uncompressed size.
    /// will panic otherwise.
    #[inline(always)]
    pub fn decompress_vec(input: &[u8]) -> Result<Vec<u8>, Error> {
        if input.len() < 4 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Input not long enough",
            ));
        }
        let bytes: [u8; 4] = input[..4].try_into().unwrap();
        let len = u32::from_le_bytes(bytes);
        let mut buf = vec![0u8; len as usize];
        let nbytes = decompress_into(&input[4..], &mut buf, Some(false))?;
        buf.truncate(nbytes);
        Ok(buf)
    }

    /// NOTE: input is expected to **not** have the size prepended. Calling decompress_into is
    /// saying you already know the output buffer min size. `output` can be larger, but it cannot
    /// be smaller than what's required.
    #[inline(always)]
    pub fn decompress_into(input: &[u8], output: &mut [u8], size_prepended: Option<bool>) -> Result<usize, Error> {
        let uncompressed_size = if size_prepended.is_some_and(|v| v) {
            None // decompress_to_buffer will read from prepended size
        } else {
            Some(output.len() as _)
        };
        let nbytes = lz4::block::decompress_to_buffer(input, uncompressed_size, output)?;
        Ok(nbytes)
    }

    #[inline(always)]
    pub fn compress_vec(
        input: &[u8],
        level: Option<u32>,
        acceleration: Option<i32>,
        prepend_size: Option<bool>,
    ) -> Result<Vec<u8>, Error> {
        let len = compress_bound(input.len(), prepend_size);
        let mut buffer = vec![0u8; len];
        let nbytes = compress_into(input, &mut buffer, level, acceleration, prepend_size)?;
        buffer.truncate(nbytes);
        Ok(buffer)
    }

    #[inline(always)]
    pub fn compress_into(
        input: &[u8],
        output: &mut [u8],
        level: Option<u32>,
        acceleration: Option<i32>,
        prepend_size: Option<bool>,
    ) -> Result<usize, Error> {
        let prepend_size = prepend_size.unwrap_or(PREPEND_SIZE);
        let mode = compression_mode(None, level.map(|v| v as _), acceleration)?;
        let nbytes = lz4::block::compress_to_buffer(input, Some(mode), prepend_size, output)?;
        Ok(nbytes)
    }

    #[inline]
    fn compression_mode(
        mode: Option<&str>,
        compression: Option<i32>,
        acceleration: Option<i32>,
    ) -> Result<CompressionMode, Error> {
        let m = match mode {
            Some(m) => match m {
                "default" => CompressionMode::DEFAULT,
                "fast" => CompressionMode::FAST(acceleration.unwrap_or(1)),
                "high_compression" => CompressionMode::HIGHCOMPRESSION(compression.unwrap_or(9)),
                _ => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Invalid compression string, needed one of 'default', 'fast', or 'high_compression'",
                    ))
                }
            },
            None => CompressionMode::DEFAULT,
        };
        Ok(m)
    }

    #[cfg(test)]
    mod tests {

        use super::{compress_vec, decompress_into, decompress_vec};

        const DATA: &[u8; 14] = b"howdy neighbor";

        #[test]
        fn round_trip_store_size() {
            let compressed = compress_vec(DATA, None, None, Some(true)).unwrap();
            let decompressed = decompress_vec(&compressed).unwrap();
            assert_eq!(&decompressed, DATA);
        }
        #[test]
        fn round_trip_no_store_size() {
            let compressed = compress_vec(DATA, None, None, Some(false)).unwrap();

            // decompressed_vec depends on prepended_size, so we can't use that.
            assert!(decompress_vec(&compressed).is_err());

            let mut decompressed = vec![0u8; DATA.len()];
            decompress_into(&compressed, &mut decompressed, Some(false)).unwrap();
            assert_eq!(&decompressed, DATA);

            // decompressed_into will allow a larger output buffer than what's needed
            let mut decompressed = vec![0u8; DATA.len() + 5_000];
            let n = decompress_into(&compressed, &mut decompressed, Some(false)).unwrap();
            assert_eq!(&decompressed[..n], DATA);
        }
    }
}
