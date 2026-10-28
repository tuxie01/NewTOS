#!/bin/sh
# run-emutos.sh <scratch> <out-dir> [falcon|tt] [nofloppy] -- BSTPROBE.PRG under the pinned EmuTOS
# ROM on Hatari 2.6.1 with tools/oracle/hatari.cfg, as tools/hatari-oracle.sh does it
# (empty HOME, SDL dummy, --conout 2, SIGKILL at the end), with three differences
# that make it a DISK run, all on the command line (hatari.cfg is not touched,
# never --saveconfig):
#   --ide-master <APFS clone of card.img>   the BIOS block device C: (IDE unit 18)
#   -d <out>/hd --gemdos-drive D            the program's own drive, NOT the test disk
#   --trace bios --trace-file               EmuTOS-side input check (Hatari decodes
#                                           every trap #13 with its arguments)
# The clone is kept as <scratch>/emutos-after.img for the image comparison.
set -eu
S=$1; OUT=$2; PROF=${3:-falcon}; FLOP=${4:-}
HERE=$(cd "$(dirname "$0")" && pwd); REPO=$(cd "$HERE/../../.." && pwd)
. "$REPO/tools/oracle/oracle.env"
[ "$(shasum -a 256 "$ORACLE_ROM" | cut -d' ' -f1)" = "$ORACLE_ROM_SHA256" ] || { echo "ROM mismatch" >&2; exit 1; }
[ ! -e "$OUT" ] || { echo "out exists" >&2; exit 1; }
mkdir -p "$OUT/hd" "$OUT/home"; OUT=$(cd "$OUT" && pwd)
cp "$S/BSTPROBE.PRG" "$OUT/hd/BSTPROBE.PRG"
IMG=$S/emutos-$PROF$FLOP-after.img; rm -f "$IMG"; cp -c "$S/card.img" "$IMG"
case $PROF in
  falcon) MACHINE="--machine falcon --monitor vga" ;;
  tt)     MACHINE="--machine tt --tos-res ttmed --monitor vga" ;;
esac
# nofloppy: --drive-a off --drive-b off, the device-parameter control run (the pin
# itself keeps Hatari's default, both floppy drives present, no disk inserted)
[ "$FLOP" = nofloppy ] && MACHINE="$MACHINE --drive-a off --drive-b off"
# shellcheck disable=SC2086
HOME="$OUT/home" SDL_VIDEODRIVER=dummy "$ORACLE_HATARI" \
    --configfile "$ORACLE_HATARI_CFG" $MACHINE --tos "$ORACLE_ROM" \
    --sound off --fast-forward on --conout 2 --run-vbls 30000 \
    --ide-master "$IMG" -d "$OUT/hd" --gemdos-drive D --auto 'D:\BSTPROBE.PRG' \
    --trace bios --trace-file "$OUT/trace-bios.txt" \
    > "$OUT/hatari.log" 2>&1 &
PID=$!
i=0
while kill -0 "$PID" 2>/dev/null; do
    [ -s "$OUT/hd/BSTPROB2.LOG" ] && { sleep 2; break; }
    i=$((i + 1)); [ "$i" -lt 240 ] || break
    sleep 0.5
done
kill -9 "$PID" 2>/dev/null || true; wait "$PID" 2>/dev/null || true
rm -rf "$OUT/home"
for f in BSTPROBE.LOG BSTPROB2.LOG; do [ -f "$OUT/hd/$f" ] && cp "$OUT/hd/$f" "$OUT/$f"; done
ls -l "$OUT"
