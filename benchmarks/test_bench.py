import gzip
import pytest
import cramjam
import pathlib
import numpy as np


if hasattr(cramjam, "experimental") and not hasattr(cramjam, "blosc2"):
    if hasattr(cramjam.experimental, "blosc2"):
        cramjam.blosc2 = cramjam.experimental.blosc2


class Bzip2CompressedFile:
    """
    Too bad can't just inherit pathlib.Path

    Simple wrapper to decompress benchmark file on read_bytes()
    """

    def __init__(self, path: pathlib.Path):
        self.path = path

    @property
    def name(self):
        return self.path.name.replace(".bz2", "")

    def read_bytes(self):
        return cramjam.bzip2.decompress(self.path.read_bytes()).read()


FILES = [
    Bzip2CompressedFile(f)
    for f in pathlib.Path(__file__).parent.joinpath("data").iterdir()
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
    "use_cramjam", (True, False), ids=lambda val: "cramjam" if val else "blosc2"
)
@pytest.mark.parametrize("file", FILES, ids=lambda val: val.name)
def test_blosc2(benchmark, file, use_cramjam: bool):
    """
    Uses snappy compression raw
    """
    import blosc2

    if not hasattr(cramjam, "blosc2"):
        pytest.skip("blosc2 not built")

    data = file.read_bytes()
    if use_cramjam:
        benchmark(
            round_trip,
            compress=cramjam.blosc2.compress_chunk,
            decompress=cramjam.blosc2.decompress_chunk,
            data=data,
        )
    else:
        benchmark(
            round_trip,
            compress=blosc2.compress,
            decompress=blosc2.decompress,
            data=data,
        )


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
    compressed_data = bytes(cramjam.snappy.compress(data))

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
    "lib", ("gzip", "cramjam-gzip", "cramjam-igzip", "isal"), ids=lambda val: val
)
@pytest.mark.parametrize("file", FILES, ids=lambda val: val.name)
def test_gzip(benchmark, file, lib):
    from isal import igzip

    data = file.read_bytes()
    if lib == "cramjam-gzip":
        benchmark(
            round_trip,
            compress=cramjam.gzip.compress,
            decompress=cramjam.gzip.decompress,
            data=data,
            level=3,
        )
    elif lib == "cramjam-isal":
        benchmark(
            round_trip,
            compress=cramjam.experimental.igzip.compress,
            decompress=cramjam.experimental.igzip.decompress,
            data=data,
            level=3,
        )
    elif lib == "gzip":
        benchmark(
            round_trip,
            compress=gzip.compress,
            decompress=gzip.decompress,
            data=data,
            compresslevel=3,
        )
    else:
        benchmark(
            round_trip,
            compress=igzip.compress,
            decompress=igzip.decompress,
            data=data,
            compresslevel=igzip._COMPRESS_LEVEL_BEST,  # 3
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
        if not (isinstance(f, (FiftyFourMbRandom, FiftyFourMbRepeating)))
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


@pytest.mark.parametrize(
    "use_cramjam", (True, False), ids=lambda val: "cramjam" if val else "bzip2"
)
@pytest.mark.parametrize("file", FILES, ids=lambda val: val.name)
def test_bzip2(benchmark, file, use_cramjam: bool):
    import bz2

    data = file.read_bytes()
    if use_cramjam:
        benchmark(
            round_trip,
            compress=cramjam.bzip2.compress,
            decompress=cramjam.bzip2.decompress,
            data=data,
        )
    else:
        benchmark(
            round_trip,
            compress=bz2.compress,
            decompress=bz2.decompress,
            data=data,
        )


@pytest.mark.parametrize(
    "use_cramjam", (True, False), ids=lambda val: "cramjam" if val else "lzma"
)
@pytest.mark.parametrize("file", FILES, ids=lambda val: val.name)
def test_lzma(benchmark, file, use_cramjam: bool):
    import lzma

    data = file.read_bytes()
    if use_cramjam:
        benchmark(
            round_trip,
            compress=cramjam.experimental.lzma.compress,
            decompress=cramjam.experimental.lzma.decompress,
            data=data,
        )
    else:
        benchmark(
            round_trip,
            compress=lzma.compress,
            decompress=lzma.decompress,
            data=data,
        )
