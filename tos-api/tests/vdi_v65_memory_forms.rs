//! V65 — memory forms, copies, `vr_trnfm`, `v_get_pixel` and the save buffers
//! at the 32-plane setting: `docs/api/vdi/raster/_raster.md` § "32-plane
//! setting: memory forms, copies, `vr_trnfm`, `v_get_pixel`, save buffers
//! (ADR-038 points 5-7 and 10, V65)" §1-9, and the wide `v_get_pixel` answer of
//! `docs/api/vdi/inquiry/_color-pixel-input.md`, pinned at 32 planes with the
//! unchanged 16-plane behaviour beside them (ADR-038 point 11).
//!
//! - §3/§9 acceptance 1: a pixel built from the ADR-038 point 8 description
//!   (red bits 16..23, green 8..15, blue 0..7; the `vq_scrninfo` table itself
//!   is TASK/V66), written into a 32-plane memory form, copied to the screen
//!   with `vro_cpyfm` and read with `v_get_pixel`, comes back as
//!   `{0x00RR, 0xGGBB}`.
//! - §3: the 32-plane device form — row `fd_wdwidth * 16` pixels, 4 bytes a
//!   pixel, the sixteen ops bitwise on all 32 bits, the overlap direction —
//!   for screen to memory, memory to screen and memory to memory; screen
//!   derived pixels are compared as `pixel & 0x00FFFFFF` (reading 3).
//! - §4: `vrt_cpyfm` into a 32-plane form in all four writing modes with the
//!   pens' 8-bit pixels.
//! - §2: the refused plane counts at 32 (9..31, 16 included, also two
//!   16-plane memory forms) are silent no-ops; 1..8 planar still runs.
//! - §5 acceptance 2: `vr_trnfm` device to standard and back (and standard to
//!   device and back) is the identity, X included, also in place; the plane
//!   order; 9..31 refused.
//! - §6: the `v_get_pixel` answer shape at both settings.
//! - §7: `bb_save_words` and the `device_mem` stamp from one depth source,
//!   sizes pinned, no write past the buffer, `bb_save`/`bb_restore` exact; the
//!   colour-icon blit keeps its 16-bit form and reads only it.
//!
//! The backend is the kernel's `VdiScreen` over a host store, shared with the
//! V63/V64 tests (`vdi_planes32_common`).
//!
//! Ticket: `TASK/V65.md`.

#![cfg(all(target_arch = "aarch64", any(target_os = "macos", target_os = "linux")))]
#![allow(clippy::too_many_arguments)]

mod vdi_planes32_common;

use tos_api::bdos::process::Abi;
use tos_api::vdi::backend::{ClipRect, RasterBackend, Rect};
use vdi_planes32_common::*;

const OP_VRO_CPYFM: i16 = 109;
const OP_VR_TRNFM: i16 = 110;
const OP_VRT_CPYFM: i16 = 121;

const S_ONLY: i16 = 3;
const S_AND_D: u8 = 1;
const MD_REPLACE: i16 = 1;
const MD_TRANS: i16 = 2;
const MD_XOR: i16 = 3;
const MD_ERASE: i16 = 4;

/// The pixel a program builds from the 32-plane description of ADR-038 point
/// 8 (`vq_scrninfo` red bits 16..23, green 8..15, blue 0..7, X 24..31 unused):
/// `0x00RRGGBB`. Written from the ADR, not from the code under test.
fn pixel_from_description(r: u8, g: u8, b: u8) -> u32 {
    const RED_SHIFT: u32 = 16; // [16..23] = 16..23
    const GREEN_SHIFT: u32 = 8; // [32..39] = 8..15
    const BLUE_SHIFT: u32 = 0; // [48..55] = 0..7
    ((r as u32) << RED_SHIFT) | ((g as u32) << GREEN_SHIFT) | ((b as u32) << BLUE_SHIFT)
}

/// The sixteen logic ops of `vro_cpyfm` as the spec states them, bitwise on
/// the whole word (`_raster.md` "Logic operations", §3 at 32 bits).
fn op32(op: u8, s: u32, d: u32) -> u32 {
    match op & 0x0f {
        0 => 0,
        1 => s & d,
        2 => s & !d,
        3 => s,
        4 => !s & d,
        5 => d,
        6 => s ^ d,
        7 => s | d,
        8 => !(s | d),
        9 => !(s ^ d),
        10 => !d,
        11 => s | !d,
        12 => !s,
        13 => !s | d,
        14 => !(s & d),
        _ => !0,
    }
}

/// An LP64 MFDB (`mt_gem.h:2757-2768`): `fd_addr` 8 bytes, then the WORDs.
#[repr(C)]
#[derive(Clone, Copy)]
struct Mfdb {
    addr: u64,
    w: i16,
    h: i16,
    wdwidth: i16,
    stand: i16,
    nplanes: i16,
    r1: i16,
    r2: i16,
    r3: i16,
}

