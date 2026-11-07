#!/bin/sh
# TASK/A142: the real m68k QED (cflib built for m68k) on the pinned oracle, with keyboard input and screenshots through --cmd-fifo.
#   hatari-qed.sh <out-dir> tt|falcontc <qed-dir with QED.APP, QED.RSC, ICONS.RSC, CONFIG.TXT, QED.CFG> global|alert|info
# Same ROM, config and Hatari as tools/hatari-oracle.sh (tools/oracle/oracle.env).
# One dialog per session, keyboard only (Hatari's --cmd-fifo has no mouse motion; Return in a QED dialog ended this m68k QED).
set -eu
OUT=$1; PROFILE=$2; SRC=$3; MODE=$4
REPO=/Volumes/Daten/Development/PiRusTOS
. "$REPO/tools/oracle/oracle.env"
mkdir -p "$OUT/hd/QED" "$OUT/home" "$OUT/shots"
OUT=$(cd "$OUT" && pwd)
cp "$SRC"/* "$OUT/hd/QED/"
case $PROFILE in
    falcontc)  MACHINE="--machine falcon --monitor vga"; E="#E 1A E0 00 1C 60" ;;
    tt)        MACHINE="--machine tt --tos-res ttmed --monitor vga"; E="" ;;
    *) echo "bad profile" >&2; exit 2 ;;
esac
if [ -n "$E" ]; then
    printf '#R 02\r\n%s\r\n#Q 41 40 43 40 43 40\r\n#M 00 00 00 FF C HARD DISK C@ @\r\n#F FF 07 @ *.*@\r\n#T 00 03 03 FF   TRASH@ @\r\n' "$E" > "$OUT/hd/EMUDESK.INF"
fi
FIFO="$OUT/fifo"; rm -f "$FIFO"
HOME="$OUT/home" SDL_VIDEODRIVER=dummy "$ORACLE_HATARI" --configfile "$ORACLE_HATARI_CFG" $MACHINE --tos "$ORACLE_ROM" \
    --sound off --fast-forward on --conout 2 -d "$OUT/hd" --gemdos-drive C --auto 'C:\QED\QED.APP' \
    --screenshot-dir "$OUT/shots" --screenshot-format png --cmd-fifo "$FIFO" > "$OUT/hatari.log" 2>&1 &
PID=$!
send() { ( echo "$1" > "$FIFO" ) & w=$!; sleep 1; kill $w 2>/dev/null || true; }
shot() { send "hatari-shortcut screenshot"; sleep 1; }
combo() { send "hatari-event keydown $1"; send "hatari-event keypress $2"; send "hatari-event keyup $1"; sleep "${3:-3}"; }
sleep "${A142_WAIT:-25}"
shot                 # 1 QED up
case $MODE in
    global) combo 56 34; shot ;;                            # Alt-G Global Settings
    alert)  combo 56 21; shot; combo 56 44; shot ;;         # Alt-Z, the key labelled Z on the German and on the US layout: Font -> alert
    info)   combo 29 49; shot; combo 29 23; shot ;;         # Ctrl-N new text, Ctrl-I Textinfo
    *) echo "bad mode" >&2 ;;
esac
kill -9 $PID 2>/dev/null || true; wait $PID 2>/dev/null || true
rm -rf "$OUT/home" "$FIFO"
ls "$OUT/shots"
{ echo "profile $PROFILE"; echo "app $(shasum -a 256 "$SRC/QED.APP" | cut -d' ' -f1)"; echo "rsc $(shasum -a 256 "$SRC/QED.RSC" | cut -d' ' -f1)"; echo "rom $(shasum -a 256 "$ORACLE_ROM" | cut -d' ' -f1)"; } > "$OUT/run.txt"
