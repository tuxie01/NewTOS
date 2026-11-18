#!/bin/bash
# boots.sh -- boot runs through run-qmp.sh: serial log + QMP screendump per run
K=/Volumes/Daten/Development/PiRusTOS/target-agents/agent-a7a3bbc49cba62b53/kern; C=/Volumes/Daten/Development/PiRusTOS/target-agents/agent-a7a3bbc49cba62b53/scratch/cards/plain.img; M=$(cd $(dirname $0) && pwd)
t() { b=$1; k=$2; tag=$3; app=$4
  if [ $b = rpi4 ]; then $M/run-qmp.sh rpi4 $K/$k-rpi4.img $C $tag "serial=on $app" 'desktop up' 5 0
  else $M/run-qmp.sh virt $K/$k-virt.img - $tag "$app" 'desktop up' 8 0; fi; }
for b in ${BOARDS:-rpi4 virt}; do
  t $b base $b-main ""
  t $b wt $b-default ""
  t $b wt $b-v16 "vdiplanes=16"
  t $b wt $b-v32 "vdiplanes=32"
  t $b wt $b-v24 "vdiplanes=24"
  t $b wt $b-vempty "vdiplanes="
  t $b wt $b-vx "vdiplanes=x"
done
