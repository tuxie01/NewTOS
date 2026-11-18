#!/bin/bash
# boots.sh -- boot runs through run-qmp.sh: serial log + QMP screendump per run
A=/Volumes/Daten/Development/PiRusTOS/target-agents/agent-a3ceacd4986f7eaf1; K=$A/kern; C=$A/scratch/cards/plain.img; M=$(cd $(dirname $0) && pwd)
t() { b=$1; k=$2; tag=$3; app=$4
  if [ $b = rpi4 ]; then $M/run-qmp.sh rpi4 $K/$k-rpi4.img $C $tag "serial=on $app" 'desktop up' 5 0
  else $M/run-qmp.sh virt $K/$k-virt.img - $tag "$app" 'desktop up' 8 0; fi; }
for b in ${BOARDS:-rpi4 virt}; do
  t $b base $b-main ""
  t $b wt $b-default ""
  t $b wt $b-v16 "vdiplanes=16"
  t $b wt $b-v32 "vdiplanes=32"
done
