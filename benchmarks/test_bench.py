import gzip
import pytest
import cramjam
import pathlib


FILES = [
    f
    for f in pathlib.Path("benchmarks/data").iterdir()
    if f.is_file() and f.name != "COPYING"
]


def round_trip(compress, decompress, data, **kwargs):
    return decompress(compress(data, **kwargs))


@pytest.mark.parametrize(
    "use_cramjam", (True, False), ids=lambda val: "cramjam" if val else "snappy"
)
@pytest.mark.parametrize("file", FILES, ids=lambda val: val.name)
def test_snappy_raw(benchmark, file, use_cramjam: bool):
    """
    Uses the non-framed format for snappy compression
    """
    import snappy

    data = file.read_bytes()
    if use_cramjam:
        benchmark(
            round_trip,
            compress=cramjam.snappy_compress_raw,
            decompress=cramjam.snappy_decompress_raw,
            data=data,
        )
    else:
        benchmark(
            round_trip,
            compress=snappy.compress,
            decompress=snappy.decompress,
            data=data,
        )


@pytest.mark.parametrize(
    "use_cramjam", (True, False), ids=lambda val: "cramjam" if val else "gzip"
)
@pytest.mark.parametrize("file", FILES, ids=lambda val: val.name)
def test_gzip(benchmark, file, use_cramjam: bool):
    data = file.read_bytes()
    if use_cramjam:
        benchmark(
            round_trip,
            compress=cramjam.gzip_compress,
            decompress=cramjam.gzip_decompress,
            data=data,
            level=9,
        )
    else:
        benchmark(
            round_trip,
            compress=gzip.compress,
            decompress=gzip.decompress,
            data=data,
            compresslevel=9,
        )


@pytest.mark.parametrize(
    "use_cramjam", (True, False), ids=lambda val: "cramjam" if val else "python-lz4"
)
@pytest.mark.parametrize("file", FILES, ids=lambda val: val.name)
def test_lz4(benchmark, file, use_cramjam: bool):
    from lz4 import frame

    data = file.read_bytes()
    if use_cramjam:
        benchmark(
            round_trip,
            compress=cramjam.lz4_compress,
            decompress=cramjam.lz4_decompress,
            data=data,
            level=4,
        )
    else:
        benchmark(
            round_trip,
            compress=frame.compress,
            decompress=frame.decompress,
            data=data,
            compression_level=4,
        )


@pytest.mark.parametrize(
    "use_cramjam", (True, False), ids=lambda val: "cramjam" if val else "brotli"
)
@pytest.mark.parametrize("file", FILES, ids=lambda val: val.name)
def test_brotli(benchmark, file, use_cramjam: bool):
    import brotli

    data = file.read_bytes()
    if use_cramjam:
        benchmark(
            round_trip,
            compress=cramjam.brotli_compress,
            decompress=cramjam.brotli_decompress,
            data=data,
        )
    else:
        benchmark(
            round_trip,
            compress=brotli.compress,
            decompress=brotli.decompress,
            data=data,
        )


@pytest.mark.parametrize(
    "use_cramjam", (True, False), ids=lambda val: "cramjam" if val else "zstd"
)
@pytest.mark.parametrize("file", FILES, ids=lambda val: val.name)
def test_zstd(benchmark, file, use_cramjam: bool):
    import zstd

    data = file.read_bytes()
    if use_cramjam:
        benchmark(
            round_trip,
            compress=cramjam.zstd_compress,
            decompress=cramjam.zstd_decompress,
            data=data,
        )
    else:
        benchmark(
            round_trip, compress=zstd.compress, decompress=zstd.decompress, data=data,
        )
