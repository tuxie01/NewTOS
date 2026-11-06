#!/bin/sh
# FS31 test: vdi-all (against the pinned oracle's reference) and AES r2 (aes-r2-wind) on the base (6b53dc5b) and the fix (20f87d1d) trees.
# Run from the main tree. $S = session scratch with trees fix/ base/.
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/7fba4f48-e8f1-4a5e-8c58-696d17828400/scratchpad/fs31t
M=measurements/measurement-2026-11-06-fs31-test
R=tosdump/reference
set -x
for T in base fix; do
  cargo xtask tosdump vdi tosdump/cases/vdi-all.tsv --tree $S/$T --out $M/runs/tab-$T-all --reference $R/EMUTOS-1a7484ab-VDI-all-falcontc.BIN --scratch $S/xt-$T
  cargo xtask tosdump aes tosdump/cases/aes-r2-wind.tsv --tree $S/$T --out $M/runs/tab-$T-r2 --reference $R/EMUTOS-1a7484ab-AES-r2wind.BIN --scratch $S/xt-$T
done
