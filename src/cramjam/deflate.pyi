from typing import Optional
from cramjam import Buffer, BufferProtocol


def decompress(data: BufferProtocol, output_len: Optional[int] = None) -> Buffer:
    """
    Deflate decompression.

    Python Example
    --------------
    >>> cramjam.deflate.decompress(compressed_bytes, output_len=Optional[int])
    """
    ...


def compress(
    data: BufferProtocol,
    level: Optional[int] = None,
    output_len: Optional[int] = None,
) -> Buffer:
    """
    Deflate compression.

    Python Example
    --------------
    >>> cramjam.deflate.compress(b'some bytes here', level=5, output_len=Optional[int])  # level defaults to 6
    """
    ...


def compress_into(
    input: BufferProtocol,
    output: BufferProtocol,
    level: Optional[int] = None,
) -> int:
    """
    Compress directly into an output buffer.

    Returns the number of bytes written.
    """
    ...


def decompress_into(
    input: BufferProtocol,
    output: BufferProtocol,
) -> int:
    """
    Decompress directly into an output buffer.

    Returns the number of bytes written.
    """
    ...


class Compressor:
    """
    Deflate Compressor object for streaming compression.
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
    Deflate Decompressor object for streaming decompression.
    """

    def decompress(self, input: BufferProtocol) -> Buffer: ...
    def flush(self) -> Buffer: ...
    def finish(self) -> Buffer: ...
