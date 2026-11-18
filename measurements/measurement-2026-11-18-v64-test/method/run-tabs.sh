#!/bin/bash
# run-tabs.sh <table-name> -- temporary table tmp-v64-<name>.tsv at main, default, vdiplanes=16, vdiplanes=32 (patched xtask in wt only)
A=/Volumes/Daten/Development/PiRusTOS/target-agents/agent-af3fb0887100d947d; N=$1
cd $A/wt
for spec in "main base -" "d wt -" "v16 wt serial=on_vdiplanes=16" "v32 wt serial=on_vdiplanes=32"; do
  set -- $spec; cfg=$1; tree=$2; unset V62_APPEND; [ "$3" != - ] && export V62_APPEND="$(echo $3 | tr _ ' ')"
  mkdir -p $A/scratch/ts-$N
  cargo xtask tosdump vdi tosdump/cases/tmp-v64-$N.tsv --tree $A/$tree --out $A/runs/$N-$cfg --scratch $A/scratch/ts-$N > $A/runs/$N-$cfg.stdout 2>&1
  echo "$N $cfg exit $?"; grep "result\|run  " $A/runs/$N-$cfg.stdout | cut -c1-200
  rm -rf $A/scratch/ts-$N
done
