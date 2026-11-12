#!/usr/bin/env bash
# run.sh KERNEL8 IMG OUTDIR TAG TIMEOUT_S SPEC...   (SPEC = MARKER:PNG[:SETTLE])
# QEMU raspi4b, headless, serial to file, QMP screendumps at serial markers
# (qmp_multi.py, from measurement-2026-09-20-a78/method). Every run boots a
# fresh copy of IMG so the image under test is never written.
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
KERNEL8="$1"; IMG="$2"; OUTDIR="$3"; TAG="$4"; TMO="$5"; shift 5
mkdir -p "$OUTDIR"
LOG="$OUTDIR/$TAG-serial.log"
SOCK="/Volumes/Daten/Development/PiRusTOS/target-agents/agent-a5362730566fb85e9/a81-$TAG.sock"
RUNIMG="$OUTDIR/$TAG-run.img"
cp "$IMG" "$RUNIMG"
rm -f "$LOG" "$SOCK"
qemu-system-aarch64 -M raspi4b -no-reboot -display none \
  -serial file:"$LOG" -append serial=on -kernel "$KERNEL8" \
  -drive file="$RUNIMG",if=sd,format=raw \
  -qmp unix:"$SOCK",server,nowait &
QPID=$!
python3 "$HERE/qmp_multi.py" "$SOCK" "$LOG" "$OUTDIR" "$TMO" "$@"
RC=$?
for _ in $(seq 1 20); do kill -0 "$QPID" 2>/dev/null || break; sleep 0.5; done
kill -9 "$QPID" 2>/dev/null; wait "$QPID" 2>/dev/null
rm -f "$SOCK" "$RUNIMG"
echo "run $TAG: rc=$RC log=$LOG"
exit $RC
