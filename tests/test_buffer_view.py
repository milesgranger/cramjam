import gc

import cramjam
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
def test_buffer_view_changing_underlying_buffer_size():
    # A buffer owning it's data
    data = Buffer()
    data.write(b"bytes")

    # Our reference buffer
    buf = Buffer(data, copy=False)

    # Can write 5 bytes, no problem.
    buf.write(b"12345")

    # 6th is an issue
    with pytest.raises(IOError, match="Too much to write on view"):
        buf.write(b"6")

    # buf if we extend our data, then we can
    assert len(buf) == 5
    data.write(b"s")
    assert len(buf) == 6  # Length sync'd with underlying buffer

    assert buf.tell() == 5
    buf.write(b"6")
    assert buf.tell() == 6

    # Now, shrink the underlying data
    data.set_len(2)
    assert buf.tell() == 2  # updated to end of buffer

    # Writing fails b/c the underlying is at 2 in length now
    with pytest.raises(IOError, match="Too much to write on view"):
        buf.write(b"6")

    # Seek to 1 and write one byte
    buf.seek(1)
    buf.write(b"1")
    assert buf.tell() == 2  # back at 2
    assert len(buf) == 2


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
