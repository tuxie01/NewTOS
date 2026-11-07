#!/bin/bash
# make-bprof-tree.sh <tree> : the BEFORE test kernel for the Pi run -- base 3a9221eb (scalar fills) with the
# raster-prof commit 8403f75b laid on top, so before and after carry the same profiler. Not product code;
# exists only in scratch. raster-prof-on-base.patch = `git diff 87b2a9fd 8403f75b -- kernel`; its two hunks
# for fill_rect/fill_pattern do not apply to the scalar code (their context is the NEON call), so the same
# one-line scope is put at the same place by hand below: after the clip, before the pixel writes.
set -eu
T=$1; H=$(cd "$(dirname "$0")" && pwd)
patch -d "$T" -p1 --no-backup-if-mismatch -r - < "$H/raster-prof-on-base.patch" || true
python3 -I - "$T/kernel/src/vdi_backend.rs" <<'EOF'
import sys
p = sys.argv[1]; s = open(p).read()
C = '#[cfg(feature = "raster-prof")] let _prof = crate::raster_prof::'
def ins(anchor, line):
    global s
    assert s.count(anchor) == 1, anchor
    s = s.replace(anchor, line + "\n" + anchor)
ins("        fb.fill_rect(x0 as u32, y0 as u32, (x1 - x0 + 1) as u32,", "        " + C + "fill_rect(x0, y0, x1, y1);")
ins("        let pmask = fill.pattern.len() - 1; // pattern len is a power of two", "        " + C + "fill_pattern(x0, y0, x1, y1);")
open(p, "w").write(s)
EOF
grep -c 'cfg(feature = "raster-prof")' "$T/kernel/src/vdi_backend.rs"
