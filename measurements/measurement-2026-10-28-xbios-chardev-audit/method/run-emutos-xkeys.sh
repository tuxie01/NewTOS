#!/bin/sh
# run-emutos-xkeys.sh <repo> <XKPROBE.PRG> <out-dir> tt|falcontc -- xkprobe.c under the
# pinned EmuTOS, otherwise exactly tools/hatari-oracle.sh's start (empty HOME, SDL
# dummy, --conout 2, GEMDOS drive C:, --auto, SIGKILL), plus --cmd-fifo: after
# "KEYREADY" appears in hatari.log the lines of method/xkeys-hatari.txt go in,
# each after its sleep; --fast-forward off (as in the bios-core audit's key runs:
# with fast-forward a key auto-repeats between keydown and keyup).
# --trace bios,xbios as the EmuTOS-side input check.
# After measurement-2026-10-28-bios-core-audit/method/run-emutos-keys.sh.
set -eu
REPO=$1; PRG=$2; OUT=$3; PROF=$4; HERE=$(cd "$(dirname "$0")" && pwd)
. "$REPO/tools/oracle/oracle.env"
[ "$(shasum -a 256 "$ORACLE_ROM" | cut -d' ' -f1)" = "$ORACLE_ROM_SHA256" ] || { echo "ROM mismatch" >&2; exit 1; }
[ ! -e "$OUT" ] || { echo "out exists" >&2; exit 1; }
mkdir -p "$OUT/hd" "$OUT/home"; OUT=$(cd "$OUT" && pwd)
cp "$PRG" "$OUT/hd/XKPROBE.PRG"
case $PROF in
  tt)       MACHINE="--machine tt --tos-res ttmed --monitor vga" ;;
  falcontc) MACHINE="--machine falcon --monitor vga" ;;
esac
# shellcheck disable=SC2086
HOME="$OUT/home" SDL_VIDEODRIVER=dummy "$ORACLE_HATARI" \
    --configfile "$ORACLE_HATARI_CFG" $MACHINE --tos "$ORACLE_ROM" \
    --sound off --fast-forward off --conout 2 --run-vbls 60000 \
    -d "$OUT/hd" --gemdos-drive C --auto 'C:\XKPROBE.PRG' --cmd-fifo "$OUT/fifo" \
    --trace bios,xbios --trace-file "$OUT/trace.txt" > "$OUT/hatari.log" 2>&1 &
PID=$!; i=0; sent=0
while kill -0 "$PID" 2>/dev/null; do
    if [ $sent = 0 ] && grep -q KEYREADY "$OUT/hatari.log"; then
        grep -v '^#' "$HERE/xkeys-hatari.txt" | grep . | while read -r s c; do sleep "$s"; echo "$c" > "$OUT/fifo"; done
        sent=1
    fi
    [ -s "$OUT/hd/XKPROBE.LOG" ] && { sleep 2; break; }
    i=$((i + 1)); [ "$i" -lt 240 ] || break; sleep 0.5
done
kill -9 "$PID" 2>/dev/null || true; wait "$PID" 2>/dev/null || true
rm -rf "$OUT/home"; [ -f "$OUT/hd/XKPROBE.LOG" ] && cp "$OUT/hd/XKPROBE.LOG" "$OUT/"; ls "$OUT"
