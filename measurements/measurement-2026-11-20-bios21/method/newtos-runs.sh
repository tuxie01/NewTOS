#!/bin/bash
# newtos-runs.sh <wt> <scratch>: three tables on NewTOS (QEMU raspi4b,
# cargo xtask tosdump, kernel rpi4 release of the tree, no features), one
# QEMU at a time: the new bios-sysvars-r1.tsv against its two references,
# and the existing bios-r1.tsv and bios-core-r1.tsv against theirs (the
# regression bar: the same differing cases as
# measurements/measurement-2026-11-20-bios20-test/RUNINFO.TXT).
# Text outputs are copied into runs/newtos-<table>/.
set -u
WT=$1; S=$2
RUN=$(cd "$(dirname "$0")/.." && pwd)/runs
REF=$WT/tosdump/reference/EMUTOS-1a7484ab
cd "$WT" || exit 1
for p in "bios-sysvars-r1 BIOS-sysvars" "bios-r1 BIOS" "bios-core-r1 BIOS-core"; do
  set -- $p
  echo "=== $1"
  cargo xtask tosdump bios tosdump/cases/$1.tsv --out "$S/newtos-$1" \
    --reference $REF-$2.BIN --reference $REF-$2-falcontc.BIN \
    --scratch "$S/scratch" > "$S/newtos-$1.out" 2>&1
  echo "exit $?"
  mkdir -p "$RUN/newtos-$1"
  cp "$S/newtos-$1.out" "$RUN/newtos-$1/xtask-tosdump.out"
  for f in BIOS.LOG BOOT-SERIAL.txt card.img.manifest tosdump-run.txt tosdiff-validate.txt qemu-stderr.txt; do
    [ -f "$S/newtos-$1/$f" ] && cp "$S/newtos-$1/$f" "$RUN/newtos-$1/"
  done
  cp "$S/newtos-$1"/tosdiff-*.txt "$RUN/newtos-$1/"
  shasum -a 256 "$S/newtos-$1/BIOS.BIN" "$S/newtos-$1/DUMPBIOS.NAP" | sed "s|$S/||" >> "$RUN/newtos-bin.sha256"
done
