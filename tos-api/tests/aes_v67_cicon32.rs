//! V67 — colour icons at the 32-plane setting, end to end: a hand-built NRSC
//! v11 resource with a 1-, a 4- (with a SELECTED image), an 8- and a 16-plane
//! variant (the last behind a 4-plane one in the same chain) is loaded through
//! the real loader (`tos_api::aes::resource::relocate_and_register`, which
//! asks the live device: the kernel's `VdiScreen` over a host store,
//! `vdi_planes32_common`), drawn through the real Object Manager
//! (`render_tree_lp64`, `G_CICON`), and the screen is read back.
//!
//! Spec: `docs/api/vdi/raster/_raster.md`, section "32-plane setting: AES
//! colour icons, the AES inventory, the order of the change (ADR-038 points
//! 10 and 11, V67)", rules I1-I11; TASK/V67.md readings 1 and 2. The expected
//! pixels are computed here from the rules, not from the code under test:
//!
//! - the variant: the deepest one with at most the device's planes, an exact
//!   one at once (EmuTOS `best_match`, `aes/gemrslib.c:267-282`, I5);
//! - the index `n`: the variant's planes 0..7, plane `p` = bit `p`; when the
//!   variant has fewer planes than the device, a pixel with every source plane
//!   set gets every further plane (so `n = 255`), and a pixel outside the mask
//!   gets `n = 0` (`expand_cicondata`, `:296-343`, I3, I4); an exact variant
//!   is taken as it is;
//! - the pen `REV_MAP_COL[n]` of a 256-colour device (`vdi/vdi_col.c:25-29,
//!   736-747`), and its pixel from the device's requested colour: at 32 planes
//!   the 8-bit pen pixel `q = (c * 255 + 500) / 1000` per gun (P3, reading 1),
//!   at 16 the 5-bit word replicated; pen 255 stays 0 (`init_wk`);
//! - the draw: the mask in the background pen (white), then the image ANDed
//!   onto the screen (`gr_gicon` / `gr_colourblit`, `aes/gemgraf.c:642-671,
//!   710-807`, I8).
//!
//! Ticket: `TASK/V67.md`.

#![cfg(all(target_arch = "aarch64", any(target_os = "macos", target_os = "linux")))]

mod vdi_planes32_common;

use tos_api::aes::object::{render_tree_lp64, Object};
use tos_api::bdos::process::Abi;
use tos_api::vdi::backend::{RasterBackend, VDI_PLANES_WIDE};
use vdi_planes32_common::*;

// ── the NRSC v11 LP64 layout (`tos-api/src/aes/resource.rs`, `NrscHeader`) ──

const HDR: usize = 84;
const OBJECT_SIZE: usize = 32;
const ICONBLK_SIZE: usize = 48;
const CICONBLK_SIZE: usize = ICONBLK_SIZE + 8;
const CICON_SIZE: usize = 48;
const NRSC_VER_CICON: u32 = 11;

const G_IBOX: u16 = 25;
const G_CICON: u16 = 33;
const NIL: i16 = -1;
const SELECTED: u16 = 0x0001;

const IW: usize = 16;
const IH: usize = 16;
const WORDS: usize = IW / 16 * IH;

/// The variants: `(block, planes, with a SELECTED image)`; block 3 chains a
/// 4-plane variant before its 16-plane one.
const VARIANTS: [(usize, i16, bool); 5] = [(0, 1, false), (1, 4, true), (2, 8, false), (3, 4, false), (3, 16, false)];
const N_CIB: usize = 4;
const N_CIC: usize = VARIANTS.len();

const OFF_TRI: usize = HDR;
const OFF_OBJ: usize = 96; // 84 + the 8-byte tree slot, padded to 8
const N_OBJ: usize = 1;
const OFF_CIB: usize = OFF_OBJ + N_OBJ * OBJECT_SIZE;
const OFF_CIC: usize = OFF_CIB + N_CIB * CICONBLK_SIZE;
const OFF_DATA: usize = OFF_CIC + N_CIC * CICON_SIZE;

fn put16(b: &mut [u8], off: usize, v: u16) {
    b[off..off + 2].copy_from_slice(&v.to_le_bytes());
}
fn put32(b: &mut [u8], off: usize, v: u32) {
    b[off..off + 4].copy_from_slice(&v.to_le_bytes());
}
fn put64(b: &mut [u8], off: usize, v: u64) {
    b[off..off + 8].copy_from_slice(&v.to_le_bytes());
}

