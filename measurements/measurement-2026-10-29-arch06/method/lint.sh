#!/usr/bin/env bash
# lint.sh <tree> <outdir> -- build both boards + clippy counts (kernel per board, tos-api host, desktop+newdesk programs per board)
set -u
T="$1"; O="$2"; mkdir -p "$O"; cd "$T"; : > "$O/summary.txt"
for B in qemu-virt rpi4; do
  cargo xtask build --board $B > "$O/build-$B.log" 2>&1; echo "build $B exit $?" >> "$O/summary.txt"
  grep -cE "^warning" "$O/build-$B.log" | sed "s/^/build-$B warnings: /" >> "$O/summary.txt"
  touch kernel/src/lib.rs tos-api/src/lib.rs
  cargo clippy -p kernel --no-default-features --features board-$B > "$O/clippy-kernel-$B.log" 2>&1; echo "clippy kernel $B exit $?" >> "$O/summary.txt"
  grep -E "^warning: .* generated [0-9]+ warning" "$O/clippy-kernel-$B.log" >> "$O/summary.txt"
  for P in desktop newdesk; do
    (cd programs && cargo clippy -p $P --release --features board-$B > "$O/clippy-$P-$B.log" 2>&1; echo "clippy $P $B exit $?" >> "$O/summary.txt")
    grep -E "^warning: .* generated [0-9]+ warning" "$O/clippy-$P-$B.log" >> "$O/summary.txt"
  done
done
cargo clippy -p tos-api --tests > "$O/clippy-tosapi-host.log" 2>&1; echo "clippy tos-api host --tests exit $?" >> "$O/summary.txt"
grep -E "^warning: .* generated [0-9]+ warning" "$O/clippy-tosapi-host.log" >> "$O/summary.txt"
echo DONE >> "$O/summary.txt"
