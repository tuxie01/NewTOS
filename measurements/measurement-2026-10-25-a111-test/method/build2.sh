#!/usr/bin/env bash
# build2.sh <rev> <name> ARM... : fresh release rpi4 kernel from git archive <rev> with several injector arms
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; . "$HERE/env.sh"
REV="$1"; NAME="$2"; shift 2; export SOURCE_DATE_EPOCH=1792368000
T="$S/trees/$NAME"; rm -rf "$T"; mkdir -p "$T" "$S/bin" "$S/logs"
(cd "$WS" && git archive "$REV") | tar -x -C "$T"
mkdir -p "$T/references"; ln -s "$WS/references/emutos" "$T/references/emutos"
for d in "$WS"/sdk/apps/*/; do d=${d%/}; [ -e "$T/sdk/apps/$(basename $d)" ] || ln -s "$d" "$T/sdk/apps/$(basename $d)"; done
python3 "$WS/tools/inject-arm.py" "$T" --tag inj "$@"
(cd "$T" && cargo xtask build --board rpi4 --release) > "$S/logs/build-$NAME.log" 2>&1 || { tail -30 "$S/logs/build-$NAME.log"; exit 1; }
cp "$T/kernel8.img" "$S/bin/kernel8-$NAME.img"; shasum -a 256 "$S/bin/kernel8-$NAME.img"
