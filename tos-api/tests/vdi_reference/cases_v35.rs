//! V35 (TASK/V35.md, finding VDI-TEXT-013 of
//! `docs/audits/2026-11-06-v18-t2-vdi-text.md`): the handlers read
//! `GSX_ENTRY`'s local `PTSIN` copy, not the caller's array.
//!
//! EmuTOS' `GSX_ENTRY` copies the caller's `CONTRL[1]` vertex pairs into
//! `lcl_ptsin` = `vdishare.main.local_ptsin` (`vdi/vdi_asm.S:21,79,93,99-108`,
//! `include/vdiext.h:50`) — for every opcode, before `screen()` looks at the
//! handle — and hands the handlers that buffer as `PTSIN`. It is one buffer:
//! a handler that reads more words than its caller announced reads what an
//! earlier VDI call left there, on any workstation.
//!
//! Sources, all from the pinned oracle, EmuTOS 1a7484ab on Hatari Falcon030
//! truecolor:
//! - cases 50/52/54/55 of `tosdump/cases/vdi-v18-t2.tsv` (CRC `BEFB79A7`),
//!   replayed against `tosdump/reference/EMUTOS-1a7484ab-VDI-v18t2-falcontc.BIN`
//!   (run `measurements/measurement-2026-11-06-v18-t2/RUNINFO.TXT`; record
//!   index = the case's position among the table's measured lines): a setter
//!   with `CONTRL[1] = 0` after a prelude that set the copy. 52 is equal by
//!   chance before V35 (both answers are the doubled 8×16 font), 55 is the
//!   control with the right count;
//! - the probe cases TPA–TPE of `tosdump/probes/vdprobe.c` (`tp`), every
//!   non-white screen word of each grab rectangle, decoded from that run's
//!   `probe/e1/VDPROBE.BIN.gz`, and the `VDPROBE.LOG` values of TPC/TPD;
//! - the probe cases TPF–TPH (`t35`, added by V35), decoded from
//!   `measurements/measurement-2026-11-07-v35/probe/e1/VDPROBE.BIN.gz`, and
//!   their `VDPROBE.LOG` widths: the copy is made on an unknown handle (TPF),
//!   is shared by two workstations (TPG), and holds what a handler wrote into
//!   it (TPH, `vs_clip`'s `arb_corner`, `vdi/vdi_control.c:239-240`) — the
//!   last one NewTOS does not reproduce (open point of TASK/V35, ignored
//!   below with that reason).
//!
//! Not on the oracle: a negative `CONTRL[1]`, whose copy loop runs 65536 − |n|
//! pairs (`vdi_asm.S:95-107`) past the buffer; NewTOS fills the buffer and
//! stops (`docs/COMPATIBILITY.md`, V23/V35), checked here against that rule.
//!
//! Hand-written, like `cases_v20`..`cases_v33`.

use crate::cases_v30::{compare, Span, B, P};
use crate::harness::{check, reset_vdi, workstation, Call, Case, Pb, VDI_LOCK};

static FALCON_TC_V18T2: &[u8] = include_bytes!("../../../tosdump/reference/EMUTOS-1a7484ab-VDI-v18t2-falcontc.BIN");

/// Pen 2 on the Falcon truecolor screen (the probe's `vsf_color(2)`).
const RED: u16 = 0xF800;

// ── vdi-v18-t2.tsv cases 50/52/54/55 ──────────────────────────────────────

const fn setter(opcode: i16, ptsin: &'static [i16]) -> Call {
    Call { opcode, sub: 0, intin: &[], ptsin }
}

const WIDTH_9: &[Call] = &[setter(16, &[9, 0])];
const HEIGHT_26: &[Call] = &[setter(12, &[0, 26])];
const MARKER_30: &[Call] = &[setter(19, &[0, 30])];

fn case(prelude: &'static [Call], id: u16, rec: usize, opcode: i16, ptsin: &'static [i16], comment: &'static str) -> Case {
    Case { prelude, id, rec, opcode, sub: 0, intin: &[], ptsin, comment }
}

/// `vsl_width` 9, then `vsl_width` with `CONTRL[1] = 0` and a poisoned
/// `PTSIN`: EmuTOS reads the 9 its copy kept (`PTSOUT[0]` 9).
#[test]
fn v35_case_0050_vsl_width_reads_the_copy() {
    check(
        FALCON_TC_V18T2,
        &case(WIDTH_9, 50, 13, 16, &[], "rider vsl_width with CONTRL[1]=0 (EmuTOS: the stale 9)"),
    );
}

