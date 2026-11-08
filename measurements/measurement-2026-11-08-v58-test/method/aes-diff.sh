#!/bin/bash
# aes-diff.sh : AES.BIN base vs change per table -- tosdiff (record compare) and the raw byte differences,
# grouped by offset within the 120-byte record (cmp -l prints the bytes in octal; V58: the kernel image is
# 4096 bytes larger).
W=/Volumes/Daten/Development/PiRusTOS/.claude/worktrees/agent-a88c9f69bef56fa4d
M=$W/measurements/measurement-2026-11-08-v58-test
TD=$W/tosdump/host/build/tosdiff
for tt in r2:aes-r2-wind r3:aes-r3-wind-state r4:aes-r4-subfunctions; do t=${tt%%:*}
  echo "== $t"
  "$TD" "$W/tosdump/cases/${tt#*:}.tsv" "$M/runs/tab-base-$t/AES.BIN" "$M/runs/tab-chg-$t/AES.BIN" | tail -3; echo "tosdiff exit ${PIPESTATUS[0]}"
  cmp -l "$M/runs/tab-base-$t/AES.BIN" "$M/runs/tab-chg-$t/AES.BIN" | awk '{print "offset-in-record", ($1-1)%120, "base", $2, "chg", $3}' | sort | uniq -c
done
