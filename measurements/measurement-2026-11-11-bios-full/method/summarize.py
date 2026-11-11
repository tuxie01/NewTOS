# summarize.py <repo> <runs-dir> -- per-case EQUAL/DIFF list and per-function roll-up from the tosdiff outputs.
# A case number printed by tosdiff = line number in the .tsv. A case is DIFF if tosdiff prints it, else EQUAL
# (EQUAL includes the relaxed return classes IGNORE/NONZERO/HANDLE/PTR_RAM, applied by tosdiff itself).
import re, sys, os, collections
repo, runs = sys.argv[1:3]
T = [("bios-core-r1", ["BIOS-core", "BIOS-core-falcontc"]), ("bios-r1", ["BIOS", "BIOS-falcontc"]),
     ("bios-r2", ["BIOS-r2-disk"]), ("bios-r2-phys", ["BIOS-r2-phys-disk"]), ("bios-r2-x001", ["BIOS-r2-x001-disk"]),
     ("bios-r2-x004", ["BIOS-r2-x004-disk"]), ("bios-r2-x005", ["BIOS-r2-x005-disk"]), ("bios-r2-x006", ["BIOS-r2-x006-disk"])]
def pf(x): return int(x, 16) if x.lower().startswith("0x") else int(x)
B13 = "Getmpb Bconstat Bconin Bconout Rwabs Setexc Tickcal Getbpb Bcostat Mediach Drvmap Kbshift".split()
out = open(os.path.join(runs, "..", "per-case.tsv"), "w")
out.write("table\treference\tline\ttrap:fn\tretclass\tresult\tlabel\n")
fnstat = collections.defaultdict(lambda: collections.defaultdict(set))  # (trap,fn) -> (table,ref) -> {EQUAL,DIFF}
summ = []
for t, refs in T:
    cases = []
    for i, l in enumerate(open(f"{repo}/tosdump/cases/{t}.tsv"), 1):
        l = l.rstrip("\n")
        if l and l[0] not in "#!+":
            f = l.split("\t")
            cases.append((i, f[0], f[2], f[4] if len(f) > 4 else ""))
    for r in refs:
        txt = open(f"{runs}/{t}/tosdiff-EMUTOS-1a7484ab-{r}.txt").read()
        diff = set(int(x) for x in re.findall(r"^case (\d+) ", txt, re.M))
        eq = 0
        relaxed = 0
        for (i, tf, rc, lab) in cases:
            res = "DIFF" if i in diff else "EQUAL"
            if res == "EQUAL":
                eq += 1
                if not rc.startswith("EXACT"): relaxed += 1
            tr, fn = tf.split(":"); key = (int(tr), pf(fn))
            fnstat[key][(t, r)].add(res)
            out.write(f"{t}\t{r}\t{i}\t{tf}\t{rc}\t{res}\t{lab}\n")
        summ.append((t, r, len(cases), eq, relaxed, len(diff)))
out.close()
print("table\treference\tcases\tEQUAL\t(of which relaxed class)\tDIFF")
for s in summ: print("\t".join(map(str, s)))
print("\nBIOS roll-up (trap 13): function, cases total/DIFF over all runs, verdict")
for fn, name in enumerate(B13):
    n = d = 0; where = []
    for (t, r), v in fnstat.get((13, fn), {}).items(): pass
    for (t, r), v in fnstat.get((13, fn), {}).items(): pass
    out2 = []
    # count per case rows
    rows = [l.split("\t") for l in open(os.path.join(runs, "..", "per-case.tsv")).read().splitlines()[1:]]
    sel = [x for x in rows if x[3].split(":")[0] == "13" and pf(x[3].split(":")[1]) == fn]
    n = len(sel); d = [x for x in sel if x[5] == "DIFF"]
    print(f"0x{fn:02X} {name}\t{n} case rows\t{len(d)} DIFF rows" + ("".join(f"\t{x[0]}/{x[1]} L{x[2]}" for x in d)))
