#!/bin/sh
# oracle-traced.sh -- TOOL08 probe: a copy of tools/hatari-oracle.sh (HEAD 3d74514b) with two
# additions, marked "TOOL08", and nothing else changed:
#   --trace bios,xbios --trace-file <out-dir>/trace.txt   Hatari's own OS-call trace
#   --parse <out-dir>/hd-dbg/rwabs.ini                    a debugger breakpoint on the pinned
#        ROM's _lrwabs (0x00e01050, references/emutos-oracle/1a7484ab/emutos.map) that dumps
#        the C handler's argument block on every entry, without stopping (:trace :quiet :file)
# The breakpoint files are written by this script; their output lands in <out-dir>/hatari.log.
#
# hatari-oracle.sh -- run one tosdump table under EmuTOS on Hatari, headless.
#
#   tools/hatari-oracle.sh <profile> <dumper.PRG> <table.tsv> <out-dir> [rom]
#   HATARI_ORACLE_DISK=<test.img> tools/hatari-oracle.sh disk ...   (same args)
#
#   profile  tt        Atari TT, TT-Medium 640x480x4 planes, VGA monitor --
#                      the machine of the EMUTOS14-* TT references
#            falcontc  Falcon030, VGA, 640x480 16-bit truecolor, set by
#                      EMUDESK.INF "#E 1A E0 00 1C 60" -- the machine of the
#                      *-falcontc references and of NewTOS' colour model
#            disk      Falcon030, VGA, EmuTOS' default mode (VsetMode 0x001A),
#                      both floppy drives off, the test image $HATARI_ORACLE_DISK
#                      (tools/mktestdisk.sh) cloned to <out-dir>/disk.img as
#                      IDE master = BIOS drive C:, read and written by EmuTOS
#                      alone (no byte swap); the dumper on the GEMDOS drive D:
#                      -- the profile for disk cases, _drvbits C: and D: (0xC)
#   dumper   DUMPVDI.PRG, DUMPAES.PRG, DUMPRAST.PRG or DUMPBIOS.PRG (m68k)
#   table    the case table; staged as VDI.TSV / AES.TSV / RASTER.TSV /
#            BIOS.TSV
#   out-dir  receives hd/ (the GEMDOS drive: C:, in profile disk D:),
#            <AREA>.BIN, <AREA>.LOG and hatari.log; must not exist yet. Profile
#            disk adds disk.img (the clone after the run; the caller deletes
#            it) and disk.sha256 (the image before and after the run)
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
    echo "usage: [HATARI_ORACLE_DISK=<test.img>] $0 tt|falcontc|disk <dumper.PRG> <table.tsv> <out-dir> [rom]" >&2; exit 2; }
PROFILE=$1; DUMPER=$2; TABLE=$3; OUT=$4

HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/../../.." && pwd)
. "$REPO/tools/oracle/oracle.env"
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
    DUMPBIOS.PRG) AREA=BIOS ;;
    *) echo "unknown dumper: $PRG" >&2; exit 2 ;;
esac

DRIVE=C
case $PROFILE in
    tt)       MACHINE="--machine tt --tos-res ttmed --monitor vga" ;;
    falcontc) MACHINE="--machine falcon --monitor vga" ;;
    # The test image is the only BIOS drive besides the dumper's: no floppy
    # (A:/B: would be in _drvbits), IDE master = C:. The dumper sits on a
    # Hatari GEMDOS drive D:, never on the image -- Hatari's --auto needs its
    # GEMDOS emulation, and with it on D: only, every C: call but the
    # desktop's first Fopen of C:\EMUDESK.INF (answered by Hatari's --auto
    # INF override) reaches EmuTOS. Measured: docs/analysis/ORACLE.md section 2.
    disk)     MACHINE="--machine falcon --monitor vga --drive-a off --drive-b off"
              DRIVE=D
              DISK=${HATARI_ORACLE_DISK:-}
              [ -n "$DISK" ] || { echo "profile disk needs HATARI_ORACLE_DISK=<test image>" >&2; exit 2; }
              [ -f "$DISK" ] || { echo "test image not found: $DISK" >&2; exit 1; } ;;
    *) echo "unknown profile: $PROFILE" >&2; exit 2 ;;
