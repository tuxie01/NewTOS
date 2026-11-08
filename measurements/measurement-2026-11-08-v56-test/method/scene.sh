#!/bin/bash
# V56: the wheel scene (tosdump/probes/run-wvprobe.sh): armed kernels of both trees, then probe + control on each,
# then the probe on the pinned oracle. One QEMU / Hatari at a time; every QEMU is killed by its PID inside the script.
set -euo pipefail
cd /Volumes/Daten/Development/PiRusTOS
S=${S:?}; export WVPROBE_SCRATCH=$S/wv
for T in base fix; do tosdump/probes/run-wvprobe.sh kernel $S/t/$T $S/k/kinj-$T.img; done
for r in "base probe" "base control" "fix probe" "fix control"; do set -- $r
  tosdump/probes/run-wvprobe.sh newtos $S/runs/scene-$1-$2 $S/k/kinj-$1.img $2 > $S/runs/scene-$1-$2.stdout 2>&1
done
tosdump/probes/run-wvprobe.sh emutos $S/runs/oracle-e1
