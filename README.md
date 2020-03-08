# pyrus-cramjam

[![Code Style](https://img.shields.io/badge/code%20style-black-000000.svg)](https://github.com/python/black)
[![CI](https://github.com/milesgranger/pyrus-cramjam/workflows/MasterCI/badge.svg?branch=master)](https://github.com/milesgranger/pyrus-cramjam/actions?query=branch=master)


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

Available algorithms:

- [X] Snappy
- [X] Brotli
- [X] Lz4
- [X] Gzip
- [X] Deflate


All available for use as:

```python
>>> import cramjam
>>> compessed = cramjam.snappy_compress(b"bytes here")
>>> cramjam.snappy_decompress(compressed)
b"bytes here"
```

Where the API is `cramjam.<compression-variant>_compress/decompress` and only accepts
python `byte` strings
