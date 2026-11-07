#!/usr/bin/env python3
"""V50 red evidence: run tos-api/tests/raster_kernel_expand.rs against deliberately wrong kernels.

red-mutants.py <worktree> <out-file> <scratch-dir>

Each mutant is one textual edit of kernel/src/raster_expand.rs (the edit names the text it
replaces and how often it occurs; every occurrence is replaced). The file is copied aside
first, the edit applied, `cargo test -p tos-api --test raster_kernel_expand` run, and the
original copied back byte for byte (sha256 checked) before the next mutant. No git command
touches the tree. A mutant is red if the test run fails (assertion, or a crash: the source
ends at an inaccessible page, so an over-read kills the test process).
"""
import hashlib, os, shutil, subprocess, sys

root, out, scratch = sys.argv[1], sys.argv[2], sys.argv[3]
src = os.path.join(root, "kernel/src/raster_expand.rs")
keep = os.path.join(scratch, "raster_expand.rs.orig")

# (name, old, new, occurrences)
MUTANTS = [
    ("M1 Transparent/RevTransparent insert-if-true becomes insert-if-false (bit -> bif)",
     'concat!("bit ", $d, ".16b, v8.16b, ", $m, ".16b")',
     'concat!("bif ", $d, ".16b, v8.16b, ", $m, ".16b")', 1),
    ("M2 the one-pixel tail of the Replace kernel is dropped",
     '"tbz {n}, #0, 9f",\n            "st1 {{v0.s}}[0], [{p}]",',
     '"b 9f",\n            "st1 {{v0.s}}[0], [{p}]",', 1),
    ("M3 start bit inside the first unit off by one",
     "let off = src.bit % ubits;",
     "let off = (src.bit + 1) % ubits;", 1),
    ("M4 RevTransparent of the word source without the complement (behaves as Transparent)",
     'bit, "eor {t:w}, {t:w}, #0xffff", "ldrh {u:w}, [{q}], #2"',
     'bit, "", "ldrh {u:w}, [{q}], #2"', 1),
    ("M5 Replace: fg and bg swapped in the first four pixels of the block",
     '"bsl v0.16b, v8.16b, v9.16b",',
     '"bsl v0.16b, v9.16b, v8.16b",', 1),
    ("M6 read-modify-write kernel: v13/v14 not saved (restore kept)",
     '"stp q13, q14, [{save}, #192]",\n',
     '', 1),
    ("M7 source row not advanced per row (both kernels; the pitch shifted out to 0)",
     '"add {src}, {src}, {sstride}",',
     '"add {src}, {src}, {sstride}, lsr #63",', 2),
    ("M8 the 8-pixel tail of the read-modify-write kernel does not slide its masks",
     '"st1 {{v11.4s, v12.4s}}, [{p}], #32",\n            "mov v0.16b, v2.16b",\n            "mov v1.16b, v3.16b",',
     '"st1 {{v11.4s, v12.4s}}, [{p}], #32",', 1),
    ("M9 refill reads a unit when it already holds the bits due (over-read past the row)",
     '"b.hs 5f\\n",',
     '"b.hi 5f\\n",', 1),
    ("M10 a skipped no-op word does not step the destination",
     '"23:",\n            "add {p}, {p}, #64",',
     '"23:",', 1),
    ("M11 MD_ERASE without the fg/bg swap",
     "WriteMode::RevTransparent => (bg, fg),\n        _ => (fg, bg),\n    }\n}",
     "WriteMode::RevTransparent => (fg, bg),\n        _ => (fg, bg),\n    }\n}", 1),
    ("M12 glyph adapter: start bit taken modulo 8 of the cell column (wide cells clipped left)",
     "stride_bytes: bpr, bit: c0, unit: Unit::Byte",
     "stride_bytes: bpr, bit: c0 & 7, unit: Unit::Byte", 1),
    ("M13 mono adapter: the bounds check lets the last word through",
     "if 2 * last + 2 > bytes.len() {",
     "if 2 * last > bytes.len() {", 1),
    ("M14 glyph adapter: the short-glyph check is off by one",
     "if last >= glyph.len() {",
     "if last > glyph.len() {", 1),
]

def sha(p):
    return hashlib.sha256(open(p, "rb").read()).hexdigest()

orig_sha = sha(src)
shutil.copyfile(src, keep)
lines = [f"original kernel/src/raster_expand.rs sha256 {orig_sha}"]
try:
    for name, old, new, count in MUTANTS:
        text = open(keep).read()
        assert text.count(old) == count, f"{name}: pattern found {text.count(old)}x, expected {count}"
        open(src, "w").write(text.replace(old, new))
        r = subprocess.run(["cargo", "test", "-p", "tos-api", "--test", "raster_kernel_expand"],
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
lines.append(f"\nrestored kernel/src/raster_expand.rs sha256 {sha(src)} (== original: {sha(src) == orig_sha})")
open(out, "w").write("\n".join(lines) + "\n")
print("\n".join(lines))
