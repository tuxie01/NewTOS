#!/usr/bin/env bash
# make-card.sh <scratch> -- the one base card of this run: CWPAR.NAP and
# CWKID.NAP (built from this directory with sdk/build.sh), NEWDESK.INF with
# "#Z 01 C:\CWPAR.NAP@" (the desktop autoruns CWPAR, no key, no injector).
# Card by tools/mkcard.sh (defaults). Writes <scratch>/base.img and its
# manifest; prints the sha256 of the programs and the card.
set -euo pipefail
SCR=$1
REPO=$(cd "$(dirname "$0")/../../.." && pwd)
M=$(cd "$(dirname "$0")" && pwd)
mkdir -p "$SCR"
cd "$REPO"
bash sdk/build.sh "$M/cwpar.c" "$SCR/CWPAR.NAP" > "$SCR/build.txt" 2>&1
bash sdk/build.sh "$M/cwkid.c" "$SCR/CWKID.NAP" >> "$SCR/build.txt" 2>&1
printf '#R 02\r\n#Z 01 C:\\CWPAR.NAP@\r\n' > "$SCR/newdesk.inf"
tools/mkcard.sh "$SCR/base.img" "$SCR/newdesk.inf" CWPAR.NAP="$SCR/CWPAR.NAP" CWKID.NAP="$SCR/CWKID.NAP" \
    > "$SCR/mkcard.stdout" 2>&1 || { cat "$SCR/mkcard.stdout" >&2; exit 1; }
shasum -a 256 "$SCR/CWPAR.NAP" "$SCR/CWKID.NAP" "$SCR/base.img"
