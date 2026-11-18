# gen-fill2.py <outdir>: controls for the boundary fill (temporary table, never committed)
import os, sys
F = ["# tmp-v64-fill2.tsv -- tester temporary table (V64 run, never committed): boundary-fill controls\n", "# opcode\tsub\tintin\tptsin\tcomment\n"]
pts = [(150,150),(110,110),(99,150),(50,50),(100,150),(201,150)]
frame = "+17\t0\t1\t-\tvsl 1 black\n+6\t0\t-\t100,100,200,100,200,200,100,200,100,100\tframe black\n"
pre = "+23\t0\t1\t-\tsolid\n" + frame + "+25\t0\t2\t-\tvsf red\n+103\t0\t1\t150,150\tboundary fill black, red\n"
for x, y in pts: F.append(pre + f"105\t0\t-\t{x},{y}\tC1 default pens line frame, probe {x},{y}\n")
strips = "+23\t0\t1\t-\tsolid\n+25\t0\t4\t-\tvsf 4\n" + "".join(f"+114\t0\t-\t{a}\tstrip\n" for a in ["100,100,200,103","100,197,200,200","100,100,103,200","197,100,200,200"])
pens = "+14\t0\t3,17,0,0\t-\tpen3 (17,0,0)\n+14\t0\t4,25,0,0\t-\tpen4 (25,0,0)\n"
pre = pens + strips + "+25\t0\t3\t-\tvsf 3\n+103\t0\t4\t150,150\tboundary fill: boundary pen 4, fill pen 3\n"
for x, y in pts: F.append(pre + f"105\t0\t-\t{x},{y}\tC2 test pens strip ring, probe {x},{y}\n")
pre = pens + strips + "+25\t0\t2\t-\tvsf red\n+103\t0\t4\t150,150\tboundary fill: boundary pen 4, fill red\n"
for x, y in pts: F.append(pre + f"105\t0\t-\t{x},{y}\tC3 test pens strip ring, red fill, probe {x},{y}\n")
pre = "+23\t0\t1\t-\tsolid\n+25\t0\t1\t-\tvsf 1\n" + "".join(f"+114\t0\t-\t{a}\tstrip\n" for a in ["100,100,200,103","100,197,200,200","100,100,103,200","197,100,200,200"]) + "+25\t0\t2\t-\tvsf red\n+103\t0\t1\t150,150\tboundary fill black strip ring, red\n"
for x, y in pts: F.append(pre + f"105\t0\t-\t{x},{y}\tC4 default pens strip ring, probe {x},{y}\n")
open(os.path.join(sys.argv[1], "tmp-v64-fill2.tsv"), "w").write("".join(F))
