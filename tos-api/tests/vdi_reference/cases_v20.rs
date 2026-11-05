//! V20 (TASK/V20.md, findings VDI-DRAW-001/002/003 of
//! `docs/audits/2026-11-05-v18-t1-vdi-draw.md`): the seven line cases of
//! `tosdump/cases/vdi-v18.tsv` (CRC `6E0EBA51`) replayed against the pinned
//! oracle, EmuTOS 1a7484ab on Hatari Falcon030 truecolor —
//! `tosdump/reference/EMUTOS-1a7484ab-VDI-v18-falcontc.BIN` (run
//! `measurements/measurement-2026-11-05-v18-t1/RUNINFO.TXT`). Each case draws
//! with a prelude and reads one pixel back with `v_get_pixel`; `INTOUT[1]`
//! is the 16-bit screen word.
//!
//! Hand-written, not generated: the rest of the table belongs to other
//! tickets (V21–V25) and is not yet expected to match. The record index is
//! the case's position among the table's measured (non-prelude) lines.
//!
//! What it pins of the kernel: the harness double draws its lines through
//! the same walk as the kernel's backend (`tos_api::vdi::line::abline`), so
//! these are the kernel's pixels, minus the store.

use crate::harness::{check as check_fields, reset_vdi, workstation, Call, Case, Pb, HDR_SIZE, REC_SIZE, VDI_LOCK};

static FALCON_TC_V18: &[u8] = include_bytes!("../../../tosdump/reference/EMUTOS-1a7484ab-VDI-v18-falcontc.BIN");

/// `INTOUT` in a record (`tosdump/core/tosdump.h`, record format 3).
const OFF_INTOUT: usize = 800;

fn be16(b: &[u8], off: usize) -> i16 {
    (((b[off] as u16) << 8) | b[off + 1] as u16) as i16
}

