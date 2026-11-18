//! V64 — the pens at the 32-plane setting: rules P1-P9 and F1-F5 of
//! `docs/api/vdi/raster/_raster.md` § "32-plane setting: pens, and every
//! place a pen becomes a pixel (ADR-038 point 4, V64)", pinned at 32 planes,
//! with the unchanged 16-plane behaviour beside them (ADR-038 point 11).
//!
//! - P1-P5: `vs_color` stores the raw request and builds the pen's pixel
//!   `0x00RRGGBB` with `q = (c * 255 + 500) / 1000`; `vq_color` answers the
//!   request (flag 0) or `(q * 1000 + 127) / 255` per gun (flag != 0); the
//!   error input of both is the 16-plane one. Every request 0..1000 of every
//!   gun goes through the VDI and is checked against the formulas, which the
//!   fixture writes from the spec (`vdi_planes32_common::q8`, `a8`), not from
//!   the code under test.
//! - P6, P7: the default pens and their exact read-back at 32 planes.
//! - P8: the calling workstation's table; the flood fill's pen colour from
//!   the physical workstation's.
//! - P9: a 32-plane pen pixel projected to 5 bits per gun is within one step
//!   of the 16-plane pen, for every request.
//! - F1-F3: the flood fill compares the masked 24-bit pixel at 32 planes, so
//!   it stops at a pen that differs only in the low 3 bits of one gun (the
//!   spec's test pens `(17,0,0)` and `(25,0,0)`, TASK/V64 acceptance), and the
//!   `OVERLAY_BIT` mask of 16 planes does not run (a blue bit at 32).
//! - F5: `v_get_pixel` kept the interim answer of V63 until V65; since V65 it
//!   answers the high and the low word of the 24-bit pixel at 32 planes
//!   (ADR-038 point 7), so the two test pens read apart; the 16-plane answer
//!   is unchanged.
//!
//! The backend is the kernel's `VdiScreen` over a host store, shared with
//! the V63 tests (`vdi_planes32_common`).
//!
//! Ticket: `TASK/V64.md`.

#![cfg(all(target_arch = "aarch64", any(target_os = "macos", target_os = "linux")))]
#![allow(clippy::too_many_arguments)]

mod vdi_planes32_common;

use tos_api::vdi::backend::{Pel, RasterBackend};
use vdi_planes32_common::*;

/// The pen the request-range tests set (an extended pen, default black).
const PEN: i16 = 20;
/// The physical workstation's handle (`v_opnwk`; `tos_api::vdi::vwk::PHYS_HANDLE`).
const PHYS: i16 = 1;

impl Ws {
    /// `vs_color(idx, r, g, b)` on this workstation.
    fn vs_color(&mut self, idx: i16, rgb: [i16; 3]) {
        self.call(14, 0, &[idx, rgb[0], rgb[1], rgb[2]], &[]);
    }
    /// `vs_color` on the physical workstation, whose table the flood fill's
    /// pen colour reads (P8, `vdi/vdi_fill.c:978-985`).
    fn vs_color_phys(&mut self, idx: i16, rgb: [i16; 3]) {
        self.pb.call(14, 0, PHYS, &[idx, rgb[0], rgb[1], rgb[2]], &[]);
    }
    /// `vq_color(idx, flag)` → `INTOUT[0..4]` and `CONTRL[4]`.
    fn vq_color(&mut self, idx: i16, flag: i16) -> ([i16; 4], i16) {
        self.call(26, 0, &[idx, flag], &[]);
        let o = &self.pb.intout;
        ([o[0], o[1], o[2], o[3]], self.pb.contrl[4])
    }
    /// The pixel pen `pen` draws: a one-pixel solid fill at `(x, y)`, read
    /// back raw from the store.
    fn drawn(&mut self, pen: i16, x: i16, y: i16) -> u32 {
        self.solid_fill(pen);
        self.vr_recfl(x, y, x, y);
        screen().raw(x, y)
    }
}

/// A request with gun `g` (0 red, 1 green, 2 blue) at `c`, the others 0.
fn gun(g: usize, c: i16) -> [i16; 3] {
    let mut rgb = [0; 3];
    rgb[g] = c;
    rgb
}

/// The byte of gun `g` in a pixel `0x00RRGGBB`.
fn byte(px: u32, g: usize) -> u32 {
    (px >> (16 - 8 * g)) & 0xFF
}

