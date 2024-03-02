import os
import gzip
import pytest
import numpy as np
import cramjam
import hashlib
from datetime import timedelta
from hypothesis import strategies as st, given, settings
from hypothesis.extra import numpy as st_np

VARIANTS = ("snappy", "brotli", "bzip2", "lz4", "gzip", "deflate", "zstd", "lzma")

# LZMA is experimental, but in testing we'll treat it like it's not in the
# experimental submodule.
# TODO: Maybe rename it to XZ, since LZMA is the legacy version.
# ref: https://github.com/fpgaminer/rust-lzma/issues/18, but then
# the rustlib and the clib both are lzma... so maybe not?
cramjam.lzma = cramjam.experimental.lzma

# Some OS can be slow or have higher variability in their runtimes on CI
settings.register_profile(
    "local", deadline=timedelta(milliseconds=1000), max_examples=100
)
settings.register_profile("CI", deadline=None, max_examples=25)
if os.getenv("CI"):
    settings.load_profile("CI")
else:
    settings.load_profile("local")


def same_same(a, b):
    return hashlib.md5(a).hexdigest() == hashlib.md5(b).hexdigest()


def test_has_version():
    from cramjam import __version__

    assert isinstance(__version__, str)


@pytest.mark.parametrize("variant_str", VARIANTS)
@given(arr=st_np.arrays(st_np.scalar_dtypes(), shape=st.integers(0, int(1e4))))
def test_variants_different_dtypes(variant_str, arr, is_pypy):
    variant = getattr(cramjam, variant_str)
    compressed = variant.compress(arr)
    decompressed = variant.decompress(compressed)
    assert same_same(bytes(decompressed), arr.tobytes())

    # And compress n dims > 1
    if arr.shape[0] % 2 == 0:
        arr = arr.reshape((2, -1))

        if is_pypy:
            try:
                compressed = variant.compress(arr)
            except:
                pytest.xfail(reason="PyPy struggles w/ multidim buffer views depending on dtype ie datetime[64]")
        else:
            compressed = variant.compress(arr)
        decompressed = variant.decompress(compressed)
        assert same_same(bytes(decompressed), arr.tobytes())


@pytest.mark.parametrize("is_bytearray", (True, False))
@pytest.mark.parametrize("variant_str", VARIANTS)
@given(uncompressed=st.binary(min_size=1))
def test_variants_simple(variant_str, is_bytearray, uncompressed: bytes):
    variant = getattr(cramjam, variant_str)

    if is_bytearray:
        uncompressed = bytearray(uncompressed)

    compressed = variant.compress(uncompressed)
    assert compressed.read() != uncompressed
    compressed.seek(0)
    assert isinstance(compressed, cramjam.Buffer)

    decompressed = variant.decompress(compressed, output_len=len(uncompressed))
    assert same_same(decompressed.read(), uncompressed)
    assert isinstance(decompressed, cramjam.Buffer)


@pytest.mark.parametrize("variant_str", VARIANTS)
def test_variants_raise_exception(variant_str):
    variant = getattr(cramjam, variant_str)
    with pytest.raises(cramjam.DecompressionError):
        variant.decompress(b"sknow")


@pytest.mark.parametrize(
    "input_type", (bytes, bytearray, "numpy", cramjam.Buffer, cramjam.File, memoryview)
)
@pytest.mark.parametrize(
    "output_type", (bytes, bytearray, "numpy", cramjam.Buffer, cramjam.File, memoryview)
)
@pytest.mark.parametrize("variant_str", VARIANTS)
@given(raw_data=st.binary())
def test_variants_compress_into(
    variant_str, input_type, output_type, raw_data, tmp_path_factory, is_pypy
):
    variant = getattr(cramjam, variant_str)

    # Setup input
    if input_type == "numpy":
        input = np.frombuffer(raw_data, dtype=np.uint8)
    elif input_type == cramjam.File:
        path = tmp_path_factory.mktemp("tmp").joinpath("input.txt")
        path.touch()
        input = cramjam.File(str(path))
        input.write(raw_data)
        input.seek(0)
    elif input_type == cramjam.Buffer:
        input = cramjam.Buffer()
        input.write(raw_data)
        input.seek(0)
    else:
        input = input_type(raw_data)

    compressed = variant.compress(raw_data)
    compressed_len = len(compressed)

    # Setup output buffer
    if output_type == "numpy":
        output = np.zeros(compressed_len, dtype=np.uint8)
    elif output_type == cramjam.File:
        path = tmp_path_factory.mktemp("tmp").joinpath("output.txt")
        path.touch()
        output = cramjam.File(str(path))
    elif output_type == cramjam.Buffer:
        output = cramjam.Buffer()
    else:
        output = output_type(b"0" * compressed_len)

    if is_pypy and isinstance(output, (bytes, memoryview)):
        with pytest.raises(TypeError):
            variant.compress_into(input, output)
        return

    n_bytes = variant.compress_into(input, output)
    assert n_bytes == compressed_len

    if hasattr(output, "read"):
        output.seek(0)
        output = output.read()
    elif hasattr(output, "tobytes"):
        output = output.tobytes()
    else:
        output = bytes(output)
    assert same_same(output, compressed)


