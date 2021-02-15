import pytest

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
