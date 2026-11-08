#!/bin/bash
# probe + x50 extra + gui phases, strictly one QEMU at a time. S = scratch (required).
set -u
S=${S:?}
export S
M=$(cd "$(dirname "$0")/.." && pwd)
bash "$M/method/accept.sh" probe
export VDPROBE_SCRATCH=$S/vp; mkdir -p "$VDPROBE_SCRATCH"
mkdir -p "$M/probe-x50"
for n in base:base chg:chg; do
  bash "$M/method/run-x50.sh" newtos "$M/probe-x50/${n%%:*}" "$S/k/${n#*:}-rpi4-release.img" > "$M/probe-x50/${n%%:*}.stdout" 2>&1
done
python3 -I "$M/../../tosdump/probes/vdprobe-compare.py" "$M/probe-x50/base" "$M/probe-x50/chg" > "$M/probe-x50/compare-base-chg.txt" 2>&1
cmp "$M/probe-x50/base/VDPROBE.BIN" "$M/probe-x50/chg/VDPROBE.BIN" > "$M/probe-x50/cmp-bin.txt" 2>&1; echo "cmp BIN exit $?" >> "$M/probe-x50/cmp-bin.txt"
diff "$M/probe-x50/base/VDPROBE.LOG" "$M/probe-x50/chg/VDPROBE.LOG" > "$M/probe-x50/log.diff"; echo "diff LOG exit $?" >> "$M/probe-x50/log.diff"
bash "$M/method/accept.sh" gui
echo ALLDONE
