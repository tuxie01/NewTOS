#!/bin/sh
# fresh release rpi4 kernel from the git archive of 4a8e1ae3 in scratch (references/ linked)
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/15767204-8d89-43e0-aae0-d8149a16a17d/scratchpad/v18t2
(cd $S/head && cargo xtask build --board rpi4 --release) > $S/build-head.log 2>&1
tail -1 $S/build-head.log
shasum -a 256 $S/head/target/aarch64-unknown-none-softfloat/release/kernel8.img
