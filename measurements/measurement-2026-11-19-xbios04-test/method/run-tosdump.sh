#!/bin/bash
# run-tosdump.sh <tree> <label> <out-root>: four tables on raspi4b, one QEMU at a time
T=$1; L=$2; O=$3
cd "$T" || exit 1
for p in "bios-r1 BIOS" "bios-core-r1 BIOS-core" "xbios-chardev-r1 XBIOS-chardev" "xbios-system-r1 XBIOS-system"; do
  set -- $p
  echo "=== $L $1"
  cargo xtask tosdump bios tosdump/cases/$1.tsv --out "$O/$L-$1" \
    --reference tosdump/reference/EMUTOS-1a7484ab-$2.BIN --scratch "$O/scratch-$L" 2>&1 | tail -15
done
