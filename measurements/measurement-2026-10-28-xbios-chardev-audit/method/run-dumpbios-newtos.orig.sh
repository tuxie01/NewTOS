#!/bin/bash
# run-dumpbios-newtos.sh <scratch> <out-dir> -- bios-r1.tsv on NewTOS at the audited
# commit (Drvmap case 74 re-measured under tosdiff). Card: tools/mkcard.sh with
# DUMPBIOS.NAP (tosdump/build-dumpbios-newtos.sh <scratch>/DUMPBIOS.NAP) + BIOS.TSV;
# kernel <scratch>/kernel8.img; QEMU raspi4b, -append serial=on.
set -u
S=$1; D=$2; HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; REPO="$(cd "$HERE/../../.." && pwd)"
mkdir -p "$D"; cd "$REPO"
tools/mkcard.sh "$S/db.img" "$HERE/z-dumpbios.inf" DUMPBIOS.NAP="$S/DUMPBIOS.NAP" BIOS.TSV=tosdump/cases/bios-r1.tsv >/dev/null 2>&1
cp "$S/db.img.manifest" "$D/db.img.manifest"
: > "$D/BOOT-SERIAL.txt"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$D/BOOT-SERIAL.txt" \
   -append serial=on -kernel "$S/kernel8.img" -drive file="$S/db.img",if=sd,format=raw &
pid=$!
for i in $(seq 1 240); do sleep 2; /usr/bin/grep -q "desktop up" "$D/BOOT-SERIAL.txt" && break; kill -0 $pid 2>/dev/null || break; done
sleep 3; kill $pid 2>/dev/null; sleep 1; kill -9 $pid 2>/dev/null; wait $pid 2>/dev/null
export MTOOLS_SKIP_CHECK=1
mcopy -o -i "$S/db.img@@32256" ::/BIOS.BIN "$D/BIOS.BIN"; mcopy -o -i "$S/db.img@@32256" ::/BIOS.LOG "$D/BIOS.LOG"
shasum -a 256 "$S/db.img" > "$D/db.img.after.sha256"
T=tosdump/cases/bios-r1.tsv; X=tosdump/host/build/tosdiff
mkdir -p "$D/cmp"; cp tosdump/reference/EMUTOS-1a7484ab-BIOS.BIN "$D/cmp/EMUTOS.BIN"; cp tosdump/reference/EMUTOS-1a7484ab-BIOS-falcontc.BIN "$D/cmp/EMUTOS-FALCONTC.BIN"; cp "$D/BIOS.BIN" "$D/cmp/NEWTOS.BIN"
"$X" --validate "$D/cmp/NEWTOS.BIN" "$T"
(cd "$D/cmp" && "$REPO/$X" "$REPO/$T" EMUTOS.BIN NEWTOS.BIN > ../tosdiff-newtos-vs-emutos.txt; echo "tt exit $?"; "$REPO/$X" "$REPO/$T" EMUTOS-FALCONTC.BIN NEWTOS.BIN > ../tosdiff-newtos-vs-emutos-falcontc.txt; echo "falcontc exit $?")
