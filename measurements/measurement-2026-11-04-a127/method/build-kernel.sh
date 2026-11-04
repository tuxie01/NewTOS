#!/usr/bin/env bash
# build-kernel.sh <repo> <scratch-tree> <arm-trigger=script> [...] -- rebuild the
# instrumented rpi4 release kernel in a SCRATCH copy of the tree: the three
# files tools/inject-arm.py patches are restored from the repo first, then the
# injector is armed with the given script(s). The repo itself is never touched.
set -euo pipefail
REPO=$1; T=$2; shift 2
for f in kernel/src/host_env.rs kernel/src/launch.rs kernel/src/shell_loop.rs; do cp "$REPO/$f" "$T/$f"; done
python3 -I "$T/tools/inject-arm.py" "$T" "$@" | tail -1
(cd "$T" && cargo xtask build --board rpi4 --release) > "$T/../build.txt" 2>&1 || { tail -20 "$T/../build.txt" >&2; exit 1; }
shasum -a 256 "$T/target/aarch64-unknown-none-softfloat/release/kernel8.img"
