#!/usr/bin/env bash
# build-kernel.sh -- scratch tree = `git archive HEAD` (kernel/tos-api unchanged by this checkpoint) + tools/inject-arm.py
# up:DESKTOP=td-shesc.script + probe-conterm.py; release rpi4 kernel -> $S/kernel8-probe.img
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/32d8cb6c-2819-4e51-b178-dafc6b28ed0d/scratchpad/mem12/cp1
WS=/Volumes/Daten/Development/PiRusTOS
T="$S/tree"; rm -rf "$T"; mkdir -p "$T"; (cd "$WS" && git archive HEAD) | tar -x -C "$T"
python3 "$WS/tools/inject-arm.py" "$T" --tag inj up:DESKTOP="$HERE/td-shesc.script" | tail -1
python3 "$HERE/probe-conterm.py" "$T"
export SOURCE_DATE_EPOCH=1792411200
(cd "$T" && cargo xtask build --board rpi4 --release) > "$S/build-kernel.log" 2>&1 || { tail -30 "$S/build-kernel.log"; exit 1; }
cp "$T/kernel8.img" "$S/kernel8-probe.img"; shasum -a 256 "$S/kernel8-probe.img"
