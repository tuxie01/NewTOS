#!/bin/bash
# test-summary.sh <cargo-xtask-test output> <out> <label> : keep the target lines and the totals.
grep -E "^     Running|^   Doc-tests|^test result|^rc " "$1" > "$2"
grep "^test result" "$1" | awk -v t="$3" '{p+=$4; f+=$6; i+=$8} END {print t": passed",p,"failed",f,"ignored",i,"("NR" test result lines)"}' >> "$2"
tail -1 "$2"
