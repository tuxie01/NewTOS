#!/usr/bin/env python3
"""NewTOS patch of HypView's resource (freemint/hypview @ 6f3eeeb, hyp_view.rsc).

Why (TASK/A141, GitHub #23): the 8-plane colour icons of upstream's
hyp_view.rsc were drawn for the NVDI palette, not for the TOS one. EmuTOS --
and NewTOS with it -- takes a plane value n of a colour icon as hardware
colour register n of the standard palette (`transform_cicon`,
references/emutos/aes/gemrslib.c:348-396; `phys_work.ext->palette[n]` is
written at `palette[MAP_COL[pen]]`, `set_color16`,
references/emutos/vdi/vdi_col.c:601-614). Under the TOS palette index 252
is brown where the artist meant grey, 250 olive for near-white, 93 bright
green for dark brown (measurements/measurement-2026-11-07-a141/RUNINFO.TXT).
XaAES remaps such icons at load time (`icn_pal_name = nvdi`,
references/freemint/xaaes/src.km/example.cnf:609-630); EmuTOS does not, and
the kernel stays EmuTOS. So the port adapts the icons once, at build time:
the user's decision B2 in TASK/A141.md.

What changes: in every CICON variant with num_planes == 8, each pixel of
col_data and (when present) sel_data. Nothing else -- not the masks, not the
4-plane or other variants, not the mono data, not the tree -- so the output
differs from the input only inside those 8-plane data rasters.

The mapping, per colour register n = 0..255 of an 8-plane icon:
  1. the register's VDI pen: pen = REV_MAP_COL[n] (EmuTOS `init_colors`,
     vdi_col.c:736-747, on a 256-colour device: MAP_COL_ROM with
     MAP_COL[1] = 255, identity 16..254, MAP_COL[255] = 15, inverted). A
     palette file is by VDI pen (XaAES `set_syspalette` hands entry i to
     vs_color(i), references/freemint/xaaes/src.km/trnfm.c:2389-2401), and
     XaAES reads an icon's plane value through the same device->VDI table
     before it looks the colour up (`devtovdi8` via `tovdilut`,
     trnfm.c:148-165, :2075-2087).
  2. the colour the artist meant: nvdi.pal[pen] (RGB, 0..1000 per gun).
  3. the replacement register m: the register whose colour in the TOS
     default palette, tos-default.pal[REV_MAP_COL[m]], is NEAREST to it.

NEAREST (the metric): the smallest squared Euclidean distance
  (r1 - r2)^2 + (g1 - g2)^2 + (b1 - b2)^2
over the three guns in VDI units (0..1000, the files' own values, no
quantisation and no weighting), taken over all 256 registers m = 0..255;
on a tie the LOWEST register number wins. Ties are real: NVDI's colour cube
steps by 200, EmuTOS' by 67, so e.g. (600, 400, 0) lies exactly between
(733, 400, 0) and (467, 400, 0). The summary printed on every run lists
them.

Inputs, both files in this directory, origins in palettes.sha256:
  nvdi.pal          FreeMiNT's NVDI palette, verbatim copy
  tos-default.pal   the NewTOS/EmuTOS default palette, written from the
                    kernel's table by the `tos-pal` command below
Format of both (XaAES `rw_syspalette`, trnfm.c:2345-2386): "PA01", then
256 x (red, green, blue) big-endian WORDs 0..1000, by VDI pen; 1540 bytes.

The CICON walk is tools/mkrsc.py's `read_cicon_table` (the same parser the
converter uses); this file only rewrites pixels in place.

Usage:
  recolour-cicons.py recolour <upstream classic .rsc> <out classic .rsc>
  recolour-cicons.py tos-pal <kernel/src/vdi_backend.rs> <out .pal>
"""
import importlib.util
import os
import re
import struct
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", "..", "..", ".."))

PAL_MAGIC = b"PA01"
PAL_SIZE = 4 + 256 * 6

# EmuTOS MAP_COL_ROM, VDI pen -> hardware register (vdi_col.c:28-29).
MAP_COL_ROM = (0, 15, 1, 2, 4, 6, 3, 5, 7, 8, 9, 10, 12, 14, 11, 13)


def load_mkrsc():
    spec = importlib.util.spec_from_file_location(
        "mkrsc", os.path.join(ROOT, "tools", "mkrsc.py"))
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return m


def rev_map_col():
    """Hardware register -> VDI pen on a 256-colour device (`init_colors`)."""
    map_col = list(MAP_COL_ROM) + list(range(16, 256))
    map_col[1] = 255           # MAP_COL[1] = numcolors - 1
    map_col[255] = 15          # EXTENDED_PALETTE: MAP_COL[MAXCOLOURS-1] = 15
    rev = [0] * 256
    for pen, reg in enumerate(map_col):
        rev[reg] = pen
    return rev


