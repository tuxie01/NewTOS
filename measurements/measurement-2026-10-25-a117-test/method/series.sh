#!/usr/bin/env bash
# series.sh : A117 base-vs-new series (one QEMU at a time). run dirs are per-run APFS clones, removed afterwards.
A=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/651de586-7dd7-487b-9e92-c8525c58f508/scratchpad/a117-test
cd $A
r() { python3 method/run.py "$@" ; rm -rf "$A/cards/run-$3"; }
rm -rf cards/run-base1
for n in 2 3; do r $A/bin/kernel8-base-a.img $A/cards/a117.img base$n $A/serial $A/shots 60 510:3:acopen 1710:3:pick1 3210:3:pick2; done
for n in 1 2 3; do r $A/bin/kernel8-new-a.img $A/cards/a117.img new$n $A/serial $A/shots 60 510:3:acopen 1710:3:pick1 3210:3:pick2; done
for n in 1 2 3; do r $A/bin/kernel8-base-td.img $A/cards/td.img basetd$n $A/serial $A/shots 100 510:3:acopen 1710:3:quit; done
for n in 1 2 3; do r $A/bin/kernel8-new-td.img $A/cards/td.img newtd$n $A/serial $A/shots 100 510:3:acopen 1710:3:quit; done
echo SERIES-DONE
