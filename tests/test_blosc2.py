import os
import subprocess
import sys
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


@pytest.mark.parametrize(
    "first_operation",
    [
        "compress",
        "compress_into",
        "compressor",
        "schunk",
        "compress_chunk",
        "compress_chunk_into",
    ],
)
def test_first_use_in_fresh_process(first_operation):
    """Blosc2 must work before any other Blosc2 call initializes global state."""
    script = r"""
import cramjam
blosc2 = cramjam.experimental.blosc2
payload = b"first Blosc2 operation" * 8
operation = __import__("sys").argv[1]
if operation == "compress":
    compressed = blosc2.compress(payload)
    restored = blosc2.decompress(compressed)
elif operation == "compress_into":
    compressed = cramjam.Buffer()
    size = blosc2.compress_into(payload, compressed)
    assert size == len(compressed)
    compressed.seek(0)
    restored = blosc2.decompress(compressed)
elif operation == "compressor":
    compressor = blosc2.Compressor()
    compressor.compress(payload)
    restored = blosc2.decompress(compressor.finish())
elif operation == "schunk":
    schunk = blosc2.SChunk()
    assert schunk.append_buffer(payload) == 1
    restored = schunk.decompress_chunk(0)
elif operation == "compress_chunk":
    restored = blosc2.decompress_chunk(blosc2.compress_chunk(payload))
else:
    compressed = bytearray(blosc2.max_compressed_len(len(payload)))
    size = blosc2.compress_chunk_into(payload, compressed)
    restored = blosc2.decompress_chunk(compressed[:size])
assert bytes(restored) == payload
print("roundtrip passed")
"""
    result = subprocess.run(
        [sys.executable, "-c", script, first_operation],
        capture_output=True,
        text=True,
        timeout=30,
    )
    assert result.returncode == 0, result.stdout + result.stderr
    assert result.stdout.strip() == "roundtrip passed"


@pytest.mark.parametrize("append", [False, True])
@pytest.mark.parametrize("data", [b"x", b"frame payload" * 1000])
def test_compress_into_file_at_position(tmp_path, append, data):
    import cramjam

    prefix = b"prefix:"
    path = tmp_path / "frame.b2"
    path.write_bytes(prefix)
    output = cramjam.File(str(path), append=append)
    # Non-append output follows the cursor. Append output writes at EOF even
    # when its cursor starts at zero, as other File writes do.
    output.seek(0 if append else len(prefix))
    nbytes = blosc2.compress_into(data, output, nthreads=1)
    assert output.tell() == len(prefix) + nbytes
    stored = path.read_bytes()
    assert stored[: len(prefix)] == prefix
    assert len(stored) == len(prefix) + nbytes
    assert bytes(blosc2.decompress(stored[len(prefix) :])) == data


def test_compress_into_file_preserves_trailing_bytes(tmp_path):
    import cramjam

    data = b"payload" * 100
    expected_frame = bytes(blosc2.compress(data, nthreads=1))
    prefix, suffix = b"prefix:", b":suffix"
    path = tmp_path / "frame.b2"
    path.write_bytes(prefix + b"\x00" * len(expected_frame) + suffix)
    output = cramjam.File(str(path))
    output.seek(len(prefix))
    nbytes = blosc2.compress_into(data, output, nthreads=1)
    assert nbytes == len(expected_frame)
    assert output.tell() == len(prefix) + nbytes
    stored = path.read_bytes()
    assert stored[: len(prefix)] == prefix
    assert stored[len(prefix) + nbytes :] == suffix
    assert bytes(blosc2.decompress(stored[len(prefix) : len(prefix) + nbytes])) == data


def test_compress_into_file_empty_input_is_noop(tmp_path):
    import cramjam

    path = tmp_path / "frame.b2"
    path.write_bytes(b"unchanged")
    output = cramjam.File(str(path))
    output.seek(3)
    assert blosc2.compress_into(b"", output) == 0
    assert output.tell() == 3
    assert path.read_bytes() == b"unchanged"
