import os
import subprocess
import tempfile
import pathlib
from datetime import timedelta

import pytest
from hypothesis import strategies as st, given, settings

import cramjam

VARIANTS = ("snappy", "brotli", "bzip2", "lz4", "gzip", "deflate", "zstd")

# Some OS can be slow or have higher variability in their runtimes on CI
settings.register_profile("local", deadline=timedelta(milliseconds=1000))
settings.register_profile("CI", deadline=None, max_examples=10)
if os.getenv("CI"):
    settings.load_profile("CI")
else:
    settings.load_profile("local")


def run_command(cmd) -> bytes:
    return subprocess.check_output(cmd.split(), stderr=subprocess.STDOUT)


@given(data=st.binary(min_size=1))
@pytest.mark.parametrize("variant", VARIANTS)
def test_cli_file_to_file(data, variant):

    with tempfile.TemporaryDirectory() as tmpdir:
        infile = pathlib.Path(tmpdir).joinpath("input.txt")
        infile.write_bytes(data)

        compressed_file = pathlib.Path(tmpdir).joinpath(f"input.txt.{variant}")

        cmd = f"cramjam-cli {variant} compress --input {infile} --output {compressed_file}"
        run_command(cmd)

        expected = bytes(getattr(cramjam, variant).compress(data))
        assert expected == compressed_file.read_bytes()

        decompressed_file = pathlib.Path(tmpdir).joinpath("decompressed.txt")
        run_command(
            f"cramjam-cli {variant} decompress --input {compressed_file} --output {decompressed_file}"
        )
        assert data == decompressed_file.read_bytes()


@given(data=st.binary(min_size=1))
@pytest.mark.parametrize("variant", VARIANTS)
def test_cli_file_to_stdout(data, variant):

    with tempfile.TemporaryDirectory() as tmpdir:
        infile = pathlib.Path(tmpdir).joinpath("input.txt")
        infile.write_bytes(data)

        cmd = f"cramjam-cli {variant} compress --input {infile}"
        out = run_command(cmd)

        expected = bytes(getattr(cramjam, variant).compress(data))
        assert expected == out

        compressed = pathlib.Path(tmpdir).joinpath(f"compressed.txt.{variant}")
        compressed.write_bytes(expected)

        cmd = f"cramjam-cli {variant} decompress --input {compressed}"
        out = run_command(cmd)
        assert out == data