// ── P1-P5: the formulas over the request range ──────────────────────────

/// The documented values of both formulas (P2, P5), through `vs_color`,
/// `vq_color` and a draw at 32 planes: `q(0) = 0`, `q(1) = 0`, `q(2) = 1`,
/// `q(500) = 128`, `q(999) = 255`, `q(1000) = 255`; `a(0) = 0`, `a(1) = 4`,
/// `a(128) = 502`, `a(254) = 996`, `a(255) = 1000`.
#[test]
fn the_documented_values_of_both_formulas_at_32_planes() {
    let _g = setting(32);
    let mut ws = Ws::open();
    for (c, q) in [(0i16, 0u32), (1, 0), (2, 1), (500, 128), (999, 255), (1000, 255)] {
        ws.vs_color(PEN, [c, c, c]);
        assert_eq!(ws.drawn(PEN, 10, 10), q << 16 | q << 8 | q, "q({c})");
    }
    // `a(q)` for the request whose `q` is the documented one.
    for (c, q, a) in [(0i16, 0u32, 0i16), (4, 1, 4), (502, 128, 502), (996, 254, 996), (1000, 255, 1000)] {
        ws.vs_color(PEN, [c, 0, 0]);
        assert_eq!(byte(ws.drawn(PEN, 10, 10), 0), q, "q({c})");
        assert_eq!(ws.vq_color(PEN, 1).0, [PEN, a, 0, 0], "a({q}) for request {c}");
    }
}

/// Every request 0..1000 of every gun at both settings: the pen's pixel,
/// `vq_color(flag 0)` and `vq_color(flag 1)`. At 32 planes the 8-bit rule
/// (P2, P3, P5) and its bound `|a(q(c)) - c| <= 2`; at 16 the 5-bit rule of
/// EmuTOS, unchanged (`vdi/vdi_col.c:588-591,601-614,804-807`).
#[test]
fn vs_color_and_vq_color_over_the_request_range() {
    for planes in [32u16, 16] {
        let _g = setting(planes);
        let mut ws = Ws::open();
        for g in 0..3 {
            for c in 0..=1000i16 {
                ws.vs_color(PEN, gun(g, c));
                let px = ws.drawn(PEN, 20, 20);
                let (set, n) = ws.vq_color(PEN, 0);
                assert_eq!((set, n), ([PEN, gun(g, c)[0], gun(g, c)[1], gun(g, c)[2]], 4), "{planes}: flag 0, gun {g}, {c}");
                let (real, _) = ws.vq_color(PEN, 1);
                if planes == 32 {
                    assert_eq!(px, pixel32([0, 1, 2].map(|i| gun(g, c)[i] as i32)), "32: pixel, gun {g}, {c}");
                    let want = a8(q8(c as i32));
                    assert_eq!(real, [PEN, gun(g, want)[0], gun(g, want)[1], gun(g, want)[2]], "32: flag 1, gun {g}, {c}");
                    assert!((want - c).abs() <= 2, "32: |a(q({c})) - {c}| = {}", (want - c).abs());
                } else {
                    let w = word16([0, 1, 2].map(|i| gun(g, c)[i] as i32));
                    assert_eq!(px, word_to_device(w), "16: pixel, gun {g}, {c}");
                    let want = a5(q5(c as i32));
                    assert_eq!(real, [PEN, gun(g, want)[0], gun(g, want)[1], gun(g, want)[2]], "16: flag 1, gun {g}, {c}");
                }
            }
        }
    }
}

/// `q(a(q)) = q` for every gun byte 0..255 at 32 planes (P5): a pen set to
/// the realised value of `q` draws the byte `q` again, and reads it back.
#[test]
fn the_realised_value_round_trips_every_gun_byte_at_32_planes() {
    let _g = setting(32);
    let mut ws = Ws::open();
    for q in 0..=255u32 {
        let a = a8(q);
        ws.vs_color(PEN, [a, a, a]);
        assert_eq!(ws.drawn(PEN, 30, 30), q << 16 | q << 8 | q, "q(a({q}))");
        assert_eq!(ws.vq_color(PEN, 1).0, [PEN, a, a, a], "a({q})");
    }
}

