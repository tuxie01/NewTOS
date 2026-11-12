#!/bin/bash
# qemu-h5.sh <out-dir> -- the H5 run of TASK/SND03 in QEMU raspi4b, as run on 2026-11-12.
# Builds the rpi4 release kernel with the probe, boots it headless without an SD card for 60 s,
# serial line to <out-dir>/qemu-h5.log. The probe's boot arguments:
#   sndprobe=qemu    leave out PWM1 and PACTL_CS (QEMU 11.1.2 does not model them; see RUNINFO.TXT)
#   sndprobe=nowait  no key wait between the probe's three screen pages (QEMU raspi4b has no keyboard)
# No `timeout` on macOS: the run is bounded by a background kill.
set -eu
O=$1
mkdir -p "$O"
cargo xtask build --board rpi4 --release --features snd-probe
cp target/aarch64-unknown-none-softfloat/release/kernel8.img "$O/kernel8-snd-probe.img"
( qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial "file:$O/qemu-h5.log" \
    -kernel "$O/kernel8-snd-probe.img" -append "serial=on sndprobe=qemu sndprobe=nowait" &
  pid=$!; sleep 60; kill $pid 2>/dev/null )
shasum -a 256 "$O/kernel8-snd-probe.img"
grep '\[snd03\]' "$O/qemu-h5.log"
