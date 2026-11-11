#!/bin/sh
# run-nondisk.sh <scratch> <runs-dir> -- bios-core-r1 and bios-r1 (second attempt; the first, with
# --scratch under /Volumes/Daten, failed in tools/mkcard.sh "Volume on disk failed to mount").
S=$1; R=$2; REF=tosdump/reference/EMUTOS-1a7484ab
cargo xtask tosdump bios tosdump/cases/bios-core-r1.tsv --out "$R/bios-core-r1" --scratch "$S" --reference $REF-BIOS-core.BIN --reference $REF-BIOS-core-falcontc.BIN > "$R/bios-core-r1.stdout" 2>&1; echo "bios-core-r1 exit $?"
cargo xtask tosdump bios tosdump/cases/bios-r1.tsv --out "$R/bios-r1" --scratch "$S" --reference $REF-BIOS.BIN --reference $REF-BIOS-falcontc.BIN > "$R/bios-r1.stdout" 2>&1; echo "bios-r1 exit $?"
