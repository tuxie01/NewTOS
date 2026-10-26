#!/bin/sh
# hatari-qed.sh -- start the EmuTOS oracle on an emulated Atari TT with the
# m68k QED, so its file selector can be photographed as the REFERENCE for ours.
#
# Same machine as the dumper reference runs (TT, 640x480x16) and the same ROM
# and Hatari as tools/hatari-oracle.sh, both from tools/oracle/oracle.env (see
# docs/analysis/ORACLE.md), so a screenshot here is comparable to a Pi 4
# screenshot at 640x480. TOS=<image> overrides the ROM, e.g. with
# $ORACLE_RELEASE_ROM for the EmuTOS 1.4 release.
#
# C: is GEMDOS-emulated from ~/.nt-hatari/hd, with QED staged under C:\QED.
# Runs WITH a window: this one is meant to be clicked, not scripted.
#
#   Hatari screenshot: F12 -> Screenshot, or Alt-G. Files land in the cwd.
#   Quit: F12 -> Quit, or Alt-Q.
set -eu

. "$(cd "$(dirname "$0")" && pwd)/oracle/oracle.env"
HATARI="$ORACLE_HATARI"
TOS="${TOS:-$ORACLE_ROM}"
HD="$HOME/.nt-hatari/hd"

[ -x "$HATARI" ] || { echo "Hatari not found: $HATARI" >&2; exit 1; }
[ -f "$TOS" ]    || { echo "TOS image not found: $TOS" >&2; exit 1; }
[ -d "$HD/QED" ] || { echo "QED not staged in $HD/QED" >&2; exit 1; }

exec "$HATARI" \
    --configfile "$ORACLE_HATARI_CFG" \
    --machine tt \
    --tos "$TOS" \
    --harddrive "$HD" \
    --sound off \
    "$@"
