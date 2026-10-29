#!/usr/bin/env bash
# run-side.sh <side> <bin-dir> <kernel8.img> <scratch> <run-dir> -- one SDK05 run on QEMU raspi4b.
#
# <bin-dir> holds CKPROBE.NCP (sdk/cflib/tests/cookie-probe.c via build-cflib.sh), QED.NAP and qed.rsc
# (sdk/apps/build-qed.sh) of one side (before = upstream cookie.c in cflib.a, after = the SDK05 overlay).
# Card from tools/mkcard.sh: C:\AUTO\CKPROBE.NCP (runs before the AES, writes C:\CKPROBE.LOG),
# C:\Apps\QED\{QED.NAP,qed.rsc}, NEWDESK.INF = method/z-qed.inf (#Z autorun of QED). Booted as an APFS
# clone, -append serial=on. HMP monitor on a pipe chardev (no socket): after WAIT_S seconds one
# screendump and a pmemsave of the low RAM_SIZE bytes, then QEMU is killed. method/find-idt.py reads
# QED's `_idt` out of the dump (symbol offset from llvm-nm of the same QED.NAP); C:\CKPROBE.LOG is read
# back from the clone with mtools. Clone and dump are deleted afterwards; their sha256 stay in run.txt.
set -uo pipefail
SIDE=$1; B=$2; K=$3; S=$4; R=$5
WAIT_S=${WAIT_S:-45}; RAM_SIZE=${RAM_SIZE:-0x40000000}
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; WS="$(cd "$HERE/../../.." && pwd)"
NM="$(ls ~/.rustup/toolchains/*/lib/rustlib/*/bin/llvm-nm | head -1)"
OUT="$R/runs/$SIDE"; W="$S/$SIDE"; mkdir -p "$OUT" "$W"
RUN="$OUT/run.txt"; : > "$RUN"
{ echo "side $SIDE  $(date '+%Y-%m-%d %H:%M:%S %Z')"; (cd "$B" && shasum -a 256 CKPROBE.NCP QED.NAP qed.rsc); shasum -a 256 "$K" | sed "s#$K#kernel8.img#"; } >> "$RUN"
(cd "$WS" && tools/mkcard.sh "$W/card.img" "$HERE/z-qed.inf" AUTO/CKPROBE.NCP="$B/CKPROBE.NCP" \
   Apps/QED/QED.NAP="$B/QED.NAP" Apps/QED/qed.rsc="$B/qed.rsc") > "$W/mkcard.log" 2>&1 || { cat "$W/mkcard.log"; exit 1; }
cp "$W/card.img.manifest" "$OUT/card.img.manifest"
echo "card $(shasum -a 256 "$W/card.img" | cut -c1-64)" >> "$RUN"
CLONE="$W/run.img"; rm -f "$CLONE"; cp -c "$W/card.img" "$CLONE"
rm -f "$W/mon.in" "$W/mon.out"; mkfifo "$W/mon.in" "$W/mon.out"
cat "$W/mon.out" > "$W/monitor.raw" & CATP=$!
exec 3<>"$W/mon.in"
# QEMU runs in $W and gets the dump file names relative: HMP parses pmemsave's size as an expression,
# and a following absolute path's '/' reads as a division ("invalid char 'p' in expression").
(cd "$W" && exec qemu-system-aarch64 -M raspi4b -no-reboot -display none -append serial=on -kernel "$K" \
  -drive file="$CLONE",if=sd,format=raw -serial file:"$OUT/SERIAL.txt" \
  -chardev pipe,id=mon,path="$W/mon" -mon chardev=mon,mode=readline) > "$OUT/qemu-stdout.txt" 2>&1 &
QP=$!
sleep "$WAIT_S"
echo "screendump shot.ppm" >&3; sleep 2
echo "pmemsave 0 $RAM_SIZE ram.bin" >&3
for i in $(seq 1 120); do sleep 1; [ -f "$W/ram.bin" ] && [ "$(stat -f %z "$W/ram.bin")" -ge $((RAM_SIZE)) ] && break; done
sleep 1; kill -9 $QP 2>/dev/null; wait $QP 2>/dev/null; exec 3>&-; kill $CATP 2>/dev/null; wait $CATP 2>/dev/null
# keep the monitor replies, not its readline echo (the lines carrying ESC sequences)
LC_ALL=C tr '\r' '\n' < "$W/monitor.raw" | LC_ALL=C grep -av $'\x1b' > "$OUT/monitor.txt"
sips -s format png "$W/shot.ppm" --out "$OUT/qed.png" > /dev/null
SYM=$("$NM" "$B/QED.NAP" | awk '$3=="_idt"{print $1}')
echo "_idt symbol (llvm-nm QED.NAP) 0x$SYM" >> "$RUN"
python3 -I "$HERE/find-idt.py" "$W/ram.bin" "$B/QED.NAP" "$SYM" >> "$RUN" 2>&1
echo "ram dump $(shasum -a 256 "$W/ram.bin" | cut -c1-64) ($(stat -f %z "$W/ram.bin") bytes, deleted)" >> "$RUN"
# FAT16 partition 1 of the mkcard image: read its start sector from the MBR.
LBA=$(python3 -I -c 'import struct,sys; d=open(sys.argv[1],"rb").read(512); print(struct.unpack_from("<I",d,0x1C6)[0])' "$CLONE")
mcopy -n -i "$CLONE@@$((LBA*512))" ::CKPROBE.LOG "$OUT/CKPROBE.LOG" 2>> "$RUN" || echo "CKPROBE.LOG missing" >> "$RUN"
echo "clone after run $(shasum -a 256 "$CLONE" | cut -c1-64) (deleted)" >> "$RUN"
rm -f "$CLONE" "$W/ram.bin" "$W/shot.ppm" "$W/mon.in" "$W/mon.out" "$W/monitor.raw"
echo "crash=$(grep -c '^\[crash\]' "$OUT/SERIAL.txt") panic=$(grep -Ec 'PANIC|panicked|\[EXC\]' "$OUT/SERIAL.txt")" >> "$RUN"
cat "$RUN"; echo; cat "$OUT/CKPROBE.LOG" 2>/dev/null