esac

mkdir -p "$OUT/hd" "$OUT/home"
OUT=$(cd "$OUT" && pwd)
# TOOL08: the breakpoint on _lrwabs and what it prints -- the six arguments as
# lrwabs(WORD r_w, UBYTE *adr, WORD numb, WORD first, WORD drive, LONG lfirst)
# (bios/bios.c:1045) receives them: 4(sp) r_w, 6 adr, 10 numb, 12 first, 14 drive, 16 lfirst.
mkdir -p "$OUT/hd-dbg"
printf 'b pc = $e01050 :trace :quiet :file %s/hd-dbg/rwabs-args.ini\n' "$OUT" > "$OUT/hd-dbg/rwabs.ini"
printf 'm "a7"-"a7+20"\n' > "$OUT/hd-dbg/rwabs-args.ini"
cp "$DUMPER" "$OUT/hd/$PRG"
cp "$TABLE" "$OUT/hd/$AREA.TSV"
if [ "$PROFILE" = falcontc ]; then
    # 0x001C = VIDEL_VGA|VIDEL_80COL|VIDEL_TRUECOLOR, read by process_inf1
    # (aes/geminit.c). Byte-identical to the INF of the 2026-09-21 runs.
    printf '#R 02\r\n#E 1A E0 00 1C 60\r\n#Q 41 40 43 40 43 40\r\n#W 00 00 02 06 26 0C 00 @\r\n#W 00 00 02 08 26 0C 00 @\r\n#W 00 00 02 0A 26 0C 00 @\r\n#W 00 00 02 0D 26 0C 00 @\r\n#M 00 00 00 FF C HARD DISK C@ @\r\n#M 01 00 00 FF D HARD DISK D@ @\r\n#F FF 07 @ *.*@\r\n#T 00 03 03 FF   TRASH@ @\r\n' \
        > "$OUT/hd/EMUDESK.INF"
fi
set --
if [ "$PROFILE" = disk ]; then
    # A clone (APFS, a full copy elsewhere), so the caller's image stays as it
    # is and the run's own image can be compared with it afterwards.
    cp -c "$DISK" "$OUT/disk.img" 2>/dev/null || cp "$DISK" "$OUT/disk.img"
    echo "before $(shasum -a 256 "$OUT/disk.img" | cut -d' ' -f1) $DISK" > "$OUT/disk.sha256"
    set -- --ide-master "$OUT/disk.img" --ide-swap 0=off
fi

# shellcheck disable=SC2086  # MACHINE is a word list by design
HOME="$OUT/home" SDL_VIDEODRIVER=dummy "$ORACLE_HATARI" \
    --configfile "$ORACLE_HATARI_CFG" \
    $MACHINE --tos "$ROM" \
    --sound off --fast-forward on --conout 2 --run-vbls 30000 "$@" \
    --trace bios,xbios --trace-file "$OUT/trace.txt" --parse "$OUT/hd-dbg/rwabs.ini" \
    -d "$OUT/hd" --gemdos-drive $DRIVE --auto "$DRIVE:\\$PRG" \
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
if [ "$PROFILE" = disk ]; then
    echo "after  $(shasum -a 256 "$OUT/disk.img" | cut -d' ' -f1) $OUT/disk.img" >> "$OUT/disk.sha256"
fi

[ -s "$OUT/hd/$AREA.LOG" ] || { echo "no $AREA.LOG -- run failed, see $OUT/hatari.log" >&2; exit 1; }
cp "$OUT/hd/$AREA.LOG" "$OUT/$AREA.LOG"
[ -f "$OUT/hd/$AREA.BIN" ] && cp "$OUT/hd/$AREA.BIN" "$OUT/$AREA.BIN"
rm -rf "$OUT/home"
cat "$OUT/$AREA.LOG"
[ "$PROFILE" != disk ] || cat "$OUT/disk.sha256"
