#!/usr/bin/env bash
# build-kernel.sh <repo> <scratch-tree> <board> [<arm-trigger=script> ...] -- an
# instrumented release kernel built in a SCRATCH copy of the working tree
# (tracked + untracked-not-ignored files, references/ linked): the files the
# tools patch are restored from <repo> first, then method/trace-patch.py adds the
# [a128] serial trace and (rpi4 only) tools/inject-arm.py arms the IKBD
# injector. <repo> itself is never touched.
set -euo pipefail
REPO=$1; T=$2; BOARD=$3; shift 3
HERE=$(cd "$(dirname "$0")" && pwd)
if [ ! -d "$T" ]; then
  mkdir -p "$T"
  (cd "$REPO" && git ls-files -co --exclude-standard | awk '!/^references\//') > "$T.files"
  rsync -a --files-from="$T.files" "$REPO/" "$T/"
  ln -s "$REPO/references" "$T/references"
  mkdir -p "$T/target"; cp -R "$REPO/target/programs" "$T/target/" 2>/dev/null || true
fi
for f in kernel/src/host_env.rs kernel/src/launch.rs kernel/src/shell_loop.rs tos-api/src/aes/input.rs tos-api/src/aes/fsel.rs tos-api/src/aes/mod.rs; do
  cp "$REPO/$f" "$T/$f"; done
python3 -I "$HERE/trace-patch.py" "$T"
if [ "$BOARD" = rpi4 ] && [ $# -gt 0 ]; then python3 -I "$T/tools/inject-arm.py" "$T" "$@" | tail -1; fi
(cd "$T" && cargo xtask build --board "$BOARD" --release) > "$T/../build-$BOARD.txt" 2>&1 || { tail -30 "$T/../build-$BOARD.txt" >&2; exit 1; }
if [ "$BOARD" = rpi4 ]; then shasum -a 256 "$T/target/aarch64-unknown-none-softfloat/release/kernel8.img"
else
  OBJCOPY=$(ls "$(rustc --print sysroot)"/lib/rustlib/*/bin/rust-objcopy | head -1)
  "$OBJCOPY" -O binary "$T/target/aarch64-unknown-none-softfloat/release/kernel" "$T/target/kernel-virt.img"
  shasum -a 256 "$T/target/kernel-virt.img"
fi
