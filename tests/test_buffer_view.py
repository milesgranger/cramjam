import gc
from concurrent.futures import ThreadPoolExecutor

import cramjam
import numpy as np
import pytest
from cramjam import Buffer


@pytest.mark.skip_pypy
@pytest.mark.parametrize("copy", (None, True, False))
def test_buffer_view(copy):
    kwargs = dict()
    if copy is not None:
        kwargs["copy"] = copy

    data = bytearray(b"bytes")
    buf = Buffer(data, **kwargs)
    buf.write(b"0")

    if copy is False:
        assert data == b"0ytes"

    # Default is to copy, None and True behavior
    else:
        assert data == b"bytes"


@pytest.mark.skip_pypy
def test_buffer_view_raises_when_writing_past_data_length_at_once():
    data = bytearray(b"bytes")
    buf = Buffer(data, copy=False)

    # Won't write pasted underlying buffer if passed data all at once
    with pytest.raises(OSError, match="Too much to write on view"):
        buf.write(b"0" * len(data) + b"0")
    assert data == b"bytes"


@pytest.mark.skip_pypy
def test_buffer_view_raises_when_writing_past_data_length_incrementally():
    data = bytearray(b"bytes")
    buf = Buffer(data, copy=False)

    # This is okay, up to length of underlying buffer
    for _ in range(len(data)):
        buf.write(b"0")

    # Whoops, one too many bytes
    with pytest.raises(OSError, match="Too much to write on view"):
        buf.write(b"0")
    assert data == b"00000"


@pytest.mark.skip_pypy
@pytest.mark.parametrize("len", range(0, 7))
def test_buffer_view_raises_when_setting_length(len):
    data = b"bytes"
    buf = Buffer(data, copy=False)

    with pytest.raises(OSError, match="Cannot set length on unowned buffer"):
        buf.set_len(len)
    assert data == b"bytes"


@pytest.mark.skip_pypy
def test_buffer_view_raises_when_truncating():
    data = b"bytes"
    buf = Buffer(data, copy=False)

    with pytest.raises(OSError, match="Cannot truncate unowned buffer"):
        buf.truncate()
    assert data == b"bytes"


@pytest.mark.skip_pypy
@pytest.mark.parametrize("whence", (0, 1, 2))
def test_buffer_view_raises_when_write_after_bad_seek(whence):
    buf = Buffer(bytearray(b"bytes"), copy=False)

    buf.seek(2, whence=0)  # Seek forward 2 from start, also okay
    buf.seek(2, whence=1)  # Seek forward 2 from current position, okay
    buf.seek(-2, whence=2)  # Seek back -2 from end, okay
    buf.seek(0)  # Set back to start

    # Seeking 10 positions from any point is not possible with len of 5
    msg = "Bad seek: cannot seek outside bounds of unowned buffer"
    with pytest.raises(OSError, match=msg):
        buf.seek(10, whence=whence)
    buf.write(b"0")


def test_buffer_view_not_supported_on_pypy(is_pypy):
    if is_pypy:
        with pytest.raises(RuntimeError, match="copy=False not supported on PyPy"):
            Buffer(b"bytes", copy=False)


@pytest.mark.skip_pypy
def test_buffer_view_cleanup():
    n_refs = 0

    def get_buffer():
        data = bytearray(b"bytes")
        buf = cramjam.Buffer(data, copy=False)

        nonlocal n_refs
        n_refs = buf.get_view_reference_count()

        return buf

    buf = get_buffer()
    gc.collect()

    ref_count = buf.get_view_reference_count()
    assert ref_count is not None
    assert 0 < ref_count < n_refs

    # Data kept alive due to internal reference
    assert buf.read() == b"bytes"


