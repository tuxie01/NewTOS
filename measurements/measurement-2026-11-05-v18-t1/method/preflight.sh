#!/bin/sh
# V18 Teil 1 pre-flight: one table on NewTOS at the audited commit (TOOL09).
# Run from the repository root. Tree = git archive of HEAD (c8ff6952) in scratch.
SCR=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/15767204-8d89-43e0-aae0-d8149a16a17d/scratchpad/v18
cargo xtask tosdump vdi tosdump/cases/vdi-all.tsv --tree $SCR/tree \
  --out measurements/measurement-2026-11-05-v18-t1/runs/newtos-vdi-all \
  --reference tosdump/reference/EMUTOS-1a7484ab-VDI-all-falcontc.BIN \
  --reference tosdump/reference/EMUTOS-1a7484ab-VDI-all.BIN \
  --scratch $SCR/xt