/// `vst_height` 26, then `vst_height` with `CONTRL[1] = 0`.
#[test]
fn v35_case_0052_vst_height_reads_the_copy() {
    check(
        FALCON_TC_V18T2,
        &case(HEIGHT_26, 52, 14, 12, &[], "rider vst_height with CONTRL[1]=0 (EmuTOS: the stale 26)"),
    );
}

/// `vsm_height` 30, then `vsm_height` with `CONTRL[1] = 0`.
#[test]
fn v35_case_0054_vsm_height_reads_the_copy() {
    check(
        FALCON_TC_V18T2,
        &case(MARKER_30, 54, 15, 19, &[], "rider vsm_height with CONTRL[1]=0 (EmuTOS: the stale 30)"),
    );
}

/// The control: `vsl_width` 9 with the right count.
#[test]
fn v35_case_0055_vsl_width_control() {
    check(FALCON_TC_V18T2, &case(&[], 55, 16, 16, &[9, 0], "rider control: vsl_width 9 with CONTRL[1]=1"));
}

// ── the probe ─────────────────────────────────────────────────────────────

/// One probe call: `CONTRL[1] = n_pts`, whatever `ptsin` holds, on `handle`
/// (`None` = the harness workstation).
struct Raw {
    opcode: i16,
    sub: i16,
    intin: &'static [i16],
    ptsin: &'static [i16],
    n_pts: i16,
    handle: Option<i16>,
}

const fn raw(opcode: i16, intin: &'static [i16], ptsin: &'static [i16], n_pts: i16) -> Raw {
    Raw { opcode, sub: 0, intin, ptsin, n_pts, handle: None }
}

const fn exact(opcode: i16, intin: &'static [i16], ptsin: &'static [i16]) -> Raw {
    raw(opcode, intin, ptsin, (ptsin.len() / 2) as i16)
}

/// Issue `calls` in order on one parameter block (the probe reuses its
/// arrays: a word a call does not set keeps the previous call's value, as
/// in `vdprobe.c`'s `ptsin[]`).
fn run(pb: &mut Pb, handle: i16, calls: &[Raw]) {
    for c in calls {
        pb.contrl.fill(0);
        pb.intin[..c.intin.len()].copy_from_slice(c.intin);
        pb.ptsin[..c.ptsin.len()].copy_from_slice(c.ptsin);
        pb.contrl[0] = c.opcode;
        pb.contrl[1] = c.n_pts;
        pb.contrl[3] = c.intin.len() as i16;
        pb.contrl[5] = c.sub;
        pb.contrl[6] = c.handle.unwrap_or(handle);
        pb.call();
    }
}

/// The probe's `tbegin` (the dumper's reset, as in `cases_v30::probe`), then
/// `calls`; returns `PTSOUT[0..4]` of the last call and leaves the screen for
/// [`compare`].
fn probe(calls: &[Raw]) -> [i16; 4] {
    let wk = workstation();
    let mut pb = Pb::new();
    reset_vdi(&mut pb, wk);
    run(&mut pb, wk.handle, calls);
    [pb.ptsout[0], pb.ptsout[1], pb.ptsout[2], pb.ptsout[3]]
}

const V_PLINE_100: Raw = exact(6, &[], &[100, 100, 200, 150]);
const FILL_RED: Raw = exact(25, &[2], &[]);
const RECFL_FRAME: Raw = exact(114, &[], &[0, 0, 299, 249]);

/// TPA: `v_pline` (100,100)-(200,150), then `vs_clip(1)` with `CONTRL[1] = 0`
/// and the caller's `PTSIN` 10,10,50,50: EmuTOS clips to the copy's
/// rectangle, the `v_pline`'s, so the frame fill covers (100,100)-(200,150).
#[test]
fn v35_probe_tpa_vs_clip_without_points_clips_to_the_copy() {
    let _guard = VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    probe(&[V_PLINE_100, raw(129, &[1], &[10, 10, 50, 50], 0), FILL_RED, RECFL_FRAME]);
    compare("TPA", &[((0, 0, 300, 250), SP_TPA)]);
}

