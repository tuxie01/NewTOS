#!/bin/sh
# hatari-oracle.sh -- run one tosdump table under EmuTOS on Hatari, headless.
#
#   tools/hatari-oracle.sh <profile> <dumper.PRG> <table.tsv> <out-dir> [rom]
#
#   profile  tt        Atari TT, TT-Medium 640x480x4 planes, VGA monitor --
#                      the machine of the EMUTOS14-* TT references
#            falcontc  Falcon030, VGA, 640x480 16-bit truecolor, set by
#                      EMUDESK.INF "#E 1A E0 00 1C 60" -- the machine of the
#                      *-falcontc references and of NewTOS' colour model
#   dumper   DUMPVDI.PRG, DUMPAES.PRG or DUMPRAST.PRG (m68k build)
#   table    the case table; staged as VDI.TSV / AES.TSV / RASTER.TSV
#   out-dir  receives hd/ (the GEMDOS drive C:), <AREA>.BIN, <AREA>.LOG and
#            hatari.log; must not exist yet
#   rom      defaults to $ORACLE_ROM from tools/oracle/oracle.env
#
# Hermetic on purpose: HOME points at an empty directory so Hatari cannot pick
# up the user's own configuration, and every setting comes from
# $ORACLE_HATARI_CFG plus the command line below. Provenance of each value:
# docs/analysis/ORACLE.md.
#
# The run is over when the dumper's final LOG is on the drive; Hatari is then
# stopped with SIGKILL (SIGTERM opens Hatari's "really quit?" dialog,
# bConfirmQuit defaults to TRUE). --run-vbls and the poll limit
# are only the upper bound for a dumper that never finishes.
set -eu

[ $# -ge 4 ] && [ $# -le 5 ] || {
    echo "usage: $0 tt|falcontc <dumper.PRG> <table.tsv> <out-dir> [rom]" >&2; exit 2; }
PROFILE=$1; DUMPER=$2; TABLE=$3; OUT=$4

HERE=$(cd "$(dirname "$0")" && pwd)
. "$HERE/oracle/oracle.env"
ROM=${5:-$ORACLE_ROM}

[ -x "$ORACLE_HATARI" ]     || { echo "Hatari not found: $ORACLE_HATARI" >&2; exit 1; }
[ -f "$ORACLE_HATARI_CFG" ] || { echo "Hatari config not found: $ORACLE_HATARI_CFG" >&2; exit 1; }
[ -f "$ROM" ]               || { echo "ROM not found: $ROM" >&2; exit 1; }
[ -f "$DUMPER" ]            || { echo "dumper not found: $DUMPER" >&2; exit 1; }
[ -f "$TABLE" ]             || { echo "table not found: $TABLE" >&2; exit 1; }
[ ! -e "$OUT" ]             || { echo "out-dir exists: $OUT" >&2; exit 1; }
if [ "$ROM" = "$ORACLE_ROM" ] &&
   [ "$(shasum -a 256 "$ROM" | cut -d' ' -f1)" != "$ORACLE_ROM_SHA256" ]; then
    echo "pinned ROM does not match ORACLE_ROM_SHA256: $ROM" >&2; exit 1
fi

PRG=$(basename "$DUMPER" | tr '[:lower:]' '[:upper:]')
case $PRG in
    DUMPVDI.PRG)  AREA=VDI ;;
    DUMPAES.PRG)  AREA=AES ;;
    DUMPRAST.PRG) AREA=RASTER ;;
    *) echo "unknown dumper: $PRG" >&2; exit 2 ;;
esac

case $PROFILE in
    tt)       MACHINE="--machine tt --tos-res ttmed --monitor vga" ;;
    falcontc) MACHINE="--machine falcon --monitor vga" ;;
    *) echo "unknown profile: $PROFILE" >&2; exit 2 ;;
esac

mkdir -p "$OUT/hd" "$OUT/home"
OUT=$(cd "$OUT" && pwd)
cp "$DUMPER" "$OUT/hd/$PRG"
cp "$TABLE" "$OUT/hd/$AREA.TSV"
if [ "$PROFILE" = falcontc ]; then
    # 0x001C = VIDEL_VGA|VIDEL_80COL|VIDEL_TRUECOLOR, read by process_inf1
    # (aes/geminit.c). Byte-identical to the INF of the 2026-09-21 runs.
    printf '#R 02\r\n#E 1A E0 00 1C 60\r\n#Q 41 40 43 40 43 40\r\n#W 00 00 02 06 26 0C 00 @\r\n#W 00 00 02 08 26 0C 00 @\r\n#W 00 00 02 0A 26 0C 00 @\r\n#W 00 00 02 0D 26 0C 00 @\r\n#M 00 00 00 FF C HARD DISK C@ @\r\n#M 01 00 00 FF D HARD DISK D@ @\r\n#F FF 07 @ *.*@\r\n#T 00 03 03 FF   TRASH@ @\r\n' \
        > "$OUT/hd/EMUDESK.INF"
fi

# shellcheck disable=SC2086  # MACHINE is a word list by design
HOME="$OUT/home" SDL_VIDEODRIVER=dummy "$ORACLE_HATARI" \
    --configfile "$ORACLE_HATARI_CFG" \
    $MACHINE --tos "$ROM" \
    --sound off --fast-forward on --conout 2 --run-vbls 30000 \
    -d "$OUT/hd" --gemdos-drive C --auto "C:\\$PRG" \
    > "$OUT/hatari.log" 2>&1 &
PID=$!

i=0
while kill -0 "$PID" 2>/dev/null; do
    # The LOG is written twice: once before the first case (ending in
    # "running N cases", so a hang still leaves it) and once at the end.
    if [ -s "$OUT/hd/$AREA.LOG" ] &&
       ! tail -n 1 "$OUT/hd/$AREA.LOG" | grep -Eq '^running [0-9]+ cases'; then
        sleep 2
        kill -9 "$PID" 2>/dev/null || true
        break
    fi
    i=$((i + 1))
    [ "$i" -lt 600 ] || { kill -9 "$PID" 2>/dev/null || true; break; }
    sleep 0.5
done
wait "$PID" 2>/dev/null || true

[ -s "$OUT/hd/$AREA.LOG" ] || { echo "no $AREA.LOG -- run failed, see $OUT/hatari.log" >&2; exit 1; }
cp "$OUT/hd/$AREA.LOG" "$OUT/$AREA.LOG"
[ -f "$OUT/hd/$AREA.BIN" ] && cp "$OUT/hd/$AREA.BIN" "$OUT/$AREA.BIN"
rm -rf "$OUT/home"
cat "$OUT/$AREA.LOG"
