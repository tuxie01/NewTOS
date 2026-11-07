#!/usr/bin/env bash
# TASK/A141: the real HYP_VIEW.NAP on NewTOS in QEMU raspi4b; no input: NEWDESK.INF #Z autorun, HYP_VIEW.CFG DEFAULT= opens the file.
#   newtos-hypview.sh <out-dir> <kernel8.img> <HYP_VIEW.NAP> <hyp_view.rsc native> <test.hyp> <scratch>
# Card by tools/mkcard.sh, booted as an APFS clone with -append serial=on, deleted after. One QMP screendump after the wait.
set -euo pipefail
OUT=$1; KERNEL=$2; NAP=$3; RSC=$4; HYP=$5; BASE=$6
REPO=/Volumes/Daten/Development/PiRusTOS
[ ! -e "$OUT" ] || { echo "out exists: $OUT" >&2; exit 1; }
mkdir -p "$OUT/shots" "$BASE"; OUT=$(cd "$OUT" && pwd)
SCR=$(mktemp -d "$BASE/run.XXXXXX")
pid=
trap '[ -z "$pid" ] || kill "$pid" 2>/dev/null || true; rm -rf "$SCR"' EXIT
sha() { shasum -a 256 "$1" | cut -d' ' -f1; }
printf '#R 02\r\n#Z 01 C:\\HYP_VIEW.NAP@\r\n' > "$SCR/newdesk.inf"
printf 'DEFAULT=C:\\TEST.HYP\r\n' > "$SCR/HYP_VIEW.CFG"
cd "$REPO"
tools/mkcard.sh "$SCR/card.img" "$SCR/newdesk.inf" "HYP_VIEW.NAP=$NAP" "HYP_VIEW.RSC=$RSC" "HYP_VIEW.CFG=$SCR/HYP_VIEW.CFG" "TEST.HYP=$HYP" > "$SCR/mkcard.stdout" 2>&1 \
    || { cat "$SCR/mkcard.stdout" >&2; exit 1; }
cp "$SCR/card.img.manifest" "$OUT/card.img.manifest"
{ echo "a141 newtos hypview ($(date -u +%Y-%m-%dT%H:%M:%SZ))"; echo "commit $(git rev-parse HEAD)"; echo "kernel $(sha "$KERNEL")"; echo "nap $(sha "$NAP")"; echo "rsc $(sha "$RSC")"; echo "hyp $(sha "$HYP")"; echo "card $(sha "$SCR/card.img")"; } > "$OUT/run.txt"
cp -c "$SCR/card.img" "$SCR/boot.img"
: > "$OUT/BOOT-SERIAL.txt"
cd "$SCR"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$OUT/BOOT-SERIAL.txt" \
    -append serial=on -kernel "$KERNEL" -drive file="$SCR/boot.img",if=sd,format=raw \
    -qmp unix:qmp.sock,server=on,wait=off &
pid=$!
sleep "${A141_WAIT:-30}"
python3 -I - qmp.sock "$OUT/shots/hypview.png" <<'EOF'
import json, socket, sys
s = socket.socket(socket.AF_UNIX); s.connect(sys.argv[1]); f = s.makefile("rw")
f.readline()
def cmd(c, a=None):
    f.write(json.dumps({"execute": c, **({"arguments": a} if a else {})}) + "\n"); f.flush()
    while True:
        r = json.loads(f.readline())
        if "return" in r or "error" in r:
            return r
cmd("qmp_capabilities")
print(cmd("screendump", {"filename": sys.argv[2], "format": "png"}))
EOF
sleep 1
kill "$pid" 2>/dev/null || true; sleep 1; kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
grep -a -c -E "Panic|Bus Error|Bus error" "$OUT/BOOT-SERIAL.txt" | sed 's/^/panic-lines /' >> "$OUT/run.txt" || true
cat "$OUT/run.txt"
