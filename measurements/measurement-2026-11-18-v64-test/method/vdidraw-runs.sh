#!/bin/bash
K=/Volumes/Daten/Development/PiRusTOS/target-agents/agent-af3fb0887100d947d/kern; C=/Volumes/Daten/Development/PiRusTOS/target-agents/agent-af3fb0887100d947d/scratch/cards/vdidraw.img; M=$(cd $(dirname $0) && pwd)
v() { $M/run-qmp.sh rpi4 $K/$1-rpi4.img $C $2 "serial=on $3" 'holding for' 30 0; }
v base draw-main ""
v wt draw-default ""
v wt draw-v16 "vdiplanes=16"
v wt draw-v32 "vdiplanes=32"
