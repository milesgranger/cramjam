import gzip
import pytest
import numpy as np
import cramjam
import hashlib


def same_same(a, b):
    return hashlib.md5(a).hexdigest() == hashlib.md5(b).hexdigest()


def test_has_version():
    from cramjam import __version__

    assert isinstance(__version__, str)


@pytest.mark.parametrize("is_bytearray", (True, False))
@pytest.mark.parametrize(
    "variant_str", ("snappy", "brotli", "lz4", "gzip", "deflate", "zstd")
)
def test_variants_simple(variant_str, is_bytearray):

    variant = getattr(cramjam, variant_str)

    uncompressed = b"some bytes to compress 123" * 100000
    if is_bytearray:
        uncompressed = bytearray(uncompressed)

    compressed = variant.compress(uncompressed)
    assert compressed.read() != uncompressed
    compressed.seek(0)
    assert isinstance(compressed, cramjam.Buffer)

    decompressed = variant.decompress(compressed, output_len=len(uncompressed))
    assert decompressed.read() == uncompressed
    assert isinstance(decompressed, cramjam.Buffer)


@pytest.mark.parametrize(
    "variant_str", ("snappy", "brotli", "lz4", "gzip", "deflate", "zstd")
)
def test_variants_raise_exception(variant_str):
    variant = getattr(cramjam, variant_str)
    with pytest.raises(cramjam.DecompressionError):
        variant.decompress(b"sknow")


@pytest.mark.parametrize(
    "input_type", (bytes, bytearray, "numpy", cramjam.Buffer, cramjam.File)
)
@pytest.mark.parametrize(
    "output_type", (bytes, bytearray, "numpy", cramjam.Buffer, cramjam.File)
)
@pytest.mark.parametrize(
    "variant_str", ("snappy", "brotli", "gzip", "deflate", "zstd", "lz4")
)
def test_variants_compress_into(variant_str, input_type, output_type, tmpdir):
    variant = getattr(cramjam, variant_str)

    raw_data = b"oh what a beautiful morning, oh what a beautiful day!!" * 10000

    # Setup input
    if input_type == "numpy":
        input = np.frombuffer(raw_data, dtype=np.uint8)
    elif input_type == cramjam.File:
        input = cramjam.File(str(tmpdir.join("input.txt")))
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
        output = cramjam.File(str(tmpdir.join("output.txt")))
    elif output_type == cramjam.Buffer:
        output = cramjam.Buffer()
    else:
        output = output_type(b"0" * compressed_len)

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
    "input_type", (bytes, bytearray, "numpy", cramjam.Buffer, cramjam.File)
)
@pytest.mark.parametrize(
    "output_type", (bytes, bytearray, "numpy", cramjam.Buffer, cramjam.File)
)
@pytest.mark.parametrize(
    "variant_str", ("snappy", "brotli", "gzip", "deflate", "zstd", "lz4")
)
def test_variants_decompress_into(variant_str, input_type, output_type, tmpdir):
    variant = getattr(cramjam, variant_str)

    raw_data = b"oh what a beautiful morning, oh what a beautiful day!!" * 100
    compressed = variant.compress(raw_data)

    # Setup input
    if input_type == "numpy":
        input = np.frombuffer(compressed, dtype=np.uint8)
    elif input_type == cramjam.File:
        input = cramjam.File(str(tmpdir.join("input.txt")))
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
        output = cramjam.File(str(tmpdir.join("output.txt")))
    elif output_type == cramjam.Buffer:
        output = cramjam.Buffer()
    else:
        output = output_type(b"0" * len(raw_data))

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


def test_variant_snappy_raw_into():
    """
    A little more special than other de/compress_into variants, as the underlying
    snappy raw api makes a hard expectation that its calculated len is used.
    """
    data = b"oh what a beautiful morning, oh what a beautiful day!!" * 1000000

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


@pytest.mark.parametrize("Obj", (cramjam.File, cramjam.Buffer))
def test_dunders(Obj, tmpdir):
    if Obj == cramjam.File:
        path = str(tmpdir.join("tmp.txt"))
        obj = Obj(path)
    else:
        obj = Obj()

    assert len(obj) == 0
    assert bool(obj) is False
    obj.write(b"12345")
    assert len(obj) == 5
    assert bool(obj) is True

    assert "len=5" in str(obj)
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
    assert bytes(out) == data


def test_gzip_multiple_streams():

    out1 = gzip.compress(b"foo")
    out2 = gzip.compress(b"bar")
    assert gzip.decompress(out1 + out2) == b"foobar"

    # works with data compressed by std gzip lib
    out = bytes(cramjam.gzip.decompress(out1 + out2))
    assert out == b"foobar"

    # works with data compressed by cramjam
    o1 = bytes(cramjam.gzip.compress(b"foo"))
    o2 = bytes(cramjam.gzip.compress(b"bar"))
    out = bytes(cramjam.gzip.decompress(o1 + o2))
    assert out == b"foobar"


@pytest.mark.parametrize(
    "mod",
    (
        cramjam.brotli,
        cramjam.deflate,
        cramjam.gzip,
        cramjam.lz4,
        cramjam.snappy,
        cramjam.zstd,
    ),
)
def test_streams_compressor(mod):
    compressor = mod.Compressor()
    compressor.compress(b"foo")
    compressor.compress(b"bar")
    out = compressor.finish()
    decompressed = mod.decompress(out)
    assert bytes(decompressed) == b"foobar"

    # just empty bytes after the first .finish()
    # same behavior as brotli.Compressor()
    assert bytes(compressor.finish()) == b""

    # compress will raise an error as the stream is completed
    with pytest.raises(cramjam.CompressionError):
        compressor.compress(b'data')
