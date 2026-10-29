#!/usr/bin/env bash
#
# run-axprobe.sh -- the kbdvec-hook-during-Bconin(1) probe tosdump/probes/axprobe.c
# (TASK/XBIOS18) on one side, with its input injected: a key 0.3 s after the
# probe's "AXPROBE:WAIT", the serial byte 'Q' 2 s after the key.
#
#   tosdump/probes/run-axprobe.sh emutos tt|falcontc <out-dir>
#   tosdump/probes/run-axprobe.sh newtos-virt <out-dir> <kernel.img>
#   tosdump/probes/run-axprobe.sh newtos-rpi4 <out-dir> <kernel8.img>
#
#   emutos       the pinned oracle, started as tools/hatari-oracle.sh starts
#                profile tt / falcontc (ROM, tools/oracle/hatari.cfg, hermetic
#                HOME, GEMDOS drive C:, --auto C:\AXPROBE.PRG; falcontc's
#                EMUDESK.INF), with --fast-forward off (real-time VBLs: the
#                injection times are wall-clock), the key through --cmd-fifo
#                ("hatari-event keydown/keyup 30", 'a'), the serial byte
#                through a FIFO on the port EmuTOS maps to AUX:
#                  tt        --rs232-in  (MFP USART, Bconmap device 6)
#                  falcontc  --scc-b-in  (SCC channel B, Bconmap device 7)
#                plus the matching output option (without it Hatari opens
#                /dev/modem and disables the port;
#                measurements/measurement-2026-10-29-xbios15-rx/RUNINFO.TXT).
#                The key goes in when C:\AXPROBE.RDY appears, + 0.3 s.
#   newtos-virt  QEMU virt as `cargo xtask run` starts it (gic-version=2,
#                cortex-a72, 272M, ramfb, qemu-xhci + usb-kbd + usb-mouse),
#                headless. qemu-virt has no drive: <kernel.img> must be a flat
#                qemu-virt image (llvm-objcopy -O binary) whose embedded
#                desktop program is AXPROBE.NAP -- built in a scratch copy of
#                the tree with kernel/embed.rs' NEWDESK entry naming
#                AXPROBE.NAP and the probe copied to
#                target/programs/qemu-virt/ (the script checks that the image
#                carries this AXPROBE.NAP byte for byte). Serial and QMP
#                through probes/axprobe-drive.py; the key is QMP send-key
#                (the usb-kbd), the byte goes on serial0 (AUX, XBIOS15).
#   newtos-rpi4  QEMU raspi4b, -append serial=on, a card from tools/mkcard.sh
#                (C:\AXPROBE.NAP, NEWDESK.INF "#Z 01 C:\AXPROBE.NAP@"), an APFS
#                clone booted. AUX has no cable on that board (Bconin(1)
#                returns 0 at once), and QEMU raspi4b has no USB keyboard: the
#                send-key reaches nothing. The run shows the no-cable answer.
#
# <out-dir> (must not exist) receives AXPROBE.LOG, run.txt (what ran, sha256
# of the probe and kernel) and the side's logs (hatari.log, inject.txt; or
# SERIAL.txt, drive.txt, card.img.manifest).
#
# Scratch: $AXPROBE_SCRATCH, else $CLAUDE_JOB_DIR/tmp; never /tmp itself.
# Deleted at the end.
set -euo pipefail

usage() {
    echo "usage: $0 emutos tt|falcontc <out-dir> | newtos-virt <out-dir> <kernel.img> | newtos-rpi4 <out-dir> <kernel8.img>" >&2
    exit 2
}
[ $# -eq 3 ] || usage
SIDE=$1
case $SIDE in
    emutos) PROFILE=$2; OUT=$3
            case $PROFILE in tt|falcontc) ;; *) usage ;; esac ;;
    newtos-virt|newtos-rpi4)
            OUT=$2; KERNEL=$3; [ -f "$KERNEL" ] || { echo "kernel not found: $KERNEL" >&2; exit 1; } ;;
    *) usage ;;
esac
[ ! -e "$OUT" ] || { echo "out-dir exists: $OUT" >&2; exit 1; }

