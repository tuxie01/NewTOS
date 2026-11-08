#!/bin/bash
# V40: base vs fix screendumps of gui.sh (cmp per shot) and the exception/panic lines of each boot.
S=${S:?}
G=$S/runs/gui
for n in desk480 desk1080 bc480 bc1080; do
  b=$G/$n-base/shots; f=$G/$n-fix/shots; nb=$(ls $b | /usr/bin/grep -c "\.png$"); nf=$(ls $f | /usr/bin/grep -c "\.png$"); d=0; dl=""
  for s in $(ls $b | /usr/bin/grep "\.png$"); do cmp -s $b/$s $f/$s || { d=$((d+1)); dl="$dl $s"; }; done
  eb=$(/usr/bin/grep -a -c -i -E "exception|panic" $G/$n-base/BOOT-SERIAL.txt); ef=$(/usr/bin/grep -a -c -i -E "exception|panic" $G/$n-fix/BOOT-SERIAL.txt)
  echo "$n: $nb shots base, $nf fix, $d differ:$dl | exc/panic base $eb fix $ef"
done
