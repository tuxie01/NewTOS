#!/usr/bin/env bash
# mkcard-zv.sh <scratch> <sample name> -- zView card through tools/mkcard.sh: C:\ZVIEW\ (the dist of sdk/zview/build/dist/ZVIEW,
# main tree, gitignored build output) + the repo's sample images (measurement-2026-10-06-zview-m4/bin/SAMPLES, tracked) +
# C:\ZVLAUNCH.NAP (tracked, measurement-2026-10-06-zview-m4/bin) + C:\ZVARG.TXT naming the sample; #Z autorun = ZVLAUNCH.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; M=/Volumes/Daten/Development/PiRusTOS; S="$1"; N="$2"
DIST=$M/sdk/zview/build/dist/ZVIEW; SM=$M/measurements/measurement-2026-10-06-zview-m4/bin
PAIRS=(ZVLAUNCH.NAP=$SM/ZVLAUNCH.NAP ZVARG.TXT=$HERE/ZVARG-$N.TXT)
while IFS= read -r f; do PAIRS+=("ZVIEW/${f#$DIST/}=$f"); done < <(find "$DIST" -type f | sort)
while IFS= read -r f; do PAIRS+=("ZVIEW/SAMPLES/$(basename "$f")=$f"); done < <(find "$SM/SAMPLES" -type f | sort)
(cd "${W:?}/wt" && tools/mkcard.sh "$S/cards/zv-$N.img" "$HERE/z-zview.inf" "${PAIRS[@]}") | tail -3
