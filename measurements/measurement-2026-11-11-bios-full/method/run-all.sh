#!/bin/sh
# run-all.sh <scratch> <runs-dir> -- every BIOS case table on NewTOS (QEMU raspi4b) through
# `cargo xtask tosdump`, each against every pinned reference it has. Run from the repository root.
S=$1; R=$2
REF=tosdump/reference/EMUTOS-1a7484ab
mkdir -p "$S" "$R"
run() { # name table refs...
  n=$1; t=$2; shift 2; set -- "$n" "$t" "$@"
  n=$1; t=$2; shift 2
  args=""; for r in "$@"; do args="$args --reference $REF-$r.BIN"; done
  cargo xtask tosdump bios tosdump/cases/$t.tsv --out "$R/$n" --scratch "$S" $args > "$R/$n.stdout" 2>&1
  echo "$n exit $?"
}
run bios-core-r1 bios-core-r1 BIOS-core BIOS-core-falcontc
run bios-r1 bios-r1 BIOS BIOS-falcontc
for T in r2 r2-phys r2-x001 r2-x004 r2-x005 r2-x006; do
  extra=""
  case $T in r2) ref=BIOS-r2-disk;; *) ref=BIOS-$T-disk;; esac
  dflag=--disk
  n=bios-$T
  cargo xtask tosdump bios tosdump/cases/bios-$T.tsv --disk --out "$R/$n" --scratch "$S" --reference $REF-$ref.BIN > "$R/$n.stdout" 2>&1
  echo "$n exit $?"
  [ -f "$R/$n/testpart-after.img" ] && mv "$R/$n/testpart-after.img" "$S/$n-testpart-after.img"
done
