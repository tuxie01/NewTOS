#!/bin/bash
# V51: rerun of the two gui runs whose screendump client could not connect to QEMU's QMP socket
# (ConnectionRefusedError right after the socket file appeared; 0 shots; the failed dirs are kept as
# runs/gui/*-qmpfail). Same commands as accept.sh phase_gui, same kernels and inputs; then the same
# comparison over all six gui pairs. Needs S (scratch of accept.sh, with k/ and APITEST.NCP).
set -u
W=/Volumes/Daten/Development/PiRusTOS/.claude/worktrees/agent-a76fdd16f4fbe1958
S=${S:?}
M=$W/measurements/measurement-2026-11-07-v51-test
A=$M/analysis
P=$W/tosdump/probes
G=$M/runs/gui
export S SHPROBE_SCRATCH=$S/sh APITEST=$S/APITEST.NCP
"$P/run-shprobe.sh" newtos-desk "$G/shprobe-desk-chg" "$S/k/chg-rpi4-release.img" > "$G/shprobe-desk-chg.stdout" 2>&1
echo "$(date +%H:%M:%S) after rerun shprobe-desk-chg: qemu under scratch: $(pgrep -f "qemu-system-aarch64.*$S" || echo none)" >> "$A/qemu-leak-check.txt"
bash "$M/method/boot-console.sh" "$G/bc1080-chg" "$S/k/chg-rpi4-release.img" "-global bcm2835-fb.xres=1920 -global bcm2835-fb.yres=1080" | tail -2
echo "$(date +%H:%M:%S) after rerun bc1080-chg: qemu under scratch: $(pgrep -f "qemu-system-aarch64.*$S" || echo none)" >> "$A/qemu-leak-check.txt"
for r in shprobe shprobe-desk desk480 desk1080 bc480 bc1080; do
  b=$G/$r-base; f=$G/$r-chg; n=0; d=0; l=""
  for p in "$b"/shots/*.png; do n=$((n+1)); q=$f/shots/$(basename "$p"); cmp -s "$p" "$q" || { d=$((d+1)); l="$l $(basename "$p" .png)"; }; done
  echo "$r: $n shots base, $(ls "$f"/shots/*.png | wc -l | tr -d ' ') chg, $d differ:$l | exc/panic base $(grep -a -ciE 'exception|panic' "$b/BOOT-SERIAL.txt") chg $(grep -a -ciE 'exception|panic' "$f/BOOT-SERIAL.txt")"
done > "$A/shots-compare-rerun.txt"
cat "$A/shots-compare-rerun.txt"
