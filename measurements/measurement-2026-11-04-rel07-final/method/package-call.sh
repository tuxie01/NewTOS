#!/usr/bin/env bash
# package-call.sh <scratch> -- the call that built the final NewTOS-0.7.zip
# (TASK/REL07 step 2b). The tool is tools/release/package-release.sh; this run
# keeps only its call. <scratch>/rel07-final = work dir (ZIP + unpacked folder),
# <scratch>/rel061/NewTOS-0.6.1.zip = the v0.6.1 GitHub asset
# (sha256 814a2c8b8e40294bca7ec2b518c7400691ccbd7edaf5078b1e9d0fbfaaf31912).
# Ball is dropped from the release (user decision 2026-11-04, TASK/REL07).
set -euo pipefail
S=$1; ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
NAMES_REMOVED="KARTE/APPS/BALL KARTE/BALL.CFG KARTE/BALL.NTX" \
KERNEL_FEATURES=crash-walk \
  "$ROOT/tools/release/package-release.sh" 0.7 "$S/rel07-final" "$S/rel061/NewTOS-0.6.1.zip"
