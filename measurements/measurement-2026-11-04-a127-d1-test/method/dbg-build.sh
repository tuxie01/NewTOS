#!/usr/bin/env bash
# dbg-build.sh <pristine-src-dir> <scratch-tree> <script> -- like A127 build-kernel.sh plus dbg-patch.py
set -euo pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
R=$1; T=$2; SC=$3
for f in kernel/src/host_env.rs kernel/src/launch.rs kernel/src/shell_loop.rs; do cp "$R/$f" "$T/$f"; done
python3 -I "$T/tools/inject-arm.py" "$T" "${TRIG:-desktop}=$SC" | tail -1
python3 -I "$HERE/dbg-patch.py" "$T"
rm -rf "$T/target"
(cd "$T" && cargo xtask build --board rpi4 --release) > "$T/../build-dbg.txt" 2>&1 || { tail -20 "$T/../build-dbg.txt" >&2; exit 1; }
shasum -a 256 "$T/target/aarch64-unknown-none-softfloat/release/kernel8.img"
