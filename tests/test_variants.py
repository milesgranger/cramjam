import pytest
import numpy as np
import cramjam
import hashlib

def same_same(a, b):
    return hashlib.md5(a).hexdigest() == hashlib.md5(b).hexdigest()

@pytest.mark.parametrize(
    "variant_str", ("snappy", "brotli", "lz4", "gzip", "deflate", "zstd")
)
def test_variants_simple(variant_str):

    variant = getattr(cramjam, variant_str)

    uncompressed = b"some bytes to compress 123" * 1000

    compressed = variant.compress(uncompressed)
    assert compressed != uncompressed

    decompressed = variant.decompress(compressed, output_len=len(uncompressed))
    assert decompressed == uncompressed


@pytest.mark.parametrize(
    "variant_str", ("snappy", "brotli", "lz4", "gzip", "deflate", "zstd")
)
def test_variants_raise_exception(variant_str):
    variant = getattr(cramjam, variant_str)
    with pytest.raises(cramjam.DecompressionError):
        variant.decompress(b'sknow')


@pytest.mark.parametrize(
    "variant_str", ("snappy", "brotli", "gzip", "deflate", "zstd")
)
def test_variants_de_compress_into(variant_str):

    # TODO: support lz4 de/compress_into

    variant = getattr(cramjam, variant_str)

    data = b"oh what a beautiful morning, oh what a beautiful day!!" * 1000000

    compressed_array = np.zeros(len(data), dtype=np.uint8)  # plenty of space
    compressed_size = variant.compress_into(data, compressed_array)
    decompressed = variant.decompress(compressed_array[:compressed_size].tobytes())
    assert same_same(decompressed, data)

    compressed = variant.compress(data)
    decompressed_array = np.zeros(len(data), np.uint8)
    decompressed_size = variant.decompress_into(compressed, decompressed_array)
    decompressed = decompressed_array[:decompressed_size].tobytes()
    assert same_same(decompressed, data)
