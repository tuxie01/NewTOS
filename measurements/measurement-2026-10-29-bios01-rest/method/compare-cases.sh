#!/bin/sh
# compare-cases.sh <runs-dir> -- per table and reference: the differing case
# numbers of runs/before-<t> and runs/after-<t> (tosdiff-*.txt, "case N"
# lines), the cases that became EQUAL and the cases that NEWLY differ
# (comm -13), and the end state of the test partition (testpart after sha256
# from tosdump-run.txt).
set -u
R=$1
for B in "$R"/before-*; do
    T=${B##*/before-}; A="$R/after-$T"
    for F in "$B"/tosdiff-EMUTOS-*.txt; do
        N=${F##*/}
        [ -f "$A/$N" ] || { echo "$T $N: no after run"; continue; }
        grep -o '^case [0-9]*' "$F" | awk '{print $2}' | sort > "$R/.b"
        grep -o '^case [0-9]*' "$A/$N" | awk '{print $2}' | sort > "$R/.a"
        echo "$T ${N#tosdiff-}: before $(wc -l < "$R/.b" | tr -d ' ') [$(sort -n "$R/.b" | tr '\n' ' ')]" \
             "after $(wc -l < "$R/.a" | tr -d ' ') [$(sort -n "$R/.a" | tr '\n' ' ')]" \
             "now-equal [$(comm -23 "$R/.b" "$R/.a" | sort -n | tr '\n' ' ')]" \
             "NEWLY-differing [$(comm -13 "$R/.b" "$R/.a" | sort -n | tr '\n' ' ')]"
    done
    EB=$(grep 'testpart after sha256' "$B/tosdump-run.txt" | awk '{print $4}')
    EA=$(grep 'testpart after sha256' "$A/tosdump-run.txt" 2>/dev/null | awk '{print $4}')
    TB=$(grep '^testpart:' "$B/tosdump-run.txt" | head -1)
    TA=$(grep '^testpart:' "$A/tosdump-run.txt" 2>/dev/null | head -1)
    echo "$T end state: before ${EB:-(kept none)} / $TB ; after ${EA:-(kept none)} / $TA"
done
rm -f "$R/.a" "$R/.b"
