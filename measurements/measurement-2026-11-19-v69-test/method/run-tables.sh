#!/bin/bash
# run-tables.sh <tree: base|wt> <cfg: default|v16|v32> -- 16-plane VDI/raster/AES/BIOS tables, as the V67/V68 runs
A=/Volumes/Daten/Development/PiRusTOS/target-agents/agent-a1ff55b6fbd5f4b6f
T=$1; CFG=$2; LBL=$T-$CFG
BA=""; [ "$CFG" = v16 ] && BA="--bootargs vdiplanes=16"; [ "$CFG" = v32 ] && BA="--bootargs vdiplanes=32"
cd $A/$T; R=tosdump/reference/EMUTOS-1a7484ab
S=$A/scratch/$LBL; mkdir -p $S; O=$A/runs/$LBL; mkdir -p $O
run() { d=$1; t=$2; shift 2; cargo xtask tosdump $d tosdump/cases/$t.tsv --out $O/$t --scratch $S $BA "$@" > $O/$t.stdout 2>&1; echo "$LBL $t exit $?"; rm -rf $S/tosdump-*; }
run vdi vdi-all --reference $R-VDI-all.BIN --reference $R-VDI-all-falcontc.BIN
run vdi vdi-v18 --reference $R-VDI-v18-falcontc.BIN
run vdi vdi-v18-t2 --reference $R-VDI-v18t2-falcontc.BIN
run vdi vdi-v18-t3 --reference $R-VDI-v18t3-falcontc.BIN
run vdi vdi-a131-palette --reference $R-VDI-a131pal.BIN --reference $R-VDI-a131pal-falcontc.BIN
run rast vdi-raster --reference $R-RASTER.BIN
run rast vdi-raster-screen --reference $R-RASTER-screen-falcontc.BIN
run rast vdi-v18-raster --reference $R-RASTER-v18-falcontc.BIN
run rast vdi-v18-t2-raster --reference $R-RASTER-v18t2-falcontc.BIN
run aes aes-r1 --reference $R-AES.BIN
run bios bios-r1 --reference $R-BIOS.BIN --reference $R-BIOS-falcontc.BIN
echo "$LBL DONE"
