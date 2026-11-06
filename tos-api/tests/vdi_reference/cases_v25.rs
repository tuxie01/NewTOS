//! V25 (TASK/V25.md, finding VDI-DRAW-010 of
//! `docs/audits/2026-11-05-v18-t1-vdi-draw.md`): a curve the clip rectangle
//! excludes is not drawn at all. EmuTOS' `gdp_curve` returns before it
//! flattens the curve when clipping is on and the clip lies wholly beyond
//! `xc ± xrad` or `yc ± yrad` (`vdi/vdi_gdp.c:395-402`) — the radius only,
//! without the line width and the arrowheads, so a wide stroke or an arrow
//! that would reach into the clip is not drawn either.
//!
//! Two sources, both from the pinned oracle, EmuTOS 1a7484ab on Hatari
//! Falcon030 truecolor:
//! - case 59 of `tosdump/cases/vdi-v18.tsv` (CRC `6E0EBA51`), replayed
//!   against `tosdump/reference/EMUTOS-1a7484ab-VDI-v18-falcontc.BIN` (run
//!   `measurements/measurement-2026-11-05-v18-t1/RUNINFO.TXT`; record index =
//!   the case's position among the table's measured lines);
//! - the probe cases K11A–K11D of `tosdump/probes/vdprobe.c` (`k11`): every
//!   non-white screen word of the grab rectangle, decoded from
//!   `probe/e1/VDPROBE.BIN.gz` of the same V18 run (the oracle series of run
//!   `measurements/measurement-2026-11-06-v24-test/RUNINFO.TXT` shows the
//!   same pixels). K11A, K11B and K11D are white throughout; K11C, whose
//!   clip cuts the curve, is the drawn control.
//!
//! The last two tests are not oracle cases: they pin the two conditions of
//! the exit from the source — clipping must be on, and the comparison is
//! strict (`<`, `>`).
//!
//! Hand-written, like `cases_v20`..`cases_v24`.

use crate::cases_v20::check;
use crate::harness::{reset_vdi, screen_word, workstation, Call, Case, Pb, VDI_LOCK};

static FALCON_TC_V18: &[u8] = include_bytes!("../../../tosdump/reference/EMUTOS-1a7484ab-VDI-v18-falcontc.BIN");

const BLACK: u16 = 0x0000;
const WHITE: u16 = 0xFFFF;

const WIDTH_9: Call = Call { opcode: 16, sub: 0, intin: &[], ptsin: &[9, 0] };
const WIDTH_11: Call = Call { opcode: 16, sub: 0, intin: &[], ptsin: &[11, 0] };
const ARROWED: Call = Call { opcode: 108, sub: 0, intin: &[1, 1], ptsin: &[] };
/// `vs_clip` on `(121,0)-(300,200)`: one column right of `xc + xrad = 120`.
const CLIP_X121: Call = Call { opcode: 129, sub: 0, intin: &[1], ptsin: &[121, 0, 300, 200] };

/// `v_arc` centre (100,100), radius 20 (`PTSIN[6]`), `beg..end` tenths.
const fn arc(beg_end: &'static [i16]) -> Call {
    Call { opcode: 11, sub: 2, intin: beg_end, ptsin: &[100, 100, 0, 0, 0, 0, 20, 0] }
}
const ARC_FULL: Call = arc(&[0, 3600]);

// ── vdi-v18.tsv case 59 ───────────────────────────────────────────────────

/// Width 9 would reach x 124 at y 90; EmuTOS has quit before drawing.
#[test]
fn v25_case_0059_wide_arc_outside_the_clip_is_not_drawn() {
    check(FALCON_TC_V18, &Case {
        prelude: &[WIDTH_9, CLIP_X121, ARC_FULL],
        id: 59,
        rec: 13,
        opcode: 105,
        sub: 0,
        intin: &[],
        ptsin: &[121, 90],
        comment: "K11 (121,90): EmuTOS has quit before drawing",
    });
}

// ── probe K11A–K11D: the whole grab rectangle ─────────────────────────────

