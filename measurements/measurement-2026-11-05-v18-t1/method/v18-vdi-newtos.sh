#!/bin/sh
# V18 T1: K10/K1/K2/K4/K11 pixel read-backs (tosdump/cases/vdi-v18.tsv) on NewTOS.
SCR=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/15767204-8d89-43e0-aae0-d8149a16a17d/scratchpad/v18
cargo xtask tosdump vdi tosdump/cases/vdi-v18.tsv --tree $SCR/tree \
  --out measurements/measurement-2026-11-05-v18-t1/runs/newtos-vdi-v18 \
  --reference tosdump/reference/EMUTOS-1a7484ab-VDI-v18-falcontc.BIN \
  --scratch $SCR/xt