/// A request outside 0..1000 is stored raw and answered raw by flag 0, and
/// clamped for the pixel and for flag 1 (P1, P4), at both settings
/// (`vdi/vdi_col.c:634-646,821-826`).
#[test]
fn an_unclamped_request_is_answered_raw_and_drawn_clamped() {
    for planes in [32u16, 16] {
        let _g = setting(planes);
        let mut ws = Ws::open();
        ws.vs_color(PEN, [-50, 1500, 0]);
        assert_eq!(ws.vq_color(PEN, 0).0, [PEN, -50, 1500, 0], "{planes}: flag 0");
        assert_eq!(ws.vq_color(PEN, 1).0, [PEN, 0, 1000, 0], "{planes}: flag 1");
        assert_eq!(ws.drawn(PEN, 10, 10) & COLOUR, 0x0000_FF00, "{planes}: pixel");
        ws.vs_color(PEN, [i16::MIN, i16::MAX, 1001]);
        assert_eq!(ws.vq_color(PEN, 0).0, [PEN, i16::MIN, i16::MAX, 1001], "{planes}: flag 0, extremes");
        assert_eq!(ws.vq_color(PEN, 1).0, [PEN, 0, 1000, 1000], "{planes}: flag 1, extremes");
    }
}

/// The error input, as at 16 planes (P1, P4; `vdi/vdi_col.c:627-632,854-860`):
/// `vs_color` outside `0 <= idx < 256` changes no pen and no request;
/// `vq_color` there answers `INTOUT[0] = -1`, `INTOUT[1..3] = 0`, four
/// outputs, for either flag.
#[test]
fn the_error_input_of_vs_color_and_vq_color_is_unchanged() {
    for planes in [32u16, 16] {
        let _g = setting(planes);
        let mut ws = Ws::open();
        let pens_before: Vec<_> = (0..256).map(|i| ws.vq_color(i, 0).0).collect();
        let real_before: Vec<_> = (0..256).map(|i| ws.vq_color(i, 1).0).collect();
        for idx in [-1i16, 256, i16::MIN, i16::MAX] {
            ws.vs_color(idx, [500, 500, 500]);
        }
        assert_eq!((0..256).map(|i| ws.vq_color(i, 0).0).collect::<Vec<_>>(), pens_before, "{planes}: requests");
        assert_eq!((0..256).map(|i| ws.vq_color(i, 1).0).collect::<Vec<_>>(), real_before, "{planes}: pens");
        for idx in [-1i16, 256, i16::MIN, i16::MAX] {
            for flag in [0i16, 1] {
                assert_eq!(ws.vq_color(idx, flag), ([-1, 0, 0, 0], 4), "{planes}: vq_color({idx}, {flag})");
            }
        }
    }
}

// ── P6, P7: the default pens ─────────────────────────────────────────────

/// The default pens at 32 planes (P6): the pixels the spec computes for
/// pens 0, 1, 2, 8, 9 and 10, and for every pen of the fixture's default
/// table; pen 255 stays 0 (`vdi/vdi_control.c:360`). Every default pen reads
/// back exactly as requested at 32 planes (P7), which the 5-bit rule does
/// not do at 16 (`a5(q5(733)) = 742`).
#[test]
fn the_default_pens_at_32_planes() {
    let _g = setting(32);
    let mut ws = Ws::open();
    for (pen, px) in [(0i16, 0x00FF_FFFFu32), (1, 0), (2, 0x00FF_0000), (8, 0x00BB_BBBB), (9, 0x0088_8888), (10, 0x00AA_0000)] {
        assert_eq!(ws.drawn(pen, 40, 40), px, "pen {pen}");
    }
    for pen in 0..256usize {
        assert_eq!(ws.drawn(pen as i16, 41, 41), default_pen_pixel(32, pen), "pen {pen}");
        let (set, _) = ws.vq_color(pen as i16, 0);
        assert_eq!(ws.vq_color(pen as i16, 1).0, set, "pen {pen}: realised = requested (P7)");
    }
    // The 16 component values of the 256 default triples (P7).
    for c in [0i16, 67, 133, 200, 267, 333, 400, 467, 533, 600, 667, 733, 800, 867, 933, 1000] {
        ws.vs_color(PEN, [c, c, c]);
        assert_eq!(ws.vq_color(PEN, 1).0, [PEN, c, c, c], "a(q({c}))");
    }
    drop(ws);
    drop(_g);
    let _g = setting(16);
    let mut ws = Ws::open();
    assert_eq!(ws.vq_color(8, 1).0, [8, 742, 742, 742], "16 planes: pen 8 (733) reads back 5-bit quantised");
}