impl Mfdb {
    fn screen() -> Mfdb {
        Mfdb { addr: 0, w: 0, h: 0, wdwidth: 0, stand: 0, nplanes: 0, r1: 0, r2: 0, r3: 0 }
    }
    fn mem(addr: *const u8, w: i16, h: i16, nplanes: i16) -> Mfdb {
        Mfdb { addr: addr as u64, w, h, wdwidth: (w + 15) / 16, stand: 0, nplanes, r1: 0, r2: 0, r3: 0 }
    }
}

/// A memory form's pixels: `fd_h` rows of `fd_wdwidth * 16` 32-bit pixels,
/// plus a canary tail of `TAIL` words that nothing may write.
struct Form32 {
    buf: Vec<u32>,
    w: i16,
    h: i16,
}

const TAIL: usize = 64;
const CANARY: u32 = 0xC0DE_CAFE;

impl Form32 {
    fn new(w: i16, h: i16, fill: impl FnMut(usize) -> u32) -> Form32 {
        let n = Self::len(w, h);
        let mut buf: Vec<u32> = (0..n).map(fill).collect();
        buf.extend(std::iter::repeat_n(CANARY, TAIL));
        Form32 { buf, w, h }
    }
    fn stride(w: i16) -> usize {
        (w as usize).div_ceil(16) * 16
    }
    fn len(w: i16, h: i16) -> usize {
        Self::stride(w) * h as usize
    }
    /// The MFDB over the pixels (a pointer the VDI may write through).
    fn mfdb(&mut self) -> Mfdb {
        Mfdb::mem(self.buf.as_mut_ptr() as *const u8, self.w, self.h, 32)
    }
    fn at(&self, x: usize, y: usize) -> u32 {
        self.buf[y * Self::stride(self.w) + x]
    }
    fn tail_intact(&self) -> bool {
        self.buf[Self::len(self.w, self.h)..].iter().all(|&v| v == CANARY)
    }
}

impl Ws {
    /// A raster call with the two MFDB pointers at `CONTRL[7..10]` and
    /// `CONTRL[11..14]` (LP64, `N_PTRINTS = 4`, `gem_vdiP.h:177,204`).
    fn call_mfdb(&mut self, opcode: i16, intin: &[i16], ptsin: &[i16], src: &Mfdb, dst: &mut Mfdb) {
        let pb = &mut self.pb;
        pb.contrl.fill(0);
        pb.intout.fill(POISON);
        pb.ptsout.fill(POISON);
        pb.intin[..intin.len()].copy_from_slice(intin);
        pb.ptsin[..ptsin.len()].copy_from_slice(ptsin);
        pb.contrl[0] = opcode;
        pb.contrl[1] = (ptsin.len() / 2) as i16;
        pb.contrl[3] = intin.len() as i16;
        pb.contrl[6] = self.h;
        let (s, d) = (src as *const Mfdb as u64, dst as *mut Mfdb as u64);
        // SAFETY: `contrl` has 16 WORDs; slots 7..10 and 11..14 hold the two
        // pointers, written unaligned as gemlib packs them.
        unsafe {
            (pb.contrl[7..].as_mut_ptr() as *mut u64).write_unaligned(s);
            (pb.contrl[11..].as_mut_ptr() as *mut u64).write_unaligned(d);
        }
        // SAFETY: `block` holds five live pointers to arrays owned by `pb`; the
        // MFDBs and their bitmaps outlive the call.
        unsafe { tos_api::vdi::dispatch(pb.block.as_ptr() as u64, Abi::Lp64) };
    }
    fn vro_cpyfm(&mut self, op: i16, s: [i16; 4], d: [i16; 4], src: &Mfdb, dst: &mut Mfdb) {
        let pts = [s[0], s[1], s[2], s[3], d[0], d[1], d[2], d[3]];
        self.call_mfdb(OP_VRO_CPYFM, &[op], &pts, src, dst);
    }
    fn vrt_cpyfm(&mut self, mode: i16, fg: i16, bg: i16, s: [i16; 4], d: [i16; 4], src: &Mfdb, dst: &mut Mfdb) {
        let pts = [s[0], s[1], s[2], s[3], d[0], d[1], d[2], d[3]];
        self.call_mfdb(OP_VRT_CPYFM, &[mode, fg, bg], &pts, src, dst);
    }
    fn vr_trnfm(&mut self, src: &Mfdb, dst: &mut Mfdb) {
        self.call_mfdb(OP_VR_TRNFM, &[], &[], src, dst);
    }
    fn vs_color(&mut self, idx: i16, rgb: [i16; 3]) {
        self.call(14, 0, &[idx, rgb[0], rgb[1], rgb[2]], &[]);
    }
}

/// Fill the visible store with deterministic 32-bit words, X noise included.
fn noise_screen(seed: u64) {
    let mut rng = Rng(seed);
    let mut px = screen().px.lock().unwrap();
    for v in px.iter_mut() {
        *v = rng.u32();
    }
}

// ── §3 / acceptance 1: the round trip ───────────────────────────────────

