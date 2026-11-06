//! V24 (TASK/V24.md, finding VDI-DRAW-009 of
//! `docs/audits/2026-11-05-v18-t1-vdi-draw.md`): `v_get_pixel` off the
//! screen reads the pixel its linear screen address lands on, as EmuTOS'
//! `pixelread` does (`vdi/vdi_fill.c:671-681`, no bounds check, address
//! `get_start_addr16`, `vdi/vdi_inline.h:14-17`).
//!
//! The four K10 cases of `tosdump/cases/vdi-v18.tsv` (CRC `6E0EBA51`)
//! replayed against the pinned oracle, EmuTOS 1a7484ab on Hatari Falcon030
//! truecolor — `tosdump/reference/EMUTOS-1a7484ab-VDI-v18-falcontc.BIN`
//! (run `measurements/measurement-2026-11-05-v18-t1/RUNINFO.TXT`). Each
//! prelude fills one on-screen pixel with `vr_recfl`; the measured call
//! reads it back through an off-screen coordinate.
//!
//! Past the last or before the first pixel of the logical screen EmuTOS
//! reads foreign memory; NewTOS answers 0 there (user decision B, TASK/V18;
//! `docs/COMPATIBILITY.md`). That half has no oracle value and is checked
//! against the cleared screen, whose every word is `0xFFFF`.
//!
//! Hand-written, like `cases_v20`..`cases_v23`; the record index is the
//! case's position among the table's measured (non-prelude) lines.

use crate::cases_v20::check;
use crate::harness::{reset_vdi, screen_word, workstation, Call, Case, Pb, Workstation, SCREEN_H, SCREEN_W, VDI_LOCK};

static FALCON_TC_V18: &[u8] = include_bytes!("../../../tosdump/reference/EMUTOS-1a7484ab-VDI-v18-falcontc.BIN");

/// `vsf_color(pen)` then `vr_recfl` of the one pixel `(x, y)`.
fn dot(pen: i16, x: i16, y: i16) -> &'static [Call] {
    let intin: &'static [i16] = Box::leak(Box::new([pen]));
    let ptsin: &'static [i16] = Box::leak(Box::new([x, y, x, y]));
    Box::leak(Box::new([
        Call { opcode: 25, sub: 0, intin, ptsin: &[] },
        Call { opcode: 114, sub: 0, intin: &[], ptsin },
    ]))
}

fn read_back(id: u16, rec: usize, prelude: &'static [Call], x: i16, y: i16, comment: &'static str) {
    let ptsin: &'static [i16] = Box::leak(Box::new([x, y]));
    check(FALCON_TC_V18, &Case { prelude, id, rec, opcode: 105, sub: 0, intin: &[], ptsin, comment });
}

/// One column past the right edge is the first pixel of the next row.
#[test]
fn v24_case_0015_x_640_reads_the_next_row() {
    read_back(15, 0, dot(2, 0, 11), 640, 10, "K10 v_get_pixel(640,10): EmuTOS reads (0,11)");
}

/// One column left of 0 is the last pixel of the previous row.
#[test]
fn v24_case_0018_x_minus_1_reads_the_previous_row() {
    read_back(18, 1, dot(3, 639, 10), -1, 11, "K10 v_get_pixel(-1,11): EmuTOS reads (639,10)");
}

/// Sixty columns past the edge are column 60 of the next row.
#[test]
fn v24_case_0021_x_700_reads_column_60_of_the_next_row() {
    read_back(21, 2, dot(4, 60, 11), 700, 10, "K10 v_get_pixel(700,10): EmuTOS reads (60,11)");
}

#[test]
fn v24_case_0024_on_screen_control() {
    read_back(24, 3, dot(4, 60, 11), 60, 11, "K10 control: v_get_pixel(60,11) on the screen");
}

/// `v_get_pixel(x, y)` on the cleared screen: `(INTOUT[0], INTOUT[1])`.
fn pixel_on_cleared_screen(x: i16, y: i16) -> (u16, u16) {
    let _guard = VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let wk = workstation();
    let mut pb = Pb::new();
    reset_vdi(&mut pb, wk);
    pb.poison();
    pb.ptsin[0] = x;
    pb.ptsin[1] = y;
    pb.contrl[0] = 105;
    pb.contrl[1] = 1;
    pb.contrl[3] = 0;
    pb.contrl[5] = 0;
    pb.contrl[6] = wk.handle;
    pb.call();
    (pb.intout[0] as u16, pb.intout[1] as u16)
}