/// TPB: as TPA with `CONTRL[1] = 1` — the first corner is the caller's
/// (10,10), the second the copy's (200,150).
#[test]
fn v35_probe_tpb_vs_clip_one_point_keeps_the_second_corner() {
    let _guard = VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    probe(&[V_PLINE_100, raw(129, &[1], &[10, 10, 50, 50], 1), FILL_RED, RECFL_FRAME]);
    compare("TPB", &[((0, 0, 300, 250), SP_TPB)]);
}

/// TPC: a `v_pline` from (9,470), then `vsl_width` with `CONTRL[1] = 0` and
/// the caller's `PTSIN[0]` 3: EmuTOS sets width 9 (`VDPROBE.LOG` "po 9 0")
/// and draws the line (20,300)-(200,300) 9 rows wide.
#[test]
fn v35_probe_tpc_vsl_width_reads_the_copy() {
    let _guard = VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let wk = workstation();
    let mut pb = Pb::new();
    reset_vdi(&mut pb, wk);
    run(&mut pb, wk.handle, &[exact(6, &[], &[9, 470, 9, 470]), raw(16, &[], &[3, 0], 0)]);
    assert_eq!([pb.ptsout[0], pb.ptsout[1]], [9, 0], "TPC: vsl_width PTSOUT (oracle log \"po 9 0\")");
    run(&mut pb, wk.handle, &[exact(6, &[], &[20, 300, 200, 300])]);
    compare("TPC", &[((0, 280, 220, 40), SP_TPC)]);
}

/// TPD: `v_pline` (0,26)-(0,26), then `vst_height` with `CONTRL[1] = 0` and
/// the caller's `PTSIN[1]` 13: EmuTOS sets height 26 (`VDPROBE.LOG`
/// "po 14 27 16 32") and 'ABC' comes out in the doubled font.
#[test]
fn v35_probe_tpd_vst_height_reads_the_copy() {
    let _guard = VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let out = probe(&[exact(6, &[], &[0, 26, 0, 26]), raw(12, &[], &[0, 13], 0)]);
    assert_eq!(out, [14, 27, 16, 32], "TPD: vst_height PTSOUT (oracle log \"po 14 27 16 32\")");
    let wk = workstation();
    let mut pb = Pb::new();
    run(&mut pb, wk.handle, &[exact(8, &[65, 66, 67], &[20, 380])]);
    compare("TPD", &[((0, 340, 100, 50), SP_TPD)]);
}

/// TPE, the control: `vs_clip(1)` with `CONTRL[1] = 2` (10,10)-(50,50).
#[test]
fn v35_probe_tpe_vs_clip_with_points_control() {
    let _guard = VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    probe(&[V_PLINE_100, exact(129, &[1], &[10, 10, 50, 50]), FILL_RED, RECFL_FRAME]);
    compare("TPE", &[((0, 0, 300, 250), SP_TPE)]);
}

/// `t35_width`: `vsl_width` with `CONTRL[1] = 0` and the caller's
/// `PTSIN[0]` 1, its `PTSOUT[0..2]`; clip off; the line (20,300)-(200,300).
fn width_and_line(pb: &mut Pb, handle: i16) -> [i16; 2] {
    run(pb, handle, &[raw(16, &[], &[1, 0], 0)]);
    let w = [pb.ptsout[0], pb.ptsout[1]];
    run(pb, handle, &[exact(129, &[0], &[0, 0, 0, 0]), exact(6, &[], &[20, 300, 200, 300])]);
    w
}

/// TPF: a `v_pline` with `CONTRL[1] = 1` (7,0) on the unknown handle 77
/// still fills the copy — `GSX_ENTRY` copies before `screen()` rejects the
/// handle (`vdi/vdi_main.c:139-148`): EmuTOS' next `vsl_width` with
/// `CONTRL[1] = 0` sets 7 (`VDPROBE.LOG` "po 7 0").
#[test]
fn v35_probe_tpf_unknown_handle_still_fills_the_copy() {
    let _guard = VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let wk = workstation();
    let mut pb = Pb::new();
    reset_vdi(&mut pb, wk);
    run(&mut pb, wk.handle, &[Raw { handle: Some(77), ..exact(6, &[], &[7, 0]) }]);
    assert_eq!(width_and_line(&mut pb, wk.handle), [7, 0], "TPF: vsl_width PTSOUT (oracle log \"po 7 0\")");
    compare("TPF", &[((0, 280, 220, 40), SP_TPF)]);
}

