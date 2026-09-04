#!/usr/bin/env bash
#
# Install the Rust toolchain Pyodide expects for wasm32-unknown-emscripten builds.
#
# Each Pyodide release pins an exact Rust toolchain, and some ship their own build of the
# wasm32-unknown-emscripten standard library, compiled against the matching Emscripten
# version and exception-handling flags. `pyodide build` exports that configuration but
# leaves installing the toolchain to the caller, so this mirrors what pyodide-build does
# for its own in-tree recipes.
#
# Note that this sets the default rustup toolchain, as pyodide-build does, because the
# toolchain has to still be in effect when maturin runs in a later step. Running this
# outside CI will change the default toolchain of the machine it runs on.
set -euo pipefail

toolchain=$(pyodide config get rust_toolchain)
target_url=$(pyodide config get rust_emscripten_target_url)

rustup toolchain install "${toolchain}"
rustup default "${toolchain}"

if [ -z "${target_url}" ]; then
    rustup target add wasm32-unknown-emscripten --toolchain "${toolchain}"
    exit 0
fi

# Replace the stock target sysroot with Pyodide's build of it.
rustlib="$(dirname "$(dirname "$(rustup which --toolchain "${toolchain}" rustc)")")/lib/rustlib"
token="${rustlib}/wasm32-unknown-emscripten_install-url.txt"
if [ -f "${token}" ] && [ "$(cat "${token}")" = "${target_url}" ]; then
    exit 0
fi

rm -rf "${rustlib}/wasm32-unknown-emscripten"
mkdir -p "${rustlib}"

# Unpack with Python rather than tar: the archive is bzip2 today but the format is not
# guaranteed, GNU tar will not sniff it off a pipe, and its bzip2 support is a separate
# binary that need not be installed. shutil handles every format Pyodide might publish.
tmpdir=$(mktemp -d)
trap 'rm -rf "${tmpdir}"' EXIT
archive="${tmpdir}/$(basename "${target_url}")"
curl -L --proto '=https' --tlsv1.2 -sSf "${target_url}" -o "${archive}"
python -c 'import shutil, sys; shutil.unpack_archive(sys.argv[1], sys.argv[2])' "${archive}" "${rustlib}"

echo "${target_url}" > "${token}"
