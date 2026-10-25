#!/usr/bin/env bash
# lint.sh <tree> <outdir> -- builds both boards dev+release (exit + warning lines), clippy (kernel per board, tos-api host,
# tos-api --tests), cargo xtask test totals. Copied from measurement-2026-10-25-a117/method/lint.sh (test file name changed).
# A kernel8.img an rpi4 build drops into the tree root is moved into <outdir>.
set -u
T="$1"; O="$2"; mkdir -p "$O"; cd "$T"; : > "$O/summary.txt"
for B in qemu-virt rpi4; do
  for P in dev release; do
    touch kernel/src/lib.rs tos-api/src/lib.rs
    if [ $P = dev ]; then cargo xtask build --board $B > "$O/build-$B-$P.log" 2>&1; else cargo xtask build --board $B --release > "$O/build-$B-$P.log" 2>&1; fi
    echo "build $B $P exit $?  warning-lines $(grep -cE '^warning' "$O/build-$B-$P.log")" >> "$O/summary.txt"
    [ -f kernel8.img ] && mv kernel8.img "$O/kernel8-$B-$P.img"
  done
  touch kernel/src/lib.rs tos-api/src/lib.rs
  cargo clippy -p kernel --no-default-features --features board-$B > "$O/clippy-kernel-$B.log" 2>&1; echo "clippy kernel $B exit $?" >> "$O/summary.txt"
  grep -E "^warning: .* generated [0-9]+ warning" "$O/clippy-kernel-$B.log" >> "$O/summary.txt"
done
touch tos-api/src/lib.rs
cargo clippy -p tos-api > "$O/clippy-tosapi-host.log" 2>&1; echo "clippy tos-api host exit $?" >> "$O/summary.txt"
grep -E "^warning: .* generated [0-9]+ warning" "$O/clippy-tosapi-host.log" >> "$O/summary.txt"
cargo clippy -p tos-api --tests > "$O/clippy-tosapi-tests.log" 2>&1; echo "clippy tos-api --tests exit $?" >> "$O/summary.txt"
grep -E "^warning: .* generated [0-9]+ warning" "$O/clippy-tosapi-tests.log" >> "$O/summary.txt"
echo "clippy tos-api --tests warnings in sched_a115_ready_list.rs: $(grep -c 'sched_a115_ready_list.rs' "$O/clippy-tosapi-tests.log")" >> "$O/summary.txt"
cargo xtask test > "$O/xtask-test.log" 2>&1; echo "cargo xtask test exit $?" >> "$O/summary.txt"
grep -E "^test result:" "$O/xtask-test.log" | awk '{p+=$4; f+=$6; i+=$8} END {print "xtask test: binaries " NR " passed " p " failed " f " ignored " i}' >> "$O/summary.txt"
echo DONE >> "$O/summary.txt"