/// Deterministic plane words: plane `p` of variant `v`, image `sel`, row `r`.
fn plane_word(v: usize, sel: bool, p: usize, r: usize) -> u16 {
    let mut x = ((v as u64 + 1) * 0x9E37_79B9) ^ ((p as u64 + 1) * 0x85EB_CA6B) ^ ((r as u64 + 1) * 0xC2B2_AE35);
    if sel {
        x ^= 0x27D4_EB2F;
    }
    x ^= x >> 15;
    x = x.wrapping_mul(0x2545_F491_4F6C_DD1D);
    let w = (x >> 32) as u16;
    // Row 0 of every image: all planes set on the left 4 pixels, so the
    // all-ones index (`n = 255` after the expansion) occurs.
    if r == 0 {
        w | 0xF000
    } else {
        w
    }
}

/// The mask: rows 0..12 full, rows 12..16 only the middle 8 pixels.
fn mask_word(r: usize) -> u16 {
    if r < 12 {
        0xFFFF
    } else {
        0x0FF0
    }
}

/// The blob and where its rasters lie.
struct Blob {
    bytes: Vec<u8>,
}

fn build() -> Blob {
    // Rasters: per block a mono mask, a mono data plane and an empty label;
    // per variant its planes and mask (and its SELECTED pair).
    let mut data = Vec::<u8>::new();
    let push_words = |data: &mut Vec<u8>, words: &[u16]| -> usize {
        let off = OFF_DATA + data.len();
        for w in words {
            data.extend_from_slice(&w.to_le_bytes());
        }
        off
    };
    let mask: Vec<u16> = (0..IH).map(mask_word).collect();
    let mut mono = Vec::new();
    for _ in 0..N_CIB {
        let m = push_words(&mut data, &mask);
        let d = push_words(&mut data, &[0u16; WORDS]);
        let t = push_words(&mut data, &[0u16; 4]); // "" (NUL), padded
        mono.push((m, d, t));
    }
    let mut cic = Vec::new();
    for (v, &(_, planes, sel)) in VARIANTS.iter().enumerate() {
        let image = |s: bool| -> Vec<u16> {
            (0..planes as usize).flat_map(|p| (0..IH).map(move |r| plane_word(v, s, p, r))).collect()
        };
        let cd = push_words(&mut data, &image(false));
        let cm = push_words(&mut data, &mask);
        let (sd, sm) = if sel {
            (push_words(&mut data, &image(true)), push_words(&mut data, &mask))
        } else {
            (0, 0)
        };
        cic.push((planes, cd, cm, sd, sm));
    }
    let total = OFF_DATA + data.len();
    let mut b = vec![0u8; total];
    b[OFF_DATA..].copy_from_slice(&data);
    b[0..4].copy_from_slice(b"NRSC");
    let hdr = [
        NRSC_VER_CICON,
        1,
        N_OBJ as u32,
        OFF_TRI as u32,
        OFF_OBJ as u32,
        total as u32,
        OFF_DATA as u32, // free strings: none
        0,
        OFF_DATA as u32, // free images: none
        0,
        OFF_CIB as u32, // TEDINFO: none
        0,
        OFF_CIB as u32, // ICONBLK: none
        0,
        OFF_CIB as u32, // BITBLK: none
        0,
        OFF_CIB as u32,
        N_CIB as u32,
        OFF_CIC as u32,
        N_CIC as u32,
    ];
    for (i, v) in hdr.iter().enumerate() {
        put32(&mut b, 4 + i * 4, *v);
    }
    put64(&mut b, OFF_TRI, OFF_OBJ as u64);
    // One root object (the trees of the file are not drawn here).
    put16(&mut b, OFF_OBJ, NIL as u16);
    put16(&mut b, OFF_OBJ + 2, NIL as u16);
    put16(&mut b, OFF_OBJ + 4, NIL as u16);
    put16(&mut b, OFF_OBJ + 6, G_IBOX);
    for (i, &(m, d, t)) in mono.iter().enumerate() {
        let o = OFF_CIB + i * CICONBLK_SIZE;
        put64(&mut b, o, m as u64);
        put64(&mut b, o + 8, d as u64);
        put64(&mut b, o + 16, t as u64);
        // ib_char: fg pen 1, bg pen 0, no character; the icon at (0,0), no label box.
        let words: [i16; 11] = [0x1000, 0, 0, 0, 0, IW as i16, IH as i16, 0, IH as i16, 0, 0];
        for (k, w) in words.iter().enumerate() {
            put16(&mut b, o + 24 + k * 2, *w as u16);
        }
        let first = VARIANTS.iter().position(|v| v.0 == i).unwrap();
        put64(&mut b, o + ICONBLK_SIZE, (OFF_CIC + first * CICON_SIZE) as u64);
    }
    for (v, &(planes, cd, cm, sd, sm)) in cic.iter().enumerate() {
        let o = OFF_CIC + v * CICON_SIZE;
        put16(&mut b, o, planes as u16);
        put64(&mut b, o + 8, cd as u64);
        put64(&mut b, o + 16, cm as u64);
        put64(&mut b, o + 24, sd as u64);
        put64(&mut b, o + 32, sm as u64);
        let next = if v + 1 < N_CIC && VARIANTS[v + 1].0 == VARIANTS[v].0 {
            (OFF_CIC + (v + 1) * CICON_SIZE) as u64
        } else {
            0
        };
        put64(&mut b, o + 40, next);
    }
    Blob { bytes: b }
}

