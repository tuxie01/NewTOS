#!/usr/bin/env bash
#
# run-xsseed.sh -- Random's first-seed probe tosdump/probes/xsseed.c
# (TASK/XBIOS11, docs/api/xbios/11-random.md "Acceptance probe") on one side.
# No disk: the probe touches no drive but its own. It must be the first
# program after boot (no earlier Random() may have moved the state): the
# oracle's --auto and NewTOS's #Z autorun start it. The probe terminates;
# the run stops the machine once the log is complete (Hatari: the "X01 info"
# line in XSSEED.LOG; NewTOS: the shell's "XSSEED.NAP' exited with rc" on
# the serial line).
#
#   tosdump/probes/run-xsseed.sh emutos tt|falcontc <out-dir>
#   tosdump/probes/run-xsseed.sh newtos <out-dir> <kernel8.img>
#
#   emutos   the pinned oracle, started as tools/hatari-oracle.sh starts
#            profile tt / falcontc (ROM, tools/oracle/hatari.cfg, hermetic
#            HOME, --fast-forward on, GEMDOS drive C:, --auto C:\XSSEED.PRG;
#            falcontc's EMUDESK.INF); the log is read from the GEMDOS drive
#            while Hatari runs
#   newtos   tools/mkcard.sh (C:\XSSEED.NAP, C:\NEWDESK.INF = "#R 02",
#            "#Z 01 C:\XSSEED.NAP@"), QEMU raspi4b booting an APFS clone of
#            the card with `-append serial=on`; the desktop's #Z autorun starts
#            the probe. The log is copied from the clone after QEMU is
#            stopped. A kernel built with `--features bios-arg-trace` leaves
#            the BIOS/XBIOS argument trace in BOOT-SERIAL.txt.
#
# <out-dir> (must not exist) receives XSSEED.LOG, run.txt (what ran, sha256
# of the probe, card, kernel; the end verdict) and the side's log (hatari.log,
# or BOOT-SERIAL.txt + card.img.manifest).
#
# Scratch: $XSSEED_SCRATCH, else $CLAUDE_JOB_DIR/tmp; never /tmp itself.
# Deleted at the end.
set -euo pipefail

usage() { echo "usage: $0 emutos tt|falcontc <out-dir> | newtos <out-dir> <kernel8.img>" >&2; exit 2; }
[ $# -eq 3 ] || usage
SIDE=$1
case $SIDE in
    emutos) PROFILE=$2; OUT=$3
            case $PROFILE in tt|falcontc) ;; *) usage ;; esac ;;
    newtos) OUT=$2; KERNEL=$3; [ -f "$KERNEL" ] || { echo "kernel not found: $KERNEL" >&2; exit 1; } ;;
    *) usage ;;
esac
[ ! -e "$OUT" ] || { echo "out-dir exists: $OUT" >&2; exit 1; }

HERE=$(cd "$(dirname "$0")" && pwd)
TD=$(cd "$HERE/.." && pwd)
REPO=$(cd "$TD/.." && pwd)
BASE=${XSSEED_SCRATCH:-${CLAUDE_JOB_DIR:+$CLAUDE_JOB_DIR/tmp}}
[ -n "$BASE" ] || { echo "set XSSEED_SCRATCH or CLAUDE_JOB_DIR (scratch, never /tmp)" >&2; exit 2; }
case $(cd "$BASE" 2>/dev/null && pwd -P || echo "$BASE") in
    /tmp|/private/tmp) echo "scratch must not be /tmp itself" >&2; exit 2 ;;
esac
mkdir -p "$BASE"
SCR=$(mktemp -d "$BASE/xsseed.XXXXXX")
trap 'rm -rf "$SCR"' EXIT

mkdir -p "$OUT"
OUT=$(cd "$OUT" && pwd)
cd "$REPO"
sha() { shasum -a 256 "$1" | cut -d' ' -f1; }

make -s -C tosdump xsseed-m68k xsseed-newtos > "$SCR/make.txt" 2>&1 || { cat "$SCR/make.txt" >&2; exit 1; }
PRG=$TD/build/XSSEED.PRG
NAP=$TD/build/XSSEED.NAP
{
    echo "xsseed run, side $SIDE ($(date -u +%Y-%m-%dT%H:%M:%SZ))"
    echo "tree      $(git rev-parse HEAD) $(git status --porcelain -- kernel tos-api tos-types tosdump/probes | grep -c . || true) dirty paths in kernel tos-api tos-types tosdump/probes"
    echo "probes    XSSEED.PRG $(sha "$PRG") XSSEED.NAP $(sha "$NAP")"
} > "$OUT/run.txt"

verdict() { # verdict <log>: the compared X01 line (the info line is not compared)
    if grep -q '^X01 info' "$1"; then
        echo "end       $(grep '^X01 first' "$1" | tr -d '\r')"
    else
        echo "end       no 'X01 info' line: the probe did not finish"
    fi
}

