#!/bin/sh
# before-after.sh <runs-dir> -- per table and reference: the differing case numbers
# before (18e8c115) and after (XBIOS15), and the cases that newly differ.
R=$1
cases() { # the case numbers tosdiff lists as differing, one line
    grep -E '^case +[0-9]+' "$1" 2>/dev/null | awk '{print $2}' | sort -n | uniq | tr '\n' ' '
}
for t in bios-r1 xbios-chardev-r1 xbios-ptr-r1 xbios-flush-r1; do
    for f in "$R/after-$t"/tosdiff-EMUTOS-*.txt; do
        ref=$(basename "$f" .txt | sed 's/^tosdiff-//')
        b=$(cases "$R/before-$t/tosdiff-$ref.txt"); a=$(cases "$f")
        new=""; for c in $a; do case " $b " in *" $c "*) ;; *) new="$new $c" ;; esac; done
        echo "$t vs $ref"
        echo "  before (18e8c115): $b ($(tail -1 "$R/before-$t/tosdiff-$ref.txt"))"
        echo "  after  (XBIOS15):  $a ($(tail -1 "$f"))"
        echo "  newly differing:  $new"
    done
done
