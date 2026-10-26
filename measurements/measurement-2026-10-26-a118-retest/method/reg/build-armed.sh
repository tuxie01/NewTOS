#!/usr/bin/env bash
# (A115: copied from measurements/measurement-2026-10-21-gh1-reg/method/build-armed.sh; outputs one level up from method/reg/)
# build-armed.sh <head|wt> <name> -- fresh scratch tree + all four injector arms (desktop=nd, up:DESKTOP=td, up:QED=qed, up:HYP_VIEW=hv), release rpi4 kernel -> $S/bin/kernel8-<name>.img
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; . "$HERE/env.sh"; export SOURCE_DATE_EPOCH=1792368000
bash "$HERE/mktree.sh" "$1" "$2" >/dev/null
T="$S/trees/$2"; rm -rf "$T/target" "$T/kernel8.img"
python3 "$WS/tools/inject-arm.py" "$T" --tag inj desktop="$HERE/nd.script" up:DESKTOP="$HERE/td.script" up:QED="$HERE/qed.script" up:HYP_VIEW="$HERE/hv.script" | tail -1
(cd "$T" && cargo xtask build --board rpi4 --release) > "$S/logs/build-$2.log" 2>&1 || { tail -30 "$S/logs/build-$2.log"; exit 1; }
cp "$T/kernel8.img" "$S/bin/kernel8-$2.img"; shasum -a 256 "$S/bin/kernel8-$2.img"
