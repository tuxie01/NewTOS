#!/usr/bin/env bash
# run-rpi4.sh <tag> <autorun TOS path> <append> <max seconds> <done-regex>
# One QEMU raspi4b run on an APFS clone of base.img with NEWDESK.INF #Z set to
# <autorun>; LDGLOOP.NCP copied to C:\ when present beside this script's run
# dir. Stops at <done-regex> in the serial log (+10 s) or after <max seconds>,
# takes a QMP screendump, deletes the clone.
set -uo pipefail
F=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/15767204-8d89-43e0-aae0-d8149a16a17d/scratchpad/proc08-fix
T=$1; AUTO=$2; APP=$3; SECS=$4; DONE=$5
OFF=$((63*512))
CARD=$F/run-$T.img
cp -c $F/base.img $CARD
sed "s|^#Z .*|#Z 01 ${AUTO//\\/\\\\}@|" /Volumes/Daten/Development/PiRusTOS/measurements/measurement-2026-10-07-zview04/method/newdesk-template.inf > $F/NEWDESK-$T.INF
mcopy -o -i "$CARD@@$OFF" $F/NEWDESK-$T.INF ::NEWDESK.INF
[ -f $F/LDGLOOP.NCP ] && mcopy -o -i "$CARD@@$OFF" $F/LDGLOOP.NCP ::LDGLOOP.NCP
rm -f $F/serial/$T.log $F/q-$T.sock
cd $F
qemu-system-aarch64 -M raspi4b -no-reboot -display none -append "$APP" -kernel $F/kernel8-rpi4.img \
  -drive file=$CARD,if=sd,format=raw -serial file:$F/serial/$T.log -qmp unix:q-$T.sock,server=on,wait=off &
P=$!
t=0
while [ $t -lt $SECS ]; do
  sleep 5; t=$((t+5))
  kill -0 $P 2>/dev/null || { echo "qemu exited at ${t}s"; break; }
  if grep -Eq "$DONE" $F/serial/$T.log 2>/dev/null; then sleep 10; t=$((t+10)); echo "done-regex at ~${t}s"; break; fi
done
python3 -I $F/method/qmp.py q-$T.sock "{\"execute\":\"screendump\",\"arguments\":{\"filename\":\"$F/screendumps/$T.png\",\"format\":\"png\"}}"
kill $P 2>/dev/null; sleep 1; kill -9 $P 2>/dev/null
rm -f $CARD $F/q-$T.sock
echo "end $T after ~${t}s"
