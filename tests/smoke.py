# Quick wheel check where the full suite is impractical (see [tool.cibuildwheel] overrides).
try:
    import cramjam
except ImportError:  # a lone cramjam-pure-rust wheel
    import cramjam_pure_rust as cramjam

for codec in (cramjam.snappy, cramjam.zstd):
    assert bytes(codec.decompress(codec.compress(b"cramjam"))) == b"cramjam"
print(cramjam.__name__, cramjam.backend, "ok")
