#!/usr/bin/env bash
# (A115: copied from measurements/measurement-2026-10-21-gh1-reg/method/drive.sh; outputs one level up from method/reg/)
# drive.sh <kernel> <card> <tag> <inj-tag> <trigger>=<script> [<trigger>=<script> ...] -- one injector run: screendumps keyed to the marks
# of each script (<script>.marks, via steps.sh; delay DL s, default 0.8) in the order given; serial -> ../serial/<tag>-serial.log,
# dumps -> ../screendumps/<tag>-<mark>.png (OUTDIR overrides). One QEMU at a time.
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
K=$1; C=$2; TAG=$3; ITAG=$4; shift 4
ST=(); for a in "$@"; do while IFS= read -r l; do ST+=("$l"); done < <(bash "$HERE/steps.sh" "${a#*=}" "$ITAG" "${a%%=*}" "${DL:-0.8}"); done
bash "$HERE/run.sh" "$K" "$C" "$TAG" "${ST[@]}"
