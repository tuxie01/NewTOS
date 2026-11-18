#!/bin/sh
# run-all.sh <scratch> <runs-dir> -- XBIOS-bearing tables, every pinned reference profile (tt, falcontc), non-disk, --scratch in the session scratchpad.
S=$1; R=$2; REF=tosdump/reference/EMUTOS-1a7484ab
run() { t=$1; shift; cargo xtask tosdump bios tosdump/cases/$t.tsv --out "$R/$t" --scratch "$S" "$@" > "$R/$t.stdout" 2>&1; echo "$t exit $?"; }
run bios-r1 --reference $REF-BIOS.BIN --reference $REF-BIOS-falcontc.BIN
run super-r1 --reference $REF-SUPER.BIN --reference $REF-SUPER-falcontc.BIN
for n in bconmap chardev flush ptr system; do
  run xbios-$n-r1 --reference $REF-XBIOS-$n.BIN --reference $REF-XBIOS-$n-falcontc.BIN
done
