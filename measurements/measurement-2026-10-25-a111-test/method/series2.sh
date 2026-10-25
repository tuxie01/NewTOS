#!/usr/bin/env bash
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/651de586-7dd7-487b-9e92-c8525c58f508/scratchpad/a111-test
cd $S
r() { python3 method/run.py "$@"; rm -rf "$S/cards/run-$3"; }
for v in base new; do
 r $S/bin/kernel8-$v-C.img $S/cards/td.img td-$v $S/serial $S/shots 50 510:3:t1-open 1600:1.5:t2-a 2200:1.5:t3-ab 2800:1.5:t4-left 3400:1.5:t5-left 4000:1.5:t6-right 4600:1.5:t7-bksp
done
echo done
