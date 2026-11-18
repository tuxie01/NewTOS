#!/bin/bash
A=/Volumes/Daten/Development/PiRusTOS/target-agents/agent-a5835b4b6e8d83e6f; M=$(cd $(dirname $0) && pwd)
t() { TMO=35 bash $M/run-qmp.sh rpi4 $A/kern/$1-rpi4.img $A/scratch/cards/vt52.img $2 "serial=on $3" ZZZNOMATCH 0 0; }
t base vt-main ""
t wt vt-default ""
t wt vt-v16 "vdiplanes=16"
t wt vt-v32 "vdiplanes=32"
