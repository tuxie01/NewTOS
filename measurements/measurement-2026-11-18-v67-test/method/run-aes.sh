#!/bin/bash
# run-aes.sh -- AES dumper (appl_init ap_planes in its LOG), aes-r1 table, four configs
A=/Volumes/Daten/Development/PiRusTOS/target-agents/agent-a5835b4b6e8d83e6f
cd $A/wt
for spec in "main base -" "default wt -" "v16 wt serial=on_vdiplanes=16" "v32 wt serial=on_vdiplanes=32"; do
  set -- $spec; CFG=$1; TREE=$2; APP=$(echo $3 | tr _ ' '); S=$A/scratch/ts-aes-$CFG; mkdir -p $S $A/runs/$CFG
  unset V62_APPEND; [ "$3" != - ] && export V62_APPEND="$APP"
  cargo xtask tosdump aes tosdump/cases/aes-r1.tsv --tree $A/$TREE --out $A/runs/$CFG/aes-r1 --scratch $S --reference tosdump/reference/EMUTOS-1a7484ab-AES.BIN > $A/runs/$CFG/aes-r1.stdout 2>&1
  echo "$CFG aes-r1 exit $? $(grep -h 'tosdiff EMUTOS' $A/runs/$CFG/aes-r1.stdout | sed 's/.*vs NEWTOS.BIN: //') | $(grep -a 'ap_planes' $A/runs/$CFG/aes-r1/*.LOG | head -1)"
  rm -rf $S
done
