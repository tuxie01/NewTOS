#!/bin/bash
# fix against base, byte for byte; tosdiff base vs fix per table.
M=measurements/measurement-2026-11-07-v34-test; R=$M/runs; A=$M/analysis
for p in "v18t2 VDI vdi-v18-t2" "all VDI vdi-all" "r2 AES aes-r2-wind" "r3 AES aes-r3-wind-state" "r4 AES aes-r4-subfunctions"; do set -- $p
  f=$R/tab-fix-$1/$2.BIN; b=$R/tab-base-$1/$2.BIN
  if cmp -s $f $b; then echo "$1 $2.BIN fix==base $(shasum -a 256 $f | cut -c1-16)"; else echo "$1 $2.BIN fix!=base ($(cmp -l $f $b | wc -l | tr -d ' ') bytes)"; fi
  tosdump/host/build/tosdiff tosdump/cases/$3.tsv $b $f > $A/tosdiff-base-vs-fix-$1.txt 2>&1; echo "  tosdiff base vs fix $1 rc $?: $(tail -1 $A/tosdiff-base-vs-fix-$1.txt)"
done
