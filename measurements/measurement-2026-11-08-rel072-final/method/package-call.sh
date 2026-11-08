#!/usr/bin/env bash
# package-call.sh <scratch> -- the call that built the published NewTOS-0.7.2.zip (TASK/REL072).
# Same call as measurements/measurement-2026-11-08-rel072-package/method/package-call.sh, fresh work dir.
set -euo pipefail
S=$1; ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
NAMES_REMOVED="KARTE/APPS/HYPVIEW/CHANGES.TXT KARTE/APPS/COREMARK/CHANGES.TXT" \
KERNEL_FEATURES=crash-walk \
  "$ROOT/tools/release/package-release.sh" 0.7.2 "$S/pk2/rel072" "$S/prev071/NewTOS-0.7.1.zip"
