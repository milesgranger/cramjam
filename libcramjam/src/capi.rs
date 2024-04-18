use libc::c_void;

use std::ffi::{c_char, CString};
use std::io::Cursor;
use std::io::Write;
use std::slice;

#[cfg(feature = "brotli")]
use crate::brotli;
#[cfg(feature = "bzip2")]
use crate::bzip2;
#[cfg(feature = "deflate")]
use crate::deflate;
#[cfg(feature = "gzip")]
use crate::gzip;
#[cfg(feature = "lz4")]
use crate::lz4;
#[cfg(feature = "snappy")]
use crate::snappy;
#[cfg(feature = "zstd")]
use crate::zstd;

#[repr(C)]
pub struct Buffer {
    data: *const u8,
    len: usize,
    owned: bool,
}

impl Buffer {
    pub fn empty() -> Self {
        Buffer {
            data: std::ptr::null(),
            len: 0,
            owned: false,
        }
    }
}

impl From<&Vec<u8>> for Buffer {
    fn from(v: &Vec<u8>) -> Self {
        Buffer {
            data: v.as_ptr(),
            len: v.len(),
            owned: false,
        }
    }
}
impl From<Vec<u8>> for Buffer {
    fn from(mut v: Vec<u8>) -> Self {
        v.shrink_to_fit();
        let buffer = Buffer {
            data: v.as_ptr(),
            len: v.len(),
            owned: true,
        };
        std::mem::forget(v);
        buffer
    }
}

/// All codecs supported by the de/compress and de/compress_into APIs
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub enum Codec {
    #[cfg(feature = "snappy")]
    #[allow(dead_code)]
    Snappy,

    #[cfg(feature = "snappy")]
    #[allow(dead_code)]
    SnappyRaw,

    #[cfg(feature = "bzip2")]
    #[allow(dead_code)]
    Bzip2,

    #[cfg(feature = "lz4")]
    #[allow(dead_code)]
    Lz4,

    #[cfg(feature = "lz4")]
    #[allow(dead_code)]
    Lz4Block,

    #[cfg(feature = "zstd")]
    #[allow(dead_code)]
    Zstd,

    #[cfg(feature = "gzip")]
    #[allow(dead_code)]
    Gzip,

    #[cfg(feature = "brotli")]
    #[allow(dead_code)]
    Brotli,
}

/// Streaming only codecs, which can create De/Compressors using the de/compressor APIs
#[derive(Debug)]
#[repr(C)]
pub enum StreamingCodec {
    #[cfg(feature = "bzip2")]
    #[allow(dead_code)]
    StreamingBzip2,

    #[cfg(feature = "snappy")]
    #[allow(dead_code)]
    StreamingSnappy,

    #[cfg(feature = "lz4")]
    #[allow(dead_code)]
    StreamingLz4,

    #[cfg(feature = "zstd")]
    #[allow(dead_code)]
    StreamingZstd,

    #[cfg(feature = "gzip")]
    #[allow(dead_code)]
    StreamingGzip,

    #[cfg(feature = "brotli")]
    #[allow(dead_code)]
    StreamingBrotli,
}

#[cfg(feature = "snappy")]
type SnappyFrameCompressor = snappy::snap::write::FrameEncoder<Vec<u8>>;
#[cfg(feature = "bzip2")]
type Bzip2Compressor = bzip2::bzip2::write::BzEncoder<Vec<u8>>;
#[cfg(feature = "lz4")]
type Lz4Compressor = crate::lz4::lz4::Encoder<Vec<u8>>;
#[cfg(feature = "gzip")]
type GzipCompressor = crate::gzip::flate2::write::GzEncoder<Vec<u8>>;
#[cfg(feature = "brotli")]
type BrotliCompressor = brotli::brotli::CompressorWriter<Vec<u8>>;
#[cfg(feature = "zstd")]
type ZstdCompressor<'a> = crate::zstd::zstd::Encoder<'a, Vec<u8>>;

type Decompressor = Cursor<Vec<u8>>;

// Set the error string to a error message pointer
#[inline(always)]
fn error_to_ptr(err: impl ToString, ptr: &mut *mut c_char) {
    let err_msg = CString::new(err.to_string()).unwrap();
    *ptr = err_msg.into_raw();
}

/// Safe to call on a nullptr
#[no_mangle]
pub extern "C" fn free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        let _ = unsafe { CString::from_raw(ptr) };
    }
}

#[no_mangle]
pub extern "C" fn free_buffer(buf: Buffer) {
    if !buf.data.is_null() && buf.owned {
        let _ = unsafe { Vec::from_raw_parts(buf.data as *mut u8, buf.len, buf.len) };
    }
}

