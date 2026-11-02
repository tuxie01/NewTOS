#!/usr/bin/env bash
# run-cw.sh <scratch> <out-dir> <kernel8.img> "<append>" <max-seconds> <stop-regex>
# One QEMU raspi4b boot of an APFS clone (cp -c) of <scratch>/base.img
# (make-card.sh), -append "<append>", serial to <out-dir>/SERIAL.txt. Stops
# when <stop-regex> (grep -E) appears in the serial log, or after
# <max-seconds>. Records kernel, card before/after (the clone) and the end
# state in <out-dir>/run.txt; the clone is deleted afterwards.
set -euo pipefail
SCR=$1; OUT=$2; KERNEL=$3; APPEND=$4; MAXS=$5; STOP=$6
[ ! -e "$OUT" ] || { echo "out-dir exists: $OUT" >&2; exit 1; }
mkdir -p "$OUT"
OUT=$(cd "$OUT" && pwd)
sha() { shasum -a 256 "$1" | cut -d' ' -f1; }
CLONE="$SCR/boot-$$.img"
cp -c "$SCR/base.img" "$CLONE"
pid=
cleanup() {
    if [ -n "$pid" ]; then kill "$pid" 2>/dev/null || true; sleep 1; kill -9 "$pid" 2>/dev/null || true; fi
    rm -f "$CLONE"
}
trap cleanup EXIT
{
    echo "run $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "kernel    $(sha "$KERNEL") $KERNEL"
    echo "append    $APPEND"
    echo "card      $(sha "$CLONE") (APFS clone of base.img)"
} > "$OUT/run.txt"
: > "$OUT/SERIAL.txt"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$OUT/SERIAL.txt" \
    -append "$APPEND" -kernel "$KERNEL" -drive file="$CLONE",if=sd,format=raw &
pid=$!
t0=$(date +%s)
state=timeout
while [ $(( $(date +%s) - t0 )) -lt "$MAXS" ]; do
    sleep 3
    if grep -Eq "$STOP" "$OUT/SERIAL.txt"; then state=stop-pattern; break; fi
    kill -0 "$pid" 2>/dev/null || { state=qemu-exited; break; }
done
t1=$(date +%s)
sleep 2
kill "$pid" 2>/dev/null || true; sleep 1; kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
pid=
echo "qemu      $state after $((t1 - t0)) s; stopped" >> "$OUT/run.txt"
echo "card      after $(sha "$CLONE") (the booted clone)" >> "$OUT/run.txt"
