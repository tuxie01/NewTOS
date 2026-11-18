#!/usr/bin/env python3
"""compare.py <runsdir> <cfg> -- base-<cfg> vs wt-<cfg>: per table the stdout verdict lines, the tosdiff reports, and every differing dump byte offset"""
import sys, glob, os, re
R, cfg = sys.argv[1], sys.argv[2]
B, W = f"{R}/base-{cfg}", f"{R}/wt-{cfg}"
RAST = {"vdi-raster", "vdi-raster-screen", "vdi-v18-raster", "vdi-v18-t2-raster"}
def norm(s):
    s = re.sub(r"/Volumes/\S*?/agent-[0-9a-f]+/(base|wt)/?", "<tree>/", s)
    s = re.sub(r"/Volumes/\S*?/agent-[0-9a-f]+/", "<agent>/", s)
    s = re.sub(r"build [0-9a-f]{4}\b", "build XXXX", s)   # the commit's own 4-hex build id (header bytes 12-15)
    return s
def verdicts(p):
    return [l for l in open(p, errors="replace").read().splitlines() if re.search(r"cases differ|run complete|tosdiff .*exit|result|bytes, sha", l)]
tabs = sorted(os.path.basename(p)[:-7] for p in glob.glob(B + "/*.stdout"))
for t in tabs:
    out = []
    vb = [re.sub(r"sha256 \S+|in \d+ s|after \d+ s|\d+ s$", "", norm(l)) for l in verdicts(f"{B}/{t}.stdout") if "total" not in l]
    vw = [re.sub(r"sha256 \S+|in \d+ s|after \d+ s|\d+ s$", "", norm(l)) for l in verdicts(f"{W}/{t}.stdout") if "total" not in l]
    vb = [x for x in vb if "sha" not in x]; vw = [x for x in vw if "sha" not in x]
    # tosdiff reports
    rep = "SAME"
    for rb in sorted(glob.glob(f"{B}/{t}/tosdiff-*.txt")):
        rw = rb.replace(B, W)
        if not os.path.exists(rw): rep = "MISSING " + os.path.basename(rw); break
        if norm(open(rb, errors="replace").read()) != norm(open(rw, errors="replace").read()): rep = "DIFF " + os.path.basename(rb); break
    nrep = len(glob.glob(f"{B}/{t}/tosdiff-*.txt"))
    dumps = [os.path.basename(p) for p in glob.glob(f"{B}/{t}/*.BIN")]
    res = []
    for d in dumps:
        a = open(f"{B}/{t}/{d}", "rb").read(); b = open(f"{W}/{t}/{d}", "rb").read()
        if len(a) != len(b): res.append(f"{d}: LENGTH {len(a)} vs {len(b)}"); continue
        diff = [i for i in range(len(a)) if a[i] != b[i]]
        bid = [i for i in diff if 12 <= i <= 15]; diff = [i for i in diff if not 12 <= i <= 15]
        d = d + (' [build-id bytes %s differ]' % bid if bid else '')
        if not diff: res.append(f"{d}: byte-identical ({len(a)} bytes)"); continue
        if t in RAST:
            rs = 3696
            cells = {}
            outside = 0
            hdr = [i for i in diff if i < 32 and not 12 <= i <= 15]
            for i in diff:
                if i < 32: continue
                rec, off = divmod(i - 32, rs)
                if (22 <= off <= 37) or (52 <= off <= 67):
                    cells.setdefault(off, 0); cells[off] += 1
                else: outside += 1
            res.append(f"{d}: {len(diff)} differing bytes; header {len(hdr)}; outside MFDB cells {outside}; offsets-in-record {sorted(cells)} "
                       f"(records touched {len(set((i-32)//rs for i in diff if i>=32))})")
        else:
            res.append(f"{d}: {len(diff)} differing bytes at {diff[:12]}")
    print(f"== {t}: verdicts {'SAME' if vb == vw else 'DIFF'}; tosdiff reports ({nrep}) {rep}; " + " | ".join(res))
    if vb != vw:
        for x, y in zip(vb, vw):
            if x != y: print("   base:", x, "\n   wt:  ", y)
