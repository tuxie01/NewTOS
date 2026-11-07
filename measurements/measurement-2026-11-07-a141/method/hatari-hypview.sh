#!/bin/sh
# TASK/A141: the real m68k HypView on the pinned oracle, one machine profile, one resource; screenshot via --cmd-fifo.
#   hatari-hypview.sh <out-dir> falcontc|falcon256|tt <HYP_VIEW.APP> <hyp_view.rsc> <test.hyp>
# Same ROM, config and Hatari as tools/hatari-oracle.sh (tools/oracle/oracle.env). The INF's "#E 1A E0 00 1C 60" is the
# falcontc line of tools/hatari-oracle.sh; falcon256 is the same with VIDEL 0x001B (memory: docs/analysis/ORACLE.md section 2).
set -eu
OUT=$1; PROFILE=$2; APP=$3; RSC=$4; HYP=$5
REPO=/Volumes/Daten/Development/PiRusTOS
. "$REPO/tools/oracle/oracle.env"
mkdir -p "$OUT/hd" "$OUT/home" "$OUT/shots"
OUT=$(cd "$OUT" && pwd)
cp "$APP" "$OUT/hd/HYP_VIEW.APP"; cp "$RSC" "$OUT/hd/HYP_VIEW.RSC"; cp "$HYP" "$OUT/hd/TEST.HYP"
printf 'DEFAULT=C:\\TEST.HYP\r\n' > "$OUT/hd/HYP_VIEW.CFG"
case $PROFILE in
    falcontc)  MACHINE="--machine falcon --monitor vga"; E="#E 1A E0 00 1C 60" ;;
    falcon256) MACHINE="--machine falcon --monitor vga"; E="#E 1A E0 00 1B 60" ;;
    tt)        MACHINE="--machine tt --tos-res ttmed --monitor vga"; E="" ;;
    *) echo "bad profile" >&2; exit 2 ;;
esac
if [ -n "$E" ]; then
    printf '#R 02\r\n%s\r\n#Q 41 40 43 40 43 40\r\n#M 00 00 00 FF C HARD DISK C@ @\r\n#F FF 07 @ *.*@\r\n#T 00 03 03 FF   TRASH@ @\r\n' "$E" > "$OUT/hd/EMUDESK.INF"
fi
FIFO="$OUT/fifo"; rm -f "$FIFO"
HOME="$OUT/home" SDL_VIDEODRIVER=dummy "$ORACLE_HATARI" --configfile "$ORACLE_HATARI_CFG" $MACHINE --tos "$ORACLE_ROM" \
    --sound off --fast-forward on --conout 2 -d "$OUT/hd" --gemdos-drive C --auto 'C:\HYP_VIEW.APP' \
    --screenshot-dir "$OUT/shots" --screenshot-format png --cmd-fifo "$FIFO" > "$OUT/hatari.log" 2>&1 &
PID=$!
send() { ( echo "$1" > "$FIFO" ) & w=$!; sleep 1; kill $w 2>/dev/null || true; }
sleep "${A141_WAIT:-25}"
send "hatari-shortcut screenshot"; sleep 2
send "hatari-shortcut screenshot"; sleep 2
kill -9 $PID 2>/dev/null || true; wait $PID 2>/dev/null || true
rm -rf "$OUT/home" "$FIFO"
ls "$OUT/shots"
{ echo "profile $PROFILE"; echo "app $(shasum -a 256 "$APP" | cut -d' ' -f1)"; echo "rsc $(shasum -a 256 "$RSC" | cut -d' ' -f1)"; echo "hyp $(shasum -a 256 "$HYP" | cut -d' ' -f1)"; echo "rom $(shasum -a 256 "$ORACLE_ROM" | cut -d' ' -f1)"; } > "$OUT/run.txt"
