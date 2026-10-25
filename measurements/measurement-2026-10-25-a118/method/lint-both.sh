#!/usr/bin/env bash
# lint-both.sh -- A118: lint.sh (builds both boards dev+release, clippy, cargo xtask test) in two scratch trees, one after the
# other: base = git archive 17cab8b6 (reg/mktree.sh head), new = working tree files (reg/mktree.sh wt). references/ and
# sdk/apps/teradesk/cicons.rsc are gitignored but needed by the rsc tests: symlinked into both trees.
# Summaries -> ../checks/lint-<side>-summary.txt; rpi4 release kernels -> $S/bin/kernel8-<side>.img (plain, for APITEST).
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; . "$HERE/reg/env.sh"; export SOURCE_DATE_EPOCH=1792368000
mkdir -p "$S/bin" "$HERE/../checks" "$HERE/../hosttest"
for side in base new; do
  case $side in base) bash "$HERE/reg/mktree.sh" head lint-base;; new) bash "$HERE/reg/mktree.sh" wt lint-new;; esac
  T="$S/trees/lint-$side"
  mkdir -p "$T/references"; ln -sfn "$WS/references/emutos" "$T/references/emutos"
  mkdir -p "$T/sdk/apps/teradesk"; ln -sf "$WS/sdk/apps/teradesk/cicons.rsc" "$T/sdk/apps/teradesk/cicons.rsc"
  bash "$HERE/lint.sh" "$T" "$S/lint-$side"
  cp "$S/lint-$side/summary.txt" "$HERE/../checks/lint-$side-summary.txt"
  cp "$S/lint-$side/kernel8-rpi4-release.img" "$S/bin/kernel8-$side.img" 2>/dev/null || true
  # The A118 host test, unchanged, in both trees (base: copied in; red there = the effect before the fix).
  cp "$WS/tos-api/tests/aes_a118_lock_takers.rs" "$T/tos-api/tests/"
  (cd "$T" && cargo test -p tos-api --test aes_a118_lock_takers) > "$HERE/../hosttest/aes_a118_lock_takers-$side.log" 2>&1
  echo "hosttest $side: $(grep -E '^test result' "$HERE/../hosttest/aes_a118_lock_takers-$side.log")" >> "$HERE/../checks/lint-$side-summary.txt"
done
echo LINT-BOTH DONE
