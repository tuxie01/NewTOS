#!/usr/bin/env python3
"""TOOL04: per test binary, separate the binary's start cost from its test time.

Usage: launch-times.py <exes.tsv> <outdir> [--list-only]

For every executable in exes.tsv (exes-from-json.py) in file order, serially:
  first  = wall time of `<exe> --list --format terse`   (first launch: no test runs)
  second = wall time of the same command again           (second launch)
  run    = wall time of `<exe>` (the full test run, libtest defaults)
and libtest's own `finished in Xs` plus the passed/failed/ignored counts.
cwd = the package directory and CARGO_MANIFEST_DIR set, as `cargo test` does.
Writes <outdir>/launch.tsv, <outdir>/names/<target>.txt (the --list output),
<outdir>/out/<target>.txt (the full run's stdout+stderr).
"""
import os
import re
import subprocess
import sys
import time

exes, outdir = sys.argv[1], sys.argv[2]
list_only = "--list-only" in sys.argv[3:]
os.makedirs(os.path.join(outdir, "names"), exist_ok=True)
os.makedirs(os.path.join(outdir, "out"), exist_ok=True)
tsv = open(os.path.join(outdir, "launch.tsv"), "a", buffering=1)
if tsv.tell() == 0:
    tsv.write("kind\ttarget\tsize\tfirst_s\tsecond_s\trun_s\tlibtest_s\tpassed\tfailed\tignored\tstart_epoch\n")
RES = re.compile(r"test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored;.*finished in ([0-9.]+)s")


def timed(cmd, cwd, env):
    t0 = time.monotonic()
    p = subprocess.run(cmd, cwd=cwd, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    return time.monotonic() - t0, p.returncode, p.stdout.decode("utf-8", "replace")


for line in open(exes):
    kind, target, exe, pdir = line.rstrip("\n").split("\t")[:4]
    env = dict(os.environ, CARGO_MANIFEST_DIR=pdir)
    size = os.path.getsize(exe)
    start = time.time()
    t1, rc1, names = timed([exe, "--list", "--format", "terse"], pdir, env)
    open(os.path.join(outdir, "names", target + ".txt"), "w").write(names)
    t2, _, _ = timed([exe, "--list", "--format", "terse"], pdir, env)
    t3, lt, pa, fa, ig = "", "", "", "", ""
    if not list_only:
        t3, rc3, out = timed([exe], pdir, env)
        open(os.path.join(outdir, "out", target + ".txt"), "w").write(out)
        m = RES.search(out)
        if m:
            pa, fa, ig, lt = m.group(2), m.group(3), m.group(4), m.group(5)
        t3 = f"{t3:.3f}"
    tsv.write(f"{kind}\t{target}\t{size}\t{t1:.3f}\t{t2:.3f}\t{t3}\t{lt}\t{pa}\t{fa}\t{ig}\t{start:.0f}\n")
    print(f"{target}: first {t1:.2f}s second {t2:.2f}s run {t3}s", flush=True)