/// After the dumper's reset, `calls` in order; then every screen word of
/// the `grab` rectangle `(x, y, w, h)` must be black inside one of `spans`
/// (`(y, x_first, x_last)`) and white everywhere else.
fn probe(name: &str, calls: &[Call], grab: (i16, i16, i16, i16), spans: &[(i16, i16, i16)]) {
    let _guard = VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let wk = workstation();
    let mut pb = Pb::new();
    reset_vdi(&mut pb, wk);
    for p in calls {
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
    let (gx, gy, gw, gh) = grab;
    let mut diffs = Vec::new();
    for y in gy..gy + gh {
        for x in gx..gx + gw {
            let inside = spans.iter().any(|&(sy, x0, x1)| sy == y && (x0..=x1).contains(&x));
            let want = if inside { BLACK } else { WHITE };
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

/// The number of black screen words in `(x, y, w, h)` after `calls`.
fn black_pixels(calls: &[Call], (gx, gy, gw, gh): (i16, i16, i16, i16)) -> usize {
    let _guard = VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let wk = workstation();
    let mut pb = Pb::new();
    reset_vdi(&mut pb, wk);
    for p in calls {
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
    let mut n = 0;
    for y in gy..gy + gh {
        for x in gx..gx + gw {
            if screen_word(x, y) == BLACK {
                n += 1;
            }
        }
    }
    n
}

const GRAB_K11: (i16, i16, i16, i16) = (60, 60, 80, 80);

/// Width 9 reaches into the clip by 4 columns; EmuTOS draws nothing.
#[test]
fn v25_probe_k11a_wide_arc_outside_the_clip_draws_nothing() {
    probe("K11A", &[WIDTH_9, CLIP_X121, ARC_FULL], GRAB_K11, &[]);
}

/// `v_ellarc` 30x20, upper half, width 11, clip from y 121 (`yc + yrad =
/// 120`): nothing, before and after V25.
#[test]
fn v25_probe_k11b_wide_ellarc_outside_the_clip_draws_nothing() {
    const ELLARC: Call = Call { opcode: 11, sub: 6, intin: &[0, 1800], ptsin: &[100, 100, 30, 20] };
    const CLIP_Y121: Call = Call { opcode: 129, sub: 0, intin: &[1], ptsin: &[0, 121, 300, 200] };
    probe("K11B", &[WIDTH_11, CLIP_Y121, ELLARC], (50, 60, 100, 80), &[]);
}

/// The control: clip `(115,0)-(300,200)` cuts the curve, so EmuTOS goes on
/// and draws the part of the wide arc right of x 115.
#[test]
fn v25_probe_k11c_wide_arc_cut_by_the_clip_is_drawn() {
    const CLIP_X115: Call = Call { opcode: 129, sub: 0, intin: &[1], ptsin: &[115, 0, 300, 200] };
    #[rustfmt::skip]
    const SPANS: &[(i16, i16, i16)] = &[
        (79, 115, 115), (80, 115, 115), (81, 115, 116), (82, 115, 117), (83, 115, 118),
        (84, 115, 119), (85, 115, 120), (86, 115, 121), (87, 115, 122), (88, 115, 122),
        (89, 115, 123), (90, 115, 123), (91, 115, 124), (92, 115, 124), (93, 115, 125),
        (94, 115, 125), (95, 115, 125), (96, 115, 125), (97, 115, 125), (98, 115, 125),
        (99, 115, 125), (100, 115, 125), (101, 115, 125), (102, 115, 125), (103, 115, 125),
        (104, 115, 125), (105, 115, 125), (106, 115, 125), (107, 115, 125), (108, 115, 124),
        (109, 115, 124), (110, 115, 123), (111, 115, 123), (112, 115, 122), (113, 115, 122),
        (114, 115, 121), (115, 115, 120), (116, 115, 119), (117, 115, 118), (118, 115, 117),
        (119, 115, 116), (120, 115, 115),
    ];
    probe("K11C", &[WIDTH_9, CLIP_X115, ARC_FULL], GRAB_K11, SPANS);
}

/// Width 1, arrowed ends, 0..900: the arrowhead at (120,100) would reach
/// into the clip; EmuTOS draws nothing.
#[test]
fn v25_probe_k11d_arrowed_arc_outside_the_clip_draws_nothing() {
    probe("K11D", &[ARROWED, CLIP_X121, arc(&[0, 900])], GRAB_K11, &[]);
}

// ── the two conditions of the exit (source, not oracle) ───────────────────

/// Clipping off: no exit, K11A's arc is drawn (`if (vwk->clip)`,
/// `vdi/vdi_gdp.c:398`). The clip rectangle is the one K11A set, then
/// switched off with `vs_clip(0)`.
#[test]
fn v25_clip_off_draws_the_arc() {
    const CLIP_OFF: Call = Call { opcode: 129, sub: 0, intin: &[0], ptsin: &[121, 0, 300, 200] };
    assert!(black_pixels(&[WIDTH_9, CLIP_X121, CLIP_OFF, ARC_FULL], GRAB_K11) > 0);
}

/// `xc + xrad == xmn_clip`: the comparison is strict (`<`, `:399`), so the
/// curve touching the clip edge is drawn — its column 120 and the wide
/// stroke's part right of it.
#[test]
fn v25_clip_edge_at_xc_plus_xrad_is_not_excluded() {
    const CLIP_X120: Call = Call { opcode: 129, sub: 0, intin: &[1], ptsin: &[120, 0, 300, 200] };
    assert!(black_pixels(&[WIDTH_9, CLIP_X120, ARC_FULL], GRAB_K11) > 0);
}
