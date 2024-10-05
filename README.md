# cramjam

[![Code Style](https://img.shields.io/badge/code%20style-black-000000.svg)](https://github.com/python/black)
[![CI](https://github.com/milesgranger/cramjam/actions/workflows/CI.yml/badge.svg)](https://github.com/milesgranger/cramjam/actions/workflows/CI.yml)
[![PyPI](https://img.shields.io/pypi/v/cramjam.svg)](https://pypi.org/project/cramjam)
[![Anaconda-Server Badge](https://anaconda.org/conda-forge/cramjam/badges/version.svg)](https://anaconda.org/conda-forge/cramjam)
[![Downloads](https://pepy.tech/badge/cramjam/month)](https://pepy.tech/project/cramjam)

[API Documentation](https://docs.rs/cramjam)

### Install
```commandline
pip install --upgrade cramjam  # Requires no Python or system dependencies!
```

### CLI

A CLI interface is available as [`cramjam-cli`](https://github.com/cramjam/cramjam-cli)

### libcramjam

A Rust crate and C friendly library available at [libcramjam](https://github.com/cramjam/libcramjam)

---

Extremely thin and easy-to-install Python bindings to de/compression algorithms in Rust.
Allows for using algorithms such as Snappy, without any system or other python dependencies.

---

##### Benchmarks

Some basic benchmarks are available [in the benchmarks directory](./benchmarks/README.md)

---

Available algorithms:

- [X] Snappy&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;`cramjam.snappy`
- [X] Brotli&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;`cramjam.brotli`
- [X] Bzip2&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;`cramjam.bzip2`
- [X] Lz4&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;`cramjam.lz4`
- [X] Gzip&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;`cramjam.gzip`
- [X] Zlib&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;`cramjam.zlib`
- [X] Deflate&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;`cramjam.deflate`
- [X] ZSTD&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;`cramjam.zstd`
- [X] XZ / LZMA&nbsp;&nbsp;`cramjam.xz`
- [X] Blosc2&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;`cramjam.experimental.blosc2`
- [X] ISA-L backend
  - [X] igzip&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;`cramjam.experimental.igzip` (only on 64-bit targets)
  - [X] ideflate&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;`cramjam.experimental.ideflate` (only on 64-bit targets)
  - [X] izlib&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;`cramjam.experimental.izlib` (only on 64-bit targets)

All available for use as:

```python
>>> import cramjam
>>> import numpy as np
>>> compressed = cramjam.snappy.compress(b"bytes here")
>>> decompressed = cramjam.snappy.decompress(compressed)
>>> decompressed
cramjam.Buffer(len=10)  # an object which implements the buffer protocol
>>> bytes(decompressed)
b"bytes here"
>>> np.frombuffer(decompressed, dtype=np.uint8)
array([ 98, 121, 116, 101, 115,  32, 104, 101, 114, 101], dtype=uint8)
```

Where the API is `cramjam.<compression-variant>.compress/decompress` and accepts 
`bytes`/`bytearray`/`numpy.array`/`cramjam.File`/`cramjam.Buffer` / `memoryview` objects.

**de/compress_into**
Additionally, all variants support `decompress_into` and `compress_into`. 
Ex.
```python
>>> import numpy as np
>>> from cramjam import snappy, Buffer
>>>
>>> data = np.frombuffer(b'some bytes here', dtype=np.uint8)
>>> data
array([115, 111, 109, 101,  32,  98, 121, 116, 101, 115,  32, 104, 101,
       114, 101], dtype=uint8)
>>>
>>> compressed = Buffer()
>>> snappy.compress_into(data, compressed)
33  # 33 bytes written to compressed buffer
>>>
>>> compressed.tell()  # Where is the buffer position?
33  # goodie!
>>>
>>> compressed.seek(0)  # Go back to the start of the buffer so we can prepare to decompress
>>> decompressed = b'0' * len(data)  # let's write to `bytes` as output
>>> decompressed
b'000000000000000'
>>>
>>> snappy.decompress_into(compressed, decompressed)
15  # 15 bytes written to decompressed
>>> decompressed
b'some bytes here'
```
