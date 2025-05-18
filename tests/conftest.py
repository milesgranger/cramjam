import platform
import sysconfig

import pytest


@pytest.fixture(scope="session")
def is_pypy():
    impl = platform.python_implementation()
    return impl.lower() == "pypy"

@pytest.fixture(scope="session")
def is_free_threaded():
    return bool(sysconfig.get_config_var("Py_GIL_DISABLED"))


def pytest_configure(config):
    config.addinivalue_line("markers", "skip_pypy: skip this test on PyPy")


def pytest_runtest_setup(item):
    if "skip_pypy" in item.keywords and platform.python_implementation() == "PyPy":
        pytest.skip("skipped on PyPy")
