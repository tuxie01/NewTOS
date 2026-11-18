#!/bin/bash
# run-si.sh -- temporary table tmp-v62-scrninfo.tsv (vq_extnd 0/1 + vq_scrninfo) at default, vdiplanes=16, vdiplanes=32
A=/Volumes/Daten/Development/PiRusTOS/target-agents/agent-ac5b7442f64fdf0de
cd $A/wt
for cfg in d v16 v32; do
  unset V62_APPEND
  case $cfg in v16) export V62_APPEND="serial=on vdiplanes=16";; v32) export V62_APPEND="serial=on vdiplanes=32";; esac
  mkdir -p $A/scratch/ts-si
  cargo xtask tosdump vdi tosdump/cases/tmp-v62-scrninfo.tsv --tree $A/wt --out $A/runs/si-$cfg --scratch $A/scratch/ts-si > $A/runs/si-$cfg.stdout 2>&1
  echo "$cfg exit $?"; grep "result\|run  " $A/runs/si-$cfg.stdout | cut -c1-200
  rm -rf $A/scratch/ts-si
done
