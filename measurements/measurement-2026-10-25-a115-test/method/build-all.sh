#!/usr/bin/env bash
# build-all.sh -- A115: every injector-armed release rpi4 kernel of the run, base (git archive 587e0f68) and new (working tree),
# built one after the other in scratch trees; each tree is removed after its kernel is copied to $S/bin.
#   reg/build-armed.sh  head|wt  -> kernel8-headA / kernel8-wtA (arms: desktop=nd, up:DESKTOP=td, up:QED=qed, up:HYP_VIEW=hv)
#   a117/build-kernel.sh 587e0f68|WORKTREE -> kernel8-base-a117 / -new-a117 (desktop=a117.script), -base-a117td / -new-a117td (up:DESKTOP=td.script)
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; . "$HERE/reg/env.sh"
mkdir -p "$S/bin" "$S/logs"
bash "$HERE/reg/build-armed.sh" head headA; rm -rf "$S/trees/headA"
bash "$HERE/reg/build-armed.sh" wt wtA; rm -rf "$S/trees/wtA"
bash "$HERE/a117/build-kernel.sh" 587e0f68 base-a117 "$HERE/a117/a117.script"; rm -rf "$S/trees/base-a117"
bash "$HERE/a117/build-kernel.sh" WORKTREE new-a117 "$HERE/a117/a117.script"; rm -rf "$S/trees/new-a117"
ARM=up:DESKTOP bash "$HERE/a117/build-kernel.sh" 587e0f68 base-a117td "$HERE/a117/td.script"; rm -rf "$S/trees/base-a117td"
ARM=up:DESKTOP bash "$HERE/a117/build-kernel.sh" WORKTREE new-a117td "$HERE/a117/td.script"; rm -rf "$S/trees/new-a117td"
echo BUILD-ALL DONE
