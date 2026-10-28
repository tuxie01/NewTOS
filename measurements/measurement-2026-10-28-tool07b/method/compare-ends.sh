#!/bin/bash
# compare-ends.sh <scratch> <runs-dir> -- the end-state rule of the closing
# tables (TASK/TOOL07, user decision 2026-10-28): after a destructive table
# both sides must end in the same state, the broken boot sector included.
# Method of measurements/measurement-2026-10-28-tool07/method/compare-testpart.sh,
# on the TEST PARTITION only (LBA 63, 130977 sectors):
#   oracle-<t>-1 vs oracle-<t>-2   the oracle reproduces its end state
#   oracle-<t>-1 vs newtos-<t>     both sides end in the same state
#   original vs oracle / newtos    which sectors the table changed
# per table: sha256, the differing 512-byte sectors (partition-relative) and
# their differing byte offsets, fsck_msdos -n on each end state.
# Input: <scratch>/end/{original,oracle-<t>-<i>,newtos-<t>}.img; output
# <runs-dir>/endstate-<t>.txt.  The images are deleted afterwards.
set -u
S=$1; R=$2; E=$S/end
cmpimg() {   # cmpimg <label> <a> <b>
    local sa sb
    sa=$(shasum -a 256 "$2" | cut -d' ' -f1); sb=$(shasum -a 256 "$3" | cut -d' ' -f1)
    if [ "$sa" = "$sb" ]; then
        echo "$1: IDENTICAL ($sa)"
    else
        echo "$1: DIFFERENT ($sa vs $sb); differing bytes (partition offset: a b, octal):"
        cmp -l "$2" "$3" | head -40 | awk '{ printf "  sector %d offset 0x%03X: %s %s\n", int(($1-1)/512), ($1-1)%512, $2, $3 }'
    fi
}
fsck_of() {
    echo "fsck_msdos -n $(basename "$1"):"
    fsck_msdos -n "$1" 2>&1 | sed 's/^/  /'; echo "  exit ${PIPESTATUS[0]}"
}
for T in x004 x005 x006; do
    {
        echo "end state of bios-r2-$T.tsv (test partition, LBA 63 + 130977)"
        cmpimg "original vs oracle-1" "$E/original.img" "$E/oracle-$T-1.img"
        cmpimg "oracle-1 vs oracle-2" "$E/oracle-$T-1.img" "$E/oracle-$T-2.img"
        if [ -f "$E/newtos-$T.img" ]; then
            cmpimg "original vs newtos" "$E/original.img" "$E/newtos-$T.img"
            cmpimg "oracle-1 vs newtos (THE RULE)" "$E/oracle-$T-1.img" "$E/newtos-$T.img"
            fsck_of "$E/newtos-$T.img"
        else
            echo "newtos: NO END STATE (no testpart-after.img)"
        fi
        fsck_of "$E/oracle-$T-1.img"
    } > "$R/endstate-$T.txt" 2>&1
    cat "$R/endstate-$T.txt" | grep -E "IDENTICAL|DIFFERENT|NO END"
done
