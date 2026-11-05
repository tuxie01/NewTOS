#!/bin/sh
# V18 T1 K7: the memory->memory table on NewTOS against the new pin reference.
SCR=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/15767204-8d89-43e0-aae0-d8149a16a17d/scratchpad/v18
cargo xtask tosdump rast tosdump/cases/vdi-v18-raster.tsv --tree $SCR/tree \
  --out measurements/measurement-2026-11-05-v18-t1/runs/newtos-v18-raster \
  --reference tosdump/reference/EMUTOS-1a7484ab-RASTER-v18-falcontc.BIN \
  --scratch $SCR/xt
