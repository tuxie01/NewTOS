#!/usr/bin/env bash
# The card of the run with --sd (TOOL10): a plain 64 MiB FAT16 card, NEWDESK.INF
# without autorun, so `cargo xtask run --board rpi4 --sd <clone>` boots to the
# desktop with one drive. Usage: mkcard-call.sh <card.img>
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
bash "$ROOT/tools/mkcard.sh" "$1" "$HERE/NEWDESK.INF"
