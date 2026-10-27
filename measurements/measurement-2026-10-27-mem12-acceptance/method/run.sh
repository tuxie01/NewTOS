#!/usr/bin/env bash
# run.sh <kernel> <card> <tag> <secs> [STEP...] -- one headless QEMU raspi4b run (-append serial=on, -no-reboot) on an
# APFS clone of $S/cards/<card>.img; serial -> <run>/serial/<tag>-serial.log. With STEPs (mdrive.py: 'REGEX@N=DELAY:name.png')
# screendumps keyed to serial lines go to <run>/screendumps/<tag>-<name>; without, the run lasts <secs>. QEMU is killed
# afterwards (or has exited by itself on a guest reset). The clone's sha256 after the run goes to <run>/checks/post-run.sha256
# (TOOL03: a run that writes to its image records it); XFILES="A.BIN B.LOG" are copied out of the clone first; then the
# clone is deleted.
set -uo pipefail
K="$1"; CARD="$2"; TAG="$3"; SECS="$4"; shift 4
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; R="$HERE/.."
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/32d8cb6c-2819-4e51-b178-dafc6b28ed0d/scratchpad/mem12/acc
mkdir -p "$R/serial" "$R/screendumps" "$R/checks" "$R/out"
IMG="$S/run-$TAG.img"; cp -c "$S/cards/$CARD.img" "$IMG"; LOG="$R/serial/$TAG-serial.log"; rm -f "$LOG"
cd "$S"; Q="$(mktemp -d q.XXXX)"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -append serial=on -kernel "$K" \
  -drive file="$IMG",if=sd,format=raw -serial file:"$LOG" -qmp unix:"$Q/qmp.sock",server=on,wait=off &
QP=$!
if [ $# -gt 0 ]; then
  python3 "$HERE/mdrive.py" "$Q/qmp.sock" "$R/screendumps" "$TAG" "$LOG" --timeout 200 --quit-delay 3 "$@"; RC=$?
else
  for _ in $(seq 1 "$SECS"); do sleep 1; kill -0 $QP 2>/dev/null || break; done; RC=0
fi
for _ in $(seq 1 20); do kill -0 "$QP" 2>/dev/null || break; sleep 0.5; done
kill -9 $QP 2>/dev/null; wait $QP 2>/dev/null
for f in ${XFILES:-}; do mcopy -n -i "$IMG@@32256" "::/$f" "$R/out/$TAG-$f" 2>/dev/null; done
echo "$(shasum -a 256 "$IMG" | awk '{print $1}')  $TAG (clone of $CARD.img after the run)" >> "$R/checks/post-run.sha256"
rm -rf "$S/$Q" "$IMG"
echo "$TAG: rc=$RC crash=$(grep -c '^\[crash\]' "$LOG") panic=$(grep -Ec 'PANIC|panicked|\[EXC\]' "$LOG") desk=$(grep -c 'desktop up' "$LOG") reset=$(grep -c '^\[reset\]' "$LOG") lines=$(grep -c . "$LOG")"
