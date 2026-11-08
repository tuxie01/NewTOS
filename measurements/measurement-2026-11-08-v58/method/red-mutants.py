#!/usr/bin/env python3
"""V58 red evidence: the line path's host tests against deliberately wrong line code.

red-mutants.py <worktree> <out-file> <scratch-dir>

Each mutant is one textual edit of one file -- kernel/src/raster_line.rs (the store side),
kernel/src/raster_kernel.rs (fill_span) or tos-api/src/vdi/line.rs (abline_runs, the run form of the
shared walk); the edit names the text it replaces and how often it occurs (every occurrence is
replaced). The file is copied aside first, the edit applied, then
`cargo test -p tos-api --test raster_kernel_line` (the store path against the per-pixel reference,
whole store compared, stores between two inaccessible pages) and, for a line.rs mutant, also
`cargo test -p tos-api --lib vdi::line` (the run form against the walk) run, and the original copied
back byte for byte (sha256 checked) before the next mutant. No version-control command touches the
tree. A mutant is red if a run fails (assertion, or a crash at a guard page).
"""
import hashlib, os, shutil, subprocess, sys

root, out, scratch = sys.argv[1], sys.argv[2], sys.argv[3]
LINE = "kernel/src/raster_line.rs"
KERN = "kernel/src/raster_kernel.rs"
WALK = "tos-api/src/vdi/line.rs"

# (name, file, old, new, occurrences)
MUTANTS = [
    ("M1 span: phase one pixel late", WALK,
     "let phase = ((ax as i64 + (lo - first)) & 15) as u32;",
     "let phase = ((ax as i64 + (lo - first) + 1) & 15) as u32;", 1),
    ("M2 span: cut one pixel past the end of the store", WALK,
     "(ay as i64 * s + bx as i64).min(len - 1)",
     "(ay as i64 * s + bx as i64).min(len)", 1),
    ("M3 span: run-on rectangle keeps the first row's column", WALK,
     "(0, r0 as u32, (s - 1) as u32, r1 as u32)",
     "((lo % s) as u32, r0 as u32, (s - 1) as u32, r1 as u32)", 1),
    ("M4 column: dash phase of a cut column restarts at 0", WALK,
     "step as isize, (klo & 15) as u32, mask);",
     "step as isize, 0, mask);", 1),
    ("M5 column: one pixel short", WALK,
     "let count = if dy >= 0 { dy as i64 + 1 } else { 0 };",
     "let count = if dy >= 0 { dy as i64 } else { 0 };", 1),
    ("M6 column: upward cut at the store end off by one", WALK,
     "(if first < len { 0 } else { ceil_div(first - (len - 1)) }",
     "(if first < len { 0 } else { ceil_div(first - len) }", 1),
    ("M7 XOR rule left out of the run form", WALK,
     "    // `abline:2299-2325`: from the left endpoint, the XOR rule.\n"
     "    let (ax, ay, mut bx, by) = if x1 < x0 { (x1, y1, x0, y0) } else { (x0, y0, x1, y1) };\n"
     "    if mode == WriteMode::Xor && !last {",
     "    // `abline:2299-2325`: from the left endpoint, the XOR rule.\n"
     "    let (ax, ay, mut bx, by) = if x1 < x0 { (x1, y1, x0, y0) } else { (x0, y0, x1, y1) };\n"
     "    if mode == WriteMode::Xor && !last && false {", 1),
    ("M8 diagonal fast path taken with only the first end on the screen", WALK,
     "&& on(x1, y1) && on(x2, y2) {",
     "&& on(x1, y1) {", 1),
    ("M9 diagonal fast path: minor y step always downwards", WALK,
     "if xmajor { (1isize, (yinc * s) as isize) }",
     "if xmajor { (1isize, s as isize) }", 1),
    ("M10 diagonal fast path: rectangle ends on the far y, not the larger", WALK,
     "x2 as u32, y1.max(y2) as u32)",
     "x2 as u32, y2 as u32)", 1),
    ("M11 diagonal fast path: dash rotated the wrong way", WALK,
     "m = m.rotate_left(1);",
     "m = m.rotate_right(1);", 1),
    ("M12 diagonal general path: column wraps one late", WALK,
     "if *col == s {",
     "if *col > s {", 1),
    ("M13 diagonal general path: one row past the store let through", WALK,
     "if (0..rows).contains(&row) {",
     "if (0..=rows).contains(&row) {", 1),
    ("M14 store column: dash bit rotated the wrong way", LINE,
     "bit = bit.rotate_right(1);",
     "bit = bit.rotate_left(1);", 1),
    ("M15 store span: always Replace", LINE,
     "fill_span(self.base.add(idx), n, phase, mask, M::MODE, self.fg, self.bg)",
     "fill_span(self.base.add(idx), n, phase, mask, WriteMode::Replace, self.fg, self.bg)", 1),
    ("M16 store Replace: fg and bg swapped", LINE,
     "unsafe { p.write_volatile(if set { fg } else { bg }) };",
     "unsafe { p.write_volatile(if set { bg } else { fg }) };", 1),
    ("M17 store Xor: low bit kept", LINE,
     "unsafe { p.write_volatile(!p.read_volatile()) };",
     "unsafe { p.write_volatile(!p.read_volatile() ^ 1) };", 1),
    ("M18 store RevTransparent: fg on the set bits", LINE,
     "        if !set {\n",
     "        if set {\n", 1),
    ("M19 fill_span: phase ignored", KERN,
     "pmask: 0, prow: 0, phase: phase as usize & 15 };",
     "pmask: 0, prow: 0, phase: 0 };", 1),
    ("M20 fill_span: one pixel short", KERN,
     "let rows = Rows { stride: 0, w: n, h: 1,",
     "let rows = Rows { stride: 0, w: n - 1, h: 1,", 1),
]

