#!/bin/sh
# newtos-runs.sh <scratch> <runs-dir> <prefix> [<tree>] -- every BIOS table
# BIOS01 (rest) is checked against, on NewTOS (QEMU raspi4b), through the
# runner of TOOL09, one run each:
#   r2 x001 x004 x005 x006 phys  tosdump/cases/bios-r2[-<t>].tsv --disk
#                                against EMUTOS-1a7484ab-BIOS-r2[-<t>]-disk.BIN
#   r1                           tosdump/cases/bios-r1.tsv against the tt and
#                                falcontc references
#   fs20 fs22 paths              tosdump/cases/gemdos-mc-<t>.tsv --testdisk
#                                <scratch>/mc/test.img (TOOL20's image, built
#                                by measurements/measurement-2026-10-29-tool20/
#                                method/mkimage.sh) against
#                                EMUTOS-1a7484ab-BIOS-mc-<t>-disk.BIN
# <tree> (default: this tree) is passed as --tree: the measured tree.
# ONLY=<t> restricts the run to one table. A destructive table's
# <out>/testpart-after.img is moved to <scratch>/end/<prefix>-<t>.img.
set -u
S=$1; R=$2; P=$3; TREE=${4:-}
HERE=$(cd "$(dirname "$0")" && pwd); REPO=$(cd "$HERE/../../.." && pwd)
cd "$REPO"
mkdir -p "$S/end" "$S/xtask"
TREEARG=""; [ -n "$TREE" ] && TREEARG="--tree $TREE"
one() { # <t> <tsv> <disk-args> <ref>...
    T=$1; TSV=$2; DISK=$3; shift 3
    REFS=""; for X in "$@"; do REFS="$REFS --reference tosdump/reference/$X"; done
    # shellcheck disable=SC2086
    cargo xtask tosdump bios "tosdump/cases/$TSV" $DISK $TREEARG $REFS \
        --out "$R/$P-$T" --scratch "$S/xtask" > "$S/$P-$T.stdout" 2>&1
    echo "$P-$T: cargo xtask tosdump exit $?"
    [ -f "$R/$P-$T/testpart-after.img" ] && mv "$R/$P-$T/testpart-after.img" "$S/end/$P-$T.img"
}
for T in r2 x001 x004 x005 x006 phys r1 fs20 fs22 paths; do
    [ -n "${ONLY:-}" ] && [ "$ONLY" != "$T" ] && continue
    case $T in
        r2)   one r2 bios-r2.tsv --disk EMUTOS-1a7484ab-BIOS-r2-disk.BIN ;;
        r1)   one r1 bios-r1.tsv "" EMUTOS-1a7484ab-BIOS.BIN EMUTOS-1a7484ab-BIOS-falcontc.BIN ;;
        fs20|fs22|paths)
              [ -f "$S/mc/test.img" ] || \
                  measurements/measurement-2026-10-29-tool20/method/mkimage.sh "$S/mc" > "$S/mc-mkimage.stdout" 2>&1
              one "$T" "gemdos-mc-$T.tsv" "--testdisk $S/mc/test.img" "EMUTOS-1a7484ab-BIOS-mc-$T-disk.BIN" ;;
        *)    one "$T" "bios-r2-$T.tsv" --disk "EMUTOS-1a7484ab-BIOS-r2-$T-disk.BIN" ;;
    esac
done
