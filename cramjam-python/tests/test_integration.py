"""
Test decompressing files which have been compressed from 
main stream third party implementations, separate from this project.
"""
import sys
import pathlib
from collections import namedtuple

import pytest
import cramjam


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
    ),
)
def test_variant(variant: Variant, integration_dir: pathlib.Path, plaintext: bytes):
    file = integration_dir.joinpath(f"plaintext.txt.{variant.suffix}")
    decompress = getattr(cramjam, variant.name).decompress
    assert bytes(decompress(file.read_bytes())) == plaintext
