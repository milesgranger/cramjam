import contextlib
import platform
import sysconfig

import pytest

# Eagerly import hypothesis modules that are otherwise imported lazily in the
# middle of the test session (e.g. on the first failing example). pytest
# assertion-rewrites hypothesis modules on import because hypothesis ships a
# pytest plugin, and a mid-session ast.parse of these files has been observed
# to fail with a bogus SyntaxError on CI runners, turning any real test
# failure into an unreportable INTERNALERROR. Importing them up front means
# they are parsed once, before any tests run, so genuine failures get
# reported normally.
import hypothesis.internal.conjecture.optimiser  # noqa: F401

with contextlib.suppress(ImportError):  # requires the optional libcst
    import hypothesis.extra._patching  # noqa: F401


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
