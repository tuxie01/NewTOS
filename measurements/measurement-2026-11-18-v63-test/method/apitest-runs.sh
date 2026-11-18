#!/bin/bash
K=/Volumes/Daten/Development/PiRusTOS/target-agents/agent-a9aec45507eb228c7/wt/../kern; C=/Volumes/Daten/Development/PiRusTOS/target-agents/agent-a9aec45507eb228c7/wt/../scratch/cards/apitest.img; M=$(cd $(dirname $0) && pwd)
a() { $M/run-qmp.sh rpi4 $K/$1-rpi4.img $C $2 "serial=on $3" 'TOTAL' 3 0; }
a wtmain api-main ""
a wt api-default ""
a wt api-v16 "vdiplanes=16"
a wt api-v32 "vdiplanes=32"
