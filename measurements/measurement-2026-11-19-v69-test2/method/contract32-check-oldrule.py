#!/usr/bin/env python3
"""contract32-check.py -- a dump taken at vdiplanes=32 against the NewTOS contract (TASK/V69).

usage: contract32-check.py <table.tsv> <dump.BIN>
       contract32-check.py --selftest

EmuTOS has no path above 16 planes (references/emutos/vdi/vdi_defs.h:22), so
a dump taken at NewTOS' 32-plane setting has no oracle to be compared with:
what it must show is the contract of ADR-038 (docs/MEMORY.md) and of the
32-plane sections of docs/api/vdi/raster/_raster.md (V63 R1-R6, V64 P1-P9,
V65 sections 2-6). This script lays those values beside every case and
compares them with the recorded words -- the check of the wide mode that
docs/TOS-REFERENCE-DUMPER.md ("The wide record and the readback") asks for
before a NEWTOS32- baseline is frozen. NewTOS contract, never "oracle".

Two kinds of table are checked:

  * a WIDE raster table (`!screen w,h,32`, tosdump/core/cases.h, "THE
    32-PLANE SETTING"; dump area 7 VDI-RASTER32, record 5744 bytes). Every
    case is computed from its own line: the buffers from their seeds
    (td_rast_fill_word, tosdump/core/record.c), the preludes and the
    measured call through a model of the contract, the readback as
    v_get_pixel answers it. The model knows the 32-plane chunky form, the
    1-plane source of vrt_cpyfm, the 32-plane vr_trnfm and every refusal of
    V65 section 2; a planar (1..8-plane) result is NOT modelled and such a
    case is refused (exit 2), never guessed.
  * the VDI table vdi-a131-palette.tsv run at 32 (area 4 VDI), which the
    spec uses as it is: vq_color requested and realised (V64 P4, P5, P7)
    and the pixel of a solid v_bar read by v_get_pixel (V65 section 6).
    Recognised by the CRC32 of the table file alone (VDI_CONTRACT_TABLES),
    never by what its cases look like: every other table run at 32 ends
    with exit 3, not checked.

THE X RULE ("paint before you copy"). The byte above R, G, B of a screen
pixel that no pen wrote is implementation-defined (V63 R3), and a raw
screen-to-memory copy carries it (V65 section 3). The model tracks which
bits of every pixel are known; a recorded memory word that depends on an
unknown bit is a fault of the TABLE (exit 2), so no frozen 32-plane dump can
pin one implementation's X. v_get_pixel masks X, so a readback is always
known.

What else every raster record must show, unchanged from the 16-plane
contract (the dispatcher's rules; no 32-plane rule changes them): d0's low
word 0, contrl[2] and contrl[4] written 0, the input arrays as laid over the
0xDEAD poison, INTOUT and PTSOUT untouched, both MFDBs' eight WORDs as the
case laid them (vr_trnfm at 32 does not write the destination's fd_stand,
V65 section 5), the MFDB pointer slots unchanged by the call.

Exit 0 when every case matches, 1 on a difference, 2 on a usage, parse or
table error, 3 when the table is neither kind (nothing to check).
"""
import struct
import sys
import zlib

POISON = 0xDEAD
FULL = 0xFFFFFFFF
SCREEN_KNOWN = 0x00FFFFFF          # a cleared screen pixel: RGB white, X unknown
SCREEN_CLEARED = 0x00FFFFFF

# ---- the contract's numbers -------------------------------------------------

# The default pens' requested triples (permille), VIDEL's palette:
# references/emutos/vdi/vdi_col.c:129-135 videl_palette1 (pens 0..15) and
# :136 videl_palette2[0] (pen 16) and [239] (pen 255). Only the pens the
# spec's table of expected values names (docs/TOS-REFERENCE-DUMPER.md, "The
# expected values at 32"); any other pen has no value here and is refused.
PENS = {
    0: (1000, 1000, 1000), 1: (0, 0, 0), 2: (1000, 0, 0), 3: (0, 1000, 0),
    4: (0, 0, 1000), 5: (0, 1000, 1000), 6: (1000, 1000, 0), 7: (1000, 0, 1000),
    8: (733, 733, 733), 9: (533, 533, 533), 10: (667, 0, 0), 11: (0, 667, 0),
    12: (0, 0, 667), 13: (0, 667, 667), 14: (667, 667, 0), 15: (667, 0, 667),
    16: (1000, 1000, 1000), 255: (0, 0, 0),
}
NCOLORS = 256                      # DEV_TAB[13] at 32 planes, ADR-038 point 3


