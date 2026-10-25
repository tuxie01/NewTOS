#!/usr/bin/env bash
# (A115: copied unchanged from measurements/measurement-2026-10-25-a117/method/build-kernel.sh)
# build-kernel.sh <src> <name> <script> -- (ARM=<trigger>, default desktop) fresh release rpi4 kernel in a scratch tree, injector armed on "desktop"
# (tools/inject-arm.py of the WORKSPACE, applied to the scratch tree only).
#   <src> = a git revision -> `git archive <rev>`;  WORKTREE -> the workspace's tracked + untracked-not-ignored files now.
# Output $S/bin/kernel8-<name>.img. Adapted from measurement-2026-10-21-gh1-fix2/method/build-kernel.sh.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; . "$HERE/env.sh"
SRC="$1"; NAME="$2"; SCR="$3"; export SOURCE_DATE_EPOCH=1792368000
T="$S/trees/$NAME"; rm -rf "$T"; mkdir -p "$T" "$S/bin" "$S/logs"
if [ "$SRC" = WORKTREE ]; then
  (cd "$WS" && git ls-files -z -co --exclude-standard | tar --null -T - -cf -) | tar -x -C "$T"
else
  (cd "$WS" && git archive "$SRC") | tar -x -C "$T"
fi
python3 "$WS/tools/inject-arm.py" "$T" --tag inj "${ARM:-desktop}=$SCR"
(cd "$T" && cargo xtask build --board rpi4 --release) > "$S/logs/build-$NAME.log" 2>&1 || { tail -30 "$S/logs/build-$NAME.log"; exit 1; }
cp "$T/kernel8.img" "$S/bin/kernel8-$NAME.img"; shasum -a 256 "$S/bin/kernel8-$NAME.img"
