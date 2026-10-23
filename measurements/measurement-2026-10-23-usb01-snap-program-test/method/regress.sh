#!/bin/bash
# regress.sh <head|base> -- real NEWDESK on qemu-virt dev kernels, default devices: mouse 20 x rel(+10,+6) and Ctrl+Z/typing "help".
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/651de586-7dd7-487b-9e92-c8525c58f508/scratchpad/usbsnap-t; R=/Volumes/Daten/Development/PiRusTOS/measurements/measurement-2026-10-23-usb01-snap-program-test; M=$R/method/regress; V=$1
for T in default; do
  $M/run-virt.sh $S/bin/virt-dev-$V.img $R/serial/regress-$V-mouse-$T.log 60 $T $M/qmp_mouse.py $S/shots $V-mouse-$T 2>&1 | /usr/bin/grep -v "^screendump"
  python3 -I $M/pixdiff.py $S/shots/$V-mouse-$T-0-before.png $S/shots/$V-mouse-$T-1-after.png | head -1 | sed "s/^/$V $T mouse: /"
done
$M/run-virt.sh $S/bin/virt-dev-$V.img $R/serial/regress-$V-key.log 60 default $M/qmp_key.py $S/shots $V-key - 2>&1 | /usr/bin/grep -v "^screendump"
python3 -I $M/pixdiff.py $S/shots/$V-key-0-before.png $S/shots/$V-key-1-after.png | head -1 | sed "s/^/$V Ctrl+Z: /"
python3 -I $M/pixdiff.py $S/shots/$V-key-1-after.png $S/shots/$V-key-2-typed.png | head -1 | sed "s/^/$V typed help: /"
# tablet-only topology: absolute events (qmp_tablet.py); the corrected tablet row (the first version of this script used qmp_mouse.py there = no valid test)
$M/run-virt.sh $S/bin/virt-dev-$V.img $R/serial/regress-$V-tablet.log 60 tablet $M/qmp_tablet.py $S/shots $V-tablet 2>&1 | /usr/bin/grep -v "^screendump"
python3 -I $M/pixdiff.py $S/shots/$V-tablet-0-before.png $S/shots/$V-tablet-1-after.png | head -1 | sed "s/^/$V tablet abs moves: /"
