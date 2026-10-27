#!/usr/bin/env bash
# run.sh -- build this run's card with method/mkcard-call.sh, copy its manifest into card/, boot an APFS clone of it
# on QEMU raspi4b (-append serial=on) with the release kernel $S/kernel8.img until "[desk] desktop up", take one QMP
# screendump, stop QEMU, delete the clone. Base image and kernel stay in scratch $S (gitignored, sha256 in RUNINFO.TXT).
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; R="$HERE/.."; WS="$(cd "$R/../.." && pwd)"
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/32d8cb6c-2819-4e51-b178-dafc6b28ed0d/scratchpad/tool02
"$HERE/mkcard-call.sh" "$S/tool02.img" > "$S/mkcard.log" 2>&1 || { cat "$S/mkcard.log"; exit 1; }
grep -v 'is deprecated' "$S/mkcard.log" > "$R/checks/mkcard.log"
cp "$S/tool02.img.manifest" "$R/card/tool02.img.manifest"
cc -o "$S/qmpshot" "$WS/tools/qmpshot.c"
CARD="$S/run.img"; rm -f "$CARD"; cp -c "$S/tool02.img" "$CARD"      # APFS clone; the base stays untouched
LOG="$R/serial/boot-serial.log"; rm -f "$LOG"
cd "$S"; Q="$(mktemp -d q.XXXX)"                                         # relative: UNIX socket path < 104 bytes
qemu-system-aarch64 -M raspi4b -no-reboot -display none -append serial=on -kernel "$S/kernel8.img" \
  -drive file="$CARD",if=sd,format=raw -serial file:"$LOG" -qmp unix:"$Q/qmp.sock",server=on,wait=off &
QP=$!
up=0; for i in $(seq 1 120); do sleep 1; grep -q '^\[desk\] desktop up' "$LOG" 2>/dev/null && { up=1; break; }; done
sleep 3                                                                   # let the desktop finish drawing
"$S/qmpshot" "$Q/qmp.sock" "$S/desktop.ppm" && sips -s format png "$S/desktop.ppm" --out "$R/screendumps/desktop.png" >/dev/null
kill -9 $QP 2>/dev/null; wait $QP 2>/dev/null; rm -rf "$S/$Q" "$CARD" "$S/desktop.ppm"
echo "desktop_up=$up after ~${i}s; crash=$(grep -c '^\[crash\]' "$LOG") panic=$(grep -Ec 'PANIC|panicked|\[EXC\]' "$LOG")"
grep -E '^\[desk\] desktop up|MXPROBE' "$LOG"
