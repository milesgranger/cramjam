# cramjam-pure-rust

The pure-Rust backend for [cramjam](https://pypi.org/project/cramjam/): the same
API, with zstd, lz4, bzip2, xz and deflate/gzip/zlib from libcramjam's pure-Rust
implementations instead of the C libraries.

```bash
pip install cramjam[pure-rust]
export CRAMJAM_BACKEND=pure-rust
```

`import cramjam` then uses this backend (`cramjam.backend == "pure-rust"`).
It can also be imported directly as `cramjam_pure_rust`.
