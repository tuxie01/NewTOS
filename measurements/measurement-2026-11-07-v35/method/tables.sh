#!/bin/sh
# V35: tosdump tables, base = git archive of 35eeb6b5 ($S/base-tree), fix = the main tree with the change.
S=${S:?}
R=tosdump/reference
run() { T=$1; D=$2; TAB=$3; O=$4; REF=$5
  if [ "$T" = base ]; then TR="--tree $S/base-tree"; else TR=""; fi
  if [ -n "$REF" ]; then
    cargo xtask tosdump $D tosdump/cases/$TAB $TR --out $S/runs/$O --reference $R/$REF.BIN --scratch $S/xt-$T
  else
    cargo xtask tosdump $D tosdump/cases/$TAB $TR --out $S/runs/$O --scratch $S/xt-$T
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
