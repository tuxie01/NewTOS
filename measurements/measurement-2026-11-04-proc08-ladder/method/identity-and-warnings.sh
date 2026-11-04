#!/usr/bin/env bash
# identity-and-warnings.sh <scratch> -- feature identity of the ladder change
# (CLAUDE.md "Testing", debug features): calls
# measurements/measurement-2026-11-02-proc08-diag/method/feature-identity.sh
# unchanged (base = git archive HEAD, aftr = working tree; rpi4/qemu-virt x
# release/dev, feature off, raw binaries, locdiff; feature-on builds and
# clippy of aftr), then the feature-on clippy of base, so the warning counts
# with the feature on have both sides.
set -uo pipefail
SCR=$1
REPO=$(cd "$(dirname "$0")/../../.." && pwd)
export SOURCE_DATE_EPOCH=1793404800
bash "$REPO/measurements/measurement-2026-11-02-proc08-diag/method/feature-identity.sh" "$SCR"
for b in rpi4 qemu-virt; do
  (cd "$SCR/base" && cargo clippy -p kernel --release --features board-$b,crash-walk) > "$SCR/bins/clippy-on-base-$b.log" 2>&1
  echo "clippy on base $b: exit $? $(grep -E '^warning: `(kernel|tos-api)` \(lib\) generated' "$SCR/bins/clippy-on-base-$b.log" | tr '\n' ' ')"
done
