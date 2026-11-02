#!/usr/bin/env bash
# runq.sh <tag> <autorun> <append> <times>
set -uo pipefail
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/15767204-8d89-43e0-aae0-d8149a16a17d/scratchpad/proc08
T=$1; AUTO=$2; APP=$3; TIMES=$4
OFF=$((63*512))
CARD=$S/run-$T.img
cp -c $S/base.img $CARD
sed "s|^#Z .*|#Z 01 ${AUTO//\\/\\\\}@|" /Volumes/Daten/Development/PiRusTOS/measurements/measurement-2026-10-07-zview04/method/newdesk-template.inf > $S/NEWDESK-$T.INF
mcopy -o -i "$CARD@@$OFF" $S/NEWDESK-$T.INF ::NEWDESK.INF
rm -f $S/$T.log $S/q-$T.sock
cd $S
qemu-system-aarch64 -M raspi4b -no-reboot -display none -append "$APP" -kernel $S/kernel8.img -drive file=$CARD,if=sd,format=raw -serial file:$S/$T.log -qmp unix:q-$T.sock,server=on,wait=off &
P=$!
python3 -I $S/sample.py q-$T.sock $S/$T "$TIMES" ${5:-2415}
sleep 1; kill -9 $P 2>/dev/null; rm -f $CARD $S/q-$T.sock
echo done $T
