#!/bin/sh
# V31 test: tosdump tables on the base (head = deaad5df + the V31 vdprobe.c, product code = base) and on the fix tree (35d052a9).
# Run from the main tree. S = session scratchpad; trees/{head,fix} = git archive of the commits (references/ linked).
S=${S:?}
M=measurements/measurement-2026-11-07-v31-test
R=tosdump/reference
run() { T=$1; D=$2; TAB=$3; O=$4; REF=$5
  if [ -n "$REF" ]; then
    cargo xtask tosdump $D tosdump/cases/$TAB --tree $S/trees/$T --out $M/runs/$O --reference $R/$REF.BIN --scratch $S/xt-$T
  else
    cargo xtask tosdump $D tosdump/cases/$TAB --tree $S/trees/$T --out $M/runs/$O --scratch $S/xt-$T
  fi; }
set -x
for T in fix head; do
  run $T vdi vdi-v18-t2.tsv tab-$T-v18t2 EMUTOS-1a7484ab-VDI-v18t2-falcontc
  run $T vdi vdi-all.tsv tab-$T-all EMUTOS-1a7484ab-VDI-all-falcontc
  run $T aes aes-r2-wind.tsv tab-$T-r2 EMUTOS-1a7484ab-AES-r2wind
  run $T aes aes-r3-wind-state.tsv tab-$T-r3 EMUTOS-1a7484ab-AES-r3
  run $T aes aes-r4-subfunctions.tsv tab-$T-r4
done
echo finished
