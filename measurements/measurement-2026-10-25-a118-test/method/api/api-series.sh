#!/usr/bin/env bash
# api-series.sh -- A118: APITEST (#Z autorun) on the base (17cab8b6) and the new plain release rpi4 kernel, memprot on/off/strict
# (MODES overrides), one QEMU at a time; then api-cmp.sh. Adapted from measurements/measurement-2026-10-17-a112-fix/method/api-series.sh.
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; . "$HERE/../reg/env.sh"
for k in base new; do
  for m in ${MODES:-on off strict}; do
    A="serial=on"; [ $m != on ] && A="serial=on memprot=$m"
    TMO=300 "$HERE/api-run.sh" "$S/bin/kernel8-$k.img" "$S/api/api.img" "api-$k-$m" "$A" "TOTAL +[0-9]+" 3
  done
done
MODES="${MODES:-on off strict}" "$HERE/api-cmp.sh"
