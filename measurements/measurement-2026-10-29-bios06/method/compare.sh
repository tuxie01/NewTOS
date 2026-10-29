#!/bin/sh
# compare.sh <runs-dir> -- per table and reference: the differing case numbers before
# (90f409ab) and after (BIOS06), and the cases that differ after but not before.
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
for t in xbios-ptr-r1 bios-r1 xbios-chardev-r1 bios-core-r1 xbios-system-r1; do
    for f in "$R/newtos-$t"/tosdiff-EMUTOS-*.txt; do
        ref=$(basename "$f" .txt); ref=${ref#tosdiff-}
        b=$(cases "$R/before-$t/tosdiff-$ref.txt"); a=$(cases "$f")
        new=$(only_in "$a" "$b")
        echo "$t vs $ref"
        echo "  before (90f409ab): $b"
        echo "  after  (BIOS06):   $a"
        echo "  newly differing:   $new"
    done
done