@pytest.mark.parametrize(
    "input_type", (bytes, bytearray, "numpy", cramjam.Buffer, cramjam.File, memoryview)
)
@pytest.mark.parametrize(
    "output_type", (bytes, bytearray, "numpy", cramjam.Buffer, cramjam.File, memoryview)
)
@pytest.mark.parametrize("variant_str", VARIANTS)
@given(raw_data=st.binary())
def test_variants_decompress_into(
    variant_str, input_type, output_type, tmp_path_factory, raw_data, is_pypy
):
    variant = getattr(cramjam, variant_str)

    compressed = variant.compress(raw_data)

    # Setup input
    if input_type == "numpy":
        input = np.frombuffer(compressed, dtype=np.uint8)
    elif input_type == cramjam.File:
        path = tmp_path_factory.mktemp("tmp").joinpath("input.txt")
        path.touch()
        input = cramjam.File(str(path))
        input.write(compressed)
        input.seek(0)
    elif input_type == cramjam.Buffer:
        input = cramjam.Buffer()
        input.write(compressed)
        input.seek(0)
    else:
        input = input_type(compressed)

    # Setup output buffer
    if output_type == "numpy":
        output = np.zeros(len(raw_data), dtype=np.uint8)
    elif output_type == cramjam.File:
        path = tmp_path_factory.mktemp("tmp").joinpath("output.txt")
        path.touch()
        output = cramjam.File(str(path))
    elif output_type == cramjam.Buffer:
        output = cramjam.Buffer()
    else:
        output = output_type(b"0" * len(raw_data))

    if is_pypy and isinstance(output, (bytes, memoryview)):
        with pytest.raises(TypeError):
            variant.decompress_into(input, output)
        return

    n_bytes = variant.decompress_into(input, output)
    assert n_bytes == len(raw_data)

    if hasattr(output, "read"):
        output.seek(0)
        output = output.read()
    elif hasattr(output, "tobytes"):
        output = output.tobytes()
    else:
        output = bytes(output)
    assert same_same(output, raw_data)


@given(data=st.binary())
def test_variant_snappy_raw_into(data):
    """
    A little more special than other de/compress_into variants, as the underlying
    snappy raw api makes a hard expectation that its calculated len is used.
    """

    compressed = cramjam.snappy.compress_raw(data)
    compressed_size = cramjam.snappy.compress_raw_max_len(data)
    compressed_buffer = np.zeros(compressed_size, dtype=np.uint8)
    n_bytes = cramjam.snappy.compress_raw_into(data, compressed_buffer)
    assert n_bytes == len(compressed)

    decompressed_buffer = np.zeros(len(data), dtype=np.uint8)
    n_bytes = cramjam.snappy.decompress_raw_into(
        compressed_buffer[:n_bytes].tobytes(), decompressed_buffer
    )
    assert n_bytes == len(data)

    assert same_same(decompressed_buffer[:n_bytes], data)


@given(data=st.binary())
def test_variant_lz4_block_into(data):
    """
    A little more special than other de/compress_into variants, as the underlying
    snappy raw api makes a hard expectation that its calculated len is used.
    """

    compressed = cramjam.lz4.compress_block(data)
    compressed_size = cramjam.lz4.compress_block_bound(data)
    compressed_buffer = np.zeros(compressed_size, dtype=np.uint8)
    n_bytes = cramjam.lz4.compress_block_into(data, compressed_buffer)
    assert n_bytes == len(compressed)
    assert same_same(compressed, compressed_buffer[:n_bytes])

    decompressed_buffer = np.zeros(len(data), dtype=np.uint8)
    n_bytes = cramjam.lz4.decompress_block_into(
        compressed_buffer[:n_bytes].tobytes(), decompressed_buffer
    )
    assert n_bytes == len(data)

    assert same_same(decompressed_buffer[:n_bytes], data)


