#!/bin/bash
# Copied from measurements/measurement-2026-10-17-a112-fix/method/run-all.sh (A115: paths one level deeper).
# usage: run-all.sh <kernel8.img> <tag red|green> <scratch-dir>
# Builds the probe roles with sdk/build.sh into <scratch>/bin, one FAT16 base
# image per case into <scratch>/img, runs each case once (QEMU raspi4b,
# serial=on, host timestamps) and writes ../serial/<case>-<tag>.log.
#   f1  ACCA (evnt_multi(MU_TIMER,250) loop) + MAIN               75 s
#   f1b ACCD (evnt_timer(4000) loop) + MAIN (MESAG|TIMER,1000)x30  75 s
#   f2  ACCB (evnt_multi(MU_MESAG)) + ACCC (evnt_mesag) + MAIN_LATE 60 s
#       (MAIN_LATE: covers the accessories' windows after evnt_timer(9000))
#   f3  ACCD_FAST (evnt_timer(1000) loop) + POLL                   90 s
set -e
K=$1; TAG=$2; W=$3
HERE="$(cd "$(dirname "$0")" && pwd)"; SDK="$HERE/../../../../sdk"; SER="$HERE/../../serial"
mkdir -p "$W/bin" "$W/img" "$SER"
b() { bash "$SDK/build.sh" "$HERE/r_$1.c" "$W/bin/$2" >/dev/null; }
b ACCA ACCA.NAC; b ACCB ACCB.NAC; b ACCC ACCC.NAC; b ACCD ACCD.NAC; b ACCDF ACCD_F.NAC
b MAIN MAIN.NAP; b MAIN3 MAIN3.NAP; b POLL POLL.NAP
shasum -a 256 "$W"/bin/* | sed "s|$W/||" > "$SER/probes-$TAG.sha256"
mk() { local img=$1; shift; [ -f "$img" ] || bash "$HERE/mkimg.sh" "$img" "$@" >/dev/null; }
mk "$W/img/f1.img"  "$W/bin/ACCA.NAC" "$W/bin/MAIN.NAP"
mk "$W/img/f1b.img" "$W/bin/ACCD.NAC" "$W/bin/MAIN.NAP"
mkdir -p "$W/f2"; cp "$W/bin/MAIN3.NAP" "$W/f2/MAIN.NAP"
mk "$W/img/f2.img"  "$W/bin/ACCB.NAC" "$W/bin/ACCC.NAC" "$W/f2/MAIN.NAP"
mkdir -p "$W/f3"; cp "$W/bin/ACCD_F.NAC" "$W/f3/ACCD.NAC"; cp "$W/bin/POLL.NAP" "$W/f3/MAIN.NAP"
mk "$W/img/f3.img"  "$W/f3/ACCD.NAC" "$W/f3/MAIN.NAP"
for c in ${CASES:-f1 f1b f2 f3}; do
  case $c in f1|f1b) s=75;; f2) s=60;; f3) s=90;; esac
  echo "== $c ($TAG) ${s}s"
  bash "$HERE/runts.sh" "$K" "$W/img/$c.img" "$SER/$c-$TAG.log" $s
done
