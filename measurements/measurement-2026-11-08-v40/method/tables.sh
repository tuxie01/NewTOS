#!/bin/sh
# V40: tosdump tables, base = $S/t/base (git archive of ad201a57), fix = $S/t/fix (base + the V40 files);
# both built in their own tree (--tree), never the main tree (a parallel worker's files are uncommitted there).
S=${S:?}
R=tosdump/reference
cd /Volumes/Daten/Development/PiRusTOS
run() { T=$1; D=$2; TAB=$3; O=$4; REF=$5
  if [ -n "$REF" ]; then
    cargo xtask tosdump $D tosdump/cases/$TAB --tree $S/t/$T --out $S/runs/$O --reference $R/$REF.BIN --scratch $S/xt-$T
  else
    cargo xtask tosdump $D tosdump/cases/$TAB --tree $S/t/$T --out $S/runs/$O --scratch $S/xt-$T
  fi; }
set -x
for T in fix base; do
  run $T vdi vdi-v18-t2.tsv tab-$T-v18t2 EMUTOS-1a7484ab-VDI-v18t2-falcontc
  run $T vdi vdi-all.tsv tab-$T-all EMUTOS-1a7484ab-VDI-all-falcontc
  run $T aes aes-r2-wind.tsv tab-$T-r2 EMUTOS-1a7484ab-AES-r2wind
  run $T aes aes-r3-wind-state.tsv tab-$T-r3 EMUTOS-1a7484ab-AES-r3
  run $T aes aes-r4-subfunctions.tsv tab-$T-r4
  rm -rf $S/xt-$T
done
echo finished
