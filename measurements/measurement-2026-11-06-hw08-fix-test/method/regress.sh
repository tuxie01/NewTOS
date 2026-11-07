#!/bin/bash
# HW08 test, item 6: tables and pictures, base (e575a0b3) against fix (16bfa004). Run from the repo root.
cd /Volumes/Daten/Development/PiRusTOS
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/7fba4f48-e8f1-4a5e-8c58-696d17828400/scratchpad/rg; T=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/7fba4f48-e8f1-4a5e-8c58-696d17828400/scratchpad/t; M=measurements/measurement-2026-11-06-hw08-fix-test
Rf=tosdump/reference; P=tosdump/probes; MM=$M/method
K=target/aarch64-unknown-none-softfloat/release/kernel8.img
export SHPROBE_SCRATCH=$S/sh SLPROBE_SCRATCH=$S/sl DESK_SCRATCH=$S/dk DESK_FILES=$S/files XDPROBE_SCRATCH=$S/xd
mkdir -p $S/sh $S/sl $S/dk $S/xd $M/results/gui $M/results/tables
set -x
for p in base:e575a0b3 fix:16bfa004; do N=${p%%:*}; C=${p#*:}
  $P/run-shprobe.sh kernel $C $S/k-sh-$N.img
  $P/run-slprobe.sh kernel $C $S/k-sl-$N.img
  $MM/desk-inj.sh kernel $C $MM/desk-mouse.txt $S/k-desk-$N.img | tail -1
done
for N in base fix; do
  cargo xtask tosdump vdi tosdump/cases/vdi-all.tsv --tree $T/$N --out $M/results/tables/vdi-all-$N --reference $Rf/EMUTOS-1a7484ab-VDI-all-falcontc.BIN --scratch $S/xt-$N
  cargo xtask tosdump aes tosdump/cases/aes-r2-wind.tsv --tree $T/$N --out $M/results/tables/r2-$N --reference $Rf/EMUTOS-1a7484ab-AES-r2wind.BIN --scratch $S/xt-$N
  cargo xtask tosdump aes tosdump/cases/aes-r3-wind-state.tsv --tree $T/$N --out $M/results/tables/r3-$N --reference $Rf/EMUTOS-1a7484ab-AES-r3.BIN --scratch $S/xt-$N
  cargo xtask tosdump aes tosdump/cases/aes-r4-subfunctions.tsv --tree $T/$N --out $M/results/tables/r4-$N --scratch $S/xt-$N
done
for N in base fix; do
  $P/run-shprobe.sh newtos $M/results/gui/shprobe-$N $S/k-sh-$N.img
  $P/run-shprobe.sh newtos-desk $M/results/gui/shprobe-desk-$N /private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/7fba4f48-e8f1-4a5e-8c58-696d17828400/scratchpad/k/$N-rpi4-rel.img
  $P/run-slprobe.sh newtos $M/results/gui/slprobe-$N $S/k-sl-$N.img
done
for N in base fix; do $P/run-slprobe.sh newtos $M/results/gui/slprobe2-$N $S/k-sl-$N.img; done
for N in base fix; do $MM/desk-inj.sh newtos $M/results/gui/desk480-$N $S/k-desk-$N.img $MM/desk-NEWDESK.INF $MM/desk-shots.txt | tail -3; done
for N in base fix; do QEMU_EXTRA="-global bcm2835-fb.xres=1920 -global bcm2835-fb.yres=1080" $MM/desk-inj.sh newtos $M/results/gui/desk1080-$N $S/k-desk-$N.img $MM/desk-NEWDESK.INF $MM/desk-shots.txt | tail -3; done
# FS31 controls: Fdelete of an open file, XDPROBE oracle vs newtos
$P/run-xdprobe.sh emutos $M/results/gui/xdprobe-emutos
for N in base fix; do
  $P/run-xdprobe.sh newtos $M/results/gui/xdprobe-$N /private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/7fba4f48-e8f1-4a5e-8c58-696d17828400/scratchpad/k/$N-rpi4-rel.img
done
echo REGRESS-DONE
