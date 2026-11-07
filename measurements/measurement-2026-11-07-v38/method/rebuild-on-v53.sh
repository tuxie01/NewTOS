#!/bin/sh
# V38: V53 landed (4cc7005c) during this run. Re-check the warnings in the main tree on the new HEAD:
# variant head = HEAD's contour.rs (copied in by hand), fix = the V38 contour.rs; each board, debug build
# and clippy after cargo clean -p tos-api -p kernel. Restores the V38 file at the end.
S=${S:?set S to the session scratchpad/v38}
M=/Volumes/Daten/Development/PiRusTOS/measurements/measurement-2026-11-07-v38
cd /Volumes/Daten/Development/PiRusTOS || exit 1
cp tos-api/src/vdi/contour.rs "$S/contour.rs.v38"
for v in head fix; do
  if [ $v = head ]; then git show HEAD:tos-api/src/vdi/contour.rs > tos-api/src/vdi/contour.rs; else cp "$S/contour.rs.v38" tos-api/src/vdi/contour.rs; fi
  for b in rpi4 qemu-virt; do
    cargo clean -p tos-api -p kernel > /dev/null 2>&1
    cargo xtask build --board $b > "$M/builds/v53-build-$v-$b.log" 2>&1; echo "build $v $b exit $?"
    cargo clean -p tos-api -p kernel > /dev/null 2>&1
    cargo clippy -p kernel --features board-$b > "$M/builds/v53-clippy-$v-$b.log" 2>&1; echo "clippy $v $b exit $?"
  done
done
cp "$S/contour.rs.v38" tos-api/src/vdi/contour.rs
shasum -a 256 tos-api/src/vdi/contour.rs
