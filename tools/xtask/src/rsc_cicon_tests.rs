//! A41 — `tools/mkrsc.py convert` must carry the colour icons of a
//! new-format (`NEW_FORMAT_RSC`, `rsdefs.h:39`) classic `.rsc` into NRSC
//! v11/v12 instead of refusing the file.
//!
//! On disk (EmuTOS `fix_cicons` → `fixup_all_ciconblks` →
//! `fixup_colour_icons`, `aes/gemrslib.c:549-573`, `:226-256`, `:185-217`)
//! a CICONBLK is a variable-length record: a 38-byte head (three placeholder
//! pointers, the eleven ICONBLK WORDs, `mainlist` = the number of CICONs),
//! `mono_words = ib_wicon/16 × ib_hicon` WORDs of mono data, the same of
//! mono mask, a fixed 12-byte label, then the CICONs: a 22-byte head
//! (`num_planes`, four placeholder pointers of which `sel_data` is a flag,
//! `next_res` = 1 while another variant follows), `mono_words × num_planes`
//! WORDs of data, `mono_words` of mask, and the same pair again for the
//! SELECTED image when flagged. The table of CICONBLK pointers is reached
//! through the extension array at `rsh_rssize` (`get_ciconblkptr`,
//! `:484-508`) and ends in a `-1` sentinel; a `G_CICON`'s `ob_spec` is the
//! 0-based index into it (`fix_objects`, `:623-626`).
//!
//! The native record is the application's C struct — `CICONBLK { ICONBLK;
//! CICON *mainlist; }` and `CICON { num_planes; col_data; col_mask;
//! sel_data; sel_mask; next_res; }` (`obdefs.h:203-217`, gemlib
//! `mt_gem.h`; LP64 56 + 48 bytes, ILP32 40 + 24) — because TeraDesk copies
//! a CICONBLK by value (`sdk/apps/teradesk/icon.c:1675`) and reads
//! `monoblk` through `ob_spec.ciconblk` (`dir.c:2090-2102`). Every pointer
//! field holds a blob offset (0 = absent), `mainlist`/`next_res` link the
//! variants, the rasters are byte-swapped to little-endian words like every
//! other raster, and a `G_CICON`'s `ob_spec` becomes the byte offset of its
//! CICONBLK. The 84-byte v11/v12 header adds the two array bases + counts.
//!
//! Part 2 (NRSC v13/v14): the file **begins with a native `RSHDR`** — the
//! 18 LE `u16` of the classic header (`rsdefs.h:16-36` order), offsets from
//! the file's first byte, padded to 40 bytes — and the NRSC header follows
//! at offset 40. EmuTOS publishes the loaded file as `ap_rscmem`
//! (`gemrslib.c:787`) and TeraDesk's `xrsrc_gaddr` parses that header on
//! its own (`sdk/apps/teradesk/rsc_load.c:496-541`: `IS_XRSC_HEADER` =
//! `rsh_vrsn & 3 == 3` must be false, then `rsh_trindex`/`rsh_object`/… as
//! offsets from `ap_rscmem`). The tests read the preamble back the way that
//! parser does and hold it to the NRSC header behind it.
//!
//! The first tests drive the tool over a **hand-built** two-icon file whose
//! every byte is known, so the expected native offsets are hand-checkable;
//! the last one converts TeraDesk's real `cicons.rsc` (38 icons) and holds
//! its native size against the kernel's `RSC_CAP`. Spec:
//! `docs/api/aes/resource-fileselector.md` §5.9–§5.10 (with the record
//! decision above superseding §5.9's table proposal); ticket `TASK/A41.md`.
//! The loader half is `tos-api/src/aes/resource.rs` (`cicon_tests`).

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::workspace_root;

/// `tos-api/src/aes/resource.rs::NRSC_VER_RSHDR` (13 LP64, 14 ILP32): the
/// generation `convert` emits — v11/v12's header behind a native `RSHDR`.
const NRSC_VER_RSHDR: u32 = 13;
/// `tos-api/src/aes/resource.rs::RSC_CAP`.
const RSC_CAP: usize = 64 * 1024;
/// `tos-api/src/aes/resource.rs::RSHDR_PAD`: the 36-byte native `RSHDR`
/// padded to 40; the NRSC magic sits at this offset.
const RSHDR_PAD: usize = 40;
/// The converter's own bound on the image (`mkrsc.py`: `RSC_CAP - RSHDR_PAD`).
const IMAGE_CAP: usize = RSC_CAP - RSHDR_PAD;