HERE=$(cd "$(dirname "$0")" && pwd)
TD=$(cd "$HERE/.." && pwd)
REPO=$(cd "$TD/.." && pwd)
BASE=${AXPROBE_SCRATCH:-${CLAUDE_JOB_DIR:+$CLAUDE_JOB_DIR/tmp}}
[ -n "$BASE" ] || { echo "set AXPROBE_SCRATCH or CLAUDE_JOB_DIR (scratch, never /tmp)" >&2; exit 2; }
case $(cd "$BASE" 2>/dev/null && pwd -P || echo "$BASE") in
    /tmp|/private/tmp) echo "scratch must not be /tmp itself" >&2; exit 2 ;;
esac
mkdir -p "$BASE"
SCR=$(mktemp -d "$BASE/axprobe.XXXXXX")
HOLD=
cleanup() {
    [ -n "$HOLD" ] && kill -9 "$HOLD" 2>/dev/null || true
    rm -rf "$SCR"
}
trap cleanup EXIT

mkdir -p "$OUT"
OUT=$(cd "$OUT" && pwd)
cd "$REPO"
sha() { shasum -a 256 "$1" | cut -d' ' -f1; }

make -s -C tosdump axprobe-m68k axprobe-newtos > "$SCR/make.txt" 2>&1 || { cat "$SCR/make.txt" >&2; exit 1; }
PRG=$TD/build/AXPROBE.PRG
NAP=$TD/build/AXPROBE.NAP
{
    echo "axprobe run, side $SIDE ($(date -u +%Y-%m-%dT%H:%M:%SZ))"
    echo "tree      $(git rev-parse HEAD) $(git status --porcelain -- kernel tos-api tos-types tosdump/probes | grep -c . || true) dirty paths in kernel tos-api tos-types tosdump/probes"
    echo "probes    AXPROBE.PRG $(sha "$PRG") AXPROBE.NAP $(sha "$NAP")"
} > "$OUT/run.txt"

# The probe's log as printed on the serial line: the first run only (on
# qemu-virt the AES starts the desktop -- the probe -- again after it ends).
serial_log() {
    awk '/AXPROBE:BEGIN/{n++} n==1' "$1" | tr -d '\r' | sed -n 's/^AXPROBE|//p'
}

case $SIDE in
emutos)
    . "$REPO/tools/oracle/oracle.env"
    [ "$(sha "$ORACLE_ROM")" = "$ORACLE_ROM_SHA256" ] || { echo "pinned ROM mismatch" >&2; exit 1; }
    case $PROFILE in
        tt)       MACHINE="--machine tt --tos-res ttmed --monitor vga"; SERIN=--rs232-in; SEROUT=--rs232-out ;;
        falcontc) MACHINE="--machine falcon --monitor vga"; SERIN=--scc-b-in; SEROUT=--scc-b-out ;;
    esac
    mkdir -p "$SCR/hd" "$SCR/home"
    cp "$PRG" "$SCR/hd/AXPROBE.PRG"
    if [ "$PROFILE" = falcontc ]; then
        printf '#R 02\r\n#E 1A E0 00 1C 60\r\n#Q 41 40 43 40 43 40\r\n#W 00 00 02 06 26 0C 00 @\r\n#W 00 00 02 08 26 0C 00 @\r\n#W 00 00 02 0A 26 0C 00 @\r\n#W 00 00 02 0D 26 0C 00 @\r\n#M 00 00 00 FF C HARD DISK C@ @\r\n#M 01 00 00 FF D HARD DISK D@ @\r\n#F FF 07 @ *.*@\r\n#T 00 03 03 FF   TRASH@ @\r\n' \
            > "$SCR/hd/EMUDESK.INF"
    fi
    mkfifo "$SCR/serin"
    sleep 900 > "$SCR/serin" &
    HOLD=$!; disown "$HOLD"
    echo "oracle    $ORACLE_HATARI, ROM $ORACLE_ROM ($ORACLE_ROM_SHA256), profile $PROFILE, serial $SERIN" >> "$OUT/run.txt"
    # shellcheck disable=SC2086
    HOME="$SCR/home" SDL_VIDEODRIVER=dummy "$ORACLE_HATARI" \
        --configfile "$ORACLE_HATARI_CFG" $MACHINE --tos "$ORACLE_ROM" \
        --sound off --fast-forward off --conout 2 --run-vbls 60000 \
        $SERIN "$SCR/serin" $SEROUT "$SCR/serout.bin" \
        -d "$SCR/hd" --gemdos-drive C --auto 'C:\AXPROBE.PRG' --cmd-fifo "$SCR/cmd" \
        > "$OUT/hatari.log" 2>&1 &
    pid=$!
    i=0; state=sent0
    : > "$OUT/inject.txt"
    while kill -0 "$pid" 2>/dev/null; do
        if [ $state = sent0 ] && [ -e "$SCR/hd/AXPROBE.RDY" ]; then
            sleep 0.3
            echo "hatari-event keydown 30" > "$SCR/cmd"; sleep 0.1
            echo "hatari-event keyup 30" > "$SCR/cmd"
            echo "key 'a' (keydown/keyup 30) at poll $i ($(date +%H:%M:%S))" >> "$OUT/inject.txt"
            sleep 2
            ( printf 'Q' > "$SCR/serin" ) &
            w=$!; sleep 1; kill -0 $w 2>/dev/null && { echo "serial writer still blocked" >> "$OUT/inject.txt"; kill -9 $w; }
            echo "serial 'Q' via $SERIN ($(date +%H:%M:%S))" >> "$OUT/inject.txt"
            state=sent
        fi
        if [ -s "$SCR/hd/AXPROBE.LOG" ] && grep -q '^Z99 end' "$SCR/hd/AXPROBE.LOG"; then
            sleep 1; break
        fi
        i=$((i + 1)); [ "$i" -lt 900 ] || { echo "timeout at poll $i" >> "$OUT/inject.txt"; break; }
        sleep 0.1
    done
    kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
    [ -s "$SCR/hd/AXPROBE.LOG" ] || { echo "no AXPROBE.LOG -- see $OUT/hatari.log" >&2; exit 1; }
    tr -d '\r' < "$SCR/hd/AXPROBE.LOG" > "$OUT/AXPROBE.LOG"
    ;;
