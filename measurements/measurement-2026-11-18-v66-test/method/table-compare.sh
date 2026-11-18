#!/bin/bash
# table-compare.sh -- per table: raw-byte differences of each config's BIN against main, and whether the tosdiff reports equal main's
R=/Volumes/Daten/Development/PiRusTOS/target-agents/agent-ab6a60009402c9c41/runs
for t in $(ls -d $R/main/*/ | xargs -n1 basename); do
  b=$(ls $R/main/$t/*.BIN 2>/dev/null | head -1); [ -z "$b" ] && { echo "$t: no BIN (main)"; continue; }
  line="$t:"
  for c in default v16 v32; do
    o=$R/$c/$t/$(basename $b)
    if [ -f "$o" ]; then n=$(cmp -l $b $o 2>/dev/null | wc -l | tr -d ' '); fo=$(cmp -l $b $o 2>/dev/null | awk '$1>32{c++} END{print c+0}'); else n=NOBIN; fo=NOBIN; fi
    rep=same; for f in $R/main/$t/tosdiff-*.txt; do g=$R/$c/$t/$(basename $f); diff -q <(sed 's#/Volumes[^ ]*##g' $f) <(sed 's#/Volumes[^ ]*##g' $g) >/dev/null 2>&1 || rep=DIFF; done
    line="$line $c: bytes_differ=$n (past 32B header: $fo) tosdiff_reports=$rep;"
  done
  echo "$line"
done
