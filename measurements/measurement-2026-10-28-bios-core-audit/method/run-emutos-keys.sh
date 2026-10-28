#!/bin/sh
# run-emutos-keys.sh <repo> <KPROBE.PRG> <out-dir> -- kprobe.c under the pinned
# EmuTOS (TT profile, otherwise exactly tools/hatari-oracle.sh's start), plus
# --cmd-fifo <out>/fifo: when "KEYREADY" appears in hatari.log (--conout 2), the
# six key strokes of kprobe.c go in as "hatari-event keydown/keyup <ST scancode>"
# (method/keys-hatari.txt), one command per 0.05 s, --fast-forward off (real-time, so no key auto-repeat between keydown and keyup).  --trace bios,xbios as input check.
set -eu
REPO=$1; PRG=$2; OUT=$3; HERE=$(cd "$(dirname "$0")" && pwd)
. "$REPO/tools/oracle/oracle.env"
[ "$(shasum -a 256 "$ORACLE_ROM" | cut -d' ' -f1)" = "$ORACLE_ROM_SHA256" ] || { echo "ROM mismatch" >&2; exit 1; }
[ ! -e "$OUT" ] || { echo "out exists" >&2; exit 1; }
mkdir -p "$OUT/hd" "$OUT/home"; OUT=$(cd "$OUT" && pwd)
cp "$PRG" "$OUT/hd/KPROBE.PRG"
HOME="$OUT/home" SDL_VIDEODRIVER=dummy "$ORACLE_HATARI" \
    --configfile "$ORACLE_HATARI_CFG" --machine tt --tos-res ttmed --monitor vga --tos "$ORACLE_ROM" \
    --sound off --fast-forward off --conout 2 --run-vbls 60000 \
    -d "$OUT/hd" --gemdos-drive C --auto 'C:\KPROBE.PRG' --cmd-fifo "$OUT/fifo" \
    --trace bios,xbios --trace-file "$OUT/trace.txt" > "$OUT/hatari.log" 2>&1 &
PID=$!; i=0; sent=0
while kill -0 "$PID" 2>/dev/null; do
    if [ $sent = 0 ] && grep -q KEYREADY "$OUT/hatari.log"; then
        grep -v '^#' "$HERE/keys-hatari.txt" | grep . | while read -r c; do echo "$c" > "$OUT/fifo"; sleep 0.05; done
        sent=1
    fi
    [ -s "$OUT/hd/KPROBE.LOG" ] && { sleep 2; break; }
    i=$((i + 1)); [ "$i" -lt 480 ] || break; sleep 0.5
done
kill -9 "$PID" 2>/dev/null || true; wait "$PID" 2>/dev/null || true
rm -rf "$OUT/home"; [ -f "$OUT/hd/KPROBE.LOG" ] && cp "$OUT/hd/KPROBE.LOG" "$OUT/"; ls "$OUT"
