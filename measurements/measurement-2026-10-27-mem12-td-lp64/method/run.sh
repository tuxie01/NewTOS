#!/usr/bin/env bash
# run.sh <old|new> -- QEMU raspi4b (-append serial=on), APFS clone of $S/td-<v>.img, kernel8-probe.img;
# QMP screendumps keyed to the injector's feed lines of the td-shesc.script marks; clone deleted afterwards.
set -uo pipefail
V="$1"; HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; R="$HERE/.."
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/32d8cb6c-2819-4e51-b178-dafc6b28ed0d/scratchpad/mem12/cp1
CARD="$S/run-$V.img"; cp -c "$S/td-$V.img" "$CARD"; LOG="$R/serial/td-$V-serial.log"; rm -f "$LOG"
cd "$S"; Q="$(mktemp -d q.XXXX)"   # relative: a UNIX socket path must stay < 104 bytes
qemu-system-aarch64 -M raspi4b -no-reboot -display none -append serial=on -kernel "$S/kernel8-probe.img" \
  -drive file="$CARD",if=sd,format=raw -serial file:"$LOG" -qmp unix:"$Q/qmp.sock",server=on,wait=off &
QP=$!
python3 "$HERE/mdrive.py" "$Q/qmp.sock" "$R/screendumps" "td-$V" "$LOG" --timeout 200 \
  '\(\+716\)@1=1.5:open.png' '\(\+1072\)@1=1.5:shesc1.png' '\(\+1416\)@1=1.5:shesc2.png'
RC=$?; kill -9 $QP 2>/dev/null; wait $QP 2>/dev/null; rm -rf "$S/$Q" "$CARD"
echo "td-$V: rc=$RC crash=$(grep -c '^\[crash\]' "$LOG") panic=$(grep -Ec 'PANIC|panicked|\[EXC\]' "$LOG")"; grep '^\[m12\]' "$LOG"
