#!/usr/bin/env python3
"""rawmap.py <run-a> <run-b> <case> [x0 x1 y0 y1] -- one vdprobe case as raw-word maps.
Symbols: '.' 0xffff (the v_clrwk white, nothing written since), ',' 0xffdf (pen 0, a cell
background written), '#' 0x0000 (pen 1), '?' anything else. The two whites differ in the
overlay bit, so the map shows exactly which pixels a text cell wrote. Prints A, B, and
the pixels that differ."""
import sys, os, struct
sys.path.insert(0, os.path.join(os.path.dirname(__file__)))
def load(path):
    b = open(os.path.join(path, "VDPROBE.BIN"), "rb").read()
    off = 6; out = {}
    while off < len(b):
        no, = struct.unpack(">H", b[off+1:off+3])
        name = b[off+3:off+11].rstrip(b"\0").decode()
        x, y, w, h = struct.unpack(">hhhh", b[off+11:off+19]); off += 19
        words = struct.unpack(">%dH" % (w*h), b[off:off+2*w*h]); off += 2*w*h
        out[name] = (no, x, y, w, h, words)
    return out
SYM = {0xffff: '.', 0xffdf: ',', 0x0000: '#', 0xf800: 'r'}
def show(lab, c, x0, x1, y0, y1):
    no, x, y, w, h, ws = c
    print(lab, "case", no, "rect", x, y, w, h, "x", x0, "..", x1)
    for r in range(y0, y1+1):
        print("%4d %s" % (r, "".join(SYM.get(ws[(r-y)*w+(q-x)], '?') for q in range(x0, x1+1))))
if __name__ == "__main__":
    a, b, name = sys.argv[1:4]
    ca, cb = load(a)[name], load(b)[name]
    _, x, y, w, h, wa = ca; wb = cb[5]
    if len(sys.argv) > 4:
        x0, x1, y0, y1 = map(int, sys.argv[4:8])
    else:
        d = [(i % w, i // w) for i in range(w*h) if wa[i] != 0xffff or wb[i] != 0xffff]
        x0, x1 = x+min(p[0] for p in d)-1, x+max(p[0] for p in d)+1
        y0, y1 = y+min(p[1] for p in d)-1, y+max(p[1] for p in d)+1
        x0, y0 = max(x0, x), max(y0, y); x1, y1 = min(x1, x+w-1), min(y1, y+h-1)
    show("E", ca, x0, x1, y0, y1); show("N", cb, x0, x1, y0, y1)
    print("D")
    for r in range(y0, y1+1):
        print("%4d %s" % (r, "".join('*' if wa[(r-y)*w+(q-x)] != wb[(r-y)*w+(q-x)] else ' ' for q in range(x0, x1+1))))
