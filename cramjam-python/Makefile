BASE_BENCH_CMD = python -m pytest -v --benchmark-sort name --benchmark-only benchmarks/ -k

test:
	python -m pytest tests -n auto -v --ignore benchmarks

test-bench:
	python -m pytest -v --benchmark-disable benchmarks/

bench:
	python -m pytest -v --benchmark-only --benchmark-sort name benchmarks/

bench-snappy-framed:
	$(BASE_BENCH_CMD) test_snappy_framed

bench-snappy-raw:
	$(BASE_BENCH_CMD) test_snappy_raw

bench-snappy-compress-into:
	$(BASE_BENCH_CMD) snappy_de_compress_into

bench-lz4:
	$(BASE_BENCH_CMD) lz4

bench-lz4-block:
	$(BASE_BENCH_CMD) lz4_block

bench-gzip:
	$(BASE_BENCH_CMD) gzip

bench-brotli:
	$(BASE_BENCH_CMD) brotli

bench-bzip2:
	$(BASE_BENCH_CMD) bzip2

bench-zstd:
	$(BASE_BENCH_CMD) zstd

bench-lzma:
	$(BASE_BENCH_CMD) lzma

dev-install:
	rm -rf ./dist
	maturin build --release --out dist --interpreter $(shell which python)
	pip uninstall cramjam -y
	pip install cramjam --no-index --find-links dist/

pypy-build:
	maturin build -i $(shell which pypy) --release --out dist
