import pytest

import cramjam
from cramjam import DecompressionError, CompressionError


@pytest.mark.parametrize(
    "variant", ("snappy", "brotli", "lz4", "gzip", "deflate", "zstd")
)
def test_variants_simple(variant):

    compress = getattr(cramjam, f"{variant}_compress")
    decompress = getattr(cramjam, f"{variant}_decompress")

    uncompressed = b"some bytes to compress 123" * 1000

    compressed = compress(uncompressed)
    assert compressed != uncompressed

    decompressed = decompress(compressed)
    assert decompressed == uncompressed


@pytest.mark.parametrize(
    "variant", ("snappy", "brotli", "lz4", "gzip", "deflate", "zstd")
)
def test_variants_raise_exception(variant):
    with pytest.raises(cramjam.DecompressionError):
        getattr(cramjam, f"{variant}_decompress")(b'sknow')