/// TPG: `vsl_width` (5,0) on a second virtual workstation B, B closed, then
/// `vsl_width` with `CONTRL[1] = 0` on ours: the copy is one buffer for all
/// workstations, EmuTOS sets 5 (`VDPROBE.LOG` "po 5 0").
#[test]
fn v35_probe_tpg_the_copy_is_shared_by_workstations() {
    let _guard = VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let wk = workstation();
    let mut pb = Pb::new();
    reset_vdi(&mut pb, wk);
    run(&mut pb, wk.handle, &[Raw { handle: Some(wk.phys), ..exact(100, &[1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2], &[]) }]);
    let b = pb.contrl[6];
    assert!(b != 0 && b != wk.handle, "TPG: a second workstation opens (handle {b})");
    run(&mut pb, b, &[exact(16, &[], &[5, 0]), exact(101, &[], &[])]);
    assert_eq!(width_and_line(&mut pb, wk.handle), [5, 0], "TPG: vsl_width PTSOUT (oracle log \"po 5 0\")");
    compare("TPG", &[((0, 280, 220, 40), SP_TPG)]);
}

/// TPH: `vs_clip(1)` (9,9)-(3,3); EmuTOS' handler sorts the corners in the
/// copy (`arb_corner`, `vdi/vdi_control.c:239-240`, `vdi/vdi_misc.c:27-41`), so
/// the next `vsl_width` with `CONTRL[1] = 0` reads 3 (`VDPROBE.LOG` "po 3 0").
/// NewTOS' handlers do not write the copy; it reads 9.
#[test]
#[ignore = "TASK/V35 open point: handlers that write PTSIN in EmuTOS (arb_corner, arc/rbox points, arrows) \
            do not write NewTOS' copy; not in the decided scope (copy only), lead/user decision pending"]
fn v35_probe_tph_handler_writes_land_in_the_copy() {
    let _guard = VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let wk = workstation();
    let mut pb = Pb::new();
    reset_vdi(&mut pb, wk);
    run(&mut pb, wk.handle, &[exact(129, &[1], &[9, 9, 3, 3])]);
    assert_eq!(width_and_line(&mut pb, wk.handle), [3, 0], "TPH: vsl_width PTSOUT (oracle log \"po 3 0\")");
    compare("TPH", &[((0, 280, 220, 40), SP_TPH)]);
}

/// A negative `CONTRL[1]` is not capped (`vdi_asm.S:95-96`, signed) and its
/// `dbra` loop moves 65536 − |n| pairs — more than the buffer holds, so the
/// buffer ends up full of the caller's pairs. NewTOS fills it and stops
/// (`docs/COMPATIBILITY.md`, V23/V35): a `v_pline` with `CONTRL[1] = -1`
/// draws nothing, and a following `vsl_width` with `CONTRL[1] = 0` reads its
/// `PTSIN[0]` 7. The caller's count is unchanged afterwards.
#[test]
fn v35_negative_count_fills_the_copy() {
    let _guard = VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let wk = workstation();
    let mut pb = Pb::new();
    reset_vdi(&mut pb, wk);
    // The harness PTSIN holds 256 words; the copy reads 2048 from a
    // 2048-word array of its own.
    let mut pts = vec![0i16; 2 * 1024];
    pts[0] = 7;
    let mut contrl = [0i16; 12];
    let (mut intin, mut intout, mut ptsout) = ([0i16; 16], [0i16; 16], [0i16; 16]);
    contrl[0] = 6;
    contrl[1] = -1;
    contrl[6] = wk.handle;
    let block: [u64; 5] = [
        contrl.as_mut_ptr() as u64,
        intin.as_mut_ptr() as u64,
        pts.as_mut_ptr() as u64,
        intout.as_mut_ptr() as u64,
        ptsout.as_mut_ptr() as u64,
    ];
    // SAFETY: five live arrays of this frame; `pts` holds the 1024 pairs the
    // copy reads for a negative count.
    unsafe { tos_api::vdi::dispatch(block.as_ptr() as u64, crate::harness::ABI) };
    assert_eq!(contrl[1], -1, "the caller's count comes back");
    run(&mut pb, wk.handle, &[raw(16, &[], &[1, 0], 0)]);
    assert_eq!([pb.ptsout[0], pb.ptsout[1]], [7, 0], "vsl_width reads the copy the negative count filled");
}

// ── the oracle's screen words (method/gen-spans.py of the V31 run) ─────────

