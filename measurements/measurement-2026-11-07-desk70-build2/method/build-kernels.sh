#!/bin/bash
# build-kernels.sh <scratch> -- the two release rpi4 kernels of this run, both from a scratch `git clone` of HEAD at <scratch>/wt
# (never the repository tree): kernel8-plain.img unchanged, kernel8-zkey.img with the scripted IKBD injector of
# measurements/measurement-2026-11-07-desk70/method/ (mk-injector.py via build-inj.sh, called from there unchanged; host_env.rs
# restored from <scratch>/host_env.rs.pristine and cmp-checked after the build), script z-key.actions of this dir.
# Copied from measurements/measurement-2026-11-07-desk70-build/method/build-kernels.sh; only the action file and tag differ.
set -eu
S=$1; M=$(cd "$(dirname "$0")" && pwd); D=$M/../../measurement-2026-11-07-desk70/method; export SOURCE_DATE_EPOCH=1792152000
mkdir -p "$S/kern"; cp "$S/wt/kernel/src/host_env.rs" "$S/host_env.rs.pristine"
(cd "$S/wt" && cargo xtask build --board rpi4 --release > "$S/build-plain.log" 2>&1)
cp "$S/wt/target/aarch64-unknown-none-softfloat/release/kernel8.img" "$S/kern/kernel8-plain.img"
"$D/build-inj.sh" "$S" "$M/z-key.actions" zkey
shasum -a 256 "$S/kern/kernel8-plain.img"
