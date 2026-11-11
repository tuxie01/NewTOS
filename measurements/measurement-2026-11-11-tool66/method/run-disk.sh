#!/bin/sh
# run-disk.sh <scratch> <runs-dir> -- the three disk tables of finding F1 (TASK/TOOL66) on NewTOS
# (QEMU raspi4b) through `cargo xtask tosdump --disk`, the call of
# measurements/measurement-2026-11-11-bios-full/method/run-all.sh. Run from the repository root.
S=$1; R=$2; REF=tosdump/reference/EMUTOS-1a7484ab
mkdir -p "$S" "$R"
for T in r2 r2-phys r2-x001; do
  case $T in r2) ref=BIOS-r2-disk;; *) ref=BIOS-$T-disk;; esac
  n=bios-$T
  cargo xtask tosdump bios tosdump/cases/bios-$T.tsv --disk --out "$R/$n" --scratch "$S" --reference $REF-$ref.BIN > "$R/$n.stdout" 2>&1
  echo "$n exit $?"
  [ -f "$R/$n/testpart-after.img" ] && mv "$R/$n/testpart-after.img" "$S/$n-testpart-after.img"
done
