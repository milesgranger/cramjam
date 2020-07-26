import io
import pytest

import cramjam
from cramjam import DecompressionError, CompressionError


@pytest.mark.parametrize("variant", ("snappy",))
@pytest.mark.parametrize("chunk_size", (-1, 1, 5, 10, 100))
def test_variants_stream(variant: str, chunk_size: int):
    data = io.BytesIO(b"some bytes to compress 123" * 1000)
    compressed = io.BytesIO()
    compress = getattr(cramjam, f"{variant}_compress_stream")
    compress(data, compressed, chunk_size=chunk_size)
    compressed.seek(0)
    print(compressed.read())


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
        getattr(cramjam, f"{variant}_decompress")(b"sknow")
