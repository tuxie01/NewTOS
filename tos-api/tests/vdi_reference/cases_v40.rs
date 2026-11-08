//! V40 (TASK/V40.md): `LSTLIN`, the flag `abline`'s XOR rule reads.
//!
//! `LSTLIN` is one global line-A word (`bios/lineavars.S:288`). `polyline`
//! alone writes it — FALSE as its loop starts, TRUE before its last segment
//! (`vdi/vdi_line.c:1106-1108`) — and `abline` reads it: in XOR mode a
//! non-vertical line drawn while it is FALSE loses its far `x` (`:2319-2325`).
//! The text underline calls `abline` directly (`vdi/vdi_text.c:527,534`), so it
//! is one pixel shorter after a polyline of fewer than two points (TVK) and of
//! full length after any other (TVJ). Every polyline counts: `v_pline` of
//! width 1, the perimeters of `polygon` (fills, circles, wide-line boxes,
//! arrowheads), `v_bar`, `v_rbox`, markers; and only when it runs — a width-3
//! `v_pline` of one point or of one zero-length segment, `v_fillarea` without
//! perimeter, a polygon outside the clip band, a curve the clip excludes and
//! `vr_recfl` leave the flag alone. It is not per workstation: a line on
//! another handle sets it for ours.
//!
//! Source of every expected value: the probe cases TVJ/TVK (`tv`, V30) and
//! L40A–L40Zb, L41A–L41D (`t40`, added by V40) of
//! `tosdump/probes/vdprobe.c` on EmuTOS 1a7484ab, Hatari Falcon030
//! truecolor, run `measurements/measurement-2026-11-08-v40/RUNINFO.TXT`
//! (`probe/e1`): one FNV-1a over each grab rectangle's screen words and the
//! count of non-white words (`method/gen-grabs.py`, the format of
//! `cases_v37`). L40V/L40W (`objc_draw`) need the AES and are not replayed
//! here; the AES writers are tested beside them (`tos-api/src/aes/object.rs`,
//! `menu.rs`), the AES's `v_pline` counterpart below
//! ([`v40_aes_draw_line_sets_lstlin`]).
//!
//! Every case sets the flag first (a 2-point `v_pline` for TRUE, a 1-point one
//! for FALSE, in replace mode at y 300), runs the call under test, then draws
//! 'ABCDEFGH' underlined in XOR at (100,100).
//!
//! Hand-written, like `cases_v20`..`cases_v37`.

use crate::harness::{raster, reset_vdi, screen_word, workstation, Call, Pb, VDI_LOCK};
use tos_api::vdi::backend::{ClipRect, RasterBackend, WriteMode};

