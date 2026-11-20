#!/bin/sh
# oracle-runs.sh <wt> <scratch>: bios-sysvars-r1.tsv on the pinned oracle,
# profiles tt and falcontc, two runs each, one Hatari at a time
# (tools/hatari-oracle.sh). Per run: BIOS.BIN/LOG, decoded.txt; then cmp of
# the two runs, tosdiff --selfcheck of run 1, tt vs falcontc.
set -u
WT=$1; S=$2
RUN=$(cd "$(dirname "$0")/.." && pwd)/runs
T=$WT/tosdump/cases/bios-sysvars-r1.tsv
D=$WT/tosdump/host/build/tosdiff
for p in tt falcontc; do
  for n in 1 2; do
    o=$S/oracle-$p-$n
    "$WT/tools/hatari-oracle.sh" $p "$WT/tosdump/build/DUMPBIOS.PRG" "$T" "$o" > "$S/oracle-$p-$n.out" 2>&1
    echo "oracle $p run $n: exit $?"
    mkdir -p "$RUN/oracle-$p-$n"
    cp "$o/BIOS.LOG" "$o/profile.txt" "$RUN/oracle-$p-$n/"
    python3 -I "$(dirname "$0")/decode-peek.py" "$o/BIOS.BIN" "$T" > "$RUN/oracle-$p-$n/decoded.txt"
    shasum -a 256 "$o/BIOS.BIN" | sed "s|$S/||" >> "$RUN/oracle-bin.sha256"
  done
  cmp "$S/oracle-$p-1/BIOS.BIN" "$S/oracle-$p-2/BIOS.BIN" > "$RUN/cmp-$p.txt" 2>&1; echo "cmp $p: exit $?" >> "$RUN/cmp-$p.txt"
  "$D" --selfcheck "$T" "$S/oracle-$p-1/BIOS.BIN" > "$RUN/selfcheck-$p.txt" 2>&1; echo "exit $?" >> "$RUN/selfcheck-$p.txt"
done
"$D" "$T" "$S/oracle-tt-1/BIOS.BIN" "$S/oracle-falcontc-1/BIOS.BIN" > "$RUN/tt-vs-falcontc.txt" 2>&1; echo "exit $?" >> "$RUN/tt-vs-falcontc.txt"
