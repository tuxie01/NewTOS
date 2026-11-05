#!/usr/bin/env python3
"""wkcheck.py <vdprobe-out-dir>... -- analyse the V26 wrap records (cases named W*, k13 of vdprobe.c):
guards intact?, which data words of each form changed against the regenerated initial fill.
Prints one line per record; with several dirs prints, per record, whether all dirs agree."""
import struct, sys, os
GUARD = 32

def load(path):
    import gzip
    q = os.path.join(path, "VDPROBE.BIN")
    b = open(q, "rb").read() if os.path.exists(q) else gzip.open(q + ".gz").read()
    off = 6; out = []
    while off < len(b):
        no, = struct.unpack(">H", b[off+1:off+3]); name = b[off+3:off+11].rstrip(b"\0").decode()
        x, y, w, h = struct.unpack(">hhhh", b[off+11:off+19]); off += 19
        words = struct.unpack(">%dH" % (w*h), b[off:off+2*w*h]); off += 2*w*h
        out.append((no, name, words))
    return out

def fill(n, seed):
    buf = []
    for i in range(GUARD + n + GUARD):
        seed = (seed * 1103515245 + 12345) & 0xFFFFFFFF
        buf.append((seed >> 16) & 0xFFFF)
    for i in range(GUARD):
        buf[i] = buf[GUARD+n+i] = 0xA5A5
    return buf

def runs(idx):
    r = []
    for i in idx:
        if r and r[-1][1] == i-1: r[-1][1] = i
        else: r.append([i, i])
    return ",".join("%d-%d" % (a, b) if a != b else "%d" % a for a, b in r)

def initial(name, tag):
    # mirror of k13 geometry
    k = name[1]
    if k == 'a': sp,swd,sh,dp,dwd,dh = 16,3,4,16,1,(5 if name[2]=='1' else 4)
    elif k == 'b': sp,swd,sh,dp,dwd,dh = 16,1,4,16,3,1
    elif k == 'c': sp,swd,sh,dp,dwd,dh = 16,1,2,16,1,2
    elif k == 'd': sp,swd,sh,dp,dwd,dh = 1,3,4,16,1,(5 if name[2]=='1' else 4)
    elif k == 'e': pl=int(name[2]); sp,swd,sh,dp,dwd,dh = pl,3,4,pl,2,6
    ns, nd = swd*sp*sh, dwd*dp*dh
    if tag == 's': return fill(ns, (0x31415926 + sp*7 + swd) & 0xFFFFFFFF), ns
    return fill(nd, (0x27182818 + dp*13 + dwd) & 0xFFFFFFFF), nd

def analyse(path):
    res = {}
    for no, name, words in load(path):
        if not name.startswith("W"): continue
        base, tag = name[:-1], name[-1]
        ini, n = initial(name[:-1], tag)
        assert len(ini) == len(words), (name, len(ini), len(words))
        gd = [i for i in range(len(words)) if (i < GUARD or i >= GUARD+n) and words[i] != 0xA5A5]
        ch = [i-GUARD for i in range(GUARD, GUARD+n) if words[i] != ini[i]]
        res[name] = (len(gd), runs(ch), words)
    return res

if __name__ == "__main__":
    rs = [analyse(p) for p in sys.argv[1:]]
    for name in rs[0]:
        g, c, _ = rs[0][name]
        agree = all(r[name][2] == rs[0][name][2] for r in rs[1:])
        print("%-7s guard_words_changed=%d changed_data_words=%s%s" % (name, g, c or "-", "" if len(rs) == 1 else ("  ALL-EQUAL" if agree else "  DIFFER")))
