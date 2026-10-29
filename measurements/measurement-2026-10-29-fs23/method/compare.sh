#!/bin/sh
# compare.sh <runs-dir> <scratch> -- FS23 (after measurement-2026-10-29-bios04/method/compare.sh):
# per table and reference the differing case numbers before (HEAD) and after (FS23), the cases
# that differ after but not before, and those that differed before but not after; read from
# the "case N" headers of the runner's tosdiff-<ref>.txt files. Then the NewTOS test
# partitions after the destructive tables, before vs after (sha256; FS23 writes no disk).
set -u
R=$1; S=$2
cases() { /usr/bin/grep -E '^case [0-9]+ ' "$1" 2>/dev/null | awk '{print $2}' | sort -n | tr '\n' ' '; }
only_in() { # only_in "<a list>" "<b list>": the words of a not in b
    for c in $1; do
        case " $2 " in
            *" $c "*) ;;
            *) printf '%s ' "$c" ;;
        esac
    done
}
for t in fs20 fs22 paths bios-r1; do
    for f in "$R/newtos-$t"/tosdiff-EMUTOS-*.txt; do
        ref=$(basename "$f" .txt); ref=${ref#tosdiff-}
        b=$(cases "$R/before-$t/tosdiff-$ref.txt"); a=$(cases "$f")
        echo "$t vs $ref"
        echo "  before (HEAD): $(echo $b | wc -w | tr -d ' ') differ: $b"
        echo "  after  (FS23): $(echo $a | wc -w | tr -d ' ') differ: $a"
        echo "  newly differing: $(only_in "$a" "$b")"
        echo "  now equal:       $(only_in "$b" "$a")"
    done
done
echo "test partition after the run, NewTOS before vs after (sha256)"
for t in fs20 fs22 paths; do
    b=$(shasum -a 256 "$S/end/before-$t.img" | awk '{print $1}')
    a=$(shasum -a 256 "$S/end/newtos-$t.img" | awk '{print $1}')
    if [ "$a" = "$b" ]; then v=identical; else v=DIFFERENT; fi
    echo "  $t  before $b  after $a  $v"
done
