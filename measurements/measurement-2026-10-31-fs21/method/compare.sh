#!/bin/sh
# compare.sh <runs-dir> <scratch> -- TASK/FS21 (after measurement-2026-10-30-fs28/method/compare.sh):
# per table and reference the differing case numbers before (HEAD) and after (FS21), the cases
# that differ after but not before, and those that differed before but not after, read from
# the "case N" headers of the runner's tosdiff-<ref>.txt files. Then the end states of the
# four gemdos-mc tables (method/compare-ends.py).
set -u
R=$1; S=$2
HERE=$(cd "$(dirname "$0")" && pwd)
cases() { /usr/bin/grep -E '^case [0-9]+ ' "$1" 2>/dev/null | awk '{print $2}' | sort -n | tr '\n' ' '; }
only_in() {
    for c in $1; do
        case " $2 " in
            *" $c "*) ;;
            *) printf '%s ' "$c" ;;
        esac
    done
}
for t in fs20 fs22 paths fs24 bios-r1; do
    for f in "$R/newtos-$t"/tosdiff-EMUTOS-*.txt; do
        ref=$(basename "$f" .txt); ref=${ref#tosdiff-}
        b=$(cases "$R/before-$t/tosdiff-$ref.txt"); a=$(cases "$f")
        echo "$t vs $ref"
        echo "  before (HEAD): $(echo $b | wc -w | tr -d ' ') differ: $b"
        echo "  after  (FS21): $(echo $a | wc -w | tr -d ' ') differ: $a"
        echo "  newly differing: $(only_in "$a" "$b")"
        echo "  now equal:       $(only_in "$b" "$a")"
    done
done
python3 "$HERE/compare-ends.py" "$S" "$R"