def q8(c):
    """V64 P2: a gun in permille to 8 bits."""
    return (c * 255 + 500) // 1000


def realised(q):
    """V64 P5: the realised gun of an 8-bit value."""
    return (q * 1000 + 127) // 255


def validate(pen):
    """validate_color_index (references/emutos/vdi/vdi_control.c:224): out of
    range answers pen 1."""
    return pen if 0 <= pen < NCOLORS else 1


class TableError(Exception):
    pass


def pen_pixel(pen):
    """V64 P3: the pen's 32-bit pixel 0x00RRGGBB (X = 0, V63 R2)."""
    if pen not in PENS:
        raise TableError("pen %d has no contract value in this checker" % pen)
    r, g, b = (q8(c) for c in PENS[pen])
    return (r << 16) | (g << 8) | b


def fill_word(seed, index):
    """td_rast_fill_word (tosdump/core/record.c), bit for bit."""
    x = (index * 1103515245 + 12345) & 0xFFFFFFFF
    x ^= (seed * 2654435761) & 0xFFFFFFFF
    x ^= x >> 15
    x = (x * 2246822519) & 0xFFFFFFFF
    x ^= x >> 13
    return x & 0xFFFF


def logic_op(op, s, d):
    """The 16 logic ops, bitwise on all 32 bits (V63 R4, V65 section 3;
    references/emutos/vdi/vdi_raster.c:1008-1220): (value, which inputs it
    reads)."""
    table = {
        0: (0, ''), 1: (s & d, 'sd'), 2: (s & ~d, 'sd'), 3: (s, 's'),
        4: (~s & d, 'sd'), 5: (d, 'd'), 6: (s ^ d, 'sd'), 7: (s | d, 'sd'),
        8: (~(s | d), 'sd'), 9: (~(s ^ d), 'sd'), 10: (~d, 'd'), 11: (s | ~d, 'sd'),
        12: (~s, 's'), 13: (~s | d, 'sd'), 14: (~(s & d), 'sd'), 15: (FULL, ''),
    }
    v, reads = table[op]
    return v & FULL, reads


def combine(op, sv, sm, dv, dm):
    v, reads = logic_op(op, sv, dv)
    m = FULL
    if 's' in reads:
        m &= sm
    if 'd' in reads:
        m &= dm
    return v, m


# ---- the table --------------------------------------------------------------

def parse_value(t):
    t = t.strip()
    neg = t.startswith('-')
    if neg:
        t = t[1:]
    v = int(t, 16) if t[:2].lower() == '0x' else int(t, 10)
    v = -v if neg else v
    return ((v + 0x8000) & 0xFFFF) - 0x8000      # a WORD, as the loader stores it


def parse_list(t):
    t = t.strip()
    if t == '-' or t == '':
        return []
    return [parse_value(x) for x in t.split(',')]


def read_table(raw):
    """(lines, crc): lines = [(line_no, kind, columns)] with kind '!', '+',
    'case'; comments and blank lines dropped (they still count)."""
    out = []
    text = raw.decode('latin-1')
    for no, line in enumerate(text.split('\n'), 1):
        line = line[:-1] if line.endswith('\r') else line
        s = line.lstrip(' \t')
        if s == '' or s.startswith('#'):
            continue
        if s.startswith('!'):
            out.append((no, '!', line.split('\t')))
            continue
        kind = '+' if s.startswith('+') else 'case'
        out.append((no, kind, line.split('\t')))
    if text.endswith('\n'):
        pass
    return out, zlib.crc32(raw) & 0xFFFFFFFF


class Mfdb:
    def __init__(self, col, planes_screen):
        col = col.strip()
        if col == 'S':
            self.screen = True
            self.buffer = None
            self.w = self.h = self.wd = self.stand = 0
            self.planes = planes_screen
            self.words = [0] * 8
        else:
            v = parse_list(col)
            self.screen = False
            self.buffer, self.w, self.h, self.wd, self.stand, self.planes = v
            self.words = [self.w, self.h, self.wd, self.stand, self.planes, 0, 0, 0]


# ---- the model --------------------------------------------------------------