#[no_mangle]
pub extern "C" fn decompress(
    codec: Codec,
    input: *const u8,
    input_len: usize,
    nbytes_read: &mut usize,
    nbytes_written: &mut usize,
    error: &mut *mut c_char,
) -> Buffer {
    let mut decompressed = Cursor::new(vec![]);
    let mut compressed = Cursor::new(unsafe { std::slice::from_raw_parts(input, input_len) });
    let ret: Result<usize, std::io::Error> = match codec {
        #[cfg(feature = "snappy")]
        Codec::Snappy => snappy::decompress(&mut compressed, &mut decompressed),
        #[cfg(feature = "snappy")]
        Codec::SnappyRaw => snappy::raw::decompress_vec(compressed.get_ref()).map(|v| {
            let len = v.len();
            *decompressed.get_mut() = v;
            decompressed.set_position(len as _);
            compressed.set_position(input_len as _); // todo, assuming it read the whole thing
            len
        }),
        #[cfg(feature = "bzip2")]
        Codec::Bzip2 => bzip2::decompress(&mut compressed, &mut decompressed),
        #[cfg(feature = "brotli")]
        Codec::Brotli => brotli::decompress(&mut compressed, &mut decompressed),
        #[cfg(feature = "gzip")]
        Codec::Gzip => gzip::decompress(&mut compressed, &mut decompressed),
        #[cfg(feature = "zstd")]
        Codec::Zstd => zstd::decompress(&mut compressed, &mut decompressed),
        #[cfg(feature = "lz4")]
        Codec::Lz4 => lz4::decompress(&mut compressed, &mut decompressed),
        #[cfg(feature = "lz4")]
        Codec::Lz4Block => lz4::block::decompress_vec(compressed.get_ref()).map(|v| {
            let len = v.len();
            *decompressed.get_mut() = v;
            decompressed.set_position(len as _);
            compressed.set_position(input_len as _); // todo, assuming it read the whole thing
            len
        }),
    };
    match ret {
        Ok(n) => {
            *nbytes_read = compressed.position() as usize;
            *nbytes_written = n;
            match decompressed.flush() {
                Ok(_) => Buffer::from(decompressed.into_inner()),
                Err(err) => {
                    error_to_ptr(err, error);
                    Buffer::empty()
                }
            }
        }
        Err(err) => {
            error_to_ptr(err, error);
            Buffer::empty()
        }
    }
}

#[no_mangle]
pub extern "C" fn compress(
    codec: Codec,
    level: i32,
    input: *const u8,
    input_len: usize,
    nbytes_read: &mut usize,
    nbytes_written: &mut usize,
    error: &mut *mut c_char,
) -> Buffer {
    if level < 0 {
        error_to_ptr("Requires compression >= 0", error);
        return Buffer::empty();
    }
    let level = Some(level);
    let mut compressed = Cursor::new(vec![]);
    let mut decompressed = Cursor::new(unsafe { std::slice::from_raw_parts(input, input_len) });
    let ret: Result<usize, std::io::Error> = match codec {
        #[cfg(feature = "snappy")]
        Codec::Snappy => snappy::compress(&mut decompressed, &mut compressed),
        #[cfg(feature = "snappy")]
        Codec::SnappyRaw => snappy::raw::compress_vec(decompressed.get_ref()).map(|v| {
            let len = v.len();
            *compressed.get_mut() = v;
            compressed.set_position(len as _);
            decompressed.set_position(input_len as _);
            len
        }),
        #[cfg(feature = "bzip2")]
        Codec::Bzip2 => bzip2::compress(&mut decompressed, &mut compressed, level.map(|v| v as _)),
        #[cfg(feature = "brotli")]
        Codec::Brotli => brotli::compress(&mut decompressed, &mut compressed, level.map(|v| v as _)),
        #[cfg(feature = "gzip")]
        Codec::Gzip => gzip::compress(&mut decompressed, &mut compressed, level.map(|v| v as _)),
        #[cfg(feature = "zstd")]
        Codec::Zstd => zstd::compress(&mut decompressed, &mut compressed, level.map(|v: i32| v as i32)),
        #[cfg(feature = "lz4")]
        Codec::Lz4 => lz4::compress(&mut decompressed, &mut compressed, level.map(|v| v as _)),
        // TODO: Support passing acceleration
        #[cfg(feature = "lz4")]
        Codec::Lz4Block => lz4::block::compress_vec(decompressed.get_ref(), level.map(|v| v as _), None, Some(true))
            .map(|v| {
                let len = v.len();
                *compressed.get_mut() = v;
                compressed.set_position(len as _);
                decompressed.set_position(input_len as _);
                len
            }), // TODO
    };
    match ret {
        Ok(n) => {
            *nbytes_read = decompressed.get_ref().len();
            *nbytes_written = n;
            match compressed.flush() {
                Ok(_) => Buffer::from(compressed.into_inner()),
                Err(err) => {
                    error_to_ptr(err, error);
                    Buffer::empty()
                }
            }
        }
        Err(err) => {
            error_to_ptr(err, error);
            Buffer::empty()
        }
    }
}

