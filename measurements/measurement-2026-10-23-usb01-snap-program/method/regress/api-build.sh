#!/usr/bin/env bash
# api-build.sh <outdir> -- the APITEST programs, built from the shared tree (copied from
# measurements/measurement-2026-10-17-arch02/method/build-programs.sh without the ARCH02 probe):
#   PTERMCH.NCP   sdk/build.sh sdk/examples/ptermch.c    (APITEST T33 child)
#   DTACHILD.NCP  sdk/build.sh sdk/examples/dtachild.c   (APITEST T29 child)
#   APITEST.NCP   sdk/apps/apitest/build-apitest.sh      (rebuilt: the tree copy is stale, 106 tests)
set -euo pipefail
O="${1:?outdir}"; HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; WS=/Volumes/Daten/Development/PiRusTOS
mkdir -p "$O"; cd "$WS"
sdk/build.sh sdk/examples/ptermch.c "$O/PTERMCH.NCP" > /dev/null 2>&1
sdk/build.sh sdk/examples/dtachild.c "$O/DTACHILD.NCP" > /dev/null 2>&1
sdk/apps/apitest/build-apitest.sh "$O/APITEST.NCP" > "$O/apitest-build.log" 2>&1
shasum -a 256 "$O"/*.NCP
