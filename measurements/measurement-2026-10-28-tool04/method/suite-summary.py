#!/usr/bin/env python3
"""TOOL04: per-target result lines of a `cargo test` / `cargo xtask test` log.

Usage: suite-summary.py <log> [<names-out>]
Prints one line per test target (unit/integration binary or doc-test set):
target, passed, failed, ignored, libtest time; then the totals split into
binaries and doctests. With <names-out>: writes every `test <name> ... <res>`
line, prefixed with its target, sorted (for the before/after name diff;
needs the log of a run with `--format pretty`, cargo's default).
"""
import re
import sys

RUN = re.compile(r"^\s*Running (?:unittests )?(\S+) \((.+)\)")
DOC = re.compile(r"^\s*Doc-tests (\S+)")
RES = re.compile(r"test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored;.*finished in ([0-9.]+)s")
TST = re.compile(r"^test (.+?) \.\.\. (\w+)")

target = None
rows = []
names = []
for line in open(sys.argv[1], errors="replace"):
    m = RUN.match(line)
    if m:
        exe = m.group(2).rsplit("/", 1)[-1]
        target = "bin:" + exe.rsplit("-", 1)[0]
        continue
    m = DOC.match(line)
    if m:
        target = "doc:" + m.group(1)
        continue
    m = TST.match(line)
    if m and target:
        names.append(f"{target}\t{m.group(1)}\t{m.group(2)}")
        continue
    m = RES.search(line)
    if m and target:
        rows.append((target, int(m.group(2)), int(m.group(3)), int(m.group(4)), float(m.group(5))))
        target = None

tot = {"bin": [0, 0, 0, 0, 0.0], "doc": [0, 0, 0, 0, 0.0]}
for t, p, f, i, s in rows:
    print(f"{t}\t{p}\t{f}\t{i}\t{s:.2f}")
    k = tot[t.split(":", 1)[0]]
    k[0] += 1; k[1] += p; k[2] += f; k[3] += i; k[4] += s
for k, v in tot.items():
    print(f"TOTAL {k}: targets={v[0]} passed={v[1]} failed={v[2]} ignored={v[3]} libtest_s={v[4]:.2f}")
a = [tot["bin"][i] + tot["doc"][i] for i in range(1, 4)]
print(f"TOTAL all: passed={a[0]} failed={a[1]} ignored={a[2]}")
if len(sys.argv) > 2:
    with open(sys.argv[2], "w") as f:
        f.write("".join(n + "\n" for n in sorted(names)))
