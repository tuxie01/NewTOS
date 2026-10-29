#!/bin/sh
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
#                      -- the profile for disk cases, _drvbits C: and D: (0xC).
#                      The profile names C:'s physical unit, IDE master
#                      2 + 8*2 = 18 (bios/disk.h:30), in D:\DISKUNIT.TXT for
#                      DUMPBIOS' unit token u:0 (docs/api/bios/04-rwabs.md
#                      "Decisions"; docs/TOS-REFERENCE-DUMPER.md section 6)
#   dumper   DUMPVDI.PRG, DUMPAES.PRG, DUMPRAST.PRG or DUMPBIOS.PRG (m68k);
#            or a probe, GCPROBE.PRG (tosdump/probes/gcprobe.c, TASK/FS20)
#            or PXPROBE.PRG (tosdump/probes/pxprobe.c, TASK/FS22): no table
#            (give -), profile disk only, always destructive (it rewrites
#            C:'s boot sector), writes <PROBE>.LOG only
#   table    the case table; staged as VDI.TSV / AES.TSV / RASTER.TSV /
#            BIOS.TSV; - for a probe
#   out-dir  receives hd/ (the GEMDOS drive: C:, in profile disk D:),
#            <AREA>.BIN, <AREA>.LOG and hatari.log; must not exist yet. Profile
#            disk adds disk.img (the clone after the run; the caller deletes
#            it) and disk.sha256 (the image before and after the run)
#   rom      defaults to $ORACLE_ROM from tools/oracle/oracle.env
#
# A DESTRUCTIVE table ("!disk <drive> destructive", the disk round's closing
# tables, docs/TOS-REFERENCE-DUMPER.md section 6) changes the test image. It
# runs only in profile disk, on the fresh clone this script makes of
# $HATARI_ORACLE_DISK for the run (out-dir must not exist, so the clone is
# new), and only if that image's partition 1 passes fsck_msdos -n -- an image
# a previous destructive run broke does not. Anything else is refused (exit 2).
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
. "$HERE/oracle/oracle.env"
ROM=${5:-$ORACLE_ROM}

[ -x "$ORACLE_HATARI" ]     || { echo "Hatari not found: $ORACLE_HATARI" >&2; exit 1; }
[ -f "$ORACLE_HATARI_CFG" ] || { echo "Hatari config not found: $ORACLE_HATARI_CFG" >&2; exit 1; }
[ -f "$ROM" ]               || { echo "ROM not found: $ROM" >&2; exit 1; }
[ -f "$DUMPER" ]            || { echo "dumper not found: $DUMPER" >&2; exit 1; }
[ ! -e "$OUT" ]             || { echo "out-dir exists: $OUT" >&2; exit 1; }

PRG=$(basename "$DUMPER" | tr '[:lower:]' '[:upper:]')
PROBE=0
case $PRG in
    DUMPVDI.PRG)  AREA=VDI ;;
    DUMPAES.PRG)  AREA=AES ;;
    DUMPRAST.PRG) AREA=RASTER ;;
    DUMPBIOS.PRG) AREA=BIOS ;;
    GCPROBE.PRG)  AREA=GCPROBE; PROBE=1 ;;
    PXPROBE.PRG)  AREA=PXPROBE; PROBE=1 ;;
    *) echo "unknown dumper: $PRG" >&2; exit 2 ;;
esac
if [ "$PROBE" = 1 ]; then
    [ "$TABLE" = - ] || { echo "$PRG is a probe: it takes no table (give -)" >&2; exit 2; }
else
    [ -f "$TABLE" ] || { echo "table not found: $TABLE" >&2; exit 1; }
fi
if [ "$ROM" = "$ORACLE_ROM" ] &&
   [ "$(shasum -a 256 "$ROM" | cut -d' ' -f1)" != "$ORACLE_ROM_SHA256" ]; then
    echo "pinned ROM does not match ORACLE_ROM_SHA256: $ROM" >&2; exit 1
fi

# Read more broadly than the dumper's loader (any whitespace separates), so a
# table the loader takes as destructive is never run as an ordinary one.
# A probe rewrites C:'s boot sector: destructive by itself.
DESTRUCTIVE=$PROBE
if [ "$PROBE" = 0 ] &&
   grep -Eq '^[[:space:]]*!disk[[:space:]]+[^[:space:]]+[[:space:]]+destructive[[:space:]]*$' "$TABLE"; then
    DESTRUCTIVE=1
fi
if [ "$DESTRUCTIVE" = 1 ]; then
    [ "$PROFILE" = disk ] || {
        echo "destructive table: runs only in profile disk, on a fresh clone of the test image" >&2; exit 2; }
fi

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
cp "$DUMPER" "$OUT/hd/$PRG"
[ "$PROBE" = 1 ] || cp "$TABLE" "$OUT/hd/$AREA.TSV"
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
    if [ "$DESTRUCTIVE" = 1 ]; then
        # partition 1 of the MBR: start and length, little-endian LONGs
        le32() { od -An -t u4 -j "$2" -N 4 "$1" | tr -d ' '; }
        command -v fsck_msdos >/dev/null || {
            echo "destructive table: fsck_msdos is needed to check the test image" >&2; rm -rf "$OUT"; exit 2; }
        dd if="$OUT/disk.img" of="$OUT/testpart-check.img" bs=512 \
           skip="$(le32 "$OUT/disk.img" 454)" count="$(le32 "$OUT/disk.img" 458)" 2>/dev/null
        if ! fsck_msdos -n "$OUT/testpart-check.img" > "$OUT/fsck-before.txt" 2>&1; then
            echo "destructive table: partition 1 of $DISK fails fsck_msdos -n -- not a fresh test image" >&2
            cat "$OUT/fsck-before.txt" >&2
            rm -rf "$OUT"; exit 2
        fi
        rm -f "$OUT/testpart-check.img"
        echo "destructive table: fresh clone, partition 1 clean (fsck_msdos -n)" >> "$OUT/disk.sha256"
    fi
    set -- --ide-master "$OUT/disk.img" --ide-swap 0=off
    # the profile's physical unit of C:, for a u:0 case (header above)
    printf '18\r\n' > "$OUT/hd/DISKUNIT.TXT"
fi

# shellcheck disable=SC2086  # MACHINE is a word list by design
HOME="$OUT/home" SDL_VIDEODRIVER=dummy "$ORACLE_HATARI" \
    --configfile "$ORACLE_HATARI_CFG" \
    $MACHINE --tos "$ROM" \
    --sound off --fast-forward on --conout 2 --run-vbls 30000 "$@" \
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
