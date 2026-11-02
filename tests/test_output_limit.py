"""Hard limits must fail before growing output, never return truncated data."""

import struct
import sys

import cramjam
import pytest
from hypothesis import given, settings, strategies as st

CODECS = ("snappy", "brotli", "bzip2", "lz4", "gzip", "deflate", "zstd", "zlib", "xz")
PURE_RUST = getattr(cramjam, "backend", "pure-rust") == "pure-rust"


def supported(name):
    if PURE_RUST and name not in ("snappy", "brotli"):
        pytest.skip("bounded output requires the C backend for this codec")
    return getattr(cramjam, name)


@pytest.mark.parametrize("name", CODECS)
@pytest.mark.parametrize("data", [b"", b"x", b"abc" * 22000])
def test_limit_boundaries(name, data):
    codec = supported(name)
    compressed = codec.compress(data)
    assert bytes(codec.decompress(compressed, max_output_size=len(data))) == data
    assert bytes(codec.decompress(compressed, max_output_size=len(data) + 1)) == data
    if data:
        with pytest.raises(cramjam.OutputLimitExceeded, match="max_output_size"):
            codec.decompress(compressed, max_output_size=len(data) - 1)


@pytest.mark.parametrize("name", CODECS)
def test_limit_cannot_be_bypassed_by_output_len(name):
    codec = supported(name)
    compressed = codec.compress(b"x" * 65536)
    # output_len alone retains its historical, non-limiting behavior.
    assert len(codec.decompress(compressed, output_len=16)) == 65536
    for hint in (None, 0, 16, sys.maxsize):
        with pytest.raises(cramjam.OutputLimitExceeded):
            codec.decompress(compressed, output_len=hint, max_output_size=16)


@pytest.mark.parametrize("name", CODECS)
def test_hint_is_clamped_and_does_not_pad_bounded_output(name):
    codec = supported(name)
    compressed = codec.compress(b"abc")
    assert (
        bytes(codec.decompress(compressed, output_len=sys.maxsize, max_output_size=16))
        == b"abc"
    )
    # A huge cap itself must not cause allocation of a huge buffer.
    assert bytes(codec.decompress(compressed, max_output_size=sys.maxsize)) == b"abc"


@pytest.mark.parametrize("name", CODECS)
@pytest.mark.parametrize("hint", [None, 0, 16, 100])
def test_omitted_limit_retains_existing_behavior(name, hint):
    codec = getattr(cramjam, name)
    compressed = bytes(codec.compress(b"abc" * 10))
    assert bytes(codec.decompress(compressed, hint)) == bytes(
        codec.decompress(compressed, hint, max_output_size=None)
    )


@pytest.mark.parametrize("name", ("zstd", "lz4", "gzip", "snappy", "bzip2"))
def test_limit_is_cumulative_across_frames(name):
    codec = supported(name)
    parts = [b"a" * 8192, b"b" * 8192, b"c" * 8192]
    compressed = b"".join(bytes(codec.compress(part)) for part in parts)
    assert bytes(codec.decompress(compressed, max_output_size=24576)) == b"".join(parts)
    with pytest.raises(cramjam.OutputLimitExceeded):
        codec.decompress(compressed, max_output_size=10000)


@pytest.mark.parametrize("name", CODECS)
@pytest.mark.parametrize(
    "input_type", [bytes, bytearray, memoryview, cramjam.Buffer, "file"]
)
def test_buffer_and_file_inputs(name, input_type, tmp_path):
    codec = supported(name)
    compressed = bytes(codec.compress(b"payload" * 1000))
    if input_type == "file":
        path = tmp_path / "compressed"
        path.write_bytes(compressed)
        source = cramjam.File(str(path))
    else:
        source = input_type(compressed)
    with pytest.raises(cramjam.OutputLimitExceeded):
        codec.decompress(source, max_output_size=100)