/// TPA: grab (0,0) 300x250, record 345 of the V18 part 2 run's probe/e1.
#[rustfmt::skip]
const SP_TPA: &[Span] = &[
    (100, 100, 200, RED), (101, 100, 200, RED), (102, 100, 200, RED), (103, 100, 200, RED),
    (104, 100, 200, RED), (105, 100, 200, RED), (106, 100, 200, RED), (107, 100, 200, RED),
    (108, 100, 200, RED), (109, 100, 200, RED), (110, 100, 200, RED), (111, 100, 200, RED),
    (112, 100, 200, RED), (113, 100, 200, RED), (114, 100, 200, RED), (115, 100, 200, RED),
    (116, 100, 200, RED), (117, 100, 200, RED), (118, 100, 200, RED), (119, 100, 200, RED),
    (120, 100, 200, RED), (121, 100, 200, RED), (122, 100, 200, RED), (123, 100, 200, RED),
    (124, 100, 200, RED), (125, 100, 200, RED), (126, 100, 200, RED), (127, 100, 200, RED),
    (128, 100, 200, RED), (129, 100, 200, RED), (130, 100, 200, RED), (131, 100, 200, RED),
    (132, 100, 200, RED), (133, 100, 200, RED), (134, 100, 200, RED), (135, 100, 200, RED),
    (136, 100, 200, RED), (137, 100, 200, RED), (138, 100, 200, RED), (139, 100, 200, RED),
    (140, 100, 200, RED), (141, 100, 200, RED), (142, 100, 200, RED), (143, 100, 200, RED),
    (144, 100, 200, RED), (145, 100, 200, RED), (146, 100, 200, RED), (147, 100, 200, RED),
    (148, 100, 200, RED), (149, 100, 200, RED), (150, 100, 200, RED),
];

/// TPB: grab (0,0) 300x250, record 346 of the V18 part 2 run's probe/e1.
#[rustfmt::skip]
const SP_TPB: &[Span] = &[
    (10, 10, 200, RED), (11, 10, 200, RED), (12, 10, 200, RED), (13, 10, 200, RED),
    (14, 10, 200, RED), (15, 10, 200, RED), (16, 10, 200, RED), (17, 10, 200, RED),
    (18, 10, 200, RED), (19, 10, 200, RED), (20, 10, 200, RED), (21, 10, 200, RED),
    (22, 10, 200, RED), (23, 10, 200, RED), (24, 10, 200, RED), (25, 10, 200, RED),
    (26, 10, 200, RED), (27, 10, 200, RED), (28, 10, 200, RED), (29, 10, 200, RED),
    (30, 10, 200, RED), (31, 10, 200, RED), (32, 10, 200, RED), (33, 10, 200, RED),
    (34, 10, 200, RED), (35, 10, 200, RED), (36, 10, 200, RED), (37, 10, 200, RED),
    (38, 10, 200, RED), (39, 10, 200, RED), (40, 10, 200, RED), (41, 10, 200, RED),
    (42, 10, 200, RED), (43, 10, 200, RED), (44, 10, 200, RED), (45, 10, 200, RED),
    (46, 10, 200, RED), (47, 10, 200, RED), (48, 10, 200, RED), (49, 10, 200, RED),
    (50, 10, 200, RED), (51, 10, 200, RED), (52, 10, 200, RED), (53, 10, 200, RED),
    (54, 10, 200, RED), (55, 10, 200, RED), (56, 10, 200, RED), (57, 10, 200, RED),
    (58, 10, 200, RED), (59, 10, 200, RED), (60, 10, 200, RED), (61, 10, 200, RED),
    (62, 10, 200, RED), (63, 10, 200, RED), (64, 10, 200, RED), (65, 10, 200, RED),
    (66, 10, 200, RED), (67, 10, 200, RED), (68, 10, 200, RED), (69, 10, 200, RED),
    (70, 10, 200, RED), (71, 10, 200, RED), (72, 10, 200, RED), (73, 10, 200, RED),
    (74, 10, 200, RED), (75, 10, 200, RED), (76, 10, 200, RED), (77, 10, 200, RED),
    (78, 10, 200, RED), (79, 10, 200, RED), (80, 10, 200, RED), (81, 10, 200, RED),
    (82, 10, 200, RED), (83, 10, 200, RED), (84, 10, 200, RED), (85, 10, 200, RED),
    (86, 10, 200, RED), (87, 10, 200, RED), (88, 10, 200, RED), (89, 10, 200, RED),
    (90, 10, 200, RED), (91, 10, 200, RED), (92, 10, 200, RED), (93, 10, 200, RED),
    (94, 10, 200, RED), (95, 10, 200, RED), (96, 10, 200, RED), (97, 10, 200, RED),
    (98, 10, 200, RED), (99, 10, 200, RED), (100, 10, 200, RED), (101, 10, 200, RED),
    (102, 10, 200, RED), (103, 10, 200, RED), (104, 10, 200, RED), (105, 10, 200, RED),
    (106, 10, 200, RED), (107, 10, 200, RED), (108, 10, 200, RED), (109, 10, 200, RED),
    (110, 10, 200, RED), (111, 10, 200, RED), (112, 10, 200, RED), (113, 10, 200, RED),
    (114, 10, 200, RED), (115, 10, 200, RED), (116, 10, 200, RED), (117, 10, 200, RED),
    (118, 10, 200, RED), (119, 10, 200, RED), (120, 10, 200, RED), (121, 10, 200, RED),
    (122, 10, 200, RED), (123, 10, 200, RED), (124, 10, 200, RED), (125, 10, 200, RED),
    (126, 10, 200, RED), (127, 10, 200, RED), (128, 10, 200, RED), (129, 10, 200, RED),
    (130, 10, 200, RED), (131, 10, 200, RED), (132, 10, 200, RED), (133, 10, 200, RED),
    (134, 10, 200, RED), (135, 10, 200, RED), (136, 10, 200, RED), (137, 10, 200, RED),
    (138, 10, 200, RED), (139, 10, 200, RED), (140, 10, 200, RED), (141, 10, 200, RED),
    (142, 10, 200, RED), (143, 10, 200, RED), (144, 10, 200, RED), (145, 10, 200, RED),
    (146, 10, 200, RED), (147, 10, 200, RED), (148, 10, 200, RED), (149, 10, 200, RED),
    (150, 10, 200, RED),
];

