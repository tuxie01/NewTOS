#!/usr/bin/env bash
# build-inj.sh <src-tree> <scratch-tree> <arm=script>... -- tester variant of
# measurements/measurement-2026-11-04-rel07-package/method/build-inj-kernel.sh: <src-tree> is a
# `git archive` export (head = fd6c7e3f, base = 3b1fd066); it is copied (no target/) to <scratch-tree>,
# tools/inject-arm.py patches the copy, release rpi4 build (no crash-walk).
set -euo pipefail
SRC=$1; T=$2; shift 2
rm -rf "$T"; mkdir -p "$T"
rsync -a --exclude target "$SRC/" "$T/"
python3 -I "$T/tools/inject-arm.py" "$T" "$@" | tail -1
(cd "$T" && cargo xtask build --board rpi4 --release) > "$T.build.txt" 2>&1 || { tail -30 "$T.build.txt" >&2; exit 1; }
shasum -a 256 "$T/target/aarch64-unknown-none-softfloat/release/kernel8.img"
