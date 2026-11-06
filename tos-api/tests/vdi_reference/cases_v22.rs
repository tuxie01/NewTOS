//! V22 (TASK/V22.md, findings VDI-DRAW-004 and 006 of
//! `docs/audits/2026-11-05-v18-t1-vdi-draw.md`): `v_fillarea` with one and
//! two vertices, and with more than `MAX_VERTICES`.
//!
//! EmuTOS' `polygon` has no minimum vertex count (`vdi/vdi_fill.c:560-610`):
//! it appends the first vertex after the last (`:594-598`), scan-fills
//! `count` edges with `clc_flit` (`:434-551`) and, with the perimeter on,
//! strokes `count + 1` points (`:606-608`). One vertex: the only edge is
//! horizontal, nothing is filled, the perimeter is the dot. Two vertices:
//! the edge there and the edge back cross every scan line at the same x, so
//! the fill alone draws one pixel per row of `(miny, maxy)` — a dotted
//! stroke, even with the perimeter off.
//!
//! Two sources, both from the pinned oracle, EmuTOS 1a7484ab on Hatari
//! Falcon030 truecolor (run
//! `measurements/measurement-2026-11-05-v18-t1/RUNINFO.TXT`):
//! - cases 51/53 of `tosdump/cases/vdi-v18.tsv` (CRC `6E0EBA51`), replayed
//!   against `tosdump/reference/EMUTOS-1a7484ab-VDI-v18-falcontc.BIN`
//!   (record index = the case's position among the table's measured lines);
//! - the probe cases K4A–K4H of `tosdump/probes/vdprobe.c`: the 80×80
//!   rectangle at (30,30), every non-white screen word written out below as
//!   row spans, decoded from `probe/e1/VDPROBE.BIN.gz` of that run (the
//!   maps in `probe/maps/22-K4A.txt` … `28-K4G.txt` show the same pixels).
//!
//! K5D (1030 vertices) is the documented deviation of decision C (TASK/V18,
//! `docs/COMPATIBILITY.md`): EmuTOS' closing point lands behind its
//! `local_ptsin` in `fill_buffer`, NewTOS closes the first 1024 cleanly.
//!
//! Hand-written, like `cases_v20`/`cases_v21`.

use crate::cases_v20::check;
use crate::harness::{reset_vdi, screen_word, workstation, Call, Case, Pb, ABI, VDI_LOCK};

static FALCON_TC_V18: &[u8] = include_bytes!("../../../tosdump/reference/EMUTOS-1a7484ab-VDI-v18-falcontc.BIN");

// ── vdi-v18.tsv cases 51/53 ───────────────────────────────────────────────

/// One vertex, perimeter on: the perimeter's single point (50,50) is drawn
/// (`polyline(vwk, ptsin, 2, …)`, `vdi/vdi_fill.c:608`).
#[test]
fn v22_case_0051_fillarea_one_point_draws_the_point() {
    check(FALCON_TC_V18, &Case {
        prelude: &[Call { opcode: 9, sub: 0, intin: &[], ptsin: &[50, 50] }],
        id: 51,
        rec: 11,
        opcode: 105,
        sub: 0,
        intin: &[],
        ptsin: &[50, 50],
        comment: "K4 the one point",
    });
}

/// Two vertices, perimeter on: (70,60) lies on the stroke.
#[test]
fn v22_case_0053_fillarea_two_points_draws_the_stroke() {
    check(FALCON_TC_V18, &Case {
        prelude: &[Call { opcode: 9, sub: 0, intin: &[], ptsin: &[50, 50, 90, 70] }],
        id: 53,
        rec: 12,
        opcode: 105,
        sub: 0,
        intin: &[],
        ptsin: &[70, 60],
        comment: "K4 the midpoint of the 2-point area",
    });
}

// ── probe K4A–K4H: the whole 80×80 rectangle ──────────────────────────────

const PERIMETER_OFF: Call = Call { opcode: 104, sub: 0, intin: &[0], ptsin: &[] };
const XOR: Call = Call { opcode: 32, sub: 0, intin: &[3], ptsin: &[] };
const FILL_RED: Call = Call { opcode: 25, sub: 0, intin: &[2], ptsin: &[] };

/// Pen 1 (black) and pen 2 (red) on the Falcon truecolor screen; pen 0
/// (the cleared screen) is white.
const BLACK: u16 = 0x0000;
const RED: u16 = 0xF800;
const WHITE: u16 = 0xFFFF;

/// The probe's grab rectangle for every K4 case (`vdprobe.c` `k4`).
const GRAB: (i16, i16, i16, i16) = (30, 30, 80, 80);

