import platform

import pytest


@pytest.fixture(scope="session")
def is_pypy():
    impl = platform.python_implementation()
    return impl.lower() == "pypy"


def pytest_configure(config):
    config.addinivalue_line("markers", "skip_pypy: skip this test on PyPy")


def pytest_runtest_setup(item):
    if "skip_pypy" in item.keywords and platform.python_implementation() == "PyPy":
        pytest.skip("skipped on PyPy")
