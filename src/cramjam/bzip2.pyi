from typing import Optional
from cramjam import Buffer, BufferProtocol


def decompress(data: BufferProtocol, output_len: Optional[int] = None) -> Buffer:
    """
    bzip2 decompression.

    Example
    -------
    ```python
    >>> cramjam.bzip2.decompress(compressed_bytes, output_len=Optional[int])
    ```
    """
    ...


def compress(
    data: BufferProtocol, level: Optional[int] = None, output_len: Optional[int] = None
) -> Buffer:
    """
    bzip2 compression.

    Example
    -------
    ```python
    >>> cramjam.bzip2.compress(b'some bytes here', level=6, output_len=Option[int])  # level defaults to 6
    ```
    """
    ...


def compress_into(
    input: BufferProtocol, output: BufferProtocol, level: Optional[int] = None
) -> int:
    """
    Compress directly into an output buffer
    """
    ...


def decompress_into(input: BufferProtocol, output: BufferProtocol) -> int:
    """
    Decompress directly into an output buffer
    """
    ...


class Compressor:
    def __init__(self, level: Optional[int] = None) -> None: ...
    def compress(self, input: bytes) -> int: ...
    def flush(self) -> Buffer: ...
    def finish(self) -> Buffer: ...


class Decompressor:
    def decompress(self, data: bytes) -> Buffer: ...
