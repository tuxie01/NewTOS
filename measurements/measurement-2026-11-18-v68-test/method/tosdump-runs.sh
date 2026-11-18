#!/bin/bash
A=/Volumes/Daten/Development/PiRusTOS/target-agents/agent-a3ceacd4986f7eaf1
R=tosdump/reference/EMUTOS-1a7484ab
RF="--reference $R-BIOS.BIN --reference $R-BIOS-falcontc.BIN"
cd $A/base && cargo xtask tosdump bios tosdump/cases/bios-r1.tsv --out $A/runs/main-default --scratch $A/scratch/base $RF > $A/out/td-main-default.txt 2>&1; echo rc $? >> $A/out/td-main-default.txt
cd $A/wt && cargo xtask tosdump bios tosdump/cases/bios-r1.tsv --out $A/runs/wt-default --scratch $A/scratch/wt1 $RF > $A/out/td-wt-default.txt 2>&1; echo rc $? >> $A/out/td-wt-default.txt
cd $A/wt && cargo xtask tosdump bios tosdump/cases/bios-r1.tsv --out $A/runs/wt-planes32 --scratch $A/scratch/wt2 --bootargs vdiplanes=32 $RF > $A/out/td-wt-32.txt 2>&1; echo rc $? >> $A/out/td-wt-32.txt
echo done > $A/out/run.done
