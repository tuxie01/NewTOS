#!/bin/sh
# diffcases.sh <runs-dir> -- one line per tosdiff file of every run under <runs-dir>:
# run, reference, "N of M cases differ", the differing case numbers. Display only.
for f in "$1"/*/tosdiff-EMUTOS-*.txt; do
    run=$(basename "$(dirname "$f")"); ref=$(basename "$f" .txt | sed 's/^tosdiff-//')
    sum=$(grep -E 'cases? differ|no case differs|0 of' "$f" | tail -1)
    cases=$(grep -E '^case [0-9]+ ' "$f" | awk '{print $2}' | tr '\n' ' ')
    printf '%-26s %-40s %s: %s\n' "$run" "$ref" "$sum" "$cases"
done
