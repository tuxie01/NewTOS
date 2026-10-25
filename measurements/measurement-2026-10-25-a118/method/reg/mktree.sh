#!/usr/bin/env bash
# (A118: copied from measurements/measurement-2026-10-21-gh1-reg/method/mktree.sh; outputs one level up from method/reg/)
# mktree.sh <head|wt> <name> -- scratch source tree: head = git archive 17cab8b6 ; wt = working tree files (tracked+untracked-not-ignored, minus measurements/ TASK/ docs/ references)
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; . "$HERE/env.sh"
T="$S/trees/$2"; mkdir -p "$T"; find "$T" -mindepth 1 -maxdepth 1 ! -name target -exec rm -rf {} +
if [ "$1" = wt ]; then (cd "$WS" && git ls-files -z -co --exclude-standard | tar --null -T - -cf -) | tar -x -C "$T"
else (cd "$WS" && git archive $HEADREV) | tar -x -C "$T"; fi
echo "tree $T"
