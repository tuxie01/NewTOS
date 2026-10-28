#!/usr/bin/env python3
"""TOOL04: test names before vs after, and fast tier vs full tier.

Usage: names-diff.py <before-names-dir> <names-fast.txt> <names-full.txt>
before = the `--list --format terse` output per binary (launch-times.py
<outdir>/names/); fast/full = suite-summary.py name files of the two tier runs.
libtest prints a #[should_panic] test as `<name> - should panic` in a run but
as `<name>` in --list; that suffix is removed before comparing.
"""
import os
import sys

def norm(n):
    return n[:-len(" - should panic")] if n.endswith(" - should panic") else n

before = set()
d = sys.argv[1]
for f in os.listdir(d):
    for line in open(os.path.join(d, f)):
        if line.endswith(": test\n"):
            before.add(("bin:" + f[:-4], line[: -len(": test\n")]))

def load(p):
    return {(t, norm(n)) for t, n, _ in (l.rstrip("\n").split("\t") for l in open(p))}

fast, full = load(sys.argv[2]), load(sys.argv[3])
full_bin = {x for x in full if x[0].startswith("bin:")}
print(f"before (binaries, --list): {len(before)}")
print(f"fast tier: {len(fast)}   full tier: {len(full)} (binaries {len(full_bin)}, doctests {len(full) - len(full_bin)})")
print("lost (before, not in full):", sorted(before - full_bin) or "none")
print("new (full, not before):", sorted(full_bin - before) or "none")
print("in fast, not in full:", sorted(fast - full) or "none")
print("in full only (not fast):", sorted(full - fast) or "none")
