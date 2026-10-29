#!/bin/sh
# oracle-flush.sh <tt|falcontc> <DUMPBIOS.PRG> <table.tsv> <out-dir> -- XBIOS01: a
# DUMPBIOS table that needs keys typed while it runs, under the pinned EmuTOS on Hatari.
# Machine, ROM, config, hermetic HOME, GEMDOS drive and --auto start are exactly
# tools/hatari-oracle.sh's (profiles tt and falcontc, incl. the falcontc EMUDESK.INF);
# added: --cmd-fifo <out>/fifo and --fast-forward off (real-time VBLs, so the table's
# Vsync wait lasts seconds and no key auto-repeats between keydown and keyup -- the
# precedent measurement-2026-10-28-bios-core-audit/method/run-emutos-keys.sh). When
# <out>/hd/BIOS.LOG ends in "running N cases", the commands of keys-hatari.txt go into
# the fifo, one per 0.05 s.
set -eu
PROFILE=$1; DUMPER=$2; TABLE=$3; OUT=$4
HERE=$(cd "$(dirname "$0")" && pwd); REPO=$(cd "$HERE/../../.." && pwd)
. "$REPO/tools/oracle/oracle.env"
[ "$(shasum -a 256 "$ORACLE_ROM" | cut -d' ' -f1)" = "$ORACLE_ROM_SHA256" ] || { echo "ROM mismatch" >&2; exit 1; }
[ ! -e "$OUT" ] || { echo "out exists: $OUT" >&2; exit 1; }
case $PROFILE in
    tt)       MACHINE="--machine tt --tos-res ttmed --monitor vga" ;;
    falcontc) MACHINE="--machine falcon --monitor vga" ;;
    *) echo "profile: tt or falcontc" >&2; exit 2 ;;
esac
mkdir -p "$OUT/hd" "$OUT/home"; OUT=$(cd "$OUT" && pwd)
cp "$DUMPER" "$OUT/hd/DUMPBIOS.PRG"; cp "$TABLE" "$OUT/hd/BIOS.TSV"
if [ "$PROFILE" = falcontc ]; then
    printf '#R 02\r\n#E 1A E0 00 1C 60\r\n#Q 41 40 43 40 43 40\r\n#W 00 00 02 06 26 0C 00 @\r\n#W 00 00 02 08 26 0C 00 @\r\n#W 00 00 02 0A 26 0C 00 @\r\n#W 00 00 02 0D 26 0C 00 @\r\n#M 00 00 00 FF C HARD DISK C@ @\r\n#M 01 00 00 FF D HARD DISK D@ @\r\n#F FF 07 @ *.*@\r\n#T 00 03 03 FF   TRASH@ @\r\n' \
        > "$OUT/hd/EMUDESK.INF"
fi
# shellcheck disable=SC2086
HOME="$OUT/home" SDL_VIDEODRIVER=dummy "$ORACLE_HATARI" \
    --configfile "$ORACLE_HATARI_CFG" $MACHINE --tos "$ORACLE_ROM" \
    --sound off --fast-forward off --conout 2 --run-vbls 60000 \
    -d "$OUT/hd" --gemdos-drive C --auto 'C:\DUMPBIOS.PRG' --cmd-fifo "$OUT/fifo" \
    > "$OUT/hatari.log" 2>&1 &
PID=$!; i=0; sent=0
while kill -0 "$PID" 2>/dev/null; do
    if [ -s "$OUT/hd/BIOS.LOG" ]; then
        last=$(tail -n 1 "$OUT/hd/BIOS.LOG")
        if [ $sent = 0 ] && echo "$last" | grep -Eq '^running [0-9]+ cases'; then
            grep -v '^#' "$HERE/keys-hatari.txt" | grep . | while read -r c; do
                echo "$c" > "$OUT/fifo"; sleep 0.05; done
            sent=1; echo "keys sent at poll $i" > "$OUT/keys-sent.txt"
        elif ! echo "$last" | grep -Eq '^running [0-9]+ cases'; then
            sleep 2; break
        fi
    fi
    i=$((i + 1)); [ "$i" -lt 1200 ] || break
    sleep 0.1
done
kill -9 "$PID" 2>/dev/null || true; wait "$PID" 2>/dev/null || true
rm -rf "$OUT/home"
[ -s "$OUT/hd/BIOS.LOG" ] || { echo "no BIOS.LOG -- see $OUT/hatari.log" >&2; exit 1; }
cp "$OUT/hd/BIOS.LOG" "$OUT/BIOS.LOG"; [ -f "$OUT/hd/BIOS.BIN" ] && cp "$OUT/hd/BIOS.BIN" "$OUT/BIOS.BIN"
cat "$OUT/BIOS.LOG"