#[no_mangle]
pub extern "C" fn decompress_into(
    codec: Codec,
    input: *const u8,
    input_len: usize,
    output: *mut u8,
    output_len: usize,
    nbytes_read: &mut usize,
    nbytes_written: &mut usize,
    error: &mut *mut c_char,
) {
    let mut compressed = Cursor::new(unsafe { std::slice::from_raw_parts(input, input_len) });
    let mut decompressed = Cursor::new(unsafe { std::slice::from_raw_parts_mut(output, output_len) });

    let ret: Result<usize, std::io::Error> = match codec {
        #[cfg(feature = "snappy")]
        Codec::Snappy => snappy::decompress(&mut compressed, &mut decompressed),
        #[cfg(feature = "snappy")]
        Codec::SnappyRaw => snappy::raw::decompress(compressed.get_ref(), decompressed.get_mut()),
        #[cfg(feature = "bzip2")]
        Codec::Bzip2 => bzip2::decompress(&mut compressed, &mut decompressed),
        #[cfg(feature = "brotli")]
        Codec::Brotli => brotli::decompress(&mut compressed, &mut decompressed),
        #[cfg(feature = "gzip")]
        Codec::Gzip => gzip::decompress(&mut compressed, &mut decompressed),
        #[cfg(feature = "zstd")]
        Codec::Zstd => zstd::decompress(&mut compressed, &mut decompressed),
        #[cfg(feature = "lz4")]
        Codec::Lz4 => lz4::decompress(&mut compressed, &mut decompressed),
        #[cfg(feature = "lz4")]
        Codec::Lz4Block => lz4::block::decompress_into(&compressed.get_ref(), decompressed.get_mut(), None),
    };
    match ret {
        Ok(n) => {
            *nbytes_written = n;
            *nbytes_read = compressed.get_ref().len();
        }
        Err(err) => {
            error_to_ptr(err, error);
            *nbytes_written = 0;
            *nbytes_read = 0;
        }
    }
}

#[no_mangle]
pub extern "C" fn compress_into(
    codec: Codec,
    level: i32,
    input: *const u8,
    input_len: usize,
    output: *mut u8,
    output_len: usize,
    nbytes_read: &mut usize,
    nbytes_written: &mut usize,
    error: &mut *mut c_char,
) {
    let mut decompressed = unsafe { std::slice::from_raw_parts(input, input_len) };
    let mut compressed = unsafe { std::slice::from_raw_parts_mut(output, output_len) };

    if level < 0 {
        error_to_ptr("Requires compression >= 0", error);
        return;
    }
    let level = Some(level);

    let ret: Result<usize, std::io::Error> = match codec {
        #[cfg(feature = "snappy")]
        Codec::Snappy => snappy::compress(&mut decompressed, &mut compressed),
        #[cfg(feature = "snappy")]
        Codec::SnappyRaw => snappy::raw::compress(decompressed, &mut compressed),
        #[cfg(feature = "bzip2")]
        Codec::Bzip2 => bzip2::compress(&mut decompressed, &mut compressed, level.map(|v| v as _)),
        #[cfg(feature = "brotli")]
        Codec::Brotli => brotli::compress(&mut decompressed, &mut compressed, level.map(|v| v as _)),
        #[cfg(feature = "gzip")]
        Codec::Gzip => gzip::compress(&mut decompressed, &mut compressed, level.map(|v| v as _)),
        #[cfg(feature = "zstd")]
        Codec::Zstd => zstd::compress(&mut decompressed, &mut compressed, level.map(|v: i32| v as i32)),
        #[cfg(feature = "lz4")]
        Codec::Lz4 => lz4::compress(&mut decompressed, &mut compressed, level.map(|v| v as _)),
        // TODO: Support passing acceleration
        #[cfg(feature = "lz4")]
        Codec::Lz4Block => lz4::block::compress_into(decompressed, compressed, level.map(|v| v as _), None, Some(true)),
    };
    match ret {
        Ok(n) => {
            *nbytes_written = n;
            *nbytes_read = decompressed.len();
        }
        Err(err) => {
            error_to_ptr(err, error);
            *nbytes_written = 0;
            *nbytes_read = 0;
        }
    }
}

/* ---------- Streaming Compressor --------------- */
#[no_mangle]
#[allow(unused_variables)]
pub extern "C" fn compressor_init(codec: StreamingCodec, level: i32, error: &mut *mut c_char) -> *mut c_void {
    match codec {
        #[cfg(feature = "bzip2")]
        StreamingCodec::StreamingBzip2 => {
            if level < 0 {
                error_to_ptr("Bzip2 requires compression level >= 0", error);
                return std::ptr::null_mut();
            }
            let compressor = bzip2::bzip2::write::BzEncoder::new(vec![], bzip2::bzip2::Compression::new(level as _));
            Box::into_raw(Box::new(compressor)) as _
        }
        #[cfg(feature = "brotli")]
        StreamingCodec::StreamingBrotli => {
            if level < 0 {
                error_to_ptr("Brotli requires compression level >= 0", error);
                return std::ptr::null_mut();
            }
            let compressor = brotli::make_write_compressor(vec![], Some(level as _));
            Box::into_raw(Box::new(compressor)) as _
        }
        #[cfg(feature = "gzip")]
        StreamingCodec::StreamingGzip => {
            if level < 1 {
                error_to_ptr("Gzip requires compression level >= 1", error);
                return std::ptr::null_mut();
            }
            let compressor = gzip::flate2::write::GzEncoder::new(vec![], gzip::flate2::Compression::new(level as _));
            Box::into_raw(Box::new(compressor)) as _
        }
        #[cfg(feature = "zstd")]
        StreamingCodec::StreamingZstd => {
            let compressor = zstd::zstd::Encoder::new(vec![], level);
            Box::into_raw(Box::new(compressor)) as _
        }
        #[cfg(feature = "snappy")]
        StreamingCodec::StreamingSnappy => {
            let compressor = snappy::snap::write::FrameEncoder::new(vec![]);
            Box::into_raw(Box::new(compressor)) as _
        }
        #[cfg(feature = "lz4")]
        StreamingCodec::StreamingLz4 => {
            if level < 0 {
                error_to_ptr("Lz4 requires compression level >= 0", error);
                return std::ptr::null_mut();
            }
            let compressor = lz4::make_write_compressor(vec![], Some(level as _));
            Box::into_raw(Box::new(compressor)) as _
        }
    }
}

