#!/bin/sh
# oracle-regression.sh <wt> <scratch>: the existing tables bios-r1.tsv and
# bios-core-r1.tsv run with this tree's DUMPBIOS.PRG on the pinned oracle
# (tt, falcontc; one Hatari at a time) and compared with their frozen
# references: tosdiff (no case may differ) and the records byte for byte
# (cmp from offset 32; the header's build id at 12..15 is the dumper's).
set -u
WT=$1; S=$2
RUN=$(cd "$(dirname "$0")/.." && pwd)/runs
D=$WT/tosdump/host/build/tosdiff
REF=$WT/tosdump/reference/EMUTOS-1a7484ab
for t in "bios-r1 BIOS" "bios-core-r1 BIOS-core"; do
  set -- $t
  for p in tt falcontc; do
    [ $p = tt ] && r=$REF-$2.BIN || r=$REF-$2-falcontc.BIN
    o=$S/regress-$1-$p
    "$WT/tools/hatari-oracle.sh" $p "$WT/tosdump/build/DUMPBIOS.PRG" "$WT/tosdump/cases/$1.tsv" "$o" > "$o.out" 2>&1
    echo "oracle $p $1: exit $?"
    f=$RUN/regress-$1-$p.txt
    "$D" "$WT/tosdump/cases/$1.tsv" "$r" "$o/BIOS.BIN" > "$f" 2>&1; echo "tosdiff exit $?" >> "$f"
    cmp "$(dirname "$r")/$(basename "$r")" "$o/BIOS.BIN" > /dev/null 2>&1; echo "cmp whole file: exit $?" >> "$f"
    tail -c +33 "$r" > "$S/a.rec"; tail -c +33 "$o/BIOS.BIN" > "$S/b.rec"
    cmp "$S/a.rec" "$S/b.rec" >> "$f" 2>&1; echo "cmp records (from offset 32): exit $?" >> "$f"
    cp "$o/BIOS.LOG" "$RUN/regress-$1-$p.LOG"
  done
done
