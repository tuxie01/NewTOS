#!/bin/bash
# rerun.sh : the runs that failed or were invalid in accept.sh, repeated one at a time (RUNINFO "Reruns").
#  - shprobe-desk base: mkcard's hdiutil unmount was refused by macOS ("Simulators are still shutting down").
#  - boot-rpi4-{base,chg}-prof: invalid (accept.sh was edited while a run was reading it; see RUNINFO).
set -u
W=/Volumes/Daten/Development/PiRusTOS/.claude/worktrees/agent-ac0a43815a49627e8
S=${S:?}; export S
M=$W/measurements/measurement-2026-11-07-v49
G=$M/runs/gui
export SHPROBE_SCRATCH=$S/sh
rm -rf "$G/shprobe-desk-base"
"$W/tosdump/probes/run-shprobe.sh" newtos-desk "$G/shprobe-desk-base" "$S/k/base-rpi4-release.img" > "$G/shprobe-desk-base.stdout" 2>&1
n=0; d=0; for p in "$G/shprobe-desk-base"/shots/*.png; do n=$((n+1)); cmp -s "$p" "$G/shprobe-desk-chg/shots/$(basename "$p")" || d=$((d+1)); done
echo "shprobe-desk rerun: $n shots base, $(ls "$G"/shprobe-desk-chg/shots/*.png | wc -l | tr -d ' ') chg, $d differ | exc/panic base $(grep -a -ciE 'exception|panic' "$G/shprobe-desk-base/BOOT-SERIAL.txt")" | tee -a "$M/analysis/shots-compare.txt"
for K in base-prof chg-prof; do
  rm -rf "$M/runs/boot-rpi4-$K"
  bash "$M/method/pi-boot.sh" "$S/k/$K-rpi4-release.img" "$M/runs/boot-rpi4-$K"
done
