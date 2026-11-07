#!/usr/bin/env bash
# arm-build2.sh <fix|base> <script> <kernel-out> <trigger e.g. up:FBENCH> : scratch copy of the tree armed with tools/inject-arm.py desktop=<script>, release rpi4.
set -euo pipefail
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/7fba4f48-e8f1-4a5e-8c58-696d17828400/scratchpad/t
REPO=/Volumes/Daten/Development/PiRusTOS
T=$S/armx-$1-$(basename $2 .txt)
rm -rf "$T"; mkdir -p "$T"; (cd "$S/$1" && tar --exclude=./target --exclude=./.git -cf - .) | tar -xf - -C "$T"
ln -sfn $REPO/references "$T/references"
python3 -I $REPO/tools/inject-arm.py "$T" "$4=$2" | tail -1
cd "$T" && cargo xtask build-programs --board rpi4 > /dev/null 2>&1 && cargo xtask build --board rpi4 --release > "$S/arm-$1.log" 2>&1 || { tail -20 "$S/arm-$1.log"; exit 1; }
cp target/aarch64-unknown-none-softfloat/release/kernel8.img "$3"
echo "armed $1 kernel $(shasum -a 256 "$3" | cut -d' ' -f1) script $(shasum -a 256 "$2" | cut -d' ' -f1)"
