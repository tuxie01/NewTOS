#!/bin/sh
# V21 acceptance: tosdump runs, head (497773f9) and base (2042cc8f); run from the main tree, sequential.
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/15767204-8d89-43e0-aae0-d8149a16a17d/scratchpad/v23t
M=measurements/measurement-2026-11-06-v23-test
R=tosdump/reference
run() { T=$1; D=$2; TAB=$3; O=$4; REF=$5
  if [ -n "$REF" ]; then
    cargo xtask tosdump $D tosdump/cases/$TAB --tree $S/$T --out $M/runs/$O --reference $R/$REF.BIN --scratch $S/xt-$T
  else
    cargo xtask tosdump $D tosdump/cases/$TAB --tree $S/$T --out $M/runs/$O --scratch $S/xt-$T
  fi; }
set -x
for T in head base; do
  run $T vdi vdi-v18.tsv t1-$T-v18 EMUTOS-1a7484ab-VDI-v18-falcontc
  run $T vdi vdi-all.tsv t3-$T-all EMUTOS-1a7484ab-VDI-all-falcontc
  run $T rast vdi-raster.tsv t3-$T-raster EMUTOS-1a7484ab-RASTER
  run $T rast vdi-raster-screen.tsv t3-$T-raster-screen EMUTOS-1a7484ab-RASTER-screen-falcontc
  run $T rast vdi-v18-raster.tsv t3-$T-v18-raster EMUTOS-1a7484ab-RASTER-v18-falcontc
  run $T aes aes-r2-wind.tsv t4-$T-r2 EMUTOS-1a7484ab-AES-r2wind
  run $T aes aes-r3-wind-state.tsv t4-$T-r3 EMUTOS-1a7484ab-AES-r3
  run $T aes aes-r4-subfunctions.tsv t4-$T-r4
done
echo finished
