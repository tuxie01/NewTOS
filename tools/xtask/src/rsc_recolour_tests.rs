//! A141 — HypView's resource patch `sdk/overlays/hypview/rsc/recolour-cicons.py`
//! must remap the 8-plane colour icons of upstream's `hyp_view.rsc` from the
//! NVDI palette to the TOS default palette, and change nothing else.
//!
//! The icons were drawn for NVDI; EmuTOS (and NewTOS) read a plane value `n`
//! as hardware colour register `n` of the default palette (`transform_cicon`,
//! `references/emutos/aes/gemrslib.c:348-396`), so index 252 shows brown where
//! the artist drew grey (`measurements/measurement-2026-11-07-a141/RUNINFO.TXT`).
//! The script maps register `n` → pen `REV_MAP_COL[n]` → `nvdi.pal[pen]` → the
//! register `m` whose default-palette colour is nearest (squared Euclidean in
//! VDI units, lowest `m` on a tie). Ticket `TASK/A141.md` (decision B2).
//!
//! This module parses the classic file on its own (not through
//! `tools/mkrsc.py`), so the byte-equality claim does not rest on the
//! script's parser: everything outside the 8-plane `col_data`/`sel_data`
//! rasters — header, tree, mono data and mask, labels, every 4-plane variant,
//! every colour mask — stays byte-equal, and every recoloured pixel takes the
//! nearest default colour. It also holds the two palette files of the patch
//! to their sources: `nvdi.pal` to FreeMiNT's, `tos-default.pal` to the
//! kernel's table and to EmuTOS' `videl_palette1/2`.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::workspace_root;

fn rsc_dir() -> PathBuf {
    workspace_root().join("sdk").join("overlays").join("hypview").join("rsc")
}

fn read(p: &Path) -> Vec<u8> {
    std::fs::read(p).unwrap_or_else(|e| panic!("cannot read {}: {e}", p.display()))
}

fn be16(d: &[u8], o: usize) -> u16 {
    u16::from_be_bytes([d[o], d[o + 1]])
}

fn be32(d: &[u8], o: usize) -> i32 {
    i32::from_be_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
}

/// A `PA01` palette file: 256 (r, g, b) big-endian WORDs by VDI pen
/// (XaAES `rw_syspalette`, `references/freemint/xaaes/src.km/trnfm.c:2345-2386`).
fn pal(d: &[u8]) -> Vec<[i64; 3]> {
    assert_eq!(&d[..4], b"PA01");
    assert_eq!(d.len(), 4 + 256 * 6);
    (0..256)
        .map(|i| {
            let o = 4 + i * 6;
            [be16(d, o) as i64, be16(d, o + 2) as i64, be16(d, o + 4) as i64]
        })
        .collect()
}

/// EmuTOS `REV_MAP_COL` on a 256-colour device (`vdi_col.c:28-29`, `:736-747`).
fn rev_map_col() -> [usize; 256] {
    let mut map: Vec<usize> = vec![0, 15, 1, 2, 4, 6, 3, 5, 7, 8, 9, 10, 12, 14, 11, 13];
    map.extend(16..256);
    map[1] = 255;
    map[255] = 15;
    let mut rev = [0usize; 256];
    for (pen, &reg) in map.iter().enumerate() {
        rev[reg] = pen;
    }
    rev
}

/// Every `(a, b, c)` / `{ a, b, c }` integer triple in `s`.
fn triples(s: &str, open: char, close: char) -> Vec<[i64; 3]> {
    s.split(open)
        .skip(1)
        .filter_map(|seg| {
            let inner = seg.split(close).next()?;
            let v: Vec<i64> = inner.split(',').filter_map(|x| x.trim().parse().ok()).collect();
            (v.len() == 3).then(|| [v[0], v[1], v[2]])
        })
        .collect()
}

/// The kernel's default palette, pens 0..255 (`kernel/src/vdi_backend.rs`
/// `VIDEL_PALETTE1` + `VIDEL_PALETTE2`).
fn kernel_palette() -> Vec<[i64; 3]> {
    let src = String::from_utf8(read(&workspace_root().join("kernel/src/vdi_backend.rs"))).unwrap();
    let table = |name: &str, n: usize| {
        let s = &src[src.find(&format!("const {name}:")).unwrap()..];
        let s = &s[s.find("= [").unwrap() + 3..];
        let t = triples(&s[..s.find("];").unwrap()], '(', ')');
        assert_eq!(t.len(), n, "{name}");
        t
    };
    let mut p = table("VIDEL_PALETTE1", 16);
    p.extend(table("VIDEL_PALETTE2", 240));
    p
}

/// EmuTOS' VDI colour source, workspace-relative (a gitignored input).
const EMUTOS_VDI_COL: &str = "references/emutos/vdi/vdi_col.c";

