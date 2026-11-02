# `cramjam` docs

GitHub: [cramjam](https://www.github.com/milesgranger/cramjam).

## What?

`cramjam` is a de/compression library containing a multitude of compression codecs
in a single, easy to install, and similar as possible API.

---

## Supported Codecs

* lz4
* gzip
* brotli
* snappy
* xz/lzma
* deflate
* zstd
* bzip
* blosc2 (experimental support)

---

## Languages

- [x] [CLI](https://github.com/milesgranger/cramjam/tree/master/cramjam-cli)  
    - `pip install cramjam-cli`
- [x] [Python](https://github.com/milesgranger/cramjam/tree/master/cramjam-python)  
    - `pip install cramjam`
    - `conda install -c conda-forge cramjam`
- [x] [Rust](https://github.com/milesgranger/cramjam/tree/master/libcramjam)  
    - `cargo add libcramjam`
- [x] [C](https://github.com/milesgranger/cramjam/tree/master/libcramjam)  
    - `cargo add libcramjam --features capi`
- [ ] JavaScript (TODO)

## Bounded decompression

Python's standard one-shot `decompress` functions, `snappy.decompress_raw` and
`lz4.decompress_block` accept the keyword-only `max_output_size` argument:

```python
import cramjam

compressed = cramjam.zstd.compress(b"example")
try:
    decoded = cramjam.zstd.decompress(compressed, max_output_size=1024)
except cramjam.OutputLimitExceededError:
    # A subclass of DecompressionError; the output is never silently truncated.
    raise
```

The limit counts decoded bytes cumulatively across all frames decoded by a call.
Zero accepts only empty output. With no limit, or `max_output_size=None`, the
previous behavior is preserved. This option is not available on the streaming
`Decompressor` classes or experimental codecs.

### Output buffers and compatibility

Framed codecs grow the output buffer as needed, checking each write before
copying or allocating. They do not allocate the entire limit up front.
`output_len`, when also supplied, is an initial-capacity hint clamped to the
limit; the result contains only decoded bytes.

Without a limit, `output_len` deliberately keeps its historical behavior:
it creates a zero-filled buffer of that length. A shorter decoded result can
therefore be padded with zeros. This difference is intentional to preserve
compatibility for existing callers. For example, decoding `b"abc"` with
`output_len=16` returns 16 bytes without a limit and 3 bytes with a limit of 16.

Raw Snappy and LZ4 blocks check the declared output size before allocating their
destination. Raw LZ4's `output_len` describes a headerless block's output bound,
so it must not exceed the limit even if the actual output might be smaller.
Allocation failures detected while creating or growing these bounded output
buffers raise `MemoryError`, rather than `DecompressionError`.

### Backend support and workspace

| Operation | Default backend | Pure-Rust backend |
| --- | --- | --- |
| Standard one-shot `decompress` | All standard codecs | Snappy, Brotli, Bzip2 |
| `snappy.decompress_raw` | Supported | Supported |
| `lz4.decompress_block` | Supported | Not supported |

Unsupported pure-Rust operations raise `NotImplementedError` when a limit is
requested; they continue to work without a limit. These restrictions reflect
the decoding paths in libcramjam 0.9.1:

- Pure-Rust DEFLATE (including gzip/zlib) and LZ4 flush after decoding whole
  blocks. The flush threshold alone does not bound a single block's allocation.
- Pure-Rust Zstd can buffer a full frame when its window exceeds 8 MiB.
- Pure-Rust XZ buffers the full output before writing to the sink.

The cap applies to output storage, not compressed-input storage, decoder
workspace, total process memory or CPU time. Pure-Rust Bzip2 flushes after each
block, but its 900 KB BWT block limit applies **before** RLE expansion. A block
can expand to 46,620,000 decoded bytes (about 44.5 MiB) before reaching the bounded
sink, in addition to other workspace and allocation overhead. Other codecs also
use scratch buffers or dictionary/window memory independently of the output cap.
