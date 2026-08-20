import os
import pytest
import numpy as np
from hypothesis import strategies as st, given, settings
from hypothesis.extra import numpy as st_np

try:
    from cramjam import experimental
except ImportError:
    pytest.skip("experimental module not built", allow_module_level=True)
else:
    if hasattr(experimental, "blosc2"):
        blosc2 = experimental.blosc2
    else:
        pytest.skip(
            "experimental module doesn't contain blosc2", allow_module_level=True
        )


settings.register_profile("local", max_examples=10, deadline=None)
settings.register_profile("CI", max_examples=5, deadline=None)

if os.getenv("CI"):
    settings.load_profile("CI")
else:
    settings.load_profile("local")


def variants(e):
    for attr in dir(e):
        # TODO: LastCodec, LastFilter, LastRegisteredCodec/Filter not supported
        if not attr.startswith("_") and not attr.lower().startswith("last"):
            yield getattr(e, attr)


@pytest.mark.parametrize("codec", variants(blosc2.Codec), ids=lambda v: str(v))
@pytest.mark.parametrize("filter", variants(blosc2.Filter), ids=lambda v: str(v))
@pytest.mark.parametrize("clevel", variants(blosc2.CLevel), ids=lambda v: str(v))
@given(data=st_np.arrays(st_np.scalar_dtypes(), shape=st.integers(0, 10_000)))
def test_roundtrip_chunk(data, codec, filter, clevel):
    compressed = blosc2.compress_chunk(data, clevel=clevel, filter=filter, codec=codec)
    decompressed = blosc2.decompress_chunk(compressed)
    assert data.tobytes() == bytes(decompressed)


@pytest.mark.parametrize("codec", variants(blosc2.Codec), ids=lambda v: str(v))
@pytest.mark.parametrize("filter", variants(blosc2.Filter), ids=lambda v: str(v))
@pytest.mark.parametrize("clevel", variants(blosc2.CLevel), ids=lambda v: str(v))
@given(data=st_np.arrays(st_np.scalar_dtypes(), shape=st.integers(0, 10_000)))
def test_roundtrip_chunk_into(data, codec, filter, clevel):
    kwargs = dict(clevel=clevel, filter=filter, codec=codec)
    nbytes_compressed = len(blosc2.compress_chunk(data, **kwargs))

    compressed = np.empty(
        blosc2.max_compressed_len(len(data.tobytes())), dtype=np.uint8
    )
    nbytes = blosc2.compress_chunk_into(data, compressed, **kwargs)

    decompressed = np.empty(len(data.tobytes()) * 2, dtype=np.uint8)
    nbytes = blosc2.decompress_chunk_into(compressed[:nbytes], decompressed)
    assert nbytes == len(data.tobytes())
    np.array_equal(data, np.frombuffer(decompressed[:nbytes], dtype=data.dtype))


def test_schunk_callbacks():
    converted = []

    def to_bytes(value):
        converted.append(("to", value))
        return value.payload

    def from_bytes(value):
        converted.append(("from", bytes(value)))
        return bytes(value)

    class Wrapped:
        payload = b"callback data"

    schunk = blosc2.SChunk(to_bytes_cb=to_bytes, from_bytes_cb=from_bytes)
    assert schunk.append_buffer(Wrapped()) == 1
    assert schunk.decompress_chunk(0) == b"callback data"
    assert converted[0][0] == "to"
    assert converted[1] == ("from", b"callback data")


def test_schunk_from_compressor():
    compressor = blosc2.Compressor()
    compressor.compress(b"data")

    schunk = blosc2.SChunk.from_compressor(compressor)
    assert schunk.nchunks == 1
    assert bytes(schunk.decompress_chunk(0)) == b"data"
