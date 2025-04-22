import pytest
from cramjam import Buffer


@pytest.mark.parametrize("copy", (None, True, False))
def test_buffer_view(copy, is_pypy):
    if is_pypy:
        pytest.skip("Zero-copy Buffer not supported on PyPy")

    kwargs = dict()
    if copy is not None:
        kwargs["copy"] = copy

    data = b"bytes"
    buf = Buffer(data, **kwargs)
    buf.write(b"0")

    if copy is False:
        assert data == b"0ytes"

    # Default is to copy, None and True behavior
    else:
        assert data == b"bytes"


def test_buffer_view_raises_when_writing_past_data_length_at_once(is_pypy):
    if is_pypy:
        pytest.skip("Zero-copy Buffer not supported on PyPy")

    data = b"bytes"
    buf = Buffer(data, copy=False)

    # Won't write pasted underlying buffer if passed data all at once
    with pytest.raises(OSError, match="Too much to write on view"):
        buf.write(b"0" * len(data) + b"0")
    assert data == b"bytes"


def test_buffer_view_raises_when_writing_past_data_length_incrementally(is_pypy):
    if is_pypy:
        pytest.skip("Zero-copy Buffer not supported on PyPy")

    data = b"bytes"
    buf = Buffer(data, copy=False)

    # This is okay, up to length of underlying buffer
    for _ in range(len(data)):
        buf.write(b"0")

    # Whoops, one too many bytes
    with pytest.raises(OSError, match="Too much to write on view"):
        buf.write(b"0")
    assert data == b"00000"


@pytest.mark.parametrize("len", range(0, 7))
def test_buffer_view_raises_when_setting_length(len, is_pypy):
    if is_pypy:
        pytest.skip("Zero-copy Buffer not supported on PyPy")

    data = b"bytes"
    buf = Buffer(data, copy=False)

    with pytest.raises(OSError, match="Cannot set length on unowned buffer"):
        buf.set_len(len)
    assert data == b"bytes"


def test_buffer_view_raises_when_truncating(is_pypy):
    if is_pypy:
        pytest.skip("Zero-copy Buffer not supported on PyPy")

    data = b"bytes"
    buf = Buffer(data, copy=False)

    with pytest.raises(OSError, match="Cannot truncate unowned buffer"):
        buf.truncate()
    assert data == b"bytes"


@pytest.mark.parametrize("whence", (0, 1, 2))
def test_buffer_view_raises_when_write_after_bad_seek(whence, is_pypy):
    if is_pypy:
        pytest.skip("Zero-copy Buffer not supported on PyPy")

    buf = Buffer(b"bytes", copy=False)

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
