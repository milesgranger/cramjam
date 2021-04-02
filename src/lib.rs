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
//!  - [`cramjam.File`](cramjam_core/io/struct.RustyFile.html)
//!  - [`cramjam.Buffer`](cramjam_core/io/struct.RustyBuffer.html)
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
pub mod deflate;
pub mod gzip;
pub mod lz4;
pub mod snappy;
pub mod zstd;

use pyo3::prelude::*;

pub use cramjam_core::exceptions::{CompressionError, DecompressionError};
pub use cramjam_core::{
    to_py_err, AsBytes, BytesType, RustyBuffer, RustyFile, RustyNumpyArray, RustyPyByteArray, RustyPyBytes,
};

#[cfg(feature = "mimallocator")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// Macro for generating the implementation of de/compression against a variant interface
#[macro_export]
macro_rules! generic {
    ($op:ident($input:expr), output_len=$output_len:ident $(, level=$level:ident)?) => {
        {
            use cramjam_core::io::RustyBuffer;

            let mut output: Vec<u8> = match $output_len {
                Some(len) => vec![0; len],
                None => vec![]
            };
            if stringify!($op) == "compress" {
                to_py_err!(CompressionError -> self::internal::$op($input, &mut Cursor::new(&mut output) $(, $level)? ))?;
            } else {
                to_py_err!(DecompressionError -> self::internal::$op($input, &mut Cursor::new(&mut output) $(, $level)? ))?;
            }
            Ok(RustyBuffer::from(output))
        }
    }
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
    m.add_class::<cramjam_core::io::RustyFile>()?;
    m.add_class::<cramjam_core::io::RustyBuffer>()?;
    make_submodule!(py -> m -> snappy);
    make_submodule!(py -> m -> brotli);
    make_submodule!(py -> m -> lz4);
    make_submodule!(py -> m -> gzip);
    make_submodule!(py -> m -> deflate);
    make_submodule!(py -> m -> zstd);

    if let Ok(lzo) = PyModule::import(py, "cramjam_lzo") {
        m.add("lzo", lzo)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {

    use std::io::Cursor;

    // Default testing data
    fn gen_data() -> Vec<u8> {
        (0..1000000)
            .map(|_| b"oh what a beautiful morning, oh what a beautiful day!!".to_vec())
            .flat_map(|v| v)
            .collect()
    }

    // Single test generation
    macro_rules! round_trip {
        ($name:ident($compress_output:ident -> $decompress_output:ident), variant=$variant:ident, compressed_len=$compressed_len:literal, $(level=$level:tt)?) => {
            #[test]
            fn $name() {
                let data = gen_data();

                let mut compressed = Vec::new();

                let compressed_size = if stringify!($decompress_output) == "Slice" {
                        compressed = (0..data.len()).map(|_| 0).collect::<Vec<u8>>();
                        let mut cursor = Cursor::new(compressed.as_mut_slice());
                        crate::$variant::internal::compress(&mut Cursor::new(data.as_slice()), &mut cursor $(, $level)?).unwrap()
                    } else {
                        crate::$variant::internal::compress(&mut Cursor::new(data.as_slice()), &mut Cursor::new(&mut compressed) $(, $level)?).unwrap()
                    };

                assert_eq!(compressed_size, $compressed_len);
                compressed.truncate(compressed_size);

                let mut decompressed = Vec::new();

                let decompressed_size = if stringify!($decompress_output) == "Slice" {
                        decompressed = (0..data.len()).map(|_| 0).collect::<Vec<u8>>();
                        let mut cursor = Cursor::new(decompressed.as_mut_slice());
                        crate::$variant::internal::decompress(&mut Cursor::new(&compressed), &mut cursor).unwrap()
                    } else {
                        crate::$variant::internal::decompress(&mut Cursor::new(&compressed), &mut decompressed).unwrap()
                    };
                assert_eq!(decompressed_size, data.len());
                if &decompressed[..decompressed_size] != &data {
                    panic!("Decompressed and original data do not match! :-(")
                }
            }
        }
    }

    // macro to generate each variation of Output::* roundtrip.
    macro_rules! test_variant {
        ($variant:ident, compressed_len=$compressed_len:literal, $(level=$level:tt)?) => {
         #[cfg(test)]
         mod $variant {
            use super::*;
            round_trip!(roundtrip_compress_via_slice_decompress_via_slice(Slice -> Slice), variant=$variant, compressed_len=$compressed_len, $(level=$level)? );
            round_trip!(roundtrip_compress_via_slice_decompress_via_vector(Slice -> Vector), variant=$variant, compressed_len=$compressed_len, $(level=$level)? );
            round_trip!(roundtrip_compress_via_vector_decompress_via_slice(Vector -> Slice), variant=$variant, compressed_len=$compressed_len, $(level=$level)? );
            round_trip!(roundtrip_compress_via_vector_decompress_via_vector(Vector -> Vector), variant=$variant, compressed_len=$compressed_len, $(level=$level)? );
         }
        }
    }

    test_variant!(snappy, compressed_len = 2572398,);
    test_variant!(gzip, compressed_len = 157192, level = None);
    test_variant!(brotli, compressed_len = 729, level = None);
    test_variant!(deflate, compressed_len = 157174, level = None);
    test_variant!(zstd, compressed_len = 4990, level = None);
    test_variant!(lz4, compressed_len = 303278, level = None);
}
