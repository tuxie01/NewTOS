#!/bin/bash
R=measurements/measurement-2026-11-06-v33-test/runs; A=measurements/measurement-2026-11-06-v33-test/analysis
for p in "r2 aes-r2-wind" "r3 aes-r3-wind-state" "r4 aes-r4-subfunctions"; do set -- $p
 tosdump/host/build/tosdiff tosdump/cases/$2.tsv $R/t5-base-$1/AES.BIN $R/t5-head-$1/AES.BIN > $A/aes-$1-base-vs-head.txt 2>&1; echo "$1 rc $?"; tail -1 $A/aes-$1-base-vs-head.txt
done