if [ "$SIDE" = emutos ]; then
    . "$REPO/tools/oracle/oracle.env"
    [ "$(sha "$ORACLE_ROM")" = "$ORACLE_ROM_SHA256" ] || { echo "pinned ROM mismatch" >&2; exit 1; }
    case $PROFILE in
        tt)       MACHINE="--machine tt --tos-res ttmed --monitor vga" ;;
        falcontc) MACHINE="--machine falcon --monitor vga" ;;
    esac
    mkdir -p "$SCR/hd" "$SCR/home"
    cp "$PRG" "$SCR/hd/XSSEED.PRG"
    if [ "$PROFILE" = falcontc ]; then
        printf '#R 02\r\n#E 1A E0 00 1C 60\r\n#Q 41 40 43 40 43 40\r\n#W 00 00 02 06 26 0C 00 @\r\n#W 00 00 02 08 26 0C 00 @\r\n#W 00 00 02 0A 26 0C 00 @\r\n#W 00 00 02 0D 26 0C 00 @\r\n#M 00 00 00 FF C HARD DISK C@ @\r\n#M 01 00 00 FF D HARD DISK D@ @\r\n#F FF 07 @ *.*@\r\n#T 00 03 03 FF   TRASH@ @\r\n' \
            > "$SCR/hd/EMUDESK.INF"
    fi
    echo "oracle    $ORACLE_HATARI, ROM $ORACLE_ROM ($ORACLE_ROM_SHA256), profile $PROFILE, cfg $ORACLE_HATARI_CFG ($(sha "$ORACLE_HATARI_CFG"))" >> "$OUT/run.txt"
    # shellcheck disable=SC2086
    HOME="$SCR/home" SDL_VIDEODRIVER=dummy "$ORACLE_HATARI" \
        --configfile "$ORACLE_HATARI_CFG" $MACHINE --tos "$ORACLE_ROM" \
        --sound off --fast-forward on --conout 2 --run-vbls 30000 \
        -d "$SCR/hd" --gemdos-drive C --auto 'C:\XSSEED.PRG' \
        > "$OUT/hatari.log" 2>&1 &
    pid=$!
    state=timeout
    for _ in $(seq 1 600); do
        if [ -s "$SCR/hd/XSSEED.LOG" ] && grep -q '^X01 info' "$SCR/hd/XSSEED.LOG"; then
            state=end-seen; sleep 1; break
        fi
        kill -0 "$pid" 2>/dev/null || { state=hatari-exited; break; }
        sleep 0.5
    done
    kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
    echo "hatari    $state; stopped (SIGKILL)" >> "$OUT/run.txt"
    [ -s "$SCR/hd/XSSEED.LOG" ] || { echo "no XSSEED.LOG -- see $OUT/hatari.log" >&2; exit 1; }
    tr -d '\r' < "$SCR/hd/XSSEED.LOG" > "$OUT/XSSEED.LOG"
else
    echo "kernel    $(sha "$KERNEL") $KERNEL" >> "$OUT/run.txt"
    printf '#R 02\r\n#Z 01 C:\\XSSEED.NAP@\r\n' > "$SCR/newdesk.inf"
    tools/mkcard.sh "$SCR/card.img" "$SCR/newdesk.inf" XSSEED.NAP="$NAP" > "$SCR/mkcard.stdout" 2>&1 || {
        cat "$SCR/mkcard.stdout" >&2; exit 1; }
    cp "$SCR/card.img.manifest" "$OUT/card.img.manifest"
    echo "card      $(sha "$SCR/card.img") (tools/mkcard.sh, C:\\XSSEED.NAP, #Z autorun)" >> "$OUT/run.txt"
    cp -c "$SCR/card.img" "$SCR/boot.img" 2>/dev/null || cp "$SCR/card.img" "$SCR/boot.img"
    : > "$OUT/BOOT-SERIAL.txt"
    qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$OUT/BOOT-SERIAL.txt" \
        -append serial=on -kernel "$KERNEL" -drive file="$SCR/boot.img",if=sd,format=raw &
    pid=$!
    state=timeout
    for _ in $(seq 1 150); do
        sleep 2
        if grep -q "XSSEED.NAP' exited with rc" "$OUT/BOOT-SERIAL.txt"; then state=probe-exited; sleep 3; break; fi
        kill -0 "$pid" 2>/dev/null || { state=qemu-exited; break; }
    done
    kill "$pid" 2>/dev/null || true; sleep 1; kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
    echo "qemu      $state; stopped" >> "$OUT/run.txt"
    echo "card      after $(sha "$SCR/boot.img") (the booted clone)" >> "$OUT/run.txt"
    # C: = MBR entry 1; its first sector is the LONG at byte 454.
    cstart=$(od -An -t u4 -j 454 -N 4 "$SCR/boot.img" | tr -d ' ')
    MTOOLS_SKIP_CHECK=1 mcopy -o -i "$SCR/boot.img@@$((cstart * 512))" ::/XSSEED.LOG "$SCR/XSSEED.LOG" \
        || { echo "no XSSEED.LOG on C:" >&2; exit 1; }
    tr -d '\r' < "$SCR/XSSEED.LOG" > "$OUT/XSSEED.LOG"
fi
verdict "$OUT/XSSEED.LOG" >> "$OUT/run.txt"
cat "$OUT/XSSEED.LOG"
tail -1 "$OUT/run.txt"