/// A pixel built from the description goes into a 32-plane form, through
/// `vro_cpyfm` onto the screen and back through `v_get_pixel` unchanged:
/// `INTOUT = {0x00RR, 0xGGBB}`. Colours with non-zero low bits per gun (no
/// 5-bit pen makes them) and an X byte in the memory word (written to the
/// screen as it is, masked by every read).
#[test]
fn a_pixel_from_the_description_round_trips_through_vro_cpyfm_and_v_get_pixel() {
    let _g = setting(32);
    let mut ws = Ws::open();
    ws.v_clrwk();
    let colours = [(0x12u8, 0x34u8, 0x56u8), (0x01, 0x02, 0x03), (0xFF, 0x80, 0x7F), (0, 0, 0), (0xFE, 0xFF, 0x01)];
    let pixels: Vec<u32> = colours.iter().map(|&(r, g, b)| pixel_from_description(r, g, b)).collect();
    // Row 0 of a 16-pixel form: the colours, each with a different X byte.
    let mut form = Form32::new(16, 1, |i| match pixels.get(i) {
        Some(&p) => p | (((i as u32) * 0x31) << 24),
        None => 0,
    });
    ws.vro_cpyfm(S_ONLY, [0, 0, 4, 0], [40, 30, 44, 30], &form.mfdb(), &mut Mfdb::screen());
    for (i, &(r, g, b)) in colours.iter().enumerate() {
        let (hi, lo) = ws.v_get_pixel(40 + i as i16, 30);
        assert_eq!((hi, lo), (r as i16, ((g as u16) << 8) | b as u16), "pixel {i} {r:02x}{g:02x}{b:02x}");
        assert_eq!(((hi as u32) << 16) | lo as u32, pixels[i]);
        // The memory word went to the store as it is, X included (§3).
        assert_eq!(screen().raw(40 + i as i16, 30), form.at(i, 0));
    }
    assert!(form.tail_intact());
}

/// Screen to memory and back: the pens' pixels saved into a 32-plane form
/// (compared masked, reading 3) and restored elsewhere give the same raw store
/// words; the copy is the raw store word, X included.
#[test]
fn screen_to_a_32_plane_form_carries_the_raw_store_word() {
    let _g = setting(32);
    let mut ws = Ws::open();
    noise_screen(0x65_0001);
    let mut form = Form32::new(40, 6, |_| 0x5A5A_5A5A);
    ws.vro_cpyfm(S_ONLY, [10, 20, 49, 25], [0, 0, 39, 5], &Mfdb::screen(), &mut form.mfdb());
    for y in 0..6 {
        for x in 0..40 {
            let raw = screen().raw(10 + x as i16, 20 + y as i16);
            assert_eq!(form.at(x, y) & COLOUR, raw & COLOUR, "({x},{y})");
            assert_eq!(form.at(x, y), raw, "the raw store word, X included (reading 3)");
        }
        // The padding columns 40..48 of the row are not part of the copy.
        for x in 40..48 {
            assert_eq!(form.at(x, y), 0x5A5A_5A5A);
        }
    }
    assert!(form.tail_intact());
    // Back to another place: the same raw words.
    ws.vro_cpyfm(S_ONLY, [0, 0, 39, 5], [200, 100, 239, 105], &form.mfdb(), &mut Mfdb::screen());
    for y in 0..6 {
        for x in 0..40 {
            assert_eq!(screen().raw(200 + x, 100 + y), screen().raw(10 + x, 20 + y));
        }
    }
}

/// The row of a 32-plane form is `fd_wdwidth * 16` pixels (reading 6), not
/// `fd_wdwidth * fd_nplanes`: a second row lands at pixel `stride`, and a form
/// of exactly `fd_h` rows is never read or written past.
#[test]
fn a_32_plane_row_is_wdwidth_times_16_pixels() {
    let _g = setting(32);
    let mut ws = Ws::open();
    // w = 20 → wdwidth 2, stride 32 pixels; 3 rows = 96 pixels.
    let mut src = Form32::new(20, 3, |i| 0x0010_0000 + i as u32);
    let mut dst = Form32::new(20, 3, |_| 0);
    ws.vro_cpyfm(S_ONLY, [0, 0, 19, 2], [0, 0, 19, 2], &src.mfdb(), &mut dst.mfdb());
    for y in 0..3 {
        for x in 0..32 {
            let want = if x < 20 { 0x0010_0000 + (y * 32 + x) as u32 } else { 0 };
            assert_eq!(dst.at(x, y), want, "({x},{y})");
        }
    }
    assert!(src.tail_intact() && dst.tail_intact());
    // On the screen: row 1 of the form is row 1 of the block.
    ws.vro_cpyfm(S_ONLY, [0, 0, 19, 2], [100, 50, 119, 52], &src.mfdb(), &mut Mfdb::screen());
    assert_eq!(screen().raw(100, 51), 0x0010_0000 + 32);
    assert_eq!(screen().raw(119, 52), 0x0010_0000 + 2 * 32 + 19);
}

// ── §3: the sixteen ops on 32 bits ──────────────────────────────────────

