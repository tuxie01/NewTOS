#!/bin/sh
# fresh release rpi4 kernels, head and base, from git archives in scratch
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/15767204-8d89-43e0-aae0-d8149a16a17d/scratchpad/v24t
for t in head base; do
  (cd $S/$t && cargo xtask build --board rpi4 --release) > $S/build-$t.log 2>&1
  tail -1 $S/build-$t.log
  shasum -a 256 $S/$t/target/aarch64-unknown-none-softfloat/release/kernel8.img
done
