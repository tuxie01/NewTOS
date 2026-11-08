#!/usr/bin/env bash
# version-call.sh <scratch> -- the version check of TASK/REL072 step 1 in QEMU
# raspi4b: boot screen, desktop About box (Ssystem(2019)) and EmuCON `ver`.
# Calls the committed scripts of the REL07 version run
# measurements/measurement-2026-11-04-rel07-version/method/ unchanged (not
# copied), as REL071 did: build-kernel.sh (rpi4 release kernel with the
# scripted IKBD injector, in a scratch copy of the working tree; the repository
# itself is never touched), mkcard-call.sh (tools/mkcard.sh, NEWDESK.INF "#R 02"
# only), run.sh (headless, -append serial=on, per-run APFS clone, deleted after,
# QMP screendumps at the given wall seconds).
set -euo pipefail
S=$1; ROOT=$(cd "$(dirname "$0")/../../.." && pwd); OUT=$(cd "$(dirname "$0")/.." && pwd)/version
M=$ROOT/measurements/measurement-2026-11-04-rel07-version/method
W=$S/rel072-ver; mkdir -p "$W" "$OUT"
"$M/build-kernel.sh" "$ROOT" "$W/tree" desktop="$M/about-ver.script" | tee "$W/kernel.sha256"
K=$W/kernel-inj.img; cp "$W/tree/target/aarch64-unknown-none-softfloat/release/kernel8.img" "$K"
"$M/mkcard-call.sh" "$W/card" | tee "$W/card.sha256"
cp "$W/card/card.img.manifest" "$OUT/"
sed -i '' "s#$S#<scratch>#g" "$OUT/card.img.manifest"
{ echo "kernel-inj.img $(shasum -a 256 "$K" | awk '{print $1}')"
  echo "card.img $(shasum -a 256 "$W/card/card.img" | awk '{print $1}')"; } > "$OUT/sha256.txt"
"$M/run.sh" "$K" "$W/card/card.img" main "$OUT" "$W/card" 10 17 22 29 30 35 42
