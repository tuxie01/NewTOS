#!/usr/bin/env python3 -I
"""counts.py <emutos n1 dir> <before n1 dir> <after n1 dir>: write commands/sectors per Fwrite
pattern, whole save (Fcreate + Fwrite + Fclose), EmuTOS, NewTOS before, NewTOS after, from WFPROBE.LOG."""
import os, re, sys
def rows(d):
    out = {}
    for l in open(os.path.join(d, "WFPROBE.LOG"), errors="replace"):
        m = re.match(r"pat total\s+(\d+) step\s+(\d+): Fcreate=\S+ writes (\d+)/(\d+) Fwrite x(\d+) last=\S+ writes (\d+)/(\d+) Fclose=\S+ writes (\d+)/(\d+)", l)
        if m:
            t, st, c1, s1, n, c2, s2, c3, s3 = map(int, m.groups())
            out[(t, st)] = (n, c2, s2, c3, s3, c1 + c2 + c3, s1 + s2 + s3)
        m = re.match(r"overwrite 64 x 100 B.*writes (\d+)/(\d+) Fclose=\S+ writes (\d+)/(\d+)", l)
        if m:
            c2, s2, c3, s3 = map(int, m.groups())
            out[(0, 100)] = (64, c2, s2, c3, s3, c2 + c3, s2 + s3)
    return out
e, b, a = (rows(d) for d in sys.argv[1:4])
print("%9s %7s %5s | %-24s | %-24s | %-24s" % ("total", "step", "calls", "EmuTOS  Fwrite Fclose all", "before  Fwrite Fclose all", "after   Fwrite Fclose all"))
def f(r):
    return "%5d/%-5d %3d %6d" % (r[1], r[2], r[3], r[5]) if r else "-"
for k in sorted(a, key=lambda x: (x[0], x[1])):
    label = "overwrite" if k[0] == 0 else str(k[0])
    print("%9s %7d %5d | %-24s | %-24s | %-24s" % (label, k[1], a[k][0], f(e.get(k)), f(b.get(k)), f(a.get(k))))
print("columns: commands/sectors during the Fwrites, commands at Fclose, commands of the whole save")
