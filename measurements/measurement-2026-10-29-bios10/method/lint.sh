#!/bin/bash
# lint.sh <tree> <tag> -- build both boards (release) and clippy kernel per board; count warnings per crate
T=$1; TAG=$2; O=$(dirname "$0")/$TAG; mkdir -p "$O"; cd "$T"
for b in rpi4 qemu-virt; do
  touch kernel/src/lib.rs tos-api/src/lib.rs
  cargo xtask build --board $b --release > "$O/build-$b.log" 2>&1; echo "build $b exit $?"
  touch kernel/src/lib.rs tos-api/src/lib.rs
  cargo clippy --package kernel --features board-$b --message-format short > "$O/clippy-$b.log" 2>&1; echo "clippy $b exit $?"
done
for f in "$O"/*.log; do
  echo "$(basename $f): $(grep -E '^(warning|error)' $f | grep -c 'generated') summary lines"
  grep -E "generated [0-9]+ warning" "$f"
done
