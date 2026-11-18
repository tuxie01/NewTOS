#!/usr/bin/env python3
"""handcheck.py <raster32.BIN> <raster32.tsv> <rscreen32.BIN> <rscreen32.tsv> -- the tester's own decoder: expected values written down
from ADR-038 / V64 P2-P3 (q = (c*255+500)/1000, pixel 0x00RRGGBB), not taken from contract32-check.py. Case id = line number of the table.
Big-endian dump; wide record 5744 bytes, BUF_A 1636, BUF_B 3684, 1024 WORDs; memory pixel k = WORD 2k (low) | WORD 2k+1 (high) << 16.
Screen readback: buf_b WORD 2*(row*32+col) = INTOUT[0], +1 = INTOUT[1]."""
import struct, re, sys
R, BA, BB = 5744, 1636, 3684
def load(binp, tsv):
    d = open(binp, "rb").read(); T = open(tsv).read().splitlines(); recs = {}
    for r in range((len(d) - 32) // R):
        o = 32 + r * R; recs[struct.unpack(">H", d[o:o+2])[0]] = d[o:o+R]
    def rec(tag):
        for i, l in enumerate(T, 1):
            if re.search(r"\t" + tag + r" ", l): return recs[i], T[i-1]
        raise SystemExit("no case " + tag)
    return rec
def W(rec, off): return struct.unpack(">1024H", rec[off:off+2048])
def mem(rec, off):
    w = W(rec, off); return [w[2*k] | (w[2*k+1] << 16) for k in range(512)]
def q(c): return (c * 255 + 500) // 1000
def px(r, g, b): return (q(r) << 16) | (q(g) << 8) | q(b)
ok = True
def check(name, cond, detail=""):
    global ok; ok &= bool(cond); print(("PASS " if cond else "FAIL ") + name + (" " + detail if detail else ""))
rec = load(sys.argv[1], sys.argv[2]); srec = load(sys.argv[3], sys.argv[4])
def region(p, r0, c0, r1, c1): return [p[r*32+c] for r in range(r0, r1+1) for c in range(c0, c1+1)]
r, _ = rec("M1");  check("1  M1  ALL_WHITE region (rows/cols 8..15) all 0x00000000", set(region(mem(r, BB), 8, 8, 15, 15)) == {0})
r, _ = rec("M16"); check("2  M16 ALL_BLACK region all 0xFFFFFFFF (X too)", set(region(mem(r, BB), 8, 8, 15, 15)) == {0xFFFFFFFF})
r, _ = rec("M4");  check("3  M4  S_ONLY region B(8..15)=A(0..7), all 32 bits", region(mem(r, BB), 8, 8, 15, 15) == region(mem(r, BA), 0, 0, 7, 7))
r, _ = rec("V5");  check("4  V5  pens 2/4 -> {00FF0000, 000000FF}", set(mem(r, BB)) == {px(1000,0,0), px(0,0,1000)} == {0x00FF0000, 0x000000FF})
r, _ = rec("V6");  check("5  V6  pens 8/9 (733/533 permille) -> {00BBBBBB, 00888888}", set(mem(r, BB)) == {px(733,733,733), px(533,533,533)} == {0xBBBBBB, 0x888888})
r, _ = rec("V8");  check("6  V8  pens 255/16 -> {00000000, 00FFFFFF}", set(mem(r, BB)) == {0, 0xFFFFFF})
r, _ = rec("V7");  check("7  V7  pens 999/-1 out of range -> pen 1 -> all 0x00000000", set(mem(r, BB)) == {0})
r, _ = rec("V4");  check("8  V4  MD_ERASE bg pen 6: clear bits 0x00FFFF00 present", 0x00FFFF00 in set(mem(r, BB)))
def rb(r, rows, cols):  # readback pairs
    w = W(r, BB); return [(w[2*(i*32+j)], w[2*(i*32+j)+1]) for i in range(rows) for j in range(cols)]
r, _ = srec("E2"); check("9  E2  screen->memory pen 5 (cyan): 512 x 0x0000FFFF", set(mem(r, BB)) == {0x0000FFFF})
r, _ = srec("E8"); check("10 E8  screen->memory pen 2 (red): 512 x 0x00FF0000", set(mem(r, BB)) == {0x00FF0000})
r, _ = srec("E1"); check("11 E1  screen->memory pen 1 (black): 512 x 0x00000000", set(mem(r, BB)) == {0})
r, _ = srec("F9"); check("12 F9  ALL_BLACK onto cleared screen reads {00FF,FFFF} (cleared = white)", set(rb(r, 16, 32)) == {(0x00FF, 0xFFFF)})
r, _ = srec("F21"); check("13 F21 16-plane source refused at 32: screen stays cleared {00FF,FFFF}", set(rb(r, 16, 32)) == {(0x00FF, 0xFFFF)} and r[836:840] == b"\xde\xad\xde\xad")
r, _ = srec("F1"); a = mem(r, BA); p = rb(r, 16, 32)
check("14 F1  round trip memory->screen->v_get_pixel: readback == A & 0x00FFFFFF, 512 pixels", [((x << 16) | y) for x, y in p] == [v & 0xFFFFFF for v in a])
r, _ = srec("H5"); p = set(rb(r, 16, 32)); check("15 H5  mono->screen red/blue read back {00FF,0000} / {0000,00FF}", p == {(0x00FF, 0x0000), (0x0000, 0x00FF)})
r, _ = srec("H7"); p = set(rb(r, 16, 32)); check("16 H7  MD_ERASE bg yellow: clear bits read {00FF,FF00}, set bits stay cleared {00FF,FFFF}", p == {(0x00FF, 0xFF00), (0x00FF, 0xFFFF)})
r, _ = srec("H9"); check("17 H9  out-of-range pens -> pen 1: all {0000,0000}", set(rb(r, 16, 32)) == {(0, 0)})
r, _ = rec("R1"); check("18 R1  16-plane memory forms refused at 32: d0 0, INTOUT[0..1] still 0xDEAD", struct.unpack(">i", r[4:8]) == (0,) and r[836:840] == b"\xde\xad\xde\xad")
print("ALL PASS" if ok else "SOME FAIL"); sys.exit(0 if ok else 1)
