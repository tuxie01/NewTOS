#!/bin/sh
# A131: the palette table on NewTOS (TOOL09 one-command run), from the main tree.
# S = session scratchpad/a131; tree = git clone of main HEAD f657cfe4 (+ references symlink).
cargo xtask tosdump vdi tosdump/cases/vdi-a131-palette.tsv --tree "$S/tree" \
  --out measurements/measurement-2026-11-05-a131/runs/newtos-pal \
  --reference tosdump/reference/EMUTOS-1a7484ab-VDI-a131pal-falcontc.BIN \
  --reference tosdump/reference/EMUTOS-1a7484ab-VDI-a131pal.BIN \
  --scratch "$S/xt"
