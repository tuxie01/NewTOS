//! V23 (TASK/V23.md, finding VDI-DRAW-005 of
//! `docs/audits/2026-11-05-v18-t1-vdi-draw.md`): the dispatcher caps
//! `CONTRL[1]` at `MAX_VERTICES` for every VDI call, as EmuTOS' trap glue
//! does, and gives the caller its own count back.
//!
//! EmuTOS' `GSX_ENTRY` (`vdi/vdi_asm.S:88-116`) saves the caller's
//! `CONTRL[1]`, compares it **signed** with `MAX_VERTICES` = 1024
//! (`include/config.h:1654`), writes 1024 into the caller's `CONTRL[1]` when
//! it is larger, copies that many vertices into its local `PTSIN`, runs
//! `screen()`, and writes the saved count back. Every handler therefore sees
//! at most 1024 vertices, and the caller reads its own count afterwards.
//!
//! The oracle side is the probe `tosdump/probes/vdprobe.c` (`k5`), measured
//! on EmuTOS 1a7484ab / Hatari Falcon030 truecolor in
//! `measurements/measurement-2026-11-05-v18-t1/RUNINFO.TXT`:
//! - K5A, `v_pmarker` of 1030 plus markers, 0..1023 at (20,20) and
//!   1024..1029 at (40 + 10k, 20): EmuTOS draws none of the six
//!   (`probe/maps/30-K5A.txt`, the whole x 32..98 band white) and logs
//!   `contrl[1] after the call 1030` (`probe/e1/VDPROBE.LOG:32`);
//! - K5B, 1025 markers, the last at (60,20): EmuTOS does not draw it
//!   (`probe/maps/31-K5B.txt`);
//! - K5C, `v_pline` of 1030 vertices: equal on both sides, `contrl[1]` 1030
//!   afterwards (`probe/e1/VDPROBE.LOG:35`).
//!
//! A negative `CONTRL[1]` is below the cap in EmuTOS' signed compare and
//! reaches the handlers unchanged; its `dbra` copy then runs 65536 − |n|
//! pairs past the local buffer, which NewTOS does not reproduce
//! (`docs/COMPATIBILITY.md`). NewTOS' handlers draw nothing for it and the
//! caller's count is unchanged afterwards.
//!
//! Hand-written, like `cases_v20`..`cases_v22`.

use crate::harness::{reset_vdi, screen_word, workstation, Pb, Workstation, ABI, VDI_LOCK};

/// The cap, EmuTOS `MAX_VERTICES` (`include/config.h:1654`).
const MAX_VERTICES: usize = 1024;

const WHITE: u16 = 0xFFFF;

/// The probe's K5A/K5B grab rectangle (`vdprobe.c` `k5`).
const MARK_RECT: (i16, i16, i16, i16) = (0, 0, 120, 40);
/// The probe's K5C grab rectangle.
const LINE_RECT: (i16, i16, i16, i16) = (0, 50, 160, 40);

/// One VDI call with a `PTSIN` of `pts.len() / 2` vertices — more than the
/// harness [`Pb`] holds — and `CONTRL[1] = n`. Returns the caller's
/// `CONTRL` after the call.
fn call_big(wk: &Workstation, opcode: i16, pts: &mut [i16], n: i16) -> [i16; 12] {
    let mut contrl = [0i16; 12];
    let mut intin = [0i16; 16];
    let mut intout = [0i16; 16];
    let mut ptsout = [0i16; 16];
    contrl[0] = opcode;
    contrl[1] = n;
    contrl[6] = wk.handle;
    let block: [u64; 5] = [
        contrl.as_mut_ptr() as u64,
        intin.as_mut_ptr() as u64,
        pts.as_mut_ptr() as u64,
        intout.as_mut_ptr() as u64,
        ptsout.as_mut_ptr() as u64,
    ];
    // SAFETY: the five pointers are live arrays of this frame; `pts` holds
    // every vertex a non-negative `n` announces, and a negative `n` reads
    // none.
    unsafe { tos_api::vdi::dispatch(block.as_ptr() as u64, ABI) };
    contrl
}

