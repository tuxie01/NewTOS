# compare.py <table.tsv> <run-prefix> -> per-case line, four configs side by side, '*' when a config differs from main
import sys, subprocess, os
tsv, pre = sys.argv[1], sys.argv[2]
R = "/Volumes/Daten/Development/PiRusTOS/target-agents/agent-a5835b4b6e8d83e6f/runs"
here = os.path.dirname(os.path.abspath(__file__))
com = [l.rstrip("\n").split("\t")[4] for l in open(tsv) if l.strip() and l[0] not in "#+"]
cfgs = ["main", "d", "v16", "v32"]
rec = {}
for c in cfgs:
    out = subprocess.run(["python3", "-I", here + "/decode.py", f"{R}/{pre}-{c}/VDI.BIN"], capture_output=True, text=True).stdout.splitlines()
    rec[c] = [l.split(" ", 2)[2] for l in out]
print("case  main | default | v16 | v32   comment   (i=intout p=ptsout)")
for i, cm in enumerate(com):
    vals = [rec[c][i] for c in cfgs]
    flag = "".join("." if v == vals[0] else "*" for v in vals[1:])
    print(f"{i:4d} {flag} " + " | ".join(vals) + "   " + cm)
