#!/bin/sh
# frozen tree = git archive 5a8e01b5 into the session scratchpad, references/ linked; release rpi4 kernel
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/ed7bb9a3-0fc2-4b37-91bb-ed70559503a3/scratchpad/t3
mkdir -p $S/head && git archive 5a8e01b5 | tar -x -C $S/head && ln -s "$PWD/references" $S/head/references
(cd $S/head && cargo xtask build --board rpi4 --release) > $S/build-head.log 2>&1
shasum -a 256 $S/head/target/aarch64-unknown-none-softfloat/release/kernel8.img
