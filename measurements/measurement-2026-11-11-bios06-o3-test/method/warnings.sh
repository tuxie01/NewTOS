#!/bin/sh
# warnings.sh <tree> <target-dir> <tag> <out-dir> -- warning counts of `cargo build` (release) and `cargo clippy`
# of the kernel on both boards, the invocation of tools/xtask build_kernel (--package kernel --no-default-features
# --features board-<b>); the embedded programs come from <tree>/target/programs.
T=$1; TD=$2; TAG=$3; O=$4; cd "$T" || exit 1
for b in rpi4 qemu-virt; do
  for k in build clippy; do
    CARGO_TARGET_DIR=$TD cargo $k --package kernel --no-default-features --features board-$b --release > "$O/$TAG-$k-$b.log" 2>&1
    echo "$TAG $k $b rc=$? : $(grep -E '^warning: .*generated' "$O/$TAG-$k-$b.log" | tr '\n' ';')"
  done
done
