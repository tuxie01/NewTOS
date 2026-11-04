#!/usr/bin/env bash
# mkcard-call.sh <scratch> -- the run's card: tools/mkcard.sh with a
# NEWDESK.INF that only sets the resolution (#R 02, no autorun): the plain
# desktop, whose Desk menu opens the About box.
set -euo pipefail
S=$1; ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
mkdir -p "$S"
printf '#R 02\r\n' > "$S/newdesk.inf"
"$ROOT/tools/mkcard.sh" "$S/card.img" "$S/newdesk.inf"
shasum -a 256 "$S/card.img"
