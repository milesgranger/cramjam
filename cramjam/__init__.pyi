from _typeshed import ReadableBuffer, WriteableBuffer
from typing import Any, Union
from . import lz4, snappy, deflate, gzip, zstd, xz, zlib, brotli, bzip2, experimental


__version__: str

# TODO: all supported python versions can just use types.Buffer
BufferProtocol = Any


class CompressionError(Exception):
    """
    Cramjam specific exception representing a failed compression operation.
    """

    ...


class DecompressionError(Exception):
    """
    Cramjam specific exception representing a failed decompression operation.
    """

    ...


class Buffer(Union[ReadableBuffer, WriteableBuffer]):
    def __init__(
        self, data: BufferProtocol | None = None, copy: bool | None = True
    ) -> None:
        """
        Initialize the buffer, with any initial `data`, which is required to implement the Buffer Protocol
        Optionally, don't make a copy of the data.

        Parameters
        ----------
        data: anything implementing the buffer protocol
        copy: bool (default True)
            Make a copy of the provided data.

        Returns
        -------
        Buffer
        """
        ...

    def get_view_reference(self) -> None | Any:
        """
        Get the PyObject this Buffer is referencing as its view,
        returns None if this Buffer owns its data.
        """
        ...

    def get_view_reference_count(self) -> None | int:
        """
        Get the PyObject reference count this Buffer is referencing as its view,
        returns None if this Buffer owns its data.
        """
        ...

    def len(self) -> int:
        """
        Length of the underlying buffer
        """
        ...

    def write(self, input: BufferProtocol) -> int:
        """
        Write some bytes to the buffer, where input data can be
        anything implementing the Buffer Protocol.
        """
        ...

    def read(self, n_bytes: int | None = -1) -> bytes:
        """
        Read from the buffer in its current position,
        returns bytes; optionally specify number of bytes to read.
        """
        ...

    def readinto(self, output: BufferProtocol) -> int:
        """
        Read from the buffer in its current position, into an object implementing the Buffer Protocol.
        """
        ...

    def seek(self, position: int, whence: int | None = 0) -> int:
        """
        Seek to a position within the buffer. whence follows the same values as IOBase.seek where:
        ```bash
        0: from start of the stream
        1: from current stream position
        2: from end of the stream
        ```
        """
        ...

    def seekable(self) -> bool:
        """
        Whether the buffer is seekable; here just for compatibility, it always returns True.
        """
        ...

    def tell(self) -> int:
        """
        Give the current position of the buffer.
        """
        ...

    def set_len(self, size: int) -> None:
        """
        Set the length of the buffer. If less than current length, it will truncate to the size given;
        otherwise will be null byte filled to the size.
        """
        ...

    def truncate(self) -> None:
        """
        Truncate the buffer
        """
        ...

    def __len__(self) -> int: ...
    def __contains__(self, x: BufferProtocol) -> bool: ...
    def __repr__(self) -> str: ...
    def __eq__(self, other) -> bool: ...
    def __bool__(self) -> bool: ...


class File[BufferProtocol]:
    def __init__(
        self,
        path: str,
        read: bool | None = None,
        write: bool | None = None,
        truncate: bool | None = None,
        append: bool | None = None,
    ) -> None:
        """
        File-like object owned on Rust side

        ### Example

        ```python
        from cramjam import File
        file = File("/tmp/file.txt", read=True, write=True, truncate=True)
        file.write(b"bytes")
        file.seek(2)
        file.read()
        b'tes'
        ```
        """
        ...

    def write(self, input: BufferProtocol) -> int:
        """
        Write some bytes to the file, where input data can be anything implementing the Buffer Protocol
        """
        ...

    def read(self, n_bytes: int | None = None) -> bytes:
        """
        Read from the file in its current position, returns `bytes`; optionally specify number of
        bytes to read.
        """
        ...

    def readinto(self, output: BufferProtocol) -> int:
        """
        Read from the file in its current position, into a object implementing the Buffer Protocol.
        """
        ...

    def seek(self, position: int, whence: int | None = 0) -> int:
        """
        Seek to a position within the file. `whence` follows the same values as [IOBase.seek](https://docs.python.org/3/library/io.html#io.IOBase.seek)
        where:
        ```bash
        0: from start of the stream
        1: from current stream position
        2: from end of the stream
        ```
        """
        ...

    def seekable(self) -> bool:
        """
        Whether the file is seekable; here just for compatibility, it always returns True.
        """
        ...

    def tell(self) -> int:
        """
        Give the current position of the file.
        """
        ...

    def set_len(self, size: int) -> None:
        """
        Set the length of the file. If less than current length, it will truncate to the size given;
        otherwise will be null byte filled to the size.
        """
        ...

    def truncate(self) -> None:
        """
        Truncate the file.
        """
        ...

    def len(self) -> int:
        """
        Length of the file in bytes
        """
        ...

    def __repr__(self) -> str: ...
    def __bool__(self) -> bool: ...
    def __len__(self) -> int: ...
