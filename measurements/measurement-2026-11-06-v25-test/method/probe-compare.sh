#!/bin/bash
A=measurements/measurement-2026-11-06-v25-test; P=$A/probe; C=tosdump/probes/vdprobe-compare.py
for pair in "e1 n-head" "e1 n-base" "e1 e2" "n-head n-head-2" "n-base n-head"; do
  set -- $pair; python3 -I $C $P/$1 $P/$2 > $A/analysis/probe-$1-vs-$2.txt 2>&1
  echo "== $1 vs $2 (rc $?)"; tail -14 $A/analysis/probe-$1-vs-$2.txt
done
