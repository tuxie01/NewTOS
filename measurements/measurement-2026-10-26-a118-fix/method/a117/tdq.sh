#!/usr/bin/env bash
# tdq.sh <build|run <side base|new> <n>> -- A118: TeraDesk launch + quit (tdquit.script, armed on up:DESKTOP) on the a117td card
# (TeraDesk #Z + A117ACC.NAC): Desk -> A117 probe, File -> Quit/Shutdown, "Quit" -> TeraDesk ends, NEWDESK comes back.
# Kernels $S/bin/kernel8-<side>-tdq.img (build-kernel.sh); shots quitdlg / onquit / after.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; . "$HERE/env.sh"
case "$1" in
  build)
    ARM=up:DESKTOP bash "$HERE/build-kernel.sh" f42f3719 base-tdq "$HERE/tdquit.script"; rm -rf "$S/trees/base-tdq"
    ARM=up:DESKTOP bash "$HERE/build-kernel.sh" WORKTREE new-tdq "$HERE/tdquit.script"; rm -rf "$S/trees/new-tdq";;
  run)
    SIDE=$2; N=$3
    python3 "$HERE/run.py" "$S/bin/kernel8-$SIDE-tdq.img" "$S/cards/a117td.img" "$SIDE-tdq-$N" "$HERE/../../serial" "$HERE/../../screendumps" 90 \
      1710:2:quitdlg 2710:0.5:onquit 2710:8:after
    rm -rf "$S/cards/run-$SIDE-tdq-$N";;
esac
