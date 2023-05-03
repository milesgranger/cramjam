pub mod brotli;
pub mod bzip2;
pub mod deflate;
pub mod gzip;
pub mod lz4;
pub mod snappy;
pub mod zstd;

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
                        crate::$variant::compress(&mut Cursor::new(data.as_slice()), &mut cursor $(, $level)?).unwrap()
                    } else {
                        crate::$variant::compress(&mut Cursor::new(data.as_slice()), &mut Cursor::new(&mut compressed) $(, $level)?).unwrap()
                    };

                assert_eq!(compressed_size, $compressed_len);
                compressed.truncate(compressed_size);

                let mut decompressed = Vec::new();

                let decompressed_size = if stringify!($decompress_output) == "Slice" {
                        decompressed = (0..data.len()).map(|_| 0).collect::<Vec<u8>>();
                        let mut cursor = Cursor::new(decompressed.as_mut_slice());
                        crate::$variant::decompress(&mut Cursor::new(&compressed), &mut cursor).unwrap()
                    } else {
                        crate::$variant::decompress(&mut Cursor::new(&compressed), &mut decompressed).unwrap()
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
    test_variant!(brotli, compressed_len = 128, level = None);
    test_variant!(bzip2, compressed_len = 14207, level = None);
    test_variant!(deflate, compressed_len = 157174, level = None);
    test_variant!(zstd, compressed_len = 4990, level = None);
    test_variant!(lz4, compressed_len = 303278, level = None);
}
