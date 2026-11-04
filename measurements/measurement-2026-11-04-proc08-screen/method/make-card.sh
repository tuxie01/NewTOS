#!/usr/bin/env bash
# make-card.sh <scratch> -- the one base card of this run: CWONE.NAP (this
# directory) and CWKID.NAP (built from
# measurements/measurement-2026-11-02-proc08-diag/method/cwkid.c, called, not
# copied) with sdk/build.sh; C:\CWCASE.TXT = "j" (each run overwrites it in
# its own clone, run-shot.sh); NEWDESK.INF "#Z 01 C:\CWONE.NAP@" (the desktop
# autoruns CWONE, no key, no injector). Card by tools/mkcard.sh (defaults).
# Writes <scratch>/base.img and its manifest; prints the sha256 of the
# programs and the card.
set -euo pipefail
SCR=$1
REPO=$(cd "$(dirname "$0")/../../.." && pwd)
M=$(cd "$(dirname "$0")" && pwd)
mkdir -p "$SCR"
cd "$REPO"
bash sdk/build.sh "$M/cwone.c" "$SCR/CWONE.NAP" > "$SCR/build.txt" 2>&1
bash sdk/build.sh "$REPO/measurements/measurement-2026-11-02-proc08-diag/method/cwkid.c" "$SCR/CWKID.NAP" >> "$SCR/build.txt" 2>&1
printf '#R 02\r\n#Z 01 C:\\CWONE.NAP@\r\n' > "$SCR/newdesk.inf"
printf 'j' > "$SCR/cwcase.txt"
tools/mkcard.sh "$SCR/base.img" "$SCR/newdesk.inf" CWONE.NAP="$SCR/CWONE.NAP" CWKID.NAP="$SCR/CWKID.NAP" \
    CWCASE.TXT="$SCR/cwcase.txt" > "$SCR/mkcard.stdout" 2>&1 || { cat "$SCR/mkcard.stdout" >&2; exit 1; }
shasum -a 256 "$SCR/CWONE.NAP" "$SCR/CWKID.NAP" "$SCR/base.img"