#[no_mangle]
pub extern "C" fn free_compressor(codec: StreamingCodec, compressor_ptr: &mut *mut c_void) {
    if !(*compressor_ptr).is_null() {
        {
            match codec {
                #[cfg(feature = "bzip2")]
                StreamingCodec::StreamingBzip2 => {
                    let _ = unsafe { Box::from_raw(*compressor_ptr as *mut Bzip2Compressor) };
                }
                #[cfg(feature = "brotli")]
                StreamingCodec::StreamingBrotli => {
                    let _ = unsafe { Box::from_raw(*compressor_ptr as *mut BrotliCompressor) };
                }
                #[cfg(feature = "gzip")]
                StreamingCodec::StreamingGzip => {
                    let _ = unsafe { Box::from_raw(*compressor_ptr as *mut GzipCompressor) };
                }
                #[cfg(feature = "zstd")]
                StreamingCodec::StreamingZstd => {
                    let _ = unsafe { Box::from_raw(*compressor_ptr as *mut ZstdCompressor) };
                }
                #[cfg(feature = "snappy")]
                StreamingCodec::StreamingSnappy => {
                    let _ = unsafe { Box::from_raw(*compressor_ptr as *mut SnappyFrameCompressor) };
                }
                #[cfg(feature = "lz4")]
                StreamingCodec::StreamingLz4 => {
                    let _ = unsafe { Box::from_raw(*compressor_ptr as *mut Lz4Compressor) };
                }
            }
        }
        *compressor_ptr = std::ptr::null_mut();
    }
}

#[no_mangle]
pub extern "C" fn compressor_inner(codec: StreamingCodec, compressor_ptr: &mut *mut c_void) -> Buffer {
    match codec {
        #[cfg(feature = "bzip2")]
        StreamingCodec::StreamingBzip2 => {
            let compressor = unsafe { Box::from_raw(*compressor_ptr as *mut Bzip2Compressor) };
            let buffer = Buffer::from(compressor.get_ref());
            *compressor_ptr = Box::into_raw(compressor) as _;
            buffer
        }
        #[cfg(feature = "brotli")]
        StreamingCodec::StreamingBrotli => {
            let compressor = unsafe { Box::from_raw(*compressor_ptr as *mut BrotliCompressor) };
            let buffer = Buffer::from(compressor.get_ref());
            *compressor_ptr = Box::into_raw(compressor) as _;
            buffer
        }
        #[cfg(feature = "gzip")]
        StreamingCodec::StreamingGzip => {
            let compressor = unsafe { Box::from_raw(*compressor_ptr as *mut GzipCompressor) };
            let buffer = Buffer::from(compressor.get_ref());
            *compressor_ptr = Box::into_raw(compressor) as _;
            buffer
        }
        #[cfg(feature = "zstd")]
        StreamingCodec::StreamingZstd => {
            let compressor = unsafe { Box::from_raw(*compressor_ptr as *mut ZstdCompressor) };
            let buffer = Buffer::from(compressor.get_ref());
            *compressor_ptr = Box::into_raw(compressor) as _;
            buffer
        }
        #[cfg(feature = "snappy")]
        StreamingCodec::StreamingSnappy => {
            let compressor = unsafe { Box::from_raw(*compressor_ptr as *mut SnappyFrameCompressor) };
            let buffer = Buffer::from(compressor.get_ref());
            *compressor_ptr = Box::into_raw(compressor) as _;
            buffer
        }
        #[cfg(feature = "lz4")]
        StreamingCodec::StreamingLz4 => {
            let compressor = unsafe { Box::from_raw(*compressor_ptr as *mut Lz4Compressor) };
            let buffer = Buffer::from(compressor.writer());
            *compressor_ptr = Box::into_raw(compressor) as _;
            buffer
        }
    }
}

