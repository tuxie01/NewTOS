#!/bin/sh
# controls.sh <scratch> <out-root> -- the measurements behind the disk profile's
# choices (docs/analysis/ORACLE.md section 2, profile disk). Every variant is the
# Hatari command of `tools/hatari-oracle.sh disk` (pinned ROM, hatari.cfg, empty
# HOME, SDL dummy, --conout 2, SIGKILL at the end), changed in exactly the named
# argument(s):
#
#   trace      the profile itself + --trace ide,bios,gemdos  (who reads the image?)
#   swap-auto  without --ide-swap 0=off  (Hatari's default "auto"), + --trace ide
#   swap-on    --ide-swap 0=on, + --trace ide
#   floppy-on  without --drive-a off --drive-b off  (Hatari's default, both on)
#   no-ide     without --ide-master  (where does bit C: come from?)
#   ide-slave  the dumper on a second IDE image (slave) instead of a GEMDOS
#              drive, no -d at all  (can the program drive avoid Hatari's
#              GEMDOS emulation?)
#
# <scratch> must hold test.img (tools/mktestdisk.sh, no files), DUMPBIOS.PRG and
# BIOS.TSV; ide-slave also prog.img (tools/mktestdisk.sh prog.img
# DUMPBIOS.PRG=... BIOS.TSV=...). Each variant runs on its own APFS clone of
# test.img, whose sha256 after the run lands in <out>/<variant>/disk.sha256;
# the clones are deleted at the end.
set -eu
S=$1; ROOT=$2
HERE=$(cd "$(dirname "$0")" && pwd); REPO=$(cd "$HERE/../../.." && pwd)
. "$REPO/tools/oracle/oracle.env"
[ "$(shasum -a 256 "$ORACLE_ROM" | cut -d' ' -f1)" = "$ORACLE_ROM_SHA256" ] || { echo "ROM mismatch" >&2; exit 1; }
mkdir -p "$ROOT"; ROOT=$(cd "$ROOT" && pwd)

run() {   # run <variant> <hatari args...>
    V=$1; shift
    OUT=$ROOT/$V
    [ ! -e "$OUT" ] || { echo "exists: $OUT" >&2; exit 1; }
    mkdir -p "$OUT/hd" "$OUT/home"
    cp "$S/DUMPBIOS.PRG" "$OUT/hd/DUMPBIOS.PRG"; cp "$S/BIOS.TSV" "$OUT/hd/BIOS.TSV"
    cp -c "$S/test.img" "$S/$V.img"
    echo "before $(shasum -a 256 "$S/$V.img" | cut -d' ' -f1)" > "$OUT/disk.sha256"
    HOME="$OUT/home" SDL_VIDEODRIVER=dummy "$ORACLE_HATARI" \
        --configfile "$ORACLE_HATARI_CFG" --tos "$ORACLE_ROM" \
        --sound off --fast-forward on --conout 2 --run-vbls 30000 "$@" \
        > "$OUT/hatari.log" 2>&1 &
    PID=$!
    i=0
    while kill -0 "$PID" 2>/dev/null; do
        if [ -s "$OUT/hd/BIOS.LOG" ] &&
           ! tail -n 1 "$OUT/hd/BIOS.LOG" | grep -Eq '^running [0-9]+ cases'; then
            sleep 2; break
        fi
        i=$((i + 1)); [ "$i" -lt 120 ] || break     # 60 s: the profile needs ~12 s
        sleep 0.5
    done
    kill -9 "$PID" 2>/dev/null || true; wait "$PID" 2>/dev/null || true
    rm -rf "$OUT/home"
    echo "after  $(shasum -a 256 "$S/$V.img" | cut -d' ' -f1)" >> "$OUT/disk.sha256"
    rm -f "$S/$V.img"
    for f in BIOS.LOG BIOS.BIN; do [ -f "$OUT/hd/$f" ] && mv "$OUT/hd/$f" "$OUT/$f"; done
    rm -rf "$OUT/hd"
    sed -i '' "s#$S/##g; s#$ROOT/##g" "$OUT/hatari.log"
    echo "$V: $(grep -a 'GEMDOS-Laufwerke' "$OUT/hatari.log" | sed 's/^ *//') | $(tail -n 1 "$OUT/BIOS.LOG" 2>/dev/null || echo 'no BIOS.LOG')"
}

M="--machine falcon --monitor vga"
NOFL="--drive-a off --drive-b off"
# The image paths contain no blanks (scratch); the argument lists are word lists.
run trace     $M $NOFL --ide-master "$S/trace.img" --ide-swap 0=off \
              -d "$ROOT/trace/hd" --gemdos-drive D --auto 'D:\DUMPBIOS.PRG' \
              --trace ide,bios,gemdos --trace-file "$ROOT/trace/trace.txt"
run swap-auto $M $NOFL --ide-master "$S/swap-auto.img" \
              -d "$ROOT/swap-auto/hd" --gemdos-drive D --auto 'D:\DUMPBIOS.PRG' \
              --trace ide --trace-file "$ROOT/swap-auto/trace.txt"
run swap-on   $M $NOFL --ide-master "$S/swap-on.img" --ide-swap 0=on \
              -d "$ROOT/swap-on/hd" --gemdos-drive D --auto 'D:\DUMPBIOS.PRG' \
              --trace ide --trace-file "$ROOT/swap-on/trace.txt"
run floppy-on $M --ide-master "$S/floppy-on.img" --ide-swap 0=off \
              -d "$ROOT/floppy-on/hd" --gemdos-drive D --auto 'D:\DUMPBIOS.PRG'
run no-ide    $M $NOFL \
              -d "$ROOT/no-ide/hd" --gemdos-drive D --auto 'D:\DUMPBIOS.PRG'
cp -c "$S/prog.img" "$S/prog-run.img"
run ide-slave $M $NOFL --ide-master "$S/ide-slave.img" --ide-swap 0=off \
              --ide-slave "$S/prog-run.img" --ide-swap 1=off --auto 'D:\DUMPBIOS.PRG' \
              --trace gemdos --trace-file "$ROOT/ide-slave/trace.txt"
echo "prog-run.img after: $(shasum -a 256 "$S/prog-run.img" | cut -d' ' -f1)" >> "$ROOT/ide-slave/disk.sha256"
MTOOLSRC=/dev/null mdir -i "$S/prog-run.img@@32256" :: > "$ROOT/ide-slave/prog-dir.txt" 2>&1 || true
rm -f "$S/prog-run.img"
