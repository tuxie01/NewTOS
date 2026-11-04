#!/usr/bin/env bash
# smoke-call.sh <scratch> -- QEMU raspi4b smoke test of the final package's
# KARTE/ (TASK/REL07 step 2b). Calls the committed scripts of the draft run
# measurements/measurement-2026-11-04-rel07-package/method/ unchanged (not
# copied): mkcard-call.sh (tools/mkcard.sh, label NEWTOS, CLIPBRD, every KARTE
# file; card-zview = KARTE plus one "#Z 01 C:\APPS\ZVIEW\ZVIEW.NAP@" line in
# NEWDESK.INF) and run.sh (headless, -append serial=on, per-run APFS clone,
# deleted after, QMP screendumps at the given wall seconds).
# Kernel = KARTE/kernel8.img of the unpacked release, nothing else.
set -euo pipefail
S=$1; ROOT=$(cd "$(dirname "$0")/../../.." && pwd); OUT=$(cd "$(dirname "$0")/.." && pwd)
DRAFT=$ROOT/measurements/measurement-2026-11-04-rel07-package/method
K=$S/rel07-final/NewTOS-0.7/KARTE
C=$S/rel07-final-cards
"$DRAFT/mkcard-call.sh" "$K" "$C"
cp "$C"/card-*.img.manifest "$C"/newdesk-*.inf "$OUT/cards/"
sed -i '' "s#$S#<scratch>#g" "$OUT"/cards/card-*.img.manifest
"$DRAFT/run.sh" "$K/kernel8.img" "$C/card-karte.img" karte "$OUT" "$C" 8 22
"$DRAFT/run.sh" "$K/kernel8.img" "$C/card-zview.img" zview "$OUT" "$C" 30
