import platform
import pytest


@pytest.fixture(scope='session')
def is_pypy():
    impl = platform.python_implementation()
    return impl.lower() == 'pypy'