/// Memory to memory, every op on all 32 bits (X included, the bitwise table
/// of `_raster.md` §3): exact, as both operands are memory pixels.
#[test]
fn all_sixteen_ops_memory_to_memory_act_on_32_bits() {
    let _g = setting(32);
    let mut ws = Ws::open();
    for op in 0..16u8 {
        let mut rng = Rng(0x65_1000 + op as u64);
        let mut src = Form32::new(48, 4, |_| rng.u32());
        let mut rng = Rng(0x65_2000 + op as u64);
        let mut dst = Form32::new(48, 4, |_| rng.u32());
        let before = dst.buf.clone();
        ws.vro_cpyfm(op as i16, [3, 1, 40, 3], [5, 0, 42, 2], &src.mfdb(), &mut dst.mfdb());
        for y in 0..4usize {
            for x in 0..48usize {
                let d0 = before[y * 48 + x];
                let want = if (5..=42).contains(&x) && y <= 2 { op32(op, src.at(x - 2, y + 1), d0) } else { d0 };
                assert_eq!(dst.at(x, y), want, "op {op} ({x},{y})");
            }
        }
        assert!(dst.tail_intact(), "op {op}");
    }
}

/// Screen to memory and memory to screen, every op: the screen side is the raw
/// store word; results that come from the screen are compared masked
/// (reading 3).
#[test]
fn all_sixteen_ops_between_screen_and_a_32_plane_form() {
    let _g = setting(32);
    let mut ws = Ws::open();
    for op in 0..16u8 {
        noise_screen(0x65_3000 + op as u64);
        let screen_before = screen().snapshot();
        let raw0 = |x: usize, y: usize| screen_before[y * W as usize + x];
        // Screen → memory.
        let mut rng = Rng(0x65_4000 + op as u64);
        let mut dst = Form32::new(32, 3, |_| rng.u32());
        let before = dst.buf.clone();
        ws.vro_cpyfm(op as i16, [60, 70, 91, 72], [0, 0, 31, 2], &Mfdb::screen(), &mut dst.mfdb());
        for y in 0..3 {
            for x in 0..32 {
                let want = op32(op, raw0(60 + x, 70 + y), before[y * 32 + x]);
                assert_eq!(dst.at(x, y) & COLOUR, want & COLOUR, "screen→memory op {op} ({x},{y})");
            }
        }
        assert!(dst.tail_intact());
        // Memory → screen.
        let mut rng = Rng(0x65_5000 + op as u64);
        let mut src = Form32::new(32, 3, |_| rng.u32());
        ws.vro_cpyfm(op as i16, [0, 0, 31, 2], [150, 10, 181, 12], &src.mfdb(), &mut Mfdb::screen());
        for y in 0..3usize {
            for x in 0..32usize {
                let want = op32(op, src.at(x, y), raw0(150 + x, 10 + y));
                let got = screen().raw(150 + x as i16, 10 + y as i16);
                assert_eq!(got & COLOUR, want & COLOUR, "memory→screen op {op} ({x},{y})");
            }
        }
        // Nothing outside the block changed.
        assert_eq!(screen().raw(149, 10), raw0(149, 10));
        assert_eq!(screen().raw(182, 12), raw0(182, 12));
    }
}

/// The overlap direction of a shared 32-plane form, as EmuTOS' `vro_cpyfm16`
/// (`vdi_raster.c:995-1002`) with 4-byte pixels: a copy one pixel to the right
/// inside the same buffer is a clean shift, not a smear.
#[test]
fn an_overlapping_copy_in_one_32_plane_form_walks_backwards() {
    let _g = setting(32);
    let mut ws = Ws::open();
    let mut form = Form32::new(16, 2, |i| 0x0100_0000 | i as u32);
    let m = form.mfdb();
    let mut m2 = m;
    ws.vro_cpyfm(S_ONLY, [0, 0, 9, 1], [1, 0, 10, 1], &m, &mut m2);
    for y in 0..2 {
        assert_eq!(form.at(0, y), 0x0100_0000 | (y * 16) as u32);
        for x in 1..=10 {
            assert_eq!(form.at(x, y), 0x0100_0000 | (y * 16 + x - 1) as u32, "({x},{y})");
        }
    }
    // And to the left, forwards.
    let mut form = Form32::new(16, 1, |i| i as u32 + 7);
    let m = form.mfdb();
    let mut m2 = m;
    ws.vro_cpyfm(S_ONLY, [1, 0, 10, 0], [0, 0, 9, 0], &m, &mut m2);
    for x in 0..10 {
        assert_eq!(form.at(x, 0), x as u32 + 8);
    }
}

// ── §4: vrt_cpyfm into a 32-plane form ──────────────────────────────────