/// EmuTOS' `videl_palette1` + `videl_palette2`
/// (`references/emutos/vdi/vdi_col.c:130-199`).
fn emutos_palette() -> Vec<[i64; 3]> {
    let src = String::from_utf8(read(&workspace_root().join(EMUTOS_VDI_COL))).unwrap();
    let table = |name: &str, n: usize| {
        let s = &src[src.find(&format!("static const WORD {name}[")).unwrap()..];
        let s = &s[s.find('=').unwrap() + 1..];
        let s = &s[s.find('{').unwrap() + 1..];
        let t = triples(&s[..s.find("};").unwrap()], '{', '}');
        assert_eq!(t.len(), n, "{name}");
        t
    };
    let mut p = table("videl_palette1", 16);
    p.extend(table("videl_palette2", 240));
    p
}

/// One CICON variant's rasters as classic file offsets: `(planes, mono_words,
/// col_data, col_mask, sel_data, sel_mask)`, `sel_*` 0 when absent.
type Variant = (usize, usize, usize, usize, usize, usize);

/// Walk the CICONBLK extension of a new-format classic `.rsc`
/// (`get_ciconblkptr` / `fixup_all_ciconblks` / `fixup_colour_icons`,
/// `references/emutos/aes/gemrslib.c:484-508`, `:226-256`, `:185-217`).
fn variants(d: &[u8]) -> Vec<Variant> {
    let vrsn = be16(d, 0);
    assert!(vrsn & 4 != 0, "NEW_FORMAT_RSC");
    let rssize = be16(d, 34) as usize;
    let cptr = be32(d, rssize + 4);
    assert!(cptr > 0, "CICON table present");
    let mut p = cptr as usize;
    let mut n = 0;
    while be32(d, p) != -1 {
        n += 1;
        p += 4;
    }
    p += 4;
    let mut out = Vec::new();
    for _ in 0..n {
        let w = be16(d, p + 12 + 10) as usize; // ib_wicon (ICONBLK word 5)
        let h = be16(d, p + 12 + 12) as usize; // ib_hicon (word 6)
        let count = be32(d, p + 34) as usize; // mainlist = number of CICONs
        let mw = (w / 16) * h;
        let mut q = p + 38 + 2 * mw * 2 + 12; // head, mono data + mask, label
        for _ in 0..count {
            let planes = be16(d, q) as usize;
            let sel_flag = be32(d, q + 2 + 8) != 0;
            q += 22;
            let col_data = q;
            q += mw * planes * 2;
            let col_mask = q;
            q += mw * 2;
            let (mut sel_data, mut sel_mask) = (0, 0);
            if sel_flag {
                sel_data = q;
                q += mw * planes * 2;
                sel_mask = q;
                q += mw * 2;
            }
            out.push((planes, mw, col_data, col_mask, sel_data, sel_mask));
        }
        p = q;
    }
    out
}

/// The colour index of every pixel of an 8-plane standard-form raster
/// (plane `p` = bit `p`, plane-sequential, MSB leftmost).
fn indices8(d: &[u8], off: usize, mw: usize) -> Vec<usize> {
    let mut v = Vec::with_capacity(mw * 16);
    for i in 0..mw {
        for bit in 0..16 {
            let mut n = 0;
            for p in 0..8 {
                if be16(d, off + (p * mw + i) * 2) & (0x8000 >> bit) != 0 {
                    n |= 1 << p;
                }
            }
            v.push(n);
        }
    }
    v
}

