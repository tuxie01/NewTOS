#!/bin/zsh
# Which profile (and split-debuginfo) does each command give the workspace units?
cd /Volumes/Daten/Development/PiRusTOS
m=${0:h}
for cmd in "build --package kernel --no-default-features --features board-qemu-virt" \
           "build --package kernel --no-default-features --features board-rpi4 --release" \
           "test --package tos-api --package tos-types --package xtask"; do
  echo "== cargo $cmd"
  cargo ${=cmd} --unit-graph -Z unstable-options 2>/dev/null | python3 -I $m/unit-graph-profiles.py
done
