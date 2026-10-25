#!/usr/bin/env bash
# (A115: copied from measurements/measurement-2026-10-17-a112-fix/method/api-cmp.sh; output paths one level up, red/green -> base/new)
# api-cmp.sh (copied from measurement-2026-10-17-arch02/method/cmp-api.sh; modes from $MODES, default on) -- APITEST red vs green per mode: the TOTAL line and the result lines ([OK]/[FAIL]/[SKIP] and "SKIP"),
# hex addresses normalised (0x...), compared line by line. Output to ../checks/api-base-vs-green.txt.
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; SER="$HERE/../../serial"; OUT="$HERE/../../checks/api-base-vs-new.txt"
: > "$OUT"
norm() { grep -E '\[(OK|FAIL|SKIP)\]|SKIP|TOTAL' "$1" | sed -E 's/0x[0-9a-fA-F]+/0x_/g'; }
for m in ${MODES:-on}; do
  r="$SER/api-base-$m-serial.log"; g="$SER/api-new-$m-serial.log"
  echo "mode $m: base $(grep TOTAL "$r" | tail -1 | tr -s ' ')" >> "$OUT"
  echo "mode $m: new  $(grep TOTAL "$g" | tail -1 | tr -s ' ')" >> "$OUT"
  echo "mode $m: result lines base $(norm "$r" | wc -l | tr -d ' ') new $(norm "$g" | wc -l | tr -d ' '), differing: $(diff <(norm "$r") <(norm "$g") | grep -c '^[<>]')" >> "$OUT"
  diff <(norm "$r") <(norm "$g") >> "$OUT"
  echo "mode $m: EXC/PANIC base $(grep -Ec '\[EXC\]|PANIC|panicked' "$r") new $(grep -Ec '\[EXC\]|PANIC|panicked' "$g")" >> "$OUT"
done
cat "$OUT"