/// Finish the decompression stream and return the underlying buffer, transfering ownership to caller
#[no_mangle]
pub extern "C" fn compressor_finish(
    codec: StreamingCodec,
    compressor_ptr: &mut *mut c_void,
    error: &mut *mut c_char,
) -> Buffer {
    let buf = match codec {
        #[cfg(feature = "bzip2")]
        StreamingCodec::StreamingBzip2 => {
            let compressor = unsafe { Box::from_raw(*compressor_ptr as *mut Bzip2Compressor) };
            match compressor.finish() {
                Ok(buf) => Buffer::from(buf),
                Err(err) => {
                    error_to_ptr(err, error);
                    Buffer::empty()
                }
            }
        }
        #[cfg(feature = "brotli")]
        StreamingCodec::StreamingBrotli => {
            let mut compressor = unsafe { Box::from_raw(*compressor_ptr as *mut BrotliCompressor) };
            if let Err(err) = compressor.flush() {
                error_to_ptr(err, error);
                return Buffer::empty();
            }
            Buffer::from(compressor.into_inner())
        }
        #[cfg(feature = "gzip")]
        StreamingCodec::StreamingGzip => {
            let compressor = unsafe { Box::from_raw(*compressor_ptr as *mut GzipCompressor) };
            match compressor.finish() {
                Ok(buf) => Buffer::from(buf),
                Err(err) => {
                    error_to_ptr(err, error);
                    Buffer::empty()
                }
            }
        }
        #[cfg(feature = "zstd")]
        StreamingCodec::StreamingZstd => {
            let compressor = unsafe { Box::from_raw(*compressor_ptr as *mut ZstdCompressor) };
            match compressor.finish() {
                Ok(buf) => Buffer::from(buf),
                Err(err) => {
                    error_to_ptr(err, error);
                    Buffer::empty()
                }
            }
        }
        #[cfg(feature = "snappy")]
        StreamingCodec::StreamingSnappy => {
            let compressor = unsafe { Box::from_raw(*compressor_ptr as *mut SnappyFrameCompressor) };
            match compressor.into_inner() {
                Ok(buf) => Buffer::from(buf),
                Err(err) => {
                    error_to_ptr(err, error);
                    Buffer::empty()
                }
            }
        }
        #[cfg(feature = "lz4")]
        StreamingCodec::StreamingLz4 => {
            let compressor = unsafe { Box::from_raw(*compressor_ptr as *mut Lz4Compressor) };
            let (w, ret) = compressor.finish();
            match ret {
                Ok(_) => Buffer::from(w),
                Err(err) => {
                    error_to_ptr(err, error);
                    Buffer::empty()
                }
            }
        }
    };
    *compressor_ptr = std::ptr::null_mut();
    buf
}

#[no_mangle]
pub extern "C" fn compressor_flush(codec: StreamingCodec, compressor_ptr: &mut *mut c_void, error: &mut *mut c_char) {
    match codec {
        #[cfg(feature = "bzip2")]
        StreamingCodec::StreamingBzip2 => {
            let mut compressor = unsafe { Box::from_raw(*compressor_ptr as *mut Bzip2Compressor) };
            if let Err(err) = compressor.flush() {
                error_to_ptr(err, error);
            }
            *compressor_ptr = Box::into_raw(compressor) as _;
        }
        #[cfg(feature = "brotli")]
        StreamingCodec::StreamingBrotli => {
            let mut compressor = unsafe { Box::from_raw(*compressor_ptr as *mut BrotliCompressor) };
            if let Err(err) = compressor.flush() {
                error_to_ptr(err, error);
            }
            *compressor_ptr = Box::into_raw(compressor) as _;
        }
        #[cfg(feature = "gzip")]
        StreamingCodec::StreamingGzip => {
            let mut compressor = unsafe { Box::from_raw(*compressor_ptr as *mut GzipCompressor) };
            if let Err(err) = compressor.flush() {
                error_to_ptr(err, error);
            }
            *compressor_ptr = Box::into_raw(compressor) as _;
        }
        #[cfg(feature = "zstd")]
        StreamingCodec::StreamingZstd => {
            let mut compressor = unsafe { Box::from_raw(*compressor_ptr as *mut ZstdCompressor) };
            if let Err(err) = compressor.flush() {
                error_to_ptr(err, error);
            }
            *compressor_ptr = Box::into_raw(compressor) as _;
        }
        #[cfg(feature = "snappy")]
        StreamingCodec::StreamingSnappy => {
            let mut compressor = unsafe { Box::from_raw(*compressor_ptr as *mut SnappyFrameCompressor) };
            if let Err(err) = compressor.flush() {
                error_to_ptr(err, error);
            }
            *compressor_ptr = Box::into_raw(compressor) as _;
        }
        #[cfg(feature = "lz4")]
        StreamingCodec::StreamingLz4 => {
            let mut compressor = unsafe { Box::from_raw(*compressor_ptr as *mut Lz4Compressor) };
            if let Err(err) = compressor.flush() {
                error_to_ptr(err, error);
            }
            *compressor_ptr = Box::into_raw(compressor) as _;
        }
    }
}

