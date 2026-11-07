#!/bin/bash
# Auditor pre-flight: one case table on NewTOS at the audited commit (TOOL09). Run from the main tree; frozen tree = fresh clone in scratch.
S=$1
cd /Volumes/Daten/Development/PiRusTOS && cargo xtask tosdump vdi tosdump/cases/vdi-all.tsv --tree "$S/wt" --out "$S/pf/run" \
  --reference tosdump/reference/EMUTOS-1a7484ab-VDI-all.BIN --reference tosdump/reference/EMUTOS-1a7484ab-VDI-all-falcontc.BIN --scratch "$S/pf/scratch"