/// The four writing modes into a 32-plane form, with the pens' 8-bit pixels
/// (`vs_color` requests whose pixels need 8 bits per gun, P2/P3): replace,
/// transparent, XOR on all 32 bits, erase.
#[test]
fn vrt_cpyfm_into_a_32_plane_form_in_the_four_writing_modes() {
    let _g = setting(32);
    let mut ws = Ws::open();
    ws.vs_color(20, [17, 600, 999]);
    ws.vs_color(21, [25, 3, 333]);
    let (fg, bg) = (pixel32([17, 600, 999]), pixel32([25, 3, 333]));
    assert_ne!(fg & 0x0007_0707, 0, "the test pen needs the low bits of its guns");
    // A 32 x 2 mono source, deterministic.
    let mono: Vec<u16> = vec![0xA5C3, 0x0FF0, 0x8001, 0x7E5A];
    let src = Mfdb { addr: mono.as_ptr() as u64, w: 32, h: 2, wdwidth: 2, stand: 0, nplanes: 1, r1: 0, r2: 0, r3: 0 };
    let bit = |x: usize, y: usize| mono[y * 2 + x / 16] & (0x8000 >> (x % 16)) != 0;
    for mode in [MD_REPLACE, MD_TRANS, MD_XOR, MD_ERASE] {
        let mut rng = Rng(0x65_6000 + mode as u64);
        let mut dst = Form32::new(40, 3, |_| rng.u32());
        let before = dst.buf.clone();
        ws.vrt_cpyfm(mode, 20, 21, [0, 0, 31, 1], [4, 1, 35, 2], &src, &mut dst.mfdb());
        for y in 0..3usize {
            for x in 0..48usize {
                let d0 = before[y * 48 + x];
                let inside = (4..36).contains(&x) && (1..3).contains(&y);
                let want = if !inside {
                    d0
                } else {
                    let set = bit(x - 4, y - 1);
                    match mode {
                        MD_REPLACE => {
                            if set {
                                fg
                            } else {
                                bg
                            }
                        }
                        MD_TRANS => {
                            if set {
                                fg
                            } else {
                                d0
                            }
                        }
                        MD_XOR => {
                            if set {
                                !d0
                            } else {
                                d0
                            }
                        }
                        _ => {
                            if set {
                                d0
                            } else {
                                bg
                            }
                        }
                    }
                };
                assert_eq!(dst.at(x, y), want, "mode {mode} ({x},{y})");
            }
        }
        assert!(dst.tail_intact(), "mode {mode}");
    }
}

// ── §2: the refused plane counts ────────────────────────────────────────

/// At the 32 setting every memory form of 9..31 planes is a silent no-op —
/// for `vro_cpyfm` against the screen in both directions and between two such
/// forms, for `vrt_cpyfm` as destination and for `vr_trnfm` as source:
/// the form, the screen and `INTOUT` untouched (O5, reading 1 and 2).
#[test]
fn memory_forms_of_9_to_31_planes_are_refused_at_32() {
    let _g = setting(32);
    let mut ws = Ws::open();
    let mono: Vec<u16> = vec![0xFFFF; 4];
    let msrc = Mfdb { addr: mono.as_ptr() as u64, w: 32, h: 2, wdwidth: 2, stand: 0, nplanes: 1, r1: 0, r2: 0, r3: 0 };
    for planes in 9i16..=31 {
        noise_screen(0x65_7000 + planes as u64);
        let screen_before = screen().snapshot();
        let mut a: Vec<u16> = (0..4096u32).map(|i| (i * 7 + planes as u32) as u16).collect();
        let mut b: Vec<u16> = (0..4096u32).map(|i| (i * 13 + 1) as u16).collect();
        let (a0, b0) = (a.clone(), b.clone());
        let mut fa = Mfdb::mem(a.as_mut_ptr() as *const u8, 32, 4, planes);
        let mut fb = Mfdb::mem(b.as_mut_ptr() as *const u8, 32, 4, planes);
        ws.vro_cpyfm(S_ONLY, [0, 0, 31, 3], [0, 0, 31, 3], &Mfdb::screen(), &mut fa);
        ws.vro_cpyfm(S_ONLY, [0, 0, 31, 3], [5, 5, 36, 8], &fa, &mut Mfdb::screen());
        ws.vro_cpyfm(S_ONLY, [0, 0, 31, 3], [0, 0, 31, 3], &fa, &mut fb);
        ws.vrt_cpyfm(MD_REPLACE, 1, 0, [0, 0, 31, 1], [0, 0, 31, 1], &msrc, &mut fb);
        assert!(ws.pb.intout.iter().all(|&v| v == POISON), "{planes} planes: INTOUT untouched");
        fb.stand = 1;
        let mut dst_mfdb = Mfdb { stand: 7, ..fa };
        ws.vr_trnfm(&fb, &mut dst_mfdb);
        assert_eq!(dst_mfdb.stand, 7, "{planes} planes: vr_trnfm leaves fd_stand alone");
        assert_eq!(a, a0, "{planes} planes: form a untouched");
        assert_eq!(b, b0, "{planes} planes: form b untouched");
        assert_eq!(screen().snapshot(), screen_before, "{planes} planes: screen untouched");
    }
}

