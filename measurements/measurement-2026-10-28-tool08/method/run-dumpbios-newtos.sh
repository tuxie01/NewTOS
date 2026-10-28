#!/bin/bash
# run-dumpbios-newtos.sh <repo> <scratch> <out-dir> <table.tsv> <ref-tt-base> <ref-falcontc-base>
#
# Copy of measurements/measurement-2026-10-28-bios-storage-audit/method/run-dumpbios-newtos.sh
# (kept unchanged next to it as run-dumpbios-newtos.orig.sh), parametrised for this audit:
#   <repo>    the tree the kernel, DUMPBIOS.NAP, tosdiff and the table come from
#             (the frozen worktree at 1680d5b2, not the shared main tree)
#   <table>   repo-relative case table, put on the card as BIOS.TSV
#   <ref-*>   repo-relative reference base names (without .BIN)
# Card: tools/mkcard.sh with DUMPBIOS.NAP (<scratch>/DUMPBIOS.NAP, built by
# tosdump/build-dumpbios-newtos.sh) + BIOS.TSV; kernel <scratch>/kernel8.img;
# QEMU raspi4b, -append serial=on.  Only the card name and the parameters differ
# from the original.
set -u
REPO=$1; S=$2; D=$3; T=$4; RTT=$5; RFC=$6
case $T in /*) TA=$T ;; *) TA=$REPO/$T ;; esac
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
mkdir -p "$D"; D="$(cd "$D" && pwd)"; cd "$REPO"
IMG="$S/db-$(basename "$D").img"; rm -f "$IMG" "$IMG.manifest"
tools/mkcard.sh "$IMG" "$HERE/z-dumpbios.inf" DUMPBIOS.NAP="$S/DUMPBIOS.NAP" BIOS.TSV="$TA" >/dev/null 2>&1
cp "$IMG.manifest" "$D/db.img.manifest"
: > "$D/BOOT-SERIAL.txt"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$D/BOOT-SERIAL.txt" \
   -append serial=on -kernel "$S/kernel8.img" -drive file="$IMG",if=sd,format=raw &
pid=$!
for i in $(seq 1 240); do sleep 2; /usr/bin/grep -q "desktop up" "$D/BOOT-SERIAL.txt" && break; kill -0 $pid 2>/dev/null || break; done
sleep 3; kill $pid 2>/dev/null; sleep 1; kill -9 $pid 2>/dev/null; wait $pid 2>/dev/null
export MTOOLS_SKIP_CHECK=1
mcopy -o -i "$IMG@@32256" ::/BIOS.BIN "$D/BIOS.BIN"; mcopy -o -i "$IMG@@32256" ::/BIOS.LOG "$D/BIOS.LOG"
shasum -a 256 "$IMG" > "$D/db.img.after.sha256"
X=tosdump/host/build/tosdiff
mkdir -p "$D/cmp"; cp "$RTT.BIN" "$D/cmp/EMUTOS.BIN"; cp "$RFC.BIN" "$D/cmp/EMUTOS-FALCONTC.BIN"; cp "$D/BIOS.BIN" "$D/cmp/NEWTOS.BIN"
"$X" --validate "$D/cmp/NEWTOS.BIN" "$TA"
(cd "$D/cmp" && "$REPO/$X" "$TA" EMUTOS.BIN NEWTOS.BIN > ../tosdiff-newtos-vs-emutos.txt; echo "tt exit $?"; "$REPO/$X" "$TA" EMUTOS-FALCONTC.BIN NEWTOS.BIN > ../tosdiff-newtos-vs-emutos-falcontc.txt; echo "falcontc exit $?")
rm -f "$IMG"
