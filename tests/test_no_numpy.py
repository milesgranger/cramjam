import pytest
import cramjam


@pytest.mark.parametrize("obj", (bytes, bytearray, cramjam.Buffer, cramjam.File))
@pytest.mark.parametrize(
    "variant_str", ("snappy", "brotli", "lz4", "gzip", "deflate", "zstd")
)
def test_no_numpy_installed(tmpdir, obj, variant_str):
    """
    These operations should work even when numpy is not installed
    """
    if cramjam.File == obj:
        data = obj(str(tmpdir.join("tmp.txt")))
        data.write(b"data")
        data.seek(0)
    else:
        data = obj(b"data")

    variant = getattr(cramjam, variant_str)
    compressed = variant.compress(data)
    decompressed = variant.decompress(compressed)
    assert decompressed.read() == b"data"