/// A 32-plane form at the 16 setting is refused (the cap 16, unchanged), and a
/// 16-plane form at the 16 setting still copies (ADR-024 point 4).
#[test]
fn at_16_planes_the_16_plane_contract_is_unchanged() {
    let _g = setting(16);
    let mut ws = Ws::open();
    noise_screen(0x65_8000);
    let screen_before = screen().snapshot();
    let mut form = Form32::new(16, 2, |_| 0x1234_5678);
    ws.vro_cpyfm(S_ONLY, [0, 0, 15, 1], [0, 0, 15, 1], &Mfdb::screen(), &mut form.mfdb());
    ws.vro_cpyfm(S_ONLY, [0, 0, 15, 1], [0, 0, 15, 1], &form.mfdb(), &mut Mfdb::screen());
    assert!(form.buf[..32].iter().all(|&v| v == 0x1234_5678), "a 32-plane form is refused at 16");
    assert_eq!(screen().snapshot(), screen_before);
    // The 16-plane form: the 16-bit word of each screen pixel.
    let mut words = vec![0u16; 32 + 16];
    let mut m16 = Mfdb::mem(words.as_mut_ptr() as *const u8, 16, 2, 16);
    ws.vro_cpyfm(S_ONLY, [7, 9, 22, 10], [0, 0, 15, 1], &Mfdb::screen(), &mut m16);
    for y in 0..2 {
        for x in 0..16 {
            assert_eq!(words[y * 16 + x], device_to_word(screen().raw(7 + x as i16, 9 + y as i16)));
        }
    }
    assert!(words[32..].iter().all(|&v| v == 0));
    // `v_get_pixel` at 16: `{0, word}`.
    ws.v_clrwk();
    assert_eq!(ws.v_get_pixel(3, 3), (0, CLEARED_WORD_16));
}

/// Below nine planes a memory form stays planar at the 32 setting: a 4-plane
/// copy gives the words it gives at 16 (independent of the screen depth).
#[test]
fn planar_forms_below_nine_planes_copy_alike_at_both_settings() {
    let run = |planes_setting: u16| -> Vec<u16> {
        let _g = setting(planes_setting);
        let mut ws = Ws::open();
        let src: Vec<u16> = (0..64u32).map(|i| (i * 0x9E37) as u16).collect();
        let mut dst: Vec<u16> = vec![0x5555; 64];
        let ms = Mfdb::mem(src.as_ptr() as *const u8, 32, 4, 4);
        let mut md = Mfdb::mem(dst.as_mut_ptr() as *const u8, 32, 4, 4);
        ws.vro_cpyfm(6, [3, 1, 25, 3], [1, 0, 23, 2], &ms, &mut md);
        dst
    };
    let at16 = run(16);
    let at32 = run(32);
    assert_ne!(at16, vec![0x5555; 64], "the planar copy ran");
    assert_eq!(at16, at32);
}

// ── §5: vr_trnfm at 32 planes ───────────────────────────────────────────

/// Device to standard and back is the identity, bit for bit, X included, and
/// so is standard to device and back; `fd_stand` of the destination is not
/// written. Separate buffers and in place.
#[test]
fn vr_trnfm_device_to_standard_and_back_is_the_identity() {
    let _g = setting(32);
    let mut ws = Ws::open();
    // w = 48, h = 5: planesize = 5 * 3 = 15 words, 240 pixels, 960 bytes.
    let (w, h) = (48i16, 5i16);
    let mut rng = Rng(0x65_9000);
    let device: Vec<u32> = (0..240).map(|_| rng.u32()).collect();
    let mut standard = vec![0u16; 15 * 32];
    let mut back = vec![0u32; 240];
    let dev_m = Mfdb { stand: 0, ..Mfdb::mem(device.as_ptr() as *const u8, w, h, 32) };
    let mut std_m = Mfdb { stand: 0, ..Mfdb::mem(standard.as_mut_ptr() as *const u8, w, h, 32) };
    ws.vr_trnfm(&dev_m, &mut std_m);
    assert_eq!(std_m.stand, 0, "fd_stand of the destination is not written");
    let std_src = Mfdb { stand: 1, ..std_m };
    let mut back_m = Mfdb { stand: 1, ..Mfdb::mem(back.as_mut_ptr() as *const u8, w, h, 32) };
    ws.vr_trnfm(&std_src, &mut back_m);
    assert_eq!(back_m.stand, 1, "fd_stand of the destination is not written");
    assert_eq!(back, device, "device → standard → device");
    // Standard → device → standard, from an arbitrary standard form.
    let mut rng = Rng(0x65_9100);
    let std0: Vec<u16> = (0..15 * 32).map(|_| rng.next() as u16).collect();
    let mut dev1 = vec![0u32; 240];
    let mut std1 = vec![0xFFFFu16; 15 * 32];
    ws.vr_trnfm(
        &Mfdb { stand: 1, ..Mfdb::mem(std0.as_ptr() as *const u8, w, h, 32) },
        &mut Mfdb::mem(dev1.as_mut_ptr() as *const u8, w, h, 32),
    );
    ws.vr_trnfm(
        &Mfdb { stand: 0, ..Mfdb::mem(dev1.as_ptr() as *const u8, w, h, 32) },
        &mut Mfdb::mem(std1.as_mut_ptr() as *const u8, w, h, 32),
    );
    assert_eq!(std1, std0, "standard → device → standard (the output is zeroed first)");
    // In place, both directions.
    let mut buf: Vec<u32> = device.clone();
    let m = Mfdb { stand: 0, ..Mfdb::mem(buf.as_mut_ptr() as *const u8, w, h, 32) };
    ws.vr_trnfm(&m, &mut m.clone());
    // SAFETY: `buf` holds 960 bytes, the standard form's 480 words.
    let as_std: Vec<u16> = unsafe { std::slice::from_raw_parts(buf.as_ptr() as *const u16, 480) }.to_vec();
    assert_eq!(as_std, standard, "in place, device → standard");
    let m = Mfdb { stand: 1, ..m };
    ws.vr_trnfm(&m, &mut m.clone());
    assert_eq!(buf, device, "in place, standard → device");
}