// ── the expected pixels, from the rules ───────────────────────────────────

/// EmuTOS `REV_MAP_COL` of a 256-colour device (`vdi_col.c:25-29,736-747`).
fn rev_map_col() -> [u8; 256] {
    const ROM: [u8; 16] = [0, 15, 1, 2, 4, 6, 3, 5, 7, 8, 9, 10, 12, 14, 11, 13];
    let mut map = [0u8; 256];
    map[..16].copy_from_slice(&ROM);
    map[1] = 255;
    for (i, m) in map.iter_mut().enumerate().skip(16) {
        *m = i as u8;
    }
    map[255] = 15;
    let mut rev = [0u8; 256];
    for (pen, &reg) in map.iter().enumerate() {
        rev[reg as usize] = pen as u8;
    }
    rev
}

/// The requested colour of pen `p` in this binary: `VIDEL_PALETTE1` for
/// 0..15, a spread of values with non-5-bit 8-bit pixels above.
fn request(p: usize) -> [i32; 3] {
    if p < 16 {
        let (r, g, b) = VIDEL_PALETTE1[p];
        [r as i32, g as i32, b as i32]
    } else {
        [((p * 37) % 1001) as i32, ((p * 91 + 7) % 1001) as i32, ((p * 13 + 500) % 1001) as i32]
    }
}

/// The device pixel of pen `p` at `planes`: the P3 pen pixel at 32, the 5-bit
/// word replicated at 16; pen 255 stays 0.
fn pen_pixel(planes: u16, p: usize) -> u32 {
    if p == 255 {
        return 0;
    }
    if planes == VDI_PLANES_WIDE {
        pixel32(request(p))
    } else {
        word_to_device(word16(request(p)))
    }
}

/// The variant `best_match` takes for block `blk` at `dev` planes.
fn chosen(blk: usize, dev: i16) -> usize {
    let mut best: Option<usize> = None;
    for (v, &(b, planes, _)) in VARIANTS.iter().enumerate() {
        if b != blk || planes > dev {
            continue;
        }
        if planes == dev {
            return v;
        }
        if best.is_none_or(|bv| planes > VARIANTS[bv].1) {
            best = Some(v);
        }
    }
    best.expect("every block has a variant <= 16 planes")
}

/// `(n, inside the mask)` of pixel `(c, r)` of variant `v` at `dev` planes.
fn index(v: usize, sel: bool, dev: i16, c: usize, r: usize) -> (usize, bool) {
    let planes = VARIANTS[v].1 as usize;
    let bit = 0x8000u16 >> c;
    let mut idx = 0usize;
    for p in 0..planes {
        if plane_word(v, sel, p, r) & bit != 0 {
            idx |= 1 << p;
        }
    }
    let inside = mask_word(r) & bit != 0;
    let n = if (planes as i16) < dev {
        if !inside {
            0
        } else if idx == (1 << planes) - 1 {
            255
        } else {
            idx & 0xff
        }
    } else {
        idx & 0xff
    };
    (n, inside)
}

/// A store word no draw makes and that both settings hold exactly (5-bit
/// guns, X = 0): what the screen is filled with before each draw.
const GROUND: u32 = 0x0084_8484;

/// The screen pixel a draw of variant `v` leaves at `(c, r)` on [`GROUND`].
fn want(planes: u16, v: usize, sel: bool, c: usize, r: usize) -> u32 {
    let dev = planes as i16;
    let rev = rev_map_col();
    let (n, inside) = index(v, sel, dev, c, r);
    let icon = pen_pixel(planes, rev[n] as usize);
    // The mask blit paints the background pen (white) inside the mask.
    let under = if inside { pen_pixel(planes, 0) } else { GROUND };
    if planes == VDI_PLANES_WIDE {
        under & icon
    } else {
        word_to_device(device_to_word(under) & device_to_word(icon))
    }
}

// ── load and draw ─────────────────────────────────────────────────────────

