//! V21 (TASK/V21.md, finding VDI-DRAW-008 of
//! `docs/audits/2026-11-05-v18-t1-vdi-draw.md`): the three `v_contourfill`
//! cases of `tosdump/cases/vdi-v18.tsv` (CRC `6E0EBA51`) replayed against the
//! pinned oracle, EmuTOS 1a7484ab on Hatari Falcon030 truecolor —
//! `tosdump/reference/EMUTOS-1a7484ab-VDI-v18-falcontc.BIN` (run
//! `measurements/measurement-2026-11-05-v18-t1/RUNINFO.TXT`).
//!
//! The prelude draws 40 dotted rows (user line style `0x8888`, rows
//! 200..317 step 3, x 20..619) and flood-fills colour 2 from (320,340) up to
//! boundary pen 1; the measured call reads one pocket between two dots with
//! `v_get_pixel`. EmuTOS fills every pocket (`0xF800`).
//!
//! Hand-written, not generated, like `cases_v20`; the record index is the
//! case's position among the table's measured (non-prelude) lines.

use crate::cases_v20::check;
use crate::harness::{Call, Case};

static FALCON_TC_V18: &[u8] = include_bytes!("../../../tosdump/reference/EMUTOS-1a7484ab-VDI-v18-falcontc.BIN");

/// The K8 prelude of `vdi-v18.tsv`, call for call.
fn lattice_prelude() -> &'static [Call] {
    let mut calls = vec![
        Call { opcode: 15, sub: 0, intin: &[7], ptsin: &[] },
        Call { opcode: 113, sub: 0, intin: &[0x8888u16 as i16], ptsin: &[] },
    ];
    for y in (200..=317).step_by(3) {
        let pts: &'static [i16] = Box::leak(Box::new([20, y, 619, y]));
        calls.push(Call { opcode: 6, sub: 0, intin: &[], ptsin: pts });
    }
    calls.push(Call { opcode: 25, sub: 0, intin: &[2], ptsin: &[] });
    calls.push(Call { opcode: 103, sub: 0, intin: &[1], ptsin: &[320, 340] });
    Box::leak(calls.into_boxed_slice())
}

fn pocket(id: u16, rec: usize, x: i16, y: i16, comment: &'static str) {
    let ptsin: &'static [i16] = Box::leak(Box::new([x, y]));
    check(FALCON_TC_V18, &Case {
        prelude: lattice_prelude(),
        id,
        rec,
        opcode: 105,
        sub: 0,
        intin: &[],
        ptsin,
        comment,
    });
}

#[test]
fn v21_case_0109_contourfill_fills_pocket_22_200() {
    pocket(109, 14, 22, 200, "K8 pocket (22,200) after the fill");
}

#[test]
fn v21_case_0154_contourfill_fills_pocket_322_230() {
    pocket(154, 15, 322, 230, "K8 pocket (322,230) after the fill");
}

#[test]
fn v21_case_0199_contourfill_fills_pocket_618_203() {
    pocket(199, 16, 618, 203, "K8 pocket (618,203) after the fill");
}
