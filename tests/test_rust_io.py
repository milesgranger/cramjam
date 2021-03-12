import pytest

@pytest.mark.parametrize("Obj", (File, Obj))
def test_obj(Obj)