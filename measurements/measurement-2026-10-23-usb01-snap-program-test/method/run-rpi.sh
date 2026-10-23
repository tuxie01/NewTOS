#!/usr/bin/env bash
# run-rpi.sh <kernel8.img> <card.img> <serial.log> "<end-regex>" <max-s> -- one headless QEMU raspi4b boot on the GIVEN
# card (the caller makes the per-test APFS clone and deletes it; two boots on one clone = "second run"), serial to the log
# (-append serial=on, SER01), screendump at the end into $SHOT (env, optional), then quit; fsck_msdos -n of the card into
# <serial.log>-fsck.txt and an mtools listing of C:\ and C:\USB into <serial.log>-card.txt.
# Adapted from measurements/measurement-2026-10-23-usb01-snap-test/method/run.sh.
set -uo pipefail
K="$1"; CARD="$2"; LOG="$3"; END="$4"; MAX="$5"
RD="$(mktemp -d "$(dirname "$CARD")/run.XXXX")"; rm -f "$LOG"
cd "$RD"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -append "serial=on" -kernel "$K" \
  -drive file="$CARD",if=sd,format=raw -serial file:"$LOG" -qmp unix:qmp.sock,server=on,wait=off &
QPID=$!; T0=$(date +%s)
qmp() { python3 -I - qmp.sock "$1" "${2:-}" <<'PY'
import json, socket, sys
s = socket.socket(socket.AF_UNIX); s.connect(sys.argv[1]); f = s.makefile("rw")
def cmd(c, **a):
    f.write(json.dumps({"execute": c, **({"arguments": a} if a else {})}) + "\n"); f.flush()
    while True:
        r = json.loads(f.readline())
        if "return" in r or "error" in r: return r
f.readline(); cmd("qmp_capabilities")
if sys.argv[2] == "shot": cmd("screendump", filename=sys.argv[3])
cmd("quit")
PY
}
while [ $(( $(date +%s) - T0 )) -lt "$MAX" ]; do grep -Eaq "$END" "$LOG" 2>/dev/null && break; kill -0 $QPID 2>/dev/null || break; sleep 1; done
sleep 3
if [ -n "${SHOT:-}" ]; then qmp shot "$RD/shot.ppm"; sips -s format png "$RD/shot.ppm" --out "$SHOT" >/dev/null 2>&1; else qmp quit; fi
for _ in $(seq 1 20); do kill -0 $QPID 2>/dev/null || break; sleep 0.5; done; kill -9 $QPID 2>/dev/null; wait $QPID 2>/dev/null
DEV=$(hdiutil attach -imagekey diskimage-class=CRawDiskImage -nomount "$CARD" | head -1 | awk '{print $1}')
fsck_msdos -n "${DEV}s1" > "$LOG-fsck.txt" 2>&1; echo "fsck rc=$?" >> "$LOG-fsck.txt"
hdiutil detach "$DEV" >/dev/null 2>&1 || true
{ for d in / /USB; do echo "== C:$d"; mdir -i "$CARD@@$((2048*512))" "::$d" 2>&1 || mdir -i "$CARD@@32256" "::$d" 2>&1; done; } > "$LOG-card.txt"
cd /; rm -rf "$RD"
echo "run $(basename "$LOG"): $(grep -ac . "$LOG") lines; end: $(grep -Eac "$END" "$LOG"); EXC/PANIC: $(grep -Eac '\[EXC\]|\[crash\]|PANIC|panicked' "$LOG")"
