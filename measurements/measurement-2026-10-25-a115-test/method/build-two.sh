#!/usr/bin/env bash
# build-two.sh -- Tester a115-test: kernels armed with two/dm.script (desktop trigger), base 587e0f68 and new (working tree).
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; . "$HERE/reg/env.sh"
bash "$HERE/a117/build-kernel.sh" 587e0f68 base-dm "$HERE/two/dm.script"; rm -rf "$S/trees/base-dm"
bash "$HERE/a117/build-kernel.sh" WORKTREE new-dm "$HERE/two/dm.script"; rm -rf "$S/trees/new-dm"