/// NRSC v13 header: the preamble, then magic + 20 u32.
const HDR: usize = RSHDR_PAD + 84;

const G_BOX: u16 = 20;
const G_CICON: u16 = 33;
const LASTOB: u16 = 0x0020;

// ── the hand-built classic file ───────────────────────────────────────────

const W: usize = 16;
const H: usize = 16;
const MONO_WORDS: usize = (W / 16) * H; // 16

/// Icon A's 1-plane data, mono data/mask and the 4-plane planes: asymmetric
/// words (the two bytes differ) so a missing byte swap shows.
fn word_a(r: usize) -> u16 {
    0x8001u16.rotate_left(r as u32) ^ ((r as u16) << 3)
}
fn word_b(p: usize, r: usize) -> u16 {
    (0x0F0Fu16.rotate_left((p * 3 + r) as u32)) ^ 0x00A5
}

/// A classic big-endian new-format `.rsc`: a root `G_BOX`, two `G_CICON`
/// objects (indices 0 and 1), two CICONBLKs — block 0 with a 1-plane and a
/// 4-plane-with-SELECTED variant, block 1 with none.
fn classic() -> Vec<u8> {
    let be16 = |v: u16| v.to_be_bytes();
    let be32 = |v: u32| v.to_be_bytes();
    let mut d = Vec::new();
    // Header (18 × u16), filled in at the end.
    d.resize(36, 0);
    // OBJECTs @36 (24 bytes each).
    let obj = |d: &mut Vec<u8>, next: i16, head: i16, tail: i16, ty: u16, flags: u16, spec: u32, x: u16, y: u16, w: u16, h: u16| {
        for v in [next as u16, head as u16, tail as u16, ty, flags, 0u16] {
            d.extend_from_slice(&be16(v));
        }
        d.extend_from_slice(&be32(spec));
        for v in [x, y, w, h] {
            d.extend_from_slice(&be16(v));
        }
    };
    let p_obj = d.len();
    obj(&mut d, -1, 1, 2, G_BOX, 0, 0x0000_1100, 0, 0, 0x0050, 0x0019);
    obj(&mut d, 2, -1, -1, G_CICON, 0, 0, 0x0102, 0x0001, 0x4800, 0x2800);
    obj(&mut d, 0, -1, -1, G_CICON, LASTOB, 1, 0x000C, 0x0001, 0x4800, 0x2800);
    // Tree index @108: one LONG, the root's offset.
    let p_tri = d.len();
    d.extend_from_slice(&be32(p_obj as u32));
    let rssize = d.len(); // 112
    // Extension array: true length (patched), CICON table offset, 0.
    let p_ext = d.len();
    d.extend_from_slice(&be32(0));
    d.extend_from_slice(&be32((p_ext + 12) as u32));
    d.extend_from_slice(&be32(0));
    // CICONBLK pointer table: two placeholders + sentinel.
    d.extend_from_slice(&be32(0));
    d.extend_from_slice(&be32(0));
    d.extend_from_slice(&be32(u32::MAX));
    // CICONBLK 0.
    let cib = |d: &mut Vec<u8>, n_cicons: u32, label: &[u8], data: u16, mask: u16| {
        for _ in 0..3 {
            d.extend_from_slice(&be32(0)); // placeholder pointers
        }
        for v in [0x1041u16, 0, 0, 0, 0, W as u16, H as u16, 0, H as u16, 72, 8] {
            d.extend_from_slice(&be16(v));
        }
        d.extend_from_slice(&be32(n_cicons));
        for _ in 0..MONO_WORDS {
            d.extend_from_slice(&be16(data));
        }
        for _ in 0..MONO_WORDS {
            d.extend_from_slice(&be16(mask));
        }
        let mut lbl = [0u8; 12];
        lbl[..label.len()].copy_from_slice(label);
        d.extend_from_slice(&lbl);
    };
    cib(&mut d, 2, b"ICON A", 0x0FF0, 0xFFFF);
    // CICON A: 1 plane, no SELECTED, another follows.
    d.extend_from_slice(&be16(1));
    for v in [0u32, 0, 0, 0, 1] {
        d.extend_from_slice(&be32(v));
    }
    for r in 0..MONO_WORDS {
        d.extend_from_slice(&be16(word_a(r)));
    }
    for _ in 0..MONO_WORDS {
        d.extend_from_slice(&be16(0xFFFF));
    }
    // CICON B: 4 planes, SELECTED flagged, last.
    d.extend_from_slice(&be16(4));
    for v in [0u32, 0, 1, 0, 0] {
        d.extend_from_slice(&be32(v));
    }
    for p in 0..4 {
        for r in 0..MONO_WORDS {
            d.extend_from_slice(&be16(word_b(p, r)));
        }
    }
    for _ in 0..MONO_WORDS {
        d.extend_from_slice(&be16(0xF0F0));
    }
    for p in 0..4 {
        for r in 0..MONO_WORDS {
            d.extend_from_slice(&be16(!word_b(p, r)));
        }
    }
    for _ in 0..MONO_WORDS {
        d.extend_from_slice(&be16(0x0F0F));
    }
    // CICONBLK 1: mono only — and a label using all 12 bytes, no NUL.
    cib(&mut d, 0, b"TWELVE CHARS", 0xF00F, 0xFFFF);
    let total = d.len();
    d[p_ext..p_ext + 4].copy_from_slice(&be32(total as u32));
    // Header: vrsn 4 (NEW_FORMAT_RSC), all sections at `p_obj` except the
    // tree index, counts, rssize.
    let hdr: [u16; 18] = [
        4,
        p_obj as u16,
        p_obj as u16,
        p_obj as u16,
        p_obj as u16,
        p_obj as u16,
        p_obj as u16,
        p_obj as u16,
        p_obj as u16,
        p_tri as u16,
        3,
        1,
        0,
        0,
        0,
        0,
        0,
        rssize as u16,
    ];
    for (i, v) in hdr.iter().enumerate() {
        d[i * 2..i * 2 + 2].copy_from_slice(&be16(*v));
    }
    d
}

