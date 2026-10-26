#!/usr/bin/env bash
# (A115: copied from measurements/measurement-2026-10-17-a112-fix/method/api-run.sh; output paths one level up, red/green -> base/new)
# api-run.sh <kernel8.img> <base.img> <tag> "<append>" "<end-regex>" [after-s] -- one headless QEMU raspi4b boot (one QEMU at a
# time) on a per-run APFS clone (cp -c) of the base card; serial -> ../serial/<tag>-serial.log. Waits (max TMO s, default
# 240) for <end-regex> in the serial log, then for "[desk] desktop up" (max 60 s) and <after-s> s more (default 5),
# takes a QMP screendump (../screendumps/<tag>.png via sips), quits QEMU, deletes the clone.
# Copied from measurement-2026-10-17-arch02/method/run.sh.
set -uo pipefail
K="$1"; BASE="$2"; TAG="$3"; APP="$4"; END="$5"; AFTER="${6:-5}"
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; SER="$HERE/../../serial"; SHOT="$HERE/../../screendumps"; mkdir -p "$SER" "$SHOT"
S="$(dirname "$BASE")"; RD="$(mktemp -d "$S/run-$TAG.XXXX")"; CARD="$RD/card.img"; LOG="$SER/$TAG-serial.log"
cp -c "$BASE" "$CARD"; rm -f "$LOG"
cd "$RD"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -append "$APP" -kernel "$K" \
  -drive file="$CARD",if=sd,format=raw -serial file:"$LOG" -qmp unix:qmp.sock,server=on,wait=off &
QPID=$!
for _ in $(seq 1 "${TMO:-240}"); do grep -Eq "$END" "$LOG" 2>/dev/null && break; kill -0 $QPID 2>/dev/null || break; sleep 1; done
for _ in $(seq 1 60); do grep -q "\[desk\] desktop up" "$LOG" 2>/dev/null && break; kill -0 $QPID 2>/dev/null || break; sleep 1; done
sleep "$AFTER"
python3 -I - qmp.sock "$RD/shot.ppm" <<'PY'
import json, socket, sys
s = socket.socket(socket.AF_UNIX); s.connect(sys.argv[1]); f = s.makefile("rw")
def cmd(c, **a):
    f.write(json.dumps({"execute": c, **({"arguments": a} if a else {})}) + "\n"); f.flush()
    while True:
        r = json.loads(f.readline())
        if "return" in r or "error" in r: return r
f.readline(); cmd("qmp_capabilities"); print("screendump", cmd("screendump", filename=sys.argv[2])); cmd("quit")
PY
for _ in $(seq 1 20); do kill -0 $QPID 2>/dev/null || break; sleep 0.5; done; kill -9 $QPID 2>/dev/null; wait $QPID 2>/dev/null
sips -s format png "$RD/shot.ppm" --out "$SHOT/$TAG.png" >/dev/null 2>&1 || echo "no screendump"
cd "$HERE"; rm -rf "$RD"
echo "run $TAG: $(grep -c . "$LOG") serial lines; end: $(grep -Ec "$END" "$LOG"); desktop up: $(grep -c '\[desk\] desktop up' "$LOG"); EXC/PANIC: $(grep -Ec '\[EXC\]|PANIC|panicked' "$LOG")"
