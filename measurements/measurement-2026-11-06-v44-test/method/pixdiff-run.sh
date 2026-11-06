#!/bin/bash
R=measurements/measurement-2026-11-06-v44-test/runs/gui
for p in "slprobe-head slprobe-fix" "slprobe-head slprobe2-fix" "slprobe-head slprobe3-fix" "slprobe-fix slprobe3-fix"; do set -- $p; echo "== $1 vs $2"
 python3 measurements/measurement-2026-11-06-v44-test/method/pixdiff.py $R/$1 $R/$2 044 067 071 094 098 121 125 148 152; done