fn le16(d: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([d[o], d[o + 1]])
}
fn le32(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(d[o..o + 4].try_into().unwrap())
}
fn le64(d: &[u8], o: usize) -> u64 {
    u64::from_le_bytes(d[o..o + 8].try_into().unwrap())
}

/// A pointer field at `o` at the blob's ABI width (odd version = LP64).
fn ptr(d: &[u8], version: u32, o: usize) -> u64 {
    if version % 2 == 1 {
        le64(d, o)
    } else {
        le32(d, o) as u64
    }
}

fn scratch_dir(name: &str) -> PathBuf {
    let dir = workspace_root().join("target").join("xtask-rsc-cicon").join(name);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

fn run_mkrsc(dir: &Path, mode: Option<&str>, src: &Path, out: &Path) -> std::process::Output {
    let root = workspace_root();
    let script = root.join("tools").join("mkrsc.py");
    let python = std::env::var_os("PYTHON").unwrap_or_else(|| "python3".into());
    let mut c = Command::new(&python);
    c.current_dir(dir).arg(&script).arg("convert");
    if let Some(m) = mode {
        c.arg(m);
    }
    c.arg(src)
        .arg(out)
        .output()
        .unwrap_or_else(|e| panic!("cannot run {python:?} {}: {e}", script.display()))
}

/// Header fields of an NRSC v13+ blob by index (`magic` + `u32[i]`, behind
/// the `RSHDR` preamble).
fn field(d: &[u8], i: usize) -> u32 {
    assert_eq!(&d[RSHDR_PAD..RSHDR_PAD + 4], b"NRSC", "NRSC magic at {RSHDR_PAD}");
    le32(d, RSHDR_PAD + 4 + i * 4)
}

/// Field `i` of the native `RSHDR` preamble (18 LE `u16`, classic order:
/// 0 `rsh_vrsn`, 1 `rsh_object`, 2 `rsh_tedinfo`, 3 `rsh_iconblk`,
/// 4 `rsh_bitblk`, 5 `rsh_frstr`, 6 `rsh_string`, 7 `rsh_imdata`,
/// 8 `rsh_frimg`, 9 `rsh_trindex`, 10 `rsh_nobs`, 11 `rsh_ntree`,
/// 12 `rsh_nted`, 13 `rsh_nib`, 14 `rsh_nbb`, 15 `rsh_nstring`,
/// 16 `rsh_nimages`, 17 `rsh_rssize`).
fn rsh(d: &[u8], i: usize) -> usize {
    le16(d, i * 2) as usize
}

/// TeraDesk's reading of the preamble (`xrsrc_gaddr`) against the NRSC
/// header: not an `RSXHDR`, every offset and count the NRSC header's, the
/// size the image's, and `_rs_trindex[0]` — the slot at `rsh_trindex` — the
/// offset of tree 0's root OBJECT (= the tree index's first slot).
fn check_rshdr(d: &[u8], vrsn: usize) {
    let version = field(d, 0);
    assert_eq!(rsh(d, 0), vrsn, "rsh_vrsn is the classic file's");
    assert_ne!(rsh(d, 0) & 3, 3, "IS_XRSC_HEADER must be false (u16 fields)");
    assert_eq!(rsh(d, 1), field(d, 4) as usize, "rsh_object = off_objects");
    assert_eq!(rsh(d, 2), field(d, 10) as usize, "rsh_tedinfo = off_ted");
    assert_eq!(rsh(d, 3), field(d, 12) as usize, "rsh_iconblk = off_ib");
    assert_eq!(rsh(d, 4), field(d, 14) as usize, "rsh_bitblk = off_bb");
    assert_eq!(rsh(d, 5), field(d, 6) as usize, "rsh_frstr = off_frstr");
    assert_eq!(rsh(d, 8), field(d, 8) as usize, "rsh_frimg = off_frimg");
    assert_eq!(rsh(d, 9), field(d, 3) as usize, "rsh_trindex = off_trindex");
    assert_eq!(rsh(d, 10), field(d, 2) as usize, "rsh_nobs = n_objects");
    assert_eq!(rsh(d, 11), field(d, 1) as usize, "rsh_ntree = n_trees");
    assert_eq!(rsh(d, 12), field(d, 11) as usize, "rsh_nted = n_ted");
    assert_eq!(rsh(d, 13), field(d, 13) as usize, "rsh_nib = n_ib");
    assert_eq!(rsh(d, 14), field(d, 15) as usize, "rsh_nbb = n_bb");
    assert_eq!(rsh(d, 15), field(d, 7) as usize, "rsh_nstring = n_frstr");
    assert_eq!(rsh(d, 16), field(d, 9) as usize, "rsh_nimages = n_frimg");
    assert_eq!(rsh(d, 17), field(d, 5) as usize, "rsh_rssize = total_size");
    assert_eq!(rsh(d, 17), d.len(), "rsh_rssize = the file size");
    // The data section the two informational fields name lies in the image.
    let ptr = if version % 2 == 1 { 8 } else { 4 };
    let data = field(d, 8) as usize + field(d, 9) as usize * ptr;
    assert_eq!(rsh(d, 6), data, "rsh_string = the data section");
    assert_eq!(rsh(d, 7), data, "rsh_imdata = the data section");
    assert!(data <= d.len());
    // `_rs_trindex[0]` at `rsh_trindex`: tree 0's root, i.e. the OBJECT array.
    let slot0 = ptr_at(d, version, rsh(d, 9));
    assert_eq!(slot0, field(d, 4) as u64, "_rs_trindex[0] = the root OBJECT's offset");
    // The preamble's pad and the alignment behind it.
    assert!(d[36..RSHDR_PAD].iter().all(|&b| b == 0), "pad bytes 36..40 are zero");
    assert_eq!(field(d, 4) % 8, 0, "the OBJECT array stays 8-aligned behind the preamble");
}

/// A pointer slot at `o` at the blob's ABI width (odd version = LP64).
fn ptr_at(d: &[u8], version: u32, o: usize) -> u64 {
    ptr(d, version, o)
}

/// The native layout of the hand-built file, per ABI.
struct Layout {
    version: u32,
    ps: usize, // pointer size
    obj: usize,
    cib: usize,
    cic: usize,
    ib: usize, // ICONBLK size = `mainlist` offset
}

fn check_hand_built(tag: &str, mode: Option<&str>, lay: Layout) {
    let dir = scratch_dir(tag);
    let src = dir.join("two.rsc");
    std::fs::write(&src, classic()).unwrap();
    let out = dir.join("two.nrsc");
    let o = run_mkrsc(&dir, mode, &src, &out);
    assert!(
        o.status.success(),
        "mkrsc.py convert {mode:?} must accept a new-format .rsc: {}\n{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    let d = std::fs::read(&out).unwrap();
    assert_eq!(&d[RSHDR_PAD..RSHDR_PAD + 4], b"NRSC", "NRSC magic behind the RSHDR");
    let version = field(&d, 0);
    assert_eq!(version, lay.version, "version");
    check_rshdr(&d, 4); // the hand-built file is new-format: rsh_vrsn 4
    assert_eq!(field(&d, 2), 3, "n_objects");
    let off_obj = field(&d, 4) as usize;
    let total = field(&d, 5) as usize;
    assert_eq!(total, d.len());
    let (off_cib, n_cib, off_cic, n_cic) =
        (field(&d, 16) as usize, field(&d, 17), field(&d, 18) as usize, field(&d, 19));
    assert_eq!(n_cib, 2, "two CICONBLKs");
    assert_eq!(n_cic, 2, "two CICONs in all");
    // The arrays follow the (empty) ICONBLK array and precede the (empty)
    // free-string table; both record sizes keep the pointer alignment.
    assert_eq!(off_cib, field(&d, 12) as usize, "CICONBLKs right after the ICONBLKs");
    assert_eq!(off_cic, off_cib + 2 * lay.cib);
    assert_eq!(field(&d, 6) as usize, off_cic + 2 * lay.cic, "frstr table right after the CICONs");
    assert_eq!(off_cib % lay.ps, 0);
    assert_eq!(off_cic % lay.ps, 0);

    // G_CICON ob_spec = the CICONBLK's byte offset (the classic index resolved).
    let spec = |i: usize| ptr(&d, version, off_obj + i * lay.obj + (if lay.ps == 8 { 16 } else { 12 }));
    assert_eq!(le16(&d, off_obj + lay.obj + 6), G_CICON);
    assert_eq!(spec(1), off_cib as u64, "object 1 → CICONBLK 0");
    assert_eq!(spec(2), (off_cib + lay.cib) as u64, "object 2 → CICONBLK 1");

    // CICONBLK 0: the ICONBLK head, then mainlist → CICON 0.
    let cib0 = off_cib;
    let words0 = cib0 + 3 * lay.ps;
    assert_eq!(le16(&d, words0), 0x1041, "ib_char");
    assert_eq!(le16(&d, words0 + 10), W as u16, "ib_wicon");
    assert_eq!(le16(&d, words0 + 12), H as u16, "ib_hicon");
    assert_eq!(le16(&d, words0 + 18), 72, "ib_wtext");
    let mainlist0 = ptr(&d, version, cib0 + lay.ib);
    assert_eq!(mainlist0, off_cic as u64, "mainlist → the first CICON");
    // Mono rasters: byte-swapped to little-endian words; the label verbatim.
    let pmask = ptr(&d, version, cib0) as usize;
    let pdata = ptr(&d, version, cib0 + lay.ps) as usize;
    let ptext = ptr(&d, version, cib0 + 2 * lay.ps) as usize;
    assert!(pmask % 2 == 0 && pdata % 2 == 0, "rasters on even offsets");
    for r in 0..MONO_WORDS {
        assert_eq!(le16(&d, pmask + r * 2), 0xFFFF);
        assert_eq!(le16(&d, pdata + r * 2), 0x0FF0);
    }
    assert_eq!(&d[ptext..ptext + 7], b"ICON A\0");

    // CICON 0 (A): 1 plane, no SELECTED, next → CICON 1.
    let cic0 = off_cic;
    assert_eq!(le16(&d, cic0), 1, "num_planes");
    let a_data = ptr(&d, version, cic0 + lay.ps) as usize;
    let a_mask = ptr(&d, version, cic0 + 2 * lay.ps) as usize;
    assert_eq!(ptr(&d, version, cic0 + 3 * lay.ps), 0, "no sel_data");
    assert_eq!(ptr(&d, version, cic0 + 4 * lay.ps), 0, "no sel_mask");
    assert_eq!(ptr(&d, version, cic0 + 5 * lay.ps), (off_cic + lay.cic) as u64, "next_res → CICON 1");
    for r in 0..MONO_WORDS {
        assert_eq!(le16(&d, a_data + r * 2), word_a(r), "A data row {r}");
        assert_eq!(le16(&d, a_mask + r * 2), 0xFFFF);
    }
    // CICON 1 (B): 4 planes, SELECTED pair, last.
    let cic1 = off_cic + lay.cic;
    assert_eq!(le16(&d, cic1), 4);
    let b_data = ptr(&d, version, cic1 + lay.ps) as usize;
    let b_mask = ptr(&d, version, cic1 + 2 * lay.ps) as usize;
    let b_sel = ptr(&d, version, cic1 + 3 * lay.ps) as usize;
    let b_selmask = ptr(&d, version, cic1 + 4 * lay.ps) as usize;
    assert_eq!(ptr(&d, version, cic1 + 5 * lay.ps), 0, "last variant");
    assert!(b_sel != 0 && b_selmask != 0, "the SELECTED pair is carried");
    for p in 0..4 {
        for r in 0..MONO_WORDS {
            assert_eq!(le16(&d, b_data + (p * MONO_WORDS + r) * 2), word_b(p, r), "B plane {p} row {r}");
            assert_eq!(le16(&d, b_sel + (p * MONO_WORDS + r) * 2), !word_b(p, r), "B sel plane {p} row {r}");
        }
    }
    for r in 0..MONO_WORDS {
        assert_eq!(le16(&d, b_mask + r * 2), 0xF0F0);
        assert_eq!(le16(&d, b_selmask + r * 2), 0x0F0F);
    }
    // Every raster lies inside the blob.
    for (off, len) in [
        (a_data, MONO_WORDS * 2),
        (a_mask, MONO_WORDS * 2),
        (b_data, MONO_WORDS * 8),
        (b_mask, MONO_WORDS * 2),
        (b_sel, MONO_WORDS * 8),
        (b_selmask, MONO_WORDS * 2),
    ] {
        assert!(off + len <= total, "raster at {off}+{len} inside {total}");
    }

    // CICONBLK 1: no variant, and the 12-byte label NUL-terminated by the tool.
    let cib1 = off_cib + lay.cib;
    assert_eq!(ptr(&d, version, cib1 + lay.ib), 0, "mainlist 0 = mono only");
    let ptext1 = ptr(&d, version, cib1 + 2 * lay.ps) as usize;
    assert_eq!(&d[ptext1..ptext1 + 13], b"TWELVE CHARS\0");
}

#[test]
fn mkrsc_convert_lp64_carries_the_colour_icons_as_c_structs() {
    check_hand_built(
        "lp64",
        None,
        Layout { version: NRSC_VER_RSHDR, ps: 8, obj: 32, cib: 56, cic: 48, ib: 48 },
    );
}

#[test]
fn mkrsc_convert_ilp32_carries_the_colour_icons_as_c_structs() {
    check_hand_built(
        "ilp32",
        Some("--ilp32"),
        Layout { version: NRSC_VER_RSHDR + 1, ps: 4, obj: 24, cib: 40, cic: 24, ib: 36 },
    );
}

/// An old-format file gets the same header with empty CICON arrays.
#[test]
fn an_old_format_file_declares_no_colour_icons() {
    const GEM_RSC: &str = "references/emutos/aes/gem.rsc";
    if !crate::test_inputs::require_paths("rsc_cicon_tests::an_old_format_file_declares_no_colour_icons", &[GEM_RSC]) {
        return;
    }
    let dir = scratch_dir("old");
    let src = workspace_root().join(GEM_RSC);
    let out = dir.join("gem.nrsc");
    let o = run_mkrsc(&dir, None, &src, &out);
    assert!(o.status.success());
    let d = std::fs::read(&out).unwrap();
    assert_eq!(field(&d, 0), NRSC_VER_RSHDR);
    assert_eq!(field(&d, 17), 0, "n_cib");
    assert_eq!(field(&d, 19), 0, "n_cic");
    assert!(d.len() > HDR);
    check_rshdr(&d, 0); // gem.rsc: rsh_vrsn 0
}

/// TeraDesk's real `cicons.rsc` (63 788 B classic, `rsh_vrsn = 4`, 38
/// CICONBLKs of one 4-plane variant each — 37 of them 32×32, block 3 48×32;
/// 36 with a SELECTED image, blocks 20 and 31 without): converts, keeps
/// every block, and the native LP64 file fits the converter's bound
/// (`RSC_CAP` less the 40-byte preamble) — with the `RSHDR` TeraDesk's
/// `load_icons` → `xrsrc_gaddr` will parse at its head.
#[test]
fn teradesk_cicons_rsc_converts_and_fits_rsc_cap() {
    const CICONS: &str = "sdk/apps/teradesk/cicons.rsc";
    if !crate::test_inputs::require_paths("rsc_cicon_tests::teradesk_cicons_rsc_converts_and_fits_rsc_cap", &[CICONS]) {
        return;
    }
    let dir = scratch_dir("teradesk");
    let src = workspace_root().join(CICONS);
    let out = dir.join("cicons.rsc");
    let o = run_mkrsc(&dir, None, &src, &out);
    assert!(
        o.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    let d = std::fs::read(&out).unwrap();
    let total = field(&d, 5) as usize;
    assert_eq!(total, d.len());
    assert!(
        total <= IMAGE_CAP,
        "native cicons.rsc is {total} bytes, over the {IMAGE_CAP}-byte image bound — rsrc_load would refuse it"
    );
    check_rshdr(&d, 4); // new-format source: rsh_vrsn 4 (NEW_FORMAT_RSC)
    assert_eq!(field(&d, 17), 38, "38 CICONBLKs");
    assert_eq!(field(&d, 19), 38, "38 CICONs");
    assert_eq!(field(&d, 2), 39, "39 objects");
    let off_obj = field(&d, 4) as usize;
    let off_cib = field(&d, 16) as usize;
    let off_cic = field(&d, 18) as usize;
    for i in 1..39 {
        let o = off_obj + i * 32;
        assert_eq!(le16(&d, o + 6), G_CICON, "object {i}");
        assert_eq!(le64(&d, o + 16), (off_cib + (i - 1) * 56) as u64, "object {i} → CICONBLK {}", i - 1);
    }
    for i in 0..38 {
        let cib = off_cib + i * 56;
        assert_eq!(le16(&d, cib + 24 + 10), if i == 3 { 48 } else { 32 }, "ib_wicon of block {i}");
        assert_eq!(le16(&d, cib + 24 + 12), 32, "ib_hicon of block {i}");
        assert_eq!(le64(&d, cib + 48), (off_cic + i * 48) as u64, "mainlist of block {i}");
        let cic = off_cic + i * 48;
        assert_eq!(le16(&d, cic), 4, "4 planes");
        let has_sel = le64(&d, cic + 24) != 0;
        assert_eq!(has_sel, !matches!(i, 20 | 31), "SELECTED image of block {i}");
        assert_eq!(le64(&d, cic + 32) != 0, has_sel, "SELECTED mask comes with the image");
        assert_eq!(le64(&d, cic + 40), 0, "single variant");
    }
    // The size the report names (LP64: 63 610 B of v11 + the 40-byte
    // preamble). Kept as a number so a converter change that grows the file
    // shows up here before it meets the bound.
    assert_eq!(total, 63_650, "native cicons.rsc size (LP64, RSHDR-headed)");
}
