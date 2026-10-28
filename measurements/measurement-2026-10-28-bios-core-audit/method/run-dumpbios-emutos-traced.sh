#!/bin/sh
# run-dumpbios-emutos-traced.sh <repo> <DUMPBIOS.PRG> <table.tsv> <out-dir> -- the EmuTOS
# input check for a DUMPBIOS table: the same start as tools/hatari-oracle.sh tt
# (pinned ROM, hatari.cfg, empty HOME, SDL dummy, --conout 2, GEMDOS drive C:,
# --auto, SIGKILL) plus --trace bios,xbios into <out>/trace.txt.  The BIN of this
# run must equal the reference (cmp) for the trace to speak for it.
set -eu
REPO=$1; PRG=$2; T=$3; OUT=$4
. "$REPO/tools/oracle/oracle.env"
[ "$(shasum -a 256 "$ORACLE_ROM" | cut -d' ' -f1)" = "$ORACLE_ROM_SHA256" ] || { echo "ROM mismatch" >&2; exit 1; }
[ ! -e "$OUT" ] || { echo "out exists" >&2; exit 1; }
mkdir -p "$OUT/hd" "$OUT/home"; OUT=$(cd "$OUT" && pwd)
cp "$PRG" "$OUT/hd/DUMPBIOS.PRG"; cp "$T" "$OUT/hd/BIOS.TSV"
HOME="$OUT/home" SDL_VIDEODRIVER=dummy "$ORACLE_HATARI" \
    --configfile "$ORACLE_HATARI_CFG" --machine tt --tos-res ttmed --monitor vga --tos "$ORACLE_ROM" \
    --sound off --fast-forward on --conout 2 --run-vbls 30000 \
    -d "$OUT/hd" --gemdos-drive C --auto 'C:\DUMPBIOS.PRG' \
    --trace bios,xbios --trace-file "$OUT/trace.txt" > "$OUT/hatari.log" 2>&1 &
PID=$!; i=0
while kill -0 "$PID" 2>/dev/null; do
    if [ -s "$OUT/hd/BIOS.LOG" ] && ! tail -n 1 "$OUT/hd/BIOS.LOG" | grep -Eq '^running [0-9]+ cases'; then sleep 2; break; fi
    i=$((i + 1)); [ "$i" -lt 600 ] || break; sleep 0.5
done
kill -9 "$PID" 2>/dev/null || true; wait "$PID" 2>/dev/null || true
rm -rf "$OUT/home"; cp "$OUT/hd/BIOS.LOG" "$OUT/hd/BIOS.BIN" "$OUT/"