/// TPC: grab (0,280) 220x40, record 347 of the V18 part 2 run's probe/e1.
#[rustfmt::skip]
const SP_TPC: &[Span] = &[
    (296, 20, 200, B), (297, 20, 200, B), (298, 20, 200, B), (299, 20, 200, B), (300, 20, 200, B),
    (301, 20, 200, B), (302, 20, 200, B), (303, 20, 200, B), (304, 20, 200, B),
];

/// TPD: grab (0,340) 100x50, record 348 of the V18 part 2 run's probe/e1.
#[rustfmt::skip]
const SP_TPD: &[Span] = &[
    (353, 20, 67, P), (354, 20, 67, P), (355, 20, 67, P), (356, 20, 67, P), (357, 20, 25, P),
    (357, 26, 29, B), (357, 30, 37, P), (357, 38, 47, B), (357, 48, 55, P), (357, 56, 63, B),
    (357, 64, 67, P), (358, 20, 25, P), (358, 26, 29, B), (358, 30, 37, P), (358, 38, 47, B),
    (358, 48, 55, P), (358, 56, 63, B), (358, 64, 67, P), (359, 20, 23, P), (359, 24, 31, B),
    (359, 32, 37, P), (359, 38, 49, B), (359, 50, 53, P), (359, 54, 65, B), (359, 66, 67, P),
    (360, 20, 23, P), (360, 24, 31, B), (360, 32, 37, P), (360, 38, 49, B), (360, 50, 53, P),
    (360, 54, 65, B), (360, 66, 67, P), (361, 20, 21, P), (361, 22, 33, B), (361, 34, 37, P),
    (361, 38, 41, B), (361, 42, 45, P), (361, 46, 49, B), (361, 50, 53, P), (361, 54, 57, B),
    (361, 58, 61, P), (361, 62, 65, B), (361, 66, 67, P), (362, 20, 21, P), (362, 22, 33, B),
    (362, 34, 37, P), (362, 38, 41, B), (362, 42, 45, P), (362, 46, 49, B), (362, 50, 53, P),
    (362, 54, 57, B), (362, 58, 61, P), (362, 62, 65, B), (362, 66, 67, P), (363, 20, 21, P),
    (363, 22, 25, B), (363, 26, 29, P), (363, 30, 33, B), (363, 34, 37, P), (363, 38, 41, B),
    (363, 42, 45, P), (363, 46, 49, B), (363, 50, 53, P), (363, 54, 57, B), (363, 58, 61, P),
    (363, 62, 65, B), (363, 66, 67, P), (364, 20, 21, P), (364, 22, 25, B), (364, 26, 29, P),
    (364, 30, 33, B), (364, 34, 37, P), (364, 38, 41, B), (364, 42, 45, P), (364, 46, 49, B),
    (364, 50, 53, P), (364, 54, 57, B), (364, 58, 61, P), (364, 62, 65, B), (364, 66, 67, P),
    (365, 20, 21, P), (365, 22, 25, B), (365, 26, 29, P), (365, 30, 33, B), (365, 34, 37, P),
    (365, 38, 49, B), (365, 50, 53, P), (365, 54, 57, B), (365, 58, 67, P), (366, 20, 21, P),
    (366, 22, 25, B), (366, 26, 29, P), (366, 30, 33, B), (366, 34, 37, P), (366, 38, 49, B),
    (366, 50, 53, P), (366, 54, 57, B), (366, 58, 67, P), (367, 20, 21, P), (367, 22, 25, B),
    (367, 26, 29, P), (367, 30, 33, B), (367, 34, 37, P), (367, 38, 47, B), (367, 48, 53, P),
    (367, 54, 57, B), (367, 58, 67, P), (368, 20, 21, P), (368, 22, 25, B), (368, 26, 29, P),
    (368, 30, 33, B), (368, 34, 37, P), (368, 38, 47, B), (368, 48, 53, P), (368, 54, 57, B),
    (368, 58, 67, P), (369, 20, 21, P), (369, 22, 33, B), (369, 34, 37, P), (369, 38, 41, B),
    (369, 42, 45, P), (369, 46, 49, B), (369, 50, 53, P), (369, 54, 57, B), (369, 58, 67, P),
    (370, 20, 21, P), (370, 22, 33, B), (370, 34, 37, P), (370, 38, 41, B), (370, 42, 45, P),
    (370, 46, 49, B), (370, 50, 53, P), (370, 54, 57, B), (370, 58, 67, P), (371, 20, 21, P),
    (371, 22, 33, B), (371, 34, 37, P), (371, 38, 41, B), (371, 42, 45, P), (371, 46, 49, B),
    (371, 50, 53, P), (371, 54, 57, B), (371, 58, 67, P), (372, 20, 21, P), (372, 22, 33, B),
    (372, 34, 37, P), (372, 38, 41, B), (372, 42, 45, P), (372, 46, 49, B), (372, 50, 53, P),
    (372, 54, 57, B), (372, 58, 67, P), (373, 20, 21, P), (373, 22, 25, B), (373, 26, 29, P),
    (373, 30, 33, B), (373, 34, 37, P), (373, 38, 41, B), (373, 42, 45, P), (373, 46, 49, B),
    (373, 50, 53, P), (373, 54, 57, B), (373, 58, 61, P), (373, 62, 65, B), (373, 66, 67, P),
    (374, 20, 21, P), (374, 22, 25, B), (374, 26, 29, P), (374, 30, 33, B), (374, 34, 37, P),
    (374, 38, 41, B), (374, 42, 45, P), (374, 46, 49, B), (374, 50, 53, P), (374, 54, 57, B),
    (374, 58, 61, P), (374, 62, 65, B), (374, 66, 67, P), (375, 20, 21, P), (375, 22, 25, B),
    (375, 26, 29, P), (375, 30, 33, B), (375, 34, 37, P), (375, 38, 41, B), (375, 42, 45, P),
    (375, 46, 49, B), (375, 50, 53, P), (375, 54, 57, B), (375, 58, 61, P), (375, 62, 65, B),
    (375, 66, 67, P), (376, 20, 21, P), (376, 22, 25, B), (376, 26, 29, P), (376, 30, 33, B),
    (376, 34, 37, P), (376, 38, 41, B), (376, 42, 45, P), (376, 46, 49, B), (376, 50, 53, P),
    (376, 54, 57, B), (376, 58, 61, P), (376, 62, 65, B), (376, 66, 67, P), (377, 20, 21, P),
    (377, 22, 25, B), (377, 26, 29, P), (377, 30, 33, B), (377, 34, 37, P), (377, 38, 49, B),
    (377, 50, 53, P), (377, 54, 65, B), (377, 66, 67, P), (378, 20, 21, P), (378, 22, 25, B),
    (378, 26, 29, P), (378, 30, 33, B), (378, 34, 37, P), (378, 38, 49, B), (378, 50, 53, P),
    (378, 54, 65, B), (378, 66, 67, P), (379, 20, 21, P), (379, 22, 25, B), (379, 26, 29, P),
    (379, 30, 33, B), (379, 34, 37, P), (379, 38, 47, B), (379, 48, 55, P), (379, 56, 63, B),
    (379, 64, 67, P), (380, 20, 21, P), (380, 22, 25, B), (380, 26, 29, P), (380, 30, 33, B),
    (380, 34, 37, P), (380, 38, 47, B), (380, 48, 55, P), (380, 56, 63, B), (380, 64, 67, P),
    (381, 20, 67, P), (382, 20, 67, P), (383, 20, 67, P), (384, 20, 67, P),
];

