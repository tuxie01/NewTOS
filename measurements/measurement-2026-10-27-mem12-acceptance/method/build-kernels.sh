#!/usr/bin/env bash
# build-kernels.sh -- release rpi4 kernels in scratch trees (git archive HEAD; kernel/tos-api unchanged since 34530791):
#   plain : no instrumentation                                         -> $S/kernel8-plain.img
#   probe : + tools/inject-arm.py up:DESKTOP=td.script up:HYP_VIEW=hv.script launch:DOOM=doom.script launch:P0PAR=p0.script
#           + method/probe-mem12.py (serial [m12p] lines)               -> $S/kernel8-probe.img
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; R="$HERE/.."; WS="$(cd "$R/../.." && pwd)"
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/32d8cb6c-2819-4e51-b178-dafc6b28ed0d/scratchpad/mem12/acc
mkdir -p "$S"; export SOURCE_DATE_EPOCH=1792411200
for k in plain probe; do
  T="$S/tree-$k"; rm -rf "$T"; mkdir -p "$T"; (cd "$WS" && git archive HEAD) | tar -x -C "$T"
  if [ $k = probe ]; then
    python3 "$WS/tools/inject-arm.py" "$T" --tag inj up:DESKTOP="$HERE/td.script" up:HYP_VIEW="$HERE/hv.script" launch:DOOM="$HERE/doom.script" launch:P0PAR="$HERE/p0.script" | tail -1
    python3 "$HERE/probe-mem12.py" "$T"
  fi
  (cd "$T" && cargo xtask build --board rpi4 --release) > "$R/logs/build-kernel-$k.log" 2>&1 || { tail -30 "$R/logs/build-kernel-$k.log"; exit 1; }
  cp "$T/kernel8.img" "$S/kernel8-$k.img"; shasum -a 256 "$S/kernel8-$k.img"
done
