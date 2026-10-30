#!/bin/bash
# lint.sh <tree> <out-dir> -- build both boards (release) and clippy kernel per board (which
# lints tos-api with it); the rustc/clippy summary lines per crate into <out-dir>
# (copied unchanged from measurement-2026-10-30-bios16/method/lint.sh, itself from bios08).
T=$1; O=$2; mkdir -p "$O"; O=$(cd "$O" && pwd); cd "$T"
for b in rpi4 qemu-virt; do
  touch kernel/src/lib.rs tos-api/src/lib.rs
  cargo xtask build --board $b --release > "$O/build-$b.log" 2>&1; echo "build $b exit $?"
  touch kernel/src/lib.rs tos-api/src/lib.rs
  cargo clippy --package kernel --features board-$b --message-format short > "$O/clippy-$b.log" 2>&1; echo "clippy $b exit $?"
done
for f in "$O"/*.log; do
  echo "$(basename $f): $(grep -c '^warning' $f) warning lines"
  grep -E "generated [0-9]+ warning" "$f"
done
