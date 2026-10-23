#!/usr/bin/env bash
# run-all.sh -- the program half of this run, in order (USB01 K10/K11, docs/api/usb/usb-snapshot.md §6 T5-T7).
# Needs: the kernels of the shared tree in $S/bin (virt-dev.img = cargo xtask build --board qemu-virt, objcopy'd;
# rpi4-rel.img = kernel8.img of cargo xtask build --board rpi4 --release; fresh build per board), and a scratch git
# worktree $S/wt of HEAD with the changed files copied in and `cargo xtask build-programs --board qemu-virt` done
# (NEWDESK.NAP saved as $S/NEWDESK.NAP.orig). Nothing in the shared tree's target/ is touched.
set -uo pipefail
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/651de586-7dd7-487b-9e92-c8525c58f508/scratchpad/usbsnap
WS=/Volumes/Daten/Development/PiRusTOS; R=$WS/measurements/measurement-2026-10-23-usb01-snap-program; M=$R/method
OBJCOPY=$(find "$(rustc --print sysroot)" -name rust-objcopy | head -1)
mkdir -p $S/prog $S/shots $S/cards
cd $WS
# 1. programs (product build exactly as tools/make-sd-image.sh does it; measurement builds via the wrappers)
sdk/build.sh sdk/examples/usbsnap.c $S/prog/USBSNAP.NCP > /dev/null
sdk/build.sh $M/usbsnap-virt.c $S/prog/USBSNAPV.NAP > /dev/null
sdk/build.sh $M/mutechk.c $S/prog/MUTECHK.NAP > /dev/null
python3 -I $M/mkreplay.py $R/serial/virt-capture.log $M/replay.h
sdk/build.sh $M/usbsnap-replay.c $S/prog/USBSNAPR.NCP > /dev/null
# 2. scratch kernels: the measurement program as NEWDESK.NAP (the program the qemu-virt shell starts)
for p in USBSNAPV MUTECHK; do
  ( cd $S/wt && cp $S/prog/$p.NAP target/programs/qemu-virt/NEWDESK.NAP \
    && cargo build --package kernel --no-default-features --features board-qemu-virt > $S/logs/wt-$p.log 2>&1 \
    && $OBJCOPY -O binary target/aarch64-unknown-none-softfloat/debug/kernel $S/bin/virt-$p.img; \
    cp $S/NEWDESK.NAP.orig target/programs/qemu-virt/NEWDESK.NAP )
done
# 3. qemu-virt: T5/T6 program run (--usb-extra topology), start timeout, no devices; MUTE/kill probe
SHOT_DIR=$S/shots TAG=virt-extra $M/run-virt.sh $S/bin/virt-USBSNAPV.img $R/serial/virt-extra.log extra $M/qmp_usbsnap.py run
python3 -I $M/extract_files.py $R/serial/virt-extra.log $R/files virt-extra
SHOT_DIR=$S/shots TAG=virt-wait $M/run-virt.sh $S/bin/virt-USBSNAPV.img $R/serial/virt-wait.log default $M/qmp_usbsnap.py wait
SHOT_DIR=$S/shots TAG=virt-none $M/run-virt.sh $S/bin/virt-USBSNAPV.img $R/serial/virt-none.log none $M/qmp_usbsnap.py nodev
$M/run-virt.sh $S/bin/virt-MUTECHK.img $R/serial/virt-mutechk.log default $M/qmp_mutechk.py
# 4. raspi4b (release kernel, card): T7 with the product build; the file half with the replay build, two boots on one
#    clone (second run -> .T05..); C:\USB as a file
printf '#Z 01 C:\\USBSNAP.NCP@\r\n#R 02\r\n' > $M/z-usbsnap.inf; printf '#Z 01 C:\\USBSNAPR.NCP@\r\n#R 02\r\n' > $M/z-replay.inf
printf 'not a folder\r\n' > $S/USB.TXTFILE
$M/mkcard.sh $S/cards/t7-base.img $M/z-usbsnap.inf USBSNAP.NCP=$S/prog/USBSNAP.NCP > /dev/null
$M/mkcard.sh $S/cards/replay-base.img $M/z-replay.inf USBSNAPR.NCP=$S/prog/USBSNAPR.NCP > /dev/null
$M/mkcard.sh $S/cards/isfile-base.img $M/z-replay.inf USBSNAPR.NCP=$S/prog/USBSNAPR.NCP USB=$S/USB.TXTFILE > /dev/null
cp -c $S/cards/t7-base.img $S/cards/t7.img
SHOT=$S/shots/rpi4-t7.png $M/run-rpi.sh $S/bin/rpi4-rel.img $S/cards/t7.img $R/serial/rpi4-t7.log "Press any key to exit" 90
rm -f $S/cards/t7.img
cp -c $S/cards/replay-base.img $S/cards/replay.img
SHOT=$S/shots/rpi4-replay1.png $M/run-rpi.sh $S/bin/rpi4-rel.img $S/cards/replay.img $R/serial/rpi4-replay1.log "Press any key to exit" 120
SHOT=$S/shots/rpi4-replay2.png $M/run-rpi.sh $S/bin/rpi4-rel.img $S/cards/replay.img $R/serial/rpi4-replay2.log "Press any key to exit" 120
rm -f $R/files/rpi4-replay-*
for f in TXT T01 T02 T03 T04 T05 T06 T07 T08 T09; do mcopy -n -i "$S/cards/replay.img@@32256" "::/USB/06270001.$f" "$R/files/rpi4-replay-06270001.$f"; done
python3 -I $M/check_files.py $R/files/rpi4-replay-*
for k in 0 1 2 3 4; do a=$(printf 'TXT T01 T02 T03 T04' | cut -d' ' -f$((k+1))); b=T0$((k+5)); cmp -s $R/files/rpi4-replay-06270001.$a $R/files/rpi4-replay-06270001.$b && echo "run 2 .$b == run 1 .$a"; done
rm -f $S/cards/replay.img
cp -c $S/cards/isfile-base.img $S/cards/isfile.img
$M/run-rpi.sh $S/bin/rpi4-rel.img $S/cards/isfile.img $R/serial/rpi4-isfile.log "Press any key to exit" 90
rm -f $S/cards/isfile.img $S/cards/*-base.img
shasum -a 256 $S/prog/* $S/bin/*.img
