#!/usr/bin/env bash
# run.sh <kernel> <tag> <secs> [INF] -- boot base.img clone in QEMU raspi4b, serial=on, kill after <secs>
SC=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/32d8cb6c-2819-4e51-b178-dafc6b28ed0d/scratchpad/mem12
K="$1"; TAG="$2"; SECS="$3"; INF="${4:-}"
CARD="$SC/run-$TAG.img"; cp -c "$SC/base.img" "$CARD"
[ -n "$INF" ] && mcopy -o -i "$CARD@@32256" "$INF" ::/NEWDESK.INF
LOG="$SC/serial/$TAG.log"; rm -f "$LOG"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -append serial=on -kernel "$K" \
  -drive file="$CARD",if=sd,format=raw -serial file:"$LOG" -monitor none &
Q=$!; sleep "$SECS"; kill -9 $Q 2>/dev/null; wait $Q 2>/dev/null
rm -f "$CARD"
echo "$TAG: lines=$(grep -c . "$LOG") crash=$(grep -c '^\[crash\]' "$LOG") desk=$(grep -c 'desktop up' "$LOG") panic=$(grep -Ec 'PANIC|panicked|\[EXC\]' "$LOG")"
grep -E '^\[crash\]' "$LOG" | head -5
