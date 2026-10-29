#!/usr/bin/env bash
# run-inj.sh <kernel> <scratch> <card> <tag> STEP... -- one headless QEMU raspi4b run (-append serial=on, -no-reboot) of the
# injector kernel on an APFS clone of <scratch>/cards/<card>.img; serial -> <run>/serial/<tag>-serial.log; screendumps
# keyed to serial lines (mdrive.py STEP 'REGEX@N=DELAY:name.png') -> <run>/screendumps/<tag>-<name>. The shape of
# measurements/measurement-2026-10-27-mem12-acceptance/method/run.sh. The clone's sha256 after the run goes to
# <run>/checks/post-run.sha256; then the clone is deleted.
set -uo pipefail
K="$1"; S="$2"; CARD="$3"; TAG="$4"; shift 4
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; R="$HERE/.."
mkdir -p "$R/serial" "$R/screendumps" "$R/checks"
IMG="$S/run-$TAG.img"; cp -c "$S/cards/$CARD.img" "$IMG"; LOG="$R/serial/$TAG-serial.log"; rm -f "$LOG"
cd "$S"; Q="$(mktemp -d q.XXXX)"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -append serial=on -kernel "$K" \
  -drive file="$IMG",if=sd,format=raw -serial file:"$LOG" -qmp unix:"$Q/qmp.sock",server=on,wait=off &
QP=$!
python3 "$HERE/mdrive.py" "$Q/qmp.sock" "$R/screendumps" "$TAG" "$LOG" --timeout 200 --quit-delay 3 "$@"; RC=$?
for _ in $(seq 1 20); do kill -0 "$QP" 2>/dev/null || break; sleep 0.5; done
kill -9 $QP 2>/dev/null; wait $QP 2>/dev/null
echo "$(shasum -a 256 "$IMG" | awk '{print $1}')  $TAG (clone of $CARD.img after the run)" >> "$R/checks/post-run.sha256"
rm -rf "$S/$Q" "$IMG"
echo "$TAG: rc=$RC crash=$(grep -c '^\[crash\]' "$LOG") panic=$(grep -Ec 'PANIC|panicked|\[EXC\]' "$LOG") desk=$(grep -c 'desktop up' "$LOG") reset=$(grep -c '^\[reset\]' "$LOG") hook=$(grep -c '^\[hook\]' "$LOG")"