#[no_mangle]
pub extern "C" fn compressor_compress(
    codec: StreamingCodec,
    compressor_ptr: &mut *mut c_void,
    input: *const u8,
    input_len: usize,
    nbytes_read: &mut usize,
    nbytes_written: &mut usize,
    error: &mut *mut c_char,
) {
    let mut decompressed = Cursor::new(unsafe { slice::from_raw_parts(input, input_len) });
    match codec {
        #[cfg(feature = "bzip2")]
        StreamingCodec::StreamingBzip2 => {
            let mut compressor = unsafe { Box::from_raw(*compressor_ptr as *mut Bzip2Compressor) };
            match std::io::copy(&mut decompressed, &mut compressor) {
                Ok(n) => {
                    *nbytes_written = n as _;
                    *nbytes_read = decompressed.position() as _;
                }
                Err(err) => {
                    error_to_ptr(err, error);
                }
            }
            *compressor_ptr = Box::into_raw(compressor) as _;
        }
        #[cfg(feature = "brotli")]
        StreamingCodec::StreamingBrotli => {
            let mut compressor = unsafe { Box::from_raw(*compressor_ptr as *mut BrotliCompressor) };
            match std::io::copy(&mut decompressed, &mut compressor) {
                Ok(n) => {
                    *nbytes_written = n as _;
                    *nbytes_read = decompressed.position() as _;
                }
                Err(err) => {
                    error_to_ptr(err, error);
                }
            }
            *compressor_ptr = Box::into_raw(compressor) as _;
        }
        #[cfg(feature = "gzip")]
        StreamingCodec::StreamingGzip => {
            let mut compressor = unsafe { Box::from_raw(*compressor_ptr as *mut GzipCompressor) };
            match std::io::copy(&mut decompressed, &mut compressor) {
                Ok(n) => {
                    *nbytes_written = n as _;
                    *nbytes_read = decompressed.position() as _;
                }
                Err(err) => {
                    error_to_ptr(err, error);
                }
            }
            *compressor_ptr = Box::into_raw(compressor) as _;
        }
        #[cfg(feature = "zstd")]
        StreamingCodec::StreamingZstd => {
            let mut compressor = unsafe { Box::from_raw(*compressor_ptr as *mut ZstdCompressor) };
            match std::io::copy(&mut decompressed, &mut compressor) {
                Ok(n) => {
                    *nbytes_written = n as _;
                    *nbytes_read = decompressed.position() as _;
                }
                Err(err) => {
                    error_to_ptr(err, error);
                }
            }
            *compressor_ptr = Box::into_raw(compressor) as _;
        }
        #[cfg(feature = "snappy")]
        StreamingCodec::StreamingSnappy => {
            let mut compressor = unsafe { Box::from_raw(*compressor_ptr as *mut SnappyFrameCompressor) };
            match std::io::copy(&mut decompressed, &mut compressor) {
                Ok(n) => {
                    *nbytes_written = n as _;
                    *nbytes_read = decompressed.position() as _;
                }
                Err(err) => {
                    error_to_ptr(err, error);
                }
            }
            *compressor_ptr = Box::into_raw(compressor) as _;
        }
        #[cfg(feature = "lz4")]
        StreamingCodec::StreamingLz4 => {
            let mut compressor = unsafe { Box::from_raw(*compressor_ptr as *mut Lz4Compressor) };
            match std::io::copy(&mut decompressed, &mut compressor) {
                Ok(n) => {
                    *nbytes_written = n as _;
                    *nbytes_read = decompressed.position() as _;
                }
                Err(err) => {
                    error_to_ptr(err, error);
                }
            }
            *compressor_ptr = Box::into_raw(compressor) as _;
        }
    }
}
#[no_mangle]
#[allow(unused_variables)]
pub extern "C" fn decompressor_init(codec: StreamingCodec) -> *mut c_void {
    // for decompression, we really only need a buffer for storing output
    // some streaming codecs, like snappy, don't have a write impl and only a
    // read impl for decompressors
    let buf: Vec<u8> = vec![];
    Box::into_raw(Box::new(Cursor::new(buf))) as _
}

#[no_mangle]
#[allow(unused_variables)]
pub extern "C" fn free_decompressor(codec: StreamingCodec, decompressor_ptr: &mut *mut c_void) {
    if !(*decompressor_ptr).is_null() {
        {
            let _ = unsafe { Box::from_raw(*decompressor_ptr as *mut Decompressor) };
        }
        *decompressor_ptr = std::ptr::null_mut();
    }
}

#[no_mangle]
#[allow(unused_variables)]
pub extern "C" fn decompressor_inner(codec: StreamingCodec, decompressor_ptr: &mut *mut c_void) -> Buffer {
    let decompressor = unsafe { Box::from_raw(*decompressor_ptr as *mut Decompressor) };
    let buf = Buffer::from(decompressor.get_ref());
    *decompressor_ptr = Box::into_raw(decompressor) as _;
    buf
}

/// Finish the decompression stream and return the underlying buffer, transfering ownership to caller
#[no_mangle]
#[allow(unused_variables)]
pub extern "C" fn decompressor_finish(
    codec: StreamingCodec,
    decompressor_ptr: &mut *mut c_void,
    error: &mut *mut c_char,
) -> Buffer {
    let mut cursor = unsafe { Box::from_raw(*decompressor_ptr as *mut Decompressor) };
    if let Err(err) = cursor.flush() {
        error_to_ptr(err, error);
        return Buffer::empty();
    };
    *decompressor_ptr = std::ptr::null_mut();
    Buffer::from(cursor.into_inner())
}