class Model:
    """One case's world: two buffers of WORDs, the screen, the clip, the fill
    pen. Every pixel value travels with the mask of its known bits."""

    def __init__(self, seed_a, seed_b, nwords):
        self.buf = [[fill_word(seed_a, i) for i in range(nwords)],
                    [fill_word(seed_b, i) for i in range(nwords)]]
        self.known = [[True] * nwords, [True] * nwords]
        self.screen = {}
        self.clip = None
        self.fill_pen = 1                   # tosdump/core/runner.c, td_reset_vdi

    # screen
    def sget(self, x, y):
        return self.screen.get((x, y), (SCREEN_CLEARED, SCREEN_KNOWN))

    def sput(self, x, y, v, m):
        if self.clip is not None:
            x1, y1, x2, y2 = self.clip
            if not (x1 <= x <= x2 and y1 <= y <= y2):
                return
        self.screen[(x, y)] = (v & FULL, m)

    # memory, 32-plane chunky form (V65 section 3): pixel k is WORDs 2k
    # (low) and 2k+1 (high) on AArch64
    def mget32(self, f, x, y):
        k = y * f.wd * 16 + x
        b, kn = self.buf[f.buffer], self.known[f.buffer]
        v = b[2 * k] | (b[2 * k + 1] << 16)
        return v, (FULL if (kn[2 * k] and kn[2 * k + 1]) else 0)

    def mput32(self, f, x, y, v, m):
        k = y * f.wd * 16 + x
        b, kn = self.buf[f.buffer], self.known[f.buffer]
        b[2 * k] = v & 0xFFFF
        b[2 * k + 1] = (v >> 16) & 0xFFFF
        # a WORD is known when every bit of its half is
        kn[2 * k] = (m & 0xFFFF) == 0xFFFF
        kn[2 * k + 1] = ((m >> 16) & 0xFFFF) == 0xFFFF

    def mono_bit(self, f, x, y):
        w = self.buf[f.buffer][y * f.wd + (x >> 4)]
        return (w >> (15 - (x & 15))) & 1

    def get(self, f, x, y):
        return self.sget(x, y) if f.screen else self.mget32(f, x, y)

    def put(self, f, x, y, v, m):
        if f.screen:
            self.sput(x, y, v, m)
        else:
            self.mput32(f, x, y, v, m)


def sort_rect(p):
    return (min(p[0], p[2]), min(p[1], p[3]), max(p[0], p[2]), max(p[1], p[3]))


def copy_geometry(ptsin, dst, model):
    """The (source, destination) pixel pairs of a 109/121 copy: the source
    rectangle sorted (arb_corner), the destination at the sorted PTSIN[4..7]
    minimum with the SOURCE extent (dont_clip,
    references/emutos/vdi/vdi_raster.c:882-900); a screen destination
    clipped when the clip is on, the visible part keeping its source offset
    (do_clip, :809-877)."""
    sx1, sy1, sx2, sy2 = sort_rect(ptsin[0:4])
    dx1, dy1 = min(ptsin[4], ptsin[6]), min(ptsin[5], ptsin[7])
    pairs = []
    for j in range(sy2 - sy1 + 1):
        for i in range(sx2 - sx1 + 1):
            dx, dy = dx1 + i, dy1 + j
            if dst.screen and model.clip is not None:
                cx1, cy1, cx2, cy2 = model.clip
                if not (cx1 <= dx <= cx2 and cy1 <= dy <= cy2):
                    continue
            pairs.append((sx1 + i, sy1 + j, dx, dy))
    return pairs


def unmodelled(what):
    raise TableError("not modelled (a planar result has no 32-plane contract value here): " + what)


def vro_cpyfm(model, intin, ptsin, src, dst):
    """vro_cpyfm at the 32-plane setting (V65 section 2): refusals first,
    then the op on 32-bit pixels, the source read before any write."""
    mode = intin[0] & 0xFFFF if intin else POISON
    if mode & ~0x1F:
        return                               # vdi_raster.c:1340-1342
    sp, dp = src.planes, dst.planes
    if dp > 32 or sp != dp:
        return                               # cap = depth; s_nxwd != d_nxwd
    if dp <= 8:
        unmodelled("vro_cpyfm of %d-plane forms" % dp)
    if dp != 32:
        return                               # 9..31 refused (O5)
    if mode & 0x10:
        return                               # PAT_FLAG, truecolor path
    op = mode & 0x0F
    pairs = copy_geometry(ptsin, dst, model)
    srcs = [model.get(src, sx, sy) for sx, sy, _, _ in pairs]
    dsts = [model.get(dst, dx, dy) for _, _, dx, dy in pairs]
    for (sx, sy, dx, dy), (sv, sm), (dv, dm) in zip(pairs, srcs, dsts):
        v, m = combine(op, sv, sm, dv, dm)
        model.put(dst, dx, dy, v, m)


