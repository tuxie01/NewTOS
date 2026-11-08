#!/bin/sh
# V56: which pass folds the empty marker functions? The base tree's rpi4 release kernel once more with LLVM's
# MergeFunctions pass switched off (-Z merge-functions=disabled; the other flag is .cargo/config.toml's), in its own
# target dir; then llvm-nm of the three markers. Evidence only, the kernel is not used for anything else.
S=${S:?}
B=$(rustc --print sysroot)/lib/rustlib/aarch64-apple-darwin/bin
cd $S/t/base
CARGO_TARGET_DIR=$S/t/base/target-nomerge \
CARGO_TARGET_AARCH64_UNKNOWN_NONE_SOFTFLOAT_RUSTFLAGS="-C force-frame-pointers=yes -Z merge-functions=disabled" \
  cargo xtask build --board rpi4 --release > $S/builds/nomerge-rpi4.log 2>&1; echo "nomerge build rc=$?"
E=$(ls $S/t/base/target-nomerge/aarch64-unknown-none-softfloat/release/kernel)
$B/llvm-nm -C $E | grep -E "vdi::mouse::(just_rts|mov_cur|aes_wheel_token|vex_wheelv)" > $S/ev/llvm-nm-base-nomerge.txt
cat $S/ev/llvm-nm-base-nomerge.txt
rm -rf $S/t/base/target-nomerge
