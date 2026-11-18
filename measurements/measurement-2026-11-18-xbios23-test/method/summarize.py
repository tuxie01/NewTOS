# summarize.py <repo> <runs-dir> -- per-case.tsv (all rows; trap column), summary.txt tables, XBIOS roll-up. DIFF = case printed by tosdiff (case no = line in .tsv).
import re, sys, os, collections
repo, runs = sys.argv[1:3]
T = [("bios-r1", ["BIOS", "BIOS-falcontc"]), ("super-r1", ["SUPER", "SUPER-falcontc"])] + \
    [(f"xbios-{n}-r1", [f"XBIOS-{n}", f"XBIOS-{n}-falcontc"]) for n in "bconmap chardev flush ptr system".split()]
def pf(x): return int(x, 16) if x.lower().startswith("0x") else int(x)
rows = []
summ = []
for t, refs in T:
    cases = []
    for i, l in enumerate(open(f"{repo}/tosdump/cases/{t}.tsv"), 1):
        l = l.rstrip("\n")
        if l and l[0] not in "#!+":
            f = l.split("\t"); cases.append((i, f[0], f[2], f[4] if len(f) > 4 else ""))
    for r in refs:
        txt = open(f"{runs}/{t}/tosdiff-EMUTOS-1a7484ab-{r}.txt").read()
        diff = set(int(x) for x in re.findall(r"^case (\d+) ", txt, re.M))
        eq = rel = n14 = d14 = 0
        for (i, tf, rc, lab) in cases:
            res = "DIFF" if i in diff else "EQUAL"
            if res == "EQUAL":
                eq += 1; rel += not rc.startswith("EXACT")
            rows.append((t, r, i, tf, rc, res, lab))
            if tf.startswith("14:"): n14 += 1; d14 += res == "DIFF"
        summ.append((t, r, len(cases), eq, rel, len(diff), n14, d14))
with open(os.path.join(runs, "..", "per-case.tsv"), "w") as o:
    o.write("table\treference\tline\ttrap:fn\tretclass\tresult\tlabel\n")
    for x in rows: o.write("\t".join(map(str, x)) + "\n")
s = open(os.path.join(runs, "..", "summary.txt"), "w")
def P(x=""): print(x); s.write(x + "\n")
P("table\treference\tcases\tEQUAL\t(relaxed)\tDIFF\ttrap14 cases\ttrap14 DIFF")
for x in summ: P("\t".join(map(str, x)))
P("\nXBIOS roll-up (trap 14): fn, case rows (tt+falcontc, all tables), DIFF rows, DIFF locations")
st = collections.defaultdict(list)
for x in rows:
    tr, fn = x[3].split(":")
    if tr == "14": st[pf(fn)].append(x)
for fn in sorted(st):
    d = [x for x in st[fn] if x[5] == "DIFF"]
    P(f"0x{fn:02X}\t{len(st[fn])}\t{len(d)}\t" + " ".join(f"{x[0]}/{x[1]}#{x[2]}" for x in d))
s.close()
