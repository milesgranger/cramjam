import pytest

import cramjam


@pytest.mark.parametrize("variant", ("snappy",))
def test_variants_simple(variant):

    compress = getattr(cramjam, f"{variant}_compress")
    decompress = getattr(cramjam, f"{variant}_decompress")

    uncompressed = b"some bytes to compress"

    compressed = compress(uncompressed)
    assert compressed != uncompressed

    decompressed = decompress(compressed)
    assert decompressed == uncompressed
