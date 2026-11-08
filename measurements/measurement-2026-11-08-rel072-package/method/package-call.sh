#!/usr/bin/env bash
# package-call.sh <scratch> -- the call that built NewTOS-0.7.2.zip (TASK/REL072).
# The tool is tools/release/package-release.sh; this run keeps only its call.
# <scratch>/rel072 = work dir (fresh; ZIP + unpacked folder),
# <scratch>/prev071/NewTOS-0.7.1.zip = the v0.7.1 GitHub asset
# (gh release download v0.7.1 -p NewTOS-0.7.1.zip, sha256
# 801ed08ac7ef3b562ed83f4f89c1054b8e5a3867907dcbd69f38f0cdf8a0ce82
# = measurements/measurement-2026-11-07-rel071-final2/RUNINFO.TXT).
# 0.7.2 changes no included program (nothing under sdk/ or programs/ since
# v0.7.1), so docs/release/v0.7.2/ has no apps/ texts: the two CHANGES.TXT of
# 0.7.1 are declared removed.
set -euo pipefail
S=$1; ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
NAMES_REMOVED="KARTE/APPS/HYPVIEW/CHANGES.TXT KARTE/APPS/COREMARK/CHANGES.TXT" \
KERNEL_FEATURES=crash-walk \
  "$ROOT/tools/release/package-release.sh" 0.7.2 "$S/rel072" "$S/prev071/NewTOS-0.7.1.zip"
