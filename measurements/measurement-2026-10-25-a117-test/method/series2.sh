#!/usr/bin/env bash
A=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/651de586-7dd7-487b-9e92-c8525c58f508/scratchpad/a117-test
cd $A
r() { python3 method/run.py "$@" ; rm -rf "$A/cards/run-$3"; }
for v in base new; do r $A/bin/kernel8-$v-a.img $A/cards/sys.img sys-$v $A/serial $A/shots 60 510:4:acopen 1710:3:pick1 3210:3:pick2; done
for v in base new; do r $A/bin/kernel8-$v-a.img $A/cards/noacc.img noacc-$v $A/serial $A/shots 60 510:3:acopen 1710:3:pick1 3210:3:pick2; done
for v in base new; do r $A/lint-$v/kernel8-rpi4-release.img $A/cards/api.img api-$v $A/serial $A/shots 150; done
echo SERIES2-DONE
