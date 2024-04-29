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
