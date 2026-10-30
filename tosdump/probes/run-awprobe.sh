#!/usr/bin/env bash
#
# run-awprobe.sh -- the Bconin(1)-waits probe tosdump/probes/awprobe.c
# (TASK/BIOS09) in QEMU raspi4b, with a serial byte injected <delay-s> after
# the probe's "AWPROBE:WAIT" (default 6 s).
#
#   tosdump/probes/run-awprobe.sh <out-dir> <kernel8.img> [<delay-s>]
#
# QEMU raspi4b, -append serial=on, a card from tools/mkcard.sh (C:\AWPROBE.NAP,
# NEWDESK.INF "#Z 01 C:\AWPROBE.NAP@"), an APFS clone booted, serial0 through
# probes/awprobe-drive.py. The serial byte 'Q' ends the first Bconin(1) wait
# (AUX = device 0's input ring on both boards since BIOS09); the run stops 8 s
# after the probe's second "W" line, i.e. while it waits again.
#
# Keys: QEMU raspi4b has no USB keyboard. For the kbdvec hook to fire during
# the wait, <kernel8.img> must come from a scratch copy of the tree with the
# IKBD injector armed on the probe (never the shared tree):
#   tools/inject-arm.py <scratch-tree> launch:AWPROBE=<script>
#   cargo xtask build --board rpi4 --release   (in <scratch-tree>, own target/)
# <script> lines "<ticks> <byte> ..." after the launch, e.g. 'a' make/break
# (1e 9e) a few seconds in, while the probe waits. A kernel without the
# injector shows the wait and the injected byte, and no hook call.
#
# The real-Pi step (MEM12 check list) needs no script: put AWPROBE.NAP on the
# card, start it (or #Z), press keys, then Ctrl+Alt+Del; see the probe's
# header for what to expect.
#
# <out-dir> (must not exist) receives AWPROBE.LOG (the probe's "AWPROBE|"
# lines from the serial log), run.txt (what ran, sha256 of probe, kernel,
# card), SERIAL.txt, drive.txt, card.img.manifest.
#
# Scratch: $AWPROBE_SCRATCH, else $CLAUDE_JOB_DIR/tmp; never /tmp itself.
# Deleted at the end.
set -euo pipefail

usage() {
    echo "usage: $0 <out-dir> <kernel8.img> [<delay-s>]" >&2
    exit 2
}
[ $# -eq 2 ] || [ $# -eq 3 ] || usage
OUT=$1; KERNEL=$2; DELAY=${3:-6}
[ -f "$KERNEL" ] || { echo "kernel not found: $KERNEL" >&2; exit 1; }
[ ! -e "$OUT" ] || { echo "out-dir exists: $OUT" >&2; exit 1; }

HERE=$(cd "$(dirname "$0")" && pwd)
TD=$(cd "$HERE/.." && pwd)
REPO=$(cd "$TD/.." && pwd)
BASE=${AWPROBE_SCRATCH:-${CLAUDE_JOB_DIR:+$CLAUDE_JOB_DIR/tmp}}
[ -n "$BASE" ] || { echo "set AWPROBE_SCRATCH or CLAUDE_JOB_DIR (scratch, never /tmp)" >&2; exit 2; }
case $(cd "$BASE" 2>/dev/null && pwd -P || echo "$BASE") in
    /tmp|/private/tmp) echo "scratch must not be /tmp itself" >&2; exit 2 ;;
esac
mkdir -p "$BASE"
SCR=$(mktemp -d "$BASE/awprobe.XXXXXX")
trap 'rm -rf "$SCR"' EXIT

mkdir -p "$OUT"
OUT=$(cd "$OUT" && pwd)
cd "$REPO"
sha() { shasum -a 256 "$1" | cut -d' ' -f1; }

make -s -C tosdump awprobe-newtos > "$SCR/make.txt" 2>&1 || { cat "$SCR/make.txt" >&2; exit 1; }
NAP=$TD/build/AWPROBE.NAP
{
    echo "awprobe run ($(date -u +%Y-%m-%dT%H:%M:%SZ))"
    echo "tree      $(git rev-parse HEAD) $(git status --porcelain -- kernel tos-api tos-types tosdump/probes | grep -c . || true) dirty paths in kernel tos-api tos-types tosdump/probes"
    echo "probe     AWPROBE.NAP $(sha "$NAP")"
    echo "kernel    $(sha "$KERNEL") $KERNEL"
    echo "serial    'Q' $DELAY s after AWPROBE:WAIT"
} > "$OUT/run.txt"

printf '#R 02\r\n#Z 01 C:\\AWPROBE.NAP@\r\n' > "$SCR/newdesk.inf"
tools/mkcard.sh "$SCR/card.img" "$SCR/newdesk.inf" AWPROBE.NAP="$NAP" > "$SCR/mkcard.stdout" 2>&1 || {
    cat "$SCR/mkcard.stdout" >&2; exit 1; }
cp "$SCR/card.img.manifest" "$OUT/card.img.manifest"
echo "card      $(sha "$SCR/card.img") (tools/mkcard.sh, C:\\AWPROBE.NAP, #Z autorun)" >> "$OUT/run.txt"
cp -c "$SCR/card.img" "$SCR/boot.img" 2>/dev/null || cp "$SCR/card.img" "$SCR/boot.img"
python3 -I "$HERE/awprobe-drive.py" "$OUT" 240 "$DELAY" -- \
    qemu-system-aarch64 -M raspi4b -no-reboot -display none @SERIAL@ \
    -append serial=on -kernel "$KERNEL" -drive file="$SCR/boot.img",if=sd,format=raw
tr -d '\r' < "$OUT/SERIAL.txt" | sed -n 's/^.*AWPROBE|//p' > "$OUT/AWPROBE.LOG"
# the probe writes C:\AWPROBE.LOG to the booted clone
echo "booted    $(sha "$SCR/boot.img") (the clone after the run)" >> "$OUT/run.txt"
[ -s "$OUT/AWPROBE.LOG" ] || { echo "empty AWPROBE.LOG -- see $OUT" >&2; exit 1; }
cat "$OUT/AWPROBE.LOG"
