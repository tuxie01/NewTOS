#!/bin/sh
# run-hatari-fsel.sh <FSPROBE.PRG> <out-dir> <scratch> <listdir> <wait-s>
# The oracle profile falcontc of tools/hatari-oracle.sh (same Hatari, pinned cfg,
# pinned ROM, EMUDESK.INF truecolor, Falcon, VGA), but with a screenshot over
# --cmd-fifo (hatari-shortcut screenshot) after <wait-s> seconds, while the
# probe's file selector is up. C: = <scratch>/hd with FSPROBE.PRG, EMUDESK.INF and
# LIST\F01..F30.TXT (copy of <listdir>). Screenshot lands in <out-dir>/shots.
set -eu
PRG=$1; OUT=$2; SCR=$3; LIST=$4; WAIT=$5
HERE=$(cd "$(dirname "$0")" && pwd); ROOT=$(cd "$HERE/../../.." && pwd)
. "$ROOT/tools/oracle/oracle.env"
[ "$(shasum -a 256 "$ORACLE_ROM" | cut -d' ' -f1)" = "$ORACLE_ROM_SHA256" ] || { echo "ROM sha mismatch" >&2; exit 1; }
HD=$SCR/hd; rm -rf "$HD"; mkdir -p "$HD/LIST" "$OUT/shots" "$OUT/serial"
cp "$PRG" "$HD/FSPROBE.PRG"; cp "$LIST"/F*.TXT "$HD/LIST/"
printf '#R 02\r\n#E 1A E0 00 1C 60\r\n#Q 41 40 43 40 43 40\r\n#W 00 00 02 06 26 0C 00 @\r\n#W 00 00 02 08 26 0C 00 @\r\n#W 00 00 02 0A 26 0C 00 @\r\n#W 00 00 02 0D 26 0C 00 @\r\n#M 00 00 00 FF C HARD DISK C@ @\r\n#M 01 00 00 FF D HARD DISK D@ @\r\n#F FF 07 @ *.*@\r\n#T 00 03 03 FF   TRASH@ @\r\n' > "$HD/EMUDESK.INF"
FIFO=$SCR/cmd.fifo; rm -f "$FIFO"   # Hatari creates the fifo itself
mkdir -p "$SCR/home"
HOME="$SCR/home" SDL_VIDEODRIVER=dummy "$ORACLE_HATARI" --configfile "$ORACLE_HATARI_CFG" \
  --machine falcon --monitor vga --tos "$ORACLE_ROM" --sound off --conout 2 \
  -d "$HD" --gemdos-drive C --auto 'C:\FSPROBE.PRG' \
  --screenshot-dir "$OUT/shots" --screenshot-format png --crop off --cmd-fifo "$FIFO" \
  > "$OUT/serial/hatari.log" 2>&1 &
PID=$!
i=0; while [ ! -p "$FIFO" ] && [ $i -lt 50 ]; do sleep 0.2; i=$((i+1)); done
exec 3<>"$FIFO"
sleep "$WAIT"
echo "hatari-shortcut screenshot" >&3
sleep 3
echo "hatari-shortcut quit" >&3
sleep 4
kill -9 $PID 2>/dev/null || true; wait $PID 2>/dev/null || true
exec 3>&-
rm -f "$FIFO"; rm -rf "$SCR/home"
ls "$OUT/shots"
