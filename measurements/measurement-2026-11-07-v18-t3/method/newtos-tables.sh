#!/bin/sh
# V18 part 3: tables on NewTOS (frozen tree = git archive 5a8e01b5 in scratch), one cargo xtask tosdump call each,
# from the main tree: the canonical vdi-all pre-flight and the new K23 table.
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/ed7bb9a3-0fc2-4b37-91bb-ed70559503a3/scratchpad/t3
M=measurements/measurement-2026-11-07-v18-t3
R=tosdump/reference
cargo xtask tosdump vdi tosdump/cases/vdi-all.tsv --tree $S/head --out $M/runs/newtos-vdi-all --reference $R/EMUTOS-1a7484ab-VDI-all-falcontc.BIN --scratch $S/xt
cargo xtask tosdump vdi tosdump/cases/vdi-v18-t3.tsv --tree $S/head --out $M/runs/newtos-vdi-v18t3 --reference $R/EMUTOS-1a7484ab-VDI-v18t3-falcontc.BIN --scratch $S/xt
