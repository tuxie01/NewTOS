#!/usr/bin/env bash
# QEMU raspi4b smoke of the retest package (not the owner's test): the release kernel of
# kernels/KERNELS.TXT, a fresh copy of the mkcard image (NEWDESK.INF autoruns TeraDesk), serial
# switched on with the SER01 token, one screendump once TeraDesk has drawn its desktop.
# qmp_multi.py is the A41 part-2 driver, copied unchanged
# (measurements/measurement-2026-09-19-a41/part2/method/qmp_multi.py).
# usage (repo root): run-smoke.sh <image> <work dir> ; the QMP socket is a relative path in <work dir>.
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
R="$(cd "$HERE/.." && pwd)"
IMG="$1"; WORK="$2"
mkdir -p "$WORK" "$R/serial" "$R/screendumps"
cp "$IMG" "$WORK/run.img"
LOG="$R/serial/smoke-serial.log"; rm -f "$LOG"
cd "$WORK"
rm -f qmp.sock
qemu-system-aarch64 -M raspi4b -no-reboot -display none \
  -serial file:"$LOG" -kernel "$R/kernels/kern1111.img" -append "serial=on" \
  -drive file=run.img,if=sd,format=raw \
  -qmp unix:qmp.sock,server,nowait &
QPID=$!
python3 "$HERE/qmp_multi.py" qmp.sock "$LOG" "$R/screendumps" 180 \
  'TERADESK\DESKTOP.NAP, sh_nextapp:smoke-tera-colour-icons-idle.png:30'
RC=$?
for _ in $(seq 1 20); do kill -0 "$QPID" 2>/dev/null || break; sleep 0.5; done
kill -9 "$QPID" 2>/dev/null; wait "$QPID" 2>/dev/null
rm -f qmp.sock
echo "smoke: rc=$RC log=$LOG"
exit $RC
