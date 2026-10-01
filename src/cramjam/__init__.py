import os

# Backend is picked once, at import. `pip install cramjam[pure-rust]` then set
# CRAMJAM_BACKEND=pure-rust to use libcramjam's pure-Rust codecs instead of C.
backend = os.environ.get("CRAMJAM_BACKEND", "c")
if backend == "pure-rust":
    try:
        from cramjam_pure_rust.cramjam import *
    except ImportError as e:
        raise ImportError(
            "CRAMJAM_BACKEND=pure-rust but cramjam-pure-rust isn't installed; `pip install cramjam[pure-rust]`"
        ) from e
elif backend == "c":
    from .cramjam import *
else:
    raise ValueError(f"CRAMJAM_BACKEND must be 'c' or 'pure-rust', got {backend!r}")
