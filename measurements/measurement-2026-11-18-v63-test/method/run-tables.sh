#!/bin/bash
# run-tables.sh <cfg> <tree-rel-to-agent-dir> <append|-> -- all XBIOS + VDI + raster tables on QEMU raspi4b via cargo xtask tosdump
# run from the branch worktree (patched xtask: V62_APPEND replaces the fixed "-append serial=on"; method/tester-xtask-append.patch)
CFG=$1; TREE=$2; APP=$3
A=/Volumes/Daten/Development/PiRusTOS/target-agents/agent-a9aec45507eb228c7
cd $A/wt; REF=tosdump/reference/EMUTOS-1a7484ab
S=$A/scratch/ts-$CFG; mkdir -p $S; O=$A/runs/$CFG; mkdir -p $O
[ "$APP" != - ] && export V62_APPEND="$APP"
run() { d=$1; t=$2; shift 2; cargo xtask tosdump $d tosdump/cases/$t.tsv --tree $A/$TREE --out $O/$t --scratch $S "$@" > $O/$t.stdout 2>&1; echo "$CFG $t exit $? $(grep -h 'tosdiff EMUTOS' $O/$t.stdout | sed 's/.*vs NEWTOS.BIN: //' | tr '\n' ';')"; rm -rf $S/tosdump-*; }
run bios bios-r1 --reference $REF-BIOS.BIN --reference $REF-BIOS-falcontc.BIN
run bios super-r1 --reference $REF-SUPER.BIN --reference $REF-SUPER-falcontc.BIN
for n in bconmap chardev flush ptr system; do run bios xbios-$n-r1 --reference $REF-XBIOS-$n.BIN --reference $REF-XBIOS-$n-falcontc.BIN; done
run vdi vdi-all --reference $REF-VDI-all.BIN --reference $REF-VDI-all-falcontc.BIN
run vdi vdi-v18 --reference $REF-VDI-v18-falcontc.BIN
run vdi vdi-v18-t2 --reference $REF-VDI-v18t2-falcontc.BIN
run vdi vdi-v18-t3 --reference $REF-VDI-v18t3-falcontc.BIN
run vdi vdi-a131-palette --reference $REF-VDI-a131pal.BIN --reference $REF-VDI-a131pal-falcontc.BIN
run rast vdi-raster --reference $REF-RASTER.BIN
run rast vdi-raster-screen --reference $REF-RASTER-screen-falcontc.BIN
run rast vdi-v18-raster --reference $REF-RASTER-v18-falcontc.BIN
run rast vdi-v18-t2-raster --reference $REF-RASTER-v18t2-falcontc.BIN
echo "$CFG DONE"
