#!/bin/sh
# V38: cargo xtask test (full) in the base and fix scratch trees, then in the main tree
# (which also holds the uncommitted V53 work). Totals = sum of all "test result" lines.
S=${S:?set S to the session scratchpad/v38}
M=/Volumes/Daten/Development/PiRusTOS/measurements/measurement-2026-11-07-v38
tot() { /usr/bin/grep -E "^test result" "$1" | awk '{p+=$4; f+=$6; i+=$8} END {print p" passed / "f" failed / "i" ignored"}'; }
for t in base fix; do
  (cd "$S/$t" && cargo xtask test) > "$S/xtask-test-$t.log" 2>&1; echo "$t exit $? $(tot $S/xtask-test-$t.log)"
done
(cd /Volumes/Daten/Development/PiRusTOS && cargo xtask test) > "$S/xtask-test-main.log" 2>&1; echo "main exit $? $(tot $S/xtask-test-main.log)"
