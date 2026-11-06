#!/bin/bash
# txbench-run.sh <out-dir> <kernel8.img> [qemu-args...]: one TXFONT.NAP run on an APFS clone of a
# fresh card; prints the five result lines. Env: TXF_NAP, TXB_SCR (scratch), run from the repo root.
set -euo pipefail
OUT=$1; KERNEL=$2; shift 2
[ ! -e "$OUT" ] || { echo "exists: $OUT" >&2; exit 1; }
mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
SCR=$(mktemp -d "$TXB_SCR/txb-run.XXXXXX"); trap 'rm -rf "$SCR"' EXIT
printf '#R 02\r\n#Z 01 C:\\TXFONT.NAP@\r\n' > "$SCR/newdesk.inf"
tools/mkcard.sh "$SCR/card.img" "$SCR/newdesk.inf" TXFONT.NAP="$TXF_NAP" > "$SCR/mk.out" 2>&1 || { cat "$SCR/mk.out"; exit 1; }
cp "$SCR/card.img.manifest" "$OUT/"; cp -c "$SCR/card.img" "$SCR/boot.img"
echo "kernel $(shasum -a 256 "$KERNEL" | cut -d' ' -f1) nap $(shasum -a 256 "$TXF_NAP" | cut -d' ' -f1) qemu-args: $*" > "$OUT/run.txt"
: > "$OUT/BOOT-SERIAL.txt"
qemu-system-aarch64 -M raspi4b -no-reboot -display none "$@" -serial file:"$OUT/BOOT-SERIAL.txt" -append serial=on -kernel "$KERNEL" -drive file="$SCR/boot.img",if=sd,format=raw &
pid=$!
state=timeout
for _ in $(seq 1 ${TXB_POLLS:-400}); do sleep 0.5
  if grep -a -q "TXFONT.NAP' exited with rc" "$OUT/BOOT-SERIAL.txt"; then state=done; break; fi
  kill -0 $pid 2>/dev/null || { state=qemu-exited; break; }
done
sleep 1; kill $pid 2>/dev/null || true; sleep 0.5; kill -9 $pid 2>/dev/null || true; wait $pid 2>/dev/null || true
echo "qemu $state" >> "$OUT/run.txt"
cstart=$(od -An -t u4 -j 454 -N 4 "$SCR/boot.img" | tr -d ' ')
MTOOLS_SKIP_CHECK=1 mcopy -o -i "$SCR/boot.img@@$((cstart * 512))" ::/TXFONT.LOG "$OUT/TXFONT.LOG" || echo "no LOG" >> "$OUT/run.txt"
tail -1 "$OUT/run.txt"; cat "$OUT/TXFONT.LOG" 2>&1
