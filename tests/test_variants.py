import pytest

import cramjam


@pytest.mark.parametrize("variant", ("snappy", "brotli", "lz4"))
def test_variants_simple(variant):

    compress = getattr(cramjam, f"{variant}_compress")
    decompress = getattr(cramjam, f"{variant}_decompress")

    uncompressed = b"some bytes to compress 123" * 1000

    compressed = compress(uncompressed)
    assert compressed != uncompressed

    decompressed = decompress(compressed)
    assert decompressed == uncompressed
