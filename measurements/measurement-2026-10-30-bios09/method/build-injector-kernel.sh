#!/bin/sh
# build-injector-kernel.sh <src> <scratch-tree> -- BIOS09: an rpi4 release
# kernel8.img with the scratch IKBD injector armed on the probe AWPROBE
# (tools/inject-arm.py, launch:AWPROBE, keys from awprobe-keys.txt beside this
# script). <src> = the tree to measure: this repository (after) or
# `git archive f065a705` extracted into the scratch (before). The copy gets its
# own target/; the shared tree is never patched.
set -eu
SRC=$1; T=$2
HERE=$(cd "$(dirname "$0")" && pwd)
mkdir -p "$T/sdk"
cd "$SRC"
rsync -a --exclude __pycache__ .cargo Cargo.toml Cargo.lock rust-toolchain.toml \
    kernel tos-api tos-types tools programs keyboards boot "$T/"
cp -R sdk/crt0.S sdk/newtos.ld sdk/include "$T/sdk/"
python3 tools/inject-arm.py "$T" "launch:AWPROBE=$HERE/awprobe-keys.txt"
cd "$T"
cargo xtask build --board rpi4 --release
