#!/usr/bin/env bash
# fmerr.sh <card|run> -- A118: the shell loop's fm_error alert, now issued as the AES call form_error through the GEM trap from
# EL1 (kernel/src/shell_loop.rs fm_error). Card $S/cards/nope.img: NEWDESK.INF with "#Z 01 C:\NOPE.NAP@" (a program that does
# not exist) -> sh_ldapp's sh_find fails (EFILNF) -> the next sh_main pass shows fm_error(2) ("file not found").
# run: plain release kernels base/new (lint-both.sh), ../api/api-run.sh: screendump 4 s after "[shell] sh_find: ... not found"
# (the alert waits for a click, so "[desk] desktop up" never comes; the 60 s wait for it runs out first).
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; . "$HERE/../reg/env.sh"
case "$1" in
  card) bash "$HERE/../reg/mkcard.sh" "$S/cards/nope.img" "$HERE/z-nope.inf" >/dev/null; shasum -a 256 "$S/cards/nope.img";;
  run) for side in base new; do TMO=120 bash "$HERE/../api/api-run.sh" "$S/bin/kernel8-$side.img" "$S/cards/nope.img" "fmerr-$side" "serial=on" "sh_find: .* not found" 4; done;;
esac
