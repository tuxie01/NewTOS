#!/usr/bin/env bash
# qemu-virt program runs (T5/T6): extra topology full run, 15-s start timeout, no-device controller, MUTE probe.
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/651de586-7dd7-487b-9e92-c8525c58f508/scratchpad/usbsnap-t; R=/Volumes/Daten/Development/PiRusTOS/measurements/measurement-2026-10-23-usb01-snap-program-test; M=$R/method
mkdir -p $S/shots
SHOT_DIR=$S/shots TAG=virt-extra $M/run-virt.sh $S/bin/virt-USBSNAPV.img $R/serial/virt-extra.log extra $M/qmp_usbsnap.py run
python3 -I $M/extract_files.py $R/serial/virt-extra.log $R/files virt-extra
SHOT_DIR=$S/shots TAG=virt-wait $M/run-virt.sh $S/bin/virt-USBSNAPV.img $R/serial/virt-wait.log default $M/qmp_usbsnap.py wait
SHOT_DIR=$S/shots TAG=virt-none $M/run-virt.sh $S/bin/virt-USBSNAPV.img $R/serial/virt-none.log none $M/qmp_usbsnap.py nodev
$M/run-virt.sh $S/bin/virt-MUTECHK.img $R/serial/virt-mutechk.log default $M/qmp_mutechk.py
