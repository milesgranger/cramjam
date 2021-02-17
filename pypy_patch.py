import sysconfig
import pathlib
import re
import sys

"""
Hacky hack, not very good support with maturin for PyPy

1. Doesn't work on Windows
2. The naming is wrong in resulting wheels in Linux & OSX

This script patches the last issue; ran after normal maturin build for these
systems on PyPy builds.
"""

abi = sysconfig.get_config_var("SOABI").replace("-", "_")
major, minor = sys.version_info.major, sys.version_info.minor

regex = re.compile(r"(?P<name>pp3[py0-9_]+-pypy[3_p0-9]+)")

for file in pathlib.Path("./wheels").iterdir():
    if file.name.endswith(".whl"):
        new_name = regex.sub(f"pp{major}{minor}-{abi}", file.name)
        new_name = new_name.replace("linux", "manylinux2010")
        print(f"Renaming {file.name} -> {new_name}")
        file.rename(file.parent.joinpath(new_name))
