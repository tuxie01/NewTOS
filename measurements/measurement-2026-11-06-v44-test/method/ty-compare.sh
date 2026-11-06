#!/bin/bash
M=measurements/measurement-2026-11-06-v44-test
for p in "ty-head ty-fix" "ty-e1 ty-fix" "ty-e1 ty-head" "ty-e1 ty-base"; do set -- $p
 python3 -I tosdump/probes/vdprobe-compare.py $M/probe/$1 $M/probe/$2 > $M/analysis/$1-vs-$2.txt 2>&1; echo "$1 vs $2: $(tail -1 $M/analysis/$1-vs-$2.txt)"; done
