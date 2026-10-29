#!/usr/bin/env bash
# run-side.sh <side> <bin-dir> <kernel8.img> <scratch> <run-dir> -- one SDK04-C6 run on QEMU raspi4b.
#
# <bin-dir> holds TDCOOKIE.NCP (sdk/overlays/teradesk/tests/build-cookie-probe.sh) and TeraDesk's dist
# (DESKTOP.NAP, desktop.rsc, icons.rsc, cicons.rsc; sdk/apps/build-teradesk.sh) of one side (before = vendored
# cookie.c, after = the SDK04-C6 overlay). Card from tools/mkcard.sh: C:\AUTO\TDCOOKIE.NCP (runs before the
# AES, writes C:\TDCOOKIE.LOG), C:\Apps\TERADESK\ with TERADESK.INF, NEWDESK.INF = z-teradesk.inf (#Z autorun of
# DESKTOP.NAP). Booted as an APFS clone, -append serial=on. HMP monitor on a pipe chardev: after WAIT_S seconds
# one screendump, then QEMU is killed. C:\TDCOOKIE.LOG is read back from the clone with mtools; the clone is
# deleted afterwards, its sha256 stays in run.txt.
set -uo pipefail
SIDE=$1; B=$2; K=$3; S=$4; R=$5
WAIT_S=${WAIT_S:-60}
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; WS="$(cd "$HERE/../../.." && pwd)"
OUT="$R/runs/$SIDE"; W="$S/$SIDE-run"; mkdir -p "$OUT" "$W"
RUN="$OUT/run.txt"; : > "$RUN"
{ echo "side $SIDE  $(date '+%Y-%m-%d %H:%M:%S %Z')"
  (cd "$B" && shasum -a 256 TDCOOKIE.NCP DESKTOP.NAP desktop.rsc icons.rsc cicons.rsc)
  shasum -a 256 "$K" | sed "s#$K#kernel8.img#"; } >> "$RUN"
(cd "$WS" && tools/mkcard.sh "$W/card.img" "$HERE/z-teradesk.inf" AUTO/TDCOOKIE.NCP="$B/TDCOOKIE.NCP" \
   Apps/TERADESK/DESKTOP.NAP="$B/DESKTOP.NAP" Apps/TERADESK/desktop.rsc="$B/desktop.rsc" \
   Apps/TERADESK/icons.rsc="$B/icons.rsc" Apps/TERADESK/cicons.rsc="$B/cicons.rsc" \
   Apps/TERADESK/TERADESK.INF="$HERE/TERADESK.INF") > "$W/mkcard.log" 2>&1 || { cat "$W/mkcard.log"; exit 1; }
cp "$W/card.img.manifest" "$OUT/card.img.manifest"
echo "card $(shasum -a 256 "$W/card.img" | cut -c1-64)" >> "$RUN"
CLONE="$W/run.img"; rm -f "$CLONE"; cp -c "$W/card.img" "$CLONE"
rm -f "$W/mon.in" "$W/mon.out"; mkfifo "$W/mon.in" "$W/mon.out"
cat "$W/mon.out" > "$W/monitor.raw" & CATP=$!
exec 3<>"$W/mon.in"
# QEMU runs in $W and gets the dump file name relative (HMP argument parsing).
(cd "$W" && exec qemu-system-aarch64 -M raspi4b -no-reboot -display none -append serial=on -kernel "$K" \
  -drive file="$CLONE",if=sd,format=raw -serial file:"$OUT/SERIAL.txt" \
  -chardev pipe,id=mon,path="$W/mon" -mon chardev=mon,mode=readline) > "$OUT/qemu-stdout.txt" 2>&1 &
QP=$!
sleep "$WAIT_S"
echo "screendump shot.ppm" >&3
for i in $(seq 1 20); do sleep 1; [ -s "$W/shot.ppm" ] && break; done
sleep 1; kill -9 $QP 2>/dev/null; wait $QP 2>/dev/null; exec 3>&-; kill $CATP 2>/dev/null; wait $CATP 2>/dev/null
sips -s format png "$W/shot.ppm" --out "$OUT/teradesk.png" > /dev/null
# FAT16 partition 1 of the mkcard image: read its start sector from the MBR.
LBA=$(python3 -I -c 'import struct,sys; d=open(sys.argv[1],"rb").read(512); print(struct.unpack_from("<I",d,0x1C6)[0])' "$CLONE")
mcopy -n -i "$CLONE@@$((LBA*512))" ::TDCOOKIE.LOG "$OUT/TDCOOKIE.LOG" 2>> "$RUN" || echo "TDCOOKIE.LOG missing" >> "$RUN"
echo "clone after run $(shasum -a 256 "$CLONE" | cut -c1-64) (deleted)" >> "$RUN"
rm -f "$CLONE" "$W/card.img" "$W/shot.ppm" "$W/mon.in" "$W/mon.out" "$W/monitor.raw"
echo "crash=$(grep -c '^\[crash\]' "$OUT/SERIAL.txt") panic=$(grep -Ec 'PANIC|panicked|\[EXC\]' "$OUT/SERIAL.txt")" >> "$RUN"
cat "$RUN"; echo; cat "$OUT/TDCOOKIE.LOG" 2>/dev/null
