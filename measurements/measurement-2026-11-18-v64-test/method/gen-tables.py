# gen-tables.py <outdir> -- writes tmp-v64-pens.tsv and tmp-v64-fill.tsv (temporary tester tables, never committed)
import sys, os
o = sys.argv[1]
H = "# opcode\tsub\tintin\tptsin\tcomment\n"
reqs = [0, 1, 2, 17, 25, 67, 133, 500, 999, 1000, -1, -50, 1001, 1500, 32767, -32768]
L = ["# tmp-v64-pens.tsv -- tester temporary table (V64 run, never committed)\n", H]
for c in reqs:
    for tag, rgb in (("ccc", f"{c},{c},{c}"), ("c00", f"{c},0,0")):
        for fl in (0, 1):
            L.append(f"+14\t0\t3,{rgb}\t-\tvs_color pen3 {tag} c={c}\n")
            L.append(f"26\t0\t3,{fl}\t-\tvq_color pen3 flag{fl} {tag} c={c}\n")
for p in (-1, 256, 257, 32767, -32768):
    for fl in (0, 1):
        L.append(f"+14\t0\t{p},500,500,500\t-\tvs_color pen {p}\n")
        L.append(f"26\t0\t{p},{fl}\t-\tvq_color pen {p} flag{fl}\n")
for p in list(range(0, 16)) + [16, 17, 25, 100, 128, 200, 254, 255]:
    for fl in (0, 1):
        L.append(f"26\t0\t{p},{fl}\t-\tdefault vq_color pen {p} flag{fl}\n")
for c in [0, 1, 2, 17, 25, 67, 133, 500, 999, 1000]:
    for tag, rgb in (("r", f"{c},0,0"), ("g", f"0,{c},0"), ("b", f"0,0,{c}"), ("w", f"{c},{c},{c}")):
        L.append(f"+14\t0\t3,{rgb}\t-\tvs_color pen3 {tag} c={c}\n")
        L.append("+25\t0\t3\t-\tvsf_color 3\n+23\t0\t1\t-\tsolid\n+114\t0\t-\t10,10,30,30\trecfl\n")
        L.append(f"105\t0\t-\t20,20\tpixel inside pen3 {tag} c={c}\n")
open(os.path.join(o, "tmp-v64-pens.tsv"), "w").write("".join(L))

F = ["# tmp-v64-fill.tsv -- tester temporary table (V64 run, never committed): flood-fill region with test pens\n", H]
pens = "+14\t0\t3,17,0,0\t-\tpen3 (17,0,0)\n+14\t0\t4,25,0,0\t-\tpen4 (25,0,0)\n"
pre = pens + "+23\t0\t1\t-\tsolid\n+25\t0\t4\t-\tvsf 4\n+114\t0\t-\t100,100,200,200\trecfl B\n+25\t0\t3\t-\tvsf 3\n+114\t0\t-\t120,120,180,180\trecfl A\n+25\t0\t2\t-\tvsf red\n+103\t0\t-1\t150,150\tfill-while-same seed in A\n"
for x, y in [(150,150),(125,125),(179,179),(119,150),(110,110),(190,190),(100,100),(199,199),(150,101),(99,99),(201,201),(50,50)]:
    F.append(pre + f"105\t0\t-\t{x},{y}\tFWS probe {x},{y}\n")
pre = pens + "+23\t0\t1\t-\tsolid\n+17\t0\t4\t-\tvsl 4\n+6\t0\t-\t100,100,200,100,200,200,100,200,100,100\tframe pen4\n+25\t0\t3\t-\tvsf 3\n+103\t0\t4\t150,150\tboundary fill colour 4\n"
for x, y in [(150,150),(110,110),(190,190),(150,190),(150,101),(99,150),(50,50),(100,150),(201,150)]:
    F.append(pre + f"105\t0\t-\t{x},{y}\tBF probe {x},{y}\n")
pre = pens + "+23\t0\t1\t-\tsolid\n+17\t0\t4\t-\tvsl 4\n+6\t0\t-\t100,100,200,100,200,200,100,200,100,100\tframe pen4\n+25\t0\t2\t-\tvsf red\n+103\t0\t4\t150,150\tboundary fill red, boundary 4\n"
for x, y in [(150,150),(110,110),(190,190),(100,150),(99,150),(50,50)]:
    F.append(pre + f"105\t0\t-\t{x},{y}\tBF-red probe {x},{y}\n")
open(os.path.join(o, "tmp-v64-fill.tsv"), "w").write("".join(F))
