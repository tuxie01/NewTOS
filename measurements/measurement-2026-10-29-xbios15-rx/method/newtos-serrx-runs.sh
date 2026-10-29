#!/bin/bash
# newtos-serrx-runs.sh <repo> <scratch> -- the NewTOS runs of this measurement, with the
# kernels and the probe in <scratch> (see RUNINFO.TXT "INPUTS"):
#   virt-kernel.img       qemu-virt, bios-arg-trace, SERRX.NAP embedded as the desktop (temporary audit wiring)
#   rpi4-kernel8.img      rpi4, bios-arg-trace (unmodified 2487bbea)
#   rpi4-diag-kernel8.img rpi4, bios-arg-trace + temporary audit drop marker in serial_rx_byte
# 'aZ1' twice per board, one control without bytes per board, one diag run.
set -u
REPO=$1; S=$2; HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
while read -r board k name bytes; do
    [ -e "$S/$name/SERIAL.txt" ] && { echo "$name: exists, kept"; continue; }
    "$HERE/newtos-serrx.sh" "$REPO" "$S" "$board" "$S/$k" "$S/probe/SERRX.NAP" "$S/$name" "$bytes" > "$S/$name.out" 2>&1
    echo "$name: $(grep -E '^(sent|nothing)' "$S/$name/SERIAL.txt.inject.txt")"
done <<LIST
virt virt-kernel.img nt-virt-1 aZ1
virt virt-kernel.img nt-virt-2 aZ1
virt virt-kernel.img nt-virt-none none
rpi4 rpi4-kernel8.img nt-rpi4-1 aZ1
rpi4 rpi4-kernel8.img nt-rpi4-2 aZ1
rpi4 rpi4-kernel8.img nt-rpi4-none none
rpi4 rpi4-diag-kernel8.img nt-rpi4-diag aZ1
LIST
