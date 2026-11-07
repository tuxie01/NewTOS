#!/usr/bin/env bash
# A141-test: boot a card (tools/mkcard.sh, APFS clone, deleted after) in QEMU raspi4b, one QMP screendump after the wait.
#   boot-shot.sh <out-dir> <kernel8.img> <scratch> <newdesk-inf-text> [dest=file ...]
set -euo pipefail
OUT=$1; KERNEL=$2; BASE=$3; INF=$4; shift 4
REPO=/Volumes/Daten/Development/PiRusTOS
mkdir -p "$OUT/shots" "$BASE"; OUT=$(cd "$OUT" && pwd)
SCR=$(mktemp -d "$BASE/run.XXXXXX"); pid=
trap '[ -z "$pid" ] || kill "$pid" 2>/dev/null || true; rm -rf "$SCR"' EXIT
printf "$INF" > "$SCR/newdesk.inf"
cd "$REPO"
tools/mkcard.sh "$SCR/card.img" "$SCR/newdesk.inf" "$@" > "$SCR/mk.out" 2>&1 || { cat "$SCR/mk.out"; exit 1; }
cp "$SCR/card.img.manifest" "$OUT/"
{ echo "commit $(git rev-parse HEAD)"; echo "kernel $(shasum -a 256 "$KERNEL"|cut -d' ' -f1)"; echo "inf $INF"; } > "$OUT/run.txt"
cp -c "$SCR/card.img" "$SCR/boot.img"; : > "$OUT/BOOT-SERIAL.txt"; cd "$SCR"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$OUT/BOOT-SERIAL.txt" -append serial=on -kernel "$KERNEL" -drive file="$SCR/boot.img",if=sd,format=raw -qmp unix:qmp.sock,server=on,wait=off &
pid=$!; sleep "${WAIT:-30}"
python3 -I - qmp.sock "$OUT/shots/boot.png" <<'PY'
import json, socket, sys
s = socket.socket(socket.AF_UNIX); s.connect(sys.argv[1]); f = s.makefile("rw"); f.readline()
def cmd(c, a=None):
    f.write(json.dumps({"execute": c, **({"arguments": a} if a else {})}) + "\n"); f.flush()
    while True:
        r = json.loads(f.readline())
        if "return" in r or "error" in r: return r
cmd("qmp_capabilities"); print(cmd("screendump", {"filename": sys.argv[2], "format": "png"}))
PY
sleep 1; kill "$pid" 2>/dev/null || true; sleep 1; kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
grep -a -c -E "Panic|Bus Error|Bus error|exception" "$OUT/BOOT-SERIAL.txt" | sed 's/^/panic-lines /' >> "$OUT/run.txt" || true
cat "$OUT/run.txt"