@pytest.mark.parametrize("Obj", (cramjam.File, cramjam.Buffer))
@given(data=st.binary())
def test_dunders(Obj, tmp_path_factory, data):
    if Obj == cramjam.File:
        path = tmp_path_factory.mktemp("tmp").joinpath("tmp.txt")
        path.touch()
        obj = Obj(str(path))
    else:
        obj = Obj()

    assert len(obj) == 0
    assert bool(obj) is False
    obj.write(data)
    assert len(obj) == len(data)
    assert bool(obj) is bool(len(data))

    assert f"len={len(data)}" in str(obj)
    if isinstance(obj, cramjam.File):
        assert f"path={path}" in str(obj)


@pytest.mark.parametrize(
    "compress_kwargs",
    (
        dict(mode="default", acceleration=1, compression=1, store_size=True),
        dict(mode="fast", acceleration=2, compression=2, store_size=False),
        dict(mode="high_compression", acceleration=3, compression=3, store_size=True),
        dict(mode="default", acceleration=5, compression=4, store_size=False),
    ),
)
def test_lz4_block(compress_kwargs):
    from cramjam import lz4

    data = b"howdy neighbor"

    # What python-lz4 outputs in block mode
    expected = b"\x0e\x00\x00\x00\xe0howdy neighbor"
    assert bytes(lz4.compress_block(data)) == expected

    # and what it does without 'store_size=True'
    expected = b"\xe0howdy neighbor"
    assert bytes(lz4.compress_block(data, store_size=False)) == expected

    # Round trip the current collection of compression kwargs
    out = lz4.decompress_block(
        lz4.compress_block(data, **compress_kwargs),
        output_len=len(data) if not compress_kwargs["store_size"] else None,
    )
    assert same_same(bytes(out), data)


@given(first=st.binary(), second=st.binary())
def test_gzip_multiple_streams(first: bytes, second: bytes):
    out1 = gzip.compress(first)
    out2 = gzip.compress(second)
    assert gzip.decompress(out1 + out2) == first + second

    # works with data compressed by std gzip lib
    out = bytes(cramjam.gzip.decompress(out1 + out2))
    assert out == first + second

    # works with data compressed by cramjam
    o1 = bytes(cramjam.gzip.compress(first))
    o2 = bytes(cramjam.gzip.compress(second))
    out = bytes(cramjam.gzip.decompress(o1 + o2))
    assert same_same(out, first + second)


@pytest.mark.parametrize(
    "mod",
    (
        cramjam.brotli,
        cramjam.bzip2,
        cramjam.deflate,
        cramjam.gzip,
        cramjam.lz4,
        cramjam.snappy,
        cramjam.zstd,
    ),
)
@given(first=st.binary(), second=st.binary())
def test_streams_compressor(mod, first: bytes, second: bytes):
    compressor = mod.Compressor()

    compressor.compress(first)
    out = bytes(compressor.flush())

    compressor.compress(second)
    out += bytes(compressor.flush())

    out += bytes(compressor.finish())
    decompressed = mod.decompress(out)
    assert same_same(bytes(decompressed), first + second)

    # just empty bytes after the first .finish()
    # same behavior as brotli.Compressor()
    assert bytes(compressor.finish()) == b""

    # compress will raise an error as the stream is completed
    with pytest.raises(cramjam.CompressionError):
        compressor.compress(b"data")


@pytest.mark.parametrize("variant_str", VARIANTS)
def test_variants_stream_decompressors(variant_str):
    variant = getattr(cramjam, variant_str)
    Decompressor = getattr(variant, "Decompressor")
    decompressor = Decompressor()

    compressed = variant.compress(b"bytes")
    for _ in range(2):
        n_bytes = decompressor.decompress(bytes(compressed))
        assert n_bytes == 5
    assert bytes(decompressor.flush()) == b"bytesbytes"
    assert bytes(decompressor.flush()) == b""

    decompressor.decompress(bytes(compressed))
    assert bytes(decompressor.finish()) == b"bytes"

    # Calling .finish renders decompressor unusable after. (API consistency with other libs)
    with pytest.raises(cramjam.DecompressionError):
        decompressor.finish()


def test_buffer_cmp():
    assert cramjam.Buffer() == cramjam.Buffer()
    assert cramjam.Buffer(b"some bytes") == cramjam.Buffer(b"some bytes")
    assert cramjam.Buffer(b"some bytes") != cramjam.Buffer(b"other bytes")
