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
