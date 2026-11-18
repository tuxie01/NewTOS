#!/bin/bash
A=/Volumes/Daten/Development/PiRusTOS/target-agents/agent-a3ceacd4986f7eaf1; K=$A/kern; C=$A/scratch/cards/apitest.img; M=$(cd $(dirname $0) && pwd)
a() { $M/run-qmp.sh rpi4 $K/$1-rpi4.img $C $2 "serial=on $3" 'TOTAL' 3 0; }
a base api-main ""
a wt api-default ""
a wt api-v32 "vdiplanes=32"
