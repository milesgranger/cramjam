import gzip
import pytest
import cramjam
import pathlib
import numpy as np


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
def test_snappy(benchmark, file, use_cramjam: bool):
    """
    Uses snappy compression
    """
    import snappy

    data = bytearray(
        file.read_bytes()
    )  # bytearray avoids double allocation in cramjam snappy by default
    # Can be slightly faster if passing output_len to compress/decompress ops
    if use_cramjam:
        benchmark(
            round_trip,
            compress=cramjam.snappy.compress,
            decompress=cramjam.snappy.decompress,
            data=data,
        )
    else:
        benchmark(
            round_trip,
            compress=snappy.compress,
            decompress=snappy.decompress,
            data=data,
        )


@pytest.mark.parametrize("op", ("decompress_into", "compress_into"))
@pytest.mark.parametrize("file", FILES, ids=lambda val: val.name)
def test_cramjam_snappy_de_compress_into(benchmark, op, file):
    """
    Uses decompress_into for snappy compression
    """
    from cramjam import snappy

    data = bytearray(file.read_bytes())
    compressed_data = cramjam.snappy.compress(data)

    operation = getattr(snappy, op)
    buffer = np.zeros(
        len(data) if op == "decompress_into" else len(compressed_data), dtype=np.uint8
    )

    benchmark(
        lambda data, buffer: operation(data, buffer),
        data=compressed_data if op == "decompress_into" else data,
        buffer=buffer,
    )


@pytest.mark.parametrize(
    "use_cramjam", (True, False), ids=lambda val: "cramjam" if val else "gzip"
)
@pytest.mark.parametrize(
    "set_output_len", (True, False), ids=lambda val: f"used-output_len={val}"
)
@pytest.mark.parametrize("file", FILES, ids=lambda val: val.name)
def test_gzip(benchmark, file, use_cramjam: bool, set_output_len: bool):
    data = file.read_bytes()
    if use_cramjam:
        if set_output_len:
            compressed_len = len(cramjam.gzip.compress(data))
            benchmark(
                round_trip,
                compress=lambda bytes: cramjam.gzip.compress(
                    bytes, level=9, output_len=compressed_len
                ),
                decompress=lambda bytes: cramjam.gzip.decompress(
                    bytes, output_len=len(data)
                ),
                data=data,
            )
        else:
            benchmark(
                round_trip,
                compress=cramjam.gzip.compress,
                decompress=cramjam.gzip.decompress,
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
            compress=cramjam.lz4.compress,
            decompress=cramjam.lz4.decompress,
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
            compress=cramjam.brotli.compress,
            decompress=cramjam.brotli.decompress,
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
            compress=cramjam.zstd.compress,
            decompress=cramjam.zstd.decompress,
            data=data,
        )
    else:
        benchmark(
            round_trip, compress=zstd.compress, decompress=zstd.decompress, data=data,
        )
