#!/bin/sh
# V32 acceptance: tosdump runs, head 877a14b9 and base 587ddbd5. Run from the main tree.
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/7fba4f48-e8f1-4a5e-8c58-696d17828400/scratchpad/v33t
M=measurements/measurement-2026-11-06-v33-test
R=tosdump/reference
run() { T=$1; D=$2; TAB=$3; O=$4; REF=$5
  if [ -n "$REF" ]; then
    cargo xtask tosdump $D tosdump/cases/$TAB --tree $S/$T --out $M/runs/$O --reference $R/$REF.BIN --scratch $S/xt-$T
  else
    cargo xtask tosdump $D tosdump/cases/$TAB --tree $S/$T --out $M/runs/$O --scratch $S/xt-$T
  fi; }
set -x
for T in head base; do
  run $T vdi vdi-v18-t2.tsv t2-$T-v18t2 EMUTOS-1a7484ab-VDI-v18t2-falcontc
  run $T vdi vdi-v18.tsv t4-$T-v18 EMUTOS-1a7484ab-VDI-v18-falcontc
  run $T vdi vdi-all.tsv t4-$T-all EMUTOS-1a7484ab-VDI-all-falcontc
  run $T vdi vdi-r3.tsv t4-$T-vdi-r3 EMUTOS-1a7484ab-VDI-r3
  run $T rast vdi-v18-t2-raster.tsv t2-$T-v18t2-raster EMUTOS-1a7484ab-RASTER-v18t2-falcontc
  run $T rast vdi-raster.tsv t4-$T-raster EMUTOS-1a7484ab-RASTER
  run $T rast vdi-raster-screen.tsv t4-$T-raster-screen EMUTOS-1a7484ab-RASTER-screen-falcontc
  run $T rast vdi-v18-raster.tsv t4-$T-v18-raster EMUTOS-1a7484ab-RASTER-v18-falcontc
  run $T aes aes-r2-wind.tsv t5-$T-r2 EMUTOS-1a7484ab-AES-r2wind
  run $T aes aes-r3-wind-state.tsv t5-$T-r3 EMUTOS-1a7484ab-AES-r3
  run $T aes aes-r4-subfunctions.tsv t5-$T-r4
done
echo finished
