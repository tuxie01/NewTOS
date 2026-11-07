#!/usr/bin/env python3
"""V49 red evidence: run tos-api/tests/raster_kernel_fill.rs against deliberately wrong kernels.

red-mutants.py <worktree> <out-file> [<scratch-dir>]

Each mutant is one textual edit of kernel/src/raster_kernel.rs. The file is copied aside first,
the edit applied, `cargo test -p tos-api --test raster_kernel_fill` run, and the original copied
back byte for byte (sha256 checked) before the next mutant. No git command touches the tree.
"""
import hashlib, os, shutil, subprocess, sys

root, out = sys.argv[1], sys.argv[2]
scratch = sys.argv[3] if len(sys.argv) > 3 else None
src = os.path.join(root, "kernel/src/raster_kernel.rs")
keep = os.path.join(scratch or os.path.dirname(out), "raster_kernel.rs.orig")

MUTANTS = [
    ("M1 Transparent/RevTransparent insert-if-true becomes insert-if-false (bit -> bif)",
     'concat!("bit ", $d, ".16b, v8.16b, ", $m, ".16b")',
     'concat!("bif ", $d, ".16b, v8.16b, ", $m, ".16b")'),
    ("M2 the one-pixel tail of the Replace kernel is dropped",
     '"tbz {n}, #0, 8f",\n                "st1 {{v0.s}}[0], [{p}]",',
     '"b 8f",\n                "st1 {{v0.s}}[0], [{p}]",'),
    ("M3 pattern phase off by one column",
     "let shift = 16 - (area.x & 15) as u32;",
     "let shift = 16 - ((area.x + 1) & 15) as u32;"),
    ("M4 RevTransparent without the complemented word (behaves as Transparent)",
     '"eor {t:w}, {t:w}, #0xffff\\ncbz {t:w}, 8f",',
     '"cbz {t:w}, 8f",'),
    ("M5 Replace: fg and bg swapped in the first four pixels of the block",
     '"bsl v0.16b, v8.16b, v9.16b",',
     '"bsl v0.16b, v9.16b, v8.16b",'),
    ("M6 read-modify-write kernel: v13/v14 not saved (restore kept)",
     '"stp q13, q14, [{save}, #192]",\n            "ld1',
     '"ld1'),
    ("M7 pattern row not advanced per row",
     '"add {prow}, {prow}, #1",\n                "subs {h}, {h}, #1",',
     '"subs {h}, {h}, #1",'),
    ("M8 the 8-pixel tail of the read-modify-write kernel does not slide its masks",
     '"st1 {{v11.4s, v12.4s}}, [{p}], #32",\n            "mov v0.16b, v2.16b",\n            "mov v1.16b, v3.16b",',
     '"st1 {{v11.4s, v12.4s}}, [{p}], #32",'),
]

def sha(p):
    return hashlib.sha256(open(p, "rb").read()).hexdigest()

orig_sha = sha(src)
shutil.copyfile(src, keep)
lines = [f"original kernel/src/raster_kernel.rs sha256 {orig_sha}"]
try:
    for name, old, new in MUTANTS:
        text = open(keep).read()
        assert text.count(old) == 1, f"{name}: pattern found {text.count(old)}x"
        open(src, "w").write(text.replace(old, new))
        r = subprocess.run(["cargo", "test", "-p", "tos-api", "--test", "raster_kernel_fill"],
                           cwd=root, capture_output=True, text=True)
        shutil.copyfile(keep, src)
        assert sha(src) == orig_sha
        res = [l for l in r.stdout.splitlines() if l.startswith("test ") or l.startswith("test result")]
        first = next((l for l in (r.stdout + r.stderr).splitlines() if "kernel != reference" in l
                      or "assertion" in l or "V registers named" in l), "")
        lines.append(f"\n== {name}\n   exit {r.returncode}")
        lines += ["   " + l for l in res]
        if first:
            lines.append("   first message: " + first.strip()[:400])
finally:
    shutil.copyfile(keep, src)
lines.append(f"\nrestored kernel/src/raster_kernel.rs sha256 {sha(src)} (== original: {sha(src) == orig_sha})")
open(out, "w").write("\n".join(lines) + "\n")
print("\n".join(lines))
