#!/bin/sh
# V18 part 3: tosdump/cases/vdi-v18-t3.tsv on the pinned oracle (falcontc twice, tt once); DUMPVDI m68k
# from the main tree's tosdump/ at 5a8e01b5 (build id 5a8e). Run from the main tree.
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/ed7bb9a3-0fc2-4b37-91bb-ed70559503a3/scratchpad/t3
M=measurements/measurement-2026-11-07-v18-t3
make -s -C tosdump BUILD="$S/dumpers/5a8e" BUILD_ID=5a8e dumpvdi-m68k
for r in 1 2; do tools/hatari-oracle.sh falcontc $S/dumpers/5a8e/DUMPVDI.PRG tosdump/cases/vdi-v18-t3.tsv $S/or-t3-$r; done
tools/hatari-oracle.sh tt $S/dumpers/5a8e/DUMPVDI.PRG tosdump/cases/vdi-v18-t3.tsv $S/or-t3-tt
for d in or-t3-1 or-t3-2 or-t3-tt; do mkdir -p $M/oracle/$d; cp $S/$d/VDI.BIN $S/$d/VDI.LOG $S/$d/hatari.log $S/$d/profile.txt $M/oracle/$d/; done
# reference = or-t3-1
cp $S/or-t3-1/VDI.BIN tosdump/reference/EMUTOS-1a7484ab-VDI-v18t3-falcontc.BIN
cp $S/or-t3-1/VDI.LOG tosdump/reference/EMUTOS-1a7484ab-VDI-v18t3-falcontc.LOG
