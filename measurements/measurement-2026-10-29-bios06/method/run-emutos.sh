#!/bin/sh
# run-emutos.sh <repo> <prg> <out-dir> tt|falcontc <trace-file> -- GNPROBE.PRG under the
# pinned EmuTOS ROM on Hatari 2.6.1 with tools/oracle/hatari.cfg, started as
# tools/hatari-oracle.sh starts a program (empty HOME, SDL dummy, --conout 2, GEMDOS
# drive C: = <out>/hd holding only the program, --auto, SIGKILL) -- after
# measurement-2026-10-28-bios-core-audit/method/run-emutos.sh, plus
# --trace bios,gemdos,cpu_exception into <trace-file> (scratch: large).  hatari.cfg is
# not touched, never --saveconfig.  Ends 10 s (wall, fast-forward) after GNPROB2.LOG
# appears (written before the Getmpb(NULL) call), or after 240 s.
set -eu
REPO=$1; PRG=$2; OUT=$3; PROF=$4; TRACE=$5
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
    --trace bios,gemdos,cpu_exception --trace-file "$TRACE" \
    > "$OUT/hatari.log" 2>&1 &
PID=$!
i=0
while kill -0 "$PID" 2>/dev/null; do
    if [ -s "$OUT/hd/GNPROB2.LOG" ]; then sleep 10; break; fi
    i=$((i + 1)); [ "$i" -lt 480 ] || break
    sleep 0.5
done
kill -9 "$PID" 2>/dev/null || true; wait "$PID" 2>/dev/null || true
rm -rf "$OUT/home"
for f in GNPROBE.LOG GNPROB2.LOG; do [ -f "$OUT/hd/$f" ] && cp "$OUT/hd/$f" "$OUT/$f"; done
ls -l "$OUT"
