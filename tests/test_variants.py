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

    uncompressed = b"some bytes to compress 123" * 1000
    if is_bytearray:
        uncompressed = bytearray(uncompressed)

    compressed = variant.compress(uncompressed)
    assert compressed != uncompressed
    assert type(compressed) == type(uncompressed)

    decompressed = variant.decompress(compressed, output_len=len(uncompressed))
    assert decompressed == uncompressed
    assert type(decompressed) == type(uncompressed)


@pytest.mark.parametrize(
    "variant_str", ("snappy", "brotli", "lz4", "gzip", "deflate", "zstd")
)
def test_variants_raise_exception(variant_str):
    variant = getattr(cramjam, variant_str)
    with pytest.raises(cramjam.DecompressionError):
        variant.decompress(b"sknow")


@pytest.mark.parametrize("variant_str", ("snappy", "brotli", "gzip", "deflate", "zstd"))
def test_variants_de_compress_into(variant_str):

    # TODO: support lz4 de/compress_into

    variant = getattr(cramjam, variant_str)

    data = b"oh what a beautiful morning, oh what a beautiful day!!" * 1000000

    compressed_array = np.zeros(len(data), dtype=np.uint8)  # plenty of space
    compressed_size = variant.compress_into(data, compressed_array)
    decompressed = variant.decompress(compressed_array[:compressed_size].tobytes())
    assert same_same(decompressed, data)

    compressed = variant.compress(data)
    decompressed_array = np.zeros(len(data), np.uint8)
    decompressed_size = variant.decompress_into(compressed, decompressed_array)
    decompressed = decompressed_array[:decompressed_size].tobytes()
    assert same_same(decompressed, data)


def test_variant_snappy_raw_into():
    """
    A little more special than other de/compress_into variants, as the underlying
    snappy raw api makes a hard expectation that its calculated len is used.
    """
    data = b"oh what a beautiful morning, oh what a beautiful day!!" * 1000000

    compressed_size = cramjam.snappy.compress_raw_max_len(data)
    compressed_buffer = np.zeros(compressed_size, dtype=np.uint8)
    n_bytes = cramjam.snappy.compress_raw_into(data, compressed_buffer)
    assert n_bytes == 2563328

    decompressed_size = cramjam.snappy.decompress_raw_len(
        compressed_buffer[:n_bytes].tobytes()
    )
    assert decompressed_size == len(data)
    decompressed_buffer = np.zeros(decompressed_size, dtype=np.uint8)
    n_bytes = cramjam.snappy.decompress_raw_into(
        compressed_buffer[:n_bytes].tobytes(), decompressed_buffer
    )
    assert n_bytes == len(data)

    assert same_same(decompressed_buffer[:n_bytes], data)