#[no_mangle]
#[allow(unused_variables)]
pub extern "C" fn decompressor_flush(
    codec: StreamingCodec,
    decompressor_ptr: &mut *mut c_void,
    error: &mut *mut c_char,
) {
    let mut cursor = unsafe { Box::from_raw(*decompressor_ptr as *mut Decompressor) };
    if let Err(err) = cursor.flush() {
        error_to_ptr(err, error);
    }
    *decompressor_ptr = Box::into_raw(cursor) as _;
}

#[no_mangle]
pub extern "C" fn decompressor_decompress(
    codec: StreamingCodec,
    decompressor_ptr: &mut *mut c_void,
    input: *const u8,
    input_len: usize,
    nbytes_read: &mut usize,
    nbytes_written: &mut usize,
    error: &mut *mut c_char,
) {
    let mut decompressed = unsafe { Box::from_raw(*decompressor_ptr as *mut Decompressor) };
    let start_pos = decompressed.position();
    let mut compressed = Cursor::new(unsafe { std::slice::from_raw_parts(input, input_len) });
    let ret: Result<usize, std::io::Error> = match codec {
        #[cfg(feature = "bzip2")]
        StreamingCodec::StreamingBzip2 => bzip2::decompress(&mut compressed, &mut decompressed),
        #[cfg(feature = "gzip")]
        StreamingCodec::StreamingGzip => gzip::decompress(&mut compressed, &mut decompressed),
        #[cfg(feature = "brotli")]
        StreamingCodec::StreamingBrotli => brotli::decompress(&mut compressed, &mut decompressed),
        #[cfg(feature = "zstd")]
        StreamingCodec::StreamingZstd => zstd::decompress(&mut compressed, &mut decompressed),
        #[cfg(feature = "snappy")]
        StreamingCodec::StreamingSnappy => snappy::decompress(&mut compressed, &mut decompressed),
        #[cfg(feature = "lz4")]
        StreamingCodec::StreamingLz4 => lz4::decompress(&mut compressed, &mut decompressed),
    };
    match ret {
        Ok(_) => {
            *nbytes_read = compressed.position() as _;
            *nbytes_written = (decompressed.position() - start_pos) as _;
        }
        Err(err) => {
            error_to_ptr(err, error);
        }
    };
    *decompressor_ptr = Box::into_raw(decompressed) as _;
}

/* -------- Codec specific functions ----------*/
#[cfg(feature = "lz4")]
#[no_mangle]
pub extern "C" fn lz4_frame_max_compression_level() -> usize {
    lz4::LZ4_ACCELERATION_MAX as _
}

#[cfg(feature = "lz4")]
#[no_mangle]
pub extern "C" fn lz4_frame_max_compressed_len(input_len: usize, compression_level: i32) -> usize {
    lz4::compress_bound(input_len, Some(compression_level as _))
}

#[cfg(feature = "lz4")]
#[no_mangle]
#[allow(unused_variables)]
pub extern "C" fn lz4_block_max_compressed_len(input_len: usize, error: &mut *mut c_char) -> usize {
    lz4::block::compress_bound(input_len, Some(true))
}

#[cfg(feature = "deflate")]
#[no_mangle]
pub extern "C" fn deflate_max_compressed_len(input_len: usize, level: i32) -> usize {
    deflate::compress_bound(input_len, Some(level))
}

#[cfg(feature = "gzip")]
#[no_mangle]
pub extern "C" fn gzip_max_compressed_len(input_len: usize, level: i32) -> usize {
    let level = if level < 0 { 0 } else { level };
    gzip::compress_bound(input_len, Some(level)).unwrap()
}

#[cfg(feature = "zstd")]
#[no_mangle]
pub extern "C" fn zstd_max_compressed_len(input_len: usize) -> usize {
    zstd::compress_bound(input_len)
}

#[cfg(feature = "snappy")]
#[no_mangle]
pub extern "C" fn snappy_raw_max_compressed_len(input_len: usize) -> usize {
    snap::raw::max_compress_len(input_len)
}

#[cfg(feature = "brotli")]
#[no_mangle]
pub extern "C" fn brotli_max_compressed_len(input_len: usize) -> usize {
    brotli::compress_bound(input_len)
}

