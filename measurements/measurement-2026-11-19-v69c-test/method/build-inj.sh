#!/usr/bin/env bash
# build-inj.sh -- scratch kernel: copy of the tested tree (mktree.sh) with the IKBD injector armed on PRESENT
# (tools/inject-arm.py up:PRESENT=nav.script), release rpi4 -> $S/bin/k-nav.img
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; WS="${WS:?}"; S="${S:?}"; SC=nav; T="$S/trees/inj-$SC"
export WS SOURCE_DATE_EPOCH=1792368000
python3 "$HERE/gen.py" "$HERE/$SC.spec" "$HERE/$SC.script" >/dev/null
bash "$HERE/mktree.sh" "$T" >/dev/null; git -C "$T" init -q 2>/dev/null
python3 "$WS/tools/inject-arm.py" "$T" --tag inj up:PRESENT="$HERE/$SC.script" | tail -1
(cd "$T" && cargo xtask build --board rpi4 --release) > "$S/logs/build-inj-$SC.log" 2>&1 || { tail -20 "$S/logs/build-inj-$SC.log"; exit 1; }
cp "$T/target/aarch64-unknown-none-softfloat/release/kernel8.img" "$S/bin/k-$SC.img"; shasum -a 256 "$S/bin/k-$SC.img"
