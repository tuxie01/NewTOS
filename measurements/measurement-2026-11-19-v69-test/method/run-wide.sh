#!/bin/bash
# the 32-plane tables and the guard rails, on the branch commit tree (wt)
A=/Volumes/Daten/Development/PiRusTOS/target-agents/agent-a1ff55b6fbd5f4b6f
cd $A/wt; S=$A/scratch/wide; mkdir -p $S; O=$A/runs/wide; mkdir -p $O
R=tosdump/reference/EMUTOS-1a7484ab
t() { name=$1; shift; cargo xtask tosdump "$@" --scratch $S > $O/$name.stdout 2>&1; echo "$name exit $?"; rm -rf $S/tosdump-*; }
t raster32        rast tosdump/cases/vdi-raster32.tsv --out $O/raster32 --bootargs vdiplanes=32
t rscreen32       rast tosdump/cases/vdi-raster-screen32.tsv --out $O/rscreen32 --bootargs vdiplanes=32
t a131pal32       vdi  tosdump/cases/vdi-a131-palette.tsv --out $O/a131pal32 --bootargs vdiplanes=32 --reference $R-VDI-a131pal.BIN --reference $R-VDI-a131pal-falcontc.BIN
# guard rail (a): wide table with oracle reference; wide table without bootargs (default boot, 16 planes)
t g-a-oracleref   rast tosdump/cases/vdi-raster32.tsv --out $O/g-a-oracleref --bootargs vdiplanes=32 --reference $R-RASTER.BIN
t g-a-oracleref2  rast tosdump/cases/vdi-raster-screen32.tsv --out $O/g-a-oracleref2 --reference $R-RASTER-screen-falcontc.BIN
# guard rail (b): wide table at the default boot, at vdiplanes=16, 16-plane raster table at vdiplanes=32 (rscreen at 32 is in the wt-v32 set)
t g-b-wide-default rast tosdump/cases/vdi-raster32.tsv --out $O/g-b-wide-default
t g-b-wide-v16    rast tosdump/cases/vdi-raster32.tsv --out $O/g-b-wide-v16 --bootargs vdiplanes=16
t g-b-n16-at32    rast tosdump/cases/vdi-raster.tsv --out $O/g-b-n16-at32 --bootargs vdiplanes=32 --reference $R-RASTER.BIN
echo WIDEDONE
