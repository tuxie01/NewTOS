#!/bin/sh
# compare.sh <runs-dir> -- (copied from measurement-2026-10-29-bios04/method/compare.sh;
# changes: run-dir names before-/after-, commit names, the five BIOS08 tables) per table and
# reference: the differing case numbers before (60824ec8) and after (BIOS08), the cases that
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
for t in bios-r1 bios-core-r1 xbios-chardev-r1 xbios-system-r1 xbios-bconmap-r1; do
    for f in "$R/after-$t"/tosdiff-EMUTOS-*.txt; do
        ref=$(basename "$f" .txt); ref=${ref#tosdiff-}
        b=$(cases "$R/before-$t/tosdiff-$ref.txt"); a=$(cases "$f")
        new=$(only_in "$a" "$b")
        echo "$t vs $ref"
        echo "  before (60824ec8): $b"
        echo "  after  (BIOS08):   $a"
        echo "  newly differing:   $new"
        echo "  now equal:         $(only_in "$b" "$a")"
    done
done
