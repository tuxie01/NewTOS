#!/bin/sh
# usage: run-hosttest.sh <scratch-dir>   (scratch outside the repo)
set -u
S="$1"; REPO=/Volumes/Daten/Development/PiRusTOS
mkdir -p "$S/src" "$S/target"
(cd "$REPO" && git archive a7ffd707) | tar -x -C "$S/src"
cd "$S/src" || exit 1
rustc -V; cargo -V
export CARGO_TARGET_DIR="$S/target"
start=$(date +%s)
cargo xtask test > "$S/test.log" 2>&1
echo "exit=$?"
echo "wall_s=$(( $(date +%s) - start ))"
