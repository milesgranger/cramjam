import io
import gzip
import pytest
import cramjam


def generate_data(n_mb: int = 1) -> bytes:
    data = io.BytesIO()
    data.seek((n_mb * 1024 * 1024) - 1)
    data.write(b"\0")
    data.seek(0)
    return data.read()


def round_trip(compress, decompress, data, **kwargs):
    return decompress(compress(data, **kwargs))


@pytest.mark.parametrize("n_mb", (1, 10, 100))
@pytest.mark.parametrize(
    "use_cramjam", (True, False), ids=lambda val: "cramjam" if val else "snappy"
)
def test_snappy_raw(benchmark, n_mb: int, use_cramjam: bool):
    """
    Uses the non-framed format for snappy compression
    """
    import snappy
    data = generate_data(n_mb)
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


@pytest.mark.parametrize("n_mb", (1, 10, 100))
@pytest.mark.parametrize(
    "use_cramjam", (True, False), ids=lambda val: "cramjam" if val else "gzip"
)
def test_gzip(benchmark, n_mb: int, use_cramjam: bool):
    data = generate_data(n_mb)
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

@pytest.mark.parametrize("n_mb", (1, 10, 100))
@pytest.mark.parametrize(
    "use_cramjam", (True, False), ids=lambda val: "cramjam" if val else "python-lz4"
)
def test_lz4(benchmark, n_mb: int, use_cramjam: bool):
    from lz4 import frame

    data = generate_data(n_mb)
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

@pytest.mark.parametrize("n_mb", (1, 5, 10))
@pytest.mark.parametrize(
    "use_cramjam", (True, False), ids=lambda val: "cramjam" if val else "brotli"
)
def test_brotli(benchmark, n_mb: int, use_cramjam: bool):
    import brotli

    data = generate_data(n_mb)
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