/// `rsrc_free` through the AES dispatcher: releases the resource and its
/// colour-icon buffer.
fn rsrc_free() {
    let mut control = [111i16, 0, 1, 0, 0];
    let mut global = [0i16; 16];
    let mut intin = [0i16; 4];
    let mut intout = [0i16; 8];
    let mut addrin = [0u64; 2];
    let mut addrout = [0u64; 2];
    let raw = [
        control.as_mut_ptr() as u64,
        global.as_mut_ptr() as u64,
        intin.as_mut_ptr() as u64,
        intout.as_mut_ptr() as u64,
        addrin.as_mut_ptr() as u64,
        addrout.as_mut_ptr() as u64,
    ];
    // SAFETY: a well-formed LP64 parameter block, alive for the call.
    unsafe { tos_api::aes::dispatch(raw.as_ptr() as u64, Abi::Lp64) };
}

/// The loaded resource: the relocated blob (its storage), and its base.
struct Loaded {
    _words: Vec<u64>,
    base: *mut u8,
}

impl Loaded {
    fn cib(&self, blk: usize) -> u64 {
        self.base as u64 + (OFF_CIB + blk * CICONBLK_SIZE) as u64
    }
    fn rd64(&self, off: usize) -> u64 {
        // SAFETY: inside the live blob.
        unsafe { (self.base.add(off) as *const u64).read_unaligned() }
    }
    fn rd16(&self, off: usize) -> u16 {
        // SAFETY: inside the live blob.
        unsafe { (self.base.add(off) as *const u16).read_unaligned() }
    }
}

impl Drop for Loaded {
    fn drop(&mut self) {
        rsrc_free();
    }
}

/// Set the requested colours of this binary, open a workstation at the
/// setting (the active pens), and load the resource for the live device.
fn load() -> Loaded {
    {
        let mut req = screen().req.lock().unwrap();
        for (p, e) in req.iter_mut().enumerate() {
            let [r, g, b] = request(p);
            *e = (r as u16, g as u16, b as u16);
        }
    }
    let src = build().bytes;
    let mut words = vec![0u64; src.len().div_ceil(8) + 1];
    let base = words.as_mut_ptr() as *mut u8;
    // SAFETY: `words` holds at least `src.len()` bytes.
    unsafe { std::ptr::copy_nonoverlapping(src.as_ptr(), base, src.len()) };
    // SAFETY: `base` addresses `src.len()` writable bytes that live as long as the result.
    assert!(unsafe { tos_api::aes::resource::relocate_and_register(base, src.len()) }, "the resource must load");
    Loaded { _words: words, base }
}

/// A virtual workstation at the setting, made the drawing one: one call on
/// it points the backend's active pens at its palette, seeded at the
/// setting's depth (as the AES's own VDI calls on its handle do before it
/// draws), whatever depth an earlier test of this binary left the physical
/// workstation's palette in.
fn open_ws() -> Ws {
    let mut ws = Ws::open();
    ws.vswr_mode(1);
    ws
}

/// Draw block `blk` with `state` at `(x, y)` on a screen filled with [`GROUND`].
fn draw(l: &Loaded, blk: usize, state: u16, x: i16, y: i16) {
    screen().px.lock().unwrap().fill(GROUND);
    let tree = [
        Object {
            ob_next: NIL,
            ob_head: 1,
            ob_tail: 1,
            ob_type: G_IBOX,
            ob_flags: 0,
            ob_state: 0,
            ob_spec: 0,
            ob_x: 0,
            ob_y: 0,
            ob_width: W as i16,
            ob_height: H as i16,
        },
        Object {
            ob_next: 0,
            ob_head: NIL,
            ob_tail: NIL,
            ob_type: G_CICON,
            ob_flags: 0,
            ob_state: state,
            ob_spec: l.cib(blk),
            ob_x: x,
            ob_y: y,
            ob_width: IW as i16,
            ob_height: IH as i16,
        },
    ];
    let b: &dyn RasterBackend = screen();
    // SAFETY: a live LP64 OBJECT array; the resource outlives the call.
    unsafe { render_tree_lp64(tree.as_ptr(), 0, 8, 0, 0, 0, 0, b) };
}

/// Every pixel of the icon and its one-pixel frame, as drawn at the setting.
fn picture(blk: usize, state: u16) -> Vec<u32> {
    let _ws = open_ws();
    let l = load();
    let (x, y) = (40i16, 30i16);
    draw(&l, blk, state, x, y);
    let mut v = Vec::new();
    for r in -1..=IH as i16 {
        for c in -1..=IW as i16 {
            v.push(screen().raw(x + c, y + r));
        }
    }
    v
}