@pytest.mark.skip_pypy
@pytest.mark.parametrize(
    "source, resize",
    (
        (bytearray(b"bytes"), lambda data: data.extend(b"s")),
        (Buffer(b"bytes"), lambda data: data.set_len(6)),
    ),
)
def test_buffer_view_pins_source(source, resize):
    # The view holds the source's export for its whole lifetime, so the source can't be
    # resized (and its memory reallocated) underneath it.
    buf = Buffer(source, copy=False)
    with pytest.raises(BufferError):
        resize(source)
    assert buf.read() == b"bytes"

    del buf
    gc.collect()
    resize(source)
    assert len(source) == 6


@pytest.mark.skip_pypy
def test_buffer_view_cannot_grow():
    buf = Buffer(bytearray(16), copy=False)
    # The view's memory belongs to the source, so a codec writing past its end must fail
    # rather than reallocate it.
    with pytest.raises(cramjam.CompressionError, match="Too much to write on view"):
        cramjam.snappy.compress_into(bytes(range(256)) * 64, buf)


@pytest.mark.skip_pypy
def test_buffer_view_of_readonly_source_rejects_writes():
    data = b"bytes"
    buf = Buffer(data, copy=False)

    with pytest.raises(TypeError, match="read-only"):
        buf.write(b"00")
    with pytest.raises(TypeError, match="read-only"):
        cramjam.snappy.decompress_raw_into(cramjam.snappy.compress_raw(b"00"), buf)
    assert data == b"bytes"
    assert buf.read() == b"bytes"  # reads are still zero-copy


@pytest.mark.skip_pypy
def test_buffer_view_of_numpy_array_is_file_like_without_copy():
    # APIs such as botocore want a file-like upload body, which a memoryview isn't;
    # Buffer(copy=False) gives one over an array's memory without copying (fsspec/s3fs#959).
    arr = np.frombuffer(b"hello world", dtype=np.uint8).copy()
    buf = Buffer(arr, copy=False)
    assert buf.get_view_reference() is arr

    assert buf.read(5) == b"hello"
    assert buf.tell() == 5
    buf.seek(0)
    arr[0] = ord("j")  # shared memory: the change shows through the Buffer
    assert buf.read() == b"jello world"


@pytest.mark.skip_pypy
def test_buffer_view_cannot_read_passed():
    data = b"bytes"
    buf = cramjam.Buffer(data, copy=False)

    # Cannot read pass length of underlying buffer
    # matches read behavior of io.BytesIO
    assert buf.read(len(data) * 2) == data

    # Cannot read pass incrementally either
    b = b""
    buf.seek(0)
    for i in range(0, 10):
        b += buf.read(i)
    assert b == data


def test_buffer_exports_memoryview():
    buf = Buffer(b"bytes")
    view = memoryview(buf)

    assert view.c_contiguous
    assert view.itemsize == 1
    assert view.ndim == 1
    assert view.nbytes == 5
    assert view.tobytes() == b"bytes"


def test_buffer_export_blocks_resize():
    # Exported views point into the Buffer's allocation; resizing would leave them dangling.
    buf = Buffer(b"a" * 64)
    view = memoryview(buf)

    for resize in (
        lambda: buf.set_len(1 << 20),
        buf.truncate,
        lambda: buf.write(b"x" * 128),
    ):
        with pytest.raises(BufferError):
            resize()
    with pytest.raises(cramjam.CompressionError, match="Existing exports"):
        cramjam.snappy.compress_into(b"x" * 1024, buf)

    buf.set_len(64)  # same length is fine
    buf.write(b"b")  # in-place write is fine
    assert view[:2] == b"ba"

    view.release()
    buf.set_len(1 << 20)
    assert len(buf) == 1 << 20


def test_concurrent_codec_calls():
    data = b"concurrent compression" * 100

    def roundtrip(_):
        compressed = cramjam.zstd.compress(data)
        return bytes(cramjam.zstd.decompress(compressed))

    with ThreadPoolExecutor(max_workers=4) as executor:
        assert list(executor.map(roundtrip, range(32))) == [data] * 32