/// The plane order of the 32-plane standard form (§5): plane `p` holds bit
/// `31 - p` of the pixel, the leftmost pixel the most significant bit of the
/// word; planes 0..7 are X, 8..15 R, 16..23 G, 24..31 B.
#[test]
fn vr_trnfm_plane_p_holds_bit_31_minus_p() {
    let _g = setting(32);
    let mut ws = Ws::open();
    // One row of 16 pixels: planesize 1 word.
    let mut device = [0u32; 16];
    device[0] = 0x8000_0001; // X bit 7 and B bit 0 of the leftmost pixel
    device[15] = 0x0080_0000; // R bit 7 of the rightmost pixel
    device[5] = pixel_from_description(0, 0x01, 0); // G bit 0 of pixel 5
    let mut standard = vec![0u16; 32];
    ws.vr_trnfm(
        &Mfdb::mem(device.as_ptr() as *const u8, 16, 1, 32),
        &mut Mfdb::mem(standard.as_mut_ptr() as *const u8, 16, 1, 32),
    );
    let mut want = [0u16; 32];
    want[0] = 0x8000; // plane 0 = bit 31, pixel 0
    want[31] = 0x8000; // plane 31 = bit 0, pixel 0
    want[8] = 0x0001; // plane 8 = bit 23 (R7), pixel 15
    want[23] = 0x0400; // plane 23 = bit 8 (G0), pixel 5
    assert_eq!(standard, want);
}

/// At the 16 setting the transform keeps its 16-plane gate: a 16-plane form
/// still goes through `vr_trnfm16` (the existing identity holds).
#[test]
fn vr_trnfm_at_16_planes_still_transforms_16_plane_forms() {
    let _g = setting(16);
    let mut ws = Ws::open();
    let mut rng = Rng(0x65_9200);
    let device: Vec<u16> = (0..32).map(|_| rng.next() as u16).collect();
    let mut standard = vec![0u16; 32];
    let mut back = vec![0u16; 32];
    ws.vr_trnfm(
        &Mfdb::mem(device.as_ptr() as *const u8, 16, 2, 16),
        &mut Mfdb::mem(standard.as_mut_ptr() as *const u8, 16, 2, 16),
    );
    assert_ne!(standard, device);
    ws.vr_trnfm(
        &Mfdb { stand: 1, ..Mfdb::mem(standard.as_ptr() as *const u8, 16, 2, 16) },
        &mut Mfdb::mem(back.as_mut_ptr() as *const u8, 16, 2, 16),
    );
    assert_eq!(back, device);
}

// ── §6: v_get_pixel ─────────────────────────────────────────────────────

/// The answer shape at 32: `{high, low}` of the masked pixel; a cleared
/// screen `{0x00FF, 0xFFFF}`; X never shows; a pen pixel comes back whole.
#[test]
fn v_get_pixel_at_32_answers_the_high_and_the_low_word() {
    let _g = setting(32);
    let mut ws = Ws::open();
    ws.v_clrwk();
    assert_eq!(ws.v_get_pixel(10, 10), (0x00FF, 0xFFFF), "a cleared screen");
    assert_eq!(ws.pb.contrl[4], 2);
    screen().put_device_pixel(11, 10, 0xA5_123456);
    assert_eq!(ws.v_get_pixel(11, 10), (0x0012, 0x3456), "X is masked");
    ws.vs_color(30, [17, 600, 999]);
    ws.solid_fill(30);
    ws.vr_recfl(20, 20, 20, 20);
    let (hi, lo) = ws.v_get_pixel(20, 20);
    assert_eq!(((hi as u32) << 16) | lo as u32, pixel32([17, 600, 999]));
}

// ── §7: save buffers ────────────────────────────────────────────────────

