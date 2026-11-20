#!/bin/bash
# newtos-negative-control.sh <wt> <neg-tree> <DUMPBIOS.NAP> <scratch>: the
# new table on the kernel BEFORE TASK/BIOS20 (main c2140d9ef, a detached
# worktree, built by cargo xtask tosdump --tree), with this tree's prebuilt
# dumper. The cells BIOS20 serves are 0 there, so the table must flag them.
set -u
WT=$1; NEG=$2; NAP=$3; S=$4
RUN=$(cd "$(dirname "$0")/.." && pwd)/runs
REF=$WT/tosdump/reference/EMUTOS-1a7484ab
cd "$WT" || exit 1
cargo xtask tosdump "$NAP" tosdump/cases/bios-sysvars-r1.tsv --tree "$NEG" \
  --out "$S/negctl-bios-sysvars-r1" --reference $REF-BIOS-sysvars.BIN \
  --reference $REF-BIOS-sysvars-falcontc.BIN --scratch "$S/scratch-neg" > "$S/negctl.out" 2>&1
echo "exit $?"
mkdir -p "$RUN/negctl-bios-sysvars-r1"
cp "$S/negctl.out" "$RUN/negctl-bios-sysvars-r1/xtask-tosdump.out"
cp "$S/negctl-bios-sysvars-r1"/tosdiff-*.txt "$S/negctl-bios-sysvars-r1/card.img.manifest" \
   "$S/negctl-bios-sysvars-r1/BIOS.LOG" "$RUN/negctl-bios-sysvars-r1/"
python3 -I "$(dirname "$0")/decode-peek.py" "$S/negctl-bios-sysvars-r1/BIOS.BIN" \
  tosdump/cases/bios-sysvars-r1.tsv > "$RUN/negctl-bios-sysvars-r1/decoded.txt"
shasum -a 256 "$S/negctl-bios-sysvars-r1/BIOS.BIN" | sed "s|$S/||" >> "$RUN/newtos-bin.sha256"
