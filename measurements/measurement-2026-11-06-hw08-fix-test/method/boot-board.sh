#!/usr/bin/env bash
# boot-board.sh <kernel8.img> <out-dir> : rpi4 raspi4b boot of a kernel on a fresh tools/mkcard.sh card until "desktop up" (bounded 100 s).
set -u
SP=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/7fba4f48-e8f1-4a5e-8c58-696d17828400/scratchpad
R=/Volumes/Daten/Development/PiRusTOS
K=$1; O=$2; mkdir -p $O
tools_card=$R/tools/mkcard.sh
cd $R; printf '#R 02\r\n' > $SP/nd.inf
$tools_card $SP/boot-card.img $SP/nd.inf > $O/mkcard.out 2>&1; cp $SP/boot-card.img.manifest $O/card.img.manifest
: > $O/BOOT-SERIAL.txt
t0=$(date +%s)
qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:$O/BOOT-SERIAL.txt -append serial=on -kernel $K -drive file=$SP/boot-card.img,if=sd,format=raw &
pid=$!
for i in $(seq 1 100); do sleep 1; grep -q "desktop up" $O/BOOT-SERIAL.txt && break; done
t1=$(date +%s); echo "desktop-up after $((t1-t0)) s (wall, incl. QEMU start)" > $O/boottime.txt
sleep 2; kill $pid 2>/dev/null; sleep 1; kill -9 $pid 2>/dev/null; wait $pid 2>/dev/null
grep -c -i "exception\|panic" $O/BOOT-SERIAL.txt >> $O/boottime.txt; grep "desktop up" $O/BOOT-SERIAL.txt >> $O/boottime.txt
rm -f $SP/boot-card.img
cat $O/boottime.txt