// ── P8: whose table ──────────────────────────────────────────────────────

/// `vs_color` writes only the calling workstation's table at 32 planes as
/// at 16 (P8, `vdi/vdi_col.c:620-649`): a second workstation draws its own
/// default for the same pen.
#[test]
fn vs_color_changes_only_the_calling_workstation_at_32_planes() {
    let _g = setting(32);
    let mut a = Ws::open();
    let mut b = Ws::open();
    a.vs_color(PEN, [17, 0, 0]);
    assert_eq!(a.drawn(PEN, 50, 50), 0x0004_0000, "the calling workstation's pen");
    assert_eq!(b.drawn(PEN, 51, 50), default_pen_pixel(32, PEN as usize), "the other workstation's pen");
    assert_eq!(b.vq_color(PEN, 0).0, [PEN, 0, 0, 0], "the other workstation's request");
}

// ── P9: 16 against 32 ────────────────────────────────────────────────────

/// For every request of every gun the 32-plane pen pixel, projected to the
/// top 5 bits of each gun, is within one 5-bit step of the 16-plane pen
/// (P9, ADR-038 point 13 (ii)); the bound 1 is reached (`c = 17`).
/// `v_get_pixel` at 32 planes answers the pen pixel itself as its high and
/// low word (ADR-038 point 7, V65; until V65 the projection, F5), at 16 the
/// 16-bit word.
#[test]
fn pens_at_16_and_32_planes_agree_within_one_5_bit_step() {
    let mut at = [Vec::new(), Vec::new()];
    for (k, planes) in [16u16, 32].into_iter().enumerate() {
        let _g = setting(planes);
        let mut ws = Ws::open();
        for g in 0..3 {
            for c in 0..=1000i16 {
                ws.vs_color(PEN, gun(g, c));
                let px = ws.drawn(PEN, 60, 60);
                let (i0, w) = ws.v_get_pixel(60, 60);
                if planes == 32 {
                    assert_eq!(i0, (px >> 16) as i16, "{planes}: INTOUT[0]");
                    assert_eq!(w, px as u16, "{planes}: v_get_pixel of gun {g}, {c}");
                } else {
                    assert_eq!(i0, 0, "{planes}: INTOUT[0]");
                    assert_eq!(w, device_to_word(px), "{planes}: v_get_pixel of gun {g}, {c}");
                }
                at[k].push(px);
            }
        }
    }
    let mut widest = 0;
    for (i, (p16, p32)) in at[0].iter().zip(&at[1]).enumerate() {
        let (a, b) = (guns5(*p16), guns5(*p32));
        let step = (a.0 - b.0).abs().max((a.1 - b.1).abs()).max((a.2 - b.2).abs());
        assert!(step <= 1, "request {i}: 0x{p16:08X} at 16, 0x{p32:08X} at 32: {step} steps");
        widest = widest.max(step);
    }
    assert_eq!(widest, 1, "the bound 1 is reached");
    assert_eq!((guns5(at[0][17]).0, guns5(at[1][17]).0), (1, 0), "c = 17: 5-bit red 1 at 16, 0 at 32");
}

// ── F1-F3: the flood fill at 32 planes ───────────────────────────────────

/// The spec's test pens: one 16-plane word, two 32-plane pixels that differ
/// only in bits 0..2 of red (`_raster.md` § 4, "Test pens").
const RING: [i16; 3] = [17, 0, 0];
const INSIDE: [i16; 3] = [25, 0, 0];
const RING_PEN: i16 = 20;
const INSIDE_PEN: i16 = 21;

#[test]
fn the_test_pens_are_one_colour_at_16_and_two_at_32_planes() {
    for planes in [16u16, 32] {
        let _g = setting(planes);
        let mut ws = Ws::open();
        ws.vs_color(RING_PEN, RING);
        ws.vs_color(INSIDE_PEN, INSIDE);
        let (ring, inside) = (ws.drawn(RING_PEN, 70, 70), ws.drawn(INSIDE_PEN, 71, 70));
        if planes == 32 {
            assert_eq!((ring, inside), (0x0004_0000, 0x0006_0000), "32 planes");
        } else {
            assert_eq!((ring, inside), (word_to_device(0x0800), word_to_device(0x0800)), "16 planes");
        }
        // `read_pixel`'s reverse map follows the pens (the first minimum
        // wins): two pens at 32, one colour at 16 (transcribed backend).
        let scr = screen();
        assert_eq!(scr.read_pixel(71, 70), if planes == 32 { INSIDE_PEN as Pel } else { RING_PEN as Pel }, "{planes}: read_pixel");
    }
}