/// After the dumper's reset, `vsm_type(2)` (plus), as the probe's `i1(18, 2)`.
fn reset_plus(pb: &mut Pb, wk: &Workstation) {
    reset_vdi(pb, wk);
    pb.contrl.fill(0);
    pb.intin[0] = 2;
    pb.contrl[0] = 18;
    pb.contrl[3] = 1;
    pb.contrl[6] = wk.handle;
    pb.call();
}

fn grab((x0, y0, w, h): (i16, i16, i16, i16)) -> Vec<u16> {
    let mut v = Vec::with_capacity(w as usize * h as usize);
    for y in y0..y0 + h {
        for x in x0..x0 + w {
            v.push(screen_word(x, y));
        }
    }
    v
}

fn differing(name: &str, rect: (i16, i16, i16, i16), want: &[u16], got: &[u16]) -> Vec<String> {
    let (x0, y0, w, _) = rect;
    want.iter()
        .zip(got)
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .map(|(i, (a, b))| {
            let (x, y) = (x0 + (i % w as usize) as i16, y0 + (i / w as usize) as i16);
            format!("{name} ({x},{y}) got {b:#06x} want {a:#06x}")
        })
        .collect()
}

/// `vdprobe.c` K5A/K5B: 1024 markers at (20,20), then the markers past the
/// cap at `extra`.
fn marker_points(extra: &[(i16, i16)]) -> Vec<i16> {
    let mut pts = vec![0i16; 2 * (MAX_VERTICES + extra.len())];
    for i in 0..MAX_VERTICES {
        pts[2 * i] = 20;
        pts[2 * i + 1] = 20;
    }
    for (k, &(x, y)) in extra.iter().enumerate() {
        pts[2 * (MAX_VERTICES + k)] = x;
        pts[2 * (MAX_VERTICES + k) + 1] = y;
    }
    pts
}

/// The screen of one plus marker at (20,20) — what 1024 markers on the same
/// spot draw in replace mode.
fn one_marker_at_20_20(pb: &mut Pb, wk: &Workstation) -> Vec<u16> {
    reset_plus(pb, wk);
    let mut pts = [20i16, 20];
    call_big(wk, 7, &mut pts, 1);
    let want = grab(MARK_RECT);
    assert!(want.iter().any(|&w| w != WHITE), "the comparison marker draws");
    want
}

/// Probe K5A: of 1030 markers EmuTOS draws the first 1024 only — the six at
/// (40 + 10k, 20) stay white (`probe/maps/30-K5A.txt`, E) — and the caller
/// reads 1030 back (`probe/e1/VDPROBE.LOG:32`).
#[test]
fn v23_probe_k5a_pmarker_1030_draws_only_the_first_1024() {
    let _guard = VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let wk = workstation();
    let mut pb = Pb::new();
    let want = one_marker_at_20_20(&mut pb, wk);

    reset_plus(&mut pb, wk);
    let extra: Vec<(i16, i16)> = (0..6).map(|k| (40 + 10 * k, 20)).collect();
    let mut pts = marker_points(&extra);
    let contrl = call_big(wk, 7, &mut pts, 1030);
    let got = grab(MARK_RECT);

    assert_eq!(contrl[1], 1030, "K5A: CONTRL[1] after the call is the caller's count");
    for &(x, y) in &extra {
        assert_eq!(screen_word(x, y), WHITE, "K5A: marker {x},{y} past MAX_VERTICES is not drawn");
    }
    let diffs = differing("K5A", MARK_RECT, &want, &got);
    assert!(diffs.is_empty(), "K5A: {} pixels differ, first: {:?}", diffs.len(), &diffs[..diffs.len().min(8)]);
}

/// Probe K5B: the 1025th marker at (60,20) is not drawn
/// (`probe/maps/31-K5B.txt`, E).
#[test]
fn v23_probe_k5b_pmarker_1025_drops_the_last() {
    let _guard = VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let wk = workstation();
    let mut pb = Pb::new();
    let want = one_marker_at_20_20(&mut pb, wk);

    reset_plus(&mut pb, wk);
    let mut pts = marker_points(&[(60, 20)]);
    let contrl = call_big(wk, 7, &mut pts, 1025);
    let got = grab(MARK_RECT);

    assert_eq!(contrl[1], 1025, "K5B: CONTRL[1] after the call is the caller's count");
    assert_eq!(screen_word(60, 20), WHITE, "K5B: the 1025th marker is not drawn");
    let diffs = differing("K5B", MARK_RECT, &want, &got);
    assert!(diffs.is_empty(), "K5B: {} pixels differ, first: {:?}", diffs.len(), &diffs[..diffs.len().min(8)]);
}