/// The edges of the logical screen's store: its first and last pixel, the
/// wrapped neighbours that still land inside it, and the indices just
/// outside it, which read 0 (decision B) where EmuTOS reads foreign memory.
#[test]
fn v24_reads_outside_the_logical_screen_return_zero() {
    let (w, h) = (SCREEN_W as i16, SCREEN_H as i16);
    // Inside the store: v_clrwk left 0xFFFF everywhere (`vdi_control.c:610-618`).
    assert_eq!(pixel_on_cleared_screen(0, 0), (0, 0xFFFF), "first pixel");
    assert_eq!(pixel_on_cleared_screen(w - 1, h - 1), (0, 0xFFFF), "last pixel");
    assert_eq!(pixel_on_cleared_screen(-1, 1), (0, 0xFFFF), "(-1,1) is (w-1,0)");
    assert_eq!(pixel_on_cleared_screen(w, h - 2), (0, 0xFFFF), "(w,h-2) is (0,h-1)");
    assert_eq!(pixel_on_cleared_screen(-w, h), (0, 0xFFFF), "(-w,h) is (0,h-1)");
    // Outside the store.
    assert_eq!(pixel_on_cleared_screen(w, h - 1), (0, 0), "one past the last pixel");
    assert_eq!(pixel_on_cleared_screen(-1, 0), (0, 0), "one before the first pixel");
    assert_eq!(pixel_on_cleared_screen(0, h), (0, 0), "the row below the screen");
    assert_eq!(pixel_on_cleared_screen(0, -1), (0, 0), "the row above the screen");
    assert_eq!(pixel_on_cleared_screen(i16::MAX, i16::MAX), (0, 0), "far past the end");
    assert_eq!(pixel_on_cleared_screen(i16::MIN, i16::MIN), (0, 0), "far before the start");
}

/// One call on the harness workstation with `CONTRL[1]`/`[3]` from the
/// slices.
fn call(pb: &mut Pb, wk: &Workstation, opcode: i16, intin: &[i16], ptsin: &[i16]) {
    pb.contrl.fill(0);
    pb.intin[..intin.len()].copy_from_slice(intin);
    pb.ptsin[..ptsin.len()].copy_from_slice(ptsin);
    pb.contrl[0] = opcode;
    pb.contrl[1] = (ptsin.len() / 2) as i16;
    pb.contrl[3] = intin.len() as i16;
    pb.contrl[6] = wk.handle;
    pb.call();
}

/// `v_contourfill` reads through the same `pixelread`, but EmuTOS' span
/// search stops at the clip edge before it reads past it
/// (`search_to_right16`/`search_to_left16` loop while `x <= xmx_clip` /
/// `x >= xmn_clip`, `vdi/vdi_fill.c:697-740`; `vs_clip(0)` sets the clip to
/// the screen, `vdi/vdi_control.c:241-249`). So a boundary pixel at
/// `(0,11)` — the run-on neighbour of `(640,10)` — does not end row 10's
/// span at the right edge: row 10 fills to `x = 639`, row 11 from `x = 1`.
#[test]
fn v24_contourfill_at_the_screen_edge_does_not_see_the_wrapped_neighbour() {
    const BLACK: u16 = 0x0000;
    const RED: u16 = 0xF800;
    let _guard = VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let wk = workstation();
    let mut pb = Pb::new();
    reset_vdi(&mut pb, wk);
    call(&mut pb, wk, 25, &[1], &[]); // vsf_color 1
    call(&mut pb, wk, 114, &[], &[0, 11, 0, 11]); // vr_recfl (0,11)
    call(&mut pb, wk, 25, &[2], &[]); // vsf_color 2
    call(&mut pb, wk, 103, &[1], &[600, 10]); // v_contourfill to pen 1
    let w = SCREEN_W as i16;
    assert_eq!(screen_word(w - 1, 10), RED, "row 10 is filled up to the right edge");
    assert_eq!(screen_word(0, 10), RED, "row 10 is filled from the left edge");
    assert_eq!(screen_word(0, 11), BLACK, "the boundary pixel stays");
    assert_eq!(screen_word(1, 11), RED, "row 11 is filled right of the boundary");
    assert_eq!(screen_word(w - 1, 11), RED, "row 11 is filled up to the right edge");
}