@pytest.mark.parametrize("name", CODECS)
@pytest.mark.parametrize(
    "limit,exception", [(-1, OverflowError), (1.5, TypeError), ("16", TypeError)]
)
def test_invalid_limit(name, limit, exception):
    with pytest.raises(exception):
        getattr(cramjam, name).decompress(b"", max_output_size=limit)


@pytest.mark.parametrize("name", CODECS)
def test_corruption_is_not_reported_as_an_output_limit(name):
    codec = supported(name)
    with pytest.raises(cramjam.DecompressionError) as error:
        codec.decompress(b"\xff" * 16, max_output_size=1000)
    assert not isinstance(error.value, cramjam.OutputLimitExceeded)


@pytest.mark.parametrize("name", CODECS)
@settings(deadline=None, max_examples=15)
@given(data=st.binary(max_size=4096), limit=st.integers(min_value=0, max_value=4096))
def test_result_matches_limit(name, data, limit):
    codec = supported(name)
    compressed = codec.compress(data)
    if len(data) > limit:
        with pytest.raises(cramjam.OutputLimitExceeded):
            codec.decompress(compressed, max_output_size=limit)
    else:
        assert bytes(codec.decompress(compressed, max_output_size=limit)) == data


@pytest.mark.parametrize("data", [b"", b"raw" * 30000])
def test_raw_snappy(data):
    compressed = cramjam.snappy.compress_raw(data)
    assert (
        bytes(cramjam.snappy.decompress_raw(compressed, max_output_size=len(data)))
        == data
    )
    if data:
        with pytest.raises(cramjam.OutputLimitExceeded):
            cramjam.snappy.decompress_raw(compressed, max_output_size=len(data) - 1)


def test_raw_snappy_rejects_declared_length_before_allocation():
    # 64 MiB decoded length in a valid raw Snappy varint, with no body.
    header = b"\x80\x80\x80\x20"
    assert cramjam.snappy.decompress_raw_len(header) == 64 * 1024 * 1024
    with pytest.raises(cramjam.OutputLimitExceeded):
        cramjam.snappy.decompress_raw(header, max_output_size=16)
    with pytest.raises(cramjam.DecompressionError) as error:
        cramjam.snappy.decompress_raw(b"\x80", max_output_size=16)
    assert not isinstance(error.value, cramjam.OutputLimitExceeded)


@pytest.mark.parametrize("store_size", [True, False])
def test_raw_lz4(store_size):
    supported("lz4")
    data = b"raw" * 30000
    compressed = cramjam.lz4.compress_block(data, store_size=store_size)
    hint = None if store_size else len(data)
    assert (
        bytes(cramjam.lz4.decompress_block(compressed, hint, max_output_size=len(data)))
        == data
    )
    with pytest.raises(cramjam.OutputLimitExceeded):
        cramjam.lz4.decompress_block(compressed, hint, max_output_size=len(data) - 1)


def test_raw_lz4_rejects_declared_length_before_allocation():
    supported("lz4")
    with pytest.raises(cramjam.OutputLimitExceeded):
        cramjam.lz4.decompress_block(
            struct.pack("<I", 64 * 1024 * 1024), max_output_size=16
        )


@pytest.mark.skipif(not PURE_RUST, reason="pure-Rust backend only")
@pytest.mark.parametrize("name", [n for n in CODECS if n not in ("snappy", "brotli")])
def test_unsupported_backend_rejects_limit_before_decoding(name):
    # Never silently accept a cap that cannot bound this backend's allocations.
    with pytest.raises(NotImplementedError, match="max_output_size"):
        getattr(cramjam, name).decompress(b"invalid", max_output_size=0)
    if name == "lz4":
        with pytest.raises(NotImplementedError, match="max_output_size"):
            cramjam.lz4.decompress_block(b"invalid", max_output_size=0)


def test_limit_exception_is_a_decompression_error():
    assert issubclass(cramjam.OutputLimitExceeded, cramjam.DecompressionError)
