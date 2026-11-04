#!/usr/bin/env bash
# build-inj-kernel.sh <repo> <scratch-tree> <arm-trigger=script> ... -- the
# release kernel (rpi4 --release --features crash-walk) PLUS the scripted IKBD
# injector (tools/inject-arm.py), built in a SCRATCH copy of the working tree
# (tracked + untracked-not-ignored files, references/ linked). <repo> is never
# touched. Copied from measurement-2026-11-04-rel07-version/method/build-kernel.sh;
# only the feature flag is new. The copy has no .git, so the build stamp's
# revision reads "(unknown)".
set -euo pipefail
REPO=$1; T=$2; shift 2
if [ ! -d "$T" ]; then
  mkdir -p "$T"
  (cd "$REPO" && git ls-files -co --exclude-standard | awk '!/^references\//') > "$T.files"
  rsync -a --files-from="$T.files" "$REPO/" "$T/"
  ln -s "$REPO/references" "$T/references"
fi
for f in kernel/src/host_env.rs kernel/src/launch.rs kernel/src/shell_loop.rs; do cp "$REPO/$f" "$T/$f"; done
python3 -I "$T/tools/inject-arm.py" "$T" "$@" | tail -1
(cd "$T" && cargo xtask build --board rpi4 --release --features crash-walk) > "$T/../build-inj.txt" 2>&1 || { tail -30 "$T/../build-inj.txt" >&2; exit 1; }
shasum -a 256 "$T/target/aarch64-unknown-none-softfloat/release/kernel8.img"
