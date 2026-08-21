from importlib.resources import files
from pathlib import Path

import cramjam


def test_typing_files_are_installed():
    package = files(cramjam)
    expected = {
        "py.typed",
        "__init__.pyi",
        "blosc2.pyi",
        "brotli.pyi",
        "bzip2.pyi",
        "deflate.pyi",
        "experimental.pyi",
        "gzip.pyi",
        "ideflate.pyi",
        "igzip.pyi",
        "izlib.pyi",
        "lz4.pyi",
        "snappy.pyi",
        "xz.pyi",
        "zlib.pyi",
        "zstd.pyi",
    }

    assert expected <= {entry.name for entry in package.iterdir()}


def test_stubs_parse_on_minimum_python():
    package = files(cramjam)
    for entry in package.iterdir():
        if entry.name.endswith(".pyi"):
            compile(entry.read_text(), str(Path(str(entry))), "exec")


def test_runtime_exports_match_stubs():
    package = files(cramjam)
    modules = {
        "brotli": cramjam.brotli,
        "bzip2": cramjam.bzip2,
        "deflate": cramjam.deflate,
        "gzip": cramjam.gzip,
        "lz4": cramjam.lz4,
        "snappy": cramjam.snappy,
        "xz": cramjam.xz,
        "zlib": cramjam.zlib,
        "zstd": cramjam.zstd,
    }

    for name, module in modules.items():
        stub = (package / f"{name}.pyi").read_text()
        for exported in dir(module):
            if not exported.startswith("_"):
                assert f"{exported}" in stub, f"{name}.{exported} is missing from its stub"
