import gzip
import pytest
import cramjam
import pathlib
import numpy as np
from memory_profiler import profile

FILES = [
    f
    for f in pathlib.Path("benchmarks/data").iterdir()
    if f.is_file() and f.name != "COPYING"
]


class FiftyFourMbRepeating:
    """
    54mb of data, where the first 54bytes are repeated 1000000 times.
    """

    name = "fifty-four-mb-repeating"

    def read_bytes(self):
        return b"oh what a beautiful morning, oh what a beautiful day!!" * 1000000


class FiftyFourMbRandom:
    """
    54mb of data, all random
    """

    name = "fifty-four-mb-random"

    def read_bytes(self):
        return np.random.randint(0, 255, size=54000000, dtype=np.uint8).tobytes()


FILES.extend([FiftyFourMbRepeating(), FiftyFourMbRandom()])


def round_trip(compress, decompress, data, **kwargs):
    return decompress(compress(data, **kwargs))


@pytest.mark.parametrize(
    "use_cramjam", (True, False), ids=lambda val: "cramjam" if val else "snappy"
)
@pytest.mark.parametrize("file", FILES, ids=lambda val: val.name)
def test_snappy_raw(benchmark, file, use_cramjam: bool):
    """
    Uses snappy compression raw
    """
    import snappy

    data = file.read_bytes()
    if use_cramjam:
        benchmark(
            round_trip,
            compress=cramjam.snappy.compress_raw,
            decompress=cramjam.snappy.decompress_raw,
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
    "use_cramjam", (True, False), ids=lambda val: "cramjam" if val else "snappy"
)
@pytest.mark.parametrize("file", FILES, ids=lambda val: val.name)
def test_snappy_framed(benchmark, file, use_cramjam: bool):
    """
    Uses snappy compression framed
    """
    import snappy

    data = bytearray(file.read_bytes())
    if use_cramjam:
        benchmark(
            round_trip,
            compress=cramjam.snappy.compress,
            decompress=cramjam.snappy.decompress,
            data=data,
        )
    else:
        compressor = snappy.StreamCompressor()
        decompressor = snappy.StreamDecompressor()
        benchmark(
            round_trip,
            compress=compressor.compress,
            decompress=decompressor.decompress,
            data=data,
        )


@pytest.mark.parametrize("op", ("decompress_into", "compress_into"))
@pytest.mark.parametrize("file", FILES, ids=lambda val: val.name)
def test_cramjam_snappy_de_compress_into(benchmark, op, file):
    """
    Uses decompress_into for snappy compression
    """
    from cramjam import snappy

    data = file.read_bytes()
    compressed_data = cramjam.snappy.compress(data)

    operation = getattr(snappy, op)
    buffer = np.zeros(
        len(data) if op == "decompress_into" else len(compressed_data),
        dtype=np.uint8,
    )

    benchmark(
        lambda data, buffer: operation(data, buffer),
        data=compressed_data if op == "decompress_into" else data,
        buffer=buffer,
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
    "use_cramjam", (True, False), ids=lambda val: "cramjam" if val else "python-lz4"
)
@pytest.mark.parametrize("file", FILES, ids=lambda val: val.name)
def test_lz4_block(benchmark, file, use_cramjam: bool):
    from lz4 import block

    data = file.read_bytes()
    if use_cramjam:
        benchmark(
            round_trip,
            compress=cramjam.lz4.compress_block,
            decompress=cramjam.lz4.decompress_block,
            data=data,
        )
    else:
        benchmark(
            round_trip,
            compress=block.compress,
            decompress=block.decompress,
            data=data,
        )


@pytest.mark.parametrize(
    "use_cramjam", (True, False), ids=lambda val: "cramjam" if val else "brotli"
)
@pytest.mark.parametrize(
    "file",
    [
        f
        for f in FILES
        if not (isinstance(f, FiftyFourMbRandom) or isinstance(f, FiftyFourMbRepeating))
    ],
    ids=lambda val: val.name,
)
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
            round_trip,
            compress=zstd.compress,
            decompress=zstd.decompress,
            data=data,
        )


@profile
def memory_profile():

    import snappy

    data = bytearray(FILES[-1].read_bytes())
    out1 = bytes(cramjam.snappy.compress_raw(data))
    _out1 = bytes(cramjam.snappy.decompress_raw(out1))
    out2 = snappy.compress(data)
    _ou2 = snappy.decompress(out2)


if __name__ == "__main__":
    memory_profile()