/// `(name, (x, y, w, h), FNV-1a of the words, non-white words)` of `method/gen-grabs.py`.
type GrabHash = (&'static str, (i16, i16, i16, i16), u64, u32);

/// Full-length XOR underline (TVJ) and the one a pixel shorter (TVK).
const FULL: u64 = 0x0f4a_f1bd_af70_383b;
const SHORT: u64 = 0x83de_c980_f10b_b581;

#[rustfmt::skip]
const GRABS: &[GrabHash] = &[
    ("TVJ", (80, 80, 120, 40), FULL, 159),
    ("TVK", (80, 80, 120, 40), SHORT, 158),
    ("L40A", (80, 80, 120, 40), SHORT, 158),
    ("L40B", (80, 80, 120, 40), SHORT, 158),
    ("L40C", (80, 80, 120, 40), FULL, 159),
    ("L40D", (80, 80, 120, 40), FULL, 159),
    ("L40E", (80, 80, 120, 40), SHORT, 158),
    ("L40F", (80, 80, 120, 40), FULL, 159),
    ("L40G", (80, 80, 120, 40), FULL, 159),
    ("L40H", (80, 80, 120, 40), SHORT, 158),
    ("L40I", (80, 80, 120, 40), SHORT, 158),
    ("L40J", (80, 80, 120, 40), SHORT, 158),
    ("L40K", (80, 80, 120, 40), FULL, 159),
    ("L40L", (80, 80, 120, 40), FULL, 159),
    ("L40M", (80, 80, 120, 40), SHORT, 158),
    ("L40N", (80, 80, 120, 40), FULL, 159),
    ("L40O", (80, 80, 120, 40), SHORT, 158),
    ("L40P", (80, 80, 120, 40), FULL, 159),
    ("L40Q", (80, 80, 120, 40), FULL, 159),
    ("L40R", (80, 80, 120, 40), FULL, 159),
    ("L40S", (80, 80, 120, 40), SHORT, 158),
    ("L40T", (80, 80, 120, 40), SHORT, 158),
    ("L40U", (80, 80, 120, 40), FULL, 159),
    ("L40X", (80, 80, 240, 40), 0xa489_9289_cf66_3ea3, 283),
    ("L40Y", (80, 60, 220, 70), 0xa335_72d5_54c1_2655, 1824),
    ("L40Z", (220, 80, 100, 40), 0x5723_e5a7_d7c9_cae1, 158),
    ("L40Za", (70, 200, 60, 120), 0xe76d_adc8_d70d_f24b, 159),
    ("L40Zb", (80, 80, 120, 40), 0xfe84_7a12_498e_e15d, 316),
    ("L41A", (80, 80, 80, 60), 0x882d_ca37_d14d_5f37, 61),
    ("L41B", (80, 80, 80, 60), 0x882d_ca37_d14d_5f37, 61),
    ("L41C", (80, 80, 80, 60), 0xaf62_467a_8f8f_b365, 32),
    ("L41D", (80, 80, 80, 60), 0xaf62_467a_8f8f_b365, 32),
];

/// The grab `name` must be the oracle's, word for word.
fn compare(name: &str) {
    let &(_, (x, y, w, h), want, nonwhite) = GRABS.iter().find(|g| g.0 == name).expect("grab in the table");
    let mut v: u64 = 0xcbf2_9ce4_8422_2325;
    let mut n = 0;
    for yy in y..y + h {
        for xx in x..x + w {
            let word = screen_word(xx, yy);
            for b in [(word >> 8) as u8, word as u8] {
                v = (v ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3);
            }
            if word != 0xFFFF {
                n += 1;
            }
        }
    }
    assert_eq!((v, n), (want, nonwhite), "{name}: the grab differs from the oracle (hash, non-white words)");
}

/// A call on our virtual workstation, or on the physical one (`Phys`, the
/// probe's AES handle from `graf_handle`), or a line drawn the way the AES
/// draws a `v_pline` (`Aes`, [`RasterBackend::draw_line`]).
enum Step {
    Ours(Call),
    Phys(Call),
    Aes,
}
use Step::{Aes, Ours, Phys};

const fn c(opcode: i16, sub: i16, intin: &'static [i16], ptsin: &'static [i16]) -> Call {
    Call { opcode, sub, intin, ptsin }
}

/// The probe's `t40_set`: TRUE = `v_pline` (10,300)-(20,300), FALSE = `v_pline` (10,300).
const SET_TRUE: Step = Ours(c(6, 0, &[], &[10, 300, 20, 300]));
const SET_FALSE: Step = Ours(c(6, 0, &[], &[10, 300]));
const UNDER_ON: Step = Ours(c(106, 0, &[8], &[]));
const XOR: Step = Ours(c(32, 0, &[3], &[]));
const ABCDEFGH: &[i16] = &[65, 66, 67, 68, 69, 70, 71, 72];
const GTEXT: Step = Ours(c(8, 0, ABCDEFGH, &[100, 100]));
const WIDTH3: Step = Ours(c(16, 0, &[], &[3, 0]));
const WIDTH1: Step = Ours(c(16, 0, &[], &[1, 0]));
const PERIM_OFF: Step = Ours(c(104, 0, &[0], &[]));
const CLIP_TOP: Step = Ours(c(129, 0, &[1], &[0, 0, 639, 200]));
const TRI: &[i16] = &[300, 300, 340, 300, 320, 340];

/// The probe's `tbegin`, `steps` in order, then [`compare`].
fn run(name: &str, steps: &[Step]) {
    let _guard = VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let wk = workstation();
    let mut pb = Pb::new();
    reset_vdi(&mut pb, wk);
    for s in steps {
        let (p, h) = match s {
            Ours(p) => (p, wk.handle),
            Phys(p) => (p, wk.phys),
            Aes => {
                raster().draw_line(10, 300, 20, 300, 1, WriteMode::Replace, ClipRect::full(640, 480), 0xFFFF);
                continue;
            }
        };
        pb.contrl.fill(0);
        pb.intin[..p.intin.len()].copy_from_slice(p.intin);
        pb.ptsin[..p.ptsin.len()].copy_from_slice(p.ptsin);
        pb.contrl[0] = p.opcode;
        pb.contrl[1] = (p.ptsin.len() / 2) as i16;
        pb.contrl[3] = p.intin.len() as i16;
        pb.contrl[5] = p.sub;
        pb.contrl[6] = h;
        pb.call();
    }
    compare(name);
}

/// The probe's `t40_under(name)` after `pre`.
fn under(name: &str, pre: &[Step]) {
    let mut steps: Vec<Step> = pre.iter().map(clone_step).collect();
    steps.extend([UNDER_ON, XOR, GTEXT]);
    run(name, &steps);
}

fn clone_step(s: &Step) -> Step {
    match s {
        Ours(p) => Ours(c(p.opcode, p.sub, p.intin, p.ptsin)),
        Phys(p) => Phys(c(p.opcode, p.sub, p.intin, p.ptsin)),
        Aes => Aes,
    }
}

/// TVJ/TVK: the underline after a 2-point (TRUE) and a 1-point (FALSE) `v_pline`.
#[test]
fn v40_probe_tvj_tvk_after_two_and_one_point_pline() {
    run("TVJ", &[UNDER_ON, XOR, Ours(c(6, 0, &[], &[10, 10, 20, 10])), GTEXT]);
    run("TVK", &[UNDER_ON, XOR, Ours(c(6, 0, &[], &[10, 10])), GTEXT]);
}

/// Width-1 `v_pline` of 0 points, and of 1 point with arrowed ends: FALSE.
#[test]
fn v40_probe_l40a_l40b_short_pline_clears() {
    under("L40A", &[SET_TRUE, Ours(c(6, 0, &[], &[]))]);
    under("L40B", &[SET_TRUE, Ours(c(108, 0, &[1, 1], &[])), Ours(c(6, 0, &[], &[10, 300])), Ours(c(108, 0, &[0, 0], &[]))]);
}

/// Width 3: one point leaves the flag (`wideline` returns at once), a real
/// segment sets it (the box's perimeter), a zero-length one leaves it.
#[test]
fn v40_probe_l40c_l40e_wideline() {
    under("L40C", &[SET_TRUE, WIDTH3, Ours(c(6, 0, &[], &[10, 300])), WIDTH1]);
    under("L40D", &[SET_FALSE, WIDTH3, Ours(c(6, 0, &[], &[10, 300, 20, 300])), WIDTH1]);
    under("L40E", &[SET_FALSE, WIDTH3, Ours(c(6, 0, &[], &[300, 300, 300, 300])), WIDTH1]);
}

/// A marker is a polyline: TRUE.
#[test]
fn v40_probe_l40f_marker() {
    under("L40F", &[SET_FALSE, Ours(c(7, 0, &[], &[300, 300]))]);
}

/// `v_fillarea`: the perimeter sets the flag; without it nothing does; with
/// 0 points and clipping off the one-point perimeter clears it; outside the
/// clip band `polygon` returns before the perimeter.
#[test]
fn v40_probe_l40g_l40j_fillarea() {
    under("L40G", &[SET_FALSE, Ours(c(9, 0, &[], TRI))]);
    under("L40H", &[SET_FALSE, PERIM_OFF, Ours(c(9, 0, &[], TRI))]);
    under("L40I", &[SET_TRUE, Ours(c(9, 0, &[], &[]))]);
    under("L40J", &[SET_FALSE, CLIP_TOP, Ours(c(9, 0, &[], TRI))]);
}

/// A polyline wholly clipped away still set the flag before its clip test.
#[test]
fn v40_probe_l40k_clipped_pline_sets() {
    under("L40K", &[SET_FALSE, CLIP_TOP, Ours(c(6, 0, &[], &[10, 300, 20, 300]))]);
}

/// The GDP outlines: `v_bar` and circle perimeters (on/off), arc, `v_rbox`,
/// `v_rfbox`; a circle the clip excludes returns before any polyline.
#[test]
fn v40_probe_l40l_l40s_gdp() {
    const BAR: &[i16] = &[300, 300, 340, 340];
    const CIRCLE: &[i16] = &[320, 320, 20, 0, 20, 0];
    const BOX: &[i16] = &[300, 300, 360, 340];
    under("L40L", &[SET_FALSE, Ours(c(11, 1, &[], BAR))]);
    under("L40M", &[SET_FALSE, PERIM_OFF, Ours(c(11, 1, &[], BAR))]);
    under("L40N", &[SET_FALSE, Ours(c(11, 4, &[], CIRCLE))]);
    under("L40O", &[SET_FALSE, PERIM_OFF, Ours(c(11, 4, &[], CIRCLE))]);
    under("L40P", &[SET_FALSE, Ours(c(11, 2, &[0, 900], &[320, 320, 0, 0, 0, 0, 20, 0]))]);
    under("L40Q", &[SET_FALSE, Ours(c(11, 8, &[], BOX))]);
    under("L40R", &[SET_FALSE, Ours(c(11, 9, &[], BOX))]);
    under("L40S", &[SET_FALSE, CLIP_TOP, Ours(c(11, 4, &[], &[320, 400, 20, 0, 20, 0]))]);
}

/// `vr_recfl` writes no flag.
#[test]
fn v40_probe_l40t_recfl() {
    under("L40T", &[SET_FALSE, Ours(c(114, 0, &[], &[300, 300, 340, 340]))]);
}

/// One flag for all workstations: a line on the physical handle sets it for ours.
#[test]
fn v40_probe_l40u_other_workstation() {
    under("L40U", &[SET_FALSE, Phys(c(6, 0, &[], &[10, 300, 20, 300]))]);
}

/// The readers: `v_justified`'s underline, the doubled underline of a larger
/// size, rotation 1800 (horizontal, shortened at its right end) and 900
/// (vertical, never shortened), two underlines in a row.
#[test]
fn v40_probe_l40x_l40zb_readers() {
    let mut j = vec![1i16, 0];
    j.extend("AB CD EF".bytes().map(i16::from));
    let j: &'static [i16] = Box::leak(j.into_boxed_slice());
    run("L40X", &[SET_FALSE, UNDER_ON, XOR, Ours(c(11, 10, j, &[100, 100, 200, 0]))]);
    run("L40Y", &[SET_FALSE, UNDER_ON, XOR, Ours(c(12, 0, &[], &[0, 26])), GTEXT]);
    run("L40Z", &[SET_FALSE, UNDER_ON, XOR, Ours(c(13, 0, &[1800], &[])), Ours(c(8, 0, ABCDEFGH, &[300, 100]))]);
    run("L40Za", &[SET_FALSE, UNDER_ON, XOR, Ours(c(13, 0, &[900], &[])), Ours(c(8, 0, ABCDEFGH, &[100, 300]))]);
    run("L40Zb", &[SET_FALSE, UNDER_ON, XOR, GTEXT, Ours(c(8, 0, ABCDEFGH, &[100, 110]))]);
}

/// XOR polylines and markers set the flag themselves: the same pixels after
/// either state (controls).
#[test]
fn v40_probe_l41a_l41d_xor_lines_and_markers() {
    const Z: &[i16] = &[100, 100, 140, 100, 140, 120];
    run("L41A", &[SET_FALSE, XOR, Ours(c(6, 0, &[], Z))]);
    run("L41B", &[SET_TRUE, XOR, Ours(c(6, 0, &[], Z))]);
    let marks = |set| {
        [
            set,
            XOR,
            Ours(c(18, 0, &[4], &[])),
            Ours(c(7, 0, &[], &[120, 110])),
            Ours(c(18, 0, &[2], &[])),
            Ours(c(7, 0, &[], &[120, 110])),
        ]
    };
    run("L41C", &marks(SET_FALSE));
    run("L41D", &marks(SET_TRUE));
}

/// The AES draws EmuTOS' `v_pline`s through [`RasterBackend::draw_line`]
/// (`gsx_xline`, `gsx_cline`, the 3D outlines; `docs/analysis/deps/mapping.tsv`
/// `v_pline`), and EmuTOS' are polylines of two or three points: TRUE. No
/// probe case of its own (the AES's lines need a GEM call); the expected
/// pixels are TVJ's.
#[test]
fn v40_aes_draw_line_sets_lstlin() {
    run("TVJ", &[SET_FALSE, Aes, UNDER_ON, XOR, GTEXT]);
}
