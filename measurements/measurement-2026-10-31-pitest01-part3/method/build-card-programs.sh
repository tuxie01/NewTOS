#!/usr/bin/env bash
# build-card-programs.sh <log-dir> -- rebuild every program of method/pitest01-card.list in one go ("all programs from
# one rebuild", docs/analysis/tester-scenario-draft-2026-10-31.md). Order of docs/sdk/06-image-and-testing.md "Build
# order" for the libraries, then the apps and probes of the list. One log per step in <log-dir>/build-<step>.log; the
# first failing step stops the script. Kernels: `tools/sync-card.sh kernels`, not here.
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; WS="$(cd "$HERE/../../.." && pwd)"
L="${1:?usage: build-card-programs.sh <log-dir>}"; mkdir -p "$L"; L="$(cd "$L" && pwd)"
cd "$WS"
step() { local n="$1"; shift; "$@" > "$L/build-$n.log" 2>&1; local rc=$?; echo "$n rc=$rc"; [ $rc -eq 0 ] || exit 1; }
mkdir -p sdk/examples/build
step libc      sdk/libc/build-libc.sh
step libm      sdk/libm/build-libm.sh
step zview     sdk/zview/build-zview.sh
step qed       sdk/apps/build-qed.sh
step teradesk  sdk/apps/build-teradesk.sh
step hypview   sdk/apps/build-hypview.sh
step doom      sdk/apps/build-doom.sh
step probes    make -C tosdump -B hzprobe-newtos awprobe-newtos dcprobe-newtos
step p0par     sdk/build.sh sdk/examples/p0par.c sdk/examples/build/P0PAR.NAP
step p0kid     sdk/build.sh sdk/examples/p0kid.c sdk/examples/build/P0KID.NAP
step p0rep     sdk/build.sh sdk/examples/p0rep.c sdk/examples/build/P0REP.NAP
echo ALL-OK
