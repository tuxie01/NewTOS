#!/bin/sh
# tables.sh <worktree> <scratch> <runs-dir> -- the three tables of BIOS06 O3 acceptance on the branch tree, as
# measurement-2026-10-29-bios06/method/newtos-tables.sh "after" (references: the committed BINs, tt + falcontc where the
# October run used both; bios-r1/bios-core-r1 as measurement-2026-11-11-bios-full: tt + falcontc).
set -u
W=$1; S=$2; R=$3; mkdir -p "$R"; R=$(cd "$R" && pwd); cd "$W"
REF=tosdump/reference
run() { cargo xtask tosdump bios "tosdump/cases/$1.tsv" --reference "$REF/$2" --reference "$REF/$3" \
    --out "$R/$1" --scratch "$S" > "$R/$1.stdout" 2>&1; echo "$1: cargo xtask tosdump exit $?"; }
run xbios-ptr-r1 EMUTOS-1a7484ab-XBIOS-ptr.BIN EMUTOS-1a7484ab-XBIOS-ptr-falcontc.BIN
run bios-core-r1 EMUTOS-1a7484ab-BIOS-core.BIN EMUTOS-1a7484ab-BIOS-core-falcontc.BIN
run bios-r1 EMUTOS-1a7484ab-BIOS.BIN EMUTOS-1a7484ab-BIOS-falcontc.BIN
