from typing import Optional
from cramjam import Buffer, BufferProtocol


def decompress(data: BufferProtocol, output_len: Optional[int] = None) -> Buffer:
    """
    LZ4 decompression.

    Example
    -------
    ```python
    >>> # Note, output_len is currently ignored; underlying algorithm does not support reading to slice at this time
    >>> cramjam.lz4.decompress(compressed_bytes, output_len=Optional[int])
    ```
    """
    ...


def compress(
    data: BufferProtocol, level: Optional[int] = None, output_len: Optional[int] = None
) -> Buffer:
    """
    LZ4 compression.

    Example
    -------
    ```python
    >>> # Note, output_len is currently ignored; underlying algorithm does not support reading to slice at this time
    >>> cramjam.lz4.compress(b'some bytes here', output_len=Optional[int])
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


def decompress_block(data: BufferProtocol, output_len: Optional[int] = None) -> Buffer:
    """
    LZ4 _block_ decompression.

    `output_len` is optional, it's the upper bound length of decompressed data; if it's not provided,
    then it's assumed `store_size=True` was used during compression and length will then be taken
    from the header, otherwise it's assumed `store_size=False` was used and no prepended size exists in input

    Example
    -------
    ```python
    >>> cramjam.lz4.decompress_block(compressed_bytes, output_len=Optional[int])
    ```

    """
    ...


def compress_block(
    data: BufferProtocol,
    output_len: Optional[int] = None,
    mode: Optional[str] = None,
    acceleration: Optional[int] = None,
    compression: Optional[int] = None,
    store_size: Optional[bool] = None,
) -> Buffer:
    """
    LZ4 _block_ compression.

    Example
    -------
    ```python
    >>> cramjam.lz4.compress_block(
    ...     b'some bytes here',
    ...     output_len=Optional[int],
    ...     mode=Option[str],
    ...     acceleration=Option[int],
    ...     compression=Option[int],
    ...     store_size=Option[bool]
    ... )
    ```
    """
    ...


def decompress_block_into(
    input: BufferProtocol,
    output: BufferProtocol,
    output_len: Optional[int] = None,
) -> int:
    """
    LZ4 _block_ decompression into a pre-allocated buffer.

    Example
    -------
    ```python
    >>> cramjam.lz4.decompress_block_into(compressed_bytes, output_buffer)
    ```
    """
    ...


def compress_block_into(
    data: BufferProtocol,
    output: BufferProtocol,
    mode: Optional[str] = None,
    acceleration: Optional[int] = None,
    store_size: Optional[bool] = None,
) -> int:
    """
    LZ4 _block_ compression into pre-allocated buffer.

    Example
    -------
    ```python
    >>> cramjam.lz4.compress_block_into(
    ...     b'some bytes here',
    ...     output=output_buffer,
    ...     mode=Option[str],
    ...     acceleration=Option[int],
    ...     compression=Option[int],
    ...     store_size=Option[bool]
    ... )
    ```

    """
    ...


def compress_block_bound(src: BufferProtocol) -> int:
    """
    Determine the size of a buffer which is guaranteed to hold the result of block compression, will error if
    data is too long to be compressed by LZ4.

    Example
    -------
    ```python
    >>> cramjam.lz4.compress_block_bound(b'some bytes here')
    ```
    """
    ...


class Compressor:
    def __init__(
        self,
        level: Optional[int] = None,
        content_checksum: Optional[bool] = None,
        block_linked: Optional[bool] = None,
    ) -> None:
        """
        Initialize a new `Compressor` instance.
        """
        ...

    def compress(self, input: bytes) -> int: ...
    def flush(self) -> Buffer: ...
    def finish(self) -> Buffer: ...


class Decompressor:
    def __init__(self, *args, **kwargs) -> None: ...
    def decompress(self, data: bytes) -> Buffer: ...