newtos-virt)
    echo "kernel    $(sha "$KERNEL") $KERNEL" >> "$OUT/run.txt"
    python3 -I - "$KERNEL" "$NAP" >> "$OUT/run.txt" <<'EOF'
import sys
img = open(sys.argv[1], 'rb').read(); nap = open(sys.argv[2], 'rb').read()
print('embedded  AXPROBE.NAP byte for byte in the image: %s' % (nap in img))
sys.exit(0 if nap in img else 1)
EOF
    python3 -I "$HERE/axprobe-drive.py" "$OUT" 120 -- \
        qemu-system-aarch64 -M virt,gic-version=2 -cpu cortex-a72 -m 272M -device ramfb \
        -device qemu-xhci -device usb-kbd -device usb-mouse \
        -display none @SOCKETS@ -no-reboot -kernel "$KERNEL"
    serial_log "$OUT/SERIAL.txt" > "$OUT/AXPROBE.LOG"
    ;;
newtos-rpi4)
    echo "kernel    $(sha "$KERNEL") $KERNEL" >> "$OUT/run.txt"
    printf '#R 02\r\n#Z 01 C:\\AXPROBE.NAP@\r\n' > "$SCR/newdesk.inf"
    tools/mkcard.sh "$SCR/card.img" "$SCR/newdesk.inf" AXPROBE.NAP="$NAP" > "$SCR/mkcard.stdout" 2>&1 || {
        cat "$SCR/mkcard.stdout" >&2; exit 1; }
    cp "$SCR/card.img.manifest" "$OUT/card.img.manifest"
    echo "card      $(sha "$SCR/card.img") (tools/mkcard.sh, C:\\AXPROBE.NAP, #Z autorun)" >> "$OUT/run.txt"
    cp -c "$SCR/card.img" "$SCR/boot.img" 2>/dev/null || cp "$SCR/card.img" "$SCR/boot.img"
    python3 -I "$HERE/axprobe-drive.py" "$OUT" 240 -- \
        qemu-system-aarch64 -M raspi4b -no-reboot -display none @SOCKETS@ \
        -append serial=on -kernel "$KERNEL" -drive file="$SCR/boot.img",if=sd,format=raw
    serial_log "$OUT/SERIAL.txt" > "$OUT/AXPROBE.LOG"
    # the probe writes C:\AXPROBE.RDY and C:\AXPROBE.LOG to the booted clone
    echo "booted    $(sha "$SCR/boot.img") (the clone after the run)" >> "$OUT/run.txt"
    ;;
esac
[ -s "$OUT/AXPROBE.LOG" ] || { echo "empty AXPROBE.LOG -- see $OUT" >&2; exit 1; }
cat "$OUT/AXPROBE.LOG"
