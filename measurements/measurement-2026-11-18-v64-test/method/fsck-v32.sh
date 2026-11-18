#!/bin/bash
# fsck-v32.sh -- APITEST card clone booted at vdiplanes=32 to TOTAL, killed; FAT partition dd'ed out; fsck_msdos -n
A=/Volumes/Daten/Development/PiRusTOS/target-agents/agent-af3fb0887100d947d; D=$A/xt/fsck; mkdir -p $D; cd $D
cp -c $A/scratch/cards/apitest.img card.img; sha256sum() { shasum -a 256 "$@"; }
LOG=$D/serial.log; rm -f $LOG
qemu-system-aarch64 -M raspi4b -no-reboot -display none -append "serial=on vdiplanes=32" -kernel $A/kern/wt-rpi4.img -drive file=card.img,if=sd,format=raw -serial file:$LOG &
Q=$!
for _ in $(seq 1 200); do grep -aq TOTAL $LOG 2>/dev/null && break; sleep 1; done; sleep 4; kill $Q; wait $Q 2>/dev/null
grep -a "TOTAL" $LOG | tail -1
echo "sha256 after run: $(shasum -a 256 card.img)"
fdisk card.img 2>/dev/null | head -8
START=$(python3 -I -c "import struct;d=open('card.img','rb').read(512);print(struct.unpack('<I',d[446+8:446+12])[0], struct.unpack('<I',d[446+12:446+16])[0])")
echo "partition 1 start,sectors: $START"; set -- $START
dd if=card.img of=fat.img bs=512 skip=$1 count=$2 2>/dev/null
fsck_msdos -n fat.img; echo "fsck exit $?"
cp $LOG /Volumes/Daten/Development/PiRusTOS/measurements/measurement-2026-11-18-v64-test/serial/fsck-v32-boot.log
cd $A; rm -rf $D
