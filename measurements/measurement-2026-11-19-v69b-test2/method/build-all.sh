#!/usr/bin/env bash
# build-all.sh -- the three injector kernels and the zView vditrace kernel (scratch trees, release rpi4)
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; . "$HERE/env.sh"
for sc in open cancel2 ok3; do bash "$HERE/build-inj.sh" $sc || echo "BUILD FAIL $sc"; done
export WS SOURCE_DATE_EPOCH=1792368000
T=$S/trees/vditrace; bash "$HERE/mktree.sh" "$T" >/dev/null; git -C "$T" init -q 2>/dev/null
python3 "$HERE/vditrace.patch.py" "$T"
(cd "$T" && cargo xtask build --board rpi4 --release --features bios-arg-trace) > "$S/logs/build-vditrace.log" 2>&1 || { tail -20 "$S/logs/build-vditrace.log"; echo "BUILD FAIL vditrace"; }
cp "$T/target/aarch64-unknown-none-softfloat/release/kernel8.img" "$S/bin/k-vditrace.img"; shasum -a 256 "$S/bin/k-vditrace.img"
echo ALLDONE
