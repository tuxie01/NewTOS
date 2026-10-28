#!/bin/sh
# TOOL04: does the entry count of target/debug/deps alone make the relink
# slow? A new target dir is built cold (`cargo test --no-run` over the host
# packages), then relinked after a touch of tos-api/src/lib.rs (baseline),
# then N empty files named like rustc's object files are added to its deps/
# and the relink is repeated. Nothing else differs between the two relinks.
# Usage: deps-bloat.sh <repo-root> <outfile> <target-parent> <N>
R=$1; OUT=$2; SCR=$3; N=$4
M=$(cd "$(dirname "$0")" && pwd)
T=$SCR/bloat
rm -rf "$T"
b() { (cd "$R" && CARGO_TARGET_DIR="$T" cargo test --no-run -p tos-api -p tos-types -p xtask > /dev/null 2>&1); }
t() { label=$1; shift; touch "$R/tos-api/src/lib.rs"
  t0=$(python3 -I -c 'import time; print(time.time())'); b
  printf '%s\twall=%s\tdeps_entries=%s\n' "$label" \
    "$(python3 -I -c "import time; print('%.1f' % (time.time() - $t0))")" \
    "$(ls -f "$T/debug/deps" | wc -l | tr -d ' ')" | tee -a "$OUT"; }
t cold-build
t relink-small-deps
python3 -I -c "
import os, sys
d, n = sys.argv[1], int(sys.argv[2])
for i in range(n):
    open(os.path.join(d, 'zzbloat-%016x.%025d.rcgu.o' % (i, i)), 'w').close()
" "$T/debug/deps" "$N"
t "relink-after-adding-$N-empty-files"
t "relink-again-$N"
rm -rf "$T"
