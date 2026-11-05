#!/usr/bin/env python3 -I
"""summary.py <script> <run-dir> ... -- one line per run and (kind, wait mode): the ret-press
values of analyse.py (ticks of 5 ms; drag: ret-release in brackets), in click order."""
import subprocess, sys, pathlib
here = pathlib.Path(__file__).parent
for run in [r for r in sys.argv[2:] if (pathlib.Path(r) / "MNPROBE.LOG").exists()]:
    out = subprocess.run([sys.executable, "-I", str(here / "analyse.py"), sys.argv[1], run],
                         capture_output=True, text=True).stdout.splitlines()
    head = out[0].split()
    groups = {}
    for l in out[2:]:
        f = l.split()
        if len(f) < 8 or not f[0].isdigit(): continue
        kind, mode, rp, rr = f[1], f[2], f[6], f[7]
        te = f[8] if len(f) > 9 else ""
        v = rp if kind == "click" else f"{rp}({rr})"
        if te: v += f"[t{te}]"
        groups.setdefault((kind, mode), []).append(v)
    print(f"{head[1]:24} dclick {head[3]}:  " + "  ".join(f"{k} {m}: {' '.join(v)}" for (k, m), v in groups.items()))
