#!/usr/bin/env python3
"""V51 red evidence: run tos-api/tests/raster_kernel_copy.rs against deliberately wrong kernels.

red-mutants.py <worktree> <out-file> <scratch-dir>

Adapted from measurements/measurement-2026-11-07-v50/method/red-mutants.py (V50 branch). Each mutant is
one textual edit of kernel/src/raster_copy.rs (the edit names the text it replaces and how often it
occurs; every occurrence is replaced). The file is copied aside first, the edit applied,
`cargo test -p tos-api --test raster_kernel_copy` run, and the original copied back byte for byte
(sha256 checked) before the next mutant. No version-control command touches the tree. A mutant is red
if the test run fails (assertion, or a crash: every store sits between two inaccessible pages, so a
read or write past the screen rectangle kills the test process).
"""
import hashlib, os, shutil, subprocess, sys

root, out, scratch = sys.argv[1], sys.argv[2], sys.argv[3]
src = os.path.join(root, "kernel/src/raster_copy.rs")
keep = os.path.join(scratch, "raster_copy.rs.orig")

# (name, old, new, occurrences)
MUTANTS = [
    ("M1 the one-pixel tail is dropped (every kernel)",
     '"tbz {n}, #0, 8f\\n",',
     '"b 8f\\n",', 1),
    ("M2 the eight-pixel tail is dropped (every kernel)",
     '"tbz {n}, #3, 5f\\n",',
     '"b 5f\\n",', 1),
    ("M3 op constant c0 ^ c1 taken as c0",
     "let (k01, k23) = (bit(0) ^ c1, bit(2) ^ c3);",
     "let (k01, k23) = (bit(0), bit(2) ^ c3);", 1),
    ("M4 bsl picks Y where S is set and X where it is clear",
     '"bsl ", $s, ".16b, ", $x, ".16b, ", $d, ".16b\\n",',
     '"bsl ", $s, ".16b, ", $d, ".16b, ", $x, ".16b\\n",', 1),
    ("M5 adapter: columns always left to right",
     "let (backward, bottom_up) = (dx > src_x, dy > src_y);",
     "let (backward, bottom_up) = (false, dy > src_y);", 1),
    ("M6 adapter: rows always top-down",
     "let (backward, bottom_up) = (dx > src_x, dy > src_y);",
     "let (backward, bottom_up) = (dx > src_x, false);", 1),
    ("M7 adapter: source column clip lets one column past the right screen edge through",
     "let (x0, x1) = (tx0.max(ox), tx1.min(ox + scr_w - 1));",
     "let (x0, x1) = (tx0.max(ox), tx1.min(ox + scr_w));", 1),
    ("M8 right-to-left rows: destination cursor starts one pixel short",
     'concat!("add {p}, {src}, {w}, lsl #2\\n", "add {q}, {dst}, {w}, lsl #2\\n")',
     'concat!("add {p}, {src}, {w}, lsl #2\\n", "add {q}, {dst}, {w}, lsl #2\\n", "sub {q}, {q}, #4\\n")', 1),
    ("M9 the pure copy also taken for op 12 (NOT S)",
     "if op == 3 {",
     "if op == 3 || op == 12 {", 1),
    ("M10 op kernel: v12/v13 not saved (restore kept)",
     '"stp q12, q13, [{save}, #192]",\n',
     '', 1),
    ("M11 bottom-up walk steps down instead of up",
     "let step = if bottom_up { pitch.wrapping_neg() } else { pitch };",
     "let step = pitch;", 1),
    ("M12 left-to-right op kernel loads the destination from the source cursor",
     '"ld1 ", $d, ", [{q}]\\n",\n            rop!($r),\n            "st1 ", $s, ", [{q}], #", $b, "\\n",',
     '"ld1 ", $d, ", [{p}]\\n",\n            rop!($r),\n            "st1 ", $s, ", [{q}], #", $b, "\\n",', 1),
    ("M13 the 16-pixel loop needs 32 pixels to start",
     '"subs {n}, {w}, #16\\n",',
     '"subs {n}, {w}, #32\\n",', 1),
    ("M14 adapter: source row clip lets the row above the screen through",
     "let (y0, y1) = (ty0.max(oy), ty1.min(oy + scr_h - 1));",
     "let (y0, y1) = (ty0.max(oy - 1), ty1.min(oy + scr_h - 1));", 1),
]

def sha(p):
    return hashlib.sha256(open(p, "rb").read()).hexdigest()

orig_sha = sha(src)
shutil.copyfile(src, keep)
lines = [f"original kernel/src/raster_copy.rs sha256 {orig_sha}"]
try:
    for name, old, new, count in MUTANTS:
        text = open(keep).read()
        assert text.count(old) == count, f"{name}: pattern found {text.count(old)}x, expected {count}"
        open(src, "w").write(text.replace(old, new))
        r = subprocess.run(["cargo", "test", "-p", "tos-api", "--test", "raster_kernel_copy"],
                           cwd=root, capture_output=True, text=True)
        shutil.copyfile(keep, src)
        assert sha(src) == orig_sha
        res = [l for l in r.stdout.splitlines()
               if (l.startswith("test ") and not l.endswith(" ok")) or l.startswith("test result")]
        both = (r.stdout + "\n" + r.stderr).splitlines()
        first = next((l for l in both if "signal:" in l), "") or next(
            (l for l in both if "kernel != reference" in l or "assertion" in l
             or "V registers named" in l or "panicked at" in l or l.startswith("error")), "")
        lines.append(f"\n== {name}\n   exit {r.returncode} -> {'RED' if r.returncode != 0 else 'GREEN (mutant survived)'}")
        lines += ["   " + l for l in res]
        if first:
            lines.append("   first message: " + first.strip()[:400])
finally:
    shutil.copyfile(keep, src)
lines.append(f"\nrestored kernel/src/raster_copy.rs sha256 {sha(src)} (== original: {sha(src) == orig_sha})")
open(out, "w").write("\n".join(lines) + "\n")
print("\n".join(lines))
