#!/bin/sh
# rerun of bios-r2 and bios-r2-phys at HEAD 24eb0b070 (the first pass ran them at b57e4bde1; a docs-only
# commit landed meanwhile, so the whole set is repeated at one HEAD). Same call as run-all.sh.
S=$1; R=$2; REF=tosdump/reference/EMUTOS-1a7484ab
for T in r2 r2-phys; do
  case $T in r2) ref=BIOS-r2-disk;; *) ref=BIOS-$T-disk;; esac
  cargo xtask tosdump bios tosdump/cases/bios-$T.tsv --disk --out "$R/bios-$T" --scratch "$S" --reference $REF-$ref.BIN > "$R/bios-$T.stdout" 2>&1; echo "bios-$T exit $?"
done
