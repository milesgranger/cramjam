from typing import Optional
from cramjam import BufferProtocol, Buffer


def decompress(data: BufferProtocol, output_len: Optional[int] = ...) -> Buffer:
    """
    Snappy decompression.

    Python Example
    --------------
    ```python
    # bytes or bytearray; bytearray is faster
    cramjam.snappy.decompress(compressed_bytes, output_len=None)
    ```
    """
    ...


def compress(data: BufferProtocol, output_len: Optional[int] = ...) -> Buffer:
    """
    Snappy compression.

    Python Example
    --------------
    ```python
    _ = cramjam.snappy.compress(b'some bytes here')
    _ = cramjam.snappy.compress(bytearray(b'this avoids double allocation in rust side, and thus faster!'))
    ```
    """
    ...


def decompress_raw(data: BufferProtocol, output_len: Optional[int] = ...) -> Buffer:
    """
    Snappy decompression, raw
    This does not use the snappy 'framed' encoding of compressed bytes.

    Python Example
    --------------
    ```python
    cramjam.snappy.decompress_raw(compressed_raw_bytes)
    ```
    """
    ...


def compress_raw(data: BufferProtocol, output_len: Optional[int] = ...) -> Buffer:
    """
    Snappy compression raw.
    This does not use the snappy 'framed' encoding of compressed bytes.

    Python Example
    --------------
    ```python
    cramjam.snappy.compress_raw(b'some bytes here')
    ```
    """
    ...


def compress_into(input: BufferProtocol, output: BufferProtocol) -> int:
    """Compress directly into an output buffer"""
    ...


def decompress_into(input: BufferProtocol, output: BufferProtocol) -> int:
    """Decompress directly into an output buffer"""
    ...


def compress_raw_into(input: BufferProtocol, output: BufferProtocol) -> int:
    """Compress raw format directly into an output buffer"""
    ...


def decompress_raw_into(input: BufferProtocol, output: BufferProtocol) -> int:
    """Decompress raw format directly into an output buffer"""
    ...


def compress_raw_max_len(data: BufferProtocol) -> int:
    """
    Get the expected max compressed length for snappy raw compression; this is the size
    of buffer that should be passed to `compress_raw_into`
    """
    ...


def decompress_raw_len(data: BufferProtocol) -> int:
    """
    Get the decompressed length for the given data. This is the size of buffer
    that should be passed to `decompress_raw_into`
    """
    ...


class Compressor:
    """Snappy Compressor object for streaming compression"""

    def __init__(self) -> None:
        """Initialize a new `Compressor` instance."""
        ...

    def compress(self, input: bytes) -> int:
        """Compress input into the current compressor's stream."""
        ...

    def flush(self) -> Buffer:
        """Flush and return current compressed stream"""
        ...

    def finish(self) -> Buffer:
        """Consume the current compressor state and return the compressed stream
        **NB** The compressor will not be usable after this method is called."""
        ...


class Decompressor:
    """Snappy streaming Decompressor (generated via make_decompressor!)."""

    ...
