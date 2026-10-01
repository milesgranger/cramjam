# cramjam

---

[![CI](https://github.com/milesgranger/cramjam/actions/workflows/CI.yml/badge.svg)](https://github.com/milesgranger/cramjam/actions/workflows/CI.yml)
[![Downloads](https://pepy.tech/badge/cramjam/month)](https://pepy.tech/project/cramjam)
[![PyPI](https://img.shields.io/pypi/v/cramjam.svg)](https://pypi.org/project/cramjam)
[![Conda Version](https://img.shields.io/conda/vn/conda-forge/cramjam.svg)](https://anaconda.org/conda-forge/cramjam)
[![NPM Version](https://img.shields.io/npm/v/cramjam)](https://www.npmjs.com/package/cramjam)

[API Documentation](https://milesgranger.github.io/cramjam/cramjam.html)

### Install (Python)

```commandline
pip install --upgrade cramjam  # Requires no Python or system dependencies!
```

#### Pure-Rust backend

By default zstd, lz4, bzip2, xz and deflate/gzip/zlib use their C libraries.
To use [libcramjam](https://github.com/cramjam/libcramjam)'s pure-Rust
implementations instead (same API, no C), install the add-on and select it
before `cramjam` is imported:

```commandline
pip install --upgrade "cramjam[pure-rust]"
export CRAMJAM_BACKEND=pure-rust  # `cramjam.backend` reports which is active
```

Output is interchangeable between backends, but the compressed bytes may differ.

## Development

Install the locked development dependencies, build the extension, and run the
test suite:

```bash
make dev-setup
make test
```

Rust checks can be run separately with `cargo test`.

### Install (JavaScript / TypeScript)

```commandline
npm install cramjam
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

- [x] Snappy&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;`cramjam.snappy`
- [x] Brotli&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;`cramjam.brotli`
- [x] Bzip2&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;`cramjam.bzip2`
- [x] Lz4&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;`cramjam.lz4`
- [x] Gzip&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;`cramjam.gzip`
- [x] Zlib&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;`cramjam.zlib`
- [x] Deflate&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;`cramjam.deflate`
- [x] ZSTD&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;`cramjam.zstd`
- [x] XZ / LZMA&nbsp;&nbsp;`cramjam.xz`

Experimental (Requires build from source enabling each feature):

- [x] Blosc2&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;`cramjam.experimental.blosc2`
- [x] ISA-L backend _(only on 64-bit targets)_
  - [x] igzip&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;`cramjam.experimental.igzip`
  - [x] ideflate&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;`cramjam.experimental.ideflate`
  - [x] izlib&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;`cramjam.experimental.izlib`

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

[TypeScript](./cramjam-js/README.md):

```typescript
import {Compress, Decompress} from 'cramjam';

const decoder = new TextDecoder();
const encoder = new TextEncoder();

const str = 'hello, world';
const encoded = encoder.encode(str);

const compressed = Compress.brotli(encoded);
const decompressed = Decompress.brotli(compressed);

const decoded = decoder.decode(decompressed);
```