fn check_block(planes: u16, blk: usize, state: u16) {
    let _ws = open_ws();
    let l = load();
    let dev = planes as i16;
    let v = chosen(blk, dev);
    let sel = state & SELECTED != 0 && VARIANTS[v].2;
    // The loader's record: `mainlist` names the chosen variant, stamped at
    // the device depth (`:470`, I8).
    let cib_off = OFF_CIB + blk * CICONBLK_SIZE;
    assert_eq!(l.rd64(cib_off + ICONBLK_SIZE), l.base as u64 + (OFF_CIC + v * CICON_SIZE) as u64, "block {blk}: variant {v}");
    assert_eq!(l.rd16(OFF_CIC + v * CICON_SIZE), planes, "num_planes = gl_nplanes");
    let (x, y) = (40i16, 30i16);
    draw(&l, blk, state, x, y);
    for r in 0..IH {
        for c in 0..IW {
            let got = screen().raw(x + c as i16, y + r as i16);
            assert_eq!(got, want(planes, v, sel, c, r), "{planes} planes, block {blk} variant {v} sel {sel}: ({c},{r})");
        }
    }
    // Nothing outside the icon: the frame around it is the ground.
    for k in -1..=IW as i16 {
        assert_eq!(screen().raw(x + k, y - 1), GROUND, "above ({k})");
        assert_eq!(screen().raw(x + k, y + IH as i16), GROUND, "below ({k})");
    }
    for k in 0..IH as i16 {
        assert_eq!(screen().raw(x - 1, y + k), GROUND, "left ({k})");
        assert_eq!(screen().raw(x + IW as i16, y + k), GROUND, "right ({k})");
    }
}

// ── the tests ─────────────────────────────────────────────────────────────

/// At 32 planes the 1-, 4-, 8- and 16-plane variants draw as the 32-plane
/// device form: each pixel the 8-bit pen pixel of `REV_MAP_COL[n]` (I2-I4),
/// the mask in white first, the image ANDed (I8), the right variant (I5),
/// nothing outside the icon (I9).
#[test]
fn at_32_planes_every_variant_draws_its_pen_pixels() {
    let _g = setting(32);
    for blk in 0..N_CIB {
        check_block(32, blk, 0);
    }
}

/// The SELECTED image of the 4-plane variant at 32 (I7).
#[test]
fn at_32_planes_the_selected_image_draws_its_pen_pixels() {
    let _g = setting(32);
    check_block(32, 1, SELECTED);
}

/// At 16 planes the same resource draws as before V67: the 16-bit words of the
/// 5-bit pens, the exact 16-plane variant without expansion or mask.
#[test]
fn at_16_planes_every_variant_draws_its_16_bit_words() {
    let _g = setting(16);
    for blk in 0..N_CIB {
        check_block(16, blk, 0);
    }
    check_block(16, 1, SELECTED);
}

/// The 16 and 32 pictures of every variant are the same within the pen
/// tolerance of V64 (at most 5 per gun, `_raster.md` P9), except the pixels
/// of the exact 16-plane variant under a clear mask bit (TASK/V67 reading 2:
/// expanded and masked at 32, taken as it is at 16).
#[test]
fn the_16_and_32_plane_pictures_agree_within_the_pen_tolerance() {
    let _g = setting(16);
    let pics16: Vec<Vec<u32>> = (0..N_CIB).map(|blk| picture(blk, 0)).collect();
    drop(_g);
    let _g = setting(32);
    let pics32: Vec<Vec<u32>> = (0..N_CIB).map(|blk| picture(blk, 0)).collect();
    let mut exact_differs = 0;
    for blk in 0..N_CIB {
        for (i, (&a, &b)) in pics16[blk].iter().zip(pics32[blk].iter()).enumerate() {
            let (c, r) = ((i % (IW + 2)) as i32 - 1, (i / (IW + 2)) as i32 - 1);
            let inside_icon = (0..IW as i32).contains(&c) && (0..IH as i32).contains(&r);
            let masked_out = inside_icon && mask_word(r as usize) & (0x8000u16 >> c) == 0;
            let d = |s: u32| ((a >> s) & 0xff).abs_diff((b >> s) & 0xff);
            let within = d(16) <= 5 && d(8) <= 5 && d(0) <= 5;
            if blk == 3 && masked_out {
                exact_differs += usize::from(!within);
                continue;
            }
            assert!(within, "block {blk} ({c},{r}): 16 {a:08x} vs 32 {b:08x}");
            assert_eq!(b >> 24, 0, "X = 0 at 32");
        }
    }
    assert!(exact_differs > 0, "the exact 16-plane variant shows its unmasked data at 16 (reading 2)");
}

