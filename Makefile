BASE_BENCH_CMD = python -m pytest -v --benchmark-sort name --benchmark-only benchmarks/ -k

test:
	python -m pytest -v --ignore benchmarks

bench:
	python -m pytest -v --benchmark-only --benchmark-sort name benchmarks/

bench-snappy:
	$(BASE_BENCH_CMD) snappy

bench-lz4:
	$(BASE_BENCH_CMD) lz4

bench-gzip:
	$(BASE_BENCH_CMD) gzip

bench-brotli:
	$(BASE_BENCH_CMD) brotli

bench-zstd:
	$(BASE_BENCH_CMD) zstd

dev-install:
	rm -rf ./wheels
	maturin build --release --out wheels --interpreter $(shell which python)
	pip uninstall cramjam -y
	pip install --no-index wheels/*
