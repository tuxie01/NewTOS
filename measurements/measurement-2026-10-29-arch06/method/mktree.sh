#!/usr/bin/env bash
# mktree.sh <scratch-tree> -- scratch source tree = the working tree's files (tracked + untracked-not-ignored),
# so scratch-only patches (selftest wiring, injector arm) never touch the shared tree. references/ symlinked.
set -euo pipefail
WS="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"; T="$1"
mkdir -p "$T"; find "$T" -mindepth 1 -maxdepth 1 ! -name target -exec rm -rf {} +
(cd "$WS" && git ls-files -z -co --exclude-standard | tar --null -T - -cf -) | tar -x -C "$T"
ln -s "$WS/references" "$T/references"
echo "tree $T"