def vrt_cpyfm(model, intin, ptsin, src, dst):
    """vrt_cpyfm at the 32-plane setting (V65 sections 2 and 4)."""
    mode = intin[0] & 0xFFFF if intin else POISON
    if mode & ~0x1F:
        return
    m4 = mode & ~0x10
    if m4 not in (1, 2, 3, 4):
        return                               # default: return
    if src.screen or src.planes != 1:
        return                               # source must be mono
    fg = validate(intin[1] if len(intin) > 1 else POISON - 0x10000)
    bg = validate(intin[2] if len(intin) > 2 else POISON - 0x10000)
    dp = dst.planes
    if not dst.screen:
        if dp <= 8:
            unmodelled("vrt_cpyfm into a %d-plane form" % dp)
        if dp != 32:
            return                           # 9..31, > 32 refused
    if mode & 0x10:
        return                               # PAT_FLAG, truecolor path
    fgp, bgp = pen_pixel(fg), pen_pixel(bg)
    for sx, sy, dx, dy in copy_geometry(ptsin, dst, model):
        bit = model.mono_bit(src, sx, sy)
        dv, dm = model.get(dst, dx, dy)
        if m4 == 1:
            model.put(dst, dx, dy, fgp if bit else bgp, FULL)
        elif m4 == 2:
            if bit:
                model.put(dst, dx, dy, fgp, FULL)
        elif m4 == 3:
            if bit:
                model.put(dst, dx, dy, ~dv, dm)
        elif m4 == 4:
            if not bit:
                model.put(dst, dx, dy, bgp, FULL)


