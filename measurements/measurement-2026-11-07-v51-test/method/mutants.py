#!/usr/bin/env python3
"""V51 tester mutants: one textual edit of kernel/src/raster_copy.rs in a SCRATCH archive tree of 1197fed7 each,
host test tos-api/tests/raster_kernel_copy.rs, file restored from a copy (sha256 checked). The worktree is not touched.
usage: mutants.py <scratch tree> <out>"""
import hashlib, os, shutil, subprocess, sys
root, out = sys.argv[1:3]
src = os.path.join(root, "kernel/src/raster_copy.rs"); keep = src + ".keep"
shutil.copy(src, keep)
sha = lambda p: hashlib.sha256(open(p, 'rb').read()).hexdigest()
h0 = sha(src)
MUT = [
 ("Q1 same-row right shift runs forward (backward only when dy != src_y)",
  "let (backward, bottom_up) = (dx > src_x, dy > src_y);", "let (backward, bottom_up) = (dx > src_x && dy != src_y, dy > src_y);"),
 ("Q2 op 10 (NOT D): k01 constant wrong for op 10 only",
  "let (k01, k23) = (bit(0) ^ c1, bit(2) ^ c3);", "let (k01, k23) = (bit(0) ^ c1 ^ if op == 10 { 1 } else { 0 }, bit(2) ^ c3);"),
 ("Q3 op 7 (S OR D): bit 3 (f(0,0)) read as set",
  "let (c1, c3) = (bit(1), bit(3));", "let (c1, c3) = (bit(1), if op == 7 { u32::MAX } else { bit(3) });"),
 ("Q4 rop kernel: restore of q10,q11 dropped (both rop kernels)",
  '            "ldp q10, q11, [{save}, #160]",\n', ''),
 ("Q5 copy kernel: restore of q2,q3 dropped (both copy kernels)",
  '            "ldp q2, q3, [{save}, #32]",\n            simd_off!(),\n            src = inout(reg) $src => _,\n            dst = inout(reg) $dst => _,\n            step = in(reg) $step,\n            w = in(reg) $w,\n            h = inout(reg) $h => _,\n            save = in(reg) $save,\n            p = out',
  '            simd_off!(),\n            src = inout(reg) $src => _,\n            dst = inout(reg) $dst => _,\n            step = in(reg) $step,\n            w = in(reg) $w,\n            h = inout(reg) $h => _,\n            save = in(reg) $save,\n            p = out'),
 ("Q6 backward copy block: the store comes before the source load (stale register stored first)",
  '"sub {p}, {p}, #", $b, "\\n",\n            "ld1 ", $s, ", [{p}]\\n",\n            "sub {q}, {q}, #", $b, "\\n",\n            "st1 ", $s, ", [{q}]\\n",',
  '"sub {q}, {q}, #", $b, "\\n",\n            "st1 ", $s, ", [{q}]\\n",\n            "sub {p}, {p}, #", $b, "\\n",\n            "ld1 ", $s, ", [{p}]\\n",'),
 ("Q7 backward rop block: D loaded after the store of the block (store of S before D load)",
  '"sub {q}, {q}, #", $b, "\\n",\n            "ld1 ", $d, ", [{q}]\\n",\n            rop!($r),\n            "st1 ", $s, ", [{q}]\\n",',
  '"sub {q}, {q}, #", $b, "\\n",\n            "st1 ", $s, ", [{q}]\\n",\n            "ld1 ", $d, ", [{q}]\\n",\n            rop!($r),\n            "st1 ", $s, ", [{q}]\\n",'),
 ("Q8 rows bottom-up only when dy > src_y + 1 (one-row scroll down goes top-down)",
  "(dx > src_x, dy > src_y);", "(dx > src_x, dy > src_y + 1);"),
 ("Q9 source row clip off by one at the bottom (oy + scr_h)",
  "let (y0, y1) = (ty0.max(oy), ty1.min(oy + scr_h - 1));", "let (y0, y1) = (ty0.max(oy), ty1.min(oy + scr_h));"),
 ("Q10 16-pixel loop counter tests remaining > 16 (last full block goes to the tail)",
  '"subs {n}, {w}, #16\\n",\n            "b.lo 4f\\n",', '"subs {n}, {w}, #17\\n",\n            "b.lo 4f\\n",'),
 ("Q11 op 5 not an early return",
  "if area.w == 0 || area.h == 0 || op == 5 {", "if area.w == 0 || area.h == 0 {"),
 ("Q12 op 3 (copy kernel) taken for op 7 too",
  "        if op == 3 {\n            if backward {", "        if op == 3 || op == 7 {\n            if backward {"),
]
res = []
for name, old, new in MUT:
    text = open(keep).read(); n = text.count(old)
    if n == 0:
        res.append(f"{name}: EDIT NOT APPLIED (text not found)"); continue
    open(src, 'w').write(text.replace(old, new))
    r = subprocess.run(["cargo", "test", "-p", "tos-api", "--test", "raster_kernel_copy"], cwd=root, capture_output=True, text=True)
    o = r.stdout + r.stderr
    failed = [l for l in o.splitlines() if l.startswith("test ") and "FAILED" in l]
    tail = [l.strip() for l in o.splitlines() if "test result" in l or "SIG" in l or "signal" in l or "error:" in l][:2]
    res.append(f"{name} (occurrences {n}): rc {r.returncode} {'RED' if r.returncode else 'GREEN(!)'}; failed tests {len(failed)}; {tail}")
    shutil.copy(keep, src)
    assert sha(src) == h0
open(out, 'w').write("\n".join(res) + f"\nrestored sha256 {sha(src)} == original {h0}\n")
print("\n".join(res))