/// TPE: grab (0,0) 300x250, record 349 of the V18 part 2 run's probe/e1.
#[rustfmt::skip]
const SP_TPE: &[Span] = &[
    (10, 10, 50, RED), (11, 10, 50, RED), (12, 10, 50, RED), (13, 10, 50, RED),
    (14, 10, 50, RED), (15, 10, 50, RED), (16, 10, 50, RED), (17, 10, 50, RED),
    (18, 10, 50, RED), (19, 10, 50, RED), (20, 10, 50, RED), (21, 10, 50, RED),
    (22, 10, 50, RED), (23, 10, 50, RED), (24, 10, 50, RED), (25, 10, 50, RED),
    (26, 10, 50, RED), (27, 10, 50, RED), (28, 10, 50, RED), (29, 10, 50, RED),
    (30, 10, 50, RED), (31, 10, 50, RED), (32, 10, 50, RED), (33, 10, 50, RED),
    (34, 10, 50, RED), (35, 10, 50, RED), (36, 10, 50, RED), (37, 10, 50, RED),
    (38, 10, 50, RED), (39, 10, 50, RED), (40, 10, 50, RED), (41, 10, 50, RED),
    (42, 10, 50, RED), (43, 10, 50, RED), (44, 10, 50, RED), (45, 10, 50, RED),
    (46, 10, 50, RED), (47, 10, 50, RED), (48, 10, 50, RED), (49, 10, 50, RED),
    (50, 10, 50, RED), (100, 100, 100, B), (101, 101, 102, B), (102, 103, 104, B), (103, 105, 106, B),
    (104, 107, 108, B), (105, 109, 110, B), (106, 111, 112, B), (107, 113, 114, B), (108, 115, 116, B),
    (109, 117, 118, B), (110, 119, 120, B), (111, 121, 122, B), (112, 123, 124, B), (113, 125, 126, B),
    (114, 127, 128, B), (115, 129, 130, B), (116, 131, 132, B), (117, 133, 134, B), (118, 135, 136, B),
    (119, 137, 138, B), (120, 139, 140, B), (121, 141, 142, B), (122, 143, 144, B), (123, 145, 146, B),
    (124, 147, 148, B), (125, 149, 150, B), (126, 151, 152, B), (127, 153, 154, B), (128, 155, 156, B),
    (129, 157, 158, B), (130, 159, 160, B), (131, 161, 162, B), (132, 163, 164, B), (133, 165, 166, B),
    (134, 167, 168, B), (135, 169, 170, B), (136, 171, 172, B), (137, 173, 174, B), (138, 175, 176, B),
    (139, 177, 178, B), (140, 179, 180, B), (141, 181, 182, B), (142, 183, 184, B), (143, 185, 186, B),
    (144, 187, 188, B), (145, 189, 190, B), (146, 191, 192, B), (147, 193, 194, B), (148, 195, 196, B),
    (149, 197, 198, B), (150, 199, 200, B),
];

/// TPF: grab (0,280) 220x40, record 934 of the V35 run's probe/e1.
#[rustfmt::skip]
const SP_TPF: &[Span] = &[
    (297, 20, 200, B), (298, 20, 200, B), (299, 20, 200, B), (300, 20, 200, B), (301, 20, 200, B),
    (302, 20, 200, B), (303, 20, 200, B),
];

/// TPG: grab (0,280) 220x40, record 935 of the V35 run's probe/e1.
#[rustfmt::skip]
const SP_TPG: &[Span] = &[
    (298, 20, 200, B), (299, 20, 200, B), (300, 20, 200, B), (301, 20, 200, B), (302, 20, 200, B),
];

/// TPH: grab (0,280) 220x40, record 936 of the V35 run's probe/e1.
#[rustfmt::skip]
const SP_TPH: &[Span] = &[
    (299, 20, 200, B), (300, 20, 200, B), (301, 20, 200, B),
];