/// The cap is `CONTRL[1] > MAX_VERTICES` (`vdi_asm.S:95-96`, `jle`): exactly
/// 1024 markers are all drawn, the 1024th at (60,20) included.
#[test]
fn v23_pmarker_exactly_1024_draws_all() {
    let _guard = VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let wk = workstation();
    let mut pb = Pb::new();
    reset_plus(&mut pb, wk);
    let mut pts = marker_points(&[]);
    pts[2 * (MAX_VERTICES - 1)] = 60;
    let contrl = call_big(wk, 7, &mut pts, MAX_VERTICES as i16);
    assert_eq!(contrl[1], MAX_VERTICES as i16);
    assert_ne!(screen_word(60, 20), WHITE, "the 1024th marker is drawn");
}

/// Probe K5C, the control: `v_pline` of 1030 vertices, 0..1022 at (20,60),
/// 1023 at (40,60), 1024.. at (40 + 20k, 80) — the first 1024 are the one
/// segment (20,60)-(40,60), equal on both sides; `CONTRL[1]` 1030 after.
#[test]
fn v23_probe_k5c_pline_1030_draws_the_first_1024() {
    let _guard = VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let wk = workstation();
    let mut pb = Pb::new();

    reset_vdi(&mut pb, wk);
    let mut seg = [20i16, 60, 40, 60];
    call_big(wk, 6, &mut seg, 2);
    let want = grab(LINE_RECT);
    assert!(want.iter().any(|&w| w != WHITE), "the comparison segment draws");

    reset_vdi(&mut pb, wk);
    let mut pts = vec![0i16; 2 * 1030];
    for i in 0..1023 {
        pts[2 * i] = 20;
        pts[2 * i + 1] = 60;
    }
    pts[2046] = 40;
    pts[2047] = 60;
    for k in 0..6 {
        pts[2 * (1024 + k)] = 40 + 20 * k as i16;
        pts[2 * (1024 + k) + 1] = 80;
    }
    let contrl = call_big(wk, 6, &mut pts, 1030);
    let got = grab(LINE_RECT);

    assert_eq!(contrl[1], 1030, "K5C: CONTRL[1] after the call is the caller's count");
    let diffs = differing("K5C", LINE_RECT, &want, &got);
    assert!(diffs.is_empty(), "K5C: {} pixels differ, first: {:?}", diffs.len(), &diffs[..diffs.len().min(8)]);
}

/// A negative `CONTRL[1]` passes EmuTOS' signed cap unchanged
/// (`vdi_asm.S:95`); NewTOS' three count readers draw nothing for it — as
/// EmuTOS' `vdi_v_pmarker` loop (`vdi_marker.c:124`), `polyline`
/// (`vdi_line.c:1106`) and `wideline` (`:1397-1398`) do not — and the
/// caller's count is unchanged afterwards. The copy past EmuTOS' buffer is
/// not reproduced (`docs/COMPATIBILITY.md`).
#[test]
fn v23_negative_count_draws_nothing_and_is_kept() {
    let _guard = VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let wk = workstation();
    let mut pb = Pb::new();
    for opcode in [6i16, 7, 9] {
        for n in [-1i16, -1030, i16::MIN] {
            reset_vdi(&mut pb, wk);
            let mut pts = [20i16, 20, 60, 20, 40, 35];
            let contrl = call_big(wk, opcode, &mut pts, n);
            assert_eq!(contrl[1], n, "opcode {opcode}, CONTRL[1] = {n}: the count is the caller's afterwards");
            assert!(
                grab(MARK_RECT).iter().all(|&w| w == WHITE),
                "opcode {opcode}, CONTRL[1] = {n}: nothing is drawn"
            );
        }
    }
}
