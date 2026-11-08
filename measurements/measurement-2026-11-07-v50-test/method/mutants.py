#!/usr/bin/env python3
"""Tester mutants for V50: one textual edit of kernel/src/raster_expand.rs each, run the host test, restore from a copy.
usage: mutants.py <worktree> <out> <scratch>"""
import hashlib, os, shutil, subprocess, sys
root, out, scratch = sys.argv[1:4]
src = os.path.join(root, "kernel/src/raster_expand.rs"); keep = os.path.join(scratch, "raster_expand.rs.keep")
shutil.copy(src, keep)
sha = lambda p: hashlib.sha256(open(p,'rb').read()).hexdigest()
h0 = sha(src)
MUT = [
 ("T1 Word Xor uses bit instead of eor", "eor, \"\", \"ldrh {u:w}, [{q}], #2\"", "bit, \"\", \"ldrh {u:w}, [{q}], #2\""),
 ("T2 MD_ERASE colour swap removed (mono_colors)", "WriteMode::RevTransparent => (bg, fg),\n        _ => (fg, bg),\n    }\n}\n\n/// `blit_mono`", None),
 ("T3 rmw kernel: restore of q8,q10 dropped", "            \"ldp q8, q10, [{save}, #128]\",\n", ""),
 ("T4 Replace kernel: restore of q10 dropped", "            \"ldr q10, [{save}, #160]\",\n", ""),
 ("T5 refill reads a unit when nb == needed (one unit too many)", "\"b.hs 5f\\n\"", "\"b.hi 5f\\n\""),
 ("T6 Byte Replace gets fg and bg swapped", "\"ldrb {u:w}, [{q}], #1\", \"56\", \"8\", dst, stride, src_p, sstride, off, w, h, fg, bg, save, bits", "\"ldrb {u:w}, [{q}], #1\", \"56\", \"8\", dst, stride, src_p, sstride, off, w, h, bg, fg, save, bits"),
 ("T7 start bit shift in row prologue dropped", "\"lsl {acc}, {acc}, {off}\\n\"", "\"\""),
 ("T8 mono start bit +1", "bit: sx0, unit: Unit::Word", "bit: sx0 + 1, unit: Unit::Word"),
 ("T9 Replace tail 2-pixel step tests bit 0", "\"tbz {n}, #1, 22f\",\n            \"st1 {{v0.2s}}, [{p}], #8\",\n            \"ext v0.16b, v0.16b, v0.16b, #8\",\n            \"22:\",\n            \"tbz {n}, #0, 9f\",\n            \"st1 {{v0.s}}[0], [{p}]\",\n            // ── next row ──\n            \"9:\",\n            \"add {dst}, {dst}, {stride}\",\n            \"add {src}, {src}, {sstride}\",\n            \"subs {h}, {h}, #1\",\n            \"b.ne 2b\",\n            \"ldp q0, q1, [{save}, #0]\",\n            \"ldp q2, q3, [{save}, #32]\",\n            \"ldp q4, q5, [{save}, #64]\",\n            \"ldp q6, q7, [{save}, #96]\",\n            \"ldp q8, q9", "\"tbz {n}, #0, 22f\",\n            \"st1 {{v0.2s}}, [{p}], #8\",\n            \"ext v0.16b, v0.16b, v0.16b, #8\",\n            \"22:\",\n            \"tbz {n}, #0, 9f\",\n            \"st1 {{v0.s}}[0], [{p}]\",\n            // ── next row ──\n            \"9:\",\n            \"add {dst}, {dst}, {stride}\",\n            \"add {src}, {src}, {sstride}\",\n            \"subs {h}, {h}, #1\",\n            \"b.ne 2b\",\n            \"ldp q0, q1, [{save}, #0]\",\n            \"ldp q2, q3, [{save}, #32]\",\n            \"ldp q4, q5, [{save}, #64]\",\n            \"ldp q6, q7, [{save}, #96]\",\n            \"ldp q8, q9"),
 ("T10 glyph accepts box wider than the cell (>= -> >)", "(x1 - x) as usize >= cell_w", "(x1 - x) as usize > cell_w"),
 ("T11 mono: 'raster too short' test off by one word", "2 * last + 2 > bytes.len()", "2 * last + 2 > bytes.len() + 2"),
 ("T12 skip of unchanged word moves 63 bytes", "\"add {p}, {p}, #64\",\n            \"subs {n}, {n}, #16\",\n            \"b.ne 3b\",\n            \"b 9f\",\n            // Tail", "\"add {p}, {p}, #60\",\n            \"subs {n}, {n}, #16\",\n            \"b.ne 3b\",\n            \"b 9f\",\n            // Tail"),
]
res = []
for name, old, new in MUT:
    if name.startswith("T2"):
        old = "WriteMode::RevTransparent => (bg, fg),"; new = "WriteMode::RevTransparent => (fg, bg),"
    text = open(keep).read(); n = text.count(old)
    if n == 0: res.append(f"{name}: EDIT NOT APPLIED (text not found)"); continue
    open(src, 'w').write(text.replace(old, new))
    r = subprocess.run(["cargo","test","-p","tos-api","--test","raster_kernel_expand"], cwd=root, capture_output=True, text=True)
    failed = [l for l in (r.stdout+r.stderr).splitlines() if l.startswith("test ") and "FAILED" in l]
    tail = [l for l in (r.stdout+r.stderr).splitlines() if "test result" in l or "SIG" in l or "signal" in l]
    res.append(f"{name} (occurrences {n}): rc {r.returncode} {'RED' if r.returncode else 'GREEN(!)'}; failed tests {len(failed)}; {tail[:2]}")
    shutil.copy(keep, src)
    assert sha(src) == h0
open(out, 'w').write("\n".join(res) + f"\nrestored sha256 {sha(src)} == original {h0}\n")
print("\n".join(res))
