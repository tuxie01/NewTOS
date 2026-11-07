#!/usr/bin/env bash
# TASK/A140 one run: card with HypView as accessory (C:\ACC\HYPVIEW.NAC + HYP_VIEW.RSC, the issue's step 1),
# QEMU raspi4b on an APFS clone, QMP screendumps at the ticks of <shots.txt> after "[inj] desktop armed".
#   run.sh <out-dir> <kernel8.img> <HYP_VIEW.NAP> <hyp_view.rsc> <shots.txt>
# env A140_SCRATCH (session scratchpad, required), A140_HYP (the .HYP put on C:\GEMSYS\GUIDES\TEST.HYP, the default search path of HypView, required)
# env A140_APPEND: kernel command line (default serial=on; the control run adds memprot=off)
set -euo pipefail
REPO=/Volumes/Daten/Development/PiRusTOS
cd "$REPO"
HERE=$(dirname "$0")
BASE=${A140_SCRATCH:?}; HYP=${A140_HYP:?}
OUT=$1; KERNEL=$2; NAP=$3; RSC=$4; SHOTS=$5
[ ! -e "$OUT" ] || { echo "out exists: $OUT" >&2; exit 1; }
mkdir -p "$OUT/shots" "$BASE"; OUT=$(cd "$OUT" && pwd)
SCR=$(mktemp -d "$BASE/run.XXXXXX")
trap 'rm -rf "$SCR"' EXIT
sha() { shasum -a 256 "$1" | cut -d' ' -f1; }
cp "$SHOTS" "$OUT/shots.txt"
tools/mkcard.sh "$SCR/card.img" "${A140_INF:?}" "HYP_VIEW.NAP=$NAP" "HYP_VIEW.RSC=$RSC" "GEMSYS/GUIDES/TEST.HYP=$HYP" > "$SCR/mkcard.stdout" 2>&1 \
    || { cat "$SCR/mkcard.stdout" >&2; exit 1; }
cp "$SCR/card.img.manifest" "$OUT/card.img.manifest"
{ echo "a140 run ($(date -u +%Y-%m-%dT%H:%M:%SZ))"; echo "kernel $(sha "$KERNEL")"; echo "nap $(sha "$NAP")"; echo "rsc $(sha "$RSC")"
  echo "hyp $(sha "$HYP")"; echo "card $(sha "$SCR/card.img")"; echo "shots $(sha "$SHOTS")"; } > "$OUT/run.txt"
cp -c "$SCR/card.img" "$SCR/boot.img"
: > "$OUT/BOOT-SERIAL.txt"
cd "$SCR"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$OUT/BOOT-SERIAL.txt" \
    -append "${A140_APPEND:-serial=on}" -kernel "$KERNEL" -drive file="$SCR/boot.img",if=sd,format=raw \
    -qmp unix:qmp.sock,server=on,wait=off &
pid=$!
for _ in $(seq 1 100); do [ -S qmp.sock ] && break; sleep 0.1; done; sleep 2
python3 -I "$REPO/tosdump/probes/shprobe-qmpshots.py" qmp.sock "$OUT/BOOT-SERIAL.txt" "$OUT/shots.txt" "$OUT/shots" "${A140_MARK:?}" &
spid=$!
wait "$spid" 2>/dev/null || echo "qmpshots failed" >> "$OUT/run.txt"
sleep 1
kill "$pid" 2>/dev/null || true; sleep 1; kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
echo "card-after $(sha "$SCR/boot.img")" >> "$OUT/run.txt"
cstart=$(od -An -t u4 -j 454 -N 4 "$SCR/boot.img" | tr -d ' ')
for f in GDTRACE.BIN AESTRACE.BIN; do
    MTOOLS_SKIP_CHECK=1 mcopy -o -i "$SCR/boot.img@@$((cstart * 512))" "::/$f" "$OUT/$f" 2>/dev/null && echo "trace $f $(sha "$OUT/$f")" >> "$OUT/run.txt" || true
done
echo "shots $(ls "$OUT/shots" | grep -c png)" >> "$OUT/run.txt"
grep -a -c -E "Panic|Bus Error|Bus error" "$OUT/BOOT-SERIAL.txt" | sed 's/^/panic-lines /' >> "$OUT/run.txt" || true
cat "$OUT/run.txt"
