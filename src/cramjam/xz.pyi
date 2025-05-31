from enum import Enum
from typing import Optional
from cramjam import BufferProtocol, Buffer


class Filter(Enum):
    """Available Filter IDs"""

    Arm: Filter
    ArmThumb: Filter
    Ia64: Filter
    Lzma1: Filter
    Lzma2: Filter
    PowerPC: Filter
    Sparc: Filter
    X86: Filter


class MatchFinder(Enum):
    """MatchFinder, used with Options.mf attribute"""

    HashChain3: MatchFinder
    HashChain4: MatchFinder
    BinaryTree2: MatchFinder
    BinaryTree3: MatchFinder
    BinaryTree4: MatchFinder


class Mode(Enum):
    """MatchFinder, used with Options.mode attribute"""

    Fast: Mode
    Normal: Mode


class Options:
    def __init__(self) -> None: ...
    def set_preset(self, preset: int) -> Options: ...
    def set_dict_size(self, dict_size: int) -> Options: ...
    def set_lc(self, lc: int) -> Options: ...
    def set_lp(self, lp: int) -> Options: ...
    def set_pb(self, pb: int) -> Options: ...
    def set_mode(self, mode: Mode) -> Options: ...
    def set_nice_len(self, nice_len: int) -> Options: ...
    def set_mf(self, mf: MatchFinder) -> Options: ...
    def set_depth(self, depth: int) -> Options: ...


class Format(Enum):
    """Possible formats"""

    AUTO: Format
    XZ: Format
    ALONE: Format
    RAW: Format


class Check(Enum):
    """Possible check configurations"""

    Crc64: Check
    Crc32: Check
    Sha256: Check
    NONE: Check


class FilterChainItem:
    def __init__(self, filter: Filter, options: Optional[Options] = None) -> None: ...


class FilterChain:
    """
    FilterChain, similar to the default Python XZ filter chain which is a list of dicts
    """

    ...

    def __init__(self) -> None: ...
    def append_filter(self, filter_chain_item: FilterChainItem) -> None: ...


def compress(
    data: BufferProtocol,
    preset: Optional[int] = None,
    format: Optional[Format] = None,
    check: Optional[Check] = None,
    filters: Optional[FilterChain] = None,
    options: Optional[Options] = None,
    output_len: Optional[int] = None,
) -> Buffer:
    """
    LZMA compression.

    Example
    -------
    ```python
    >>> _ = cramjam.xz.compress(b'some bytes here')
    >>> # Defaults to XZ format, you can use the deprecated LZMA format like this:
    >>> _ = cramjam.xz.compress(b'some bytes here', format=cramjam.xz.Format.ALONE)
    ```
    """
    ...


def compress_into(
    input: BufferProtocol,
    output: BufferProtocol,
    preset: Optional[int] = None,
    format: Optional[Format] = None,
    check: Optional[Check] = None,
    filters: Optional[FilterChain] = None,
    options: Optional[Options] = None,
) -> Buffer:
    """
    LZMA compression.

    Compress directly into an output buffer
    """
    ...


def decompress_into(data: BufferProtocol, output: BufferProtocol) -> int:
    """
    Decompress directly into an output buffer
    """
    ...


def decompress(data: BufferProtocol, output_len: Optional[int] = None) -> Buffer:
    """
    LZMA decompression.

    Example
    -------

    ```python
    >>> # bytes or bytearray; bytearray is faster
    >>> cramjam.xz.decompress(compressed_bytes, output_len=Optional[None])
    ```
    """
