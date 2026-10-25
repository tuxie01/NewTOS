#!/usr/bin/env bash
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/651de586-7dd7-487b-9e92-c8525c58f508/scratchpad/a111-test
cd $S
r() { python3 method/run.py "$@"; rm -rf "$S/cards/run-$3"; }
for v in base new; do
 r $S/bin/kernel8-$v-A.img $S/cards/base.img nf-$v $S/serial $S/shots 40 900:1.5:s1-open 1500:1.5:s2-A 2100:1.5:s3-AB 2700:1.5:s4-left 3300:1.5:s5-left 3900:1.5:s6-right 4500:1.5:s7-bksp
 r $S/bin/kernel8-$v-B.img $S/cards/base.img rn-$v $S/serial $S/shots 40 1100:1.5:r1-open 1700:1.5:r2-left 2300:1.5:r3-bksp 2900:1.5:r4-X 3500:1.5:r5-esc
 r $S/bin/kernel8-$v-A.img $S/cards/qed.img fsel-$v $S/serial $S/shots 60 1100:1.5:f1 1400:1.5:f2 1700:1.5:f3 2100:1.5:f4 2500:1.5:f5 2900:1.5:f6 3300:1.5:f7
done
echo SERIES1-DONE