def read_pal(path):
    p = open(path, "rb").read()
    if len(p) != PAL_SIZE or p[:4] != PAL_MAGIC:
        sys.exit(f"{path}: not a 256-colour PA01 palette ({len(p)} bytes)")
    return [struct.unpack(">3H", p[4 + 6 * i:10 + 6 * i]) for i in range(256)]


def dist2(a, b):
    return sum((a[i] - b[i]) ** 2 for i in range(3))


def build_lut(src_pal, dst_pal):
    """Register n -> register m (module doc: NEAREST). Returns (lut, ties)
    with ties[n] = every register at the minimal distance, for n with more
    than one."""
    rev = rev_map_col()
    dst = [dst_pal[rev[m]] for m in range(256)]
    lut, ties = [], {}
    for n in range(256):
        c = src_pal[rev[n]]
        d = [dist2(c, dst[m]) for m in range(256)]
        best = min(d)
        cands = [m for m in range(256) if d[m] == best]
        lut.append(cands[0])       # lowest register on a tie
        if len(cands) > 1:
            ties[n] = cands
    return lut, ties


def recolour_raster(d, off, mono_words, lut, used):
    """Rewrite one 8-plane standard-form raster in place: plane p is bit p,
    plane-sequential, big-endian WORDs, MSB = leftmost pixel."""
    n = mono_words
    words = list(struct.unpack(">%dH" % (8 * n), d[off:off + 16 * n]))
    out = [0] * (8 * n)
    for i in range(n):
        for bit in range(16):
            mask = 0x8000 >> bit
            v = 0
            for p in range(8):
                if words[p * n + i] & mask:
                    v |= 1 << p
            used[v] = used.get(v, 0) + 1
            w = lut[v]
            for p in range(8):
                if w & (1 << p):
                    out[p * n + i] |= mask
    d[off:off + 16 * n] = struct.pack(">%dH" % (8 * n), *out)


def recolour(src, dst):
    mkrsc = load_mkrsc()
    d = bytearray(open(src, "rb").read())
    hdr = struct.unpack(">18H", bytes(d[:36]))
    blocks = mkrsc.read_cicon_table(bytes(d), hdr[0], hdr[17])
    nvdi = read_pal(os.path.join(HERE, "nvdi.pal"))
    tos = read_pal(os.path.join(HERE, "tos-default.pal"))
    lut, ties = build_lut(nvdi, tos)
    used = {}
    rasters = 0
    for words, _pdata, _pmask, _ptext, variants in blocks:
        mono_words = (words[5] // 16) * words[6]
        for planes, col_data, _col_mask, sel_data, _sel_mask in variants:
            if planes != 8:
                continue
            for off in (col_data, sel_data):
                if off:
                    recolour_raster(d, off, mono_words, lut, used)
                    rasters += 1
    open(dst, "wb").write(bytes(d))
    rev = rev_map_col()
    print(f"recolour-cicons: {len(blocks)} CICONBLKs, {rasters} 8-plane rasters "
          f"recoloured (nvdi.pal -> tos-default.pal)")
    print(f"recolour-cicons: lut: {len(set(lut))} distinct targets for 256 registers, "
          f"{sum(lut[n] == n for n in range(256))} unchanged, {len(ties)} tied")
    print(f"recolour-cicons: {len(used)} registers used in the icons:")
    for n in sorted(used):
        tie = f"  tie {ties[n]}" if n in ties else ""
        print(f"  {n:3d} x{used[n]:5d}  nvdi{nvdi[rev[n]]} -> {lut[n]:3d} "
              f"tos{tos[rev[lut[n]]]}{tie}")


def tos_pal(rs, out):
    """Write the kernel's default palette (pens 0..15 VIDEL_PALETTE1, 16..255
    VIDEL_PALETTE2, kernel/src/vdi_backend.rs) as a PA01 file."""
    src = open(rs).read()

    def table(name, count):
        m = re.search(r"const %s: \[\(u16, u16, u16\); %d\] = \[(.*?)\];" % (name, count),
                      src, re.S)
        if not m:
            sys.exit(f"{rs}: table {name} not found")
        t = [tuple(int(x) for x in g)
             for g in re.findall(r"\((\d+),\s*(\d+),\s*(\d+)\)", m.group(1))]
        if len(t) != count:
            sys.exit(f"{rs}: {name} has {len(t)} entries, expected {count}")
        return t

    pal = table("VIDEL_PALETTE1", 16) + table("VIDEL_PALETTE2", 240)
    data = PAL_MAGIC + b"".join(struct.pack(">3H", *c) for c in pal)
    open(out, "wb").write(data)


def main():
    if len(sys.argv) == 4 and sys.argv[1] == "recolour":
        recolour(sys.argv[2], sys.argv[3])
    elif len(sys.argv) == 4 and sys.argv[1] == "tos-pal":
        tos_pal(sys.argv[2], sys.argv[3])
    else:
        sys.exit(__doc__.split("Usage:")[1])


if __name__ == "__main__":
    main()
