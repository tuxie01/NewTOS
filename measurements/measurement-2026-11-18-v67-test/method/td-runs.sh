#!/bin/bash
# td-runs.sh -- TeraDesk (cicons.rsc) desktop on raspi4b, td.img, per kernel/config; serial + QMP screendump after TMO seconds
A=/Volumes/Daten/Development/PiRusTOS/target-agents/agent-a5835b4b6e8d83e6f; M=$(cd $(dirname $0) && pwd)
t() { TMO=50 bash $M/run-qmp.sh rpi4 $A/kern/$1-rpi4.img $A/scratch/cards/td.img $2 "serial=on $3" ZZZNOMATCH 0 0; }
t wt td-v16 "vdiplanes=16"
t wt td-v32 "vdiplanes=32"
