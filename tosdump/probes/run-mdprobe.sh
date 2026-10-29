#!/usr/bin/env bash
#
# run-mdprobe.sh -- the MIDI/IKBD device probe tosdump/probes/mdprobe.c
# (TASK/BIOS04) on one side. No disk: the probe touches no drive but its own.
# It ends waiting in Bconin(3) on the empty MIDI ring (part W): the run stops
# the machine HOLD seconds (wall clock, default 10) after the probe's log
# holds the line "W01 before", and records whether "W01 after" came.
#
#   tosdump/probes/run-mdprobe.sh emutos tt|falcontc <out-dir>
#   tosdump/probes/run-mdprobe.sh newtos <out-dir> <kernel8.img>
#
#   emutos   the pinned oracle, started as tools/hatari-oracle.sh starts
#            profile tt / falcontc (ROM, tools/oracle/hatari.cfg, hermetic
#            HOME, --fast-forward on, GEMDOS drive C:, --auto C:\MDPROBE.PRG;
#            falcontc's EMUDESK.INF); the log is read from the GEMDOS drive
#            while Hatari runs
#   newtos   tools/mkcard.sh (C:\MDPROBE.NAP, C:\NEWDESK.INF = "#R 02",
#            "#Z 01 C:\MDPROBE.NAP@"), QEMU raspi4b booting an APFS clone of
#            the card with `-append serial=on`; the desktop's #Z autorun starts
#            the probe. HOLD starts at the probe's console line
#            "MDPROBE:WAIT" on the serial line; the log is copied from the
#            clone after QEMU is stopped. A kernel built with
#            `--features bios-arg-trace` leaves the BIOS argument trace in
#            BOOT-SERIAL.txt.
#
# <out-dir> (must not exist) receives MDPROBE.LOG, run.txt (what ran, sha256
# of the probe, card, kernel; the W01 verdict) and the side's log (hatari.log,
# or BOOT-SERIAL.txt + card.img.manifest).
#
# Scratch: $MDPROBE_SCRATCH, else $CLAUDE_JOB_DIR/tmp; never /tmp itself.
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
HOLD=${HOLD:-10}

HERE=$(cd "$(dirname "$0")" && pwd)
TD=$(cd "$HERE/.." && pwd)
REPO=$(cd "$TD/.." && pwd)
BASE=${MDPROBE_SCRATCH:-${CLAUDE_JOB_DIR:+$CLAUDE_JOB_DIR/tmp}}
[ -n "$BASE" ] || { echo "set MDPROBE_SCRATCH or CLAUDE_JOB_DIR (scratch, never /tmp)" >&2; exit 2; }
case $(cd "$BASE" 2>/dev/null && pwd -P || echo "$BASE") in
    /tmp|/private/tmp) echo "scratch must not be /tmp itself" >&2; exit 2 ;;
esac
mkdir -p "$BASE"
SCR=$(mktemp -d "$BASE/mdprobe.XXXXXX")
trap 'rm -rf "$SCR"' EXIT

mkdir -p "$OUT"
OUT=$(cd "$OUT" && pwd)
cd "$REPO"
sha() { shasum -a 256 "$1" | cut -d' ' -f1; }

make -s -C tosdump mdprobe-m68k mdprobe-newtos > "$SCR/make.txt" 2>&1 || { cat "$SCR/make.txt" >&2; exit 1; }
PRG=$TD/build/MDPROBE.PRG
NAP=$TD/build/MDPROBE.NAP
{
    echo "mdprobe run, side $SIDE ($(date -u +%Y-%m-%dT%H:%M:%SZ))"
    echo "tree      $(git rev-parse HEAD) $(git status --porcelain -- kernel tos-api tos-types tosdump/probes | grep -c . || true) dirty paths in kernel tos-api tos-types tosdump/probes"
    echo "probes    MDPROBE.PRG $(sha "$PRG") MDPROBE.NAP $(sha "$NAP")"
} > "$OUT/run.txt"

