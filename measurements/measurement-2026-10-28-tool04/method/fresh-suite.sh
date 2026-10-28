#!/bin/sh
# TOOL04: the full host suite (`cargo test` over tos-api, tos-types, xtask --
# the command `cargo xtask test` runs) in a new, empty CARGO_TARGET_DIR
# (the state after `cargo clean`), so every test binary and every doctest is
# compiled, linked and launched for the first time ("cold"). Then
# tos-api/src/lib.rs is touched (mtime only) and the suite runs again in the
# same, now populated, target dir ("relink": what an edit in tos-api costs).
# Usage: fresh-suite.sh <repo-root> <outdir> <target-parent> <label> [VAR=value...]
# Writes <outdir>/fresh-<label>-{cold,relink}.log (cargo's output) and appends
# one summary line per phase to <outdir>/fresh-suite.txt. The target dir is
# removed afterwards.
R=$1; OUT=$2; SCR=$3; label=$4; shift 4
T=$SCR/fresh-$label
rm -rf "$T"
for phase in cold relink; do
  [ "$phase" = relink ] && touch "$R/tos-api/src/lib.rs"
  log=$OUT/fresh-$label-$phase.log
  t0=$(python3 -I -c 'import time; print(time.time())')
  (cd "$R" && env CARGO_TARGET_DIR="$T" "$@" cargo test -p tos-api -p tos-types -p xtask > "$log" 2>&1)
  rc=$?
  wall=$(python3 -I -c "import time; print('%.1f' % (time.time() - $t0))")
  fin=$(/usr/bin/grep -m1 'Finished `test`' "$log" | sed 's/.* in //')
  n=$(ls -f "$T/debug/deps" | wc -l | tr -d ' ')
  no=$(ls -f "$T/debug/deps" | /usr/bin/grep -c '\.o$')
  du=$(du -sk "$T/debug/deps" | cut -f1)
  sum=$(/usr/bin/grep '^test result:' "$log" | awk '{p+=$4; f+=$6; i+=$8} END{print p "/" f "/" i}')
  printf '%s\t%s\t[%s]\trc=%s\twall=%ss\tbuild=%s\tpassed/failed/ignored=%s\tdeps_entries=%s\tdeps_o=%s\tdeps_kib=%s\n' \
    "$label" "$phase" "$*" "$rc" "$wall" "$fin" "$sum" "$n" "$no" "$du" | tee -a "$OUT/fresh-suite.txt"
done
rm -rf "$T"