/// The box `(x0, y0)..=(x1, y1)` filled with pen `inside`, framed by a
/// two-pixel ring of pen `ring`, on a cleared screen; both pens set on the
/// drawing and on the physical workstation. Returns the ring's outer box.
fn ringed_box(ws: &mut Ws, ring: (i16, [i16; 3]), inside: (i16, [i16; 3])) -> (i16, i16, i16, i16) {
    ws.v_clrwk();
    for (pen, rgb) in [ring, inside] {
        ws.vs_color(pen, rgb);
        ws.vs_color_phys(pen, rgb);
    }
    ws.vswr_mode(1);
    ws.solid_fill(ring.0);
    ws.vr_recfl(38, 28, 101, 71);
    ws.solid_fill(inside.0);
    ws.vr_recfl(40, 30, 99, 69);
    (38, 28, 101, 71)
}

/// Which part of the ringed box a store index is in.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Part {
    Inside,
    Ring,
    Outside,
}

fn part(i: usize) -> Part {
    let (x, y) = ((i % W as usize) as i16, (i / W as usize) as i16);
    if (40..=99).contains(&x) && (30..=69).contains(&y) {
        Part::Inside
    } else if (38..=101).contains(&x) && (28..=71).contains(&y) {
        Part::Ring
    } else {
        Part::Outside
    }
}

/// Run `v_contourfill(60, 50, intin0)` with the fill pen 2 over the ringed
/// box; returns the parts the fill changed, each with the pixel it left.
fn fill_parts(ws: &mut Ws, intin0: i16) -> Vec<(Part, bool)> {
    let scr = screen();
    let before = scr.snapshot();
    ws.solid_fill(2);
    ws.v_contourfill(60, 50, intin0);
    let after = scr.snapshot();
    let red = scr.pel_to_rgb(2);
    let mut seen: Vec<(Part, bool)> = Vec::new();
    for (i, (b, a)) in before.iter().zip(&after).enumerate() {
        if a != b {
            assert_eq!(*a, red, "pixel {i}: changed to something other than the fill pen");
        }
        let entry = (part(i), a != b);
        if !seen.contains(&entry) {
            seen.push(entry);
        }
    }
    seen.sort_by_key(|(p, c)| (*p as u8, *c));
    seen
}

/// At 32 planes a contour fill stops at a pen that differs from the region
/// only in the low 3 bits of one gun (TASK/V64 acceptance, F1-F3): the
/// boundary fill with the ring pen as boundary (`INTIN[0] >= 0`, the
/// physical workstation's pixel) and the fill-while-same (`INTIN[0] < 0`)
/// each fill exactly the inside. At 16 planes the two pens are one colour:
/// the boundary fill finds its seed span of the boundary colour and draws
/// nothing; the fill-while-same runs over the ring up to the cleared
/// screen (EmuTOS, `vdi/vdi_fill.c:755,763,990-991`).
#[test]
fn a_contour_fill_stops_at_a_pen_differing_in_the_low_3_bits_at_32_planes() {
    use Part::*;
    for planes in [32u16, 16] {
        for intin0 in [RING_PEN, -1] {
            let _g = setting(planes);
            let mut ws = Ws::open();
            ringed_box(&mut ws, (RING_PEN, RING), (INSIDE_PEN, INSIDE));
            let got = fill_parts(&mut ws, intin0);
            let want = match (planes, intin0 >= 0) {
                (32, _) => vec![(Inside, true), (Ring, false), (Outside, false)],
                (_, true) => vec![(Inside, false), (Ring, false), (Outside, false)],
                (_, false) => vec![(Inside, true), (Ring, true), (Outside, false)],
            };
            assert_eq!(got, want, "{planes} planes, INTIN[0] = {intin0}");
        }
    }
}

