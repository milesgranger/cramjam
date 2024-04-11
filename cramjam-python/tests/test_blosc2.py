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
    blosc2 = experimental.blosc2


settings.register_profile("local", max_examples=10)
settings.register_profile("CI", max_examples=5)

if os.getenv("CI"):
    settings.load_profile("CI")
else:
    settings.load_profile("local")


def variants(e):
    for attr in dir(e):
        # TODO: LastCodec, LastFilter, LastRegisteredCodec/Filter not supported
        if not attr.startswith('_') and not attr.lower().startswith('last'):
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

    compressed = np.empty(blosc2.max_compressed_len(len(data.tobytes())), dtype=np.uint8)
    nbytes = blosc2.compress_chunk_into(data, compressed, **kwargs)

    decompressed = np.empty(len(data.tobytes()) * 2, dtype=np.uint8)
    nbytes = blosc2.decompress_chunk_into(compressed[:nbytes], decompressed)
    assert nbytes == len(data.tobytes())
    np.array_equal(data, np.frombuffer(decompressed[:nbytes], dtype=data.dtype))
