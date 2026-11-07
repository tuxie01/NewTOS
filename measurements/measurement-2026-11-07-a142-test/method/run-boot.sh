#!/usr/bin/env bash
# A142-test: boot a kernel to the desktop (or an autorun program) and take one QMP screendump after <wait> s.
#   run-boot.sh <out-dir> rpi4|virt <kernel> <wait-s> <inf-text|-> [dest=file ...]
# rpi4: card by tools/mkcard.sh (APFS clone, deleted after), -append serial=on. virt: no card (qemu-virt has no C:), kernel ELF, ramfb.
set -euo pipefail
REPO=/Volumes/Daten/Development/PiRusTOS; cd "$REPO"
BASE=${A142_SCRATCH:?}
OUT=$1; BOARD=$2; KERNEL=$3; WAIT=$4; INF=$5; shift 5
[ ! -e "$OUT" ] || { echo "out exists" >&2; exit 1; }
mkdir -p "$OUT/shots" "$BASE"; OUT=$(cd "$OUT" && pwd)
SCR=$(mktemp -d "$BASE/run.XXXXXX"); trap 'rm -rf "$SCR"' EXIT
sha() { shasum -a 256 "$1" | cut -d' ' -f1; }
: > "$OUT/BOOT-SERIAL.txt"
{ echo "a142-test boot run $BOARD ($(date -u +%Y-%m-%dT%H:%M:%SZ))"; echo "kernel $(sha "$KERNEL")"; echo "inf $INF"; } > "$OUT/run.txt"
cd "$SCR"
if [ "$BOARD" = rpi4 ]; then
  if [ "$INF" = "-" ]; then printf '#R 02\r\n' > "$SCR/NEWDESK.INF"; else printf '#R 02\r\n%b\r\n' "$INF" > "$SCR/NEWDESK.INF"; fi
  (cd "$REPO" && tools/mkcard.sh "$SCR/card.img" "$SCR/NEWDESK.INF" "$@" > "$SCR/mkcard.stdout" 2>&1) || { cat "$SCR/mkcard.stdout" >&2; exit 1; }
  cp "$SCR/card.img.manifest" "$OUT/card.img.manifest"; cp -c "$SCR/card.img" "$SCR/boot.img"
  echo "card $(sha "$SCR/card.img")" >> "$OUT/run.txt"
  qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$OUT/BOOT-SERIAL.txt" -append serial=on -kernel "$KERNEL" -drive file="$SCR/boot.img",if=sd,format=raw -qmp unix:qmp.sock,server=on,wait=off &
else
  qemu-system-aarch64 -M virt,gic-version=2 -cpu cortex-a72 -m 272M -device ramfb -display none -serial file:"$OUT/BOOT-SERIAL.txt" -no-reboot -kernel "$KERNEL" -qmp unix:qmp.sock,server=on,wait=off &
fi
pid=$!
for _ in $(seq 1 100); do [ -S qmp.sock ] && break; sleep 0.1; done
sleep "$WAIT"
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
