#!/usr/bin/env bash
# aes-run.sh <kernel8.img> <scratch> <out-prefix> <shot-delays...> -- XBIOS01 AES retest:
# a card from tools/mkcard.sh with method/aes-newdesk.inf (drive C: icon at grid 0,0,
# trash), booted as an APFS clone on QEMU raspi4b (-append serial=on) until
# "[desk] desktop up"; then one QMP screendump per delay (seconds after desktop up,
# cumulative), stop, delete the clone. With an injector kernel armed `desktop`
# (tools/inject-arm.py) the key/mouse script plays meanwhile.
set -uo pipefail
K=$1; S=$2; OUT=$3; shift 3
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; WS="$(cd "$HERE/../../.." && pwd)"
(cd "$WS" && tools/mkcard.sh "$S/aes.img" "$HERE/aes-newdesk.inf") > "$S/mkcard.log" 2>&1 || { cat "$S/mkcard.log"; exit 1; }
cp "$S/aes.img.manifest" "$OUT.card.img.manifest"
cc -o "$S/qmpshot" "$WS/tools/qmpshot.c"
CARD="$S/run.img"; rm -f "$CARD"; cp -c "$S/aes.img" "$CARD"
LOG="$OUT.serial.txt"; rm -f "$LOG"
cd "$S"; Q="$(mktemp -d q.XXXX)"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -append serial=on -kernel "$K" \
  -drive file="$CARD",if=sd,format=raw -serial file:"$LOG" -qmp unix:"$Q/qmp.sock",server=on,wait=off &
QP=$!
up=0; for i in $(seq 1 120); do sleep 1; grep -q '^\[desk\] desktop up' "$LOG" 2>/dev/null && { up=1; break; }; done
n=0; for d in "$@"; do sleep "$d"; n=$((n+1))
  "$S/qmpshot" "$Q/qmp.sock" "$S/shot.ppm" && sips -s format png "$S/shot.ppm" --out "$OUT.shot$n.png" >/dev/null; done
kill -9 $QP 2>/dev/null; wait $QP 2>/dev/null; rm -rf "$S/$Q" "$CARD" "$S/shot.ppm"
echo "desktop_up=$up after ~${i}s; crash=$(grep -c '^\[crash\]' "$LOG") panic=$(grep -Ec 'PANIC|panicked|\[EXC\]' "$LOG") iorec_warn=$(grep -c '\[iorec\] warning' "$LOG")"
