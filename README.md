# pyrus-cramjam

[![Code Style](https://img.shields.io/badge/code%20style-black-000000.svg)](https://github.com/python/black)
[![CI](https://github.com/milesgranger/pyrus-cramjam/workflows/MasterCI/badge.svg?branch=master)](https://github.com/milesgranger/pyrus-cramjam/actions?query=branch=master)

[API Documentation](https://docs.rs/cramjam)

### Install
```commandline
pip install --upgrade cramjam  # Requires no Python or system dependencies!
```

---

Extremely thin Python bindings to de/compression algorithms in Rust.
Allows for using algorithms such as Snappy, without any system dependencies.

This is handy when being used in environments like AWS Lambda, where installing
packages like `python-snappy` becomes difficult because of system level dependencies.

---

##### Benchmarks

Some basic benchmarks are available [in the benchmarks directory](./benchmarks/README.md)

---

Available algorithms:

- [X] Snappy
- [X] Brotli
- [X] Lz4
- [X] Gzip
- [X] Deflate
- [X] ZSTD

All available for use as:

```python
>>> import cramjam
>>> compessed = cramjam.snappy.compress(b"bytes here")
>>> cramjam.snappy.decompress(compressed)
b"bytes here"
```

Where the API is `cramjam.<compression-variant>.compress/decompress` and accepts
both `bytes` and `bytearray` objects.

**Special note!**  
If you know the length of the de/compress output, you
can provide `output_len=<<some int>>` to any `de/compress`
to get ~1.5-3x performance increase as this allows single 
buffer allocation. 

For `snappy` with `bytearray`s, it's only a mild improvement
as we currently are able to estimate the buffer size and can
resize the resulting `bytearray` to the correct size.
