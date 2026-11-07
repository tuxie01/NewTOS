#!/usr/bin/env bash
# package-call.sh <scratch> -- the call that built NewTOS-0.7.1.zip (TASK/REL071).
# The tool is tools/release/package-release.sh; this run keeps only its call.
# <scratch>/rel071 = work dir (fresh; ZIP + unpacked folder),
# <scratch>/prev07/NewTOS-0.7.zip = the v0.7 GitHub asset
# (gh release download v0.7 -p NewTOS-0.7.zip, sha256
# a51913942d7c8f10427e5cf5e8ce001fc4514f744a98265457e5245a8ecbbbba).
# Same name list as 0.7: no NAMES_REMOVED / NAMES_ADDED.
set -euo pipefail
S=$1; ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
KERNEL_FEATURES=crash-walk \
  "$ROOT/tools/release/package-release.sh" 0.7.1 "$S/rel071" "$S/prev07/NewTOS-0.7.zip"
