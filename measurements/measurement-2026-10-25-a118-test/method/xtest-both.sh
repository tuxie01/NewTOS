#!/usr/bin/env bash
# xtest-both.sh -- A118: `cargo xtask test` again in the two lint trees of lint-both.sh, after the gitignored inputs of the rsc
# tests are linked correctly (lint-both.sh's first run linked references/ into the tracked references/ directory and missed
# the cicons.rsc directory: 5 xtask rsc tests failed on BOTH sides for that reason alone, checks/lint-*-summary.txt).
# Totals -> ../checks/xtest-<side>.txt.
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; . "$HERE/reg/env.sh"
for side in ${SIDES:-base new}; do
  T="$S/trees/lint-$side"
  rm -f "$T/references/references"; ln -sfn "$WS/references/emutos" "$T/references/emutos"
  mkdir -p "$T/sdk/apps/teradesk"; ln -sf "$WS/sdk/apps/teradesk/cicons.rsc" "$T/sdk/apps/teradesk/cicons.rsc"
  # lint-both.sh copied the A118 host test into the base tree to show it red there; the base suite runs without it.
  [ $side = base ] && rm -f "$T/tos-api/tests/aes_a118_lock_takers.rs"
  (cd "$T" && cargo xtask test) > "$S/lint-$side/xtask-test-2.log" 2>&1; rc=$?
  { echo "cargo xtask test ($side) exit $rc"
    grep -E "^test result:" "$S/lint-$side/xtask-test-2.log" | awk '{p+=$4; f+=$6; i+=$8} END {print "binaries " NR " passed " p " failed " f " ignored " i}'
    grep -E "^test .* FAILED$" "$S/lint-$side/xtask-test-2.log"; } > "$HERE/../checks/xtest-$side.txt"
done
echo XTEST-BOTH DONE