/// Replay `case` and compare its `v_get_pixel` answer with the oracle.
///
/// `check` compares every other field, but it excludes `v_get_pixel`'s
/// `INTOUT[0..1]` from the byte comparison (`harness::MACHINE_EXCLUDED`,
/// keyed on the opcode because the four-plane TT dumps answer pens) and
/// checks the harness's own screen word in its place — which says nothing
/// about whether the pixel is the oracle's. Against this Falcon truecolor
/// reference the two words are directly comparable, so they are compared
/// here, after the same reset and preludes `check` issues.
fn check(dump: &[u8], case: &Case) {
    check_fields(dump, case);

    let _guard = VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let wk = workstation();
    let mut pb = Pb::new();
    reset_vdi(&mut pb, wk);
    for p in case.prelude {
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
    pb.poison();
    pb.ptsin[..case.ptsin.len()].copy_from_slice(case.ptsin);
    pb.contrl[0] = case.opcode;
    pb.contrl[1] = (case.ptsin.len() / 2) as i16;
    pb.contrl[3] = 0;
    pb.contrl[5] = case.sub;
    pb.contrl[6] = wk.handle;
    pb.call();

    let rec = &dump[HDR_SIZE + case.rec * REC_SIZE..][..REC_SIZE];
    assert_eq!(be16(rec, 0) as u16, case.id, "record {} is case {}", case.rec, case.id);
    for i in 0..2 {
        assert_eq!(
            pb.intout[i] as u16,
            be16(rec, OFF_INTOUT + i * 2) as u16,
            "case {} ({}): v_get_pixel INTOUT[{i}] against the Falcon truecolor oracle",
            case.id,
            case.comment
        );
    }
}

const XOR: Call = Call { opcode: 32, sub: 0, intin: &[3], ptsin: &[] };
const CLIP_30_90: Call = Call { opcode: 129, sub: 0, intin: &[1], ptsin: &[30, 30, 90, 90] };

// ── VDI-DRAW-001: XOR polyline, vertical segment not last ─────────────────

/// EmuTOS draws a vertical segment before the XOR shortening
/// (`vdi/vdi_line.c:2275-2283` returns before `:2319`), so the shared
/// vertex (20,40) is complemented by both segments and ends white.
#[test]
fn v20_case_0029_xor_polyline_vertical_segment_is_not_shortened() {
    check(FALCON_TC_V18, &Case {
        prelude: &[XOR, Call { opcode: 6, sub: 0, intin: &[], ptsin: &[20, 20, 20, 40, 40, 40] }],
        id: 29,
        rec: 4,
        opcode: 105,
        sub: 0,
        intin: &[],
        ptsin: &[20, 40],
        comment: "K1 the shared vertex (20,40): EmuTOS XORs it twice",
    });
}

/// The upward vertical segment must not reach one pixel past its end.
#[test]
fn v20_case_0032_xor_upward_vertical_segment_stays_inside() {
    check(FALCON_TC_V18, &Case {
        prelude: &[XOR, Call { opcode: 6, sub: 0, intin: &[], ptsin: &[20, 40, 20, 20, 40, 20] }],
        id: 32,
        rec: 5,
        opcode: 105,
        sub: 0,
        intin: &[],
        ptsin: &[20, 19],
        comment: "K1 (20,19), one pixel past the upward vertical segment",
    });
}

#[test]
fn v20_case_0035_xor_vertical_segment_control() {
    check(FALCON_TC_V18, &Case {
        prelude: &[XOR, Call { opcode: 6, sub: 0, intin: &[], ptsin: &[20, 20, 20, 40, 40, 40] }],
        id: 35,
        rec: 6,
        opcode: 105,
        sub: 0,
        intin: &[],
        ptsin: &[20, 39],
        comment: "K1 control (20,39) on the vertical segment",
    });
}

// ── VDI-DRAW-002: a clipped segment is redrawn from its clipped endpoints ─

/// `clip_line` moves the endpoints onto the clip edges with
/// `mul_div_round` and the Bresenham starts again from there
/// (`vdi/vdi_line.c:1057-1099`, `:1115-1119`): (30,31)-(90,32).
#[test]
fn v20_case_0040_clipped_segment_pixel_32_31() {
    check(FALCON_TC_V18, &Case {
        prelude: &[CLIP_30_90, Call { opcode: 6, sub: 0, intin: &[], ptsin: &[0, 31, 127, 33] }],
        id: 40,
        rec: 7,
        opcode: 105,
        sub: 0,
        intin: &[],
        ptsin: &[32, 31],
        comment: "K2 clipped segment: (32,31)",
    });
}

#[test]
fn v20_case_0043_clipped_segment_pixel_32_32() {
    check(FALCON_TC_V18, &Case {
        prelude: &[CLIP_30_90, Call { opcode: 6, sub: 0, intin: &[], ptsin: &[0, 31, 127, 33] }],
        id: 43,
        rec: 8,
        opcode: 105,
        sub: 0,
        intin: &[],
        ptsin: &[32, 32],
        comment: "K2 clipped segment: (32,32)",
    });
}

// ── VDI-DRAW-003: clipping off runs on into the next row ──────────────────

/// With `vs_clip(0)` the segment is not clipped (`vdi/vdi_line.c:1115`);
/// x 640..700 of row 120 are x 0..60 of row 121 in the 640-pixel store.
#[test]
fn v20_case_0045_clip_off_runs_into_the_next_row() {
    check(FALCON_TC_V18, &Case {
        prelude: &[Call { opcode: 6, sub: 0, intin: &[], ptsin: &[600, 120, 700, 120] }],
        id: 45,
        rec: 9,
        opcode: 105,
        sub: 0,
        intin: &[],
        ptsin: &[5, 121],
        comment: "K2 clip off: (5,121), the wrapped tail",
    });
}

#[test]
fn v20_case_0047_clip_off_control_on_the_screen_part() {
    check(FALCON_TC_V18, &Case {
        prelude: &[Call { opcode: 6, sub: 0, intin: &[], ptsin: &[600, 120, 700, 120] }],
        id: 47,
        rec: 10,
        opcode: 105,
        sub: 0,
        intin: &[],
        ptsin: &[639, 120],
        comment: "K2 control: (639,120) on the screen part",
    });
}
