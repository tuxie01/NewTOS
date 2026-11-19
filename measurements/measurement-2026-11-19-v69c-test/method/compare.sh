#!/usr/bin/env bash
# compare.sh -- pixel compares of the screendumps (pngcmp.py exact, pngtol.py at 5 bits per gun)
cd "$(dirname "$0")/../screendumps"
cmp1() { for n in arm s1 s2 s3 s4 end; do printf '%s vs %s %-4s exact: %s' "$1" "$2" $n "$(python3 ../method/pngcmp.py $1-$n.png $2-$n.png | cut -c1-70)"; [ -n "$3" ] && printf '  | 5bit: %s' "$(python3 ../method/pngtol.py $1-$n.png $2-$n.png | head -1 | cut -c1-80)"; echo; done; }
cmp1 wt-v16 wt-main-v16
cmp1 wt-v16 wt-v16b
cmp1 wt-v16 wt-v16x
cmp1 wt-v32 wt-v16 t
cmp1 wt-v32b wt-v16 t
cmp1 wt-v32 wt-v32b
