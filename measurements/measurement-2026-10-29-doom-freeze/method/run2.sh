#!/usr/bin/env bash
# run.sh <kernel> <card> <tag> [extra mdrive steps...] -- headless QEMU raspi4b (serial=on), APFS clone of the card,
# gdbstub on tcp 1234+N kept open after the steps when KEEP=1; screendumps keyed to injector feed lines.
set -uo pipefail
K="$1"; CARD="$2"; TAG="$3"; shift 3
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/45e8e806-7ba7-4edc-ace3-d4dd625a7f8d/scratchpad/doomfreeze
OUT="$S/out/$TAG"; mkdir -p "$OUT"
IMG="$S/run-$TAG.img"; cp -c "$CARD" "$IMG"; LOG="$OUT/serial.log"; rm -f "$LOG"
cd "$S"; Q="q-$TAG"; mkdir -p "$Q"; rm -f "$Q/qmp.sock"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -append serial=on -kernel "$K" \
  -drive file="$IMG",if=sd,format=raw -serial file:"$LOG" -qmp unix:"$Q/qmp.sock",server=on,wait=off \
  -gdb tcp::${GDBPORT:-1234} &
QP=$!; echo $QP > "$Q/pid"
python3 /Volumes/Daten/Development/PiRusTOS/measurements/measurement-2026-10-29-bios15/method/mdrive.py "$Q/qmp.sock" "$OUT" "$TAG" "$LOG" --timeout 200 --quit-delay ${QD:-3} \
  'DOOM: keys@1=2:start.png' 'feed 0x81@1=1:menu.png' 'feed 0xd0@12=1:d12.png' 'feed 0xd0@20=1:d20.png' \
  'feed 0xd0@30=1:d30.png' 'feed 0xd0@31=1:held.png' 'feed 0xd0@37=2:mouse.png' 'feed 0x81@2=3:end.png' "$@"; RC=$?
for _ in $(seq 1 20); do kill -0 "$QP" 2>/dev/null || break; sleep 0.5; done
kill -9 $QP 2>/dev/null; wait $QP 2>/dev/null
rm -rf "$S/$Q" "$IMG"
echo "$TAG: rc=$RC crash=$(grep -c '^\[crash\]' "$LOG") panic=$(grep -Ec 'PANIC|panicked|\[EXC\]' "$LOG") hook=$(grep -c '^\[hook\]' "$LOG") iorec=$(grep -c '^\[iorec\]' "$LOG")"
