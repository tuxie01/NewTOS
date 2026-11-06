#!/bin/bash
M=measurements/measurement-2026-11-06-v44; R=$M/runs; A=$M/analysis
for p in "r2 aes-r2-wind" "r3 aes-r3-wind-state" "r4 aes-r4-subfunctions"; do set -- $p
 tosdump/host/build/tosdiff tosdump/cases/$2.tsv $R/tab-head-$1/AES.BIN $R/tab-fix-$1/AES.BIN > $A/aes-$1-before-vs-fix.txt 2>&1; echo "$1 rc $?"; tail -1 $A/aes-$1-before-vs-fix.txt
done