def sha(p):
    return hashlib.sha256(open(p, "rb").read()).hexdigest()

def run(args):
    r = subprocess.run(["cargo", "test", "-p", "tos-api"] + args, cwd=root, capture_output=True, text=True)
    tail = [l for l in (r.stdout + r.stderr).splitlines() if l.startswith("test result") or "panicked at" in l
            or "signal:" in l or "SIGSEGV" in l or "SIGBUS" in l or l.startswith("error")]
    return r.returncode, tail

os.makedirs(scratch, exist_ok=True)
files = sorted({m[1] for m in MUTANTS})
orig = {}
for f in files:
    orig[f] = sha(os.path.join(root, f))
    shutil.copyfile(os.path.join(root, f), os.path.join(scratch, os.path.basename(f) + ".orig"))
lines = [f"original {f} sha256 {orig[f]}" for f in files]
red = 0
try:
    for name, f, old, new, count in MUTANTS:
        src = os.path.join(root, f)
        keep = os.path.join(scratch, os.path.basename(f) + ".orig")
        text = open(keep).read()
        assert text.count(old) == count, f"{name}: pattern found {text.count(old)}x, expected {count}"
        open(src, "w").write(text.replace(old, new))
        rc, tail = run(["--test", "raster_kernel_line"])
        rcs = [("raster_kernel_line", rc, tail)]
        if f == WALK:
            rc2, tail2 = run(["--lib", "vdi::line"])
            rcs.append(("lib vdi::line", rc2, tail2))
        shutil.copyfile(keep, src)
        assert sha(src) == orig[f]
        is_red = any(r != 0 for _, r, _ in rcs)
        red += is_red
        lines.append(f"{'RED  ' if is_red else 'GREEN'} {name} ({f})")
        for t, r, tl in rcs:
            lines.append(f"      {t}: exit {r}")
            for l in tl[:4]:
                lines.append(f"        {l.strip()[:200]}")
finally:
    for f in files:
        shutil.copyfile(os.path.join(scratch, os.path.basename(f) + ".orig"), os.path.join(root, f))
        assert sha(os.path.join(root, f)) == orig[f]
rc, tail = run(["--test", "raster_kernel_line"])
rc2, tail2 = run(["--lib", "vdi::line"])
lines.append(f"after restore: raster_kernel_line exit {rc} {tail[-1:] if tail else ''}; lib vdi::line exit {rc2} {tail2[-1:] if tail2 else ''}")
lines.append(f"{red} of {len(MUTANTS)} mutants red")
for f in files:
    lines.append(f"restored {f} sha256 {sha(os.path.join(root, f))}")
open(out, "w").write("\n".join(lines) + "\n")
print("\n".join(lines))
