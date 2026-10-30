#!/bin/bash
# (copied from measurement-2026-10-30-bios16/method/run-p0par.sh; only "BIOS16 tree" -> "BIOS17 tree")
# run-p0par.sh <scratch> <injector kernel8.img> <out-dir> -- MEM12 retest (page 0 read-only in
# user state, writable in super state) with the MEM12 acceptance probe binaries
# measurements/measurement-2026-10-27-mem12-acceptance/dumpers/P0PAR.NAP + P0KID.NAP (source
# sdk/examples/p0par.c, p0kid.c) on the BIOS17 tree. The kernel is a scratch copy of the tree
# armed with `tools/inject-arm.py <copy> launch:P0PAR=method/p0.script` (the acceptance run's
# script, copied: one space key per crash screen). Card: tools/mkcard.sh, NEWDESK.INF
# "#R 02 / #Z 01 C:\P0PAR.NAP@", booted as an APFS clone; QEMU raspi4b -append serial=on.
set -u
S=$1; K=$2; D=$3
HERE=$(cd "$(dirname "$0")" && pwd); REPO=$(cd "$HERE/../../.." && pwd)
P=$REPO/measurements/measurement-2026-10-27-mem12-acceptance/dumpers
[ ! -e "$D" ] || { echo "out exists: $D" >&2; exit 1; }
mkdir -p "$D"; D=$(cd "$D" && pwd)
IMG="$S/p0-card.img"; rm -f "$IMG" "$IMG.manifest" "$S/p0-boot.img"
printf '#R 02\r\n#Z 01 C:\\P0PAR.NAP@\r\n' > "$S/z-p0par.inf"
(cd "$REPO" && tools/mkcard.sh "$IMG" "$S/z-p0par.inf" "P0PAR.NAP=$P/P0PAR.NAP" "P0KID.NAP=$P/P0KID.NAP" >/dev/null 2>&1) || { echo "mkcard failed" >&2; exit 1; }
cp "$IMG.manifest" "$D/card.img.manifest"
{ echo "probes $(shasum -a 256 "$P/P0PAR.NAP" | cut -d' ' -f1) P0PAR.NAP, $(shasum -a 256 "$P/P0KID.NAP" | cut -d' ' -f1) P0KID.NAP"
  echo "kernel $(shasum -a 256 "$K" | cut -d' ' -f1) (injector copy)"
  echo "card   $(shasum -a 256 "$IMG" | cut -d' ' -f1) (tools/mkcard.sh)"; } > "$D/run.txt"
cp -c "$IMG" "$S/p0-boot.img"
: > "$D/BOOT-SERIAL.txt"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$D/BOOT-SERIAL.txt" \
   -append serial=on -kernel "$K" -drive file="$S/p0-boot.img",if=sd,format=raw 2>/dev/null &
pid=$!
for _ in $(seq 1 150); do
  sleep 2
  /usr/bin/grep -aq "P0PAR.NAP' exited with rc" "$D/BOOT-SERIAL.txt" &&
    /usr/bin/grep -aq "desktop up" "$D/BOOT-SERIAL.txt" && break
  kill -0 $pid 2>/dev/null || break
done
sleep 3; kill $pid 2>/dev/null; sleep 1; kill -9 $pid 2>/dev/null; wait $pid 2>/dev/null
echo "card   after $(shasum -a 256 "$S/p0-boot.img" | cut -d' ' -f1) (the booted clone)" >> "$D/run.txt"
rm -f "$IMG" "$IMG.manifest" "$S/p0-boot.img"
/usr/bin/grep -a "^P0PAR:\|\[hook\]\|PANIC\|exited with rc" "$D/BOOT-SERIAL.txt"
