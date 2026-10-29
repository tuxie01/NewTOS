#!/bin/bash
# newtos-serrx.sh <repo> <scratch> <rpi4|virt> <kernel image> <SERRX.NAP> <out-dir> [<bytes>]
# XBIOS15: the serrx probe on NewTOS (2487bbea, kernels built in a frozen `git archive`
# tree with --features bios-arg-trace), serial0 driven by method/qemu-serial-drive.py:
# the bytes (default 'aZ1', 'none' = control) go into the PL011 receive line 0.3 s after
# the probe printed "SERRX:WAIT".
#   rpi4: QEMU raspi4b, -append serial=on, card from tools/mkcard.sh with C:\SERRX.NAP and
#         NEWDESK.INF "#R 02 / #Z 01 C:\SERRX.NAP@" (booted as a per-run file, deleted).
#   virt: QEMU virt (the `cargo xtask run` machine: gic-version=2, cortex-a72, 272M, ramfb,
#         qemu-xhci + usb-kbd + usb-mouse), flat kernel image; no drive -- the probe is the
#         embedded desktop program (temporary audit wiring in the frozen tree's kernel/embed.rs, see
#         RUNINFO.TXT), its log is read from the serial line ("SERRX|" lines).
set -u
REPO=$1; S=$2; BOARD=$3; KIMG=$4; NAP=$5; D=$6; BYTES=${7:-aZ1}
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
mkdir -p "$D"; D=$(cd "$D" && pwd)
shasum -a 256 "$KIMG" "$NAP" > "$D/inputs.sha256"
case $BOARD in
rpi4)
    IMG="$S/card-$(basename "$D").img"; rm -f "$IMG" "$IMG.manifest"
    printf '#R 02\r\n#Z 01 C:\\SERRX.NAP@\r\n' > "$S/z-serrx.inf"
    cp "$S/z-serrx.inf" "$D/NEWDESK.INF"
    (cd "$REPO" && tools/mkcard.sh "$IMG" "$S/z-serrx.inf" "SERRX.NAP=$NAP" >/dev/null 2>&1) || { echo "mkcard failed"; exit 1; }
    cp "$IMG.manifest" "$D/card.img.manifest"; shasum -a 256 "$IMG" > "$D/card.img.before.sha256"
    python3 "$HERE/qemu-serial-drive.py" "$D/SERIAL.txt" "SERRX:WAIT" "$BYTES" \
        "AUDIT:xbios15-serial-rx:END" 300 -- \
        qemu-system-aarch64 -M raspi4b -no-reboot -display none @SERIAL@ \
        -append serial=on -kernel "$KIMG" -drive file="$IMG",if=sd,format=raw
    export MTOOLS_SKIP_CHECK=1
    mcopy -o -i "$IMG@@32256" ::/SERRX.LOG "$D/SERRX.LOG" 2>/dev/null || echo "no SERRX.LOG on the card" > "$D/SERRX.LOG.missing"
    shasum -a 256 "$IMG" > "$D/card.img.after.sha256"; rm -f "$IMG" "$IMG.manifest"
    ;;
virt)
    python3 "$HERE/qemu-serial-drive.py" "$D/SERIAL.txt" "SERRX:WAIT" "$BYTES" \
        "AUDIT:xbios15-serial-rx:END" 300 -- \
        qemu-system-aarch64 -M virt,gic-version=2 -cpu cortex-a72 -m 272M -device ramfb \
        -device qemu-xhci -device usb-kbd -device usb-mouse \
        -display none @SERIAL@ -no-reboot -kernel "$KIMG"
    ;;
*) echo "board: rpi4 or virt"; exit 2 ;;
esac
# the probe's log as printed on the serial line (first run of the probe only)
awk '/AUDIT:xbios15-serial-rx:BEGIN/{n++} n==1' "$D/SERIAL.txt" | tr -d '\r' | sed -n 's/^SERRX|//p' > "$D/SERRX-serial.LOG"
cat "$D/SERIAL.txt.inject.txt"
