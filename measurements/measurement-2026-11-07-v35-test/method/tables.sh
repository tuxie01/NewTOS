#!/bin/sh
# V35 test: tosdump tables on the base tree (3a9221eb) and the fix tree (a862f764). S = scratchpad; trees $S/t/{base,fix}.
S=${S:?}
M=measurements/measurement-2026-11-07-v35-test
R=tosdump/reference
run() { T=$1; D=$2; TAB=$3; O=$4; REF=$5
  if [ -n "$REF" ]; then
    cargo xtask tosdump $D tosdump/cases/$TAB --tree $S/t/$T --out $M/runs/$O --reference $R/$REF.BIN --scratch $S/xt-$T
  else
    cargo xtask tosdump $D tosdump/cases/$TAB --tree $S/t/$T --out $M/runs/$O --scratch $S/xt-$T
  fi; }
set -x
for T in fix base; do
  run $T vdi vdi-v18-t2.tsv tab-$T-v18t2 EMUTOS-1a7484ab-VDI-v18t2-falcontc
  run $T vdi vdi-all.tsv tab-$T-all EMUTOS-1a7484ab-VDI-all-falcontc
  run $T aes aes-r2-wind.tsv tab-$T-r2 EMUTOS-1a7484ab-AES-r2wind
  run $T aes aes-r3-wind-state.tsv tab-$T-r3 EMUTOS-1a7484ab-AES-r3
  run $T aes aes-r4-subfunctions.tsv tab-$T-r4
done
echo finished
