#!/usr/bin/env bash
# (A115: copied from measurements/measurement-2026-10-21-gh1-reg/method/run.sh; outputs one level up from method/reg/)
# run.sh <kernel8.img> <base card.img> <tag> STEP... -- one headless QEMU raspi4b run (-append "serial=on ${APPEND_EXTRA}") on an APFS clone
# (cp -c) of the base card; serial -> ../serial/<tag>-serial.log; marker-keyed QMP screendumps (mdrive.py: STEP = 'REGEX@N=DELAY:name.png',
# -> ../screendumps/<tag>-<name>); QEMU killed afterwards, clone deleted. TMO = per-step marker timeout, QDELAY = wait after the last step.
# Copied from measurement-2026-10-18-con01-test/method/m113/run.sh + before the clone is deleted: fsck_msdos -n
# (../serial/<tag>-fsck.txt) and an mtools listing of C:\ C:\SUB C:\TMP or LSDIRS (../serial/<tag>-card.txt; MTYPE="path ..." also
# prints those files).
set -uo pipefail
K="$1"; BASE="$2"; TAG="$3"; shift 3
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; . "$HERE/env.sh"
OUT="${OUTDIR:-$HERE/../../screendumps}"; LOG="$HERE/../../serial/$TAG-serial.log"; mkdir -p "$OUT" "$HERE/../../serial"
mkdir -p "$S/img"; CARD="$S/img/run-$TAG.img"; cp -c "$BASE" "$CARD"; rm -f "$LOG"
cd "$S"; SDIR="$(mktemp -d q.XXXXXX)"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -append "$(echo serial=on ${APPEND_EXTRA:-} | xargs)" \
  -kernel "$K" -drive file="$CARD",if=sd,format=raw -serial file:"$LOG" \
  -qmp unix:"$SDIR/qmp.sock",server=on,wait=off &
QPID=$!
python3 "$HERE/mdrive.py" "$SDIR/qmp.sock" "$OUT" "$TAG" "$LOG" --timeout "${TMO:-150}" --quit-delay "${QDELAY:-0}" "$@"
RC=$?
for _ in $(seq 1 20); do kill -0 "$QPID" 2>/dev/null || break; sleep 0.5; done
kill -9 "$QPID" 2>/dev/null; wait "$QPID" 2>/dev/null
DEV=$(hdiutil attach -imagekey diskimage-class=CRawDiskImage -nomount "$CARD" 2>/dev/null | head -1 | awk '{print $1}')
fsck_msdos -n "${DEV}s1" > "$HERE/../../serial/$TAG-fsck.txt" 2>&1; echo "fsck rc=$?" >> "$HERE/../../serial/$TAG-fsck.txt"
hdiutil detach "$DEV" >/dev/null 2>&1 || true
{ for d in ${LSDIRS:-/ /SUB /TMP}; do echo "== C:$d"; mdir -i "$CARD@@32256" "::$d" 2>&1; done
  for f in ${MTYPE:-}; do echo "== mtype $f"; mtype -i "$CARD@@32256" "::$f" 2>&1 | od -c | head -20; done; } > "$HERE/../../serial/$TAG-card.txt"
for f in ${XFILES:-}; do mcopy -n -i "$CARD@@32256" "::$f" "$HERE/../../serial/$TAG-$(basename $f)" 2>&1; done   # XFILES="SVCSEQ.LOG ..." copied out of the clone (added by the Tester of GH1-reg)
rm -rf "$SDIR"; rm -f "$CARD"
echo "run $TAG: rc=$RC fsck: $(tail -1 "$HERE/../../serial/$TAG-fsck.txt") $(grep -E "files|clusters" "$HERE/../../serial/$TAG-fsck.txt" | tr "\n" " ") EXC/PANIC: $(grep -Ec '\[EXC\]|PANIC|panicked' "$LOG") lines: $(grep -c . "$LOG") sched-switch: $(grep -c '\[sched\] switch' "$LOG")"