/// After the dumper's reset and `prelude`, `v_fillarea` through `pts`; then
/// every screen word of [`GRAB`] must be `colour` inside one of `spans`
/// (`(y, x_first, x_last)`) and white everywhere else.
fn probe(name: &str, prelude: &[Call], pts: &'static [i16], colour: u16, spans: &[(i16, i16, i16)]) {
    let _guard = VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let wk = workstation();
    let mut pb = Pb::new();
    reset_vdi(&mut pb, wk);
    for p in prelude.iter().chain(core::iter::once(&Call { opcode: 9, sub: 0, intin: &[], ptsin: pts })) {
        pb.contrl.fill(0);
        pb.intin[..p.intin.len()].copy_from_slice(p.intin);
        pb.ptsin[..p.ptsin.len()].copy_from_slice(p.ptsin);
        pb.contrl[0] = p.opcode;
        pb.contrl[1] = (p.ptsin.len() / 2) as i16;
        pb.contrl[3] = p.intin.len() as i16;
        pb.contrl[5] = p.sub;
        pb.contrl[6] = wk.handle;
        pb.call();
    }
    let (gx, gy, gw, gh) = GRAB;
    let mut diffs = Vec::new();
    for y in gy..gy + gh {
        for x in gx..gx + gw {
            let inside = spans.iter().any(|&(sy, x0, x1)| sy == y && (x0..=x1).contains(&x));
            let want = if inside { colour } else { WHITE };
            let got = screen_word(x, y);
            if got != want {
                diffs.push(format!("({x},{y}) got {got:#06x} want {want:#06x}"));
            }
        }
    }
    assert!(
        diffs.is_empty(),
        "{name}: {} pixels differ from the oracle probe, first: {:?}",
        diffs.len(),
        &diffs[..diffs.len().min(8)]
    );
}

/// The fill rows of the stroke (50,50)-(90,70) without the perimeter: one
/// pixel per scan line 51..69, x = 52, 54, … 88 (`clc_flit` skips the top
/// row `y > end`, and row 70 is no crossing, `(dy1^dy2) < 0`).
fn diagonal_fill_rows() -> Vec<(i16, i16, i16)> {
    (51..=69).map(|y| (y, 52 + 2 * (y - 51), 52 + 2 * (y - 51))).collect()
}

#[test]
fn v22_probe_k4a_one_point_perimeter_on() {
    probe("K4A", &[], &[50, 50], BLACK, &[(50, 50, 50)]);
}

#[test]
fn v22_probe_k4b_two_points_perimeter_on() {
    // The perimeter (50,50)-(90,70)-(50,50) over the fill: two pixels per
    // row 51..70, the vertex (50,50) alone on row 50.
    let mut spans = vec![(50, 50, 50)];
    spans.extend((51..=70).map(|y| (y, 51 + 2 * (y - 51), 52 + 2 * (y - 51))));
    probe("K4B", &[], &[50, 50, 90, 70], BLACK, &spans);
}

#[test]
fn v22_probe_k4c_two_points_perimeter_off_fills_a_dotted_stroke() {
    probe("K4C", &[PERIMETER_OFF], &[50, 50, 90, 70], BLACK, &diagonal_fill_rows());
}

#[test]
fn v22_probe_k4d_two_points_vertical_perimeter_off() {
    let spans: Vec<_> = (51..=79).map(|y| (y, 50, 50)).collect();
    probe("K4D", &[PERIMETER_OFF], &[50, 50, 50, 80], BLACK, &spans);
}

#[test]
fn v22_probe_k4e_two_points_xor_perimeter_on() {
    // XOR: the perimeter complements the fill dots back and the pixels the
    // fill left white on; what stays black is one pixel per row 51..70.
    let mut spans = diagonal_fill_rows();
    spans.push((70, 90, 90));
    probe("K4E", &[XOR], &[50, 50, 90, 70], BLACK, &spans);
}

#[test]
fn v22_probe_k4f_one_point_perimeter_off_draws_nothing() {
    probe("K4F", &[PERIMETER_OFF], &[50, 50], BLACK, &[]);
}

#[test]
fn v22_probe_k4g_two_points_horizontal_perimeter_on() {
    // Horizontal edges are never filled (`clc_flit`, `dy == 0`); the
    // perimeter draws the line.
    probe("K4G", &[], &[40, 60, 90, 60], BLACK, &[(60, 40, 90)]);
}