/// `bb_save_words` follows the setting: `fd_wdwidth * 8 * h` at 16 (unchanged),
/// `fd_wdwidth * 16 * h` at 32 — EmuTOS' `((w + 15) / 16) * 2 * h * gl_nplanes`
/// bytes (`gemmnext.c:157`) over 4. The whole 640 x 480 screen: 614 400 bytes at
/// 16, 1 228 800 at 32.
#[test]
fn bb_save_words_pins_the_size_at_both_settings() {
    for (planes, per_px_bytes) in [(16u16, 2usize), (32, 4)] {
        let _g = setting(planes);
        let b: &dyn RasterBackend = screen();
        for (w, h) in [(1i16, 1i16), (15, 3), (16, 3), (17, 2), (640, 480), (1920, 1080), (0, 5), (5, 0)] {
            let wd = (w.max(0) as usize).div_ceil(16);
            let bytes = wd * 2 * h.max(0) as usize * planes as usize;
            assert_eq!(bytes, wd * 16 * h.max(0) as usize * per_px_bytes);
            assert_eq!(tos_api::vdi::bb_save_words(b, w, h) * 4, bytes, "{planes} planes {w}x{h}");
        }
        assert_eq!(tos_api::vdi::bb_save_words(b, 640, 480) * 4, if planes == 16 { 614_400 } else { 1_228_800 });
        assert_eq!(tos_api::aes::gsxif::gsx_mcalc(b), tos_api::vdi::bb_save_words(b, W as i16, H as i16) * 4);
    }
}

/// A `bb_save` at 32 fills exactly `bb_save_words` words — the stamp and the
/// size come from one source, so nothing lands past the buffer — and the
/// matching `bb_restore` puts the raw store words back bit for bit.
#[test]
fn bb_save_at_32_stays_inside_bb_save_words_and_restores_exactly() {
    let _g = setting(32);
    // Opened once: `v_opnwk` clears the screen.
    let mut ws = Ws::open();
    let b: &dyn RasterBackend = screen();
    for (x, y, w, h) in [(13i16, 7i16, 37i16, 9i16), (0, 0, 16, 1), (300, 190, 20, 10), (0, 0, W as i16, H as i16)] {
        noise_screen(0x65_A000 + w as u64);
        let before = screen().snapshot();
        let words = tos_api::vdi::bb_save_words(b, w, h);
        let mut buf = vec![CANARY; words + TAIL];
        // SAFETY: `buf` holds `bb_save_words(w, h)` words (plus the canary).
        unsafe { tos_api::vdi::bb_save(b, buf.as_mut_ptr(), x, y, w, h) };
        assert!(buf[words..].iter().all(|&v| v == CANARY), "{w}x{h}: nothing past bb_save_words");
        // The image is the 32-plane device form, row `wdwidth * 16`.
        let stride = (w as usize).div_ceil(16) * 16;
        let cw = w.min(W as i16 - x) as usize;
        let ch = h.min(H as i16 - y) as usize;
        for r in 0..ch {
            for c in 0..cw {
                assert_eq!(buf[r * stride + c], screen().raw(x + c as i16, y + r as i16), "{w}x{h} ({c},{r})");
            }
        }
        // Scribble over the area, then restore.
        ws.solid_fill(1);
        ws.vr_recfl(x, y, x + w - 1, y + h - 1);
        // SAFETY: `buf` holds the image `bb_save` stored for this rectangle.
        unsafe { tos_api::vdi::bb_restore(b, buf.as_ptr(), x, y, w, h, ClipRect::full(W, H)) };
        let after = screen().snapshot();
        let diff: Vec<(usize, usize, u32, u32)> = (0..after.len())
            .filter(|&i| after[i] != before[i])
            .map(|i| (i % W as usize, i / W as usize, before[i], after[i]))
            .take(8)
            .collect();
        assert!(diff.is_empty(), "{w}x{h}: restored bit for bit, X included; first differences {diff:x?}");
    }
}

/// The colour-icon blit keeps its 16-bit form at 32 (reading 4): each 16-bit
/// icon pixel is expanded to its device pixel and `S_AND_D` runs against the
/// raw 32-bit screen word, row stride `((w + 15) / 16) * 16` 16-bit pixels.
/// The icon buffer is exactly that long; a 32-plane stride would read past it
/// into the trailing words, which would turn the result black.
#[test]
fn the_colour_icon_blit_reads_its_16_bit_form_at_32() {
    let _g = setting(32);
    let _ws = Ws::open();
    let b: &dyn RasterBackend = screen();
    let (w, h) = (24i16, 4i16); // stride 32 pixels
    let n = 32 * h as usize;
    let mut rng = Rng(0x65_B000);
    let mut icon: Vec<u16> = (0..n).map(|_| rng.next() as u16 ).collect();
    icon.extend(std::iter::repeat_n(0u16, 4 * n)); // would AND the screen to 0
    noise_screen(0x65_B100);
    let before = screen().snapshot();
    let raw0 = |x: usize, y: usize| before[y * W as usize + x];
    // SAFETY: `icon` holds `((w + 15) / 16) * 16 * h` u16s (and more).
    unsafe { tos_api::vdi::blit_device_mem(b, icon.as_ptr(), Rect { x: 50, y: 60, w, h }, S_AND_D, ClipRect::full(W, H)) };
    for y in 0..h as usize {
        for x in 0..w as usize {
            let want = raw0(50 + x, 60 + y) & word_to_device(icon[y * 32 + x]);
            assert_eq!(screen().raw(50 + x as i16, 60 + y as i16), want, "({x},{y})");
        }
    }
    assert_eq!(screen().raw(50 + w, 60), raw0(50 + w as usize, 60));
}
