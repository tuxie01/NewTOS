#!/usr/bin/env bash
# mkwt.sh <dir> <rev> : scratch git worktree + symlinks of gitignored inputs
set -e
WS=/Volumes/Daten/Development/PiRusTOS; SP=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/651de586-7dd7-487b-9e92-c8525c58f508/scratchpad
cd $WS; git worktree add --detach "$1" "$2" >/dev/null
while read p; do p="${p%/}"; mkdir -p "$1/$(dirname $p)"; [ -e "$1/$p" ] || ln -s "$WS/$p" "$1/$p"; done < $SP/ign.txt
