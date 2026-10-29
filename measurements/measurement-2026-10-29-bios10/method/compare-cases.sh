#!/bin/bash
# compare-cases.sh <runs-dir> -- per table and reference: the differing case numbers before
# and after (the "case N" headers of each tosdiff-*.txt), the cases newly differing after,
# and the cases no longer differing. Display only.
R=$1
cases() { grep -E '^case [0-9]+' "$1" | awk '{print $2}' | sort -n | tr '\n' ' '; }
for t in bios-r1 bios-core-r1 xbios-chardev-r1 xbios-system-r1; do
  for f in "$R/before-$t"/tosdiff-EMUTOS-*.txt; do
    n=$(basename "$f"); a="$R/after-$t/$n"
    b_l=$(cases "$f"); a_l=$(cases "$a")
    new=$(comm -13 <(echo "$b_l" | tr ' ' '\n' | grep . | sort) <(echo "$a_l" | tr ' ' '\n' | grep . | sort) | sort -n | tr '\n' ' ')
    gone=$(comm -23 <(echo "$b_l" | tr ' ' '\n' | grep . | sort) <(echo "$a_l" | tr ' ' '\n' | grep . | sort) | sort -n | tr '\n' ' ')
    echo "$t $n: before $(echo $b_l | wc -w | tr -d ' ') -> after $(echo $a_l | wc -w | tr -d ' ')  newly differing: [${new% }]  now equal: [${gone% }]"
    echo "   before: $b_l"
  done
done
