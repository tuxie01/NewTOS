#!/bin/sh
# V44: the tosdump tables and the vdprobe on the state before the fix (prefix = tree ac6ca0e5) and with it
# (fix = tree ac6ca0e5 + the V44 text.rs). Run from the main tree. Byte compare: method/cmp-bins.sh.
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/7fba4f48-e8f1-4a5e-8c58-696d17828400/scratchpad/v44t
M=measurements/measurement-2026-11-06-v44-test
R=tosdump/reference
run() { T=$1; D=$2; TAB=$3; O=$4; REF=$5
  if [ -n "$REF" ]; then
    cargo xtask tosdump $D tosdump/cases/$TAB --tree $S/$T --out $M/runs/$O --reference $R/$REF.BIN --scratch $S/xt-$T
  else
    cargo xtask tosdump $D tosdump/cases/$TAB --tree $S/$T --out $M/runs/$O --scratch $S/xt-$T
  fi; }
set -x
for T in fix head; do
  run $T vdi vdi-v18-t2.tsv tab-$T-v18t2 EMUTOS-1a7484ab-VDI-v18t2-falcontc
  run $T vdi vdi-v18.tsv tab-$T-v18 EMUTOS-1a7484ab-VDI-v18-falcontc
  run $T vdi vdi-all.tsv tab-$T-all EMUTOS-1a7484ab-VDI-all-falcontc
  run $T vdi vdi-r3.tsv tab-$T-vdi-r3 EMUTOS-1a7484ab-VDI-r3
  run $T rast vdi-v18-t2-raster.tsv tab-$T-v18t2-raster EMUTOS-1a7484ab-RASTER-v18t2-falcontc
  run $T rast vdi-raster.tsv tab-$T-raster EMUTOS-1a7484ab-RASTER
  run $T rast vdi-raster-screen.tsv tab-$T-raster-screen EMUTOS-1a7484ab-RASTER-screen-falcontc
  run $T rast vdi-v18-raster.tsv tab-$T-v18-raster EMUTOS-1a7484ab-RASTER-v18-falcontc
  run $T aes aes-r2-wind.tsv tab-$T-r2 EMUTOS-1a7484ab-AES-r2wind
  run $T aes aes-r3-wind-state.tsv tab-$T-r3 EMUTOS-1a7484ab-AES-r3
  run $T aes aes-r4-subfunctions.tsv tab-$T-r4
done
export VDPROBE_SCRATCH=$S/vp
mkdir -p $S/vp $M/probe
P=tosdump/probes
$P/run-vdprobe.sh newtos $M/probe/n-fix $S/kernels/fix.img > $M/probe/n-fix.stdout 2>&1
$P/run-vdprobe.sh newtos $M/probe/n-head $S/kernels/head.img > $M/probe/n-head.stdout 2>&1
$P/run-vdprobe.sh emutos falcontc $M/probe/e1 > $M/probe/e1.stdout 2>&1
echo finished
