#!/usr/bin/env bash
# A142-test: HypView on NewTOS raspi4b (QEMU), card by tools/mkcard.sh (APFS clone, deleted after), HYP_VIEW.CFG DEFAULT=C:\TEST.HYP, QMP shots after "[inj] up:HYP_VIEW armed".
#   run-hv.sh <out-dir> <kernel8.img> <HYP_VIEW.NAP> <hyp_view.rsc> <test.hyp> <shots.txt>
set -euo pipefail
REPO=/Volumes/Daten/Development/PiRusTOS
cd "$REPO"
BASE=${A142_SCRATCH:?}
OUT=$1; KERNEL=$2; NAP=$3; RSC=$4; HYP=$5; SHOTS=$6
[ ! -e "$OUT" ] || { echo "out exists: $OUT" >&2; exit 1; }
mkdir -p "$OUT/shots" "$BASE"; OUT=$(cd "$OUT" && pwd)
SCR=$(mktemp -d "$BASE/run.XXXXXX")
trap 'rm -rf "$SCR"' EXIT
sha() { shasum -a 256 "$1" | cut -d' ' -f1; }
cp "$SHOTS" "$OUT/shots.txt"
printf '#R 02\r\n#Z 01 C:\\HYP_VIEW.NAP@\r\n' > "$SCR/newdesk.inf"
printf 'DEFAULT=C:\\TEST.HYP\r\n' > "$SCR/HYP_VIEW.CFG"
tools/mkcard.sh "$SCR/card.img" "$SCR/newdesk.inf" "HYP_VIEW.NAP=$NAP" "HYP_VIEW.RSC=$RSC" "HYP_VIEW.CFG=$SCR/HYP_VIEW.CFG" "TEST.HYP=$HYP" > "$SCR/mkcard.stdout" 2>&1 || { cat "$SCR/mkcard.stdout" >&2; exit 1; }
cp "$SCR/card.img.manifest" "$OUT/card.img.manifest"
{ echo "a142-test hv run ($(date -u +%Y-%m-%dT%H:%M:%SZ))"; echo "kernel $(sha "$KERNEL")"; echo "nap $(sha "$NAP")"; echo "rsc $(sha "$RSC")"; echo "hyp $(sha "$HYP")"; echo "card $(sha "$SCR/card.img")"; } > "$OUT/run.txt"
cp -c "$SCR/card.img" "$SCR/boot.img"
: > "$OUT/BOOT-SERIAL.txt"
cd "$SCR"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$OUT/BOOT-SERIAL.txt" -append serial=on -kernel "$KERNEL" -drive file="$SCR/boot.img",if=sd,format=raw -qmp unix:qmp.sock,server=on,wait=off &
pid=$!
for _ in $(seq 1 100); do [ -S qmp.sock ] && break; sleep 0.1; done; sleep 2
python3 -I "$REPO/tosdump/probes/shprobe-qmpshots.py" qmp.sock "$OUT/BOOT-SERIAL.txt" "$OUT/shots.txt" "$OUT/shots" "[inj] up:HYP_VIEW armed at hz_200" &
spid=$!
wait "$spid" 2>/dev/null || echo "qmpshots failed" >> "$OUT/run.txt"
sleep 1; kill "$pid" 2>/dev/null || true; sleep 1; kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
echo "shots $(ls "$OUT/shots" | grep -c png)" >> "$OUT/run.txt"
grep -a -c -E "Panic|Bus Error|Bus error|exception" "$OUT/BOOT-SERIAL.txt" | sed 's/^/panic-lines /' >> "$OUT/run.txt" || true
cat "$OUT/run.txt"
