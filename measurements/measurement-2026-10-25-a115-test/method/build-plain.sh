#!/usr/bin/env bash
# build-plain.sh -- Tester a115-test: plain (no injector) release rpi4 kernels, base 587e0f68 and new (working tree) -> $S/bin/kernel8-{base,new}.img
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; . "$HERE/reg/env.sh"; export SOURCE_DATE_EPOCH=1792368000
for side in base new; do
  T="$S/trees/plain-$side"; rm -rf "$T"; mkdir -p "$T"
  if [ $side = new ]; then (cd "$WS" && git ls-files -z -co --exclude-standard | tar --null -T - -cf -) | tar -x -C "$T"; else (cd "$WS" && git archive $HEADREV) | tar -x -C "$T"; fi
  (cd "$T" && cargo xtask build --board rpi4 --release) > "$S/logs/build-plain-$side.log" 2>&1 || { tail -30 "$S/logs/build-plain-$side.log"; exit 1; }
  cp "$T/kernel8.img" "$S/bin/kernel8-$side.img"; shasum -a 256 "$S/bin/kernel8-$side.img"; rm -rf "$T"
done
bash "$HERE/api/api-build.sh" "$S/api/bin"
mkdir -p "$S/api"; bash "$HERE/api/api-card.sh" "$S/api/api.img" api "$S/api/bin/APITEST.NCP" "$S/api/bin/PTERMCH.NCP" "$S/api/bin/DTACHILD.NCP"
