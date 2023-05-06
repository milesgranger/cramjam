# cramjam-cli

[![CI](https://github.com/milesgranger/pyrus-cramjam/workflows/CI/badge.svg?branch=master)](https://github.com/milesgranger/pyrus-cramjam/actions?query=branch=master)
[![PyPI](https://img.shields.io/pypi/v/cramjam-cli.svg)](https://pypi.org/project/cramjam-cli)
[![Anaconda-Server Badge](https://anaconda.org/conda-forge/cramjam-cli/badges/version.svg)](https://anaconda.org/conda-forge/cramjam-cli)
[![Downloads](https://pepy.tech/badge/cramjam-cli/month)](https://pepy.tech/project/cramjam-cli)


### Install  (only via pip or conda for now)
```commandline
pip install --upgrade cramjam-cli  # Requires no Python or system dependencies!
```

---

Simple CLI to a variety of compression algorithms

---

Available algorithms:

- [X] snappy
- [X] brotli
- [X] bzip2
- [X] lz4
- [X] gzip
- [X] deflate
- [X] zstd

All available for use as:

```bash
cramjam-cli snappy compress --input myfile.txt --output myfile.txt.snappy
cramjam-cli lz4 compress --input myfile.txt  # omitting --output will write to stdout
cat myfile.txt | cramjam-cli zstd compress --output myfile.txt.zstd  # omitting --input will read from stdin
```
