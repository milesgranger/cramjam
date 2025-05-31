from typing import Optional
from cramjam import Buffer, BufferProtocol


def decompress(data: BufferProtocol, output_len: Optional[int] = None) -> Buffer:
    """
    Brotli decompression.

    Python Example
    --------------
    >>> cramjam.brotli.decompress(compressed_bytes, output_len=Optional[int])
    """
    ...


def compress(
    data: BufferProtocol,
    level: Optional[int] = None,
    output_len: Optional[int] = None,
) -> Buffer:
    """
    Brotli compression.

    Python Example
    --------------
    >>> cramjam.brotli.compress(b'some bytes here', level=9, output_len=Optional[int])  # level defaults to 11
    """
    ...


def compress_into(
    input: BufferProtocol,
    output: BufferProtocol,
    level: Optional[int] = None,
) -> int:
    """
    Compress directly into an output buffer.
    """
    ...


def decompress_into(
    input: BufferProtocol,
    output: BufferProtocol,
) -> int:
    """
    Decompress directly into an output buffer.
    """
    ...


class Compressor:
    """
    Brotli Compressor object for streaming compression.
    """

    def __init__(self, level: Optional[int] = None) -> None:
        """
        Initialize a new `Compressor` instance.
        """
        ...

    def compress(self, input: BufferProtocol) -> int:
        """
        Compress input into the current compressor's stream.

        Returns the number of bytes written to the stream.
        """
        ...

    def flush(self) -> Buffer:
        """
        Flush and return current compressed stream.
        """
        ...

    def finish(self) -> Buffer:
        """
        Consume the current compressor state and return the compressed stream.

        NB: The compressor will not be usable after this method is called.
        """
        ...


class Decompressor:
    """
    Brotli Decompressor object for streaming decompression.
    """

    def decompress(self, input: BufferProtocol) -> Buffer: ...
    def flush(self) -> Buffer: ...
    def finish(self) -> Buffer: ...
