#!/bin/sh
# run-emutos.sh <repo> <prg> <out-dir> tt|falcontc <done-file> [<wait-after-s>]
# One probe program under the pinned EmuTOS ROM on Hatari 2.6.1 with
# tools/oracle/hatari.cfg, exactly as tools/hatari-oracle.sh starts it (empty
# HOME, SDL dummy, --conout 2, GEMDOS drive C: = <out>/hd holding only the
# program, --auto, SIGKILL), plus --trace bios,xbios into <out>/trace.txt for the
# EmuTOS-side input check.  hatari.cfg is not touched, never --saveconfig.
# Ends when <done-file> appears in <out>/hd (then 2 s), or after 240 s; for the
# blocking-Bconin variant <wait-after-s> seconds after the first BCHANG.LOG.
set -eu
REPO=$1; PRG=$2; OUT=$3; PROF=$4; DONE=$5; WAITS=${6:-}
. "$REPO/tools/oracle/oracle.env"
[ "$(shasum -a 256 "$ORACLE_ROM" | cut -d' ' -f1)" = "$ORACLE_ROM_SHA256" ] || { echo "ROM mismatch" >&2; exit 1; }
[ ! -e "$OUT" ] || { echo "out exists" >&2; exit 1; }
mkdir -p "$OUT/hd" "$OUT/home"; OUT=$(cd "$OUT" && pwd)
NAME=$(basename "$PRG"); cp "$PRG" "$OUT/hd/$NAME"
case $PROF in
  tt)       MACHINE="--machine tt --tos-res ttmed --monitor vga" ;;
  falcontc) MACHINE="--machine falcon --monitor vga"
            printf '#R 02\r\n#E 1A E0 00 1C 60\r\n#Q 41 40 43 40 43 40\r\n#W 00 00 02 06 26 0C 00 @\r\n#W 00 00 02 08 26 0C 00 @\r\n#W 00 00 02 0A 26 0C 00 @\r\n#W 00 00 02 0D 26 0C 00 @\r\n#M 00 00 00 FF C HARD DISK C@ @\r\n#M 01 00 00 FF D HARD DISK D@ @\r\n#F FF 07 @ *.*@\r\n#T 00 03 03 FF   TRASH@ @\r\n' > "$OUT/hd/EMUDESK.INF" ;;
esac
# shellcheck disable=SC2086
HOME="$OUT/home" SDL_VIDEODRIVER=dummy "$ORACLE_HATARI" \
    --configfile "$ORACLE_HATARI_CFG" $MACHINE --tos "$ORACLE_ROM" \
    --sound off --fast-forward on --conout 2 --run-vbls 60000 \
    -d "$OUT/hd" --gemdos-drive C --auto "C:\\$NAME" \
    --trace bios,xbios --trace-file "$OUT/trace.txt" \
    > "$OUT/hatari.log" 2>&1 &
PID=$!
i=0
while kill -0 "$PID" 2>/dev/null; do
    if [ -n "$WAITS" ] && [ -s "$OUT/hd/BCHANG.LOG" ]; then sleep "$WAITS"; break; fi
    if [ -z "$WAITS" ] && [ -s "$OUT/hd/$DONE" ]; then sleep 2; break; fi
    i=$((i + 1)); [ "$i" -lt 480 ] || break
    sleep 0.5
done
kill -9 "$PID" 2>/dev/null || true; wait "$PID" 2>/dev/null || true
rm -rf "$OUT/home"
for f in BCPROBE.LOG BCPROB2.LOG BCHANG.LOG; do [ -f "$OUT/hd/$f" ] && cp "$OUT/hd/$f" "$OUT/$f"; done
ls -l "$OUT"
