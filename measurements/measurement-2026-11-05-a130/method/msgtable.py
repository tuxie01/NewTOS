#!/usr/bin/env python3
"""msgtable.py <emutos SLPROBE.LOG> <newtos SLPROBE.LOG> -- the window messages per gesture of the
slprobe main input (v3 order, gen-slprobe-input.py): one primary message per gesture (the first
WM_* after the gesture's press, WM_REDRAW and the work-area button returns skipped), plus the count
of further equal WM_ARROWED while the button was held (X lines) and the reply position."""
import re
import sys

G = []
for cfg in ("1000", "300", "-1"):
    for ax in ("V", "H"):
        G += [(cfg, ax, "dragslow"), (cfg, ax, "dragfast")]
        G += [(cfg, ax, "pagehi")] if cfg == "1000" else [(cfg, ax, "pagelo"), (cfg, ax, "pagehi")]
NAMES = {20: "WM_REDRAW", 24: "WM_ARROWED", 25: "WM_HSLID", 26: "WM_VSLID"}
WA = ["UPPAGE", "DNPAGE", "UPLINE", "DNLINE", "LFPAGE", "RTPAGE", "LFLINE", "RTLINE"]


def parse(path):
    ev = []
    for line in open(path):
        m = re.match(r"R\d+ t=\d+ which=0010 mx=(-?\d+) my=(-?\d+) .*msg=([-\d,]+)", line)
        if m:
            msg = list(map(int, m.group(3).split(",")))
            if msg[0] == 20:
                continue
            s = NAMES.get(msg[0], str(msg[0]))
            s += f" {WA[msg[4]]}" if msg[0] == 24 else f" {msg[4]}"
            ev.append([s, 0])
            continue
        m = re.match(r"X\d+ (\d+) more equal", line)
        if m:
            ev[-1][1] = int(m.group(1))
    return ev


e, n = parse(sys.argv[1]), parse(sys.argv[2])
assert len(e) == len(n) == len(G), (len(e), len(n), len(G))
print(f"{'cfg':5s} {'axis':4s} {'gesture':9s} | {'EmuTOS':34s} | NewTOS")
for g, a, b in zip(G, e, n):
    fa = a[0] + (f" (+{a[1]} while held)" if a[1] else "")
    fb = b[0] + (f" (+{b[1]} while held)" if b[1] else "")
    print(f"{g[0]:5s} {g[1]:4s} {g[2]:9s} | {fa:34s} | {fb}{'' if fa == fb else '   DIFF'}")
