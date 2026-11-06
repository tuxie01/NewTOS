#!/bin/sh
# V18 part 2: the new tables on the pinned oracle (falcontc twice, raster also tt); dumpers built from the
# main tree's tosdump/ (HEAD 4a8e1ae3, build id 4a8e) into scratch. Run from the main tree.
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/15767204-8d89-43e0-aae0-d8149a16a17d/scratchpad/v18t2
M=measurements/measurement-2026-11-06-v18-t2
set -e
make -s -C tosdump BUILD="$S/dumpers/4a8e" BUILD_ID=4a8e dumpvdi-m68k dumprast-m68k
mkdir -p $M/oracle
for r in 1 2; do
  tools/hatari-oracle.sh falcontc $S/dumpers/4a8e/DUMPVDI.PRG tosdump/cases/vdi-v18-t2.tsv $S/or-vdi-$r > $M/oracle/vdi-falcontc-$r.stdout 2>&1
  tools/hatari-oracle.sh falcontc $S/dumpers/4a8e/DUMPRAST.PRG tosdump/cases/vdi-v18-t2-raster.tsv $S/or-rast-$r > $M/oracle/rast-falcontc-$r.stdout 2>&1
done
tools/hatari-oracle.sh tt $S/dumpers/4a8e/DUMPRAST.PRG tosdump/cases/vdi-v18-t2-raster.tsv $S/or-rast-tt > $M/oracle/rast-tt.stdout 2>&1
for d in or-vdi-1 or-vdi-2 or-rast-1 or-rast-2 or-rast-tt; do
  mkdir -p $M/oracle/$d; cp $S/$d/*.BIN $S/$d/*.LOG $S/$d/hatari.log $S/$d/profile.txt $M/oracle/$d/
done
shasum -a 256 $M/oracle/*/*.BIN $M/oracle/*/*.LOG $S/dumpers/4a8e/*.PRG
