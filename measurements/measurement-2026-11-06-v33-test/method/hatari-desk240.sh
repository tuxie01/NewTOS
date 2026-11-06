#!/bin/bash
# EmuTOS (pinned oracle ROM) desktop at 640x240 (INF mode word 0x011C as k19-240.sh): one screenshot via the cmd-fifo.
#   hatari-desk240.sh <out-dir> [mode-hex-pair, default "01 1C"]
set -eu
REPO=/Volumes/Daten/Development/PiRusTOS; cd "$REPO"; . tools/oracle/oracle.env
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/7fba4f48-e8f1-4a5e-8c58-696d17828400/scratchpad/v33t
OUT=$1; MODE=${2:-01 1C}
[ ! -e "$OUT" ] || { echo "out exists" >&2; exit 1; }
mkdir -p "$OUT/shots"; OUT=$(cd "$OUT" && pwd)
SCR=$(mktemp -d "$S/hd240.XXXXXX"); mkdir -p "$SCR/hd" "$SCR/home"
for f in "$S"/files/*; do cp "$f" "$SCR/hd/"; done
printf '#R 02\r\n#E 1A E0 %s 60\r\n#Q 41 40 43 40 43 40\r\n#W 00 00 02 06 26 0C 00 C:\\*.*@\r\n#W 00 00 02 08 26 0C 00 @\r\n#W 00 00 02 0A 26 0C 00 @\r\n#W 00 00 02 0D 26 0C 00 @\r\n#M 00 00 00 FF C HARD DISK C@ @\r\n#M 01 00 00 FF D HARD DISK D@ @\r\n#F FF 07 @ *.*@\r\n#T 00 03 03 FF   TRASH@ @\r\n' "$MODE" > "$SCR/hd/EMUDESK.INF"
cp "$SCR/hd/EMUDESK.INF" "$OUT/EMUDESK.INF"
FIFO=$SCR/cmd.fifo
HOME="$SCR/home" SDL_VIDEODRIVER=dummy "$ORACLE_HATARI" --configfile "$ORACLE_HATARI_CFG" \
    --machine falcon --monitor vga --tos "$ORACLE_ROM" --sound off --conout 2 \
    -d "$SCR/hd" --gemdos-drive C --screenshot-dir "$OUT/shots" --screenshot-format png --crop on \
    --cmd-fifo "$FIFO" > "$OUT/hatari.log" 2>&1 &
HPID=$!
until [ -p "$FIFO" ]; do sleep 1; done; exec 3>"$FIFO"
sleep 40
echo "hatari-shortcut screenshot" >&3; sleep 3
echo "hatari-shortcut quit" >&3; sleep 5
kill -0 $HPID 2>/dev/null && kill -9 $HPID 2>/dev/null || true
wait $HPID 2>/dev/null || true
echo "rom $(shasum -a 256 "$ORACLE_ROM" | cut -d' ' -f1) cfg $(shasum -a 256 "$ORACLE_HATARI_CFG" | cut -d' ' -f1)" > "$OUT/run.txt"
ls "$OUT/shots" >> "$OUT/run.txt"; cat "$OUT/run.txt"
rm -rf "$SCR"
