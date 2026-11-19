#!/usr/bin/env bash
# mkcards.sh -- PRESENT cards through tools/mkcard.sh: one per PRESENT.NAP build (wt = d82cf2a23, main = 2d00b4bb5, alert = scratch
# variant that refuses 32 planes); SLIDES.TXT + TEST.PNG = the tracked sample; #Z autorun = z-present.inf.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; . "$HERE/env.sh"
for side in ${SIDES:-wt wt-main}; do
  (cd "$WS" && tools/mkcard.sh "$S/cards/pr-$side.img" "$HERE/z-present.inf" \
     Apps/PRESENT/PRESENT.NAP="$S/bin/present-$side/PRESENT.NAP" Apps/PRESENT/SLIDES.TXT="$WS/sdk/apps/present/sample/SLIDES.TXT" \
     Apps/PRESENT/TEST.PNG="$S/bin/present-$side/TEST.PNG")
  mkdir -p "$HERE/manifests"; cp "$S/cards/pr-$side.img.manifest" "$HERE/manifests/"
done
