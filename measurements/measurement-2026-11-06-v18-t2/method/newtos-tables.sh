#!/bin/sh
# V18 part 2: tables on NewTOS (frozen tree = git archive 4a8e1ae3 in scratch), one cargo xtask tosdump call
# each, from the main tree; plus the canonical vdi-all/vdi-raster-screen pre-flight.
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/15767204-8d89-43e0-aae0-d8149a16a17d/scratchpad/v18t2
M=measurements/measurement-2026-11-06-v18-t2
R=tosdump/reference
run() { D=$1; TAB=$2; O=$3; shift 3
  refs=""; for r in "$@"; do refs="$refs --reference $R/$r.BIN"; done
  cargo xtask tosdump $D tosdump/cases/$TAB --tree $S/head --out $M/runs/$O $refs --scratch $S/xt; }
set -x
run vdi vdi-all.tsv newtos-vdi-all EMUTOS-1a7484ab-VDI-all-falcontc
run rast vdi-raster-screen.tsv newtos-raster-screen EMUTOS-1a7484ab-RASTER-screen-falcontc
run vdi vdi-v18-t2.tsv newtos-vdi-v18t2 EMUTOS-1a7484ab-VDI-v18t2-falcontc
run rast vdi-v18-t2-raster.tsv newtos-rast-v18t2 EMUTOS-1a7484ab-RASTER-v18t2-falcontc
echo finished
