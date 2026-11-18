#!/bin/bash
K=/Volumes/Daten/Development/PiRusTOS/target-agents/agent-a9aec45507eb228c7/kern; C=/Volumes/Daten/Development/PiRusTOS/target-agents/agent-a9aec45507eb228c7/scratch/cards/vdidraw.img; M=$(cd $(dirname $0) && pwd)
v() { $M/run-qmp.sh rpi4 $K/$1-rpi4.img $C $2 "serial=on $3" 'holding for' 50 0; }
v wtmain draw2-main ""
v wt draw2-default ""
v wt draw2-v16 "vdiplanes=16"
v wt draw2-v32 "vdiplanes=32"
