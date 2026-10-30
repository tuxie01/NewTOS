#!/bin/sh
# compare.sh <runs-dir> -- (copied from measurement-2026-10-30-xbios04/method/compare.sh;
# changes: commit names a8d875a7 / COOK02, the table list) per table and
# reference: the differing case numbers before (a8d875a7) and after (COOK02), the cases that
# differ after but not before, and the cases that differed before but not after.
# Reads the "case N" headers of the runner's tosdiff-<ref>.txt files.
set -u
R=$1
cases() { /usr/bin/grep -E '^case [0-9]+ ' "$1" 2>/dev/null | awk '{print $2}' | sort -n | tr '\n' ' '; }
only_in() { # only_in "<a list>" "<b list>": the words of a not in b
    for c in $1; do
        case " $2 " in
            *" $c "*) ;;
            *) printf '%s ' "$c" ;;
        esac
    done
}
for t in jar-r1 bios-r1 xbios-system-r1; do
    for f in "$R/after-$t"/tosdiff-EMUTOS-*.txt; do
        ref=$(basename "$f" .txt); ref=${ref#tosdiff-}
        b=$(cases "$R/before-$t/tosdiff-$ref.txt"); a=$(cases "$f")
        new=$(only_in "$a" "$b")
        echo "$t vs $ref"
        echo "  before (a8d875a7): $b"
        echo "  after  (COOK02):  $a"
        echo "  newly differing:   $new"
        echo "  now equal:         $(only_in "$b" "$a")"
    done
done
