#!/bin/sh
# run-tables.sh <out-dir> <tree> <scratch> -- the two BIOS16 retest tables on
# NewTOS (QEMU raspi4b) through `cargo xtask tosdump` (TOOL09), each against
# both oracle references (tt, falcontc). Run from the repository root.
#   before: <tree> = `git archive b27569f9` unpacked in the scratch (HEAD before BIOS16)
#   after : <tree> = the working tree with BIOS16
OUT=$1; TREE=$2; SCR=$3
R=tosdump/reference
mkdir -p "$OUT"
run() { # name table refbase
  cargo xtask tosdump bios tosdump/cases/$2 --out "$OUT/$1" --tree "$TREE" --scratch "$SCR" \
     --reference $R/EMUTOS-1a7484ab-$3.BIN --reference $R/EMUTOS-1a7484ab-$3-falcontc.BIN > "$OUT/$1.stdout" 2>&1
  echo "$1 exit $?"; grep -E "differ" "$OUT/$1.stdout"
}
run bios-core-r1 bios-core-r1.tsv BIOS-core
run super-r1 super-r1.tsv SUPER
