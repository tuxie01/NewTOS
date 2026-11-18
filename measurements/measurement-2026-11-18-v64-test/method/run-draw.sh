#!/bin/bash
# run-draw.sh -- temporary table tmp-v63-draw.tsv (fills/lines/text/v_clrwk/contourfill/XOR read back by v_get_pixel) at main, default, vdiplanes=16, vdiplanes=32
A=/Volumes/Daten/Development/PiRusTOS/target-agents/agent-af3fb0887100d947d
cd $A/wt
for spec in "main base -" "d wt -" "v16 wt serial=on_vdiplanes=16" "v32 wt serial=on_vdiplanes=32"; do
  set -- $spec; cfg=$1; tree=$2; unset V62_APPEND; [ "$3" != - ] && export V62_APPEND="$(echo $3 | tr _ ' ')"
  mkdir -p $A/scratch/ts-dr
  cargo xtask tosdump vdi tosdump/cases/tmp-v63-draw.tsv --tree $A/$tree --out $A/runs/draw-$cfg --scratch $A/scratch/ts-dr > $A/runs/draw-$cfg.stdout 2>&1
  echo "$cfg exit $?"; grep "result\|run  " $A/runs/draw-$cfg.stdout | cut -c1-200
  rm -rf $A/scratch/ts-dr
done
