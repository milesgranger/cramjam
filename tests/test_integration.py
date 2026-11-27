"""
Test decompressing files which have been compressed from
main stream third party implementations, separate from this project.
"""

import sys
import lzma
import pathlib
from collections import namedtuple

import pytest
from hypothesis import strategies as st, given, settings
from hypothesis.extra import numpy as st_np

import cramjam
from .test_variants import same_same


@pytest.fixture
def integration_dir():
    return pathlib.Path(__file__).parent.joinpath("data/integration")


@pytest.fixture
def plaintext(integration_dir):
    return integration_dir.joinpath("plaintext.txt").read_bytes()


Variant = namedtuple("Variant", ("name", "suffix"))


@pytest.mark.skipif(
    sys.platform.startswith("win"), reason="Bytes comparison fails on windows"
)
@pytest.mark.parametrize(
    "variant",
    (
        Variant("gzip", "gz"),
        Variant("bzip2", "bz2"),
        Variant("zstd", "zst"),
        Variant("brotli", "br"),
        Variant("lz4", "lz4"),
        Variant("snappy", "snappy"),
        Variant("xz", "lzma"),
    ),
)
def test_variant(variant: Variant, integration_dir: pathlib.Path, plaintext: bytes):
    file = integration_dir.joinpath(f"plaintext.txt.{variant.suffix}")
    decompress = getattr(cramjam, variant.name).decompress
    assert bytes(decompress(file.read_bytes())) == plaintext


@given(data=st.binary(min_size=1, max_size=int(1e6)))
@pytest.mark.parametrize("format", (lzma.FORMAT_ALONE, lzma.FORMAT_XZ))
def test_lzma_compat(data, format):
    # Decompress from std lzma lib
    compressed = lzma.compress(data, format=format)
    uncompressed = cramjam.xz.decompress(compressed)
    assert same_same(bytes(uncompressed), data)

    # std lzma lib can decompress us
    cjformat = (
        cramjam.xz.Format.ALONE if format == lzma.FORMAT_ALONE else cramjam.xz.Format.XZ
    )
    compressed = cramjam.xz.compress(data, format=cjformat)
    uncompressed = lzma.decompress(bytes(compressed), format=format)
    assert same_same(uncompressed, data)


@given(data=st.binary(min_size=1, max_size=int(1e6)))
@pytest.mark.parametrize("set_output_len", (True, False))
def test_lz4_decompress_block_into_non_prepended_size(data, set_output_len):
    compressed = cramjam.lz4.compress_block(data, store_size=False)

    # Check both scenarios of user explicitly providing output_len or not
    output_len = len(data) if set_output_len else None

    # If we have data, and the output buffer isn't long enough
    # it ought to fail
    if data:
        with pytest.raises(cramjam.DecompressionError):
            cramjam.lz4.decompress_block_into(
                compressed, bytearray(0), output_len=output_len
            )

        # But if we explicitly provide the right output_len,
        # while the output is less, we'll know why.
        match = f"output_len set to {len(data)}, but output is less"
        with pytest.raises(cramjam.DecompressionError, match=match):
            cramjam.lz4.decompress_block_into(
                compressed, bytearray(0), output_len=len(data)
            )

    # If it's the same length as original data, it's okay.
    out = bytearray(len(data))
    n = cramjam.lz4.decompress_block_into(compressed, out, output_len=output_len)
    assert same_same(bytes(out), data)

    # pre-allocated buffer is larger, also okay
    out = bytearray(len(compressed) * 2)
    n = cramjam.lz4.decompress_block_into(compressed, out, output_len=output_len)
    assert same_same(bytes(out[:n]), data)


def _lz4_block_text(n):
    words = [b"alpha ", b"beta ", b"gamma ", b"delta ", b"lorem ", b"ipsum ", b"\n"]
    state = 0x9E3779B9
    out = bytearray()
    while len(out) < n:
        state = (state * 1103515245 + 12345) % 2**31
        out += words[state % len(words)]
    return bytes(out)


@pytest.mark.parametrize(
    "kwargs",
    (
        {},
        {"mode": "default"},
        {"mode": "fast"},
        {"mode": "fast", "acceleration": 8},
        {"mode": "high_compression"},
        {"mode": "high_compression", "compression": 1},
        {"mode": "high_compression", "compression": 4},
        {"mode": "high_compression", "compression": 12},
    ),
)
@pytest.mark.parametrize("store_size", (True, False))
def test_lz4_compress_block_modes_round_trip(kwargs, store_size):
    data = _lz4_block_text(100_000)
    compressed = cramjam.lz4.compress_block(data, store_size=store_size, **kwargs)
    output_len = None if store_size else len(data)
    assert same_same(bytes(cramjam.lz4.decompress_block(compressed, output_len)), data)

    out = bytearray(cramjam.lz4.compress_block_bound(data))
    n = cramjam.lz4.compress_block_into(data, out, store_size=store_size, **kwargs)
    assert bytes(out[:n]) == bytes(compressed)


def test_lz4_compress_block_high_compression_level():
    # https://github.com/milesgranger/cramjam/issues/265
    data = _lz4_block_text(500_000)

    def size(**kwargs):
        return len(cramjam.lz4.compress_block(data, store_size=False, **kwargs))

    default = size()
    hc4 = size(mode="high_compression", compression=4)
    hc9 = size(mode="high_compression", compression=9)
    assert hc4 < default
    assert hc9 < hc4
    # as in python-lz4, `compression` only applies to mode="high_compression"
    assert size(compression=9) == default
    assert size(mode="high_compression") == hc9


def test_lz4_compress_block_fast_acceleration():
    data = _lz4_block_text(500_000)

    def size(**kwargs):
        return len(cramjam.lz4.compress_block(data, store_size=False, **kwargs))

    default = size()
    assert size(mode="fast") == default
    assert size(mode="fast", acceleration=1) == default
    assert size(mode="fast", acceleration=64) > default
    # as in python-lz4, `acceleration` only applies to mode="fast"
    assert size(acceleration=64) == default
    assert size(mode="high_compression", acceleration=64) == size(
        mode="high_compression"
    )


@pytest.mark.parametrize("func", ("compress_block", "compress_block_into"))
def test_lz4_compress_block_invalid_mode(func):
    args = (
        (b"some bytes", bytearray(64))
        if func == "compress_block_into"
        else (b"some bytes",)
    )
    with pytest.raises(ValueError, match="Invalid mode"):
        getattr(cramjam.lz4, func)(*args, mode="not-a-mode")
    with pytest.raises(ValueError, match="non-negative"):
        getattr(cramjam.lz4, func)(*args, mode="high_compression", compression=-1)


def test_module_registration():
    import cramjam

    assert cramjam.snappy.__name__ == "snappy"
    assert cramjam.experimental.__name__ == "experimental"
    assert issubclass(cramjam.CompressionError, Exception)
    assert issubclass(cramjam.DecompressionError, Exception)


def test_xz_configuration_objects():
    import cramjam

    options = cramjam.xz.Options()
    options.set_mode(cramjam.xz.Mode.Normal)
    options.set_mf(cramjam.xz.MatchFinder.BinaryTree4)
    filters = cramjam.xz.FilterChain()
    filters.append_filter(cramjam.xz.FilterChainItem(cramjam.xz.Filter.Lzma2, options))

    data = b"configured xz compression"
    compressed = cramjam.xz.compress(
        data,
        format=cramjam.xz.Format.XZ,
        check=cramjam.xz.Check.Crc64,
        filters=filters,
    )
    assert bytes(cramjam.xz.decompress(compressed)) == data
