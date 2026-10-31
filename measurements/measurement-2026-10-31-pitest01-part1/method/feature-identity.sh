#!/usr/bin/env bash
# feature-identity.sh <scratch> -- the kernel built from the tree with the
# PITEST01 change, feature crash-nowait OFF, against the kernel built from HEAD
# (CLAUDE.md "Testing", debug features). The method of
# measurements/measurement-2026-10-31-fs26/method/feature-identity.sh with the
# feature name changed: two scratch copies of equal path length, no .git in
# either (build stamp rev "unknown" on both sides), the same
# SOURCE_DATE_EPOCH: base = `git archive HEAD`, aftr = `git ls-files -co
# --exclude-standard` of the working tree. Four builds each (rpi4/qemu-virt x
# release/dev), raw binaries via llvm-objcopy, then
# measurements/measurement-2026-10-28-tool08/method/locdiff.py on each pair
# (called, not copied): every differing word must be the line field of a
# core::panic::Location record. Plus the feature-on build for both boards
# (release and dev), and the clippy warning counts of both sides.
set -uo pipefail
SCR=$1
FEATURE=crash-nowait
REPO=$(cd "$(dirname "$0")/../../.." && pwd)
OC=$(rustc --print sysroot)/lib/rustlib/aarch64-apple-darwin/bin/llvm-objcopy
export SOURCE_DATE_EPOCH=1793404800
rm -rf "$SCR/base" "$SCR/aftr"; mkdir -p "$SCR/base" "$SCR/aftr" "$SCR/bins"
(cd "$REPO" && git archive HEAD) | tar -x -C "$SCR/base" --exclude measurements
(cd "$REPO" && git ls-files -co --exclude-standard -z | tr '\0' '\n' | grep -v '^measurements/' | tr '\n' '\0' |
    rsync -a --from0 --files-from=- ./ "$SCR/aftr/")
for side in base aftr; do
  for b in rpi4 qemu-virt; do
    for p in release dev; do
      R=(); [ $p = release ] && R=(--release)
      (cd "$SCR/$side" && cargo xtask build --board $b ${R[@]+"${R[@]}"}) > "$SCR/bins/build-$side-$b-$p.log" 2>&1 ||
          { echo "BUILD FAIL $side $b $p"; tail -20 "$SCR/bins/build-$side-$b-$p.log"; exit 1; }
      d=$([ $p = release ] && echo release || echo debug)
      "$OC" -O binary "$SCR/$side/target/aarch64-unknown-none-softfloat/$d/kernel" "$SCR/bins/$side-$b-$p.bin"
    done
    (cd "$SCR/$side" && cargo clippy -p kernel --release --features board-$b) > "$SCR/bins/clippy-$side-$b.log" 2>&1
    echo "clippy $side $b: exit $? $(grep -E '^warning: `(kernel|tos-api)` \(lib\) generated' "$SCR/bins/clippy-$side-$b.log" | tr '\n' ' ')"
  done
done
for b in rpi4 qemu-virt; do
  for p in release dev; do
    R=(); [ $p = release ] && R=(--release)
    (cd "$SCR/aftr" && cargo xtask build --board $b ${R[@]+"${R[@]}"} --features $FEATURE) > "$SCR/bins/build-on-$b-$p.log" 2>&1 &&
        echo "feature on, $b $p: build OK" || echo "feature on, $b $p: BUILD FAIL"
  done
  (cd "$SCR/aftr" && cargo clippy -p kernel --release --features board-$b,$FEATURE) > "$SCR/bins/clippy-on-$b.log" 2>&1
  echo "clippy on $b: exit $? $(grep -E '^warning: `(kernel|tos-api)` \(lib\) generated' "$SCR/bins/clippy-on-$b.log" | tr '\n' ' ')"
done
(cd "$SCR/bins" && shasum -a 256 *.bin)
for b in rpi4 qemu-virt; do
  for p in release dev; do
    echo "== locdiff $b-$p"
    python3 "$REPO/measurements/measurement-2026-10-28-tool08/method/locdiff.py" "$SCR/bins/base-$b-$p.bin" "$SCR/bins/aftr-$b-$p.bin" > "$SCR/bins/locdiff-$b-$p.txt"
    echo "exit $?"; tail -3 "$SCR/bins/locdiff-$b-$p.txt"
  done
done
