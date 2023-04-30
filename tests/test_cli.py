import pathlib
import subprocess
import pytest
import cramjam
import tempfile
from hypothesis import strategies as st, given, settings

from tests.test_variants import VARIANTS


def run_command(command: str) -> bytes:
    p = subprocess.Popen(
        command.split(), stderr=subprocess.PIPE, stdout=subprocess.PIPE
    )
    p.wait()
    stdout, stderr = p.communicate()
    if stderr:
        raise OSError(stderr)
    return stdout


@pytest.mark.parametrize("variant", VARIANTS)
@given(uncompressed=st.binary(min_size=1))
def test_cli(uncompressed: bytes, variant):

    with tempfile.TemporaryDirectory() as tmp_path:
        input = pathlib.Path(tmp_path).joinpath("data.txt")
        input.write_bytes(uncompressed)

        compressed = pathlib.Path(tmp_path).joinpath("data.txt.compressed")
        run_command(f"cramjam {variant} compress --input {input} --output {compressed}")

        expected = getattr(cramjam, variant).compress(uncompressed)
        assert compressed.read_bytes() == bytes(expected)

        decompressed = pathlib.Path(tmp_path).joinpath("data.txt.decompressed")
        run_command(
            f"cramjam {variant} decompress --input {compressed} --output {decompressed}"
        )
        assert decompressed.read_bytes() == uncompressed
