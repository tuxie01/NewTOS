#!/usr/bin/env bash
# smoke-call.sh <scratch> -- QEMU raspi4b boot of the 0.7.1 package's KARTE/
# (TASK/REL071). Calls the committed scripts of the REL07 draft run
# measurements/measurement-2026-11-04-rel07-package/method/ unchanged (not
# copied): mkcard-call.sh (tools/mkcard.sh, label NEWTOS, CLIPBRD, every KARTE
# file) and run.sh (headless, -append serial=on, per-run APFS clone, deleted
# after, QMP screendumps at the given wall seconds). Only card-karte is booted.
# Kernel = KARTE/kernel8.img of the unpacked release, nothing else.
set -euo pipefail
S=$1; ROOT=$(cd "$(dirname "$0")/../../.." && pwd); OUT=$(cd "$(dirname "$0")/.." && pwd)
DRAFT=$ROOT/measurements/measurement-2026-11-04-rel07-package/method
K=$S/rel071/NewTOS-0.7.1/KARTE
C=$S/tmp/rel071-cards
"$DRAFT/mkcard-call.sh" "$K" "$C"
mkdir -p "$OUT/cards"
cp "$C"/card-karte.img.manifest "$OUT/cards/"
sed -i '' "s#$S#<scratch>#g" "$OUT"/cards/card-karte.img.manifest
"$DRAFT/run.sh" "$K/kernel8.img" "$C/card-karte.img" karte "$OUT" "$C" 8 22
