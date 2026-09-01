"""Compressor.flush() must make everything written so far available to a
streaming decoder (sync-flush semantics), and the xz options the API exposes
(RAW format, filter chains, SHA-256 check) must actually work."""
import bz2
import lzma
import zlib

import pytest
import cramjam


def sample(n: int, seed: int = 1) -> bytes:
    words = [b"alpha ", b"beta ", b"gamma ", b"delta ", b"omega "]
    out = bytearray()
    x = seed * 2654435761 | 1
    while len(out) < n:
        x ^= (x << 13) & 0xFFFFFFFF
        x ^= x >> 17
        x ^= (x << 5) & 0xFFFFFFFF
        out += bytes([(x >> 24) & 0xFF]) if x % 7 == 0 else words[x % 5]
    return bytes(out[:n])


def _decode_prefix(decoder, chunk: bytes) -> bytes:
    try:
        return decoder(chunk)
    except (EOFError, ValueError, lzma.LZMAError, zlib.error, OSError):
        return b""


STREAMING_DECODERS = {
    "gzip": lambda: zlib.decompressobj(wbits=31).decompress,
    "zlib": lambda: zlib.decompressobj(wbits=15).decompress,
    "deflate": lambda: zlib.decompressobj(wbits=-15).decompress,
    "xz": lambda: lzma.LZMADecompressor().decompress,
    "bzip2": lambda: bz2.BZ2Decompressor().decompress,
}
try:
    import lz4.frame

    STREAMING_DECODERS["lz4"] = lambda: lz4.frame.LZ4FrameDecompressor().decompress
except ImportError:  # pragma: no cover
    pass


@pytest.mark.parametrize("variant", ("gzip", "zlib", "deflate", "xz", "bzip2", "lz4", "zstd"))
def test_compressor_flush_makes_data_available(variant):
    mod = getattr(cramjam, variant)
    first, second = sample(300_000, 1), sample(50_000, 2)

    compressor = mod.Compressor()
    compressor.compress(first)
    flushed = bytes(compressor.flush())
    # A flush after real input must produce output, not defer to finish().
    assert len(flushed) > 8

    if variant in STREAMING_DECODERS:
        got = _decode_prefix(STREAMING_DECODERS[variant](), flushed)
        if variant == "bzip2":
            # libbzip2's BZ_FLUSH emits whole bytes only (blocks are not
            # byte-aligned), so the block completes with the next bytes.
            assert first.startswith(got)
        else:
            assert got == first

    compressor.compress(second)
    rest = bytes(compressor.flush()) + bytes(compressor.finish())
    assert bytes(mod.decompress(flushed + rest)) == first + second
    if variant in STREAMING_DECODERS:
        dec = STREAMING_DECODERS[variant]()
        assert dec(flushed) + dec(rest) == first + second


def test_compressor_flush_without_input_is_harmless():
    for variant in ("gzip", "zlib", "deflate", "xz", "bzip2", "lz4", "zstd"):
        mod = getattr(cramjam, variant)
        compressor = mod.Compressor()
        out = bytes(compressor.flush())
        compressor.compress(b"payload")
        out += bytes(compressor.flush()) + bytes(compressor.flush()) + bytes(compressor.finish())
        assert bytes(mod.decompress(out)) == b"payload", variant


# ---------------------------------------------------------------------------
# xz options
# ---------------------------------------------------------------------------

def _chain(*filters):
    chain = cramjam.xz.FilterChain()
    for f in filters:
        chain.append_filter(cramjam.xz.FilterChainItem(f))
    return chain


def test_xz_raw_format_lzma2():
    data = sample(120_000)
    raw = bytes(cramjam.xz.compress(data, format=cramjam.xz.Format.RAW))
    assert lzma.decompress(raw, format=lzma.FORMAT_RAW, filters=[{"id": lzma.FILTER_LZMA2}]) == data


def test_xz_raw_format_lzma1():
    data = sample(120_000)
    raw = bytes(cramjam.xz.compress(data, format=cramjam.xz.Format.RAW, filters=_chain(cramjam.xz.Filter.Lzma1)))
    assert lzma.decompress(raw, format=lzma.FORMAT_RAW, filters=[{"id": lzma.FILTER_LZMA1}]) == data


@pytest.mark.parametrize(
    "bcj,pyfilter",
    [
        (cramjam.xz.Filter.X86, lzma.FILTER_X86),
        (cramjam.xz.Filter.Arm, lzma.FILTER_ARM),
        (cramjam.xz.Filter.ArmThumb, lzma.FILTER_ARMTHUMB),
        (cramjam.xz.Filter.Sparc, lzma.FILTER_SPARC),
        (cramjam.xz.Filter.Ia64, lzma.FILTER_IA64),
    ],
)
def test_xz_bcj_filter_chains(bcj, pyfilter):
    # Byte patterns that the branch converters actually rewrite (E8/E9 calls).
    data = bytes([0xE8, 0x10, 0x00, 0x00, 0x00, 0x90, 0xE9, 0x34, 0x12, 0x00, 0x00]) * 20_000 + sample(50_000)
    out = bytes(cramjam.xz.compress(data, filters=_chain(bcj, cramjam.xz.Filter.Lzma2)))
    assert lzma.decompress(out) == data  # liblzma applies the filter flags from the block header
    assert bytes(cramjam.xz.decompress(out)) == data
    # And liblzma-encoded input with the same chain decodes with ours. Some
    # stdlib lzma builds (notably PyPy's cffi `_lzma`) can *decode* a BCJ
    # stream but can't set up a BCJ filter chain for *encoding* — they raise
    # LZMA_PROG_ERROR ("Internal error"). The forward direction above already
    # validated our encoder against liblzma; skip the reverse check where the
    # interpreter's lzma can't produce the reference stream.
    try:
        theirs = lzma.compress(data, filters=[{"id": pyfilter}, {"id": lzma.FILTER_LZMA2, "preset": 6}])
    except lzma.LZMAError as e:
        pytest.skip(f"stdlib lzma cannot encode a BCJ filter chain here: {e}")
    assert bytes(cramjam.xz.decompress(theirs)) == data


def test_xz_sha256_check():
    data = sample(200_000)
    out = bytes(cramjam.xz.compress(data, check=cramjam.xz.Check.Sha256))
    assert lzma.decompress(out) == data
    assert bytes(cramjam.xz.decompress(out)) == data
    theirs = lzma.compress(data, check=lzma.CHECK_SHA256)
    assert bytes(cramjam.xz.decompress(theirs)) == data
    # Corruption in the payload is caught by the check.
    bad = bytearray(out)
    bad[len(bad) // 2] ^= 0x55
    with pytest.raises(cramjam.DecompressionError):
        cramjam.xz.decompress(bytes(bad))
