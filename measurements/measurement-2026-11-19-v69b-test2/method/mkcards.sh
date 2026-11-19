#!/usr/bin/env bash
# mkcards.sh -- TeraDesk-only cards through tools/mkcard.sh, one per DESKTOP.NAP build (shadow builds of agent dir:
# $A/shadow/build-wt = branch d237b0a3b overlay, build-wt-main = main 9d90f0589 overlay = unpatched); #Z autorun = z-teradesk.inf.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; . "$HERE/env.sh"
for side in wt main; do
  D=$A/shadow/build-$([ $side = main ] && echo wt-main || echo wt)/dist
  (cd "$WS" && tools/mkcard.sh "$S/cards/td-$side.img" "$HERE/z-teradesk.inf" \
     Apps/TERADESK/DESKTOP.NAP="$D/DESKTOP.NAP" Apps/TERADESK/desktop.rsc="$D/desktop.rsc" \
     Apps/TERADESK/icons.rsc="$D/icons.rsc" Apps/TERADESK/cicons.rsc="$D/cicons.rsc" \
     Apps/TERADESK/TERADESK.INF="$HERE/TERADESK.INF")
done
