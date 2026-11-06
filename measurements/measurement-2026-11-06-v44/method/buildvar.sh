#!/bin/bash
# buildvar.sh <base|head> <variant> -> $S/kernels/<tree>-<variant>.img
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/7fba4f48-e8f1-4a5e-8c58-696d17828400/scratchpad/v44
T=$1; V=$2
python3 $S/scripts/layers.py $S/$T $V || exit 1
(cd $S/$T && cargo xtask build --board rpi4 --release) > $S/kernels/$T-$V.build.log 2>&1 || { tail -30 $S/kernels/$T-$V.build.log; exit 1; }
cp $S/$T/target/aarch64-unknown-none-softfloat/release/kernel8.img $S/kernels/$T-$V.img
echo "$T-$V $(shasum -a 256 $S/kernels/$T-$V.img | cut -c1-16)"
