#!/bin/sh
# V56: tosdump tables, base and fix tree (method/trees.sh), one cargo xtask tosdump call each (as V53's tables.sh).
S=${S:?}
cd /Volumes/Daten/Development/PiRusTOS
R=tosdump/reference
run() { T=$1; D=$2; TAB=$3; O=$4; REF=$5
  if [ -n "$REF" ]; then
    cargo xtask tosdump $D tosdump/cases/$TAB --tree $S/t/$T --out $S/runs/$O --reference $R/$REF.BIN --scratch $S/xt-$T
  else
    cargo xtask tosdump $D tosdump/cases/$TAB --tree $S/t/$T --out $S/runs/$O --scratch $S/xt-$T
  fi; }
set -x
mkdir -p $S/runs
for T in fix base; do
  run $T vdi vdi-all.tsv tab-$T-all EMUTOS-1a7484ab-VDI-all-falcontc
  run $T aes aes-r2-wind.tsv tab-$T-r2 EMUTOS-1a7484ab-AES-r2wind
  run $T aes aes-r3-wind-state.tsv tab-$T-r3 EMUTOS-1a7484ab-AES-r3
  run $T aes aes-r4-subfunctions.tsv tab-$T-r4
done
echo finished