fn recolour(src: &Path, out: &Path) {
    let python = std::env::var_os("PYTHON").unwrap_or_else(|| "python3".into());
    let script = rsc_dir().join("recolour-cicons.py");
    let o = Command::new(&python)
        .arg(&script)
        .arg("recolour")
        .arg(src)
        .arg(out)
        .output()
        .unwrap_or_else(|e| panic!("cannot run {python:?} {}: {e}", script.display()));
    assert!(
        o.status.success(),
        "recolour-cicons.py failed: {}\n{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
}

fn dist2(a: [i64; 3], b: [i64; 3]) -> i64 {
    (0..3).map(|i| (a[i] - b[i]).pow(2)).sum()
}

/// The patch's palettes are their sources: `nvdi.pal` byte-equal to
/// FreeMiNT's, `tos-default.pal` equal to the kernel's default table and to
/// EmuTOS' `videl_palette1/2`.
#[test]
fn recolour_palettes_equal_their_sources() {
    const FREEMINT_PAL: &str = "references/freemint/xaaes/src.km/pal/nvdi.pal";
    if !crate::test_inputs::require_paths(
        "rsc_recolour_tests::recolour_palettes_equal_their_sources",
        &[FREEMINT_PAL, EMUTOS_VDI_COL],
    ) {
        return;
    }
    let nvdi = read(&rsc_dir().join("nvdi.pal"));
    let freemint = read(&workspace_root().join(FREEMINT_PAL));
    assert_eq!(nvdi, freemint, "nvdi.pal is a verbatim copy");
    let tos = pal(&read(&rsc_dir().join("tos-default.pal")));
    assert_eq!(tos, kernel_palette(), "tos-default.pal = kernel/src/vdi_backend.rs");
    assert_eq!(tos, emutos_palette(), "tos-default.pal = EmuTOS vdi_col.c");
}

/// Upstream's `hyp_view.rsc` (vendored, `sdk/apps/hypview-sources.sha256`):
/// the three registers the ticket names land on the TOS colours the artist
/// meant, every recoloured pixel takes the nearest default colour, and every
/// byte outside the 8-plane data rasters is unchanged.
#[test]
fn hypview_rsc_8plane_icons_take_the_nearest_tos_colour() {
    const HYP_VIEW_RSC: &str = "sdk/apps/hypview/hyp_view.rsc";
    if !crate::test_inputs::require_paths(
        "rsc_recolour_tests::hypview_rsc_8plane_icons_take_the_nearest_tos_colour",
        &[HYP_VIEW_RSC],
    ) {
        return;
    }
    let src = workspace_root().join(HYP_VIEW_RSC);
    let dir = workspace_root().join("target").join("xtask-rsc-recolour");
    std::fs::create_dir_all(&dir).unwrap();
    let out = dir.join("hyp_view-recoloured.rsc");
    let _ = std::fs::remove_file(&out);
    recolour(&src, &out);
    let a = read(&src);
    let b = read(&out);
    assert_eq!(a.len(), b.len(), "same length");

    let vs = variants(&a);
    assert_eq!(vs.len(), 29, "29 CICON variants (15 blocks)");
    assert_eq!(vs.iter().filter(|v| v.0 == 8).count(), 14, "14 of them 8-plane");
    assert_eq!(vs.iter().filter(|v| v.0 == 4).count(), 15, "15 of them 4-plane");

    // Byte-equality outside the 8-plane data rasters.
    let mut changeable = vec![false; a.len()];
    for &(planes, mw, col_data, _, sel_data, _) in &vs {
        if planes == 8 {
            for off in [col_data, sel_data] {
                if off != 0 {
                    changeable[off..off + mw * 16].iter_mut().for_each(|c| *c = true);
                }
            }
        }
    }
    let foreign: Vec<usize> = (0..a.len()).filter(|&i| !changeable[i] && a[i] != b[i]).collect();
    assert!(foreign.is_empty(), "bytes changed outside the 8-plane data: {:?}", &foreign[..foreign.len().min(16)]);

    // Every pixel: the register's NVDI colour, replaced by the nearest TOS one.
    let rev = rev_map_col();
    let nvdi = pal(&read(&rsc_dir().join("nvdi.pal")));
    let tos = pal(&read(&rsc_dir().join("tos-default.pal")));
    let reg = |p: &[[i64; 3]], n: usize| p[rev[n]];
    let mut named = [0usize; 3];
    for &(planes, mw, col_data, _, sel_data, _) in &vs {
        if planes != 8 {
            continue;
        }
        for off in [col_data, sel_data] {
            if off == 0 {
                continue;
            }
            let before = indices8(&a, off, mw);
            let after = indices8(&b, off, mw);
            for (&n, &m) in before.iter().zip(&after) {
                let c = reg(&nvdi, n);
                let best = (0..256).map(|k| dist2(c, reg(&tos, k))).min().unwrap();
                assert_eq!(dist2(c, reg(&tos, m)), best, "register {n} -> {m} is not a nearest colour");
                assert_eq!(m, (0..256).find(|&k| dist2(c, reg(&tos, k)) == best).unwrap(), "tie: lowest register");
                // The ticket's three: NVDI grey 753 / 949 and brown (400,200,0).
                match n {
                    252 => {
                        assert_eq!((m, reg(&tos, m)), (7, [733, 733, 733]));
                        named[0] += 1;
                    }
                    250 => {
                        assert_eq!((m, reg(&tos, m)), (17, [933, 933, 933]));
                        named[1] += 1;
                    }
                    93 => {
                        assert_eq!((m, reg(&tos, m)), (227, [467, 200, 0]));
                        named[2] += 1;
                    }
                    _ => {}
                }
            }
        }
    }
    // The pixel counts of RUNINFO.TXT's table count the pixels under both
    // variants' masks; these are every pixel of the 8-plane data.
    assert_eq!(named, [1383, 656, 103], "pixels of registers 252, 250, 93");
    assert_ne!(a, b, "the 8-plane data did change");
}