/// The three-vertex control: unchanged by V22, the same before and after.
#[test]
fn v22_probe_k4h_three_points_control() {
    #[rustfmt::skip]
    const SPANS: &[(i16, i16, i16)] = &[
        (50, 50, 50), (51, 50, 52), (52, 51, 54), (53, 51, 56), (54, 51, 58),
        (55, 51, 60), (56, 52, 62), (57, 52, 64), (58, 52, 66), (59, 52, 68),
        (60, 53, 70), (61, 53, 72), (62, 53, 74), (63, 53, 76), (64, 54, 78),
        (65, 54, 80), (66, 54, 82), (67, 54, 84), (68, 55, 86), (69, 55, 88),
        (70, 55, 90), (71, 55, 89), (72, 56, 87), (73, 56, 86), (74, 56, 84),
        (75, 56, 83), (76, 57, 81), (77, 57, 80), (78, 57, 78), (79, 57, 77),
        (80, 58, 75), (81, 58, 74), (82, 58, 72), (83, 58, 71), (84, 59, 69),
        (85, 59, 68), (86, 59, 66), (87, 59, 65), (88, 60, 63), (89, 60, 62),
        (90, 60, 60),
    ];
    probe("K4H", &[FILL_RED], &[50, 50, 90, 70, 60, 90], RED, SPANS);
}

/// `CONTRL[1] = 0` draws nothing: no edge to fill, and the perimeter is a
/// one-point polyline, which `polyline` does not draw
/// (`vdi/vdi_line.c:1106`, `i = count - 1 > 0`).
#[test]
fn v22_fillarea_zero_points_draws_nothing() {
    probe("zero vertices", &[], &[], BLACK, &[]);
}

// ── K5D: more than MAX_VERTICES (decision C, documented deviation) ────────

/// `v_fillarea` with 1030 vertices, the first 1024 a triangle (vertices
/// 0..1021 at (100,100), 1022 (140,100), 1023 (120,130), 1024.. elsewhere —
/// `vdprobe.c` `k5`, K5D). NewTOS takes the first 1024 and closes them back
/// to vertex 0, so the screen equals the three-vertex triangle. EmuTOS
/// writes its closing point to `local_ptsin[2048]`, which is `fill_buffer`
/// (`include/vdiext.h:47-51`, `vdi/vdi_fill.c:596`), and draws whatever the
/// scan conversion leaves there — no fill and an outline to a wrong point;
/// `docs/COMPATIBILITY.md` records the difference.
#[test]
fn v22_fillarea_over_max_vertices_closes_the_first_1024_cleanly() {
    const N: usize = 1030;
    let _guard = VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let wk = workstation();
    let mut pb = Pb::new();

    // The triangle alone, for comparison.
    reset_vdi(&mut pb, wk);
    pb.contrl.fill(0);
    pb.ptsin[..6].copy_from_slice(&[100, 100, 140, 100, 120, 130]);
    pb.contrl[0] = 9;
    pb.contrl[1] = 3;
    pb.contrl[6] = wk.handle;
    pb.call();
    let rect = |x: i16, y: i16| screen_word(x, y);
    let mut want = Vec::new();
    for y in 90..260 {
        for x in 90..270 {
            want.push(rect(x, y));
        }
    }
    assert!(want.iter().any(|&w| w != WHITE), "the comparison triangle draws");

    // The 1030-vertex call, through a parameter block of its own (the
    // harness's PTSIN holds 128 vertices).
    reset_vdi(&mut pb, wk);
    let mut contrl = [0i16; 12];
    let mut intin = [0i16; 16];
    let mut ptsin = vec![0i16; 2 * N];
    let mut intout = [0i16; 16];
    let mut ptsout = [0i16; 16];
    for i in 0..1022 {
        ptsin[2 * i] = 100;
        ptsin[2 * i + 1] = 100;
    }
    ptsin[2044..2048].copy_from_slice(&[140, 100, 120, 130]);
    for k in 0..6 {
        ptsin[2 * (1024 + k)] = 200 + 10 * k as i16;
        ptsin[2 * (1024 + k) + 1] = 200 + 7 * k as i16;
    }
    contrl[0] = 9;
    contrl[1] = N as i16;
    contrl[6] = wk.handle;
    let block: [u64; 5] = [
        contrl.as_mut_ptr() as u64,
        intin.as_mut_ptr() as u64,
        ptsin.as_mut_ptr() as u64,
        intout.as_mut_ptr() as u64,
        ptsout.as_mut_ptr() as u64,
    ];
    // SAFETY: the five pointers are live arrays of this frame; PTSIN holds
    // all 1030 vertices CONTRL[1] announces.
    unsafe { tos_api::vdi::dispatch(block.as_ptr() as u64, ABI) };

    let mut got = Vec::new();
    for y in 90..260 {
        for x in 90..270 {
            got.push(rect(x, y));
        }
    }
    let differ = want.iter().zip(&got).filter(|(a, b)| a != b).count();
    assert_eq!(differ, 0, "K5D: the first 1024 vertices close cleanly to the triangle");
}