def vr_trnfm(model, src, dst):
    """vr_trnfm at the 32-plane setting (V65 section 5): a 32-plane source is
    transformed (plane p = bit 31 - p), 9..31 is a no-op, the destination's
    fd_stand is not written."""
    sp = src.planes
    if sp <= 8:
        unmodelled("vr_trnfm of a %d-plane form" % sp)
    if sp != 32:
        return
    ps = src.h * src.wd                      # WORDs per plane, from the source
    n = ps * 16                              # pixels
    sb = list(model.buf[src.buffer])         # read before any write (in place)
    out = [0] * (ps * 32)
    if src.stand != 0:                       # standard -> device
        for k in range(n):
            pix = 0
            for p in range(32):
                if (sb[p * ps + k // 16] >> (15 - (k & 15))) & 1:
                    pix |= 1 << (31 - p)
            out[2 * k] = pix & 0xFFFF
            out[2 * k + 1] = pix >> 16
    else:                                    # device -> standard
        for k in range(n):
            pix = sb[2 * k] | (sb[2 * k + 1] << 16)
            for p in range(32):
                if (pix >> (31 - p)) & 1:
                    out[p * ps + k // 16] |= 1 << (15 - (k & 15))
    for i, w in enumerate(out):
        model.buf[dst.buffer][i] = w
        model.known[dst.buffer][i] = True


def vr_recfl(model, ptsin):
    x1, y1, x2, y2 = sort_rect(ptsin[0:4])
    pix = pen_pixel(validate(model.fill_pen))
    for y in range(y1, y2 + 1):
        for x in range(x1, x2 + 1):
            model.sput(x, y, pix, FULL)


def run_call(model, op, intin, ptsin, src, dst):
    if op == 109:
        vro_cpyfm(model, intin, ptsin, src, dst)
    elif op == 121:
        vrt_cpyfm(model, intin, ptsin, src, dst)
    elif op == 110:
        vr_trnfm(model, src, dst)
    elif op == 25:
        model.fill_pen = intin[0]
    elif op == 114:
        vr_recfl(model, ptsin)
    elif op == 129:
        model.clip = sort_rect(ptsin[0:4]) if intin[0] else None
    else:
        raise TableError("opcode %d not modelled" % op)


# ---- the records ------------------------------------------------------------

RAST32_RECORD = 5744
RAST32_BUF_A, RAST32_BUF_B, RAST32_BUF_WORDS = 1636, 3684, 1024
VDI_RECORD = 1592


def words(rec, off, n):
    return list(struct.unpack('>%dH' % n, rec[off:off + 2 * n]))


def header(b):
    if len(b) < 32 or b[:4] != b'TDMP':
        raise TableError("not a tosdump file")
    version, area, rsize, count = struct.unpack('>HHHH', b[4:12])
    crc, = struct.unpack('>I', b[16:20])
    return area, rsize, count, crc


QUIET = False        # the selftest's own forged dumps print no case lines


class Report:
    def __init__(self):
        self.diffs = 0
        self.cases = 0
        self.bad_cases = set()

    def diff(self, case, what, want, got):
        if case not in self.bad_cases:
            self.bad_cases.add(case)
        self.diffs += 1
        if self.diffs <= 400 and not QUIET:
            print("  case %d: %s: contract 0x%04X, recorded 0x%04X" % (case, what, want, got))


def check_params(rep, cid, rec, op, intin, ptsin, src, dst):
    ret_lo = struct.unpack('>H', rec[6:8])[0]
    if ret_lo != 0:
        rep.diff(cid, "d0", 0, ret_lo)
    c, cb = words(rec, 8, 15), words(rec, 38, 15)
    want_before = [op, len(ptsin) // 2, POISON, len(intin), POISON, 0]
    want_after = [op, len(ptsin) // 2, 0, len(intin), 0, 0]
    for i in range(6):
        if cb[i] != want_before[i] & 0xFFFF:
            rep.diff(cid, "contrl_before[%d]" % i, want_before[i] & 0xFFFF, cb[i])
        if c[i] != want_after[i] & 0xFFFF:
            rep.diff(cid, "contrl[%d]" % i, want_after[i] & 0xFFFF, c[i])
    for i in range(6, 15):                   # the handle and the MFDB pointers
        if c[i] != cb[i]:
            rep.diff(cid, "contrl[%d] changed by the call" % i, cb[i], c[i])
    for name, off, n, vals in (("intin", 68, 128, intin), ("ptsin", 324, 256, ptsin)):
        got = words(rec, off, n)
        for i in range(n):
            want = (vals[i] & 0xFFFF) if i < len(vals) else POISON
            if got[i] != want:
                rep.diff(cid, "%s[%d]" % (name, i), want, got[i])
    for name, off, n in (("intout", 836, 128), ("ptsout", 1092, 256)):
        for i, g in enumerate(words(rec, off, n)):
            if g != POISON:
                rep.diff(cid, "%s[%d] (untouched)" % (name, i), POISON, g)
    for name, off, m in (("src_mfdb", 1604, src), ("dst_mfdb", 1620, dst)):
        for i, g in enumerate(words(rec, off, 8)):
            if g != m.words[i] & 0xFFFF:
                rep.diff(cid, "%s[%d]" % (name, i), m.words[i] & 0xFFFF, g)


def check_raster32(lines, b):
    area, rsize, count, crc = header(b)
    if area != 7 or rsize != RAST32_RECORD:
        raise TableError("a wide table's dump is area 7, record 5744 bytes; this one is area %d, %d bytes"
                         % (area, rsize))
    cases, pending = [], []
    for no, kind, col in lines:
        if kind == '!':
            continue
        if len(col) < 7:
            raise TableError("line %d: seven columns" % no)
        op = parse_value(col[0].strip().lstrip('+'))
        call = (op, parse_list(col[1]), parse_list(col[2]), col[3], col[4], col[5])
        if kind == '+':
            pending.append(call)
        else:
            cases.append((no, call, pending))
            pending = []
    if len(cases) != count:
        raise TableError("the table has %d cases, the dump %d records" % (len(cases), count))
    rep = Report()
    for idx, (no, (op, intin, ptsin, scol, dcol, fcol), pre) in enumerate(cases):
        rec = b[32 + idx * rsize:32 + (idx + 1) * rsize]
        cid = struct.unpack('>H', rec[0:2])[0]
        if cid != no:
            raise TableError("record %d is case %d, the table has %d there" % (idx, cid, no))
        rep.cases += 1
        fill = parse_list(fcol)
        model = Model(fill[0] & 0xFFFF, fill[1] & 0xFFFF, RAST32_BUF_WORDS)
        for pop, pin, ppt, ps, pd, _ in pre:
            psrc = Mfdb(ps, 32) if ps.strip() != '-' else None
            pdst = Mfdb(pd, 32) if pd.strip() != '-' else None
            run_call(model, pop, pin, ppt, psrc, pdst)
        src, dst = Mfdb(scol, 32), Mfdb(dcol, 32)
        run_call(model, op, intin, ptsin, src, dst)
        if len(fill) == 6:                   # the readback, through v_get_pixel
            rx, ry, rw, rh = fill[2:6]
            for r in range(rh):
                for c in range(rw):
                    v, m = model.sget(rx + c, ry + r)
                    if (m & 0x00FFFFFF) != 0x00FFFFFF:
                        raise TableError("line %d: the readback depends on unknown bits" % no)
                    pix = v & 0x00FFFFFF
                    i = 2 * (r * 32 + c)
                    model.buf[1][i], model.buf[1][i + 1] = pix >> 16, pix & 0xFFFF
                    model.known[1][i] = model.known[1][i + 1] = True
        check_params(rep, cid, rec, op, intin, ptsin, src, dst)
        for bi, (name, off) in enumerate((("buf_a", RAST32_BUF_A), ("buf_b", RAST32_BUF_B))):
            got = words(rec, off, RAST32_BUF_WORDS)
            for i in range(RAST32_BUF_WORDS):
                if not model.known[bi][i]:
                    raise TableError("line %d: %s[%d] depends on the unused byte of a screen pixel no pen "
                                     "wrote -- paint the source first (the X rule)" % (no, name, i))
                if got[i] != model.buf[bi][i]:
                    rep.diff(cid, "%s[%d]" % (name, i), model.buf[bi][i], got[i])
    return rep


def check_palette(lines, b):
    """vdi-a131-palette.tsv at 32: vq_color (26) and v_get_pixel (105)."""
    area, rsize, count, crc = header(b)
    if area != 4 or rsize != VDI_RECORD:
        raise TableError("a VDI table's dump is area 4, record 1592 bytes")
    cases, pen = [], None
    for no, kind, col in lines:
        if kind == '!':
            continue
        op = parse_value(col[0].strip().lstrip('+'))
        intin = parse_list(col[2])
        if kind == '+':
            if op == 25:
                pen = intin[0]
            continue
        cases.append((no, op, intin, pen))
        pen = None
    if len(cases) != count:
        raise TableError("the table has %d cases, the dump %d records" % (len(cases), count))
    rep = Report()
    for idx, (no, op, intin, fill_pen) in enumerate(cases):
        rec = b[32 + idx * rsize:32 + (idx + 1) * rsize]
        cid = struct.unpack('>H', rec[0:2])[0]
        if cid != no:
            raise TableError("record %d is case %d, the table has %d there" % (idx, cid, no))
        rep.cases += 1
        io = words(rec, 800, 4)
        if op == 26:
            p, flag = intin[0], intin[1]
            rgb = PENS.get(p)
            if rgb is None:
                raise TableError("line %d: pen %d has no contract value" % (no, p))
            want = rgb if flag == 0 else tuple(realised(q8(c)) for c in rgb)
            for k in range(3):
                if io[1 + k] != want[k]:
                    rep.diff(cid, "intout[%d] (vq_color pen %d flag %d)" % (1 + k, p, flag), want[k], io[1 + k])
        elif op == 105:
            pix = pen_pixel(validate(fill_pen if fill_pen is not None else 1))
            for k, want in ((0, pix >> 16), (1, pix & 0xFFFF)):
                if io[k] != want:
                    rep.diff(cid, "intout[%d] (v_get_pixel, pen %s)" % (k, fill_pen), want, io[k])
    return rep


# The VDI tables this checker has contract values for, bound by the CRC32 of
# the whole table file -- the identity every dump of it carries (header bytes
# 16-19) and tosdiff refuses a mismatch on. check_palette's rule (a v_get_pixel
# reads the pixel of the v_bar its own preludes painted in their own pen) holds
# for this one table only; no other table, and no edited copy of this one, may
# be judged by it (TASK/V69, the tester's finding F1: vdi-v18.tsv, also all
# vq_color/v_get_pixel cases, was judged by it and reported false violations).
VDI_CONTRACT_TABLES = {
    0x7E23929E: "vdi-a131-palette.tsv",
}


def wide_planes(lines):
    for no, kind, col in lines:
        if kind == '!' and col[0].strip() == '!screen' and len(col) > 1:
            v = parse_list(col[1])
            return len(v) == 3 and v[2] == 32
    return False


def classify(lines, crc, area):
    """Which contract the table is checked against: 'wide' for a raster table
    that declares a 32-plane screen (its own explicit marker, cases.h, "THE
    32-PLANE SETTING"; every case of it is modelled or refused), 'palette' for
    a VDI table named in VDI_CONTRACT_TABLES by its CRC, None for any other --
    nothing to check."""
    if wide_planes(lines):
        return 'wide'
    try:
        ops = {parse_value(c[0].strip()) for no, kind, c in lines if kind == 'case'}
    except ValueError:
        ops = set()
    if area == 4 and ops and ops <= {26, 105}:
        return 'palette'
    return None


def check(raw, b, name):
    """(exit code, kind, report) of a table's bytes and a dump's bytes."""
    lines, crc = read_table(raw)
    area, rsize, count, dcrc = header(b)
    if dcrc != crc:
        raise TableError("the dump carries case table CRC %08X, %s is %08X" % (dcrc, name, crc))
    kind = classify(lines, crc, area)
    if kind == 'wide':
        return "wide raster table (area 7 VDI-RASTER32)", check_raster32(lines, b)
    if kind == 'palette':
        return "VDI palette table", check_palette(lines, b)
    return None, None


def selftest():
    """The contract's numbers the checker is built on, against the spec."""
    fails = 0

    def eq(what, got, want):
        nonlocal fails
        if got != want:
            print("  FAIL %s: %r, want %r" % (what, got, want))
            fails += 1
    # docs/TOS-REFERENCE-DUMPER.md, "The expected values at 32"
    for pen, want in ((0, 0xFFFFFF), (1, 0), (2, 0xFF0000), (3, 0x00FF00), (4, 0x0000FF), (5, 0x00FFFF),
                      (6, 0xFFFF00), (7, 0xFF00FF), (8, 0xBBBBBB), (9, 0x888888), (10, 0xAA0000),
                      (11, 0x00AA00), (12, 0x0000AA), (13, 0x00AAAA), (14, 0xAAAA00), (15, 0xAA00AA),
                      (16, 0xFFFFFF), (255, 0)):
        eq("pen %d" % pen, pen_pixel(pen), want)
    for c, a in ((667, 667), (733, 733), (533, 533), (1000, 1000), (0, 0)):
        eq("realised %d" % c, realised(q8(c)), a)
    eq("validate 999", validate(999), 1)
    eq("validate -1", validate(-1), 1)
    eq("validate 255", validate(255), 255)
    # td_rast_fill_word: words the C host test records (test_record.c uses
    # seed 1 index 5); recomputed here from the C formula's constants
    eq("fill differs by index", fill_word(1, 0) != fill_word(1, 1), True)
    eq("fill differs by seed", fill_word(1, 0) != fill_word(2, 0), True)
    # vr_trnfm: standard -> device -> standard is the identity, X included
    m = Model(7, 8, RAST32_BUF_WORDS)
    before = list(m.buf[0])
    f = Mfdb("0,32,16,2,1,32", 32)
    vr_trnfm(m, f, Mfdb("0,32,16,2,0,32", 32))
    vr_trnfm(m, Mfdb("0,32,16,2,0,32", 32), f)
    eq("vr_trnfm identity", m.buf[0], before)
    # the X rule: a cleared screen pixel copied raw to memory is not known
    m = Model(1, 2, RAST32_BUF_WORDS)
    vro_cpyfm(m, [3], [0, 0, 0, 0, 0, 0, 0, 0], Mfdb("S", 32), Mfdb("1,32,16,2,0,32", 32))
    eq("X of a cleared pixel", (m.known[1][0], m.known[1][1]), (True, False))
    # a v_get_pixel of a cleared pixel: {0x00FF, 0xFFFF}
    eq("cleared pixel", m.sget(5, 5)[0] & 0xFFFFFF, 0xFFFFFF)
    # which tables are checked at all (the tester's finding F1): of the
    # shipped tables only the two wide ones and vdi-a131-palette.tsv; every
    # other one -- vdi-v18.tsv, whose cases are all v_get_pixel, among them --
    # has no contract values here and is never judged by the palette rule
    import glob
    import os
    cases = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'cases')
    want = {'vdi-a131-palette.tsv': 'palette', 'vdi-raster32.tsv': 'wide', 'vdi-raster-screen32.tsv': 'wide'}
    seen = 0
    for path in sorted(glob.glob(os.path.join(cases, '*.tsv'))):
        name = os.path.basename(path)
        lines, crc = read_table(open(path, 'rb').read())
        for area in (4, 5, 6, 7):
            got = classify(lines, crc, area)
            exp = want.get(name) if (want.get(name) == 'wide' or area == 4) else None
            if got != exp:
                eq("%s as area %d" % (name, area), got, exp)
        seen += 1
    eq("shipped tables classified", seen >= 40, True)
    # end to end: a VDI dump of vdi-v18.tsv whose v_get_pixel cases answer
    # the white pixel at 32, {0x00FF, 0xFFFF} -- the run the tester saw
    # reported red -- is "not checked", not a contract difference
    raw = open(os.path.join(cases, 'vdi-v18.tsv'), 'rb').read()
    lines, crc = read_table(raw)
    ncase = sum(1 for _, k, _ in lines if k == 'case')
    hdr = b'TDMP' + struct.pack('>HHHH', 3, 4, VDI_RECORD, ncase) + b'test' + struct.pack('>IHH', crc, 8, 2) + bytes(8)
    recs = b''
    for no, k, col in lines:
        if k == 'case':
            rec = bytearray(VDI_RECORD)
            rec[0:4] = struct.pack('>HH', no, parse_value(col[0]) & 0xFFFF)
            rec[800:804] = struct.pack('>HH', 0x00FF, 0xFFFF)
            recs += bytes(rec)
    eq("vdi-v18.tsv at 32 is not judged", check(raw, hdr + recs, 'vdi-v18.tsv')[0], None)
    # and the palette table's rule still reports a wrong pixel (the negative
    # control): the same white answer for every case of vdi-a131-palette.tsv
    raw = open(os.path.join(cases, 'vdi-a131-palette.tsv'), 'rb').read()
    lines, crc = read_table(raw)
    ncase = sum(1 for _, k, _ in lines if k == 'case')
    hdr = b'TDMP' + struct.pack('>HHHH', 3, 4, VDI_RECORD, ncase) + b'test' + struct.pack('>IHH', crc, 8, 2) + bytes(8)
    recs = b''
    for no, k, col in lines:
        if k == 'case':
            rec = bytearray(VDI_RECORD)
            rec[0:4] = struct.pack('>HH', no, parse_value(col[0]) & 0xFFFF)
            rec[800:804] = struct.pack('>HH', 0x00FF, 0xFFFF)
            recs += bytes(rec)
    global QUIET
    QUIET = True
    kind, rep = check(raw, hdr + recs, 'vdi-a131-palette.tsv')
    QUIET = False
    eq("vdi-a131-palette.tsv is the palette table", kind is not None and 'palette' in kind, True)
    eq("a wrong palette dump is reported", rep is not None and len(rep.bad_cases) > 0, True)
    print("  ok" if fails == 0 else "  %d failure(s)" % fails)
    return 1 if fails else 0


def main(argv):
    if len(argv) == 2 and argv[1] == '--selftest':
        return selftest()
    if len(argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    try:
        raw = open(argv[1], 'rb').read()
        b = open(argv[2], 'rb').read()
        kind, rep = check(raw, b, argv[1])
        if kind is None:
            print("contract32-check: %s has no NewTOS-contract values here (neither a wide raster table nor "
                  "a VDI table of VDI_CONTRACT_TABLES): not checked" % argv[1])
            return 3
    except (TableError, OSError, ValueError, IndexError) as e:
        print("contract32-check: %s" % e, file=sys.stderr)
        return 2
    print("NewTOS contract (ADR-038, 32-plane setting), no oracle: %s, %s" % (argv[1], kind))
    print("%d of %d cases differ from the NewTOS contract" % (len(rep.bad_cases), rep.cases))
    return 1 if rep.bad_cases else 0


if __name__ == '__main__':
    sys.exit(main(sys.argv))
