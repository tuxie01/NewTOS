#!/bin/bash
# re-test runs on ef2a14395 (wt): the 11 tables at vdiplanes=32, the 3 contract tables, vdi-v18 and vdi-raster at the default boot
A=/Volumes/Daten/Development/PiRusTOS/target-agents/agent-a1ff55b6fbd5f4b6f
cd $A/wt; R=tosdump/reference/EMUTOS-1a7484ab; S=$A/scratch; mkdir -p $S $A/runs
t() { name=$1; shift; cargo xtask tosdump "$@" --out $A/runs/$name --scratch $S > $A/runs/$name.stdout 2>&1; echo "$name xtask-exit $?"; rm -rf $S/tosdump-*; }
B="--bootargs vdiplanes=32"
t v32-vdi-v18 vdi tosdump/cases/vdi-v18.tsv $B --reference $R-VDI-v18-falcontc.BIN
t v32-vdi-all vdi tosdump/cases/vdi-all.tsv $B --reference $R-VDI-all.BIN --reference $R-VDI-all-falcontc.BIN
t v32-vdi-v18-t2 vdi tosdump/cases/vdi-v18-t2.tsv $B --reference $R-VDI-v18t2-falcontc.BIN
t v32-vdi-v18-t3 vdi tosdump/cases/vdi-v18-t3.tsv $B --reference $R-VDI-v18t3-falcontc.BIN
t v32-vdi-raster rast tosdump/cases/vdi-raster.tsv $B --reference $R-RASTER.BIN
t v32-vdi-raster-screen rast tosdump/cases/vdi-raster-screen.tsv $B --reference $R-RASTER-screen-falcontc.BIN
t v32-vdi-v18-raster rast tosdump/cases/vdi-v18-raster.tsv $B --reference $R-RASTER-v18-falcontc.BIN
t v32-vdi-v18-t2-raster rast tosdump/cases/vdi-v18-t2-raster.tsv $B --reference $R-RASTER-v18t2-falcontc.BIN
t v32-aes-r1 aes tosdump/cases/aes-r1.tsv $B --reference $R-AES.BIN
t v32-bios-r1 bios tosdump/cases/bios-r1.tsv $B --reference $R-BIOS.BIN --reference $R-BIOS-falcontc.BIN
t c-raster32 rast tosdump/cases/vdi-raster32.tsv $B
t c-rscreen32 rast tosdump/cases/vdi-raster-screen32.tsv $B
t c-a131pal32 vdi tosdump/cases/vdi-a131-palette.tsv $B --reference $R-VDI-a131pal.BIN --reference $R-VDI-a131pal-falcontc.BIN
t d-vdi-v18 vdi tosdump/cases/vdi-v18.tsv --reference $R-VDI-v18-falcontc.BIN
t d-vdi-raster rast tosdump/cases/vdi-raster.tsv --reference $R-RASTER.BIN
echo T2DONE