verdict() { # verdict <log>: what part W shows
    if grep -q '^W01 after' "$1"; then
        echo "W01       Bconin(3) on the empty ring RETURNED: $(grep '^W01 after' "$1" | tr -d '\r')"
    elif grep -q '^W01 before' "$1"; then
        echo "W01       Bconin(3) on the empty ring still waiting ${HOLD} s (wall) after 'W01 before'"
    else
        echo "W01       no 'W01 before' line: the probe did not reach part W"
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
    cp "$PRG" "$SCR/hd/MDPROBE.PRG"
    if [ "$PROFILE" = falcontc ]; then
        printf '#R 02\r\n#E 1A E0 00 1C 60\r\n#Q 41 40 43 40 43 40\r\n#W 00 00 02 06 26 0C 00 @\r\n#W 00 00 02 08 26 0C 00 @\r\n#W 00 00 02 0A 26 0C 00 @\r\n#W 00 00 02 0D 26 0C 00 @\r\n#M 00 00 00 FF C HARD DISK C@ @\r\n#M 01 00 00 FF D HARD DISK D@ @\r\n#F FF 07 @ *.*@\r\n#T 00 03 03 FF   TRASH@ @\r\n' \
            > "$SCR/hd/EMUDESK.INF"
    fi
    echo "oracle    $ORACLE_HATARI, ROM $ORACLE_ROM ($ORACLE_ROM_SHA256), profile $PROFILE, cfg $ORACLE_HATARI_CFG ($(sha "$ORACLE_HATARI_CFG"))" >> "$OUT/run.txt"
    # shellcheck disable=SC2086
    HOME="$SCR/home" SDL_VIDEODRIVER=dummy "$ORACLE_HATARI" \
        --configfile "$ORACLE_HATARI_CFG" $MACHINE --tos "$ORACLE_ROM" \
        --sound off --fast-forward on --conout 2 --run-vbls 30000 \
        -d "$SCR/hd" --gemdos-drive C --auto 'C:\MDPROBE.PRG' \
        > "$OUT/hatari.log" 2>&1 &
    pid=$!
    state=timeout
    for _ in $(seq 1 600); do
        if [ -s "$SCR/hd/MDPROBE.LOG" ] && grep -q '^W01 before' "$SCR/hd/MDPROBE.LOG"; then
            state=wait-seen; sleep "$HOLD"; break
        fi
        kill -0 "$pid" 2>/dev/null || { state=hatari-exited; break; }
        sleep 0.5
    done
    kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
    echo "hatari    $state; stopped (SIGKILL)" >> "$OUT/run.txt"
    [ -s "$SCR/hd/MDPROBE.LOG" ] || { echo "no MDPROBE.LOG -- see $OUT/hatari.log" >&2; exit 1; }
    tr -d '\r' < "$SCR/hd/MDPROBE.LOG" > "$OUT/MDPROBE.LOG"
else
    echo "kernel    $(sha "$KERNEL") $KERNEL" >> "$OUT/run.txt"
    printf '#R 02\r\n#Z 01 C:\\MDPROBE.NAP@\r\n' > "$SCR/newdesk.inf"
    tools/mkcard.sh "$SCR/card.img" "$SCR/newdesk.inf" MDPROBE.NAP="$NAP" > "$SCR/mkcard.stdout" 2>&1 || {
        cat "$SCR/mkcard.stdout" >&2; exit 1; }
    cp "$SCR/card.img.manifest" "$OUT/card.img.manifest"
    echo "card      $(sha "$SCR/card.img") (tools/mkcard.sh, C:\\MDPROBE.NAP, #Z autorun)" >> "$OUT/run.txt"
    cp -c "$SCR/card.img" "$SCR/boot.img" 2>/dev/null || cp "$SCR/card.img" "$SCR/boot.img"
    : > "$OUT/BOOT-SERIAL.txt"
    qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$OUT/BOOT-SERIAL.txt" \
        -append serial=on -kernel "$KERNEL" -drive file="$SCR/boot.img",if=sd,format=raw &
    pid=$!
    state=timeout
    for _ in $(seq 1 150); do
        sleep 2
        if grep -q "MDPROBE:WAIT" "$OUT/BOOT-SERIAL.txt"; then state=wait-seen; sleep "$HOLD"; break; fi
        if grep -q "MDPROBE.NAP' exited with rc" "$OUT/BOOT-SERIAL.txt"; then state=probe-exited; sleep 3; break; fi
        kill -0 "$pid" 2>/dev/null || { state=qemu-exited; break; }
    done
    kill "$pid" 2>/dev/null || true; sleep 1; kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
    echo "qemu      $state; stopped" >> "$OUT/run.txt"
    echo "card      after $(sha "$SCR/boot.img") (the booted clone)" >> "$OUT/run.txt"
    # C: = MBR entry 1; its first sector is the LONG at byte 454.
    cstart=$(od -An -t u4 -j 454 -N 4 "$SCR/boot.img" | tr -d ' ')
    MTOOLS_SKIP_CHECK=1 mcopy -o -i "$SCR/boot.img@@$((cstart * 512))" ::/MDPROBE.LOG "$SCR/MDPROBE.LOG" \
        || { echo "no MDPROBE.LOG on C:" >&2; exit 1; }
    tr -d '\r' < "$SCR/MDPROBE.LOG" > "$OUT/MDPROBE.LOG"
fi
verdict "$OUT/MDPROBE.LOG" >> "$OUT/run.txt"
cat "$OUT/MDPROBE.LOG"
tail -1 "$OUT/run.txt"
