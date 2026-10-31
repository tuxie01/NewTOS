#!/usr/bin/env bash
# repro.sh <outdir> <SOURCE_DATE_EPOCH> -- XBIOS12 reproducibility: per board, touch
# tos-api/build.rs, tos-api/src/gemdos/time.rs and kernel/src/lib.rs (mtime only), so
# cargo really recompiles tos-api (build script reruns) and kernel; rebuild with the
# same SOURCE_DATE_EPOCH, objcopy to <outdir>/kernel-<board>-2.img, sha256. Compare
# with the image boot.sh wrote for the same epoch (cmp).
set -uo pipefail
OUT=$1; export SOURCE_DATE_EPOCH=$2
REPO=$(cd "$(dirname "$0")/../../.." && pwd)
OC=$(rustc --print sysroot)/lib/rustlib/aarch64-apple-darwin/bin/llvm-objcopy
mkdir -p "$OUT"
for B in rpi4 qemu-virt; do
  touch "$REPO/tos-api/build.rs" "$REPO/tos-api/src/gemdos/time.rs" "$REPO/kernel/src/lib.rs"
  (cd "$REPO" && cargo xtask build --board "$B") > "$OUT/rebuild-$B.log" 2>&1 || { echo "BUILD FAIL $B"; exit 1; }
  echo "$B: $(grep -c 'Compiling tos-api\|Compiling kernel' "$OUT/rebuild-$B.log") crates recompiled"
  "$OC" -O binary "$REPO/target/aarch64-unknown-none-softfloat/debug/kernel" "$OUT/kernel-$B-2.img"
  shasum -a 256 "$OUT/kernel-$B-2.img"
  cmp "$OUT/kernel-$B.img" "$OUT/kernel-$B-2.img" && echo "$B: identical"
done
