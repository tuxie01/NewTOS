#!/usr/bin/env bash
# mkcard-call.sh <image> -- this run's card: the shared tools/mkcard.sh (TASK/TOOL02) with this run's content.
# Only the call lives here, not a copy of the tool. QEMU card only, never a real card.
#   C:\NEWDESK.INF      method/NEWDESK.INF (measurement-2026-10-27-mem12-td-lp64/method/z-teradesk.inf without its #Z line, CRLF)
#   C:\AUTO\MXPROBE.NCP sdk/libc/build/MXPROBE.NCP  -- runs from \AUTO\ at boot, prints [MXPROBE] lines: shows the card is read
#   C:\ARGVTEST.NAP     sdk/libc/build/argvtest.NAP, own FAT stamp 2020-01-01 12:00 (exercises the @stamp suffix)
#   empty C:\SUB\, C:\TMP\ (MKCARD_DIRS); label TOOL02; size and default stamp = tool defaults (64 MiB, 202610171200)
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; WS="$(cd "$HERE/../../.." && pwd)"
MKCARD_LABEL=TOOL02 MKCARD_DIRS="SUB TMP" "$WS/tools/mkcard.sh" "${1:?image}" "$HERE/NEWDESK.INF" \
  "AUTO/MXPROBE.NCP=$WS/sdk/libc/build/MXPROBE.NCP" \
  "ARGVTEST.NAP=$WS/sdk/libc/build/argvtest.NAP@202001011200"