#[cfg(feature = "snappy")]
#[no_mangle]
pub extern "C" fn snappy_raw_decompressed_len(input: *const u8, input_len: usize, error: &mut *mut c_char) -> isize {
    let input = unsafe { slice::from_raw_parts(input, input_len) };
    match snap::raw::decompress_len(input) {
        Ok(n) => n as _,
        Err(err) => {
            error_to_ptr(err, error);
            -1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DATA: &[u8; 5] = b"bytes";

    #[cfg(feature = "lz4")]
    #[test]
    fn test_lz4_frame_max_compressed_len() {
        // A known simple test case, expected len taken from lz4/lz4 repo
        let len = lz4_frame_max_compressed_len(25, 4);
        assert_eq!(len, 65544);
    }

    #[cfg(feature = "lz4")]
    #[test]
    fn test_lz4_block_max_compressed_len() {
        let mut error: *mut c_char = std::ptr::null_mut();
        let len = lz4_block_max_compressed_len(10, &mut error);
        assert!(error.is_null());
        assert_eq!(len, 30);
    }

    #[cfg(feature = "snappy")]
    #[test]
    fn test_snappy_raw_max_compressed_len() {
        let len = snappy_raw_max_compressed_len(10);
        assert_eq!(len, 43);
    }

    #[cfg(feature = "snappy")]
    #[test]
    fn test_snappy_raw_decompressed_len() {
        let uncompressed = b"bytes";
        let mut compressed = vec![0; snappy_raw_max_compressed_len(uncompressed.len())];
        let nbytes_written = snappy::raw::compress(uncompressed, &mut compressed).unwrap();

        let mut error: *mut c_char = std::ptr::null_mut();
        let len = snappy_raw_decompressed_len(compressed.as_ptr(), nbytes_written, &mut error);

        assert!(error.is_null());
        assert_eq!(len as usize, uncompressed.len());
    }

    #[cfg(feature = "snappy")]
    #[test]
    fn test_snappy_roundtrip() {
        let mut expected = vec![];
        snappy::compress(Cursor::new(DATA), &mut expected).unwrap();
        roundtrip(Codec::Snappy, &expected, 0);
    }
    #[cfg(feature = "snappy")]
    #[test]
    fn test_snappy_raw_roundtrip() {
        let expected = snappy::raw::compress_vec(DATA).unwrap();
        roundtrip(Codec::SnappyRaw, &expected, 0);
    }
    #[cfg(feature = "lz4")]
    #[test]
    fn test_lz4_roundtrip() {
        let mut expected = Cursor::new(vec![]);
        lz4::compress(Cursor::new(DATA), &mut expected, Some(6)).unwrap();
        let expected = expected.into_inner();
        roundtrip(Codec::Lz4, &expected, 6);
    }
    #[cfg(feature = "lz4")]
    #[test]
    fn test_lz4_block_roundtrip() {
        let expected = lz4::block::compress_vec(DATA, Some(6), Some(1), Some(true)).unwrap();
        roundtrip(Codec::Lz4Block, &expected, 6);
    }
    #[cfg(feature = "bzip2")]
    #[test]
    fn test_bzip2_roundtrip() {
        let mut expected = Cursor::new(vec![]);
        bzip2::compress(Cursor::new(DATA), &mut expected, Some(6)).unwrap();
        let expected = expected.into_inner();
        roundtrip(Codec::Bzip2, &expected, 6);
    }
    #[cfg(feature = "brotli")]
    #[test]
    fn test_brotli_roundtrip() {
        let mut expected = Cursor::new(vec![]);
        brotli::compress(Cursor::new(DATA), &mut expected, Some(6)).unwrap();
        let expected = expected.into_inner();
        roundtrip(Codec::Brotli, &expected, 6);
    }
    #[cfg(feature = "zstd")]
    #[test]
    fn test_zstd_roundtrip() {
        let mut expected = Cursor::new(vec![]);
        zstd::compress(Cursor::new(DATA), &mut expected, Some(6)).unwrap();
        let expected = expected.into_inner();
        roundtrip(Codec::Zstd, &expected, 6);
    }

    fn roundtrip(codec: Codec, expected: &[u8], level: i32) {
        let mut nbytes_read = 0;
        let mut nbytes_written = 0;
        let mut error = std::ptr::null_mut();
        let buffer = compress(
            codec,
            level,
            DATA.as_ptr(),
            DATA.len(),
            &mut nbytes_read,
            &mut nbytes_written,
            &mut error,
        );
        if !error.is_null() {
            let error = unsafe { CString::from_raw(error) };
            panic!("Failed: {}", error.to_str().unwrap());
        }
        assert_eq!(nbytes_read, DATA.len());
        assert_eq!(nbytes_written, buffer.len);
        assert!(buffer.owned);

        // retrieve compressed data and compare to actual rust impl
        let compressed = unsafe { Vec::from_raw_parts(buffer.data as *mut u8, buffer.len, buffer.len) };
        assert_eq!(&compressed, expected);

        // And decompress
        nbytes_read = 0;
        nbytes_written = 0;

        let buffer = decompress(
            codec,
            compressed.as_ptr(),
            compressed.len(),
            &mut nbytes_read,
            &mut nbytes_written,
            &mut error,
        );
        if !error.is_null() {
            let error = unsafe { CString::from_raw(error) };
            panic!("Failed: {}", error.to_str().unwrap());
        }
        assert_eq!(nbytes_read, compressed.len());
        assert_eq!(nbytes_written, buffer.len);
        assert_eq!(nbytes_written, DATA.len());
        assert!(buffer.owned);
        let decompressed = unsafe { Vec::from_raw_parts(buffer.data as *mut u8, buffer.len, buffer.len) };
        assert_eq!(DATA.as_slice(), &decompressed);
    }
}
