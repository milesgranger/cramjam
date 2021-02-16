import pytest
import numpy as np
import cramjam


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

    uncompressed = b"some bytes to compress 123 " * 10
    uncompressed_len = len(uncompressed)

    # Get output len of compressed
    compressed = variant.compress(uncompressed)
    compressed_len = len(compressed)

    compress_into_buffer = np.zeros(compressed_len, dtype=np.uint8)
    size = variant.compress_into(uncompressed, compress_into_buffer)
    if size != compressed_len:
        breakpoint()
