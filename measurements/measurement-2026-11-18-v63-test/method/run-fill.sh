#!/bin/bash
# run-draw.sh -- temporary table tmp-v63-fill.tsv (fills/lines/text/v_clrwk/contourfill/XOR read back by v_get_pixel) at main, default, vdiplanes=16, vdiplanes=32
A=/Volumes/Daten/Development/PiRusTOS/target-agents/agent-a9aec45507eb228c7
cd $A/wt
for spec in "main wtmain -" "d wt -" "v16 wt serial=on_vdiplanes=16" "v32 wt serial=on_vdiplanes=32"; do
  set -- $spec; cfg=$1; tree=$2; unset V62_APPEND; [ "$3" != - ] && export V62_APPEND="$(echo $3 | tr _ ' ')"
  mkdir -p $A/scratch/ts-fl
  cargo xtask tosdump vdi tosdump/cases/tmp-v63-fill.tsv --tree $A/$tree --out $A/runs/fill-$cfg --scratch $A/scratch/ts-fl > $A/runs/fill-$cfg.stdout 2>&1
  echo "$cfg exit $?"; grep "result\|run  " $A/runs/fill-$cfg.stdout | cut -c1-200
  rm -rf $A/scratch/ts-fl
done
