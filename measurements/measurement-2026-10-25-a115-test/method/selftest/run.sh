#!/usr/bin/env bash
# run.sh -- A115: the three unwired scheduler selftests on qemu-virt (release), base (git archive 587e0f68) and new (working tree),
# each in a scratch tree with wire.py; serial -> ../../serial/selftest-<side>.log. Trees removed afterwards.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; . "$HERE/../reg/env.sh"
for side in base new; do
  T="$S/trees/st-$side"; rm -rf "$T"; mkdir -p "$T"
  if [ $side = new ]; then (cd "$WS" && git ls-files -z -co --exclude-standard | tar --null -T - -cf -) | tar -x -C "$T"
  else (cd "$WS" && git archive 587e0f68) | tar -x -C "$T"; fi
  python3 "$HERE/wire.py" "$T"
  (cd "$T" && cargo xtask build --board qemu-virt --release) > "$S/logs/build-st-$side.log" 2>&1 || { tail -30 "$S/logs/build-st-$side.log"; exit 1; }
  ELF=$(ls "$T"/target/aarch64-unknown-none-softfloat/release/kernel)
  shasum -a 256 "$ELF"
  python3 "$HERE/virtrun.py" "$ELF" "$S/st-$side.img" "$HERE/../../serial/selftest-$side.log" "$S/st-$side.png" 90 'desktop up|\[EXC\]|PANIC'
  rm -rf "$T"
done
