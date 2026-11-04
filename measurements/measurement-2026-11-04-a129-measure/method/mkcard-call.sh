#!/usr/bin/env bash
# mkcard-call.sh <scratch> -- the run's card: tools/mkcard.sh with FSPROBE.NAP,
# C:\LIST\ holding 30 one-line files F01.TXT .. F30.TXT (> 9 entries, so the
# list scrolls) and NEWDESK.INF autorunning the probe (#Z).
set -euo pipefail
S=$1; ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
mkdir -p "$S/list"; pairs=()
for i in $(seq -w 1 30); do echo "file $i" > "$S/list/F$i.TXT"; pairs+=("LIST/F$i.TXT=$S/list/F$i.TXT"); done
printf '#R 02\r\n#Z 01 C:\\FSPROBE.NAP@\r\n' > "$S/newdesk.inf"
"$ROOT/tools/mkcard.sh" "$S/card.img" "$S/newdesk.inf" FSPROBE.NAP="$S/FSPROBE.NAP" "${pairs[@]}"
shasum -a 256 "$S/card.img"
