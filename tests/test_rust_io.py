import pytest

from cramjam import File, Buffer


@pytest.mark.parametrize("Obj", (File, Buffer))
def test_obj_api(tmpdir, Obj):
    if isinstance(Obj, File):
        buf = File(str(tmpdir.join("file.txt")))
    else:
        buf = Buffer()

    assert buf.write(b"bytes") == 5
    assert buf.tell() == 5
    assert buf.seek(0) == 0
    assert buf.read() == b"bytes"
    assert buf.seek(-1, 2) == 4  # set one byte backwards from end; position 4
    assert buf.read() == b"s"
    assert buf.seek(-2, whence=1) == 3  # set two bytes from current (end): position 3
    assert buf.read() == b"es"

    with pytest.raises(ValueError):
        buf.seek(1, 3)  # only 0, 1, 2 are valid seek from positions

    for out in (
        bytearray(b"12345"),
        File(str(tmpdir.join("test.txt"))),
        Buffer(),
    ):
        buf.seek(0)

        expected = b"bytes"

        buf.readinto(out)

        # Will update the output buffer
        if isinstance(out, (File, Buffer)):
            out.seek(0)
            assert out.read() == expected
        else:
            assert out == bytearray(expected)

    # Set the length
    buf.set_len(2)
    buf.seek(0)
    assert buf.read() == b"by"
    buf.set_len(10)
    buf.seek(0)
    assert buf.read() == b"by\x00\x00\x00\x00\x00\x00\x00\x00"

    # truncate
    buf.truncate()
    buf.seek(0)
    assert buf.read() == b""


@pytest.mark.parametrize("Obj", (File, Buffer))
@pytest.mark.parametrize("out", (b"12345", memoryview(b"12345")))
def test_readinto_rejects_readonly_buffer(tmpdir, Obj, out):
    buf = File(str(tmpdir.join("file.txt"))) if Obj is File else Buffer()
    buf.write(b"bytes")
    buf.seek(0)

    with pytest.raises(OSError, match="read-only"):
        buf.readinto(out)
