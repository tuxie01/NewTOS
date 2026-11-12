#!/bin/sh
# newtos-bios-r1.sh <frozen tree> <scratch> <out> -- the DMA sound inquiry rows of
# tosdump/cases/bios-r1.tsv (230-235) on NewTOS, frozen tree, both pinned references.
T=$1; S=$2; O=$3; REF=tosdump/reference/EMUTOS-1a7484ab
cargo xtask tosdump bios tosdump/cases/bios-r1.tsv --tree "$T" --out "$O" --scratch "$S" \
    --features bios-arg-trace --reference $REF-BIOS.BIN --reference $REF-BIOS-falcontc.BIN
