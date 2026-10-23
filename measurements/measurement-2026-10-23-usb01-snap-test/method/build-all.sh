#!/bin/bash
# build-all.sh <wt> <tag> : fresh builds per board in a scratch worktree with its own target dir
set -u
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/651de586-7dd7-487b-9e92-c8525c58f508/scratchpad/usb01-snt
WT=$1; TAG=$2; EXTRA=${3:-}
export SOURCE_DATE_EPOCH=1792152000
cd $WT
OBJCOPY=$(find "$(rustc --print sysroot)" -name rust-objcopy | head -1)
b() { # name, profile-dir, args...
  n=$1; d=$2; shift 2
  rm -f target/aarch64-unknown-none-softfloat/$d/kernel
  cargo xtask build "$@" > $S/logs/$TAG-$n.log 2>&1; echo "$TAG $n rc=$?"
}
b virt-dev debug --board qemu-virt
"$OBJCOPY" -O binary target/aarch64-unknown-none-softfloat/debug/kernel $S/bin/virt-dev-$TAG.img
b virt-rel release --board qemu-virt --release
b rpi4-rel release --board rpi4 --release
cp kernel8.img $S/bin/kernel8-$TAG-rpi4.img
if [ -n "$EXTRA" ]; then
b rpi4-diag release --board rpi4 --release --features usb-diag
cp kernel8.img $S/bin/kernel8-$TAG-rpi4-usbdiag.img
fi
shasum -a 256 $S/bin/*$TAG*
