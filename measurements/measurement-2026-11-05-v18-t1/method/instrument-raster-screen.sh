#!/bin/sh
# V18 T1: the read-back instrument (vro_cpyfm screen->memory, 16 planes) against the pin.
SCR=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/15767204-8d89-43e0-aae0-d8149a16a17d/scratchpad/v18
cargo xtask tosdump rast tosdump/cases/vdi-raster-screen.tsv --tree $SCR/tree \
  --out measurements/measurement-2026-11-05-v18-t1/runs/newtos-raster-screen \
  --reference tosdump/reference/EMUTOS-1a7484ab-RASTER-screen-falcontc.BIN \
  --scratch $SCR/xt