/// The `OVERLAY_BIT` mask of 16 planes does not run at 32 (F2): pens
/// `(0,0,0)` and `(0,0,125)` are the pixels `0x000000` and `0x000020`, which
/// differ only in blue bit 5 (`q(125) = 32`). A boundary fill with the black
/// ring as boundary and a fill-while-same both fill exactly the inside; with
/// the mask the inside would equal the boundary and neither would.
#[test]
fn the_overlay_bit_mask_does_not_run_at_32_planes() {
    use Part::*;
    let _g = setting(32);
    for intin0 in [1i16, -1] {
        let mut ws = Ws::open();
        ringed_box(&mut ws, (1, [0, 0, 0]), (INSIDE_PEN, [0, 0, 125]));
        assert_eq!(screen().raw(60, 50), 0x0000_0020, "the inside pen's pixel");
        assert_eq!(screen().raw(38, 28), 0, "the ring pen's pixel");
        assert_eq!(fill_parts(&mut ws, intin0), vec![(Inside, true), (Ring, false), (Outside, false)], "INTIN[0] = {intin0}");
    }
}

/// `v_get_pixel` at 32 planes answers the high and the low word of the
/// masked 24-bit pixel (ADR-038 point 7, V65, which replaced V63's interim
/// projection F5), so the two test pens read apart as the flood fill tells
/// them apart: `{4, 0}` and `{6, 0}` (`q(17) = 4`, `q(25) = 6`); at 16 both
/// answer `{0, 0x0800}`.
#[test]
fn v_get_pixel_answers_the_wide_pixel_at_32_planes() {
    for planes in [32u16, 16] {
        let _g = setting(planes);
        let mut ws = Ws::open();
        ws.vs_color(RING_PEN, RING);
        ws.vs_color(INSIDE_PEN, INSIDE);
        ws.drawn(RING_PEN, 80, 80);
        ws.drawn(INSIDE_PEN, 81, 80);
        screen().put_device_pixel(82, 80, 0xA500_0000 | screen().raw(81, 80));
        let (ring, inside) = if planes == 32 { ((4, 0), (6, 0)) } else { ((0, 0x0800), (0, 0x0800)) };
        assert_eq!(ws.v_get_pixel(80, 80), ring, "{planes}: ring pen");
        assert_eq!(ws.v_get_pixel(81, 80), inside, "{planes}: inside pen");
        if planes == 32 {
            assert_eq!(ws.v_get_pixel(82, 80), inside, "32: X noise is masked");
        }
    }
}

// ── the VT52 console's colour source (TASK/V64, slice cut point 2) ───────

/// The VT52 console reads its colours from the physical workstation's pens
/// (`conout.c:162-166`; foreground register 15 = pen 255, background
/// register 0 = pen 0, `tos-api/src/vdi/escape.rs` `REV_MAP_COL`). At 16
/// planes that is the 16-bit word as before; at 32 the pen's pixel, written
/// as it is by the 32-bit cell writer (TASK/V67, `docs/api/bios/_vt52-32plane.md`
/// C1, C2; until V67 the 16-bit writer took its top 5 bits per gun). With the
/// request `(17, 0, 0)` the two differ: 5-bit red 1 at 16, 8-bit red 4 at 32
/// (`q(17) = 4`).
#[test]
fn the_vt52_console_takes_its_colours_from_the_pen_table() {
    for planes in [16u16, 32] {
        let _g = setting(planes);
        let mut ws = Ws::open();
        ws.vs_color_phys(255, RING);
        let scr = screen();
        tos_api::vdi::alpha_init(scr);
        let cleared = scr.snapshot();
        tos_api::vdi::alpha_conout(scr, b'W');
        let after = scr.snapshot();
        let (fg, bg) = if planes == 32 {
            (pixel32([17, 0, 0]), pixel32([1000, 1000, 1000]))
        } else {
            (word_to_device(word16([17, 0, 0])), word_to_device(word16([1000, 1000, 1000])))
        };
        assert!(cleared.iter().all(|&p| p == bg), "{planes}: the console clears to pen 0, 0x{bg:08X}");
        let drawn: Vec<u32> = cleared.iter().zip(&after).filter(|(c, a)| c != a).map(|(_, &a)| a).collect();
        assert!(drawn.len() > 10, "{planes}: the glyph drew {} pixels", drawn.len());
        assert!(drawn.iter().all(|&p| p == fg), "{planes}: glyph pixels are pen 255, 0x{fg:08X}");
    }
}
