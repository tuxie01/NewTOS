//! V30 (TASK/V30.md, findings VDI-TEXT-004/005/006 of
//! `docs/audits/2026-11-06-v18-t2-vdi-text.md`): the text underline and
//! the glyphs of `output_text` against the pinned oracle.
//!
//! - **004:** the underline runs from `startx` to `DESTX`, the pen *behind*
//!   the last glyph, and `abline` draws that endpoint too
//!   (`vdi/vdi_text.c:500-535`, `line->x2 = DESTX` `:509`) — one pixel more
//!   than the glyph cells cover.
//! - **005:** with `vs_clip(0)` `check_clip` returns 0
//!   (`vdi/vdi_textblit.c:128-133`), `do_clip` cuts nothing (`:176-183`) and
//!   `screen_blit16` writes every cell pixel at `v_bas_ad + DESTY * v_lin_wr
//!   + DESTX * 2` (`:1033-1035`): a glyph past the right edge continues at
//!   the start of the next row, one left of `x = 0` at the end of the
//!   previous row. NewTOS does the same inside the logical screen and drops
//!   what falls outside it (user decision, TASK/V30, the rule of decision A,
//!   `docs/COMPATIBILITY.md`).
//! - **006:** the underline with clipping off is `abline` without
//!   `clip_line` (`vdi/vdi_text.c:519-534`), so it runs on like any line
//!   with clipping off (V20).
//! - **Found by V30's own cases:** `LN_MASK` turns from underline line to
//!   underline line (`abline`, then `rorw1`, `vdi/vdi_text.c:541`), and the
//!   underline of a horizontal `v_justified` carries the outline's shift
//!   (`:410-411`).
//!
//! Three sources, all from the pinned oracle, EmuTOS 1a7484ab on Hatari
//! Falcon030 truecolor (run
//! `measurements/measurement-2026-11-06-v18-t2/RUNINFO.TXT`):
//! - cases 61/64/67/70/74/76/79 of `tosdump/cases/vdi-v18-t2.tsv` (CRC
//!   `BEFB79A7`), replayed against
//!   `tosdump/reference/EMUTOS-1a7484ab-VDI-v18t2-falcontc.BIN` (record
//!   index = the case's position among the table's measured lines);
//! - the probe cases TUA–TUF and T14B/T14C/T14G/T14H/T14L of
//!   `tosdump/probes/vdprobe.c` (`tu`, `t14`): every non-white screen word
//!   of each grab rectangle, decoded from `probe/e1/VDPROBE.BIN.gz` of that
//!   run (the maps `probe/maps/339-TUAw.txt` … `344-TUF.txt`,
//!   `306-T14B.txt` … `316-T14L.txt` show the differing ones). `B` is pen 1
//!   (black, glyph ink and underline), `P` is pen 0 (`0xFFDF`, the glyph
//!   cell's background in replace mode); every other word is the cleared
//!   screen's `0xFFFF`. TUD (clip on) and TUF (inside the screen) are the
//!   controls; TUD's wrap rectangle `TUDw` is white.
//!
//! - the probe cases TV* of the same probe (`tv`), added by V30 for the
//!   paths those leave out: rotated, the writing modes, effects,
//!   `v_justified`, the direct blit; decoded from
//!   `measurements/measurement-2026-11-06-v30/probe/e1/VDPROBE.BIN.gz`.
//!
//! The T14 cases send up to 4100 characters in one call, more than the
//! harness's `INTIN`; they go through their own parameter block
//! ([`big_call`]).
//!
//! Hand-written, like `cases_v20`..`cases_v29`.

use crate::cases_v20::check;
use crate::harness::{reset_vdi, screen_word, workstation, Call, Case, Pb, ABI, VDI_LOCK};

static FALCON_TC_V18T2: &[u8] = include_bytes!("../../../tosdump/reference/EMUTOS-1a7484ab-VDI-v18t2-falcontc.BIN");

const B: u16 = 0x0000;
const P: u16 = 0xFFDF;
const WHITE: u16 = 0xFFFF;

const ABCDEFGH: &[i16] = &[65, 66, 67, 68, 69, 70, 71, 72];

const UNDERLINE: Call = Call { opcode: 106, sub: 0, intin: &[8], ptsin: &[] };
const UNDERLINE_LIGHT: Call = Call { opcode: 106, sub: 0, intin: &[8 | 2], ptsin: &[] };
const CLIP_SCREEN: Call = Call { opcode: 129, sub: 0, intin: &[1], ptsin: &[0, 0, 639, 479] };
const HEIGHT_26: Call = Call { opcode: 12, sub: 0, intin: &[], ptsin: &[0, 26] };

/// `v_gtext` 'ABCDEFGH' at `PTSIN[0..1]`.
const fn gtext(at: &'static [i16]) -> Call {
    Call { opcode: 8, sub: 0, intin: ABCDEFGH, ptsin: at }
}

fn case(prelude: &'static [Call], id: u16, rec: usize, ptsin: &'static [i16], comment: &'static str) -> Case {
    Case { prelude, id, rec, opcode: 105, sub: 0, intin: &[], ptsin, comment }
}

// ── vdi-v18-t2.tsv cases 61/64/67/70/74/76/79 ─────────────────────────────

/// 'ABCDEFGH' at (100,100) in the 6×6 font covers x 100..147; the underline
/// ends at `DESTX` = 148 (004).
#[test]
fn v30_case_0061_underline_ends_on_destx() {
    const PRE: &[Call] = &[UNDERLINE, gtext(&[100, 100])];
    check(FALCON_TC_V18T2, &case(PRE, 61, 17, &[148, 101], "rider underline end (148,101)"));
}

/// The control: (147,101), the last underline pixel inside the glyph cells.
#[test]
fn v30_case_0064_underline_last_cell_pixel_control() {
    const PRE: &[Call] = &[UNDERLINE, gtext(&[100, 100])];
    check(FALCON_TC_V18T2, &case(PRE, 64, 18, &[147, 101], "rider control (147,101)"));
}

/// From x −30 the underline ends at 18 (004 with clipping off).
#[test]
fn v30_case_0067_underline_end_from_x_minus_30() {
    const PRE: &[Call] = &[UNDERLINE, gtext(&[-30, 150])];
    check(FALCON_TC_V18T2, &case(PRE, 67, 19, &[18, 151], "rider underline end (18,151)"));
}

/// Clip off, from (600,100): the underline 600..648 runs on into row 102
/// up to x 8 (006).
#[test]
fn v30_case_0070_underline_runs_on_into_the_next_row() {
    const PRE: &[Call] = &[UNDERLINE, gtext(&[600, 100])];
    check(FALCON_TC_V18T2, &case(PRE, 70, 20, &[8, 102], "rider underline past x 639: (8,102)"));
}

/// The control: clip on, the underline stops at x 639, (8,102) stays white.
#[test]
fn v30_case_0074_underline_clip_on_control() {
    const PRE: &[Call] = &[UNDERLINE, CLIP_SCREEN, gtext(&[600, 100])];
    check(FALCON_TC_V18T2, &case(PRE, 74, 21, &[8, 102], "rider control: clip on, (8,102)"));
}

/// Clip off, no effects: the glyphs past x 639 continue in rows 97.. at
/// x 0 (005).
#[test]
fn v30_case_0076_glyph_runs_on_into_the_next_row() {
    const PRE: &[Call] = &[gtext(&[600, 100])];
    check(FALCON_TC_V18T2, &case(PRE, 76, 22, &[0, 97], "rider glyph past x 639: (0,97)"));
}

/// The control: clip on, (0,97) stays white.
#[test]
fn v30_case_0079_glyph_clip_on_control() {
    const PRE: &[Call] = &[CLIP_SCREEN, gtext(&[600, 100])];
    check(FALCON_TC_V18T2, &case(PRE, 79, 23, &[0, 97], "rider control: clip on, (0,97)"));
}

// ── the probe: whole grab rectangles ──────────────────────────────────────

/// `(y, x_first, x_last, word)`.
type Span = (i16, i16, i16, u16);

/// A grab rectangle `(x, y, w, h)` and the oracle's non-white words in it.
type Grab = ((i16, i16, i16, i16), &'static [Span]);

/// Every screen word of every grab rectangle must be the word of the span
/// it lies in, and `0xFFFF` outside every span.
fn compare(name: &str, grabs: &[Grab]) {
    let mut diffs = Vec::new();
    for &((gx, gy, gw, gh), spans) in grabs {
        for y in gy..gy + gh {
            for x in gx..gx + gw {
                let want = spans
                    .iter()
                    .find(|&&(sy, x0, x1, _)| sy == y && (x0..=x1).contains(&x))
                    .map_or(WHITE, |&(_, _, _, w)| w);
                let got = screen_word(x, y);
                if got != want {
                    diffs.push(format!("({x},{y}) got {got:#06x} want {want:#06x}"));
                }
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

/// The probe's `tbegin` (its `reset` plus the text part, the dumper's
/// reset), then `calls` in order, then [`compare`].
fn probe(name: &str, calls: &[Call], grabs: &[Grab]) {
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
    compare(name, grabs);
}

/// One VDI call with an `INTIN` of any length — the probe's `bigin`
/// (`gtext_big`, `gtext_big2`, `justified_big`), up to 4198 words.
fn big_call(opcode: i16, sub: i16, intin: &[i16], ptsin: &[i16], handle: i16) {
    let mut contrl = vec![0i16; 16];
    let mut intin_v = intin.to_vec();
    intin_v.resize(intin.len().max(16), 0);
    let mut ptsin_v = vec![0i16; 16];
    ptsin_v[..ptsin.len()].copy_from_slice(ptsin);
    let mut intout = vec![0i16; 512];
    let mut ptsout = vec![0i16; 512];
    contrl[0] = opcode;
    contrl[1] = (ptsin.len() / 2) as i16;
    contrl[3] = intin.len() as i16;
    contrl[5] = sub;
    contrl[6] = handle;
    let block: [u64; 5] = [
        contrl.as_mut_ptr() as u64,
        intin_v.as_mut_ptr() as u64,
        ptsin_v.as_mut_ptr() as u64,
        intout.as_mut_ptr() as u64,
        ptsout.as_mut_ptr() as u64,
    ];
    // SAFETY: `block` holds five live pointers to vectors that outlive the
    // call and are larger than anything the opcodes under test touch.
    unsafe { tos_api::vdi::dispatch(block.as_ptr() as u64, ABI) };
}

/// 'A'..'Z' cycling, `n` codes (the probe's `gtext_big`).
fn big_text(n: usize) -> Vec<i16> {
    (0..n).map(|i| b'A' as i16 + (i % 26) as i16).collect()
}

/// The probe's `tbegin`, `pre` (small calls), then one big call, then
/// [`compare`].
fn probe_big(name: &str, pre: &[Call], opcode: i16, sub: i16, intin: &[i16], ptsin: &[i16], grabs: &[Grab]) {
    let _guard = VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let wk = workstation();
    let mut pb = Pb::new();
    reset_vdi(&mut pb, wk);
    for p in pre {
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
    big_call(opcode, sub, intin, ptsin, wk.handle);
    compare(name, grabs);
}

// ── TU: the underline with clipping off ───────────────────────────────────

/// Clip off, 'ABCDEFGH' underlined at (600,100): the last glyphs and the
/// underline continue in the next rows at x 0.
#[test]
fn v30_probe_tua_underline_runs_past_x_639() {
    probe("TUA", &[UNDERLINE, gtext(&[600, 100])], &[((560, 80, 80, 40), SP_TUA), ((0, 80, 80, 40), SP_TUAW)]);
}

/// Clip off, from x −30: the first glyphs and the underline start at the
/// end of the previous rows.
#[test]
fn v30_probe_tub_underline_starts_left_of_x_0() {
    probe("TUB", &[UNDERLINE, gtext(&[-30, 150])], &[((0, 130, 80, 40), SP_TUB), ((600, 130, 40, 40), SP_TUBW)]);
}

/// Clip off, exact double (`vst_height` 26): the thick underline runs on.
#[test]
fn v30_probe_tuc_doubled_underline_runs_on() {
    probe(
        "TUC",
        &[UNDERLINE, HEIGHT_26, gtext(&[560, 200])],
        &[((520, 160, 120, 60), SP_TUC), ((0, 160, 120, 60), SP_TUCW)],
    );
}

/// The control: clip on, nothing past x 639.
#[test]
fn v30_probe_tud_clip_on_control() {
    probe(
        "TUD",
        &[UNDERLINE, CLIP_SCREEN, gtext(&[600, 100])],
        &[((560, 80, 80, 40), SP_TUD), ((0, 80, 80, 40), SP_TUDW)],
    );
}

/// Clip off, underline + light: the dotted underline runs on — row 301 of
/// the screen rectangle and row 302 of the wrap rectangle, the rows the
/// underline is drawn in.
///
/// Only those rows: EmuTOS draws the *glyphs* of this case unlightened in
/// truecolor, NewTOS lightens them — a separate finding, measured but not
/// traced to its cause, outside V30 (audit
/// `docs/audits/2026-11-06-v18-t2-vdi-text.md` section 9, "TUE";
/// `measurements/measurement-2026-11-06-v18-t2/probe/maps/343-TUE.txt`).
/// The glyphs' bottom cell row, which these rows also hold, is empty in
/// the 6×6 font, so the lightening does not reach them.
#[test]
fn v30_probe_tue_light_underline_runs_on() {
    probe("TUE", &[UNDERLINE_LIGHT, gtext(&[600, 300])], &[((560, 301, 80, 1), SP_TUE), ((0, 302, 80, 1), SP_TUEW)]);
}

/// The control inside the screen: only the underline's end pixel (004).
#[test]
fn v30_probe_tuf_underline_inside() {
    probe("TUF", &[UNDERLINE, gtext(&[100, 100])], &[((80, 80, 120, 40), SP_TUF)]);
}

// ── T14: long strings with clipping off ───────────────────────────────────

/// Exact double, 4100 characters from (10,200): the `WORD` pen wraps once
/// and every glyph lands at its store index.
#[test]
fn v30_probe_t14b_doubled_4100_chars() {
    probe_big("T14B", &[HEIGHT_26], 8, 0, &big_text(4100), &[10, 200], &[((0, 160, 640, 60), SP_T14B)]);
}

/// Unscaled, 4100 characters from (10,60).
#[test]
fn v30_probe_t14c_unscaled_4100_chars() {
    probe_big("T14C", &[], 8, 0, &big_text(4100), &[10, 60], &[((0, 40, 640, 30), SP_T14C)]);
}

/// Exact double, 20 characters from x −60: the leading glyphs end the
/// previous rows.
#[test]
fn v30_probe_t14g_doubled_from_x_minus_60() {
    const T: &[i16] = &[65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84];
    probe("T14G", &[HEIGHT_26, Call { opcode: 8, sub: 0, intin: T, ptsin: &[-60, 60] }], &[((0, 30, 640, 40), SP_T14G)]);
}

/// Exact double, `v_justified` of 4096 characters (every seventh a space),
/// interchar, `max_x` −32768: an advance of 8 from (10,200).
#[test]
fn v30_probe_t14h_justified_4096_chars() {
    let mut intin = vec![0i16, 1];
    intin.extend((0..4096).map(|i| if i % 7 == 6 { b' ' as i16 } else { b'A' as i16 + (i % 26) as i16 }));
    probe_big("T14H", &[HEIGHT_26], 11, 10, &intin, &[10, 200, -32768, 0], &[((0, 170, 640, 40), SP_T14H)]);
}

/// Exact double, 4095 characters from (42,200), the last one code 14 at
/// x 10 — the control of T14K (V31) without its `XDDA` drift.
#[test]
fn v30_probe_t14l_doubled_4095_chars() {
    let mut intin = big_text(4095);
    intin[4094] = 14;
    probe_big("T14L", &[HEIGHT_26], 8, 0, &intin, &[42, 200], &[((0, 160, 64, 50), SP_T14L)]);
}

// ── TV: the same behaviour on the paths TU and T14 leave out ──────────────

const ROT_900: Call = Call { opcode: 13, sub: 0, intin: &[900], ptsin: &[] };
const ROT_1800: Call = Call { opcode: 13, sub: 0, intin: &[1800], ptsin: &[] };
const ROT_2700: Call = Call { opcode: 13, sub: 0, intin: &[2700], ptsin: &[] };
const CLIP_TVE: Call = Call { opcode: 129, sub: 0, intin: &[1], ptsin: &[90, 280, 120, 295] };
const OUTLINE_UNDERLINE: Call = Call { opcode: 106, sub: 0, intin: &[8 | 16], ptsin: &[] };
const HEIGHT_20: Call = Call { opcode: 12, sub: 0, intin: &[], ptsin: &[0, 20] };
const HEIGHT_13: Call = Call { opcode: 12, sub: 0, intin: &[], ptsin: &[0, 13] };
const MODE_TRANS: Call = Call { opcode: 32, sub: 0, intin: &[2], ptsin: &[] };
const MODE_XOR: Call = Call { opcode: 32, sub: 0, intin: &[3], ptsin: &[] };
const MODE_REV: Call = Call { opcode: 32, sub: 0, intin: &[4], ptsin: &[] };
const ALIGN_RIGHT: Call = Call { opcode: 39, sub: 0, intin: &[2, 0], ptsin: &[] };
const LINE_10_20: Call = Call { opcode: 6, sub: 0, intin: &[], ptsin: &[10, 10, 20, 10] };

/// `v_justified` of 'AB CD EF' with `flags` = interword, interchar.
const fn justified(iw_ic: &'static [i16], at: &'static [i16]) -> Call {
    Call { opcode: 11, sub: 10, intin: iw_ic, ptsin: at }
}

const JUST_AB_CD_EF: &[i16] = &[1, 0, 65, 66, 32, 67, 68, 32, 69, 70];
const JUST_ABCDEFGH: &[i16] = &[0, 1, 65, 66, 67, 68, 69, 70, 71, 72];

/// Rotation 900, inside the screen: the vertical underline up to `DESTY`.
#[test]
fn v30_probe_tva_rot900_inside() {
    probe("TVA", &[UNDERLINE, ROT_900, gtext(&[100, 300])], &[((70, 200, 60, 120), SP_TVA)]);
}

/// Rotation 900, clip off, past x 639: cells and underline run on into the
/// next rows.
#[test]
fn v30_probe_tvb_rot900_runs_on_past_x_639() {
    probe(
        "TVB",
        &[UNDERLINE, ROT_900, gtext(&[642, 300])],
        &[((600, 200, 40, 120), SP_TVB), ((0, 200, 40, 120), SP_TVBW)],
    );
}

/// Rotation 1800, clip off, left of x 0: the horizontal underline and the
/// cells run on into the previous rows.
#[test]
fn v30_probe_tvc_rot1800_runs_on_left_of_x_0() {
    probe(
        "TVC",
        &[UNDERLINE, ROT_1800, gtext(&[20, 200])],
        &[((0, 170, 60, 50), SP_TVC), ((590, 170, 50, 50), SP_TVCW)],
    );
}

/// Rotation 2700, clip off, left of x 0.
#[test]
fn v30_probe_tvd_rot2700_runs_on_left_of_x_0() {
    probe(
        "TVD",
        &[UNDERLINE, ROT_2700, gtext(&[-2, 200])],
        &[((0, 180, 40, 80), SP_TVD), ((600, 180, 40, 80), SP_TVDW)],
    );
}

/// The control: rotation 900, clip on, cut by `clip_line` and `do_clip`.
#[test]
fn v30_probe_tve_rot900_clip_on_control() {
    probe("TVE", &[UNDERLINE, ROT_900, CLIP_TVE, gtext(&[100, 300])], &[((70, 200, 60, 120), SP_TVE)]);
}

/// Exact double, light, rotation 900: the two underline lines, `LN_MASK`
/// carried by `abline` and `rorw1` from one to the next. Only the columns
/// of the underline: the glyphs of a light run are the separate finding
/// described at [`v30_probe_tue_light_underline_runs_on`].
#[test]
fn v30_probe_tvf_rot900_light_underline_mask() {
    probe("TVF", &[UNDERLINE_LIGHT, HEIGHT_26, ROT_900, gtext(&[200, 300])], &[((201, 100, 4, 220), SP_TVF)]);
}

/// As TVF, rotation 2700.
#[test]
fn v30_probe_tvg_rot2700_light_underline_mask() {
    probe("TVG", &[UNDERLINE_LIGHT, HEIGHT_26, ROT_2700, gtext(&[300, 100])], &[((296, 80, 4, 220), SP_TVG)]);
}

/// As TVF, rotation 1800: `abline` leaves the mask of a horizontal line
/// alone, only `rorw1` turns it.
#[test]
fn v30_probe_tvh_rot1800_light_underline_mask() {
    probe("TVH", &[UNDERLINE_LIGHT, HEIGHT_26, ROT_1800, gtext(&[500, 100])], &[((300, 96, 220, 4), SP_TVH)]);
}

/// As TVF, horizontal: the second line holds the opposite dots.
#[test]
fn v30_probe_tvi_light_underline_mask() {
    probe("TVI", &[UNDERLINE_LIGHT, HEIGHT_26, gtext(&[100, 300])], &[((80, 301, 160, 4), SP_TVI)]);
}

/// XOR after a two-point `v_pline` (`LSTLIN` TRUE): the end pixel is drawn.
#[test]
fn v30_probe_tvj_xor_underline() {
    probe("TVJ", &[UNDERLINE, MODE_XOR, LINE_10_20, gtext(&[100, 100])], &[((80, 80, 120, 40), SP_TVJ)]);
}

/// Outline: `startx` and `DESTX` carry the outline's shift.
#[test]
fn v30_probe_tvl_outline_underline() {
    probe("TVL", &[OUTLINE_UNDERLINE, gtext(&[100, 100])], &[((80, 80, 130, 40), SP_TVL)]);
}

/// `v_justified`: the underline ends on the justified pen.
#[test]
fn v30_probe_tvm_justified_underline() {
    probe(
        "TVM",
        &[UNDERLINE, justified(JUST_AB_CD_EF, &[100, 100, 200, 0])],
        &[((80, 80, 240, 40), SP_TVM)],
    );
}

/// `v_justified` with outline: its underline carries the outline's shift
/// in both axes too.
#[test]
fn v30_probe_tvn_justified_outline_underline() {
    probe(
        "TVN",
        &[OUTLINE_UNDERLINE, justified(JUST_AB_CD_EF, &[100, 100, 200, 0])],
        &[((80, 80, 240, 40), SP_TVN)],
    );
}

/// `v_justified` squeezed to length 10: the pen ends near the start.
#[test]
fn v30_probe_tvq_justified_squeezed_underline() {
    probe(
        "TVQ",
        &[UNDERLINE, justified(JUST_ABCDEFGH, &[100, 100, 10, 0])],
        &[((60, 80, 120, 40), SP_TVQ)],
    );
}

/// Clip off, transparent: run-on without the cell background.
#[test]
fn v30_probe_tvr_transparent_runs_on() {
    probe(
        "TVR",
        &[UNDERLINE, MODE_TRANS, gtext(&[600, 100])],
        &[((560, 80, 80, 40), SP_TVR), ((0, 80, 80, 40), SP_TVRW)],
    );
}

/// Clip off, XOR.
#[test]
fn v30_probe_tvs_xor_runs_on() {
    probe(
        "TVS",
        &[UNDERLINE, MODE_XOR, LINE_10_20, gtext(&[600, 100])],
        &[((560, 80, 80, 40), SP_TVS), ((0, 80, 80, 40), SP_TVSW)],
    );
}

/// Clip off, reverse transparent.
#[test]
fn v30_probe_tvt_rev_transparent_runs_on() {
    probe(
        "TVT",
        &[UNDERLINE, MODE_REV, gtext(&[600, 100])],
        &[((560, 80, 80, 40), SP_TVT), ((0, 80, 80, 40), SP_TVTW)],
    );
}

/// Clip off, DDA-scaled (`vst_height` 20).
#[test]
fn v30_probe_tvv_scaled_runs_on() {
    probe(
        "TVV",
        &[HEIGHT_20, gtext(&[600, 100])],
        &[((560, 60, 80, 60), SP_TVV), ((0, 60, 80, 60), SP_TVVW)],
    );
}

/// Clip off, right-aligned from x 20: the run starts in the previous rows.
#[test]
fn v30_probe_tvw_right_aligned_runs_on() {
    probe(
        "TVW",
        &[UNDERLINE, ALIGN_RIGHT, gtext(&[20, 150])],
        &[((0, 130, 80, 40), SP_TVW), ((560, 130, 80, 40), SP_TVWW)],
    );
}

/// The 8×16 font inside the screen: EmuTOS takes its direct blit, NewTOS
/// the general path, the same pixels.
#[test]
fn v30_probe_tvx_8x16_inside() {
    probe("TVX", &[HEIGHT_13, gtext(&[100, 100])], &[((80, 80, 120, 40), SP_TVX)]);
}

/// The 8×16 font past x 639: the general path on both sides.
#[test]
fn v30_probe_tvy_8x16_runs_on() {
    probe(
        "TVY",
        &[HEIGHT_13, gtext(&[600, 100])],
        &[((560, 80, 80, 40), SP_TVY), ((0, 80, 80, 40), SP_TVYW)],
    );
}

/// `v_justified` as TVF, the underline 202 pixels long: an even count, so
/// `abline`'s rotation and `rorw1` turn the second line's dots against the
/// first. TVF's 129 pixels leave the phase where a mask that never turned
/// would have it.
#[test]
fn v30_probe_twa_rot900_light_underline_even_length() {
    const J: &[i16] = &[0, 1, 65, 66, 67, 68, 69, 70, 71, 72];
    probe(
        "TWA",
        &[UNDERLINE_LIGHT, HEIGHT_26, ROT_900, justified(J, &[200, 300, 201, 0])],
        &[((201, 90, 4, 230), SP_TWA)],
    );
}

// ── the oracle's words, decoded from probe/e1/VDPROBE.BIN.gz ──────────────

/// TUA: grab (560,80) 80x40, record 339 of probe/e1.
#[rustfmt::skip]
const SP_TUA: &[Span] = &[
    (96, 600, 600, P), (96, 601, 603, B), (96, 604, 605, P), (96, 606, 609, B), (96, 610, 612, P),
    (96, 613, 616, B), (96, 617, 617, P), (96, 618, 621, B), (96, 622, 623, P), (96, 624, 628, B),
    (96, 629, 629, P), (96, 630, 634, B), (96, 635, 636, P), (96, 637, 639, B), (97, 600, 600, B),
    (97, 601, 603, P), (97, 604, 604, B), (97, 605, 605, P), (97, 606, 606, B), (97, 607, 609, P),
    (97, 610, 610, B), (97, 611, 611, P), (97, 612, 612, B), (97, 613, 617, P), (97, 618, 618, B),
    (97, 619, 621, P), (97, 622, 622, B), (97, 623, 623, P), (97, 624, 624, B), (97, 625, 629, P),
    (97, 630, 630, B), (97, 631, 635, P), (97, 636, 636, B), (97, 637, 639, P), (98, 600, 604, B),
    (98, 605, 605, P), (98, 606, 609, B), (98, 610, 611, P), (98, 612, 612, B), (98, 613, 617, P),
    (98, 618, 618, B), (98, 619, 621, P), (98, 622, 622, B), (98, 623, 623, P), (98, 624, 627, B),
    (98, 628, 629, P), (98, 630, 633, B), (98, 634, 635, P), (98, 636, 636, B), (98, 637, 638, P),
    (98, 639, 639, B), (99, 600, 600, B), (99, 601, 603, P), (99, 604, 604, B), (99, 605, 605, P),
    (99, 606, 606, B), (99, 607, 609, P), (99, 610, 610, B), (99, 611, 611, P), (99, 612, 612, B),
    (99, 613, 617, P), (99, 618, 618, B), (99, 619, 621, P), (99, 622, 622, B), (99, 623, 623, P),
    (99, 624, 624, B), (99, 625, 629, P), (99, 630, 630, B), (99, 631, 635, P), (99, 636, 636, B),
    (99, 637, 639, P), (100, 600, 600, B), (100, 601, 603, P), (100, 604, 604, B), (100, 605, 605, P),
    (100, 606, 609, B), (100, 610, 612, P), (100, 613, 616, B), (100, 617, 617, P), (100, 618, 621, B),
    (100, 622, 623, P), (100, 624, 628, B), (100, 629, 629, P), (100, 630, 630, B), (100, 631, 636, P),
    (100, 637, 639, B), (101, 600, 639, B),
];
/// TUAw: grab (0,80) 80x40, record 339 of probe/e1.
#[rustfmt::skip]
const SP_TUAW: &[Span] = &[
    (97, 0, 0, B), (97, 1, 1, P), (97, 2, 2, B), (97, 3, 5, P), (97, 6, 6, B), (97, 7, 7, P), (98, 0, 1, P),
    (98, 2, 2, B), (98, 3, 5, P), (98, 6, 6, B), (98, 7, 7, P), (99, 0, 0, B), (99, 1, 1, P), (99, 2, 6, B),
    (99, 7, 7, P), (100, 0, 0, B), (100, 1, 1, P), (100, 2, 2, B), (100, 3, 5, P), (100, 6, 6, B),
    (100, 7, 7, P), (101, 0, 0, B), (101, 1, 1, P), (101, 2, 2, B), (101, 3, 5, P), (101, 6, 6, B),
    (101, 7, 7, P), (102, 0, 8, B),
];
/// TUB: grab (0,130) 80x40, record 340 of probe/e1.
#[rustfmt::skip]
const SP_TUB: &[Span] = &[
    (146, 0, 4, B), (146, 5, 6, P), (146, 7, 10, B), (146, 11, 11, P), (146, 12, 12, B), (146, 13, 15, P),
    (146, 16, 16, B), (146, 17, 17, P), (147, 0, 0, B), (147, 1, 5, P), (147, 6, 6, B), (147, 7, 11, P),
    (147, 12, 12, B), (147, 13, 15, P), (147, 16, 16, B), (147, 17, 17, P), (148, 0, 3, B), (148, 4, 5, P),
    (148, 6, 6, B), (148, 7, 8, P), (148, 9, 10, B), (148, 11, 11, P), (148, 12, 16, B), (148, 17, 17, P),
    (149, 0, 0, B), (149, 1, 5, P), (149, 6, 6, B), (149, 7, 9, P), (149, 10, 10, B), (149, 11, 11, P),
    (149, 12, 12, B), (149, 13, 15, P), (149, 16, 16, B), (149, 17, 17, P), (150, 0, 0, B), (150, 1, 6, P),
    (150, 7, 10, B), (150, 11, 11, P), (150, 12, 12, B), (150, 13, 15, P), (150, 16, 16, B), (150, 17, 17, P),
    (151, 0, 18, B),
];
/// TUBw: grab (600,130) 40x40, record 340 of probe/e1.
#[rustfmt::skip]
const SP_TUBW: &[Span] = &[
    (145, 610, 610, P), (145, 611, 613, B), (145, 614, 615, P), (145, 616, 619, B), (145, 620, 622, P),
    (145, 623, 626, B), (145, 627, 627, P), (145, 628, 631, B), (145, 632, 633, P), (145, 634, 638, B),
    (145, 639, 639, P), (146, 610, 610, B), (146, 611, 613, P), (146, 614, 614, B), (146, 615, 615, P),
    (146, 616, 616, B), (146, 617, 619, P), (146, 620, 620, B), (146, 621, 621, P), (146, 622, 622, B),
    (146, 623, 627, P), (146, 628, 628, B), (146, 629, 631, P), (146, 632, 632, B), (146, 633, 633, P),
    (146, 634, 634, B), (146, 635, 639, P), (147, 610, 614, B), (147, 615, 615, P), (147, 616, 619, B),
    (147, 620, 621, P), (147, 622, 622, B), (147, 623, 627, P), (147, 628, 628, B), (147, 629, 631, P),
    (147, 632, 632, B), (147, 633, 633, P), (147, 634, 637, B), (147, 638, 639, P), (148, 610, 610, B),
    (148, 611, 613, P), (148, 614, 614, B), (148, 615, 615, P), (148, 616, 616, B), (148, 617, 619, P),
    (148, 620, 620, B), (148, 621, 621, P), (148, 622, 622, B), (148, 623, 627, P), (148, 628, 628, B),
    (148, 629, 631, P), (148, 632, 632, B), (148, 633, 633, P), (148, 634, 634, B), (148, 635, 639, P),
    (149, 610, 610, B), (149, 611, 613, P), (149, 614, 614, B), (149, 615, 615, P), (149, 616, 619, B),
    (149, 620, 622, P), (149, 623, 626, B), (149, 627, 627, P), (149, 628, 631, B), (149, 632, 633, P),
    (149, 634, 638, B), (149, 639, 639, P), (150, 610, 639, B),
];
/// TUC: grab (520,160) 120x60, record 341 of probe/e1.
#[rustfmt::skip]
const SP_TUC: &[Span] = &[
    (173, 560, 639, P), (174, 560, 639, P), (175, 560, 639, P), (176, 560, 639, P), (177, 560, 565, P),
    (177, 566, 569, B), (177, 570, 577, P), (177, 578, 587, B), (177, 588, 595, P), (177, 596, 603, B),
    (177, 604, 609, P), (177, 610, 617, B), (177, 618, 625, P), (177, 626, 637, B), (177, 638, 639, P),
    (178, 560, 565, P), (178, 566, 569, B), (178, 570, 577, P), (178, 578, 587, B), (178, 588, 595, P),
    (178, 596, 603, B), (178, 604, 609, P), (178, 610, 617, B), (178, 618, 625, P), (178, 626, 637, B),
    (178, 638, 639, P), (179, 560, 563, P), (179, 564, 571, B), (179, 572, 577, P), (179, 578, 589, B),
    (179, 590, 593, P), (179, 594, 605, B), (179, 606, 609, P), (179, 610, 619, B), (179, 620, 625, P),
    (179, 626, 637, B), (179, 638, 639, P), (180, 560, 563, P), (180, 564, 571, B), (180, 572, 577, P),
    (180, 578, 589, B), (180, 590, 593, P), (180, 594, 605, B), (180, 606, 609, P), (180, 610, 619, B),
    (180, 620, 625, P), (180, 626, 637, B), (180, 638, 639, P), (181, 560, 561, P), (181, 562, 573, B),
    (181, 574, 577, P), (181, 578, 581, B), (181, 582, 585, P), (181, 586, 589, B), (181, 590, 593, P),
    (181, 594, 597, B), (181, 598, 601, P), (181, 602, 605, B), (181, 606, 609, P), (181, 610, 613, B),
    (181, 614, 615, P), (181, 616, 621, B), (181, 622, 625, P), (181, 626, 629, B), (181, 630, 639, P),
    (182, 560, 561, P), (182, 562, 573, B), (182, 574, 577, P), (182, 578, 581, B), (182, 582, 585, P),
    (182, 586, 589, B), (182, 590, 593, P), (182, 594, 597, B), (182, 598, 601, P), (182, 602, 605, B),
    (182, 606, 609, P), (182, 610, 613, B), (182, 614, 615, P), (182, 616, 621, B), (182, 622, 625, P),
    (182, 626, 629, B), (182, 630, 639, P), (183, 560, 561, P), (183, 562, 565, B), (183, 566, 569, P),
    (183, 570, 573, B), (183, 574, 577, P), (183, 578, 581, B), (183, 582, 585, P), (183, 586, 589, B),
    (183, 590, 593, P), (183, 594, 597, B), (183, 598, 601, P), (183, 602, 605, B), (183, 606, 609, P),
    (183, 610, 613, B), (183, 614, 617, P), (183, 618, 621, B), (183, 622, 625, P), (183, 626, 629, B),
    (183, 630, 639, P), (184, 560, 561, P), (184, 562, 565, B), (184, 566, 569, P), (184, 570, 573, B),
    (184, 574, 577, P), (184, 578, 581, B), (184, 582, 585, P), (184, 586, 589, B), (184, 590, 593, P),
    (184, 594, 597, B), (184, 598, 601, P), (184, 602, 605, B), (184, 606, 609, P), (184, 610, 613, B),
    (184, 614, 617, P), (184, 618, 621, B), (184, 622, 625, P), (184, 626, 629, B), (184, 630, 639, P),
    (185, 560, 561, P), (185, 562, 565, B), (185, 566, 569, P), (185, 570, 573, B), (185, 574, 577, P),
    (185, 578, 589, B), (185, 590, 593, P), (185, 594, 597, B), (185, 598, 609, P), (185, 610, 613, B),
    (185, 614, 617, P), (185, 618, 621, B), (185, 622, 625, P), (185, 626, 635, B), (185, 636, 639, P),
    (186, 560, 561, P), (186, 562, 565, B), (186, 566, 569, P), (186, 570, 573, B), (186, 574, 577, P),
    (186, 578, 589, B), (186, 590, 593, P), (186, 594, 597, B), (186, 598, 609, P), (186, 610, 613, B),
    (186, 614, 617, P), (186, 618, 621, B), (186, 622, 625, P), (186, 626, 635, B), (186, 636, 639, P),
    (187, 560, 561, P), (187, 562, 565, B), (187, 566, 569, P), (187, 570, 573, B), (187, 574, 577, P),
    (187, 578, 587, B), (187, 588, 593, P), (187, 594, 597, B), (187, 598, 609, P), (187, 610, 613, B),
    (187, 614, 617, P), (187, 618, 621, B), (187, 622, 625, P), (187, 626, 635, B), (187, 636, 639, P),
    (188, 560, 561, P), (188, 562, 565, B), (188, 566, 569, P), (188, 570, 573, B), (188, 574, 577, P),
    (188, 578, 587, B), (188, 588, 593, P), (188, 594, 597, B), (188, 598, 609, P), (188, 610, 613, B),
    (188, 614, 617, P), (188, 618, 621, B), (188, 622, 625, P), (188, 626, 635, B), (188, 636, 639, P),
    (189, 560, 561, P), (189, 562, 573, B), (189, 574, 577, P), (189, 578, 581, B), (189, 582, 585, P),
    (189, 586, 589, B), (189, 590, 593, P), (189, 594, 597, B), (189, 598, 609, P), (189, 610, 613, B),
    (189, 614, 617, P), (189, 618, 621, B), (189, 622, 625, P), (189, 626, 629, B), (189, 630, 639, P),
    (190, 560, 561, P), (190, 562, 573, B), (190, 574, 577, P), (190, 578, 581, B), (190, 582, 585, P),
    (190, 586, 589, B), (190, 590, 593, P), (190, 594, 597, B), (190, 598, 609, P), (190, 610, 613, B),
    (190, 614, 617, P), (190, 618, 621, B), (190, 622, 625, P), (190, 626, 629, B), (190, 630, 639, P),
    (191, 560, 561, P), (191, 562, 573, B), (191, 574, 577, P), (191, 578, 581, B), (191, 582, 585, P),
    (191, 586, 589, B), (191, 590, 593, P), (191, 594, 597, B), (191, 598, 609, P), (191, 610, 613, B),
    (191, 614, 617, P), (191, 618, 621, B), (191, 622, 625, P), (191, 626, 629, B), (191, 630, 639, P),
    (192, 560, 561, P), (192, 562, 573, B), (192, 574, 577, P), (192, 578, 581, B), (192, 582, 585, P),
    (192, 586, 589, B), (192, 590, 593, P), (192, 594, 597, B), (192, 598, 609, P), (192, 610, 613, B),
    (192, 614, 617, P), (192, 618, 621, B), (192, 622, 625, P), (192, 626, 629, B), (192, 630, 639, P),
    (193, 560, 561, P), (193, 562, 565, B), (193, 566, 569, P), (193, 570, 573, B), (193, 574, 577, P),
    (193, 578, 581, B), (193, 582, 585, P), (193, 586, 589, B), (193, 590, 593, P), (193, 594, 597, B),
    (193, 598, 601, P), (193, 602, 605, B), (193, 606, 609, P), (193, 610, 613, B), (193, 614, 617, P),
    (193, 618, 621, B), (193, 622, 625, P), (193, 626, 629, B), (193, 630, 639, P), (194, 560, 561, P),
    (194, 562, 565, B), (194, 566, 569, P), (194, 570, 573, B), (194, 574, 577, P), (194, 578, 581, B),
    (194, 582, 585, P), (194, 586, 589, B), (194, 590, 593, P), (194, 594, 597, B), (194, 598, 601, P),
    (194, 602, 605, B), (194, 606, 609, P), (194, 610, 613, B), (194, 614, 617, P), (194, 618, 621, B),
    (194, 622, 625, P), (194, 626, 629, B), (194, 630, 639, P), (195, 560, 561, P), (195, 562, 565, B),
    (195, 566, 569, P), (195, 570, 573, B), (195, 574, 577, P), (195, 578, 581, B), (195, 582, 585, P),
    (195, 586, 589, B), (195, 590, 593, P), (195, 594, 597, B), (195, 598, 601, P), (195, 602, 605, B),
    (195, 606, 609, P), (195, 610, 613, B), (195, 614, 615, P), (195, 616, 621, B), (195, 622, 625, P),
    (195, 626, 629, B), (195, 630, 639, P), (196, 560, 561, P), (196, 562, 565, B), (196, 566, 569, P),
    (196, 570, 573, B), (196, 574, 577, P), (196, 578, 581, B), (196, 582, 585, P), (196, 586, 589, B),
    (196, 590, 593, P), (196, 594, 597, B), (196, 598, 601, P), (196, 602, 605, B), (196, 606, 609, P),
    (196, 610, 613, B), (196, 614, 615, P), (196, 616, 621, B), (196, 622, 625, P), (196, 626, 629, B),
    (196, 630, 639, P), (197, 560, 561, P), (197, 562, 565, B), (197, 566, 569, P), (197, 570, 573, B),
    (197, 574, 577, P), (197, 578, 589, B), (197, 590, 593, P), (197, 594, 605, B), (197, 606, 609, P),
    (197, 610, 619, B), (197, 620, 625, P), (197, 626, 637, B), (197, 638, 639, P), (198, 560, 561, P),
    (198, 562, 565, B), (198, 566, 569, P), (198, 570, 573, B), (198, 574, 577, P), (198, 578, 589, B),
    (198, 590, 593, P), (198, 594, 605, B), (198, 606, 609, P), (198, 610, 619, B), (198, 620, 625, P),
    (198, 626, 637, B), (198, 638, 639, P), (199, 560, 561, P), (199, 562, 565, B), (199, 566, 569, P),
    (199, 570, 573, B), (199, 574, 577, P), (199, 578, 587, B), (199, 588, 595, P), (199, 596, 603, B),
    (199, 604, 609, P), (199, 610, 617, B), (199, 618, 625, P), (199, 626, 637, B), (199, 638, 639, P),
    (200, 560, 561, P), (200, 562, 565, B), (200, 566, 569, P), (200, 570, 573, B), (200, 574, 577, P),
    (200, 578, 587, B), (200, 588, 595, P), (200, 596, 603, B), (200, 604, 609, P), (200, 610, 617, B),
    (200, 618, 625, P), (200, 626, 637, B), (200, 638, 639, P), (201, 560, 639, P), (202, 560, 639, P),
    (203, 560, 639, B), (204, 560, 639, B),
];
/// TUCw: grab (0,160) 120x60, record 341 of probe/e1.
#[rustfmt::skip]
const SP_TUCW: &[Span] = &[
    (174, 0, 47, P), (175, 0, 47, P), (176, 0, 47, P), (177, 0, 47, P), (178, 0, 1, P), (178, 2, 13, B),
    (178, 14, 19, P), (178, 20, 29, B), (178, 30, 33, P), (178, 34, 37, B), (178, 38, 41, P),
    (178, 42, 45, B), (178, 46, 47, P), (179, 0, 1, P), (179, 2, 13, B), (179, 14, 19, P), (179, 20, 29, B),
    (179, 30, 33, P), (179, 34, 37, B), (179, 38, 41, P), (179, 42, 45, B), (179, 46, 47, P), (180, 0, 1, P),
    (180, 2, 13, B), (180, 14, 17, P), (180, 18, 29, B), (180, 30, 33, P), (180, 34, 37, B), (180, 38, 41, P),
    (180, 42, 45, B), (180, 46, 47, P), (181, 0, 1, P), (181, 2, 13, B), (181, 14, 17, P), (181, 18, 29, B),
    (181, 30, 33, P), (181, 34, 37, B), (181, 38, 41, P), (181, 42, 45, B), (181, 46, 47, P), (182, 0, 1, P),
    (182, 2, 5, B), (182, 6, 17, P), (182, 18, 21, B), (182, 22, 33, P), (182, 34, 37, B), (182, 38, 41, P),
    (182, 42, 45, B), (182, 46, 47, P), (183, 0, 1, P), (183, 2, 5, B), (183, 6, 17, P), (183, 18, 21, B),
    (183, 22, 33, P), (183, 34, 37, B), (183, 38, 41, P), (183, 42, 45, B), (183, 46, 47, P), (184, 0, 1, P),
    (184, 2, 5, B), (184, 6, 17, P), (184, 18, 21, B), (184, 22, 33, P), (184, 34, 37, B), (184, 38, 41, P),
    (184, 42, 45, B), (184, 46, 47, P), (185, 0, 1, P), (185, 2, 5, B), (185, 6, 17, P), (185, 18, 21, B),
    (185, 22, 33, P), (185, 34, 37, B), (185, 38, 41, P), (185, 42, 45, B), (185, 46, 47, P), (186, 0, 1, P),
    (186, 2, 11, B), (186, 12, 17, P), (186, 18, 21, B), (186, 22, 23, P), (186, 24, 29, B), (186, 30, 33, P),
    (186, 34, 45, B), (186, 46, 47, P), (187, 0, 1, P), (187, 2, 11, B), (187, 12, 17, P), (187, 18, 21, B),
    (187, 22, 23, P), (187, 24, 29, B), (187, 30, 33, P), (187, 34, 45, B), (187, 46, 47, P), (188, 0, 1, P),
    (188, 2, 11, B), (188, 12, 17, P), (188, 18, 21, B), (188, 22, 23, P), (188, 24, 29, B), (188, 30, 33, P),
    (188, 34, 45, B), (188, 46, 47, P), (189, 0, 1, P), (189, 2, 11, B), (189, 12, 17, P), (189, 18, 21, B),
    (189, 22, 23, P), (189, 24, 29, B), (189, 30, 33, P), (189, 34, 45, B), (189, 46, 47, P), (190, 0, 1, P),
    (190, 2, 5, B), (190, 6, 17, P), (190, 18, 21, B), (190, 22, 25, P), (190, 26, 29, B), (190, 30, 33, P),
    (190, 34, 37, B), (190, 38, 41, P), (190, 42, 45, B), (190, 46, 47, P), (191, 0, 1, P), (191, 2, 5, B),
    (191, 6, 17, P), (191, 18, 21, B), (191, 22, 25, P), (191, 26, 29, B), (191, 30, 33, P), (191, 34, 37, B),
    (191, 38, 41, P), (191, 42, 45, B), (191, 46, 47, P), (192, 0, 1, P), (192, 2, 5, B), (192, 6, 17, P),
    (192, 18, 21, B), (192, 22, 25, P), (192, 26, 29, B), (192, 30, 33, P), (192, 34, 37, B),
    (192, 38, 41, P), (192, 42, 45, B), (192, 46, 47, P), (193, 0, 1, P), (193, 2, 5, B), (193, 6, 17, P),
    (193, 18, 21, B), (193, 22, 25, P), (193, 26, 29, B), (193, 30, 33, P), (193, 34, 37, B),
    (193, 38, 41, P), (193, 42, 45, B), (193, 46, 47, P), (194, 0, 1, P), (194, 2, 5, B), (194, 6, 17, P),
    (194, 18, 21, B), (194, 22, 25, P), (194, 26, 29, B), (194, 30, 33, P), (194, 34, 37, B),
    (194, 38, 41, P), (194, 42, 45, B), (194, 46, 47, P), (195, 0, 1, P), (195, 2, 5, B), (195, 6, 17, P),
    (195, 18, 21, B), (195, 22, 25, P), (195, 26, 29, B), (195, 30, 33, P), (195, 34, 37, B),
    (195, 38, 41, P), (195, 42, 45, B), (195, 46, 47, P), (196, 0, 1, P), (196, 2, 5, B), (196, 6, 17, P),
    (196, 18, 21, B), (196, 22, 25, P), (196, 26, 29, B), (196, 30, 33, P), (196, 34, 37, B),
    (196, 38, 41, P), (196, 42, 45, B), (196, 46, 47, P), (197, 0, 1, P), (197, 2, 5, B), (197, 6, 17, P),
    (197, 18, 21, B), (197, 22, 25, P), (197, 26, 29, B), (197, 30, 33, P), (197, 34, 37, B),
    (197, 38, 41, P), (197, 42, 45, B), (197, 46, 47, P), (198, 0, 1, P), (198, 2, 5, B), (198, 6, 17, P),
    (198, 18, 29, B), (198, 30, 33, P), (198, 34, 37, B), (198, 38, 41, P), (198, 42, 45, B),
    (198, 46, 47, P), (199, 0, 1, P), (199, 2, 5, B), (199, 6, 17, P), (199, 18, 29, B), (199, 30, 33, P),
    (199, 34, 37, B), (199, 38, 41, P), (199, 42, 45, B), (199, 46, 47, P), (200, 0, 1, P), (200, 2, 5, B),
    (200, 6, 19, P), (200, 20, 27, B), (200, 28, 33, P), (200, 34, 37, B), (200, 38, 41, P), (200, 42, 45, B),
    (200, 46, 47, P), (201, 0, 1, P), (201, 2, 5, B), (201, 6, 19, P), (201, 20, 27, B), (201, 28, 33, P),
    (201, 34, 37, B), (201, 38, 41, P), (201, 42, 45, B), (201, 46, 47, P), (202, 0, 47, P), (203, 0, 47, P),
    (204, 0, 48, B), (205, 0, 48, B),
];
/// TUD: grab (560,80) 80x40, record 342 of probe/e1.
#[rustfmt::skip]
const SP_TUD: &[Span] = &[
    (96, 600, 600, P), (96, 601, 603, B), (96, 604, 605, P), (96, 606, 609, B), (96, 610, 612, P),
    (96, 613, 616, B), (96, 617, 617, P), (96, 618, 621, B), (96, 622, 623, P), (96, 624, 628, B),
    (96, 629, 629, P), (96, 630, 634, B), (96, 635, 636, P), (96, 637, 639, B), (97, 600, 600, B),
    (97, 601, 603, P), (97, 604, 604, B), (97, 605, 605, P), (97, 606, 606, B), (97, 607, 609, P),
    (97, 610, 610, B), (97, 611, 611, P), (97, 612, 612, B), (97, 613, 617, P), (97, 618, 618, B),
    (97, 619, 621, P), (97, 622, 622, B), (97, 623, 623, P), (97, 624, 624, B), (97, 625, 629, P),
    (97, 630, 630, B), (97, 631, 635, P), (97, 636, 636, B), (97, 637, 639, P), (98, 600, 604, B),
    (98, 605, 605, P), (98, 606, 609, B), (98, 610, 611, P), (98, 612, 612, B), (98, 613, 617, P),
    (98, 618, 618, B), (98, 619, 621, P), (98, 622, 622, B), (98, 623, 623, P), (98, 624, 627, B),
    (98, 628, 629, P), (98, 630, 633, B), (98, 634, 635, P), (98, 636, 636, B), (98, 637, 638, P),
    (98, 639, 639, B), (99, 600, 600, B), (99, 601, 603, P), (99, 604, 604, B), (99, 605, 605, P),
    (99, 606, 606, B), (99, 607, 609, P), (99, 610, 610, B), (99, 611, 611, P), (99, 612, 612, B),
    (99, 613, 617, P), (99, 618, 618, B), (99, 619, 621, P), (99, 622, 622, B), (99, 623, 623, P),
    (99, 624, 624, B), (99, 625, 629, P), (99, 630, 630, B), (99, 631, 635, P), (99, 636, 636, B),
    (99, 637, 639, P), (100, 600, 600, B), (100, 601, 603, P), (100, 604, 604, B), (100, 605, 605, P),
    (100, 606, 609, B), (100, 610, 612, P), (100, 613, 616, B), (100, 617, 617, P), (100, 618, 621, B),
    (100, 622, 623, P), (100, 624, 628, B), (100, 629, 629, P), (100, 630, 630, B), (100, 631, 636, P),
    (100, 637, 639, B), (101, 600, 639, B),
];
/// TUDw: grab (0,80) 80x40, record 342 of probe/e1.
#[rustfmt::skip]
const SP_TUDW: &[Span] = &[
];
/// TUE: grab (560,280) 80x40, record 343 of probe/e1.
#[rustfmt::skip]
const SP_TUE: &[Span] = &[
    (296, 600, 600, P), (296, 601, 603, B), (296, 604, 605, P), (296, 606, 609, B), (296, 610, 612, P),
    (296, 613, 616, B), (296, 617, 617, P), (296, 618, 621, B), (296, 622, 623, P), (296, 624, 628, B),
    (296, 629, 629, P), (296, 630, 634, B), (296, 635, 636, P), (296, 637, 639, B), (297, 600, 600, B),
    (297, 601, 603, P), (297, 604, 604, B), (297, 605, 605, P), (297, 606, 606, B), (297, 607, 609, P),
    (297, 610, 610, B), (297, 611, 611, P), (297, 612, 612, B), (297, 613, 617, P), (297, 618, 618, B),
    (297, 619, 621, P), (297, 622, 622, B), (297, 623, 623, P), (297, 624, 624, B), (297, 625, 629, P),
    (297, 630, 630, B), (297, 631, 635, P), (297, 636, 636, B), (297, 637, 639, P), (298, 600, 604, B),
    (298, 605, 605, P), (298, 606, 609, B), (298, 610, 611, P), (298, 612, 612, B), (298, 613, 617, P),
    (298, 618, 618, B), (298, 619, 621, P), (298, 622, 622, B), (298, 623, 623, P), (298, 624, 627, B),
    (298, 628, 629, P), (298, 630, 633, B), (298, 634, 635, P), (298, 636, 636, B), (298, 637, 638, P),
    (298, 639, 639, B), (299, 600, 600, B), (299, 601, 603, P), (299, 604, 604, B), (299, 605, 605, P),
    (299, 606, 606, B), (299, 607, 609, P), (299, 610, 610, B), (299, 611, 611, P), (299, 612, 612, B),
    (299, 613, 617, P), (299, 618, 618, B), (299, 619, 621, P), (299, 622, 622, B), (299, 623, 623, P),
    (299, 624, 624, B), (299, 625, 629, P), (299, 630, 630, B), (299, 631, 635, P), (299, 636, 636, B),
    (299, 637, 639, P), (300, 600, 600, B), (300, 601, 603, P), (300, 604, 604, B), (300, 605, 605, P),
    (300, 606, 609, B), (300, 610, 612, P), (300, 613, 616, B), (300, 617, 617, P), (300, 618, 621, B),
    (300, 622, 623, P), (300, 624, 628, B), (300, 629, 629, P), (300, 630, 630, B), (300, 631, 636, P),
    (300, 637, 639, B), (301, 600, 600, P), (301, 601, 601, B), (301, 602, 602, P), (301, 603, 603, B),
    (301, 604, 604, P), (301, 605, 605, B), (301, 606, 606, P), (301, 607, 607, B), (301, 608, 608, P),
    (301, 609, 609, B), (301, 610, 610, P), (301, 611, 611, B), (301, 612, 612, P), (301, 613, 613, B),
    (301, 614, 614, P), (301, 615, 615, B), (301, 616, 616, P), (301, 617, 617, B), (301, 618, 618, P),
    (301, 619, 619, B), (301, 620, 620, P), (301, 621, 621, B), (301, 622, 622, P), (301, 623, 623, B),
    (301, 624, 624, P), (301, 625, 625, B), (301, 626, 626, P), (301, 627, 627, B), (301, 628, 628, P),
    (301, 629, 629, B), (301, 630, 630, P), (301, 631, 631, B), (301, 632, 632, P), (301, 633, 633, B),
    (301, 634, 634, P), (301, 635, 635, B), (301, 636, 636, P), (301, 637, 637, B), (301, 638, 638, P),
    (301, 639, 639, B),
];
/// TUEw: grab (0,280) 80x40, record 343 of probe/e1.
#[rustfmt::skip]
const SP_TUEW: &[Span] = &[
    (297, 0, 0, B), (297, 1, 1, P), (297, 2, 2, B), (297, 3, 5, P), (297, 6, 6, B), (297, 7, 7, P),
    (298, 0, 1, P), (298, 2, 2, B), (298, 3, 5, P), (298, 6, 6, B), (298, 7, 7, P), (299, 0, 0, B),
    (299, 1, 1, P), (299, 2, 6, B), (299, 7, 7, P), (300, 0, 0, B), (300, 1, 1, P), (300, 2, 2, B),
    (300, 3, 5, P), (300, 6, 6, B), (300, 7, 7, P), (301, 0, 0, B), (301, 1, 1, P), (301, 2, 2, B),
    (301, 3, 5, P), (301, 6, 6, B), (301, 7, 7, P), (302, 0, 0, P), (302, 1, 1, B), (302, 2, 2, P),
    (302, 3, 3, B), (302, 4, 4, P), (302, 5, 5, B), (302, 6, 6, P), (302, 7, 7, B), (302, 8, 8, P),
];
/// TUF: grab (80,80) 120x40, record 344 of probe/e1.
#[rustfmt::skip]
const SP_TUF: &[Span] = &[
    (96, 100, 100, P), (96, 101, 103, B), (96, 104, 105, P), (96, 106, 109, B), (96, 110, 112, P),
    (96, 113, 116, B), (96, 117, 117, P), (96, 118, 121, B), (96, 122, 123, P), (96, 124, 128, B),
    (96, 129, 129, P), (96, 130, 134, B), (96, 135, 136, P), (96, 137, 140, B), (96, 141, 141, P),
    (96, 142, 142, B), (96, 143, 145, P), (96, 146, 146, B), (96, 147, 147, P), (97, 100, 100, B),
    (97, 101, 103, P), (97, 104, 104, B), (97, 105, 105, P), (97, 106, 106, B), (97, 107, 109, P),
    (97, 110, 110, B), (97, 111, 111, P), (97, 112, 112, B), (97, 113, 117, P), (97, 118, 118, B),
    (97, 119, 121, P), (97, 122, 122, B), (97, 123, 123, P), (97, 124, 124, B), (97, 125, 129, P),
    (97, 130, 130, B), (97, 131, 135, P), (97, 136, 136, B), (97, 137, 141, P), (97, 142, 142, B),
    (97, 143, 145, P), (97, 146, 146, B), (97, 147, 147, P), (98, 100, 104, B), (98, 105, 105, P),
    (98, 106, 109, B), (98, 110, 111, P), (98, 112, 112, B), (98, 113, 117, P), (98, 118, 118, B),
    (98, 119, 121, P), (98, 122, 122, B), (98, 123, 123, P), (98, 124, 127, B), (98, 128, 129, P),
    (98, 130, 133, B), (98, 134, 135, P), (98, 136, 136, B), (98, 137, 138, P), (98, 139, 140, B),
    (98, 141, 141, P), (98, 142, 146, B), (98, 147, 147, P), (99, 100, 100, B), (99, 101, 103, P),
    (99, 104, 104, B), (99, 105, 105, P), (99, 106, 106, B), (99, 107, 109, P), (99, 110, 110, B),
    (99, 111, 111, P), (99, 112, 112, B), (99, 113, 117, P), (99, 118, 118, B), (99, 119, 121, P),
    (99, 122, 122, B), (99, 123, 123, P), (99, 124, 124, B), (99, 125, 129, P), (99, 130, 130, B),
    (99, 131, 135, P), (99, 136, 136, B), (99, 137, 139, P), (99, 140, 140, B), (99, 141, 141, P),
    (99, 142, 142, B), (99, 143, 145, P), (99, 146, 146, B), (99, 147, 147, P), (100, 100, 100, B),
    (100, 101, 103, P), (100, 104, 104, B), (100, 105, 105, P), (100, 106, 109, B), (100, 110, 112, P),
    (100, 113, 116, B), (100, 117, 117, P), (100, 118, 121, B), (100, 122, 123, P), (100, 124, 128, B),
    (100, 129, 129, P), (100, 130, 130, B), (100, 131, 136, P), (100, 137, 140, B), (100, 141, 141, P),
    (100, 142, 142, B), (100, 143, 145, P), (100, 146, 146, B), (100, 147, 147, P), (101, 100, 148, B),
];
/// T14B: grab (0,160) 640x60, record 306 of probe/e1.
#[rustfmt::skip]
const SP_T14B: &[Span] = &[
    (160, 0, 639, P), (161, 0, 639, P), (162, 0, 639, P), (163, 0, 639, P), (164, 0, 639, P),
    (165, 0, 639, P), (166, 0, 639, P), (167, 0, 639, P), (168, 0, 639, P), (169, 0, 639, P),
    (170, 0, 639, P), (171, 0, 639, P), (172, 0, 639, P), (173, 0, 639, P), (174, 0, 639, P),
    (175, 0, 639, P), (176, 0, 75, P), (176, 76, 87, B), (176, 88, 91, P), (176, 92, 103, B),
    (176, 104, 109, P), (176, 110, 119, B), (176, 120, 123, P), (176, 124, 127, B), (176, 128, 131, P),
    (176, 132, 135, B), (176, 136, 139, P), (176, 140, 151, B), (176, 152, 163, P), (176, 164, 167, B),
    (176, 168, 169, P), (176, 170, 173, B), (176, 174, 177, P), (176, 178, 181, B), (176, 182, 187, P),
    (176, 188, 191, B), (176, 192, 201, P), (176, 202, 205, B), (176, 206, 211, P), (176, 212, 215, B),
    (176, 216, 219, P), (176, 220, 223, B), (176, 224, 227, P), (176, 228, 231, B), (176, 232, 237, P),
    (176, 238, 245, B), (176, 246, 251, P), (176, 252, 261, B), (176, 262, 269, P), (176, 270, 277, B),
    (176, 278, 281, P), (176, 282, 291, B), (176, 292, 301, P), (176, 302, 311, B), (176, 312, 315, P),
    (176, 316, 327, B), (176, 328, 331, P), (176, 332, 335, B), (176, 336, 339, P), (176, 340, 343, B),
    (176, 344, 347, P), (176, 348, 351, B), (176, 352, 355, P), (176, 356, 359, B), (176, 360, 361, P),
    (176, 362, 365, B), (176, 366, 371, P), (176, 372, 375, B), (176, 376, 379, P), (176, 380, 383, B),
    (176, 384, 387, P), (176, 388, 391, B), (176, 392, 395, P), (176, 396, 399, B), (176, 400, 403, P),
    (176, 404, 407, B), (176, 408, 411, P), (176, 412, 423, B), (176, 424, 431, P), (176, 432, 435, B),
    (176, 436, 443, P), (176, 444, 453, B), (176, 454, 461, P), (176, 462, 469, B), (176, 470, 475, P),
    (176, 476, 483, B), (176, 484, 491, P), (176, 492, 503, B), (176, 504, 507, P), (176, 508, 519, B),
    (176, 520, 525, P), (176, 526, 535, B), (176, 536, 539, P), (176, 540, 543, B), (176, 544, 547, P),
    (176, 548, 551, B), (176, 552, 555, P), (176, 556, 567, B), (176, 568, 579, P), (176, 580, 583, B),
    (176, 584, 585, P), (176, 586, 589, B), (176, 590, 593, P), (176, 594, 597, B), (176, 598, 603, P),
    (176, 604, 607, B), (176, 608, 617, P), (176, 618, 621, B), (176, 622, 627, P), (176, 628, 631, B),
    (176, 632, 635, P), (176, 636, 639, B), (177, 0, 3, P), (177, 4, 7, B), (177, 8, 13, P), (177, 14, 21, B),
    (177, 22, 27, P), (177, 28, 37, B), (177, 38, 45, P), (177, 46, 53, B), (177, 54, 57, P),
    (177, 58, 67, B), (177, 68, 75, P), (177, 76, 87, B), (177, 88, 91, P), (177, 92, 103, B),
    (177, 104, 109, P), (177, 110, 119, B), (177, 120, 123, P), (177, 124, 127, B), (177, 128, 131, P),
    (177, 132, 135, B), (177, 136, 139, P), (177, 140, 151, B), (177, 152, 163, P), (177, 164, 167, B),
    (177, 168, 169, P), (177, 170, 173, B), (177, 174, 177, P), (177, 178, 181, B), (177, 182, 187, P),
    (177, 188, 191, B), (177, 192, 201, P), (177, 202, 205, B), (177, 206, 211, P), (177, 212, 215, B),
    (177, 216, 219, P), (177, 220, 223, B), (177, 224, 227, P), (177, 228, 231, B), (177, 232, 237, P),
    (177, 238, 245, B), (177, 246, 251, P), (177, 252, 261, B), (177, 262, 269, P), (177, 270, 277, B),
    (177, 278, 281, P), (177, 282, 291, B), (177, 292, 301, P), (177, 302, 311, B), (177, 312, 315, P),
    (177, 316, 327, B), (177, 328, 331, P), (177, 332, 335, B), (177, 336, 339, P), (177, 340, 343, B),
    (177, 344, 347, P), (177, 348, 351, B), (177, 352, 355, P), (177, 356, 359, B), (177, 360, 361, P),
    (177, 362, 365, B), (177, 366, 371, P), (177, 372, 375, B), (177, 376, 379, P), (177, 380, 383, B),
    (177, 384, 387, P), (177, 388, 391, B), (177, 392, 395, P), (177, 396, 399, B), (177, 400, 403, P),
    (177, 404, 407, B), (177, 408, 411, P), (177, 412, 423, B), (177, 424, 431, P), (177, 432, 435, B),
    (177, 436, 443, P), (177, 444, 453, B), (177, 454, 461, P), (177, 462, 469, B), (177, 470, 475, P),
    (177, 476, 483, B), (177, 484, 491, P), (177, 492, 503, B), (177, 504, 507, P), (177, 508, 519, B),
    (177, 520, 525, P), (177, 526, 535, B), (177, 536, 539, P), (177, 540, 543, B), (177, 544, 547, P),
    (177, 548, 551, B), (177, 552, 555, P), (177, 556, 567, B), (177, 568, 579, P), (177, 580, 583, B),
    (177, 584, 585, P), (177, 586, 589, B), (177, 590, 593, P), (177, 594, 597, B), (177, 598, 603, P),
    (177, 604, 607, B), (177, 608, 617, P), (177, 618, 621, B), (177, 622, 627, P), (177, 628, 631, B),
    (177, 632, 635, P), (177, 636, 639, B), (178, 0, 3, P), (178, 4, 7, B), (178, 8, 13, P), (178, 14, 21, B),
    (178, 22, 27, P), (178, 28, 37, B), (178, 38, 45, P), (178, 46, 53, B), (178, 54, 57, P),
    (178, 58, 67, B), (178, 68, 75, P), (178, 76, 87, B), (178, 88, 91, P), (178, 92, 103, B),
    (178, 104, 107, P), (178, 108, 119, B), (178, 120, 123, P), (178, 124, 127, B), (178, 128, 131, P),
    (178, 132, 135, B), (178, 136, 139, P), (178, 140, 151, B), (178, 152, 163, P), (178, 164, 167, B),
    (178, 168, 169, P), (178, 170, 173, B), (178, 174, 177, P), (178, 178, 181, B), (178, 182, 187, P),
    (178, 188, 191, B), (178, 192, 201, P), (178, 202, 205, B), (178, 206, 211, P), (178, 212, 215, B),
    (178, 216, 219, P), (178, 220, 223, B), (178, 224, 227, P), (178, 228, 231, B), (178, 232, 235, P),
    (178, 236, 247, B), (178, 248, 251, P), (178, 252, 263, B), (178, 264, 267, P), (178, 268, 279, B),
    (178, 280, 281, P), (178, 282, 293, B), (178, 294, 299, P), (178, 300, 311, B), (178, 312, 315, P),
    (178, 316, 327, B), (178, 328, 331, P), (178, 332, 335, B), (178, 336, 339, P), (178, 340, 343, B),
    (178, 344, 347, P), (178, 348, 351, B), (178, 352, 355, P), (178, 356, 359, B), (178, 360, 361, P),
    (178, 362, 365, B), (178, 366, 371, P), (178, 372, 375, B), (178, 376, 379, P), (178, 380, 383, B),
    (178, 384, 387, P), (178, 388, 391, B), (178, 392, 395, P), (178, 396, 399, B), (178, 400, 403, P),
    (178, 404, 407, B), (178, 408, 411, P), (178, 412, 423, B), (178, 424, 429, P), (178, 430, 437, B),
    (178, 438, 443, P), (178, 444, 455, B), (178, 456, 459, P), (178, 460, 471, B), (178, 472, 475, P),
    (178, 476, 485, B), (178, 486, 491, P), (178, 492, 503, B), (178, 504, 507, P), (178, 508, 519, B),
    (178, 520, 523, P), (178, 524, 535, B), (178, 536, 539, P), (178, 540, 543, B), (178, 544, 547, P),
    (178, 548, 551, B), (178, 552, 555, P), (178, 556, 567, B), (178, 568, 579, P), (178, 580, 583, B),
    (178, 584, 585, P), (178, 586, 589, B), (178, 590, 593, P), (178, 594, 597, B), (178, 598, 603, P),
    (178, 604, 607, B), (178, 608, 617, P), (178, 618, 621, B), (178, 622, 627, P), (178, 628, 631, B),
    (178, 632, 635, P), (178, 636, 639, B), (179, 0, 3, P), (179, 4, 7, B), (179, 8, 11, P), (179, 12, 23, B),
    (179, 24, 27, P), (179, 28, 39, B), (179, 40, 43, P), (179, 44, 55, B), (179, 56, 57, P),
    (179, 58, 69, B), (179, 70, 75, P), (179, 76, 87, B), (179, 88, 91, P), (179, 92, 103, B),
    (179, 104, 107, P), (179, 108, 119, B), (179, 120, 123, P), (179, 124, 127, B), (179, 128, 131, P),
    (179, 132, 135, B), (179, 136, 139, P), (179, 140, 151, B), (179, 152, 163, P), (179, 164, 167, B),
    (179, 168, 169, P), (179, 170, 173, B), (179, 174, 177, P), (179, 178, 181, B), (179, 182, 187, P),
    (179, 188, 191, B), (179, 192, 201, P), (179, 202, 205, B), (179, 206, 211, P), (179, 212, 215, B),
    (179, 216, 219, P), (179, 220, 223, B), (179, 224, 227, P), (179, 228, 231, B), (179, 232, 235, P),
    (179, 236, 247, B), (179, 248, 251, P), (179, 252, 263, B), (179, 264, 267, P), (179, 268, 279, B),
    (179, 280, 281, P), (179, 282, 293, B), (179, 294, 299, P), (179, 300, 311, B), (179, 312, 315, P),
    (179, 316, 327, B), (179, 328, 331, P), (179, 332, 335, B), (179, 336, 339, P), (179, 340, 343, B),
    (179, 344, 347, P), (179, 348, 351, B), (179, 352, 355, P), (179, 356, 359, B), (179, 360, 361, P),
    (179, 362, 365, B), (179, 366, 371, P), (179, 372, 375, B), (179, 376, 379, P), (179, 380, 383, B),
    (179, 384, 387, P), (179, 388, 391, B), (179, 392, 395, P), (179, 396, 399, B), (179, 400, 403, P),
    (179, 404, 407, B), (179, 408, 411, P), (179, 412, 423, B), (179, 424, 429, P), (179, 430, 437, B),
    (179, 438, 443, P), (179, 444, 455, B), (179, 456, 459, P), (179, 460, 471, B), (179, 472, 475, P),
    (179, 476, 485, B), (179, 486, 491, P), (179, 492, 503, B), (179, 504, 507, P), (179, 508, 519, B),
    (179, 520, 523, P), (179, 524, 535, B), (179, 536, 539, P), (179, 540, 543, B), (179, 544, 547, P),
    (179, 548, 551, B), (179, 552, 555, P), (179, 556, 567, B), (179, 568, 579, P), (179, 580, 583, B),
    (179, 584, 585, P), (179, 586, 589, B), (179, 590, 593, P), (179, 594, 597, B), (179, 598, 603, P),
    (179, 604, 607, B), (179, 608, 617, P), (179, 618, 621, B), (179, 622, 627, P), (179, 628, 631, B),
    (179, 632, 635, P), (179, 636, 639, B), (180, 0, 3, P), (180, 4, 7, B), (180, 8, 11, P), (180, 12, 23, B),
    (180, 24, 27, P), (180, 28, 39, B), (180, 40, 43, P), (180, 44, 55, B), (180, 56, 57, P),
    (180, 58, 69, B), (180, 70, 75, P), (180, 76, 79, B), (180, 80, 91, P), (180, 92, 95, B),
    (180, 96, 107, P), (180, 108, 111, B), (180, 112, 123, P), (180, 124, 127, B), (180, 128, 131, P),
    (180, 132, 135, B), (180, 136, 143, P), (180, 144, 147, B), (180, 148, 163, P), (180, 164, 167, B),
    (180, 168, 169, P), (180, 170, 173, B), (180, 174, 175, P), (180, 176, 179, B), (180, 180, 187, P),
    (180, 188, 191, B), (180, 192, 201, P), (180, 202, 207, B), (180, 208, 209, P), (180, 210, 215, B),
    (180, 216, 219, P), (180, 220, 223, B), (180, 224, 227, P), (180, 228, 231, B), (180, 232, 235, P),
    (180, 236, 239, B), (180, 240, 243, P), (180, 244, 247, B), (180, 248, 251, P), (180, 252, 255, B),
    (180, 256, 259, P), (180, 260, 263, B), (180, 264, 267, P), (180, 268, 271, B), (180, 272, 275, P),
    (180, 276, 279, B), (180, 280, 281, P), (180, 282, 285, B), (180, 286, 289, P), (180, 290, 293, B),
    (180, 294, 299, P), (180, 300, 303, B), (180, 304, 319, P), (180, 320, 323, B), (180, 324, 331, P),
    (180, 332, 335, B), (180, 336, 339, P), (180, 340, 343, B), (180, 344, 347, P), (180, 348, 351, B),
    (180, 352, 355, P), (180, 356, 359, B), (180, 360, 361, P), (180, 362, 365, B), (180, 366, 371, P),
    (180, 372, 375, B), (180, 376, 379, P), (180, 380, 383, B), (180, 384, 387, P), (180, 388, 391, B),
    (180, 392, 395, P), (180, 396, 399, B), (180, 400, 403, P), (180, 404, 407, B), (180, 408, 417, P),
    (180, 418, 421, B), (180, 422, 427, P), (180, 428, 439, B), (180, 440, 443, P), (180, 444, 447, B),
    (180, 448, 451, P), (180, 452, 455, B), (180, 456, 459, P), (180, 460, 463, B), (180, 464, 467, P),
    (180, 468, 471, B), (180, 472, 475, P), (180, 476, 479, B), (180, 480, 481, P), (180, 482, 487, B),
    (180, 488, 491, P), (180, 492, 495, B), (180, 496, 507, P), (180, 508, 511, B), (180, 512, 523, P),
    (180, 524, 527, B), (180, 528, 539, P), (180, 540, 543, B), (180, 544, 547, P), (180, 548, 551, B),
    (180, 552, 559, P), (180, 560, 563, B), (180, 564, 579, P), (180, 580, 583, B), (180, 584, 585, P),
    (180, 586, 589, B), (180, 590, 591, P), (180, 592, 595, B), (180, 596, 603, P), (180, 604, 607, B),
    (180, 608, 617, P), (180, 618, 623, B), (180, 624, 625, P), (180, 626, 631, B), (180, 632, 635, P),
    (180, 636, 639, B), (181, 0, 3, P), (181, 4, 7, B), (181, 8, 11, P), (181, 12, 15, B), (181, 16, 19, P),
    (181, 20, 23, B), (181, 24, 27, P), (181, 28, 31, B), (181, 32, 35, P), (181, 36, 39, B),
    (181, 40, 43, P), (181, 44, 47, B), (181, 48, 51, P), (181, 52, 55, B), (181, 56, 57, P),
    (181, 58, 61, B), (181, 62, 65, P), (181, 66, 69, B), (181, 70, 75, P), (181, 76, 79, B),
    (181, 80, 91, P), (181, 92, 95, B), (181, 96, 107, P), (181, 108, 111, B), (181, 112, 123, P),
    (181, 124, 127, B), (181, 128, 131, P), (181, 132, 135, B), (181, 136, 143, P), (181, 144, 147, B),
    (181, 148, 163, P), (181, 164, 167, B), (181, 168, 169, P), (181, 170, 173, B), (181, 174, 175, P),
    (181, 176, 179, B), (181, 180, 187, P), (181, 188, 191, B), (181, 192, 201, P), (181, 202, 207, B),
    (181, 208, 209, P), (181, 210, 215, B), (181, 216, 219, P), (181, 220, 223, B), (181, 224, 227, P),
    (181, 228, 231, B), (181, 232, 235, P), (181, 236, 239, B), (181, 240, 243, P), (181, 244, 247, B),
    (181, 248, 251, P), (181, 252, 255, B), (181, 256, 259, P), (181, 260, 263, B), (181, 264, 267, P),
    (181, 268, 271, B), (181, 272, 275, P), (181, 276, 279, B), (181, 280, 281, P), (181, 282, 285, B),
    (181, 286, 289, P), (181, 290, 293, B), (181, 294, 299, P), (181, 300, 303, B), (181, 304, 319, P),
    (181, 320, 323, B), (181, 324, 331, P), (181, 332, 335, B), (181, 336, 339, P), (181, 340, 343, B),
    (181, 344, 347, P), (181, 348, 351, B), (181, 352, 355, P), (181, 356, 359, B), (181, 360, 361, P),
    (181, 362, 365, B), (181, 366, 371, P), (181, 372, 375, B), (181, 376, 379, P), (181, 380, 383, B),
    (181, 384, 387, P), (181, 388, 391, B), (181, 392, 395, P), (181, 396, 399, B), (181, 400, 403, P),
    (181, 404, 407, B), (181, 408, 417, P), (181, 418, 421, B), (181, 422, 427, P), (181, 428, 439, B),
    (181, 440, 443, P), (181, 444, 447, B), (181, 448, 451, P), (181, 452, 455, B), (181, 456, 459, P),
    (181, 460, 463, B), (181, 464, 467, P), (181, 468, 471, B), (181, 472, 475, P), (181, 476, 479, B),
    (181, 480, 481, P), (181, 482, 487, B), (181, 488, 491, P), (181, 492, 495, B), (181, 496, 507, P),
    (181, 508, 511, B), (181, 512, 523, P), (181, 524, 527, B), (181, 528, 539, P), (181, 540, 543, B),
    (181, 544, 547, P), (181, 548, 551, B), (181, 552, 559, P), (181, 560, 563, B), (181, 564, 579, P),
    (181, 580, 583, B), (181, 584, 585, P), (181, 586, 589, B), (181, 590, 591, P), (181, 592, 595, B),
    (181, 596, 603, P), (181, 604, 607, B), (181, 608, 617, P), (181, 618, 623, B), (181, 624, 625, P),
    (181, 626, 631, B), (181, 632, 635, P), (181, 636, 639, B), (182, 0, 3, P), (182, 4, 7, B),
    (182, 8, 11, P), (182, 12, 15, B), (182, 16, 19, P), (182, 20, 23, B), (182, 24, 27, P), (182, 28, 31, B),
    (182, 32, 35, P), (182, 36, 39, B), (182, 40, 43, P), (182, 44, 47, B), (182, 48, 51, P),
    (182, 52, 55, B), (182, 56, 57, P), (182, 58, 61, B), (182, 62, 65, P), (182, 66, 69, B),
    (182, 70, 75, P), (182, 76, 79, B), (182, 80, 91, P), (182, 92, 95, B), (182, 96, 107, P),
    (182, 108, 111, B), (182, 112, 123, P), (182, 124, 127, B), (182, 128, 131, P), (182, 132, 135, B),
    (182, 136, 143, P), (182, 144, 147, B), (182, 148, 163, P), (182, 164, 167, B), (182, 168, 169, P),
    (182, 170, 173, B), (182, 174, 175, P), (182, 176, 179, B), (182, 180, 187, P), (182, 188, 191, B),
    (182, 192, 201, P), (182, 202, 207, B), (182, 208, 209, P), (182, 210, 215, B), (182, 216, 219, P),
    (182, 220, 225, B), (182, 226, 227, P), (182, 228, 231, B), (182, 232, 235, P), (182, 236, 239, B),
    (182, 240, 243, P), (182, 244, 247, B), (182, 248, 251, P), (182, 252, 255, B), (182, 256, 259, P),
    (182, 260, 263, B), (182, 264, 267, P), (182, 268, 271, B), (182, 272, 275, P), (182, 276, 279, B),
    (182, 280, 281, P), (182, 282, 285, B), (182, 286, 289, P), (182, 290, 293, B), (182, 294, 299, P),
    (182, 300, 303, B), (182, 304, 319, P), (182, 320, 323, B), (182, 324, 331, P), (182, 332, 335, B),
    (182, 336, 339, P), (182, 340, 343, B), (182, 344, 347, P), (182, 348, 351, B), (182, 352, 355, P),
    (182, 356, 359, B), (182, 360, 361, P), (182, 362, 365, B), (182, 366, 371, P), (182, 372, 375, B),
    (182, 376, 381, P), (182, 382, 389, B), (182, 390, 395, P), (182, 396, 399, B), (182, 400, 403, P),
    (182, 404, 407, B), (182, 408, 417, P), (182, 418, 421, B), (182, 422, 427, P), (182, 428, 431, B),
    (182, 432, 435, P), (182, 436, 439, B), (182, 440, 443, P), (182, 444, 447, B), (182, 448, 451, P),
    (182, 452, 455, B), (182, 456, 459, P), (182, 460, 463, B), (182, 464, 467, P), (182, 468, 471, B),
    (182, 472, 475, P), (182, 476, 479, B), (182, 480, 483, P), (182, 484, 487, B), (182, 488, 491, P),
    (182, 492, 495, B), (182, 496, 507, P), (182, 508, 511, B), (182, 512, 523, P), (182, 524, 527, B),
    (182, 528, 539, P), (182, 540, 543, B), (182, 544, 547, P), (182, 548, 551, B), (182, 552, 559, P),
    (182, 560, 563, B), (182, 564, 579, P), (182, 580, 583, B), (182, 584, 585, P), (182, 586, 589, B),
    (182, 590, 591, P), (182, 592, 595, B), (182, 596, 603, P), (182, 604, 607, B), (182, 608, 617, P),
    (182, 618, 623, B), (182, 624, 625, P), (182, 626, 631, B), (182, 632, 635, P), (182, 636, 639, B),
    (183, 0, 1, B), (183, 2, 3, P), (183, 4, 7, B), (183, 8, 11, P), (183, 12, 15, B), (183, 16, 19, P),
    (183, 20, 23, B), (183, 24, 27, P), (183, 28, 31, B), (183, 32, 35, P), (183, 36, 39, B),
    (183, 40, 43, P), (183, 44, 47, B), (183, 48, 51, P), (183, 52, 55, B), (183, 56, 57, P),
    (183, 58, 61, B), (183, 62, 65, P), (183, 66, 69, B), (183, 70, 75, P), (183, 76, 79, B),
    (183, 80, 91, P), (183, 92, 95, B), (183, 96, 107, P), (183, 108, 111, B), (183, 112, 123, P),
    (183, 124, 127, B), (183, 128, 131, P), (183, 132, 135, B), (183, 136, 143, P), (183, 144, 147, B),
    (183, 148, 163, P), (183, 164, 167, B), (183, 168, 169, P), (183, 170, 173, B), (183, 174, 175, P),
    (183, 176, 179, B), (183, 180, 187, P), (183, 188, 191, B), (183, 192, 201, P), (183, 202, 207, B),
    (183, 208, 209, P), (183, 210, 215, B), (183, 216, 219, P), (183, 220, 225, B), (183, 226, 227, P),
    (183, 228, 231, B), (183, 232, 235, P), (183, 236, 239, B), (183, 240, 243, P), (183, 244, 247, B),
    (183, 248, 251, P), (183, 252, 255, B), (183, 256, 259, P), (183, 260, 263, B), (183, 264, 267, P),
    (183, 268, 271, B), (183, 272, 275, P), (183, 276, 279, B), (183, 280, 281, P), (183, 282, 285, B),
    (183, 286, 289, P), (183, 290, 293, B), (183, 294, 299, P), (183, 300, 303, B), (183, 304, 319, P),
    (183, 320, 323, B), (183, 324, 331, P), (183, 332, 335, B), (183, 336, 339, P), (183, 340, 343, B),
    (183, 344, 347, P), (183, 348, 351, B), (183, 352, 355, P), (183, 356, 359, B), (183, 360, 361, P),
    (183, 362, 365, B), (183, 366, 371, P), (183, 372, 375, B), (183, 376, 381, P), (183, 382, 389, B),
    (183, 390, 395, P), (183, 396, 399, B), (183, 400, 403, P), (183, 404, 407, B), (183, 408, 417, P),
    (183, 418, 421, B), (183, 422, 427, P), (183, 428, 431, B), (183, 432, 435, P), (183, 436, 439, B),
    (183, 440, 443, P), (183, 444, 447, B), (183, 448, 451, P), (183, 452, 455, B), (183, 456, 459, P),
    (183, 460, 463, B), (183, 464, 467, P), (183, 468, 471, B), (183, 472, 475, P), (183, 476, 479, B),
    (183, 480, 483, P), (183, 484, 487, B), (183, 488, 491, P), (183, 492, 495, B), (183, 496, 507, P),
    (183, 508, 511, B), (183, 512, 523, P), (183, 524, 527, B), (183, 528, 539, P), (183, 540, 543, B),
    (183, 544, 547, P), (183, 548, 551, B), (183, 552, 559, P), (183, 560, 563, B), (183, 564, 579, P),
    (183, 580, 583, B), (183, 584, 585, P), (183, 586, 589, B), (183, 590, 591, P), (183, 592, 595, B),
    (183, 596, 603, P), (183, 604, 607, B), (183, 608, 617, P), (183, 618, 623, B), (183, 624, 625, P),
    (183, 626, 631, B), (183, 632, 635, P), (183, 636, 639, B), (184, 0, 1, B), (184, 2, 3, P),
    (184, 4, 7, B), (184, 8, 11, P), (184, 12, 15, B), (184, 16, 19, P), (184, 20, 23, B), (184, 24, 27, P),
    (184, 28, 31, B), (184, 32, 35, P), (184, 36, 39, B), (184, 40, 43, P), (184, 44, 47, B),
    (184, 48, 51, P), (184, 52, 55, B), (184, 56, 57, P), (184, 58, 61, B), (184, 62, 65, P),
    (184, 66, 69, B), (184, 70, 75, P), (184, 76, 85, B), (184, 86, 91, P), (184, 92, 101, B),
    (184, 102, 107, P), (184, 108, 111, B), (184, 112, 113, P), (184, 114, 119, B), (184, 120, 123, P),
    (184, 124, 135, B), (184, 136, 143, P), (184, 144, 147, B), (184, 148, 163, P), (184, 164, 167, B),
    (184, 168, 169, P), (184, 170, 177, B), (184, 178, 187, P), (184, 188, 191, B), (184, 192, 201, P),
    (184, 202, 215, B), (184, 216, 219, P), (184, 220, 225, B), (184, 226, 227, P), (184, 228, 231, B),
    (184, 232, 235, P), (184, 236, 239, B), (184, 240, 243, P), (184, 244, 247, B), (184, 248, 251, P),
    (184, 252, 255, B), (184, 256, 259, P), (184, 260, 263, B), (184, 264, 267, P), (184, 268, 271, B),
    (184, 272, 275, P), (184, 276, 279, B), (184, 280, 281, P), (184, 282, 285, B), (184, 286, 289, P),
    (184, 290, 293, B), (184, 294, 299, P), (184, 300, 305, B), (184, 306, 319, P), (184, 320, 323, B),
    (184, 324, 331, P), (184, 332, 335, B), (184, 336, 339, P), (184, 340, 343, B), (184, 344, 347, P),
    (184, 348, 351, B), (184, 352, 355, P), (184, 356, 359, B), (184, 360, 361, P), (184, 362, 365, B),
    (184, 366, 371, P), (184, 372, 375, B), (184, 376, 381, P), (184, 382, 389, B), (184, 390, 397, P),
    (184, 398, 405, B), (184, 406, 415, P), (184, 416, 419, B), (184, 420, 427, P), (184, 428, 431, B),
    (184, 432, 435, P), (184, 436, 439, B), (184, 440, 443, P), (184, 444, 455, B), (184, 456, 459, P),
    (184, 460, 463, B), (184, 464, 475, P), (184, 476, 479, B), (184, 480, 483, P), (184, 484, 487, B),
    (184, 488, 491, P), (184, 492, 501, B), (184, 502, 507, P), (184, 508, 517, B), (184, 518, 523, P),
    (184, 524, 527, B), (184, 528, 529, P), (184, 530, 535, B), (184, 536, 539, P), (184, 540, 551, B),
    (184, 552, 559, P), (184, 560, 563, B), (184, 564, 579, P), (184, 580, 583, B), (184, 584, 585, P),
    (184, 586, 593, B), (184, 594, 603, P), (184, 604, 607, B), (184, 608, 617, P), (184, 618, 631, B),
    (184, 632, 635, P), (184, 636, 639, B), (185, 0, 1, B), (185, 2, 3, P), (185, 4, 7, B), (185, 8, 11, P),
    (185, 12, 15, B), (185, 16, 19, P), (185, 20, 23, B), (185, 24, 27, P), (185, 28, 31, B),
    (185, 32, 35, P), (185, 36, 39, B), (185, 40, 43, P), (185, 44, 47, B), (185, 48, 51, P),
    (185, 52, 55, B), (185, 56, 57, P), (185, 58, 61, B), (185, 62, 65, P), (185, 66, 69, B),
    (185, 70, 75, P), (185, 76, 85, B), (185, 86, 91, P), (185, 92, 101, B), (185, 102, 107, P),
    (185, 108, 111, B), (185, 112, 113, P), (185, 114, 119, B), (185, 120, 123, P), (185, 124, 135, B),
    (185, 136, 143, P), (185, 144, 147, B), (185, 148, 163, P), (185, 164, 167, B), (185, 168, 169, P),
    (185, 170, 177, B), (185, 178, 187, P), (185, 188, 191, B), (185, 192, 201, P), (185, 202, 215, B),
    (185, 216, 219, P), (185, 220, 225, B), (185, 226, 227, P), (185, 228, 231, B), (185, 232, 235, P),
    (185, 236, 239, B), (185, 240, 243, P), (185, 244, 247, B), (185, 248, 251, P), (185, 252, 255, B),
    (185, 256, 259, P), (185, 260, 263, B), (185, 264, 267, P), (185, 268, 271, B), (185, 272, 275, P),
    (185, 276, 279, B), (185, 280, 281, P), (185, 282, 285, B), (185, 286, 289, P), (185, 290, 293, B),
    (185, 294, 299, P), (185, 300, 305, B), (185, 306, 319, P), (185, 320, 323, B), (185, 324, 331, P),
    (185, 332, 335, B), (185, 336, 339, P), (185, 340, 343, B), (185, 344, 347, P), (185, 348, 351, B),
    (185, 352, 355, P), (185, 356, 359, B), (185, 360, 361, P), (185, 362, 365, B), (185, 366, 371, P),
    (185, 372, 375, B), (185, 376, 381, P), (185, 382, 389, B), (185, 390, 397, P), (185, 398, 405, B),
    (185, 406, 415, P), (185, 416, 419, B), (185, 420, 427, P), (185, 428, 431, B), (185, 432, 435, P),
    (185, 436, 439, B), (185, 440, 443, P), (185, 444, 455, B), (185, 456, 459, P), (185, 460, 463, B),
    (185, 464, 475, P), (185, 476, 479, B), (185, 480, 483, P), (185, 484, 487, B), (185, 488, 491, P),
    (185, 492, 501, B), (185, 502, 507, P), (185, 508, 517, B), (185, 518, 523, P), (185, 524, 527, B),
    (185, 528, 529, P), (185, 530, 535, B), (185, 536, 539, P), (185, 540, 551, B), (185, 552, 559, P),
    (185, 560, 563, B), (185, 564, 579, P), (185, 580, 583, B), (185, 584, 585, P), (185, 586, 593, B),
    (185, 594, 603, P), (185, 604, 607, B), (185, 608, 617, P), (185, 618, 631, B), (185, 632, 635, P),
    (185, 636, 639, B), (186, 0, 1, B), (186, 2, 3, P), (186, 4, 7, B), (186, 8, 11, P), (186, 12, 15, B),
    (186, 16, 19, P), (186, 20, 23, B), (186, 24, 27, P), (186, 28, 31, B), (186, 32, 35, P),
    (186, 36, 39, B), (186, 40, 43, P), (186, 44, 47, B), (186, 48, 51, P), (186, 52, 55, B),
    (186, 56, 57, P), (186, 58, 61, B), (186, 62, 65, P), (186, 66, 69, B), (186, 70, 75, P),
    (186, 76, 85, B), (186, 86, 91, P), (186, 92, 101, B), (186, 102, 107, P), (186, 108, 111, B),
    (186, 112, 113, P), (186, 114, 119, B), (186, 120, 123, P), (186, 124, 135, B), (186, 136, 143, P),
    (186, 144, 147, B), (186, 148, 163, P), (186, 164, 167, B), (186, 168, 169, P), (186, 170, 177, B),
    (186, 178, 187, P), (186, 188, 191, B), (186, 192, 201, P), (186, 202, 205, B), (186, 206, 207, P),
    (186, 208, 209, B), (186, 210, 211, P), (186, 212, 215, B), (186, 216, 219, P), (186, 220, 231, B),
    (186, 232, 235, P), (186, 236, 239, B), (186, 240, 243, P), (186, 244, 247, B), (186, 248, 251, P),
    (186, 252, 255, B), (186, 256, 259, P), (186, 260, 263, B), (186, 264, 267, P), (186, 268, 271, B),
    (186, 272, 275, P), (186, 276, 279, B), (186, 280, 281, P), (186, 282, 293, B), (186, 294, 301, P),
    (186, 302, 307, B), (186, 308, 319, P), (186, 320, 323, B), (186, 324, 331, P), (186, 332, 335, B),
    (186, 336, 339, P), (186, 340, 343, B), (186, 344, 347, P), (186, 348, 351, B), (186, 352, 355, P),
    (186, 356, 359, B), (186, 360, 361, P), (186, 362, 365, B), (186, 366, 367, P), (186, 368, 369, B),
    (186, 370, 371, P), (186, 372, 375, B), (186, 376, 383, P), (186, 384, 387, B), (186, 388, 397, P),
    (186, 398, 405, B), (186, 406, 415, P), (186, 416, 419, B), (186, 420, 427, P), (186, 428, 431, B),
    (186, 432, 435, P), (186, 436, 439, B), (186, 440, 443, P), (186, 444, 453, B), (186, 454, 459, P),
    (186, 460, 463, B), (186, 464, 475, P), (186, 476, 479, B), (186, 480, 483, P), (186, 484, 487, B),
    (186, 488, 491, P), (186, 492, 501, B), (186, 502, 507, P), (186, 508, 517, B), (186, 518, 523, P),
    (186, 524, 527, B), (186, 528, 529, P), (186, 530, 535, B), (186, 536, 539, P), (186, 540, 551, B),
    (186, 552, 559, P), (186, 560, 563, B), (186, 564, 579, P), (186, 580, 583, B), (186, 584, 585, P),
    (186, 586, 593, B), (186, 594, 603, P), (186, 604, 607, B), (186, 608, 617, P), (186, 618, 621, B),
    (186, 622, 623, P), (186, 624, 625, B), (186, 626, 627, P), (186, 628, 631, B), (186, 632, 635, P),
    (186, 636, 639, B), (187, 0, 7, B), (187, 8, 11, P), (187, 12, 15, B), (187, 16, 19, P), (187, 20, 23, B),
    (187, 24, 27, P), (187, 28, 31, B), (187, 32, 35, P), (187, 36, 39, B), (187, 40, 43, P),
    (187, 44, 47, B), (187, 48, 51, P), (187, 52, 55, B), (187, 56, 57, P), (187, 58, 69, B),
    (187, 70, 75, P), (187, 76, 85, B), (187, 86, 91, P), (187, 92, 101, B), (187, 102, 107, P),
    (187, 108, 111, B), (187, 112, 113, P), (187, 114, 119, B), (187, 120, 123, P), (187, 124, 135, B),
    (187, 136, 143, P), (187, 144, 147, B), (187, 148, 163, P), (187, 164, 167, B), (187, 168, 169, P),
    (187, 170, 177, B), (187, 178, 187, P), (187, 188, 191, B), (187, 192, 201, P), (187, 202, 205, B),
    (187, 206, 207, P), (187, 208, 209, B), (187, 210, 211, P), (187, 212, 215, B), (187, 216, 219, P),
    (187, 220, 231, B), (187, 232, 235, P), (187, 236, 239, B), (187, 240, 243, P), (187, 244, 247, B),
    (187, 248, 251, P), (187, 252, 255, B), (187, 256, 259, P), (187, 260, 263, B), (187, 264, 267, P),
    (187, 268, 271, B), (187, 272, 275, P), (187, 276, 279, B), (187, 280, 281, P), (187, 282, 293, B),
    (187, 294, 301, P), (187, 302, 307, B), (187, 308, 319, P), (187, 320, 323, B), (187, 324, 331, P),
    (187, 332, 335, B), (187, 336, 339, P), (187, 340, 343, B), (187, 344, 347, P), (187, 348, 351, B),
    (187, 352, 355, P), (187, 356, 359, B), (187, 360, 361, P), (187, 362, 365, B), (187, 366, 367, P),
    (187, 368, 369, B), (187, 370, 371, P), (187, 372, 375, B), (187, 376, 383, P), (187, 384, 387, B),
    (187, 388, 397, P), (187, 398, 405, B), (187, 406, 415, P), (187, 416, 419, B), (187, 420, 427, P),
    (187, 428, 431, B), (187, 432, 435, P), (187, 436, 439, B), (187, 440, 443, P), (187, 444, 453, B),
    (187, 454, 459, P), (187, 460, 463, B), (187, 464, 475, P), (187, 476, 479, B), (187, 480, 483, P),
    (187, 484, 487, B), (187, 488, 491, P), (187, 492, 501, B), (187, 502, 507, P), (187, 508, 517, B),
    (187, 518, 523, P), (187, 524, 527, B), (187, 528, 529, P), (187, 530, 535, B), (187, 536, 539, P),
    (187, 540, 551, B), (187, 552, 559, P), (187, 560, 563, B), (187, 564, 579, P), (187, 580, 583, B),
    (187, 584, 585, P), (187, 586, 593, B), (187, 594, 603, P), (187, 604, 607, B), (187, 608, 617, P),
    (187, 618, 621, B), (187, 622, 623, P), (187, 624, 625, B), (187, 626, 627, P), (187, 628, 631, B),
    (187, 632, 635, P), (187, 636, 639, B), (188, 0, 7, B), (188, 8, 11, P), (188, 12, 15, B),
    (188, 16, 19, P), (188, 20, 23, B), (188, 24, 27, P), (188, 28, 31, B), (188, 32, 35, P),
    (188, 36, 39, B), (188, 40, 43, P), (188, 44, 47, B), (188, 48, 51, P), (188, 52, 55, B),
    (188, 56, 57, P), (188, 58, 69, B), (188, 70, 75, P), (188, 76, 79, B), (188, 80, 91, P),
    (188, 92, 95, B), (188, 96, 107, P), (188, 108, 111, B), (188, 112, 115, P), (188, 116, 119, B),
    (188, 120, 123, P), (188, 124, 127, B), (188, 128, 131, P), (188, 132, 135, B), (188, 136, 143, P),
    (188, 144, 147, B), (188, 148, 163, P), (188, 164, 167, B), (188, 168, 169, P), (188, 170, 173, B),
    (188, 174, 175, P), (188, 176, 179, B), (188, 180, 187, P), (188, 188, 191, B), (188, 192, 201, P),
    (188, 202, 205, B), (188, 206, 207, P), (188, 208, 209, B), (188, 210, 211, P), (188, 212, 215, B),
    (188, 216, 219, P), (188, 220, 231, B), (188, 232, 235, P), (188, 236, 239, B), (188, 240, 243, P),
    (188, 244, 247, B), (188, 248, 251, P), (188, 252, 263, B), (188, 264, 267, P), (188, 268, 271, B),
    (188, 272, 275, P), (188, 276, 279, B), (188, 280, 281, P), (188, 282, 291, B), (188, 292, 303, P),
    (188, 304, 309, B), (188, 310, 319, P), (188, 320, 323, B), (188, 324, 331, P), (188, 332, 335, B),
    (188, 336, 339, P), (188, 340, 343, B), (188, 344, 347, P), (188, 348, 351, B), (188, 352, 355, P),
    (188, 356, 359, B), (188, 360, 361, P), (188, 362, 365, B), (188, 366, 367, P), (188, 368, 369, B),
    (188, 370, 371, P), (188, 372, 375, B), (188, 376, 383, P), (188, 384, 387, B), (188, 388, 399, P),
    (188, 400, 403, B), (188, 404, 413, P), (188, 414, 417, B), (188, 418, 427, P), (188, 428, 439, B),
    (188, 440, 443, P), (188, 444, 447, B), (188, 448, 451, P), (188, 452, 455, B), (188, 456, 459, P),
    (188, 460, 463, B), (188, 464, 475, P), (188, 476, 479, B), (188, 480, 483, P), (188, 484, 487, B),
    (188, 488, 491, P), (188, 492, 495, B), (188, 496, 507, P), (188, 508, 511, B), (188, 512, 523, P),
    (188, 524, 527, B), (188, 528, 531, P), (188, 532, 535, B), (188, 536, 539, P), (188, 540, 543, B),
    (188, 544, 547, P), (188, 548, 551, B), (188, 552, 559, P), (188, 560, 563, B), (188, 564, 579, P),
    (188, 580, 583, B), (188, 584, 585, P), (188, 586, 589, B), (188, 590, 591, P), (188, 592, 595, B),
    (188, 596, 603, P), (188, 604, 607, B), (188, 608, 617, P), (188, 618, 621, B), (188, 622, 623, P),
    (188, 624, 625, B), (188, 626, 627, P), (188, 628, 631, B), (188, 632, 635, P), (188, 636, 639, B),
    (189, 0, 7, B), (189, 8, 11, P), (189, 12, 15, B), (189, 16, 19, P), (189, 20, 23, B), (189, 24, 27, P),
    (189, 28, 39, B), (189, 40, 43, P), (189, 44, 47, B), (189, 48, 51, P), (189, 52, 55, B),
    (189, 56, 57, P), (189, 58, 67, B), (189, 68, 75, P), (189, 76, 79, B), (189, 80, 91, P),
    (189, 92, 95, B), (189, 96, 107, P), (189, 108, 111, B), (189, 112, 115, P), (189, 116, 119, B),
    (189, 120, 123, P), (189, 124, 127, B), (189, 128, 131, P), (189, 132, 135, B), (189, 136, 143, P),
    (189, 144, 147, B), (189, 148, 163, P), (189, 164, 167, B), (189, 168, 169, P), (189, 170, 173, B),
    (189, 174, 175, P), (189, 176, 179, B), (189, 180, 187, P), (189, 188, 191, B), (189, 192, 201, P),
    (189, 202, 205, B), (189, 206, 207, P), (189, 208, 209, B), (189, 210, 211, P), (189, 212, 215, B),
    (189, 216, 219, P), (189, 220, 231, B), (189, 232, 235, P), (189, 236, 239, B), (189, 240, 243, P),
    (189, 244, 247, B), (189, 248, 251, P), (189, 252, 263, B), (189, 264, 267, P), (189, 268, 271, B),
    (189, 272, 275, P), (189, 276, 279, B), (189, 280, 281, P), (189, 282, 291, B), (189, 292, 303, P),
    (189, 304, 309, B), (189, 310, 319, P), (189, 320, 323, B), (189, 324, 331, P), (189, 332, 335, B),
    (189, 336, 339, P), (189, 340, 343, B), (189, 344, 347, P), (189, 348, 351, B), (189, 352, 355, P),
    (189, 356, 359, B), (189, 360, 361, P), (189, 362, 365, B), (189, 366, 367, P), (189, 368, 369, B),
    (189, 370, 371, P), (189, 372, 375, B), (189, 376, 383, P), (189, 384, 387, B), (189, 388, 399, P),
    (189, 400, 403, B), (189, 404, 413, P), (189, 414, 417, B), (189, 418, 427, P), (189, 428, 439, B),
    (189, 440, 443, P), (189, 444, 447, B), (189, 448, 451, P), (189, 452, 455, B), (189, 456, 459, P),
    (189, 460, 463, B), (189, 464, 475, P), (189, 476, 479, B), (189, 480, 483, P), (189, 484, 487, B),
    (189, 488, 491, P), (189, 492, 495, B), (189, 496, 507, P), (189, 508, 511, B), (189, 512, 523, P),
    (189, 524, 527, B), (189, 528, 531, P), (189, 532, 535, B), (189, 536, 539, P), (189, 540, 543, B),
    (189, 544, 547, P), (189, 548, 551, B), (189, 552, 559, P), (189, 560, 563, B), (189, 564, 579, P),
    (189, 580, 583, B), (189, 584, 585, P), (189, 586, 589, B), (189, 590, 591, P), (189, 592, 595, B),
    (189, 596, 603, P), (189, 604, 607, B), (189, 608, 617, P), (189, 618, 621, B), (189, 622, 623, P),
    (189, 624, 625, B), (189, 626, 627, P), (189, 628, 631, B), (189, 632, 635, P), (189, 636, 639, B),
    (190, 0, 7, B), (190, 8, 11, P), (190, 12, 15, B), (190, 16, 19, P), (190, 20, 23, B), (190, 24, 27, P),
    (190, 28, 39, B), (190, 40, 43, P), (190, 44, 47, B), (190, 48, 51, P), (190, 52, 55, B),
    (190, 56, 57, P), (190, 58, 67, B), (190, 68, 75, P), (190, 76, 79, B), (190, 80, 91, P),
    (190, 92, 95, B), (190, 96, 107, P), (190, 108, 111, B), (190, 112, 115, P), (190, 116, 119, B),
    (190, 120, 123, P), (190, 124, 127, B), (190, 128, 131, P), (190, 132, 135, B), (190, 136, 143, P),
    (190, 144, 147, B), (190, 148, 163, P), (190, 164, 167, B), (190, 168, 169, P), (190, 170, 173, B),
    (190, 174, 175, P), (190, 176, 179, B), (190, 180, 187, P), (190, 188, 191, B), (190, 192, 201, P),
    (190, 202, 205, B), (190, 206, 211, P), (190, 212, 215, B), (190, 216, 219, P), (190, 220, 223, B),
    (190, 224, 225, P), (190, 226, 231, B), (190, 232, 235, P), (190, 236, 239, B), (190, 240, 243, P),
    (190, 244, 247, B), (190, 248, 251, P), (190, 252, 261, B), (190, 262, 267, P), (190, 268, 271, B),
    (190, 272, 275, P), (190, 276, 279, B), (190, 280, 281, P), (190, 282, 285, B), (190, 286, 287, P),
    (190, 288, 291, B), (190, 292, 305, P), (190, 306, 311, B), (190, 312, 319, P), (190, 320, 323, B),
    (190, 324, 331, P), (190, 332, 335, B), (190, 336, 339, P), (190, 340, 343, B), (190, 344, 347, P),
    (190, 348, 351, B), (190, 352, 355, P), (190, 356, 359, B), (190, 360, 361, P), (190, 362, 375, B),
    (190, 376, 381, P), (190, 382, 389, B), (190, 390, 399, P), (190, 400, 403, B), (190, 404, 413, P),
    (190, 414, 417, B), (190, 418, 427, P), (190, 428, 439, B), (190, 440, 443, P), (190, 444, 447, B),
    (190, 448, 451, P), (190, 452, 455, B), (190, 456, 459, P), (190, 460, 463, B), (190, 464, 475, P),
    (190, 476, 479, B), (190, 480, 483, P), (190, 484, 487, B), (190, 488, 491, P), (190, 492, 495, B),
    (190, 496, 507, P), (190, 508, 511, B), (190, 512, 523, P), (190, 524, 527, B), (190, 528, 531, P),
    (190, 532, 535, B), (190, 536, 539, P), (190, 540, 543, B), (190, 544, 547, P), (190, 548, 551, B),
    (190, 552, 559, P), (190, 560, 563, B), (190, 564, 579, P), (190, 580, 583, B), (190, 584, 585, P),
    (190, 586, 589, B), (190, 590, 591, P), (190, 592, 595, B), (190, 596, 603, P), (190, 604, 607, B),
    (190, 608, 617, P), (190, 618, 621, B), (190, 622, 627, P), (190, 628, 631, B), (190, 632, 635, P),
    (190, 636, 639, B), (191, 0, 1, P), (191, 2, 7, B), (191, 8, 11, P), (191, 12, 15, B), (191, 16, 19, P),
    (191, 20, 23, B), (191, 24, 27, P), (191, 28, 37, B), (191, 38, 43, P), (191, 44, 47, B),
    (191, 48, 51, P), (191, 52, 55, B), (191, 56, 57, P), (191, 58, 61, B), (191, 62, 63, P),
    (191, 64, 67, B), (191, 68, 75, P), (191, 76, 79, B), (191, 80, 91, P), (191, 92, 95, B),
    (191, 96, 107, P), (191, 108, 111, B), (191, 112, 115, P), (191, 116, 119, B), (191, 120, 123, P),
    (191, 124, 127, B), (191, 128, 131, P), (191, 132, 135, B), (191, 136, 143, P), (191, 144, 147, B),
    (191, 148, 163, P), (191, 164, 167, B), (191, 168, 169, P), (191, 170, 173, B), (191, 174, 175, P),
    (191, 176, 179, B), (191, 180, 187, P), (191, 188, 191, B), (191, 192, 201, P), (191, 202, 205, B),
    (191, 206, 211, P), (191, 212, 215, B), (191, 216, 219, P), (191, 220, 223, B), (191, 224, 225, P),
    (191, 226, 231, B), (191, 232, 235, P), (191, 236, 239, B), (191, 240, 243, P), (191, 244, 247, B),
    (191, 248, 251, P), (191, 252, 261, B), (191, 262, 267, P), (191, 268, 271, B), (191, 272, 275, P),
    (191, 276, 279, B), (191, 280, 281, P), (191, 282, 285, B), (191, 286, 287, P), (191, 288, 291, B),
    (191, 292, 305, P), (191, 306, 311, B), (191, 312, 319, P), (191, 320, 323, B), (191, 324, 331, P),
    (191, 332, 335, B), (191, 336, 339, P), (191, 340, 343, B), (191, 344, 347, P), (191, 348, 351, B),
    (191, 352, 355, P), (191, 356, 359, B), (191, 360, 361, P), (191, 362, 375, B), (191, 376, 381, P),
    (191, 382, 389, B), (191, 390, 399, P), (191, 400, 403, B), (191, 404, 413, P), (191, 414, 417, B),
    (191, 418, 427, P), (191, 428, 439, B), (191, 440, 443, P), (191, 444, 447, B), (191, 448, 451, P),
    (191, 452, 455, B), (191, 456, 459, P), (191, 460, 463, B), (191, 464, 475, P), (191, 476, 479, B),
    (191, 480, 483, P), (191, 484, 487, B), (191, 488, 491, P), (191, 492, 495, B), (191, 496, 507, P),
    (191, 508, 511, B), (191, 512, 523, P), (191, 524, 527, B), (191, 528, 531, P), (191, 532, 535, B),
    (191, 536, 539, P), (191, 540, 543, B), (191, 544, 547, P), (191, 548, 551, B), (191, 552, 559, P),
    (191, 560, 563, B), (191, 564, 579, P), (191, 580, 583, B), (191, 584, 585, P), (191, 586, 589, B),
    (191, 590, 591, P), (191, 592, 595, B), (191, 596, 603, P), (191, 604, 607, B), (191, 608, 617, P),
    (191, 618, 621, B), (191, 622, 627, P), (191, 628, 631, B), (191, 632, 635, P), (191, 636, 639, B),
    (192, 0, 1, P), (192, 2, 7, B), (192, 8, 11, P), (192, 12, 15, B), (192, 16, 19, P), (192, 20, 23, B),
    (192, 24, 27, P), (192, 28, 37, B), (192, 38, 43, P), (192, 44, 47, B), (192, 48, 51, P),
    (192, 52, 55, B), (192, 56, 57, P), (192, 58, 61, B), (192, 62, 63, P), (192, 64, 67, B),
    (192, 68, 75, P), (192, 76, 79, B), (192, 80, 91, P), (192, 92, 95, B), (192, 96, 107, P),
    (192, 108, 111, B), (192, 112, 115, P), (192, 116, 119, B), (192, 120, 123, P), (192, 124, 127, B),
    (192, 128, 131, P), (192, 132, 135, B), (192, 136, 143, P), (192, 144, 147, B), (192, 148, 155, P),
    (192, 156, 159, B), (192, 160, 163, P), (192, 164, 167, B), (192, 168, 169, P), (192, 170, 173, B),
    (192, 174, 177, P), (192, 178, 181, B), (192, 182, 187, P), (192, 188, 191, B), (192, 192, 201, P),
    (192, 202, 205, B), (192, 206, 211, P), (192, 212, 215, B), (192, 216, 219, P), (192, 220, 223, B),
    (192, 224, 225, P), (192, 226, 231, B), (192, 232, 235, P), (192, 236, 239, B), (192, 240, 243, P),
    (192, 244, 247, B), (192, 248, 251, P), (192, 252, 255, B), (192, 256, 267, P), (192, 268, 271, B),
    (192, 272, 275, P), (192, 276, 279, B), (192, 280, 281, P), (192, 282, 285, B), (192, 286, 289, P),
    (192, 290, 293, B), (192, 294, 307, P), (192, 308, 311, B), (192, 312, 319, P), (192, 320, 323, B),
    (192, 324, 331, P), (192, 332, 335, B), (192, 336, 339, P), (192, 340, 343, B), (192, 344, 349, P),
    (192, 350, 357, B), (192, 358, 361, P), (192, 362, 375, B), (192, 376, 381, P), (192, 382, 389, B),
    (192, 390, 399, P), (192, 400, 403, B), (192, 404, 411, P), (192, 412, 415, B), (192, 416, 427, P),
    (192, 428, 431, B), (192, 432, 435, P), (192, 436, 439, B), (192, 440, 443, P), (192, 444, 447, B),
    (192, 448, 451, P), (192, 452, 455, B), (192, 456, 459, P), (192, 460, 463, B), (192, 464, 467, P),
    (192, 468, 471, B), (192, 472, 475, P), (192, 476, 479, B), (192, 480, 483, P), (192, 484, 487, B),
    (192, 488, 491, P), (192, 492, 495, B), (192, 496, 507, P), (192, 508, 511, B), (192, 512, 523, P),
    (192, 524, 527, B), (192, 528, 531, P), (192, 532, 535, B), (192, 536, 539, P), (192, 540, 543, B),
    (192, 544, 547, P), (192, 548, 551, B), (192, 552, 559, P), (192, 560, 563, B), (192, 564, 571, P),
    (192, 572, 575, B), (192, 576, 579, P), (192, 580, 583, B), (192, 584, 585, P), (192, 586, 589, B),
    (192, 590, 593, P), (192, 594, 597, B), (192, 598, 603, P), (192, 604, 607, B), (192, 608, 617, P),
    (192, 618, 621, B), (192, 622, 627, P), (192, 628, 631, B), (192, 632, 635, P), (192, 636, 639, B),
    (193, 0, 1, P), (193, 2, 7, B), (193, 8, 11, P), (193, 12, 15, B), (193, 16, 19, P), (193, 20, 23, B),
    (193, 24, 27, P), (193, 28, 31, B), (193, 32, 43, P), (193, 44, 47, B), (193, 48, 51, P),
    (193, 52, 55, B), (193, 56, 57, P), (193, 58, 61, B), (193, 62, 65, P), (193, 66, 69, B),
    (193, 70, 75, P), (193, 76, 79, B), (193, 80, 91, P), (193, 92, 95, B), (193, 96, 107, P),
    (193, 108, 111, B), (193, 112, 115, P), (193, 116, 119, B), (193, 120, 123, P), (193, 124, 127, B),
    (193, 128, 131, P), (193, 132, 135, B), (193, 136, 143, P), (193, 144, 147, B), (193, 148, 155, P),
    (193, 156, 159, B), (193, 160, 163, P), (193, 164, 167, B), (193, 168, 169, P), (193, 170, 173, B),
    (193, 174, 177, P), (193, 178, 181, B), (193, 182, 187, P), (193, 188, 191, B), (193, 192, 201, P),
    (193, 202, 205, B), (193, 206, 211, P), (193, 212, 215, B), (193, 216, 219, P), (193, 220, 223, B),
    (193, 224, 225, P), (193, 226, 231, B), (193, 232, 235, P), (193, 236, 239, B), (193, 240, 243, P),
    (193, 244, 247, B), (193, 248, 251, P), (193, 252, 255, B), (193, 256, 267, P), (193, 268, 271, B),
    (193, 272, 275, P), (193, 276, 279, B), (193, 280, 281, P), (193, 282, 285, B), (193, 286, 289, P),
    (193, 290, 293, B), (193, 294, 307, P), (193, 308, 311, B), (193, 312, 319, P), (193, 320, 323, B),
    (193, 324, 331, P), (193, 332, 335, B), (193, 336, 339, P), (193, 340, 343, B), (193, 344, 349, P),
    (193, 350, 357, B), (193, 358, 361, P), (193, 362, 375, B), (193, 376, 381, P), (193, 382, 389, B),
    (193, 390, 399, P), (193, 400, 403, B), (193, 404, 411, P), (193, 412, 415, B), (193, 416, 427, P),
    (193, 428, 431, B), (193, 432, 435, P), (193, 436, 439, B), (193, 440, 443, P), (193, 444, 447, B),
    (193, 448, 451, P), (193, 452, 455, B), (193, 456, 459, P), (193, 460, 463, B), (193, 464, 467, P),
    (193, 468, 471, B), (193, 472, 475, P), (193, 476, 479, B), (193, 480, 483, P), (193, 484, 487, B),
    (193, 488, 491, P), (193, 492, 495, B), (193, 496, 507, P), (193, 508, 511, B), (193, 512, 523, P),
    (193, 524, 527, B), (193, 528, 531, P), (193, 532, 535, B), (193, 536, 539, P), (193, 540, 543, B),
    (193, 544, 547, P), (193, 548, 551, B), (193, 552, 559, P), (193, 560, 563, B), (193, 564, 571, P),
    (193, 572, 575, B), (193, 576, 579, P), (193, 580, 583, B), (193, 584, 585, P), (193, 586, 589, B),
    (193, 590, 593, P), (193, 594, 597, B), (193, 598, 603, P), (193, 604, 607, B), (193, 608, 617, P),
    (193, 618, 621, B), (193, 622, 627, P), (193, 628, 631, B), (193, 632, 635, P), (193, 636, 639, B),
    (194, 0, 1, P), (194, 2, 7, B), (194, 8, 11, P), (194, 12, 15, B), (194, 16, 19, P), (194, 20, 23, B),
    (194, 24, 27, P), (194, 28, 31, B), (194, 32, 43, P), (194, 44, 47, B), (194, 48, 51, P),
    (194, 52, 55, B), (194, 56, 57, P), (194, 58, 61, B), (194, 62, 65, P), (194, 66, 69, B),
    (194, 70, 75, P), (194, 76, 79, B), (194, 80, 91, P), (194, 92, 95, B), (194, 96, 107, P),
    (194, 108, 111, B), (194, 112, 115, P), (194, 116, 119, B), (194, 120, 123, P), (194, 124, 127, B),
    (194, 128, 131, P), (194, 132, 135, B), (194, 136, 143, P), (194, 144, 147, B), (194, 148, 155, P),
    (194, 156, 159, B), (194, 160, 163, P), (194, 164, 167, B), (194, 168, 169, P), (194, 170, 173, B),
    (194, 174, 177, P), (194, 178, 181, B), (194, 182, 187, P), (194, 188, 191, B), (194, 192, 201, P),
    (194, 202, 205, B), (194, 206, 211, P), (194, 212, 215, B), (194, 216, 219, P), (194, 220, 223, B),
    (194, 224, 227, P), (194, 228, 231, B), (194, 232, 235, P), (194, 236, 239, B), (194, 240, 243, P),
    (194, 244, 247, B), (194, 248, 251, P), (194, 252, 255, B), (194, 256, 267, P), (194, 268, 271, B),
    (194, 272, 273, P), (194, 274, 275, B), (194, 276, 277, P), (194, 278, 279, B), (194, 280, 281, P),
    (194, 282, 285, B), (194, 286, 289, P), (194, 290, 293, B), (194, 294, 307, P), (194, 308, 311, B),
    (194, 312, 319, P), (194, 320, 323, B), (194, 324, 331, P), (194, 332, 335, B), (194, 336, 339, P),
    (194, 340, 343, B), (194, 344, 349, P), (194, 350, 357, B), (194, 358, 361, P), (194, 362, 367, B),
    (194, 368, 369, P), (194, 370, 375, B), (194, 376, 379, P), (194, 380, 383, B), (194, 384, 387, P),
    (194, 388, 391, B), (194, 392, 399, P), (194, 400, 403, B), (194, 404, 411, P), (194, 412, 415, B),
    (194, 416, 427, P), (194, 428, 431, B), (194, 432, 435, P), (194, 436, 439, B), (194, 440, 443, P),
    (194, 444, 447, B), (194, 448, 451, P), (194, 452, 455, B), (194, 456, 459, P), (194, 460, 463, B),
    (194, 464, 467, P), (194, 468, 471, B), (194, 472, 475, P), (194, 476, 479, B), (194, 480, 481, P),
    (194, 482, 487, B), (194, 488, 491, P), (194, 492, 495, B), (194, 496, 507, P), (194, 508, 511, B),
    (194, 512, 523, P), (194, 524, 527, B), (194, 528, 531, P), (194, 532, 535, B), (194, 536, 539, P),
    (194, 540, 543, B), (194, 544, 547, P), (194, 548, 551, B), (194, 552, 559, P), (194, 560, 563, B),
    (194, 564, 571, P), (194, 572, 575, B), (194, 576, 579, P), (194, 580, 583, B), (194, 584, 585, P),
    (194, 586, 589, B), (194, 590, 593, P), (194, 594, 597, B), (194, 598, 603, P), (194, 604, 607, B),
    (194, 608, 617, P), (194, 618, 621, B), (194, 622, 627, P), (194, 628, 631, B), (194, 632, 635, P),
    (194, 636, 639, B), (195, 0, 3, P), (195, 4, 7, B), (195, 8, 11, P), (195, 12, 15, B), (195, 16, 19, P),
    (195, 20, 23, B), (195, 24, 27, P), (195, 28, 31, B), (195, 32, 43, P), (195, 44, 47, B),
    (195, 48, 49, P), (195, 50, 51, B), (195, 52, 53, P), (195, 54, 55, B), (195, 56, 57, P),
    (195, 58, 61, B), (195, 62, 65, P), (195, 66, 69, B), (195, 70, 75, P), (195, 76, 79, B),
    (195, 80, 91, P), (195, 92, 95, B), (195, 96, 107, P), (195, 108, 111, B), (195, 112, 115, P),
    (195, 116, 119, B), (195, 120, 123, P), (195, 124, 127, B), (195, 128, 131, P), (195, 132, 135, B),
    (195, 136, 143, P), (195, 144, 147, B), (195, 148, 155, P), (195, 156, 159, B), (195, 160, 163, P),
    (195, 164, 167, B), (195, 168, 169, P), (195, 170, 173, B), (195, 174, 177, P), (195, 178, 181, B),
    (195, 182, 187, P), (195, 188, 191, B), (195, 192, 201, P), (195, 202, 205, B), (195, 206, 211, P),
    (195, 212, 215, B), (195, 216, 219, P), (195, 220, 223, B), (195, 224, 227, P), (195, 228, 231, B),
    (195, 232, 235, P), (195, 236, 239, B), (195, 240, 243, P), (195, 244, 247, B), (195, 248, 251, P),
    (195, 252, 255, B), (195, 256, 267, P), (195, 268, 271, B), (195, 272, 273, P), (195, 274, 275, B),
    (195, 276, 277, P), (195, 278, 279, B), (195, 280, 281, P), (195, 282, 285, B), (195, 286, 289, P),
    (195, 290, 293, B), (195, 294, 307, P), (195, 308, 311, B), (195, 312, 319, P), (195, 320, 323, B),
    (195, 324, 331, P), (195, 332, 335, B), (195, 336, 339, P), (195, 340, 343, B), (195, 344, 349, P),
    (195, 350, 357, B), (195, 358, 361, P), (195, 362, 367, B), (195, 368, 369, P), (195, 370, 375, B),
    (195, 376, 379, P), (195, 380, 383, B), (195, 384, 387, P), (195, 388, 391, B), (195, 392, 399, P),
    (195, 400, 403, B), (195, 404, 411, P), (195, 412, 415, B), (195, 416, 427, P), (195, 428, 431, B),
    (195, 432, 435, P), (195, 436, 439, B), (195, 440, 443, P), (195, 444, 447, B), (195, 448, 451, P),
    (195, 452, 455, B), (195, 456, 459, P), (195, 460, 463, B), (195, 464, 467, P), (195, 468, 471, B),
    (195, 472, 475, P), (195, 476, 479, B), (195, 480, 481, P), (195, 482, 487, B), (195, 488, 491, P),
    (195, 492, 495, B), (195, 496, 507, P), (195, 508, 511, B), (195, 512, 523, P), (195, 524, 527, B),
    (195, 528, 531, P), (195, 532, 535, B), (195, 536, 539, P), (195, 540, 543, B), (195, 544, 547, P),
    (195, 548, 551, B), (195, 552, 559, P), (195, 560, 563, B), (195, 564, 571, P), (195, 572, 575, B),
    (195, 576, 579, P), (195, 580, 583, B), (195, 584, 585, P), (195, 586, 589, B), (195, 590, 593, P),
    (195, 594, 597, B), (195, 598, 603, P), (195, 604, 607, B), (195, 608, 617, P), (195, 618, 621, B),
    (195, 622, 627, P), (195, 628, 631, B), (195, 632, 635, P), (195, 636, 639, B), (196, 0, 3, P),
    (196, 4, 7, B), (196, 8, 11, P), (196, 12, 15, B), (196, 16, 19, P), (196, 20, 23, B), (196, 24, 27, P),
    (196, 28, 31, B), (196, 32, 43, P), (196, 44, 47, B), (196, 48, 49, P), (196, 50, 51, B),
    (196, 52, 53, P), (196, 54, 55, B), (196, 56, 57, P), (196, 58, 61, B), (196, 62, 65, P),
    (196, 66, 69, B), (196, 70, 75, P), (196, 76, 87, B), (196, 88, 91, P), (196, 92, 95, B),
    (196, 96, 107, P), (196, 108, 119, B), (196, 120, 123, P), (196, 124, 127, B), (196, 128, 131, P),
    (196, 132, 135, B), (196, 136, 139, P), (196, 140, 151, B), (196, 152, 155, P), (196, 156, 167, B),
    (196, 168, 169, P), (196, 170, 173, B), (196, 174, 179, P), (196, 180, 183, B), (196, 184, 187, P),
    (196, 188, 199, B), (196, 200, 201, P), (196, 202, 205, B), (196, 206, 211, P), (196, 212, 215, B),
    (196, 216, 219, P), (196, 220, 223, B), (196, 224, 227, P), (196, 228, 231, B), (196, 232, 235, P),
    (196, 236, 247, B), (196, 248, 251, P), (196, 252, 255, B), (196, 256, 267, P), (196, 268, 277, B),
    (196, 278, 281, P), (196, 282, 285, B), (196, 286, 291, P), (196, 292, 295, B), (196, 296, 299, P),
    (196, 300, 311, B), (196, 312, 319, P), (196, 320, 323, B), (196, 324, 331, P), (196, 332, 343, B),
    (196, 344, 351, P), (196, 352, 355, B), (196, 356, 361, P), (196, 362, 365, B), (196, 366, 371, P),
    (196, 372, 375, B), (196, 376, 379, P), (196, 380, 383, B), (196, 384, 387, P), (196, 388, 391, B),
    (196, 392, 399, P), (196, 400, 403, B), (196, 404, 411, P), (196, 412, 423, B), (196, 424, 427, P),
    (196, 428, 431, B), (196, 432, 435, P), (196, 436, 439, B), (196, 440, 443, P), (196, 444, 455, B),
    (196, 456, 459, P), (196, 460, 471, B), (196, 472, 475, P), (196, 476, 485, B), (196, 486, 491, P),
    (196, 492, 503, B), (196, 504, 507, P), (196, 508, 511, B), (196, 512, 523, P), (196, 524, 535, B),
    (196, 536, 539, P), (196, 540, 543, B), (196, 544, 547, P), (196, 548, 551, B), (196, 552, 555, P),
    (196, 556, 567, B), (196, 568, 571, P), (196, 572, 583, B), (196, 584, 585, P), (196, 586, 589, B),
    (196, 590, 595, P), (196, 596, 599, B), (196, 600, 603, P), (196, 604, 615, B), (196, 616, 617, P),
    (196, 618, 621, B), (196, 622, 627, P), (196, 628, 631, B), (196, 632, 635, P), (196, 636, 639, B),
    (197, 0, 3, P), (197, 4, 7, B), (197, 8, 11, P), (197, 12, 23, B), (197, 24, 27, P), (197, 28, 31, B),
    (197, 32, 43, P), (197, 44, 53, B), (197, 54, 57, P), (197, 58, 61, B), (197, 62, 67, P),
    (197, 68, 71, B), (197, 72, 75, P), (197, 76, 87, B), (197, 88, 91, P), (197, 92, 95, B),
    (197, 96, 107, P), (197, 108, 119, B), (197, 120, 123, P), (197, 124, 127, B), (197, 128, 131, P),
    (197, 132, 135, B), (197, 136, 139, P), (197, 140, 151, B), (197, 152, 155, P), (197, 156, 167, B),
    (197, 168, 169, P), (197, 170, 173, B), (197, 174, 179, P), (197, 180, 183, B), (197, 184, 187, P),
    (197, 188, 199, B), (197, 200, 201, P), (197, 202, 205, B), (197, 206, 211, P), (197, 212, 215, B),
    (197, 216, 219, P), (197, 220, 223, B), (197, 224, 227, P), (197, 228, 231, B), (197, 232, 235, P),
    (197, 236, 247, B), (197, 248, 251, P), (197, 252, 255, B), (197, 256, 267, P), (197, 268, 277, B),
    (197, 278, 281, P), (197, 282, 285, B), (197, 286, 291, P), (197, 292, 295, B), (197, 296, 299, P),
    (197, 300, 311, B), (197, 312, 319, P), (197, 320, 323, B), (197, 324, 331, P), (197, 332, 343, B),
    (197, 344, 351, P), (197, 352, 355, B), (197, 356, 361, P), (197, 362, 365, B), (197, 366, 371, P),
    (197, 372, 375, B), (197, 376, 379, P), (197, 380, 383, B), (197, 384, 387, P), (197, 388, 391, B),
    (197, 392, 399, P), (197, 400, 403, B), (197, 404, 411, P), (197, 412, 423, B), (197, 424, 427, P),
    (197, 428, 431, B), (197, 432, 435, P), (197, 436, 439, B), (197, 440, 443, P), (197, 444, 455, B),
    (197, 456, 459, P), (197, 460, 471, B), (197, 472, 475, P), (197, 476, 485, B), (197, 486, 491, P),
    (197, 492, 503, B), (197, 504, 507, P), (197, 508, 511, B), (197, 512, 523, P), (197, 524, 535, B),
    (197, 536, 539, P), (197, 540, 543, B), (197, 544, 547, P), (197, 548, 551, B), (197, 552, 555, P),
    (197, 556, 567, B), (197, 568, 571, P), (197, 572, 583, B), (197, 584, 585, P), (197, 586, 589, B),
    (197, 590, 595, P), (197, 596, 599, B), (197, 600, 603, P), (197, 604, 615, B), (197, 616, 617, P),
    (197, 618, 621, B), (197, 622, 627, P), (197, 628, 631, B), (197, 632, 635, P), (197, 636, 639, B),
    (198, 0, 3, P), (198, 4, 7, B), (198, 8, 11, P), (198, 12, 23, B), (198, 24, 27, P), (198, 28, 31, B),
    (198, 32, 43, P), (198, 44, 53, B), (198, 54, 57, P), (198, 58, 61, B), (198, 62, 67, P),
    (198, 68, 71, B), (198, 72, 75, P), (198, 76, 87, B), (198, 88, 91, P), (198, 92, 95, B),
    (198, 96, 109, P), (198, 110, 117, B), (198, 118, 123, P), (198, 124, 127, B), (198, 128, 131, P),
    (198, 132, 135, B), (198, 136, 139, P), (198, 140, 151, B), (198, 152, 157, P), (198, 158, 165, B),
    (198, 166, 169, P), (198, 170, 173, B), (198, 174, 179, P), (198, 180, 183, B), (198, 184, 187, P),
    (198, 188, 199, B), (198, 200, 201, P), (198, 202, 205, B), (198, 206, 211, P), (198, 212, 215, B),
    (198, 216, 219, P), (198, 220, 223, B), (198, 224, 227, P), (198, 228, 231, B), (198, 232, 237, P),
    (198, 238, 245, B), (198, 246, 251, P), (198, 252, 255, B), (198, 256, 269, P), (198, 270, 273, B),
    (198, 274, 275, P), (198, 276, 279, B), (198, 280, 281, P), (198, 282, 285, B), (198, 286, 291, P),
    (198, 292, 295, B), (198, 296, 299, P), (198, 300, 309, B), (198, 310, 319, P), (198, 320, 323, B),
    (198, 324, 333, P), (198, 334, 341, B), (198, 342, 351, P), (198, 352, 355, B), (198, 356, 361, P),
    (198, 362, 363, B), (198, 364, 373, P), (198, 374, 375, B), (198, 376, 379, P), (198, 380, 383, B),
    (198, 384, 387, P), (198, 388, 391, B), (198, 392, 399, P), (198, 400, 403, B), (198, 404, 411, P),
    (198, 412, 423, B), (198, 424, 427, P), (198, 428, 431, B), (198, 432, 435, P), (198, 436, 439, B),
    (198, 440, 443, P), (198, 444, 453, B), (198, 454, 461, P), (198, 462, 469, B), (198, 470, 475, P),
    (198, 476, 483, B), (198, 484, 491, P), (198, 492, 503, B), (198, 504, 507, P), (198, 508, 511, B),
    (198, 512, 525, P), (198, 526, 533, B), (198, 534, 539, P), (198, 540, 543, B), (198, 544, 547, P),
    (198, 548, 551, B), (198, 552, 555, P), (198, 556, 567, B), (198, 568, 573, P), (198, 574, 581, B),
    (198, 582, 585, P), (198, 586, 589, B), (198, 590, 595, P), (198, 596, 599, B), (198, 600, 603, P),
    (198, 604, 615, B), (198, 616, 617, P), (198, 618, 621, B), (198, 622, 627, P), (198, 628, 631, B),
    (198, 632, 635, P), (198, 636, 639, B), (199, 0, 3, P), (199, 4, 7, B), (199, 8, 13, P), (199, 14, 21, B),
    (199, 22, 27, P), (199, 28, 31, B), (199, 32, 45, P), (199, 46, 49, B), (199, 50, 51, P),
    (199, 52, 55, B), (199, 56, 57, P), (199, 58, 61, B), (199, 62, 67, P), (199, 68, 71, B),
    (199, 72, 75, P), (199, 76, 87, B), (199, 88, 91, P), (199, 92, 95, B), (199, 96, 109, P),
    (199, 110, 117, B), (199, 118, 123, P), (199, 124, 127, B), (199, 128, 131, P), (199, 132, 135, B),
    (199, 136, 139, P), (199, 140, 151, B), (199, 152, 157, P), (199, 158, 165, B), (199, 166, 169, P),
    (199, 170, 173, B), (199, 174, 179, P), (199, 180, 183, B), (199, 184, 187, P), (199, 188, 199, B),
    (199, 200, 201, P), (199, 202, 205, B), (199, 206, 211, P), (199, 212, 215, B), (199, 216, 219, P),
    (199, 220, 223, B), (199, 224, 227, P), (199, 228, 231, B), (199, 232, 237, P), (199, 238, 245, B),
    (199, 246, 251, P), (199, 252, 255, B), (199, 256, 269, P), (199, 270, 273, B), (199, 274, 275, P),
    (199, 276, 279, B), (199, 280, 281, P), (199, 282, 285, B), (199, 286, 291, P), (199, 292, 295, B),
    (199, 296, 299, P), (199, 300, 309, B), (199, 310, 319, P), (199, 320, 323, B), (199, 324, 333, P),
    (199, 334, 341, B), (199, 342, 351, P), (199, 352, 355, B), (199, 356, 361, P), (199, 362, 363, B),
    (199, 364, 373, P), (199, 374, 375, B), (199, 376, 379, P), (199, 380, 383, B), (199, 384, 387, P),
    (199, 388, 391, B), (199, 392, 399, P), (199, 400, 403, B), (199, 404, 411, P), (199, 412, 423, B),
    (199, 424, 427, P), (199, 428, 431, B), (199, 432, 435, P), (199, 436, 439, B), (199, 440, 443, P),
    (199, 444, 453, B), (199, 454, 461, P), (199, 462, 469, B), (199, 470, 475, P), (199, 476, 483, B),
    (199, 484, 491, P), (199, 492, 503, B), (199, 504, 507, P), (199, 508, 511, B), (199, 512, 525, P),
    (199, 526, 533, B), (199, 534, 539, P), (199, 540, 543, B), (199, 544, 547, P), (199, 548, 551, B),
    (199, 552, 555, P), (199, 556, 567, B), (199, 568, 573, P), (199, 574, 581, B), (199, 582, 585, P),
    (199, 586, 589, B), (199, 590, 595, P), (199, 596, 599, B), (199, 600, 603, P), (199, 604, 615, B),
    (199, 616, 617, P), (199, 618, 621, B), (199, 622, 627, P), (199, 628, 631, B), (199, 632, 635, P),
    (199, 636, 639, B), (200, 0, 3, P), (200, 4, 7, B), (200, 8, 13, P), (200, 14, 21, B), (200, 22, 27, P),
    (200, 28, 31, B), (200, 32, 45, P), (200, 46, 49, B), (200, 50, 51, P), (200, 52, 55, B),
    (200, 56, 57, P), (200, 58, 61, B), (200, 62, 67, P), (200, 68, 71, B), (200, 72, 639, P),
    (201, 0, 639, P), (202, 0, 639, P), (203, 0, 639, P), (204, 0, 639, P), (205, 0, 639, P),
    (206, 0, 639, P), (207, 0, 639, P), (208, 0, 639, P), (209, 0, 639, P), (210, 0, 639, P),
    (211, 0, 639, P), (212, 0, 639, P), (213, 0, 639, P), (214, 0, 639, P), (215, 0, 639, P),
    (216, 0, 639, P), (217, 0, 639, P), (218, 0, 639, P), (219, 0, 639, P),
];
/// T14C: grab (0,40) 640x30, record 307 of probe/e1.
#[rustfmt::skip]
const SP_T14C: &[Span] = &[
    (56, 10, 10, P), (56, 11, 13, B), (56, 14, 15, P), (56, 16, 19, B), (56, 20, 22, P), (56, 23, 26, B),
    (56, 27, 27, P), (56, 28, 31, B), (56, 32, 33, P), (56, 34, 38, B), (56, 39, 39, P), (56, 40, 44, B),
    (56, 45, 46, P), (56, 47, 50, B), (56, 51, 51, P), (56, 52, 52, B), (56, 53, 55, P), (56, 56, 56, B),
    (56, 57, 58, P), (56, 59, 61, B), (56, 62, 67, P), (56, 68, 68, B), (56, 69, 70, P), (56, 71, 71, B),
    (56, 72, 73, P), (56, 74, 74, B), (56, 75, 76, P), (56, 77, 77, B), (56, 78, 81, P), (56, 82, 82, B),
    (56, 83, 85, P), (56, 86, 86, B), (56, 87, 87, P), (56, 88, 88, B), (56, 89, 91, P), (56, 92, 92, B),
    (56, 93, 94, P), (56, 95, 97, B), (56, 98, 99, P), (56, 100, 103, B), (56, 104, 106, P),
    (56, 107, 109, B), (56, 110, 111, P), (56, 112, 115, B), (56, 116, 118, P), (56, 119, 122, B),
    (56, 123, 123, P), (56, 124, 128, B), (56, 129, 129, P), (56, 130, 130, B), (56, 131, 133, P),
    (56, 134, 134, B), (56, 135, 135, P), (56, 136, 136, B), (56, 137, 139, P), (56, 140, 140, B),
    (56, 141, 141, P), (56, 142, 142, B), (56, 143, 145, P), (56, 146, 146, B), (56, 147, 147, P),
    (56, 148, 148, B), (56, 149, 151, P), (56, 152, 152, B), (56, 153, 153, P), (56, 154, 154, B),
    (56, 155, 157, P), (56, 158, 158, B), (56, 159, 159, P), (56, 160, 164, B), (56, 165, 166, P),
    (56, 167, 169, B), (56, 170, 171, P), (56, 172, 175, B), (56, 176, 178, P), (56, 179, 182, B),
    (56, 183, 183, P), (56, 184, 187, B), (56, 188, 189, P), (56, 190, 194, B), (56, 195, 195, P),
    (56, 196, 200, B), (56, 201, 202, P), (56, 203, 206, B), (56, 207, 207, P), (56, 208, 208, B),
    (56, 209, 211, P), (56, 212, 212, B), (56, 213, 214, P), (56, 215, 217, B), (56, 218, 223, P),
    (56, 224, 224, B), (56, 225, 226, P), (56, 227, 227, B), (56, 228, 229, P), (56, 230, 230, B),
    (56, 231, 232, P), (56, 233, 233, B), (56, 234, 237, P), (56, 238, 238, B), (56, 239, 241, P),
    (56, 242, 242, B), (56, 243, 243, P), (56, 244, 244, B), (56, 245, 247, P), (56, 248, 248, B),
    (56, 249, 250, P), (56, 251, 253, B), (56, 254, 255, P), (56, 256, 259, B), (56, 260, 262, P),
    (56, 263, 265, B), (56, 266, 267, P), (56, 268, 271, B), (56, 272, 274, P), (56, 275, 278, B),
    (56, 279, 279, P), (56, 280, 284, B), (56, 285, 285, P), (56, 286, 286, B), (56, 287, 289, P),
    (56, 290, 290, B), (56, 291, 291, P), (56, 292, 292, B), (56, 293, 295, P), (56, 296, 296, B),
    (56, 297, 297, P), (56, 298, 298, B), (56, 299, 301, P), (56, 302, 302, B), (56, 303, 303, P),
    (56, 304, 304, B), (56, 305, 307, P), (56, 308, 308, B), (56, 309, 309, P), (56, 310, 310, B),
    (56, 311, 313, P), (56, 314, 314, B), (56, 315, 315, P), (56, 316, 320, B), (56, 321, 322, P),
    (56, 323, 325, B), (56, 326, 327, P), (56, 328, 331, B), (56, 332, 334, P), (56, 335, 338, B),
    (56, 339, 339, P), (56, 340, 343, B), (56, 344, 345, P), (56, 346, 350, B), (56, 351, 351, P),
    (56, 352, 356, B), (56, 357, 358, P), (56, 359, 362, B), (56, 363, 363, P), (56, 364, 364, B),
    (56, 365, 367, P), (56, 368, 368, B), (56, 369, 370, P), (56, 371, 373, B), (56, 374, 379, P),
    (56, 380, 380, B), (56, 381, 382, P), (56, 383, 383, B), (56, 384, 385, P), (56, 386, 386, B),
    (56, 387, 388, P), (56, 389, 389, B), (56, 390, 393, P), (56, 394, 394, B), (56, 395, 397, P),
    (56, 398, 398, B), (56, 399, 399, P), (56, 400, 400, B), (56, 401, 403, P), (56, 404, 404, B),
    (56, 405, 406, P), (56, 407, 409, B), (56, 410, 411, P), (56, 412, 415, B), (56, 416, 418, P),
    (56, 419, 421, B), (56, 422, 423, P), (56, 424, 427, B), (56, 428, 430, P), (56, 431, 434, B),
    (56, 435, 435, P), (56, 436, 440, B), (56, 441, 441, P), (56, 442, 442, B), (56, 443, 445, P),
    (56, 446, 446, B), (56, 447, 447, P), (56, 448, 448, B), (56, 449, 451, P), (56, 452, 452, B),
    (56, 453, 453, P), (56, 454, 454, B), (56, 455, 457, P), (56, 458, 458, B), (56, 459, 459, P),
    (56, 460, 460, B), (56, 461, 463, P), (56, 464, 464, B), (56, 465, 465, P), (56, 466, 466, B),
    (56, 467, 469, P), (56, 470, 470, B), (56, 471, 471, P), (56, 472, 476, B), (56, 477, 478, P),
    (56, 479, 481, B), (56, 482, 483, P), (56, 484, 487, B), (56, 488, 490, P), (56, 491, 494, B),
    (56, 495, 495, P), (56, 496, 499, B), (56, 500, 501, P), (56, 502, 506, B), (56, 507, 507, P),
    (56, 508, 512, B), (56, 513, 514, P), (56, 515, 518, B), (56, 519, 519, P), (56, 520, 520, B),
    (56, 521, 523, P), (56, 524, 524, B), (56, 525, 526, P), (56, 527, 529, B), (56, 530, 535, P),
    (56, 536, 536, B), (56, 537, 538, P), (56, 539, 539, B), (56, 540, 541, P), (56, 542, 542, B),
    (56, 543, 544, P), (56, 545, 545, B), (56, 546, 549, P), (56, 550, 550, B), (56, 551, 553, P),
    (56, 554, 554, B), (56, 555, 555, P), (56, 556, 556, B), (56, 557, 559, P), (56, 560, 560, B),
    (56, 561, 562, P), (56, 563, 565, B), (56, 566, 567, P), (56, 568, 571, B), (56, 572, 574, P),
    (56, 575, 577, B), (56, 578, 579, P), (56, 580, 583, B), (56, 584, 586, P), (56, 587, 590, B),
    (56, 591, 591, P), (56, 592, 596, B), (56, 597, 597, P), (56, 598, 598, B), (56, 599, 601, P),
    (56, 602, 602, B), (56, 603, 603, P), (56, 604, 604, B), (56, 605, 607, P), (56, 608, 608, B),
    (56, 609, 609, P), (56, 610, 610, B), (56, 611, 613, P), (56, 614, 614, B), (56, 615, 615, P),
    (56, 616, 616, B), (56, 617, 619, P), (56, 620, 620, B), (56, 621, 621, P), (56, 622, 622, B),
    (56, 623, 625, P), (56, 626, 626, B), (56, 627, 627, P), (56, 628, 632, B), (56, 633, 634, P),
    (56, 635, 637, B), (56, 638, 639, P), (57, 0, 3, B), (57, 4, 6, P), (57, 7, 10, B), (57, 11, 11, P),
    (57, 12, 15, B), (57, 16, 17, P), (57, 18, 22, B), (57, 23, 23, P), (57, 24, 28, B), (57, 29, 30, P),
    (57, 31, 34, B), (57, 35, 35, P), (57, 36, 36, B), (57, 37, 39, P), (57, 40, 40, B), (57, 41, 42, P),
    (57, 43, 45, B), (57, 46, 51, P), (57, 52, 52, B), (57, 53, 54, P), (57, 55, 55, B), (57, 56, 57, P),
    (57, 58, 58, B), (57, 59, 60, P), (57, 61, 61, B), (57, 62, 65, P), (57, 66, 66, B), (57, 67, 69, P),
    (57, 70, 70, B), (57, 71, 71, P), (57, 72, 72, B), (57, 73, 75, P), (57, 76, 76, B), (57, 77, 78, P),
    (57, 79, 81, B), (57, 82, 83, P), (57, 84, 87, B), (57, 88, 90, P), (57, 91, 93, B), (57, 94, 95, P),
    (57, 96, 99, B), (57, 100, 102, P), (57, 103, 106, B), (57, 107, 107, P), (57, 108, 112, B),
    (57, 113, 113, P), (57, 114, 114, B), (57, 115, 117, P), (57, 118, 118, B), (57, 119, 119, P),
    (57, 120, 120, B), (57, 121, 123, P), (57, 124, 124, B), (57, 125, 125, P), (57, 126, 126, B),
    (57, 127, 129, P), (57, 130, 130, B), (57, 131, 131, P), (57, 132, 132, B), (57, 133, 135, P),
    (57, 136, 136, B), (57, 137, 137, P), (57, 138, 138, B), (57, 139, 141, P), (57, 142, 142, B),
    (57, 143, 143, P), (57, 144, 148, B), (57, 149, 150, P), (57, 151, 153, B), (57, 154, 155, P),
    (57, 156, 159, B), (57, 160, 162, P), (57, 163, 166, B), (57, 167, 167, P), (57, 168, 171, B),
    (57, 172, 173, P), (57, 174, 178, B), (57, 179, 179, P), (57, 180, 184, B), (57, 185, 186, P),
    (57, 187, 190, B), (57, 191, 191, P), (57, 192, 192, B), (57, 193, 195, P), (57, 196, 196, B),
    (57, 197, 198, P), (57, 199, 201, B), (57, 202, 207, P), (57, 208, 208, B), (57, 209, 210, P),
    (57, 211, 211, B), (57, 212, 213, P), (57, 214, 214, B), (57, 215, 216, P), (57, 217, 217, B),
    (57, 218, 221, P), (57, 222, 222, B), (57, 223, 225, P), (57, 226, 226, B), (57, 227, 227, P),
    (57, 228, 228, B), (57, 229, 231, P), (57, 232, 232, B), (57, 233, 234, P), (57, 235, 237, B),
    (57, 238, 239, P), (57, 240, 243, B), (57, 244, 246, P), (57, 247, 249, B), (57, 250, 251, P),
    (57, 252, 255, B), (57, 256, 258, P), (57, 259, 262, B), (57, 263, 263, P), (57, 264, 268, B),
    (57, 269, 269, P), (57, 270, 270, B), (57, 271, 273, P), (57, 274, 274, B), (57, 275, 275, P),
    (57, 276, 276, B), (57, 277, 279, P), (57, 280, 280, B), (57, 281, 281, P), (57, 282, 282, B),
    (57, 283, 285, P), (57, 286, 286, B), (57, 287, 287, P), (57, 288, 288, B), (57, 289, 291, P),
    (57, 292, 292, B), (57, 293, 293, P), (57, 294, 294, B), (57, 295, 297, P), (57, 298, 298, B),
    (57, 299, 299, P), (57, 300, 304, B), (57, 305, 306, P), (57, 307, 309, B), (57, 310, 311, P),
    (57, 312, 315, B), (57, 316, 318, P), (57, 319, 322, B), (57, 323, 323, P), (57, 324, 327, B),
    (57, 328, 329, P), (57, 330, 334, B), (57, 335, 335, P), (57, 336, 340, B), (57, 341, 342, P),
    (57, 343, 346, B), (57, 347, 347, P), (57, 348, 348, B), (57, 349, 351, P), (57, 352, 352, B),
    (57, 353, 354, P), (57, 355, 357, B), (57, 358, 363, P), (57, 364, 364, B), (57, 365, 366, P),
    (57, 367, 367, B), (57, 368, 369, P), (57, 370, 370, B), (57, 371, 372, P), (57, 373, 373, B),
    (57, 374, 377, P), (57, 378, 378, B), (57, 379, 381, P), (57, 382, 382, B), (57, 383, 383, P),
    (57, 384, 384, B), (57, 385, 387, P), (57, 388, 388, B), (57, 389, 390, P), (57, 391, 393, B),
    (57, 394, 395, P), (57, 396, 399, B), (57, 400, 402, P), (57, 403, 405, B), (57, 406, 407, P),
    (57, 408, 411, B), (57, 412, 414, P), (57, 415, 418, B), (57, 419, 419, P), (57, 420, 424, B),
    (57, 425, 425, P), (57, 426, 426, B), (57, 427, 429, P), (57, 430, 430, B), (57, 431, 431, P),
    (57, 432, 432, B), (57, 433, 435, P), (57, 436, 436, B), (57, 437, 437, P), (57, 438, 438, B),
    (57, 439, 441, P), (57, 442, 442, B), (57, 443, 443, P), (57, 444, 444, B), (57, 445, 447, P),
    (57, 448, 448, B), (57, 449, 449, P), (57, 450, 450, B), (57, 451, 453, P), (57, 454, 454, B),
    (57, 455, 455, P), (57, 456, 460, B), (57, 461, 462, P), (57, 463, 465, B), (57, 466, 467, P),
    (57, 468, 471, B), (57, 472, 474, P), (57, 475, 478, B), (57, 479, 479, P), (57, 480, 483, B),
    (57, 484, 485, P), (57, 486, 490, B), (57, 491, 491, P), (57, 492, 496, B), (57, 497, 498, P),
    (57, 499, 502, B), (57, 503, 503, P), (57, 504, 504, B), (57, 505, 507, P), (57, 508, 508, B),
    (57, 509, 510, P), (57, 511, 513, B), (57, 514, 519, P), (57, 520, 520, B), (57, 521, 522, P),
    (57, 523, 523, B), (57, 524, 525, P), (57, 526, 526, B), (57, 527, 528, P), (57, 529, 529, B),
    (57, 530, 533, P), (57, 534, 534, B), (57, 535, 537, P), (57, 538, 538, B), (57, 539, 539, P),
    (57, 540, 540, B), (57, 541, 543, P), (57, 544, 544, B), (57, 545, 546, P), (57, 547, 549, B),
    (57, 550, 551, P), (57, 552, 555, B), (57, 556, 558, P), (57, 559, 561, B), (57, 562, 563, P),
    (57, 564, 567, B), (57, 568, 570, P), (57, 571, 574, B), (57, 575, 575, P), (57, 576, 580, B),
    (57, 581, 581, P), (57, 582, 582, B), (57, 583, 585, P), (57, 586, 586, B), (57, 587, 587, P),
    (57, 588, 588, B), (57, 589, 591, P), (57, 592, 592, B), (57, 593, 593, P), (57, 594, 594, B),
    (57, 595, 597, P), (57, 598, 598, B), (57, 599, 599, P), (57, 600, 600, B), (57, 601, 603, P),
    (57, 604, 604, B), (57, 605, 605, P), (57, 606, 606, B), (57, 607, 609, P), (57, 610, 610, B),
    (57, 611, 611, P), (57, 612, 616, B), (57, 617, 618, P), (57, 619, 621, B), (57, 622, 623, P),
    (57, 624, 627, B), (57, 628, 630, P), (57, 631, 634, B), (57, 635, 635, P), (57, 636, 639, B),
    (58, 0, 1, P), (58, 2, 6, B), (58, 7, 7, P), (58, 8, 12, B), (58, 13, 14, P), (58, 15, 18, B),
    (58, 19, 19, P), (58, 20, 20, B), (58, 21, 23, P), (58, 24, 24, B), (58, 25, 26, P), (58, 27, 29, B),
    (58, 30, 35, P), (58, 36, 36, B), (58, 37, 38, P), (58, 39, 39, B), (58, 40, 41, P), (58, 42, 42, B),
    (58, 43, 44, P), (58, 45, 45, B), (58, 46, 49, P), (58, 50, 50, B), (58, 51, 53, P), (58, 54, 54, B),
    (58, 55, 55, P), (58, 56, 56, B), (58, 57, 59, P), (58, 60, 60, B), (58, 61, 62, P), (58, 63, 65, B),
    (58, 66, 67, P), (58, 68, 71, B), (58, 72, 74, P), (58, 75, 77, B), (58, 78, 79, P), (58, 80, 83, B),
    (58, 84, 86, P), (58, 87, 90, B), (58, 91, 91, P), (58, 92, 96, B), (58, 97, 97, P), (58, 98, 98, B),
    (58, 99, 101, P), (58, 102, 102, B), (58, 103, 103, P), (58, 104, 104, B), (58, 105, 107, P),
    (58, 108, 108, B), (58, 109, 109, P), (58, 110, 110, B), (58, 111, 113, P), (58, 114, 114, B),
    (58, 115, 115, P), (58, 116, 116, B), (58, 117, 119, P), (58, 120, 120, B), (58, 121, 121, P),
    (58, 122, 122, B), (58, 123, 125, P), (58, 126, 126, B), (58, 127, 127, P), (58, 128, 132, B),
    (58, 133, 134, P), (58, 135, 137, B), (58, 138, 139, P), (58, 140, 143, B), (58, 144, 146, P),
    (58, 147, 150, B), (58, 151, 151, P), (58, 152, 155, B), (58, 156, 157, P), (58, 158, 162, B),
    (58, 163, 163, P), (58, 164, 168, B), (58, 169, 170, P), (58, 171, 174, B), (58, 175, 175, P),
    (58, 176, 176, B), (58, 177, 179, P), (58, 180, 180, B), (58, 181, 182, P), (58, 183, 185, B),
    (58, 186, 191, P), (58, 192, 192, B), (58, 193, 194, P), (58, 195, 195, B), (58, 196, 197, P),
    (58, 198, 198, B), (58, 199, 200, P), (58, 201, 201, B), (58, 202, 205, P), (58, 206, 206, B),
    (58, 207, 209, P), (58, 210, 210, B), (58, 211, 211, P), (58, 212, 212, B), (58, 213, 215, P),
    (58, 216, 216, B), (58, 217, 218, P), (58, 219, 221, B), (58, 222, 223, P), (58, 224, 227, B),
    (58, 228, 230, P), (58, 231, 233, B), (58, 234, 235, P), (58, 236, 239, B), (58, 240, 242, P),
    (58, 243, 246, B), (58, 247, 247, P), (58, 248, 252, B), (58, 253, 253, P), (58, 254, 254, B),
    (58, 255, 257, P), (58, 258, 258, B), (58, 259, 259, P), (58, 260, 260, B), (58, 261, 263, P),
    (58, 264, 264, B), (58, 265, 265, P), (58, 266, 266, B), (58, 267, 269, P), (58, 270, 270, B),
    (58, 271, 271, P), (58, 272, 272, B), (58, 273, 275, P), (58, 276, 276, B), (58, 277, 277, P),
    (58, 278, 278, B), (58, 279, 281, P), (58, 282, 282, B), (58, 283, 283, P), (58, 284, 288, B),
    (58, 289, 290, P), (58, 291, 293, B), (58, 294, 295, P), (58, 296, 299, B), (58, 300, 302, P),
    (58, 303, 306, B), (58, 307, 307, P), (58, 308, 311, B), (58, 312, 313, P), (58, 314, 318, B),
    (58, 319, 319, P), (58, 320, 324, B), (58, 325, 326, P), (58, 327, 330, B), (58, 331, 331, P),
    (58, 332, 332, B), (58, 333, 335, P), (58, 336, 336, B), (58, 337, 338, P), (58, 339, 341, B),
    (58, 342, 347, P), (58, 348, 348, B), (58, 349, 350, P), (58, 351, 351, B), (58, 352, 353, P),
    (58, 354, 354, B), (58, 355, 356, P), (58, 357, 357, B), (58, 358, 361, P), (58, 362, 362, B),
    (58, 363, 365, P), (58, 366, 366, B), (58, 367, 367, P), (58, 368, 368, B), (58, 369, 371, P),
    (58, 372, 372, B), (58, 373, 374, P), (58, 375, 377, B), (58, 378, 379, P), (58, 380, 383, B),
    (58, 384, 386, P), (58, 387, 389, B), (58, 390, 391, P), (58, 392, 395, B), (58, 396, 398, P),
    (58, 399, 402, B), (58, 403, 403, P), (58, 404, 408, B), (58, 409, 409, P), (58, 410, 410, B),
    (58, 411, 413, P), (58, 414, 414, B), (58, 415, 415, P), (58, 416, 416, B), (58, 417, 419, P),
    (58, 420, 420, B), (58, 421, 421, P), (58, 422, 422, B), (58, 423, 425, P), (58, 426, 426, B),
    (58, 427, 427, P), (58, 428, 428, B), (58, 429, 431, P), (58, 432, 432, B), (58, 433, 433, P),
    (58, 434, 434, B), (58, 435, 437, P), (58, 438, 438, B), (58, 439, 439, P), (58, 440, 444, B),
    (58, 445, 446, P), (58, 447, 449, B), (58, 450, 451, P), (58, 452, 455, B), (58, 456, 458, P),
    (58, 459, 462, B), (58, 463, 463, P), (58, 464, 467, B), (58, 468, 469, P), (58, 470, 474, B),
    (58, 475, 475, P), (58, 476, 480, B), (58, 481, 482, P), (58, 483, 486, B), (58, 487, 487, P),
    (58, 488, 488, B), (58, 489, 491, P), (58, 492, 492, B), (58, 493, 494, P), (58, 495, 497, B),
    (58, 498, 503, P), (58, 504, 504, B), (58, 505, 506, P), (58, 507, 507, B), (58, 508, 509, P),
    (58, 510, 510, B), (58, 511, 512, P), (58, 513, 513, B), (58, 514, 517, P), (58, 518, 518, B),
    (58, 519, 521, P), (58, 522, 522, B), (58, 523, 523, P), (58, 524, 524, B), (58, 525, 527, P),
    (58, 528, 528, B), (58, 529, 530, P), (58, 531, 533, B), (58, 534, 535, P), (58, 536, 539, B),
    (58, 540, 542, P), (58, 543, 545, B), (58, 546, 547, P), (58, 548, 551, B), (58, 552, 554, P),
    (58, 555, 558, B), (58, 559, 559, P), (58, 560, 564, B), (58, 565, 565, P), (58, 566, 566, B),
    (58, 567, 569, P), (58, 570, 570, B), (58, 571, 571, P), (58, 572, 572, B), (58, 573, 575, P),
    (58, 576, 576, B), (58, 577, 577, P), (58, 578, 578, B), (58, 579, 581, P), (58, 582, 582, B),
    (58, 583, 583, P), (58, 584, 584, B), (58, 585, 587, P), (58, 588, 588, B), (58, 589, 589, P),
    (58, 590, 590, B), (58, 591, 593, P), (58, 594, 594, B), (58, 595, 595, P), (58, 596, 600, B),
    (58, 601, 602, P), (58, 603, 605, B), (58, 606, 607, P), (58, 608, 611, B), (58, 612, 614, P),
    (58, 615, 618, B), (58, 619, 619, P), (58, 620, 623, B), (58, 624, 625, P), (58, 626, 630, B),
    (58, 631, 631, P), (58, 632, 636, B), (58, 637, 638, P), (58, 639, 639, B), (59, 0, 2, B), (59, 3, 3, P),
    (59, 4, 4, B), (59, 5, 7, P), (59, 8, 8, B), (59, 9, 10, P), (59, 11, 13, B), (59, 14, 19, P),
    (59, 20, 20, B), (59, 21, 22, P), (59, 23, 23, B), (59, 24, 25, P), (59, 26, 26, B), (59, 27, 28, P),
    (59, 29, 29, B), (59, 30, 33, P), (59, 34, 34, B), (59, 35, 37, P), (59, 38, 38, B), (59, 39, 39, P),
    (59, 40, 40, B), (59, 41, 43, P), (59, 44, 44, B), (59, 45, 46, P), (59, 47, 49, B), (59, 50, 51, P),
    (59, 52, 55, B), (59, 56, 58, P), (59, 59, 61, B), (59, 62, 63, P), (59, 64, 67, B), (59, 68, 70, P),
    (59, 71, 74, B), (59, 75, 75, P), (59, 76, 80, B), (59, 81, 81, P), (59, 82, 82, B), (59, 83, 85, P),
    (59, 86, 86, B), (59, 87, 87, P), (59, 88, 88, B), (59, 89, 91, P), (59, 92, 92, B), (59, 93, 93, P),
    (59, 94, 94, B), (59, 95, 97, P), (59, 98, 98, B), (59, 99, 99, P), (59, 100, 100, B), (59, 101, 103, P),
    (59, 104, 104, B), (59, 105, 105, P), (59, 106, 106, B), (59, 107, 109, P), (59, 110, 110, B),
    (59, 111, 111, P), (59, 112, 116, B), (59, 117, 118, P), (59, 119, 121, B), (59, 122, 123, P),
    (59, 124, 127, B), (59, 128, 130, P), (59, 131, 134, B), (59, 135, 135, P), (59, 136, 139, B),
    (59, 140, 141, P), (59, 142, 146, B), (59, 147, 147, P), (59, 148, 152, B), (59, 153, 154, P),
    (59, 155, 158, B), (59, 159, 159, P), (59, 160, 160, B), (59, 161, 163, P), (59, 164, 164, B),
    (59, 165, 166, P), (59, 167, 169, B), (59, 170, 175, P), (59, 176, 176, B), (59, 177, 178, P),
    (59, 179, 179, B), (59, 180, 181, P), (59, 182, 182, B), (59, 183, 184, P), (59, 185, 185, B),
    (59, 186, 189, P), (59, 190, 190, B), (59, 191, 193, P), (59, 194, 194, B), (59, 195, 195, P),
    (59, 196, 196, B), (59, 197, 199, P), (59, 200, 200, B), (59, 201, 202, P), (59, 203, 205, B),
    (59, 206, 207, P), (59, 208, 211, B), (59, 212, 214, P), (59, 215, 217, B), (59, 218, 219, P),
    (59, 220, 223, B), (59, 224, 226, P), (59, 227, 230, B), (59, 231, 231, P), (59, 232, 236, B),
    (59, 237, 237, P), (59, 238, 238, B), (59, 239, 241, P), (59, 242, 242, B), (59, 243, 243, P),
    (59, 244, 244, B), (59, 245, 247, P), (59, 248, 248, B), (59, 249, 249, P), (59, 250, 250, B),
    (59, 251, 253, P), (59, 254, 254, B), (59, 255, 255, P), (59, 256, 256, B), (59, 257, 259, P),
    (59, 260, 260, B), (59, 261, 261, P), (59, 262, 262, B), (59, 263, 265, P), (59, 266, 266, B),
    (59, 267, 267, P), (59, 268, 272, B), (59, 273, 274, P), (59, 275, 277, B), (59, 278, 279, P),
    (59, 280, 283, B), (59, 284, 286, P), (59, 287, 290, B), (59, 291, 291, P), (59, 292, 295, B),
    (59, 296, 297, P), (59, 298, 302, B), (59, 303, 303, P), (59, 304, 308, B), (59, 309, 310, P),
    (59, 311, 314, B), (59, 315, 315, P), (59, 316, 316, B), (59, 317, 319, P), (59, 320, 320, B),
    (59, 321, 322, P), (59, 323, 325, B), (59, 326, 331, P), (59, 332, 332, B), (59, 333, 334, P),
    (59, 335, 335, B), (59, 336, 337, P), (59, 338, 338, B), (59, 339, 340, P), (59, 341, 341, B),
    (59, 342, 345, P), (59, 346, 346, B), (59, 347, 349, P), (59, 350, 350, B), (59, 351, 351, P),
    (59, 352, 352, B), (59, 353, 355, P), (59, 356, 356, B), (59, 357, 358, P), (59, 359, 361, B),
    (59, 362, 363, P), (59, 364, 367, B), (59, 368, 370, P), (59, 371, 373, B), (59, 374, 375, P),
    (59, 376, 379, B), (59, 380, 382, P), (59, 383, 386, B), (59, 387, 387, P), (59, 388, 392, B),
    (59, 393, 393, P), (59, 394, 394, B), (59, 395, 397, P), (59, 398, 398, B), (59, 399, 399, P),
    (59, 400, 400, B), (59, 401, 403, P), (59, 404, 404, B), (59, 405, 405, P), (59, 406, 406, B),
    (59, 407, 409, P), (59, 410, 410, B), (59, 411, 411, P), (59, 412, 412, B), (59, 413, 415, P),
    (59, 416, 416, B), (59, 417, 417, P), (59, 418, 418, B), (59, 419, 421, P), (59, 422, 422, B),
    (59, 423, 423, P), (59, 424, 428, B), (59, 429, 430, P), (59, 431, 433, B), (59, 434, 435, P),
    (59, 436, 439, B), (59, 440, 442, P), (59, 443, 446, B), (59, 447, 447, P), (59, 448, 451, B),
    (59, 452, 453, P), (59, 454, 458, B), (59, 459, 459, P), (59, 460, 464, B), (59, 465, 466, P),
    (59, 467, 470, B), (59, 471, 471, P), (59, 472, 472, B), (59, 473, 475, P), (59, 476, 476, B),
    (59, 477, 478, P), (59, 479, 481, B), (59, 482, 487, P), (59, 488, 488, B), (59, 489, 490, P),
    (59, 491, 491, B), (59, 492, 493, P), (59, 494, 494, B), (59, 495, 496, P), (59, 497, 497, B),
    (59, 498, 501, P), (59, 502, 502, B), (59, 503, 505, P), (59, 506, 506, B), (59, 507, 507, P),
    (59, 508, 508, B), (59, 509, 511, P), (59, 512, 512, B), (59, 513, 514, P), (59, 515, 517, B),
    (59, 518, 519, P), (59, 520, 523, B), (59, 524, 526, P), (59, 527, 529, B), (59, 530, 531, P),
    (59, 532, 535, B), (59, 536, 538, P), (59, 539, 542, B), (59, 543, 543, P), (59, 544, 548, B),
    (59, 549, 549, P), (59, 550, 550, B), (59, 551, 553, P), (59, 554, 554, B), (59, 555, 555, P),
    (59, 556, 556, B), (59, 557, 559, P), (59, 560, 560, B), (59, 561, 561, P), (59, 562, 562, B),
    (59, 563, 565, P), (59, 566, 566, B), (59, 567, 567, P), (59, 568, 568, B), (59, 569, 571, P),
    (59, 572, 572, B), (59, 573, 573, P), (59, 574, 574, B), (59, 575, 577, P), (59, 578, 578, B),
    (59, 579, 579, P), (59, 580, 584, B), (59, 585, 586, P), (59, 587, 589, B), (59, 590, 591, P),
    (59, 592, 595, B), (59, 596, 598, P), (59, 599, 602, B), (59, 603, 603, P), (59, 604, 607, B),
    (59, 608, 609, P), (59, 610, 614, B), (59, 615, 615, P), (59, 616, 620, B), (59, 621, 622, P),
    (59, 623, 626, B), (59, 627, 627, P), (59, 628, 628, B), (59, 629, 631, P), (59, 632, 632, B),
    (59, 633, 634, P), (59, 635, 637, B), (59, 638, 639, P), (60, 0, 3, P), (60, 4, 4, B), (60, 5, 6, P),
    (60, 7, 7, B), (60, 8, 9, P), (60, 10, 10, B), (60, 11, 12, P), (60, 13, 13, B), (60, 14, 17, P),
    (60, 18, 18, B), (60, 19, 21, P), (60, 22, 22, B), (60, 23, 23, P), (60, 24, 24, B), (60, 25, 27, P),
    (60, 28, 28, B), (60, 29, 30, P), (60, 31, 33, B), (60, 34, 35, P), (60, 36, 39, B), (60, 40, 42, P),
    (60, 43, 45, B), (60, 46, 47, P), (60, 48, 51, B), (60, 52, 54, P), (60, 55, 58, B), (60, 59, 59, P),
    (60, 60, 64, B), (60, 65, 65, P), (60, 66, 66, B), (60, 67, 69, P), (60, 70, 70, B), (60, 71, 71, P),
    (60, 72, 72, B), (60, 73, 75, P), (60, 76, 76, B), (60, 77, 77, P), (60, 78, 78, B), (60, 79, 81, P),
    (60, 82, 82, B), (60, 83, 83, P), (60, 84, 84, B), (60, 85, 87, P), (60, 88, 88, B), (60, 89, 89, P),
    (60, 90, 90, B), (60, 91, 93, P), (60, 94, 94, B), (60, 95, 95, P), (60, 96, 100, B), (60, 101, 102, P),
    (60, 103, 105, B), (60, 106, 107, P), (60, 108, 111, B), (60, 112, 114, P), (60, 115, 118, B),
    (60, 119, 119, P), (60, 120, 123, B), (60, 124, 125, P), (60, 126, 130, B), (60, 131, 131, P),
    (60, 132, 136, B), (60, 137, 138, P), (60, 139, 142, B), (60, 143, 143, P), (60, 144, 144, B),
    (60, 145, 147, P), (60, 148, 148, B), (60, 149, 150, P), (60, 151, 153, B), (60, 154, 159, P),
    (60, 160, 160, B), (60, 161, 162, P), (60, 163, 163, B), (60, 164, 165, P), (60, 166, 166, B),
    (60, 167, 168, P), (60, 169, 169, B), (60, 170, 173, P), (60, 174, 174, B), (60, 175, 177, P),
    (60, 178, 178, B), (60, 179, 179, P), (60, 180, 180, B), (60, 181, 183, P), (60, 184, 184, B),
    (60, 185, 186, P), (60, 187, 189, B), (60, 190, 191, P), (60, 192, 195, B), (60, 196, 198, P),
    (60, 199, 201, B), (60, 202, 203, P), (60, 204, 207, B), (60, 208, 210, P), (60, 211, 214, B),
    (60, 215, 215, P), (60, 216, 220, B), (60, 221, 221, P), (60, 222, 222, B), (60, 223, 225, P),
    (60, 226, 226, B), (60, 227, 227, P), (60, 228, 228, B), (60, 229, 231, P), (60, 232, 232, B),
    (60, 233, 233, P), (60, 234, 234, B), (60, 235, 237, P), (60, 238, 238, B), (60, 239, 239, P),
    (60, 240, 240, B), (60, 241, 243, P), (60, 244, 244, B), (60, 245, 245, P), (60, 246, 246, B),
    (60, 247, 249, P), (60, 250, 250, B), (60, 251, 251, P), (60, 252, 256, B), (60, 257, 258, P),
    (60, 259, 261, B), (60, 262, 263, P), (60, 264, 267, B), (60, 268, 270, P), (60, 271, 274, B),
    (60, 275, 275, P), (60, 276, 279, B), (60, 280, 281, P), (60, 282, 286, B), (60, 287, 287, P),
    (60, 288, 292, B), (60, 293, 294, P), (60, 295, 298, B), (60, 299, 299, P), (60, 300, 300, B),
    (60, 301, 303, P), (60, 304, 304, B), (60, 305, 306, P), (60, 307, 309, B), (60, 310, 315, P),
    (60, 316, 316, B), (60, 317, 318, P), (60, 319, 319, B), (60, 320, 321, P), (60, 322, 322, B),
    (60, 323, 324, P), (60, 325, 325, B), (60, 326, 329, P), (60, 330, 330, B), (60, 331, 333, P),
    (60, 334, 334, B), (60, 335, 335, P), (60, 336, 336, B), (60, 337, 339, P), (60, 340, 340, B),
    (60, 341, 342, P), (60, 343, 345, B), (60, 346, 347, P), (60, 348, 351, B), (60, 352, 354, P),
    (60, 355, 357, B), (60, 358, 359, P), (60, 360, 363, B), (60, 364, 366, P), (60, 367, 370, B),
    (60, 371, 371, P), (60, 372, 376, B), (60, 377, 377, P), (60, 378, 378, B), (60, 379, 381, P),
    (60, 382, 382, B), (60, 383, 383, P), (60, 384, 384, B), (60, 385, 387, P), (60, 388, 388, B),
    (60, 389, 389, P), (60, 390, 390, B), (60, 391, 393, P), (60, 394, 394, B), (60, 395, 395, P),
    (60, 396, 396, B), (60, 397, 399, P), (60, 400, 400, B), (60, 401, 401, P), (60, 402, 402, B),
    (60, 403, 405, P), (60, 406, 406, B), (60, 407, 407, P), (60, 408, 412, B), (60, 413, 414, P),
    (60, 415, 417, B), (60, 418, 419, P), (60, 420, 423, B), (60, 424, 426, P), (60, 427, 430, B),
    (60, 431, 431, P), (60, 432, 435, B), (60, 436, 437, P), (60, 438, 442, B), (60, 443, 443, P),
    (60, 444, 448, B), (60, 449, 450, P), (60, 451, 454, B), (60, 455, 455, P), (60, 456, 456, B),
    (60, 457, 459, P), (60, 460, 460, B), (60, 461, 462, P), (60, 463, 465, B), (60, 466, 471, P),
    (60, 472, 472, B), (60, 473, 474, P), (60, 475, 475, B), (60, 476, 477, P), (60, 478, 478, B),
    (60, 479, 480, P), (60, 481, 481, B), (60, 482, 485, P), (60, 486, 486, B), (60, 487, 489, P),
    (60, 490, 490, B), (60, 491, 491, P), (60, 492, 492, B), (60, 493, 495, P), (60, 496, 496, B),
    (60, 497, 498, P), (60, 499, 501, B), (60, 502, 503, P), (60, 504, 507, B), (60, 508, 510, P),
    (60, 511, 513, B), (60, 514, 515, P), (60, 516, 519, B), (60, 520, 522, P), (60, 523, 526, B),
    (60, 527, 527, P), (60, 528, 532, B), (60, 533, 533, P), (60, 534, 534, B), (60, 535, 537, P),
    (60, 538, 538, B), (60, 539, 539, P), (60, 540, 540, B), (60, 541, 543, P), (60, 544, 544, B),
    (60, 545, 545, P), (60, 546, 546, B), (60, 547, 549, P), (60, 550, 550, B), (60, 551, 551, P),
    (60, 552, 552, B), (60, 553, 555, P), (60, 556, 556, B), (60, 557, 557, P), (60, 558, 558, B),
    (60, 559, 561, P), (60, 562, 562, B), (60, 563, 563, P), (60, 564, 568, B), (60, 569, 570, P),
    (60, 571, 573, B), (60, 574, 575, P), (60, 576, 579, B), (60, 580, 582, P), (60, 583, 586, B),
    (60, 587, 587, P), (60, 588, 591, B), (60, 592, 593, P), (60, 594, 598, B), (60, 599, 599, P),
    (60, 600, 604, B), (60, 605, 606, P), (60, 607, 610, B), (60, 611, 611, P), (60, 612, 612, B),
    (60, 613, 615, P), (60, 616, 616, B), (60, 617, 618, P), (60, 619, 621, B), (60, 622, 627, P),
    (60, 628, 628, B), (60, 629, 630, P), (60, 631, 631, B), (60, 632, 633, P), (60, 634, 634, B),
    (60, 635, 636, P), (60, 637, 637, B), (60, 638, 639, P), (61, 0, 1, P), (61, 2, 2, B), (61, 3, 5, P),
    (61, 6, 6, B), (61, 7, 7, P), (61, 8, 8, B), (61, 9, 11, P), (61, 12, 12, B), (61, 13, 14, P),
    (61, 15, 17, B), (61, 18, 19, P), (61, 20, 23, B), (61, 24, 26, P), (61, 27, 29, B), (61, 30, 31, P),
    (61, 32, 35, B), (61, 36, 38, P), (61, 39, 42, B), (61, 43, 43, P), (61, 44, 48, B), (61, 49, 49, P),
    (61, 50, 50, B), (61, 51, 53, P), (61, 54, 54, B), (61, 55, 55, P), (61, 56, 56, B), (61, 57, 59, P),
    (61, 60, 60, B), (61, 61, 61, P), (61, 62, 62, B), (61, 63, 65, P), (61, 66, 66, B), (61, 67, 67, P),
    (61, 68, 68, B), (61, 69, 71, P), (61, 72, 72, B), (61, 73, 73, P), (61, 74, 74, B), (61, 75, 77, P),
    (61, 78, 78, B), (61, 79, 79, P), (61, 80, 84, B), (61, 85, 86, P), (61, 87, 89, B), (61, 90, 91, P),
    (61, 92, 95, B), (61, 96, 98, P), (61, 99, 102, B), (61, 103, 103, P), (61, 104, 107, B),
    (61, 108, 109, P), (61, 110, 114, B), (61, 115, 115, P), (61, 116, 120, B), (61, 121, 122, P),
    (61, 123, 126, B), (61, 127, 127, P), (61, 128, 128, B), (61, 129, 131, P), (61, 132, 132, B),
    (61, 133, 134, P), (61, 135, 137, B), (61, 138, 143, P), (61, 144, 144, B), (61, 145, 146, P),
    (61, 147, 147, B), (61, 148, 149, P), (61, 150, 150, B), (61, 151, 152, P), (61, 153, 153, B),
    (61, 154, 157, P), (61, 158, 158, B), (61, 159, 161, P), (61, 162, 162, B), (61, 163, 163, P),
    (61, 164, 164, B), (61, 165, 167, P), (61, 168, 168, B), (61, 169, 170, P), (61, 171, 173, B),
    (61, 174, 175, P), (61, 176, 179, B), (61, 180, 182, P), (61, 183, 185, B), (61, 186, 187, P),
    (61, 188, 191, B), (61, 192, 194, P), (61, 195, 198, B), (61, 199, 199, P), (61, 200, 204, B),
    (61, 205, 205, P), (61, 206, 206, B), (61, 207, 209, P), (61, 210, 210, B), (61, 211, 211, P),
    (61, 212, 212, B), (61, 213, 215, P), (61, 216, 216, B), (61, 217, 217, P), (61, 218, 218, B),
    (61, 219, 221, P), (61, 222, 222, B), (61, 223, 223, P), (61, 224, 224, B), (61, 225, 227, P),
    (61, 228, 228, B), (61, 229, 229, P), (61, 230, 230, B), (61, 231, 233, P), (61, 234, 234, B),
    (61, 235, 235, P), (61, 236, 240, B), (61, 241, 242, P), (61, 243, 245, B), (61, 246, 247, P),
    (61, 248, 251, B), (61, 252, 254, P), (61, 255, 258, B), (61, 259, 259, P), (61, 260, 263, B),
    (61, 264, 265, P), (61, 266, 270, B), (61, 271, 271, P), (61, 272, 276, B), (61, 277, 278, P),
    (61, 279, 282, B), (61, 283, 283, P), (61, 284, 284, B), (61, 285, 287, P), (61, 288, 288, B),
    (61, 289, 290, P), (61, 291, 293, B), (61, 294, 299, P), (61, 300, 300, B), (61, 301, 302, P),
    (61, 303, 303, B), (61, 304, 305, P), (61, 306, 306, B), (61, 307, 308, P), (61, 309, 309, B),
    (61, 310, 313, P), (61, 314, 314, B), (61, 315, 317, P), (61, 318, 318, B), (61, 319, 319, P),
    (61, 320, 320, B), (61, 321, 323, P), (61, 324, 324, B), (61, 325, 326, P), (61, 327, 329, B),
    (61, 330, 331, P), (61, 332, 335, B), (61, 336, 338, P), (61, 339, 341, B), (61, 342, 343, P),
    (61, 344, 347, B), (61, 348, 350, P), (61, 351, 354, B), (61, 355, 355, P), (61, 356, 360, B),
    (61, 361, 361, P), (61, 362, 362, B), (61, 363, 365, P), (61, 366, 366, B), (61, 367, 367, P),
    (61, 368, 368, B), (61, 369, 371, P), (61, 372, 372, B), (61, 373, 373, P), (61, 374, 374, B),
    (61, 375, 377, P), (61, 378, 378, B), (61, 379, 379, P), (61, 380, 380, B), (61, 381, 383, P),
    (61, 384, 384, B), (61, 385, 385, P), (61, 386, 386, B), (61, 387, 389, P), (61, 390, 390, B),
    (61, 391, 391, P), (61, 392, 396, B), (61, 397, 398, P), (61, 399, 401, B), (61, 402, 403, P),
    (61, 404, 407, B), (61, 408, 410, P), (61, 411, 414, B), (61, 415, 415, P), (61, 416, 419, B),
    (61, 420, 421, P), (61, 422, 426, B), (61, 427, 427, P), (61, 428, 432, B), (61, 433, 434, P),
    (61, 435, 438, B), (61, 439, 439, P), (61, 440, 440, B), (61, 441, 443, P), (61, 444, 444, B),
    (61, 445, 446, P), (61, 447, 449, B), (61, 450, 455, P), (61, 456, 456, B), (61, 457, 458, P),
    (61, 459, 459, B), (61, 460, 461, P), (61, 462, 462, B), (61, 463, 464, P), (61, 465, 465, B),
    (61, 466, 469, P), (61, 470, 470, B), (61, 471, 473, P), (61, 474, 474, B), (61, 475, 475, P),
    (61, 476, 476, B), (61, 477, 479, P), (61, 480, 480, B), (61, 481, 482, P), (61, 483, 485, B),
    (61, 486, 487, P), (61, 488, 491, B), (61, 492, 494, P), (61, 495, 497, B), (61, 498, 499, P),
    (61, 500, 503, B), (61, 504, 506, P), (61, 507, 510, B), (61, 511, 511, P), (61, 512, 516, B),
    (61, 517, 517, P), (61, 518, 518, B), (61, 519, 521, P), (61, 522, 522, B), (61, 523, 523, P),
    (61, 524, 524, B), (61, 525, 527, P), (61, 528, 528, B), (61, 529, 529, P), (61, 530, 530, B),
    (61, 531, 533, P), (61, 534, 534, B), (61, 535, 535, P), (61, 536, 536, B), (61, 537, 539, P),
    (61, 540, 540, B), (61, 541, 541, P), (61, 542, 542, B), (61, 543, 545, P), (61, 546, 546, B),
    (61, 547, 547, P), (61, 548, 552, B), (61, 553, 554, P), (61, 555, 557, B), (61, 558, 559, P),
    (61, 560, 563, B), (61, 564, 566, P), (61, 567, 570, B), (61, 571, 571, P), (61, 572, 575, B),
    (61, 576, 577, P), (61, 578, 582, B), (61, 583, 583, P), (61, 584, 588, B), (61, 589, 590, P),
    (61, 591, 594, B), (61, 595, 595, P), (61, 596, 596, B), (61, 597, 599, P), (61, 600, 600, B),
    (61, 601, 602, P), (61, 603, 605, B), (61, 606, 611, P), (61, 612, 612, B), (61, 613, 614, P),
    (61, 615, 615, B), (61, 616, 617, P), (61, 618, 618, B), (61, 619, 620, P), (61, 621, 621, B),
    (61, 622, 625, P), (61, 626, 626, B), (61, 627, 629, P), (61, 630, 630, B), (61, 631, 631, P),
    (61, 632, 632, B), (61, 633, 635, P), (61, 636, 636, B), (61, 637, 638, P), (61, 639, 639, B),
    (62, 0, 1, B), (62, 2, 3, P), (62, 4, 7, B), (62, 8, 10, P), (62, 11, 13, B), (62, 14, 15, P),
    (62, 16, 19, B), (62, 20, 22, P), (62, 23, 26, B), (62, 27, 27, P), (62, 28, 32, B), (62, 33, 33, P),
    (62, 34, 34, B), (62, 35, 37, P), (62, 38, 38, B), (62, 39, 39, P), (62, 40, 40, B), (62, 41, 43, P),
    (62, 44, 44, B), (62, 45, 45, P), (62, 46, 46, B), (62, 47, 49, P), (62, 50, 50, B), (62, 51, 51, P),
    (62, 52, 52, B), (62, 53, 55, P), (62, 56, 56, B), (62, 57, 57, P), (62, 58, 58, B), (62, 59, 61, P),
    (62, 62, 62, B), (62, 63, 63, P), (62, 64, 68, B), (62, 69, 70, P), (62, 71, 73, B), (62, 74, 75, P),
    (62, 76, 79, B), (62, 80, 82, P), (62, 83, 86, B), (62, 87, 87, P), (62, 88, 91, B), (62, 92, 93, P),
    (62, 94, 98, B), (62, 99, 99, P), (62, 100, 104, B), (62, 105, 106, P), (62, 107, 110, B),
    (62, 111, 111, P), (62, 112, 112, B), (62, 113, 115, P), (62, 116, 116, B), (62, 117, 118, P),
    (62, 119, 121, B), (62, 122, 127, P), (62, 128, 128, B), (62, 129, 130, P), (62, 131, 131, B),
    (62, 132, 133, P), (62, 134, 134, B), (62, 135, 136, P), (62, 137, 137, B), (62, 138, 141, P),
    (62, 142, 142, B), (62, 143, 145, P), (62, 146, 146, B), (62, 147, 147, P), (62, 148, 148, B),
    (62, 149, 151, P), (62, 152, 152, B), (62, 153, 154, P), (62, 155, 157, B), (62, 158, 159, P),
    (62, 160, 163, B), (62, 164, 166, P), (62, 167, 169, B), (62, 170, 171, P), (62, 172, 175, B),
    (62, 176, 178, P), (62, 179, 182, B), (62, 183, 183, P), (62, 184, 188, B), (62, 189, 189, P),
    (62, 190, 190, B), (62, 191, 193, P), (62, 194, 194, B), (62, 195, 195, P), (62, 196, 196, B),
    (62, 197, 199, P), (62, 200, 200, B), (62, 201, 201, P), (62, 202, 202, B), (62, 203, 205, P),
    (62, 206, 206, B), (62, 207, 207, P), (62, 208, 208, B), (62, 209, 211, P), (62, 212, 212, B),
    (62, 213, 213, P), (62, 214, 214, B), (62, 215, 217, P), (62, 218, 218, B), (62, 219, 219, P),
    (62, 220, 224, B), (62, 225, 226, P), (62, 227, 229, B), (62, 230, 231, P), (62, 232, 235, B),
    (62, 236, 238, P), (62, 239, 242, B), (62, 243, 243, P), (62, 244, 247, B), (62, 248, 249, P),
    (62, 250, 254, B), (62, 255, 255, P), (62, 256, 260, B), (62, 261, 262, P), (62, 263, 266, B),
    (62, 267, 267, P), (62, 268, 268, B), (62, 269, 271, P), (62, 272, 272, B), (62, 273, 274, P),
    (62, 275, 277, B), (62, 278, 283, P), (62, 284, 284, B), (62, 285, 286, P), (62, 287, 287, B),
    (62, 288, 289, P), (62, 290, 290, B), (62, 291, 292, P), (62, 293, 293, B), (62, 294, 297, P),
    (62, 298, 298, B), (62, 299, 301, P), (62, 302, 302, B), (62, 303, 303, P), (62, 304, 304, B),
    (62, 305, 307, P), (62, 308, 308, B), (62, 309, 310, P), (62, 311, 313, B), (62, 314, 315, P),
    (62, 316, 319, B), (62, 320, 322, P), (62, 323, 325, B), (62, 326, 327, P), (62, 328, 331, B),
    (62, 332, 334, P), (62, 335, 338, B), (62, 339, 339, P), (62, 340, 344, B), (62, 345, 345, P),
    (62, 346, 346, B), (62, 347, 349, P), (62, 350, 350, B), (62, 351, 351, P), (62, 352, 352, B),
    (62, 353, 355, P), (62, 356, 356, B), (62, 357, 357, P), (62, 358, 358, B), (62, 359, 361, P),
    (62, 362, 362, B), (62, 363, 363, P), (62, 364, 364, B), (62, 365, 367, P), (62, 368, 368, B),
    (62, 369, 369, P), (62, 370, 370, B), (62, 371, 373, P), (62, 374, 374, B), (62, 375, 375, P),
    (62, 376, 380, B), (62, 381, 382, P), (62, 383, 385, B), (62, 386, 387, P), (62, 388, 391, B),
    (62, 392, 394, P), (62, 395, 398, B), (62, 399, 399, P), (62, 400, 403, B), (62, 404, 405, P),
    (62, 406, 410, B), (62, 411, 411, P), (62, 412, 416, B), (62, 417, 418, P), (62, 419, 422, B),
    (62, 423, 423, P), (62, 424, 424, B), (62, 425, 427, P), (62, 428, 428, B), (62, 429, 430, P),
    (62, 431, 433, B), (62, 434, 439, P), (62, 440, 440, B), (62, 441, 442, P), (62, 443, 443, B),
    (62, 444, 445, P), (62, 446, 446, B), (62, 447, 448, P), (62, 449, 449, B), (62, 450, 453, P),
    (62, 454, 454, B), (62, 455, 457, P), (62, 458, 458, B), (62, 459, 459, P), (62, 460, 460, B),
    (62, 461, 463, P), (62, 464, 464, B), (62, 465, 466, P), (62, 467, 469, B), (62, 470, 471, P),
    (62, 472, 475, B), (62, 476, 478, P), (62, 479, 481, B), (62, 482, 483, P), (62, 484, 487, B),
    (62, 488, 490, P), (62, 491, 494, B), (62, 495, 495, P), (62, 496, 500, B), (62, 501, 501, P),
    (62, 502, 502, B), (62, 503, 505, P), (62, 506, 506, B), (62, 507, 507, P), (62, 508, 508, B),
    (62, 509, 511, P), (62, 512, 512, B), (62, 513, 513, P), (62, 514, 514, B), (62, 515, 517, P),
    (62, 518, 518, B), (62, 519, 519, P), (62, 520, 520, B), (62, 521, 523, P), (62, 524, 524, B),
    (62, 525, 525, P), (62, 526, 526, B), (62, 527, 529, P), (62, 530, 530, B), (62, 531, 531, P),
    (62, 532, 536, B), (62, 537, 538, P), (62, 539, 541, B), (62, 542, 543, P), (62, 544, 547, B),
    (62, 548, 550, P), (62, 551, 554, B), (62, 555, 555, P), (62, 556, 559, B), (62, 560, 561, P),
    (62, 562, 566, B), (62, 567, 567, P), (62, 568, 572, B), (62, 573, 574, P), (62, 575, 578, B),
    (62, 579, 579, P), (62, 580, 580, B), (62, 581, 583, P), (62, 584, 584, B), (62, 585, 586, P),
    (62, 587, 589, B), (62, 590, 595, P), (62, 596, 596, B), (62, 597, 598, P), (62, 599, 599, B),
    (62, 600, 601, P), (62, 602, 602, B), (62, 603, 604, P), (62, 605, 605, B), (62, 606, 609, P),
    (62, 610, 610, B), (62, 611, 613, P), (62, 614, 614, B), (62, 615, 615, P), (62, 616, 616, B),
    (62, 617, 619, P), (62, 620, 620, B), (62, 621, 622, P), (62, 623, 625, B), (62, 626, 627, P),
    (62, 628, 631, B), (62, 632, 634, P), (62, 635, 637, B), (62, 638, 639, P), (63, 0, 3, B), (63, 4, 6, P),
    (63, 7, 10, B), (63, 11, 11, P), (63, 12, 16, B), (63, 17, 17, P), (63, 18, 18, B), (63, 19, 21, P),
    (63, 22, 22, B), (63, 23, 23, P), (63, 24, 24, B), (63, 25, 27, P), (63, 28, 28, B), (63, 29, 29, P),
    (63, 30, 30, B), (63, 31, 33, P), (63, 34, 34, B), (63, 35, 35, P), (63, 36, 36, B), (63, 37, 39, P),
    (63, 40, 40, B), (63, 41, 41, P), (63, 42, 42, B), (63, 43, 45, P), (63, 46, 46, B), (63, 47, 47, P),
    (63, 48, 52, B), (63, 53, 54, P), (63, 55, 57, B), (63, 58, 59, P), (63, 60, 63, B), (63, 64, 66, P),
    (63, 67, 70, B), (63, 71, 71, P), (63, 72, 75, B), (63, 76, 77, P), (63, 78, 82, B), (63, 83, 83, P),
    (63, 84, 88, B), (63, 89, 90, P), (63, 91, 94, B), (63, 95, 95, P), (63, 96, 96, B), (63, 97, 99, P),
    (63, 100, 100, B), (63, 101, 102, P), (63, 103, 105, B), (63, 106, 111, P), (63, 112, 112, B),
    (63, 113, 114, P), (63, 115, 115, B), (63, 116, 117, P), (63, 118, 118, B), (63, 119, 120, P),
    (63, 121, 121, B), (63, 122, 125, P), (63, 126, 126, B), (63, 127, 129, P), (63, 130, 130, B),
    (63, 131, 131, P), (63, 132, 132, B), (63, 133, 135, P), (63, 136, 136, B), (63, 137, 138, P),
    (63, 139, 141, B), (63, 142, 143, P), (63, 144, 147, B), (63, 148, 150, P), (63, 151, 153, B),
    (63, 154, 155, P), (63, 156, 159, B), (63, 160, 162, P), (63, 163, 166, B), (63, 167, 167, P),
    (63, 168, 172, B), (63, 173, 173, P), (63, 174, 174, B), (63, 175, 177, P), (63, 178, 178, B),
    (63, 179, 179, P), (63, 180, 180, B), (63, 181, 183, P), (63, 184, 184, B), (63, 185, 185, P),
    (63, 186, 186, B), (63, 187, 189, P), (63, 190, 190, B), (63, 191, 191, P), (63, 192, 192, B),
    (63, 193, 195, P), (63, 196, 196, B), (63, 197, 197, P), (63, 198, 198, B), (63, 199, 201, P),
    (63, 202, 202, B), (63, 203, 203, P), (63, 204, 208, B), (63, 209, 210, P), (63, 211, 213, B),
    (63, 214, 215, P), (63, 216, 219, B), (63, 220, 222, P), (63, 223, 226, B), (63, 227, 227, P),
    (63, 228, 231, B), (63, 232, 233, P), (63, 234, 238, B), (63, 239, 239, P), (63, 240, 244, B),
    (63, 245, 246, P), (63, 247, 250, B), (63, 251, 251, P), (63, 252, 252, B), (63, 253, 255, P),
    (63, 256, 256, B), (63, 257, 258, P), (63, 259, 261, B), (63, 262, 267, P), (63, 268, 268, B),
    (63, 269, 270, P), (63, 271, 271, B), (63, 272, 273, P), (63, 274, 274, B), (63, 275, 276, P),
    (63, 277, 277, B), (63, 278, 281, P), (63, 282, 282, B), (63, 283, 285, P), (63, 286, 286, B),
    (63, 287, 287, P), (63, 288, 288, B), (63, 289, 291, P), (63, 292, 292, B), (63, 293, 294, P),
    (63, 295, 297, B), (63, 298, 299, P), (63, 300, 303, B), (63, 304, 306, P), (63, 307, 309, B),
    (63, 310, 311, P), (63, 312, 315, B), (63, 316, 318, P), (63, 319, 322, B), (63, 323, 323, P),
    (63, 324, 328, B), (63, 329, 329, P), (63, 330, 330, B), (63, 331, 333, P), (63, 334, 334, B),
    (63, 335, 335, P), (63, 336, 336, B), (63, 337, 339, P), (63, 340, 340, B), (63, 341, 341, P),
    (63, 342, 342, B), (63, 343, 345, P), (63, 346, 346, B), (63, 347, 347, P), (63, 348, 348, B),
    (63, 349, 351, P), (63, 352, 352, B), (63, 353, 353, P), (63, 354, 354, B), (63, 355, 357, P),
    (63, 358, 358, B), (63, 359, 359, P), (63, 360, 364, B), (63, 365, 366, P), (63, 367, 369, B),
    (63, 370, 371, P), (63, 372, 375, B), (63, 376, 378, P), (63, 379, 382, B), (63, 383, 383, P),
    (63, 384, 387, B), (63, 388, 389, P), (63, 390, 394, B), (63, 395, 395, P), (63, 396, 400, B),
    (63, 401, 402, P), (63, 403, 406, B), (63, 407, 407, P), (63, 408, 408, B), (63, 409, 411, P),
    (63, 412, 412, B), (63, 413, 414, P), (63, 415, 417, B), (63, 418, 423, P), (63, 424, 424, B),
    (63, 425, 426, P), (63, 427, 427, B), (63, 428, 429, P), (63, 430, 430, B), (63, 431, 432, P),
    (63, 433, 433, B), (63, 434, 437, P), (63, 438, 438, B), (63, 439, 441, P), (63, 442, 442, B),
    (63, 443, 443, P), (63, 444, 444, B), (63, 445, 447, P), (63, 448, 448, B), (63, 449, 450, P),
    (63, 451, 453, B), (63, 454, 455, P), (63, 456, 459, B), (63, 460, 462, P), (63, 463, 465, B),
    (63, 466, 467, P), (63, 468, 471, B), (63, 472, 474, P), (63, 475, 478, B), (63, 479, 479, P),
    (63, 480, 484, B), (63, 485, 485, P), (63, 486, 486, B), (63, 487, 489, P), (63, 490, 490, B),
    (63, 491, 491, P), (63, 492, 492, B), (63, 493, 495, P), (63, 496, 496, B), (63, 497, 497, P),
    (63, 498, 498, B), (63, 499, 501, P), (63, 502, 502, B), (63, 503, 503, P), (63, 504, 504, B),
    (63, 505, 507, P), (63, 508, 508, B), (63, 509, 509, P), (63, 510, 510, B), (63, 511, 513, P),
    (63, 514, 514, B), (63, 515, 515, P), (63, 516, 520, B), (63, 521, 522, P), (63, 523, 525, B),
    (63, 526, 527, P), (63, 528, 531, B), (63, 532, 534, P), (63, 535, 538, B), (63, 539, 539, P),
    (63, 540, 543, B), (63, 544, 545, P), (63, 546, 550, B), (63, 551, 551, P), (63, 552, 556, B),
    (63, 557, 558, P), (63, 559, 562, B), (63, 563, 563, P), (63, 564, 564, B), (63, 565, 567, P),
    (63, 568, 568, B), (63, 569, 570, P), (63, 571, 573, B), (63, 574, 579, P), (63, 580, 580, B),
    (63, 581, 582, P), (63, 583, 583, B), (63, 584, 585, P), (63, 586, 586, B), (63, 587, 588, P),
    (63, 589, 589, B), (63, 590, 593, P), (63, 594, 594, B), (63, 595, 597, P), (63, 598, 598, B),
    (63, 599, 599, P), (63, 600, 600, B), (63, 601, 603, P), (63, 604, 604, B), (63, 605, 606, P),
    (63, 607, 609, B), (63, 610, 611, P), (63, 612, 615, B), (63, 616, 618, P), (63, 619, 621, B),
    (63, 622, 623, P), (63, 624, 627, B), (63, 628, 630, P), (63, 631, 634, B), (63, 635, 635, P),
    (63, 636, 639, B), (64, 0, 0, B), (64, 1, 1, P), (64, 2, 2, B), (64, 3, 5, P), (64, 6, 6, B),
    (64, 7, 7, P), (64, 8, 8, B), (64, 9, 11, P), (64, 12, 12, B), (64, 13, 13, P), (64, 14, 14, B),
    (64, 15, 17, P), (64, 18, 18, B), (64, 19, 19, P), (64, 20, 20, B), (64, 21, 23, P), (64, 24, 24, B),
    (64, 25, 25, P), (64, 26, 26, B), (64, 27, 29, P), (64, 30, 30, B), (64, 31, 31, P), (64, 32, 36, B),
    (64, 37, 38, P), (64, 39, 41, B), (64, 42, 43, P), (64, 44, 47, B), (64, 48, 50, P), (64, 51, 54, B),
    (64, 55, 55, P), (64, 56, 59, B), (64, 60, 61, P), (64, 62, 66, B), (64, 67, 67, P), (64, 68, 72, B),
    (64, 73, 74, P), (64, 75, 78, B), (64, 79, 79, P), (64, 80, 80, B), (64, 81, 83, P), (64, 84, 84, B),
    (64, 85, 86, P), (64, 87, 89, B), (64, 90, 95, P), (64, 96, 96, B), (64, 97, 98, P), (64, 99, 99, B),
    (64, 100, 101, P), (64, 102, 102, B), (64, 103, 104, P), (64, 105, 105, B), (64, 106, 109, P),
    (64, 110, 110, B), (64, 111, 113, P), (64, 114, 114, B), (64, 115, 115, P), (64, 116, 116, B),
    (64, 117, 119, P), (64, 120, 120, B), (64, 121, 122, P), (64, 123, 125, B), (64, 126, 127, P),
    (64, 128, 131, B), (64, 132, 134, P), (64, 135, 137, B), (64, 138, 139, P), (64, 140, 143, B),
    (64, 144, 146, P), (64, 147, 150, B), (64, 151, 151, P), (64, 152, 156, B), (64, 157, 157, P),
    (64, 158, 158, B), (64, 159, 161, P), (64, 162, 162, B), (64, 163, 163, P), (64, 164, 164, B),
    (64, 165, 167, P), (64, 168, 168, B), (64, 169, 169, P), (64, 170, 170, B), (64, 171, 173, P),
    (64, 174, 174, B), (64, 175, 175, P), (64, 176, 176, B), (64, 177, 179, P), (64, 180, 180, B),
    (64, 181, 181, P), (64, 182, 182, B), (64, 183, 185, P), (64, 186, 186, B), (64, 187, 187, P),
    (64, 188, 192, B), (64, 193, 194, P), (64, 195, 197, B), (64, 198, 199, P), (64, 200, 203, B),
    (64, 204, 206, P), (64, 207, 210, B), (64, 211, 211, P), (64, 212, 215, B), (64, 216, 217, P),
    (64, 218, 222, B), (64, 223, 223, P), (64, 224, 228, B), (64, 229, 230, P), (64, 231, 234, B),
    (64, 235, 235, P), (64, 236, 236, B), (64, 237, 239, P), (64, 240, 240, B), (64, 241, 242, P),
    (64, 243, 245, B), (64, 246, 251, P), (64, 252, 252, B), (64, 253, 254, P), (64, 255, 255, B),
    (64, 256, 257, P), (64, 258, 258, B), (64, 259, 260, P), (64, 261, 261, B), (64, 262, 265, P),
    (64, 266, 266, B), (64, 267, 269, P), (64, 270, 270, B), (64, 271, 271, P), (64, 272, 272, B),
    (64, 273, 275, P), (64, 276, 276, B), (64, 277, 278, P), (64, 279, 281, B), (64, 282, 283, P),
    (64, 284, 287, B), (64, 288, 290, P), (64, 291, 293, B), (64, 294, 295, P), (64, 296, 299, B),
    (64, 300, 302, P), (64, 303, 306, B), (64, 307, 307, P), (64, 308, 312, B), (64, 313, 313, P),
    (64, 314, 314, B), (64, 315, 317, P), (64, 318, 318, B), (64, 319, 319, P), (64, 320, 320, B),
    (64, 321, 323, P), (64, 324, 324, B), (64, 325, 325, P), (64, 326, 326, B), (64, 327, 329, P),
    (64, 330, 330, B), (64, 331, 331, P), (64, 332, 332, B), (64, 333, 335, P), (64, 336, 336, B),
    (64, 337, 337, P), (64, 338, 338, B), (64, 339, 341, P), (64, 342, 342, B), (64, 343, 343, P),
    (64, 344, 348, B), (64, 349, 350, P), (64, 351, 353, B), (64, 354, 355, P), (64, 356, 359, B),
    (64, 360, 362, P), (64, 363, 366, B), (64, 367, 367, P), (64, 368, 371, B), (64, 372, 373, P),
    (64, 374, 378, B), (64, 379, 379, P), (64, 380, 384, B), (64, 385, 386, P), (64, 387, 390, B),
    (64, 391, 391, P), (64, 392, 392, B), (64, 393, 395, P), (64, 396, 396, B), (64, 397, 398, P),
    (64, 399, 401, B), (64, 402, 407, P), (64, 408, 408, B), (64, 409, 410, P), (64, 411, 411, B),
    (64, 412, 413, P), (64, 414, 414, B), (64, 415, 416, P), (64, 417, 417, B), (64, 418, 421, P),
    (64, 422, 422, B), (64, 423, 425, P), (64, 426, 426, B), (64, 427, 427, P), (64, 428, 428, B),
    (64, 429, 431, P), (64, 432, 432, B), (64, 433, 434, P), (64, 435, 437, B), (64, 438, 439, P),
    (64, 440, 443, B), (64, 444, 446, P), (64, 447, 449, B), (64, 450, 451, P), (64, 452, 455, B),
    (64, 456, 458, P), (64, 459, 462, B), (64, 463, 463, P), (64, 464, 468, B), (64, 469, 469, P),
    (64, 470, 470, B), (64, 471, 473, P), (64, 474, 474, B), (64, 475, 475, P), (64, 476, 476, B),
    (64, 477, 479, P), (64, 480, 480, B), (64, 481, 481, P), (64, 482, 482, B), (64, 483, 485, P),
    (64, 486, 486, B), (64, 487, 487, P), (64, 488, 488, B), (64, 489, 491, P), (64, 492, 492, B),
    (64, 493, 493, P), (64, 494, 494, B), (64, 495, 497, P), (64, 498, 498, B), (64, 499, 499, P),
    (64, 500, 504, B), (64, 505, 506, P), (64, 507, 509, B), (64, 510, 511, P), (64, 512, 515, B),
    (64, 516, 518, P), (64, 519, 522, B), (64, 523, 523, P), (64, 524, 527, B), (64, 528, 529, P),
    (64, 530, 534, B), (64, 535, 535, P), (64, 536, 540, B), (64, 541, 542, P), (64, 543, 546, B),
    (64, 547, 547, P), (64, 548, 548, B), (64, 549, 551, P), (64, 552, 552, B), (64, 553, 554, P),
    (64, 555, 557, B), (64, 558, 563, P), (64, 564, 564, B), (64, 565, 566, P), (64, 567, 567, B),
    (64, 568, 569, P), (64, 570, 570, B), (64, 571, 572, P), (64, 573, 573, B), (64, 574, 577, P),
    (64, 578, 578, B), (64, 579, 581, P), (64, 582, 582, B), (64, 583, 583, P), (64, 584, 584, B),
    (64, 585, 587, P), (64, 588, 588, B), (64, 589, 590, P), (64, 591, 593, B), (64, 594, 595, P),
    (64, 596, 599, B), (64, 600, 602, P), (64, 603, 605, B), (64, 606, 607, P), (64, 608, 611, B),
    (64, 612, 614, P), (64, 615, 618, B), (64, 619, 619, P), (64, 620, 624, B), (64, 625, 625, P),
    (64, 626, 626, B), (64, 627, 629, P), (64, 630, 630, B), (64, 631, 631, P), (64, 632, 632, B),
    (64, 633, 635, P), (64, 636, 636, B), (64, 637, 637, P), (64, 638, 638, B), (64, 639, 639, P),
    (65, 0, 1, P), (65, 2, 2, B), (65, 3, 3, P), (65, 4, 4, B), (65, 5, 7, P), (65, 8, 8, B), (65, 9, 9, P),
    (65, 10, 10, B), (65, 11, 13, P), (65, 14, 14, B), (65, 15, 15, P), (65, 16, 20, B), (65, 21, 22, P),
    (65, 23, 25, B), (65, 26, 27, P), (65, 28, 31, B), (65, 32, 34, P), (65, 35, 38, B), (65, 39, 39, P),
    (65, 40, 43, B), (65, 44, 45, P), (65, 46, 50, B), (65, 51, 51, P), (65, 52, 56, B), (65, 57, 58, P),
    (65, 59, 62, B), (65, 63, 63, P), (65, 64, 64, B), (65, 65, 67, P), (65, 68, 68, B), (65, 69, 70, P),
    (65, 71, 73, B), (65, 74, 79, P), (65, 80, 80, B), (65, 81, 82, P), (65, 83, 83, B), (65, 84, 85, P),
    (65, 86, 86, B), (65, 87, 88, P), (65, 89, 89, B), (65, 90, 93, P), (65, 94, 94, B), (65, 95, 97, P),
    (65, 98, 98, B), (65, 99, 99, P), (65, 100, 100, B), (65, 101, 103, P), (65, 104, 104, B),
    (65, 105, 106, P), (65, 107, 109, B), (65, 110, 111, P), (65, 112, 115, B), (65, 116, 118, P),
    (65, 119, 121, B), (65, 122, 123, P), (65, 124, 127, B), (65, 128, 130, P), (65, 131, 134, B),
    (65, 135, 135, P), (65, 136, 140, B), (65, 141, 141, P), (65, 142, 142, B), (65, 143, 145, P),
    (65, 146, 146, B), (65, 147, 147, P), (65, 148, 148, B), (65, 149, 151, P), (65, 152, 152, B),
    (65, 153, 153, P), (65, 154, 154, B), (65, 155, 157, P), (65, 158, 158, B), (65, 159, 159, P),
    (65, 160, 160, B), (65, 161, 163, P), (65, 164, 164, B), (65, 165, 165, P), (65, 166, 166, B),
    (65, 167, 169, P), (65, 170, 170, B), (65, 171, 171, P), (65, 172, 176, B), (65, 177, 178, P),
    (65, 179, 181, B), (65, 182, 183, P), (65, 184, 187, B), (65, 188, 190, P), (65, 191, 194, B),
    (65, 195, 195, P), (65, 196, 199, B), (65, 200, 201, P), (65, 202, 206, B), (65, 207, 207, P),
    (65, 208, 212, B), (65, 213, 214, P), (65, 215, 218, B), (65, 219, 219, P), (65, 220, 220, B),
    (65, 221, 223, P), (65, 224, 224, B), (65, 225, 226, P), (65, 227, 229, B), (65, 230, 235, P),
    (65, 236, 236, B), (65, 237, 238, P), (65, 239, 239, B), (65, 240, 241, P), (65, 242, 242, B),
    (65, 243, 244, P), (65, 245, 245, B), (65, 246, 249, P), (65, 250, 250, B), (65, 251, 253, P),
    (65, 254, 254, B), (65, 255, 255, P), (65, 256, 256, B), (65, 257, 259, P), (65, 260, 260, B),
    (65, 261, 262, P), (65, 263, 265, B), (65, 266, 267, P), (65, 268, 271, B), (65, 272, 274, P),
    (65, 275, 277, B), (65, 278, 279, P), (65, 280, 283, B), (65, 284, 286, P), (65, 287, 290, B),
    (65, 291, 291, P), (65, 292, 296, B), (65, 297, 297, P), (65, 298, 298, B), (65, 299, 301, P),
    (65, 302, 302, B), (65, 303, 303, P), (65, 304, 304, B), (65, 305, 307, P), (65, 308, 308, B),
    (65, 309, 309, P), (65, 310, 310, B), (65, 311, 313, P), (65, 314, 314, B), (65, 315, 315, P),
    (65, 316, 316, B), (65, 317, 319, P), (65, 320, 320, B), (65, 321, 321, P), (65, 322, 322, B),
    (65, 323, 325, P), (65, 326, 326, B), (65, 327, 327, P), (65, 328, 332, B), (65, 333, 334, P),
    (65, 335, 337, B), (65, 338, 339, P), (65, 340, 343, B), (65, 344, 346, P), (65, 347, 350, B),
    (65, 351, 351, P), (65, 352, 355, B), (65, 356, 357, P), (65, 358, 362, B), (65, 363, 363, P),
    (65, 364, 368, B), (65, 369, 370, P), (65, 371, 374, B), (65, 375, 375, P), (65, 376, 376, B),
    (65, 377, 379, P), (65, 380, 380, B), (65, 381, 382, P), (65, 383, 385, B), (65, 386, 391, P),
    (65, 392, 392, B), (65, 393, 394, P), (65, 395, 395, B), (65, 396, 397, P), (65, 398, 398, B),
    (65, 399, 400, P), (65, 401, 401, B), (65, 402, 405, P), (65, 406, 406, B), (65, 407, 409, P),
    (65, 410, 410, B), (65, 411, 411, P), (65, 412, 412, B), (65, 413, 415, P), (65, 416, 416, B),
    (65, 417, 418, P), (65, 419, 421, B), (65, 422, 423, P), (65, 424, 427, B), (65, 428, 430, P),
    (65, 431, 433, B), (65, 434, 435, P), (65, 436, 439, B), (65, 440, 442, P), (65, 443, 446, B),
    (65, 447, 447, P), (65, 448, 452, B), (65, 453, 453, P), (65, 454, 454, B), (65, 455, 457, P),
    (65, 458, 458, B), (65, 459, 459, P), (65, 460, 460, B), (65, 461, 463, P), (65, 464, 464, B),
    (65, 465, 465, P), (65, 466, 466, B), (65, 467, 469, P), (65, 470, 470, B), (65, 471, 471, P),
    (65, 472, 472, B), (65, 473, 475, P), (65, 476, 476, B), (65, 477, 477, P), (65, 478, 478, B),
    (65, 479, 481, P), (65, 482, 482, B), (65, 483, 483, P), (65, 484, 488, B), (65, 489, 490, P),
    (65, 491, 493, B), (65, 494, 495, P), (65, 496, 499, B), (65, 500, 502, P), (65, 503, 506, B),
    (65, 507, 507, P), (65, 508, 511, B), (65, 512, 513, P), (65, 514, 518, B), (65, 519, 519, P),
    (65, 520, 524, B), (65, 525, 526, P), (65, 527, 530, B), (65, 531, 531, P), (65, 532, 532, B),
    (65, 533, 535, P), (65, 536, 536, B), (65, 537, 538, P), (65, 539, 541, B), (65, 542, 547, P),
    (65, 548, 548, B), (65, 549, 550, P), (65, 551, 551, B), (65, 552, 553, P), (65, 554, 554, B),
    (65, 555, 556, P), (65, 557, 557, B), (65, 558, 561, P), (65, 562, 562, B), (65, 563, 565, P),
    (65, 566, 566, B), (65, 567, 567, P), (65, 568, 568, B), (65, 569, 571, P), (65, 572, 572, B),
    (65, 573, 574, P), (65, 575, 577, B), (65, 578, 579, P), (65, 580, 583, B), (65, 584, 586, P),
    (65, 587, 589, B), (65, 590, 591, P), (65, 592, 595, B), (65, 596, 598, P), (65, 599, 602, B),
    (65, 603, 603, P), (65, 604, 608, B), (65, 609, 609, P), (65, 610, 610, B), (65, 611, 613, P),
    (65, 614, 614, B), (65, 615, 615, P), (65, 616, 616, B), (65, 617, 619, P), (65, 620, 620, B),
    (65, 621, 621, P), (65, 622, 622, B), (65, 623, 625, P), (65, 626, 626, B), (65, 627, 627, P),
    (65, 628, 628, B), (65, 629, 631, P), (65, 632, 632, B), (65, 633, 633, P), (65, 634, 634, B),
    (65, 635, 637, P), (65, 638, 638, B), (65, 639, 639, P), (66, 0, 4, B), (66, 5, 6, P), (66, 7, 9, B),
    (66, 10, 11, P), (66, 12, 15, B), (66, 16, 18, P), (66, 19, 22, B), (66, 23, 23, P), (66, 24, 27, B),
    (66, 28, 29, P), (66, 30, 34, B), (66, 35, 35, P), (66, 36, 40, B), (66, 41, 42, P), (66, 43, 46, B),
    (66, 47, 47, P), (66, 48, 48, B), (66, 49, 51, P), (66, 52, 52, B), (66, 53, 54, P), (66, 55, 57, B),
    (66, 58, 63, P), (66, 64, 64, B), (66, 65, 66, P), (66, 67, 67, B), (66, 68, 69, P), (66, 70, 70, B),
    (66, 71, 72, P), (66, 73, 73, B), (66, 74, 77, P), (66, 78, 78, B), (66, 79, 81, P), (66, 82, 82, B),
    (66, 83, 83, P), (66, 84, 84, B), (66, 85, 87, P), (66, 88, 88, B), (66, 89, 90, P), (66, 91, 93, B),
    (66, 94, 95, P), (66, 96, 99, B), (66, 100, 102, P), (66, 103, 105, B), (66, 106, 107, P),
    (66, 108, 111, B), (66, 112, 114, P), (66, 115, 118, B), (66, 119, 119, P), (66, 120, 124, B),
    (66, 125, 125, P), (66, 126, 126, B), (66, 127, 129, P), (66, 130, 130, B), (66, 131, 131, P),
    (66, 132, 132, B), (66, 133, 135, P), (66, 136, 136, B), (66, 137, 137, P), (66, 138, 138, B),
    (66, 139, 141, P), (66, 142, 142, B), (66, 143, 143, P), (66, 144, 144, B), (66, 145, 147, P),
    (66, 148, 148, B), (66, 149, 149, P), (66, 150, 150, B), (66, 151, 153, P), (66, 154, 154, B),
    (66, 155, 155, P), (66, 156, 160, B), (66, 161, 162, P), (66, 163, 165, B), (66, 166, 167, P),
    (66, 168, 171, B), (66, 172, 174, P), (66, 175, 178, B), (66, 179, 179, P), (66, 180, 183, B),
    (66, 184, 185, P), (66, 186, 190, B), (66, 191, 191, P), (66, 192, 196, B), (66, 197, 198, P),
    (66, 199, 202, B), (66, 203, 203, P), (66, 204, 204, B), (66, 205, 207, P), (66, 208, 208, B),
    (66, 209, 210, P), (66, 211, 213, B), (66, 214, 219, P), (66, 220, 220, B), (66, 221, 222, P),
    (66, 223, 223, B), (66, 224, 225, P), (66, 226, 226, B), (66, 227, 228, P), (66, 229, 229, B),
    (66, 230, 233, P), (66, 234, 234, B), (66, 235, 237, P), (66, 238, 238, B), (66, 239, 239, P),
    (66, 240, 240, B), (66, 241, 243, P), (66, 244, 244, B), (66, 245, 246, P), (66, 247, 249, B),
    (66, 250, 251, P), (66, 252, 255, B), (66, 256, 258, P), (66, 259, 261, B), (66, 262, 263, P),
    (66, 264, 267, B), (66, 268, 270, P), (66, 271, 274, B), (66, 275, 275, P), (66, 276, 280, B),
    (66, 281, 281, P), (66, 282, 282, B), (66, 283, 285, P), (66, 286, 286, B), (66, 287, 287, P),
    (66, 288, 288, B), (66, 289, 291, P), (66, 292, 292, B), (66, 293, 293, P), (66, 294, 294, B),
    (66, 295, 297, P), (66, 298, 298, B), (66, 299, 299, P), (66, 300, 300, B), (66, 301, 303, P),
    (66, 304, 304, B), (66, 305, 305, P), (66, 306, 306, B), (66, 307, 309, P), (66, 310, 310, B),
    (66, 311, 311, P), (66, 312, 316, B), (66, 317, 318, P), (66, 319, 321, B), (66, 322, 323, P),
    (66, 324, 327, B), (66, 328, 330, P), (66, 331, 334, B), (66, 335, 335, P), (66, 336, 339, B),
    (66, 340, 341, P), (66, 342, 346, B), (66, 347, 347, P), (66, 348, 352, B), (66, 353, 354, P),
    (66, 355, 358, B), (66, 359, 359, P), (66, 360, 360, B), (66, 361, 363, P), (66, 364, 364, B),
    (66, 365, 366, P), (66, 367, 369, B), (66, 370, 375, P), (66, 376, 376, B), (66, 377, 378, P),
    (66, 379, 379, B), (66, 380, 381, P), (66, 382, 382, B), (66, 383, 384, P), (66, 385, 385, B),
    (66, 386, 389, P), (66, 390, 390, B), (66, 391, 393, P), (66, 394, 394, B), (66, 395, 395, P),
    (66, 396, 396, B), (66, 397, 399, P), (66, 400, 400, B), (66, 401, 402, P), (66, 403, 405, B),
    (66, 406, 407, P), (66, 408, 411, B), (66, 412, 414, P), (66, 415, 417, B), (66, 418, 419, P),
    (66, 420, 423, B), (66, 424, 426, P), (66, 427, 430, B), (66, 431, 431, P), (66, 432, 436, B),
    (66, 437, 437, P), (66, 438, 438, B), (66, 439, 441, P), (66, 442, 442, B), (66, 443, 443, P),
    (66, 444, 444, B), (66, 445, 447, P), (66, 448, 448, B), (66, 449, 449, P), (66, 450, 450, B),
    (66, 451, 453, P), (66, 454, 454, B), (66, 455, 455, P), (66, 456, 456, B), (66, 457, 459, P),
    (66, 460, 460, B), (66, 461, 461, P), (66, 462, 462, B), (66, 463, 465, P), (66, 466, 466, B),
    (66, 467, 467, P), (66, 468, 472, B), (66, 473, 474, P), (66, 475, 477, B), (66, 478, 479, P),
    (66, 480, 483, B), (66, 484, 486, P), (66, 487, 490, B), (66, 491, 491, P), (66, 492, 495, B),
    (66, 496, 497, P), (66, 498, 502, B), (66, 503, 503, P), (66, 504, 508, B), (66, 509, 510, P),
    (66, 511, 514, B), (66, 515, 515, P), (66, 516, 516, B), (66, 517, 519, P), (66, 520, 520, B),
    (66, 521, 522, P), (66, 523, 525, B), (66, 526, 531, P), (66, 532, 532, B), (66, 533, 534, P),
    (66, 535, 535, B), (66, 536, 537, P), (66, 538, 538, B), (66, 539, 540, P), (66, 541, 541, B),
    (66, 542, 545, P), (66, 546, 546, B), (66, 547, 549, P), (66, 550, 550, B), (66, 551, 551, P),
    (66, 552, 552, B), (66, 553, 555, P), (66, 556, 556, B), (66, 557, 558, P), (66, 559, 561, B),
    (66, 562, 563, P), (66, 564, 567, B), (66, 568, 570, P), (66, 571, 573, B), (66, 574, 575, P),
    (66, 576, 579, B), (66, 580, 582, P), (66, 583, 586, B), (66, 587, 587, P), (66, 588, 592, B),
    (66, 593, 593, P), (66, 594, 594, B), (66, 595, 597, P), (66, 598, 598, B), (66, 599, 599, P),
    (66, 600, 600, B), (66, 601, 603, P), (66, 604, 604, B), (66, 605, 605, P), (66, 606, 606, B),
    (66, 607, 609, P), (66, 610, 610, B), (66, 611, 611, P), (66, 612, 612, B), (66, 613, 615, P),
    (66, 616, 616, B), (66, 617, 617, P), (66, 618, 618, B), (66, 619, 621, P), (66, 622, 622, B),
    (66, 623, 623, P), (66, 624, 628, B), (66, 629, 630, P), (66, 631, 633, B), (66, 634, 635, P),
    (66, 636, 639, B), (67, 0, 2, P), (67, 3, 6, B), (67, 7, 7, P), (67, 8, 11, B), (67, 12, 13, P),
    (67, 14, 18, B), (67, 19, 19, P), (67, 20, 24, B), (67, 25, 26, P), (67, 27, 30, B), (67, 31, 31, P),
    (67, 32, 32, B), (67, 33, 35, P), (67, 36, 36, B), (67, 37, 38, P), (67, 39, 41, B), (67, 42, 47, P),
    (67, 48, 48, B), (67, 49, 50, P), (67, 51, 51, B), (67, 52, 53, P), (67, 54, 54, B), (67, 55, 56, P),
    (67, 57, 57, B), (67, 58, 61, P), (67, 62, 62, B), (67, 63, 65, P), (67, 66, 66, B), (67, 67, 67, P),
    (67, 68, 68, B), (67, 69, 71, P), (67, 72, 72, B), (67, 73, 74, P), (67, 75, 77, B), (67, 78, 79, P),
    (67, 80, 83, B), (67, 84, 86, P), (67, 87, 89, B), (67, 90, 91, P), (67, 92, 95, B), (67, 96, 98, P),
    (67, 99, 102, B), (67, 103, 103, P), (67, 104, 108, B), (67, 109, 109, P), (67, 110, 110, B),
    (67, 111, 113, P), (67, 114, 114, B), (67, 115, 115, P), (67, 116, 116, B), (67, 117, 119, P),
    (67, 120, 120, B), (67, 121, 121, P), (67, 122, 122, B), (67, 123, 125, P), (67, 126, 126, B),
    (67, 127, 127, P), (67, 128, 128, B), (67, 129, 131, P), (67, 132, 132, B), (67, 133, 133, P),
    (67, 134, 134, B), (67, 135, 137, P), (67, 138, 138, B), (67, 139, 139, P), (67, 140, 144, B),
    (67, 145, 146, P), (67, 147, 149, B), (67, 150, 151, P), (67, 152, 155, B), (67, 156, 158, P),
    (67, 159, 162, B), (67, 163, 163, P), (67, 164, 167, B), (67, 168, 169, P), (67, 170, 174, B),
    (67, 175, 175, P), (67, 176, 180, B), (67, 181, 182, P), (67, 183, 186, B), (67, 187, 187, P),
    (67, 188, 188, B), (67, 189, 191, P), (67, 192, 192, B), (67, 193, 194, P), (67, 195, 197, B),
    (67, 198, 203, P), (67, 204, 204, B), (67, 205, 206, P), (67, 207, 207, B), (67, 208, 209, P),
    (67, 210, 210, B), (67, 211, 212, P), (67, 213, 213, B), (67, 214, 217, P), (67, 218, 218, B),
    (67, 219, 221, P), (67, 222, 222, B), (67, 223, 223, P), (67, 224, 224, B), (67, 225, 227, P),
    (67, 228, 228, B), (67, 229, 230, P), (67, 231, 233, B), (67, 234, 235, P), (67, 236, 239, B),
    (67, 240, 242, P), (67, 243, 245, B), (67, 246, 247, P), (67, 248, 251, B), (67, 252, 254, P),
    (67, 255, 258, B), (67, 259, 259, P), (67, 260, 264, B), (67, 265, 265, P), (67, 266, 266, B),
    (67, 267, 269, P), (67, 270, 270, B), (67, 271, 271, P), (67, 272, 272, B), (67, 273, 275, P),
    (67, 276, 276, B), (67, 277, 277, P), (67, 278, 278, B), (67, 279, 281, P), (67, 282, 282, B),
    (67, 283, 283, P), (67, 284, 284, B), (67, 285, 287, P), (67, 288, 288, B), (67, 289, 289, P),
    (67, 290, 290, B), (67, 291, 293, P), (67, 294, 294, B), (67, 295, 295, P), (67, 296, 300, B),
    (67, 301, 302, P), (67, 303, 305, B), (67, 306, 307, P), (67, 308, 311, B), (67, 312, 314, P),
    (67, 315, 318, B), (67, 319, 319, P), (67, 320, 323, B), (67, 324, 325, P), (67, 326, 330, B),
    (67, 331, 331, P), (67, 332, 336, B), (67, 337, 338, P), (67, 339, 342, B), (67, 343, 343, P),
    (67, 344, 344, B), (67, 345, 347, P), (67, 348, 348, B), (67, 349, 350, P), (67, 351, 353, B),
    (67, 354, 359, P), (67, 360, 360, B), (67, 361, 362, P), (67, 363, 363, B), (67, 364, 365, P),
    (67, 366, 366, B), (67, 367, 368, P), (67, 369, 369, B), (67, 370, 373, P), (67, 374, 374, B),
    (67, 375, 377, P), (67, 378, 378, B), (67, 379, 379, P), (67, 380, 380, B), (67, 381, 383, P),
    (67, 384, 384, B), (67, 385, 386, P), (67, 387, 389, B), (67, 390, 391, P), (67, 392, 395, B),
    (67, 396, 398, P), (67, 399, 401, B), (67, 402, 403, P), (67, 404, 407, B), (67, 408, 410, P),
    (67, 411, 414, B), (67, 415, 415, P), (67, 416, 420, B), (67, 421, 421, P), (67, 422, 422, B),
    (67, 423, 425, P), (67, 426, 426, B), (67, 427, 427, P), (67, 428, 428, B), (67, 429, 431, P),
    (67, 432, 432, B), (67, 433, 433, P), (67, 434, 434, B), (67, 435, 437, P), (67, 438, 438, B),
    (67, 439, 439, P), (67, 440, 440, B), (67, 441, 443, P), (67, 444, 444, B), (67, 445, 445, P),
    (67, 446, 446, B), (67, 447, 449, P), (67, 450, 450, B), (67, 451, 451, P), (67, 452, 456, B),
    (67, 457, 458, P), (67, 459, 461, B), (67, 462, 463, P), (67, 464, 467, B), (67, 468, 470, P),
    (67, 471, 474, B), (67, 475, 475, P), (67, 476, 479, B), (67, 480, 481, P), (67, 482, 486, B),
    (67, 487, 487, P), (67, 488, 492, B), (67, 493, 494, P), (67, 495, 498, B), (67, 499, 499, P),
    (67, 500, 500, B), (67, 501, 503, P), (67, 504, 504, B), (67, 505, 506, P), (67, 507, 509, B),
    (67, 510, 515, P), (67, 516, 516, B), (67, 517, 518, P), (67, 519, 519, B), (67, 520, 521, P),
    (67, 522, 522, B), (67, 523, 524, P), (67, 525, 525, B), (67, 526, 529, P), (67, 530, 530, B),
    (67, 531, 533, P), (67, 534, 534, B), (67, 535, 535, P), (67, 536, 536, B), (67, 537, 539, P),
    (67, 540, 540, B), (67, 541, 542, P), (67, 543, 545, B), (67, 546, 547, P), (67, 548, 551, B),
    (67, 552, 554, P), (67, 555, 557, B), (67, 558, 559, P), (67, 560, 563, B), (67, 564, 566, P),
    (67, 567, 570, B), (67, 571, 571, P), (67, 572, 576, B), (67, 577, 577, P), (67, 578, 578, B),
    (67, 579, 581, P), (67, 582, 582, B), (67, 583, 583, P), (67, 584, 584, B), (67, 585, 587, P),
    (67, 588, 588, B), (67, 589, 589, P), (67, 590, 590, B), (67, 591, 593, P), (67, 594, 594, B),
    (67, 595, 595, P), (67, 596, 596, B), (67, 597, 599, P), (67, 600, 600, B), (67, 601, 601, P),
    (67, 602, 602, B), (67, 603, 605, P), (67, 606, 606, B), (67, 607, 607, P), (67, 608, 612, B),
    (67, 613, 614, P), (67, 615, 617, B), (67, 618, 619, P), (67, 620, 623, B), (67, 624, 626, P),
    (67, 627, 630, B), (67, 631, 631, P), (67, 632, 635, B), (67, 636, 637, P), (67, 638, 639, B),
    (68, 0, 2, B), (68, 3, 3, P), (68, 4, 8, B), (68, 9, 10, P), (68, 11, 14, B), (68, 15, 15, P),
    (68, 16, 16, B), (68, 17, 19, P), (68, 20, 20, B), (68, 21, 22, P), (68, 23, 25, B), (68, 26, 31, P),
    (68, 32, 32, B), (68, 33, 34, P), (68, 35, 35, B), (68, 36, 37, P), (68, 38, 38, B), (68, 39, 40, P),
    (68, 41, 41, B), (68, 42, 45, P), (68, 46, 46, B), (68, 47, 49, P), (68, 50, 50, B), (68, 51, 51, P),
    (68, 52, 52, B), (68, 53, 55, P), (68, 56, 56, B), (68, 57, 58, P), (68, 59, 61, B), (68, 62, 63, P),
    (68, 64, 67, B), (68, 68, 70, P), (68, 71, 73, B), (68, 74, 75, P), (68, 76, 79, B), (68, 80, 82, P),
    (68, 83, 86, B), (68, 87, 87, P), (68, 88, 92, B), (68, 93, 93, P), (68, 94, 94, B), (68, 95, 97, P),
    (68, 98, 98, B), (68, 99, 99, P), (68, 100, 100, B), (68, 101, 103, P), (68, 104, 104, B),
    (68, 105, 105, P), (68, 106, 106, B), (68, 107, 109, P), (68, 110, 110, B), (68, 111, 111, P),
    (68, 112, 112, B), (68, 113, 115, P), (68, 116, 116, B), (68, 117, 117, P), (68, 118, 118, B),
    (68, 119, 121, P), (68, 122, 122, B), (68, 123, 123, P), (68, 124, 128, B), (68, 129, 130, P),
    (68, 131, 133, B), (68, 134, 135, P), (68, 136, 139, B), (68, 140, 142, P), (68, 143, 146, B),
    (68, 147, 147, P), (68, 148, 151, B), (68, 152, 153, P), (68, 154, 158, B), (68, 159, 159, P),
    (68, 160, 164, B), (68, 165, 166, P), (68, 167, 170, B), (68, 171, 171, P), (68, 172, 172, B),
    (68, 173, 175, P), (68, 176, 176, B), (68, 177, 178, P), (68, 179, 181, B), (68, 182, 187, P),
    (68, 188, 188, B), (68, 189, 190, P), (68, 191, 191, B), (68, 192, 193, P), (68, 194, 194, B),
    (68, 195, 196, P), (68, 197, 197, B), (68, 198, 201, P), (68, 202, 202, B), (68, 203, 205, P),
    (68, 206, 206, B), (68, 207, 207, P), (68, 208, 208, B), (68, 209, 211, P), (68, 212, 212, B),
    (68, 213, 214, P), (68, 215, 217, B), (68, 218, 219, P), (68, 220, 223, B), (68, 224, 226, P),
    (68, 227, 229, B), (68, 230, 231, P), (68, 232, 235, B), (68, 236, 238, P), (68, 239, 242, B),
    (68, 243, 243, P), (68, 244, 248, B), (68, 249, 249, P), (68, 250, 250, B), (68, 251, 253, P),
    (68, 254, 254, B), (68, 255, 255, P), (68, 256, 256, B), (68, 257, 259, P), (68, 260, 260, B),
    (68, 261, 261, P), (68, 262, 262, B), (68, 263, 265, P), (68, 266, 266, B), (68, 267, 267, P),
    (68, 268, 268, B), (68, 269, 271, P), (68, 272, 272, B), (68, 273, 273, P), (68, 274, 274, B),
    (68, 275, 277, P), (68, 278, 278, B), (68, 279, 279, P), (68, 280, 284, B), (68, 285, 286, P),
    (68, 287, 289, B), (68, 290, 291, P), (68, 292, 295, B), (68, 296, 298, P), (68, 299, 302, B),
    (68, 303, 303, P), (68, 304, 307, B), (68, 308, 309, P), (68, 310, 314, B), (68, 315, 315, P),
    (68, 316, 320, B), (68, 321, 322, P), (68, 323, 326, B), (68, 327, 327, P), (68, 328, 328, B),
    (68, 329, 331, P), (68, 332, 332, B), (68, 333, 334, P), (68, 335, 337, B), (68, 338, 343, P),
    (68, 344, 344, B), (68, 345, 346, P), (68, 347, 347, B), (68, 348, 349, P), (68, 350, 350, B),
    (68, 351, 352, P), (68, 353, 353, B), (68, 354, 357, P), (68, 358, 358, B), (68, 359, 361, P),
    (68, 362, 362, B), (68, 363, 363, P), (68, 364, 364, B), (68, 365, 367, P), (68, 368, 368, B),
    (68, 369, 370, P), (68, 371, 373, B), (68, 374, 375, P), (68, 376, 379, B), (68, 380, 382, P),
    (68, 383, 385, B), (68, 386, 387, P), (68, 388, 391, B), (68, 392, 394, P), (68, 395, 398, B),
    (68, 399, 399, P), (68, 400, 404, B), (68, 405, 405, P), (68, 406, 406, B), (68, 407, 409, P),
    (68, 410, 410, B), (68, 411, 411, P), (68, 412, 412, B), (68, 413, 415, P), (68, 416, 416, B),
    (68, 417, 417, P), (68, 418, 418, B), (68, 419, 421, P), (68, 422, 422, B), (68, 423, 423, P),
    (68, 424, 424, B), (68, 425, 427, P), (68, 428, 428, B), (68, 429, 429, P), (68, 430, 430, B),
    (68, 431, 433, P), (68, 434, 434, B), (68, 435, 435, P), (68, 436, 440, B), (68, 441, 442, P),
    (68, 443, 445, B), (68, 446, 447, P), (68, 448, 451, B), (68, 452, 454, P), (68, 455, 458, B),
    (68, 459, 459, P), (68, 460, 463, B), (68, 464, 465, P), (68, 466, 470, B), (68, 471, 471, P),
    (68, 472, 476, B), (68, 477, 478, P), (68, 479, 482, B), (68, 483, 483, P), (68, 484, 484, B),
    (68, 485, 487, P), (68, 488, 488, B), (68, 489, 490, P), (68, 491, 493, B), (68, 494, 499, P),
    (68, 500, 500, B), (68, 501, 502, P), (68, 503, 503, B), (68, 504, 505, P), (68, 506, 506, B),
    (68, 507, 508, P), (68, 509, 509, B), (68, 510, 513, P), (68, 514, 514, B), (68, 515, 517, P),
    (68, 518, 518, B), (68, 519, 519, P), (68, 520, 520, B), (68, 521, 523, P), (68, 524, 524, B),
    (68, 525, 526, P), (68, 527, 529, B), (68, 530, 531, P), (68, 532, 535, B), (68, 536, 538, P),
    (68, 539, 541, B), (68, 542, 543, P), (68, 544, 547, B), (68, 548, 550, P), (68, 551, 554, B),
    (68, 555, 555, P), (68, 556, 560, B), (68, 561, 561, P), (68, 562, 562, B), (68, 563, 565, P),
    (68, 566, 566, B), (68, 567, 567, P), (68, 568, 568, B), (68, 569, 571, P), (68, 572, 572, B),
    (68, 573, 573, P), (68, 574, 574, B), (68, 575, 577, P), (68, 578, 578, B), (68, 579, 579, P),
    (68, 580, 580, B), (68, 581, 583, P), (68, 584, 584, B), (68, 585, 585, P), (68, 586, 586, B),
    (68, 587, 589, P), (68, 590, 590, B), (68, 591, 591, P), (68, 592, 596, B), (68, 597, 598, P),
    (68, 599, 601, B), (68, 602, 603, P), (68, 604, 607, B), (68, 608, 610, P), (68, 611, 614, B),
    (68, 615, 615, P), (68, 616, 619, B), (68, 620, 621, P), (68, 622, 626, B), (68, 627, 627, P),
    (68, 628, 632, B), (68, 633, 634, P), (68, 635, 638, B), (68, 639, 639, P), (69, 0, 0, B), (69, 1, 3, P),
    (69, 4, 4, B), (69, 5, 6, P), (69, 7, 9, B), (69, 10, 15, P), (69, 16, 16, B), (69, 17, 18, P),
    (69, 19, 19, B), (69, 20, 21, P), (69, 22, 22, B), (69, 23, 24, P), (69, 25, 25, B), (69, 26, 29, P),
    (69, 30, 30, B), (69, 31, 33, P), (69, 34, 34, B), (69, 35, 35, P), (69, 36, 36, B), (69, 37, 39, P),
    (69, 40, 40, B), (69, 41, 42, P), (69, 43, 45, B), (69, 46, 47, P), (69, 48, 51, B), (69, 52, 54, P),
    (69, 55, 57, B), (69, 58, 59, P), (69, 60, 63, B), (69, 64, 66, P), (69, 67, 70, B), (69, 71, 71, P),
    (69, 72, 76, B), (69, 77, 77, P), (69, 78, 78, B), (69, 79, 81, P), (69, 82, 82, B), (69, 83, 83, P),
    (69, 84, 84, B), (69, 85, 87, P), (69, 88, 88, B), (69, 89, 89, P), (69, 90, 90, B), (69, 91, 93, P),
    (69, 94, 94, B), (69, 95, 95, P), (69, 96, 96, B), (69, 97, 99, P), (69, 100, 100, B), (69, 101, 101, P),
    (69, 102, 102, B), (69, 103, 105, P), (69, 106, 106, B), (69, 107, 107, P), (69, 108, 112, B),
    (69, 113, 114, P), (69, 115, 117, B), (69, 118, 119, P), (69, 120, 123, B), (69, 124, 126, P),
    (69, 127, 130, B), (69, 131, 131, P), (69, 132, 135, B), (69, 136, 137, P), (69, 138, 142, B),
    (69, 143, 143, P), (69, 144, 148, B), (69, 149, 150, P), (69, 151, 154, B), (69, 155, 155, P),
    (69, 156, 156, B), (69, 157, 159, P), (69, 160, 160, B), (69, 161, 162, P), (69, 163, 165, B),
    (69, 166, 171, P), (69, 172, 172, B), (69, 173, 174, P), (69, 175, 175, B), (69, 176, 177, P),
    (69, 178, 178, B), (69, 179, 180, P), (69, 181, 181, B), (69, 182, 185, P), (69, 186, 186, B),
    (69, 187, 189, P), (69, 190, 190, B), (69, 191, 191, P), (69, 192, 192, B), (69, 193, 195, P),
    (69, 196, 196, B), (69, 197, 198, P), (69, 199, 201, B), (69, 202, 203, P), (69, 204, 207, B),
    (69, 208, 210, P), (69, 211, 213, B), (69, 214, 215, P), (69, 216, 219, B), (69, 220, 222, P),
    (69, 223, 226, B), (69, 227, 227, P), (69, 228, 232, B), (69, 233, 233, P), (69, 234, 234, B),
    (69, 235, 237, P), (69, 238, 238, B), (69, 239, 239, P), (69, 240, 240, B), (69, 241, 243, P),
    (69, 244, 244, B), (69, 245, 245, P), (69, 246, 246, B), (69, 247, 249, P), (69, 250, 250, B),
    (69, 251, 251, P), (69, 252, 252, B), (69, 253, 255, P), (69, 256, 256, B), (69, 257, 257, P),
    (69, 258, 258, B), (69, 259, 261, P), (69, 262, 262, B), (69, 263, 263, P), (69, 264, 268, B),
    (69, 269, 270, P), (69, 271, 273, B), (69, 274, 275, P), (69, 276, 279, B), (69, 280, 282, P),
    (69, 283, 286, B), (69, 287, 287, P), (69, 288, 291, B), (69, 292, 293, P), (69, 294, 298, B),
    (69, 299, 299, P), (69, 300, 304, B), (69, 305, 306, P), (69, 307, 310, B), (69, 311, 311, P),
    (69, 312, 312, B), (69, 313, 315, P), (69, 316, 316, B), (69, 317, 318, P), (69, 319, 321, B),
    (69, 322, 327, P), (69, 328, 328, B), (69, 329, 330, P), (69, 331, 331, B), (69, 332, 333, P),
    (69, 334, 334, B), (69, 335, 336, P), (69, 337, 337, B), (69, 338, 341, P), (69, 342, 342, B),
    (69, 343, 345, P), (69, 346, 346, B), (69, 347, 347, P), (69, 348, 348, B), (69, 349, 351, P),
    (69, 352, 352, B), (69, 353, 354, P), (69, 355, 357, B), (69, 358, 359, P), (69, 360, 363, B),
    (69, 364, 366, P), (69, 367, 369, B), (69, 370, 371, P), (69, 372, 375, B), (69, 376, 378, P),
    (69, 379, 382, B), (69, 383, 383, P), (69, 384, 388, B), (69, 389, 389, P), (69, 390, 390, B),
    (69, 391, 393, P), (69, 394, 394, B), (69, 395, 395, P), (69, 396, 396, B), (69, 397, 399, P),
    (69, 400, 400, B), (69, 401, 401, P), (69, 402, 402, B), (69, 403, 405, P), (69, 406, 406, B),
    (69, 407, 407, P), (69, 408, 408, B), (69, 409, 411, P), (69, 412, 412, B), (69, 413, 413, P),
    (69, 414, 414, B), (69, 415, 417, P), (69, 418, 418, B), (69, 419, 419, P), (69, 420, 424, B),
    (69, 425, 426, P), (69, 427, 429, B), (69, 430, 431, P), (69, 432, 435, B), (69, 436, 438, P),
    (69, 439, 442, B), (69, 443, 443, P), (69, 444, 447, B), (69, 448, 449, P), (69, 450, 454, B),
    (69, 455, 455, P), (69, 456, 460, B), (69, 461, 462, P), (69, 463, 466, B), (69, 467, 467, P),
    (69, 468, 468, B), (69, 469, 471, P), (69, 472, 472, B), (69, 473, 474, P), (69, 475, 477, B),
    (69, 478, 483, P), (69, 484, 484, B), (69, 485, 486, P), (69, 487, 487, B), (69, 488, 489, P),
    (69, 490, 490, B), (69, 491, 492, P), (69, 493, 493, B), (69, 494, 497, P), (69, 498, 498, B),
    (69, 499, 501, P), (69, 502, 502, B), (69, 503, 503, P), (69, 504, 504, B), (69, 505, 507, P),
    (69, 508, 508, B), (69, 509, 510, P), (69, 511, 513, B), (69, 514, 515, P), (69, 516, 519, B),
    (69, 520, 522, P), (69, 523, 525, B), (69, 526, 527, P), (69, 528, 531, B), (69, 532, 534, P),
    (69, 535, 538, B), (69, 539, 539, P), (69, 540, 544, B), (69, 545, 545, P), (69, 546, 546, B),
    (69, 547, 549, P), (69, 550, 550, B), (69, 551, 551, P), (69, 552, 552, B), (69, 553, 555, P),
    (69, 556, 556, B), (69, 557, 557, P), (69, 558, 558, B), (69, 559, 561, P), (69, 562, 562, B),
    (69, 563, 563, P), (69, 564, 564, B), (69, 565, 567, P), (69, 568, 568, B), (69, 569, 569, P),
    (69, 570, 570, B), (69, 571, 573, P), (69, 574, 574, B), (69, 575, 575, P), (69, 576, 580, B),
    (69, 581, 582, P), (69, 583, 585, B), (69, 586, 587, P), (69, 588, 591, B), (69, 592, 594, P),
    (69, 595, 598, B), (69, 599, 599, P), (69, 600, 603, B), (69, 604, 605, P), (69, 606, 610, B),
    (69, 611, 611, P), (69, 612, 616, B), (69, 617, 618, P), (69, 619, 622, B), (69, 623, 623, P),
    (69, 624, 624, B), (69, 625, 627, P), (69, 628, 628, B), (69, 629, 630, P), (69, 631, 633, B),
    (69, 634, 639, P),
];
/// T14G: grab (0,30) 640x40, record 311 of probe/e1.
#[rustfmt::skip]
const SP_T14G: &[Span] = &[
    (32, 580, 639, P), (33, 0, 259, P), (33, 580, 639, P), (34, 0, 259, P), (34, 580, 639, P),
    (35, 0, 259, P), (35, 580, 639, P), (36, 0, 259, P), (36, 580, 585, P), (36, 586, 589, B),
    (36, 590, 597, P), (36, 598, 607, B), (36, 608, 615, P), (36, 616, 623, B), (36, 624, 629, P),
    (36, 630, 637, B), (36, 638, 639, P), (37, 0, 5, P), (37, 6, 17, B), (37, 18, 21, P), (37, 22, 33, B),
    (37, 34, 39, P), (37, 40, 49, B), (37, 50, 53, P), (37, 54, 57, B), (37, 58, 61, P), (37, 62, 65, B),
    (37, 66, 69, P), (37, 70, 81, B), (37, 82, 93, P), (37, 94, 97, B), (37, 98, 99, P), (37, 100, 103, B),
    (37, 104, 107, P), (37, 108, 111, B), (37, 112, 117, P), (37, 118, 121, B), (37, 122, 131, P),
    (37, 132, 135, B), (37, 136, 141, P), (37, 142, 145, B), (37, 146, 149, P), (37, 150, 153, B),
    (37, 154, 157, P), (37, 158, 161, B), (37, 162, 167, P), (37, 168, 175, B), (37, 176, 181, P),
    (37, 182, 191, B), (37, 192, 199, P), (37, 200, 207, B), (37, 208, 211, P), (37, 212, 221, B),
    (37, 222, 231, P), (37, 232, 241, B), (37, 242, 245, P), (37, 246, 257, B), (37, 258, 259, P),
    (37, 580, 585, P), (37, 586, 589, B), (37, 590, 597, P), (37, 598, 607, B), (37, 608, 615, P),
    (37, 616, 623, B), (37, 624, 629, P), (37, 630, 637, B), (37, 638, 639, P), (38, 0, 5, P), (38, 6, 17, B),
    (38, 18, 21, P), (38, 22, 33, B), (38, 34, 39, P), (38, 40, 49, B), (38, 50, 53, P), (38, 54, 57, B),
    (38, 58, 61, P), (38, 62, 65, B), (38, 66, 69, P), (38, 70, 81, B), (38, 82, 93, P), (38, 94, 97, B),
    (38, 98, 99, P), (38, 100, 103, B), (38, 104, 107, P), (38, 108, 111, B), (38, 112, 117, P),
    (38, 118, 121, B), (38, 122, 131, P), (38, 132, 135, B), (38, 136, 141, P), (38, 142, 145, B),
    (38, 146, 149, P), (38, 150, 153, B), (38, 154, 157, P), (38, 158, 161, B), (38, 162, 167, P),
    (38, 168, 175, B), (38, 176, 181, P), (38, 182, 191, B), (38, 192, 199, P), (38, 200, 207, B),
    (38, 208, 211, P), (38, 212, 221, B), (38, 222, 231, P), (38, 232, 241, B), (38, 242, 245, P),
    (38, 246, 257, B), (38, 258, 259, P), (38, 580, 583, P), (38, 584, 591, B), (38, 592, 597, P),
    (38, 598, 609, B), (38, 610, 613, P), (38, 614, 625, B), (38, 626, 629, P), (38, 630, 639, B),
    (39, 0, 5, P), (39, 6, 17, B), (39, 18, 21, P), (39, 22, 33, B), (39, 34, 37, P), (39, 38, 49, B),
    (39, 50, 53, P), (39, 54, 57, B), (39, 58, 61, P), (39, 62, 65, B), (39, 66, 69, P), (39, 70, 81, B),
    (39, 82, 93, P), (39, 94, 97, B), (39, 98, 99, P), (39, 100, 103, B), (39, 104, 107, P),
    (39, 108, 111, B), (39, 112, 117, P), (39, 118, 121, B), (39, 122, 131, P), (39, 132, 135, B),
    (39, 136, 141, P), (39, 142, 145, B), (39, 146, 149, P), (39, 150, 153, B), (39, 154, 157, P),
    (39, 158, 161, B), (39, 162, 165, P), (39, 166, 177, B), (39, 178, 181, P), (39, 182, 193, B),
    (39, 194, 197, P), (39, 198, 209, B), (39, 210, 211, P), (39, 212, 223, B), (39, 224, 229, P),
    (39, 230, 241, B), (39, 242, 245, P), (39, 246, 257, B), (39, 258, 259, P), (39, 580, 583, P),
    (39, 584, 591, B), (39, 592, 597, P), (39, 598, 609, B), (39, 610, 613, P), (39, 614, 625, B),
    (39, 626, 629, P), (39, 630, 639, B), (40, 0, 5, P), (40, 6, 17, B), (40, 18, 21, P), (40, 22, 33, B),
    (40, 34, 37, P), (40, 38, 49, B), (40, 50, 53, P), (40, 54, 57, B), (40, 58, 61, P), (40, 62, 65, B),
    (40, 66, 69, P), (40, 70, 81, B), (40, 82, 93, P), (40, 94, 97, B), (40, 98, 99, P), (40, 100, 103, B),
    (40, 104, 107, P), (40, 108, 111, B), (40, 112, 117, P), (40, 118, 121, B), (40, 122, 131, P),
    (40, 132, 135, B), (40, 136, 141, P), (40, 142, 145, B), (40, 146, 149, P), (40, 150, 153, B),
    (40, 154, 157, P), (40, 158, 161, B), (40, 162, 165, P), (40, 166, 177, B), (40, 178, 181, P),
    (40, 182, 193, B), (40, 194, 197, P), (40, 198, 209, B), (40, 210, 211, P), (40, 212, 223, B),
    (40, 224, 229, P), (40, 230, 241, B), (40, 242, 245, P), (40, 246, 257, B), (40, 258, 259, P),
    (40, 580, 581, P), (40, 582, 593, B), (40, 594, 597, P), (40, 598, 601, B), (40, 602, 605, P),
    (40, 606, 609, B), (40, 610, 613, P), (40, 614, 617, B), (40, 618, 621, P), (40, 622, 625, B),
    (40, 626, 629, P), (40, 630, 633, B), (40, 634, 635, P), (40, 636, 639, B), (41, 0, 1, B), (41, 2, 5, P),
    (41, 6, 9, B), (41, 10, 21, P), (41, 22, 25, B), (41, 26, 37, P), (41, 38, 41, B), (41, 42, 53, P),
    (41, 54, 57, B), (41, 58, 61, P), (41, 62, 65, B), (41, 66, 73, P), (41, 74, 77, B), (41, 78, 93, P),
    (41, 94, 97, B), (41, 98, 99, P), (41, 100, 103, B), (41, 104, 105, P), (41, 106, 109, B),
    (41, 110, 117, P), (41, 118, 121, B), (41, 122, 131, P), (41, 132, 137, B), (41, 138, 139, P),
    (41, 140, 145, B), (41, 146, 149, P), (41, 150, 153, B), (41, 154, 157, P), (41, 158, 161, B),
    (41, 162, 165, P), (41, 166, 169, B), (41, 170, 173, P), (41, 174, 177, B), (41, 178, 181, P),
    (41, 182, 185, B), (41, 186, 189, P), (41, 190, 193, B), (41, 194, 197, P), (41, 198, 201, B),
    (41, 202, 205, P), (41, 206, 209, B), (41, 210, 211, P), (41, 212, 215, B), (41, 216, 219, P),
    (41, 220, 223, B), (41, 224, 229, P), (41, 230, 233, B), (41, 234, 249, P), (41, 250, 253, B),
    (41, 254, 259, P), (41, 580, 581, P), (41, 582, 593, B), (41, 594, 597, P), (41, 598, 601, B),
    (41, 602, 605, P), (41, 606, 609, B), (41, 610, 613, P), (41, 614, 617, B), (41, 618, 621, P),
    (41, 622, 625, B), (41, 626, 629, P), (41, 630, 633, B), (41, 634, 635, P), (41, 636, 639, B),
    (42, 0, 1, B), (42, 2, 5, P), (42, 6, 9, B), (42, 10, 21, P), (42, 22, 25, B), (42, 26, 37, P),
    (42, 38, 41, B), (42, 42, 53, P), (42, 54, 57, B), (42, 58, 61, P), (42, 62, 65, B), (42, 66, 73, P),
    (42, 74, 77, B), (42, 78, 93, P), (42, 94, 97, B), (42, 98, 99, P), (42, 100, 103, B), (42, 104, 105, P),
    (42, 106, 109, B), (42, 110, 117, P), (42, 118, 121, B), (42, 122, 131, P), (42, 132, 137, B),
    (42, 138, 139, P), (42, 140, 145, B), (42, 146, 149, P), (42, 150, 153, B), (42, 154, 157, P),
    (42, 158, 161, B), (42, 162, 165, P), (42, 166, 169, B), (42, 170, 173, P), (42, 174, 177, B),
    (42, 178, 181, P), (42, 182, 185, B), (42, 186, 189, P), (42, 190, 193, B), (42, 194, 197, P),
    (42, 198, 201, B), (42, 202, 205, P), (42, 206, 209, B), (42, 210, 211, P), (42, 212, 215, B),
    (42, 216, 219, P), (42, 220, 223, B), (42, 224, 229, P), (42, 230, 233, B), (42, 234, 249, P),
    (42, 250, 253, B), (42, 254, 259, P), (42, 580, 581, P), (42, 582, 585, B), (42, 586, 589, P),
    (42, 590, 593, B), (42, 594, 597, P), (42, 598, 601, B), (42, 602, 605, P), (42, 606, 609, B),
    (42, 610, 613, P), (42, 614, 617, B), (42, 618, 621, P), (42, 622, 625, B), (42, 626, 629, P),
    (42, 630, 633, B), (42, 634, 637, P), (42, 638, 639, B), (43, 0, 1, B), (43, 2, 5, P), (43, 6, 9, B),
    (43, 10, 21, P), (43, 22, 25, B), (43, 26, 37, P), (43, 38, 41, B), (43, 42, 53, P), (43, 54, 57, B),
    (43, 58, 61, P), (43, 62, 65, B), (43, 66, 73, P), (43, 74, 77, B), (43, 78, 93, P), (43, 94, 97, B),
    (43, 98, 99, P), (43, 100, 103, B), (43, 104, 105, P), (43, 106, 109, B), (43, 110, 117, P),
    (43, 118, 121, B), (43, 122, 131, P), (43, 132, 137, B), (43, 138, 139, P), (43, 140, 145, B),
    (43, 146, 149, P), (43, 150, 155, B), (43, 156, 157, P), (43, 158, 161, B), (43, 162, 165, P),
    (43, 166, 169, B), (43, 170, 173, P), (43, 174, 177, B), (43, 178, 181, P), (43, 182, 185, B),
    (43, 186, 189, P), (43, 190, 193, B), (43, 194, 197, P), (43, 198, 201, B), (43, 202, 205, P),
    (43, 206, 209, B), (43, 210, 211, P), (43, 212, 215, B), (43, 216, 219, P), (43, 220, 223, B),
    (43, 224, 229, P), (43, 230, 233, B), (43, 234, 249, P), (43, 250, 253, B), (43, 254, 259, P),
    (43, 580, 581, P), (43, 582, 585, B), (43, 586, 589, P), (43, 590, 593, B), (43, 594, 597, P),
    (43, 598, 601, B), (43, 602, 605, P), (43, 606, 609, B), (43, 610, 613, P), (43, 614, 617, B),
    (43, 618, 621, P), (43, 622, 625, B), (43, 626, 629, P), (43, 630, 633, B), (43, 634, 637, P),
    (43, 638, 639, B), (44, 0, 1, B), (44, 2, 5, P), (44, 6, 9, B), (44, 10, 21, P), (44, 22, 25, B),
    (44, 26, 37, P), (44, 38, 41, B), (44, 42, 53, P), (44, 54, 57, B), (44, 58, 61, P), (44, 62, 65, B),
    (44, 66, 73, P), (44, 74, 77, B), (44, 78, 93, P), (44, 94, 97, B), (44, 98, 99, P), (44, 100, 103, B),
    (44, 104, 105, P), (44, 106, 109, B), (44, 110, 117, P), (44, 118, 121, B), (44, 122, 131, P),
    (44, 132, 137, B), (44, 138, 139, P), (44, 140, 145, B), (44, 146, 149, P), (44, 150, 155, B),
    (44, 156, 157, P), (44, 158, 161, B), (44, 162, 165, P), (44, 166, 169, B), (44, 170, 173, P),
    (44, 174, 177, B), (44, 178, 181, P), (44, 182, 185, B), (44, 186, 189, P), (44, 190, 193, B),
    (44, 194, 197, P), (44, 198, 201, B), (44, 202, 205, P), (44, 206, 209, B), (44, 210, 211, P),
    (44, 212, 215, B), (44, 216, 219, P), (44, 220, 223, B), (44, 224, 229, P), (44, 230, 233, B),
    (44, 234, 249, P), (44, 250, 253, B), (44, 254, 259, P), (44, 580, 581, P), (44, 582, 585, B),
    (44, 586, 589, P), (44, 590, 593, B), (44, 594, 597, P), (44, 598, 609, B), (44, 610, 613, P),
    (44, 614, 617, B), (44, 618, 629, P), (44, 630, 633, B), (44, 634, 637, P), (44, 638, 639, B),
    (45, 0, 1, B), (45, 2, 5, P), (45, 6, 15, B), (45, 16, 21, P), (45, 22, 31, B), (45, 32, 37, P),
    (45, 38, 41, B), (45, 42, 43, P), (45, 44, 49, B), (45, 50, 53, P), (45, 54, 65, B), (45, 66, 73, P),
    (45, 74, 77, B), (45, 78, 93, P), (45, 94, 97, B), (45, 98, 99, P), (45, 100, 107, B), (45, 108, 117, P),
    (45, 118, 121, B), (45, 122, 131, P), (45, 132, 145, B), (45, 146, 149, P), (45, 150, 155, B),
    (45, 156, 157, P), (45, 158, 161, B), (45, 162, 165, P), (45, 166, 169, B), (45, 170, 173, P),
    (45, 174, 177, B), (45, 178, 181, P), (45, 182, 185, B), (45, 186, 189, P), (45, 190, 193, B),
    (45, 194, 197, P), (45, 198, 201, B), (45, 202, 205, P), (45, 206, 209, B), (45, 210, 211, P),
    (45, 212, 215, B), (45, 216, 219, P), (45, 220, 223, B), (45, 224, 229, P), (45, 230, 235, B),
    (45, 236, 249, P), (45, 250, 253, B), (45, 254, 259, P), (45, 580, 581, P), (45, 582, 585, B),
    (45, 586, 589, P), (45, 590, 593, B), (45, 594, 597, P), (45, 598, 609, B), (45, 610, 613, P),
    (45, 614, 617, B), (45, 618, 629, P), (45, 630, 633, B), (45, 634, 637, P), (45, 638, 639, B),
    (46, 0, 1, B), (46, 2, 5, P), (46, 6, 15, B), (46, 16, 21, P), (46, 22, 31, B), (46, 32, 37, P),
    (46, 38, 41, B), (46, 42, 43, P), (46, 44, 49, B), (46, 50, 53, P), (46, 54, 65, B), (46, 66, 73, P),
    (46, 74, 77, B), (46, 78, 93, P), (46, 94, 97, B), (46, 98, 99, P), (46, 100, 107, B), (46, 108, 117, P),
    (46, 118, 121, B), (46, 122, 131, P), (46, 132, 145, B), (46, 146, 149, P), (46, 150, 155, B),
    (46, 156, 157, P), (46, 158, 161, B), (46, 162, 165, P), (46, 166, 169, B), (46, 170, 173, P),
    (46, 174, 177, B), (46, 178, 181, P), (46, 182, 185, B), (46, 186, 189, P), (46, 190, 193, B),
    (46, 194, 197, P), (46, 198, 201, B), (46, 202, 205, P), (46, 206, 209, B), (46, 210, 211, P),
    (46, 212, 215, B), (46, 216, 219, P), (46, 220, 223, B), (46, 224, 229, P), (46, 230, 235, B),
    (46, 236, 249, P), (46, 250, 253, B), (46, 254, 259, P), (46, 580, 581, P), (46, 582, 585, B),
    (46, 586, 589, P), (46, 590, 593, B), (46, 594, 597, P), (46, 598, 607, B), (46, 608, 613, P),
    (46, 614, 617, B), (46, 618, 629, P), (46, 630, 633, B), (46, 634, 637, P), (46, 638, 639, B),
    (47, 0, 1, B), (47, 2, 5, P), (47, 6, 15, B), (47, 16, 21, P), (47, 22, 31, B), (47, 32, 37, P),
    (47, 38, 41, B), (47, 42, 43, P), (47, 44, 49, B), (47, 50, 53, P), (47, 54, 65, B), (47, 66, 73, P),
    (47, 74, 77, B), (47, 78, 93, P), (47, 94, 97, B), (47, 98, 99, P), (47, 100, 107, B), (47, 108, 117, P),
    (47, 118, 121, B), (47, 122, 131, P), (47, 132, 135, B), (47, 136, 137, P), (47, 138, 139, B),
    (47, 140, 141, P), (47, 142, 145, B), (47, 146, 149, P), (47, 150, 161, B), (47, 162, 165, P),
    (47, 166, 169, B), (47, 170, 173, P), (47, 174, 177, B), (47, 178, 181, P), (47, 182, 185, B),
    (47, 186, 189, P), (47, 190, 193, B), (47, 194, 197, P), (47, 198, 201, B), (47, 202, 205, P),
    (47, 206, 209, B), (47, 210, 211, P), (47, 212, 223, B), (47, 224, 231, P), (47, 232, 237, B),
    (47, 238, 249, P), (47, 250, 253, B), (47, 254, 259, P), (47, 580, 581, P), (47, 582, 585, B),
    (47, 586, 589, P), (47, 590, 593, B), (47, 594, 597, P), (47, 598, 607, B), (47, 608, 613, P),
    (47, 614, 617, B), (47, 618, 629, P), (47, 630, 633, B), (47, 634, 637, P), (47, 638, 639, B),
    (48, 0, 1, B), (48, 2, 5, P), (48, 6, 15, B), (48, 16, 21, P), (48, 22, 31, B), (48, 32, 37, P),
    (48, 38, 41, B), (48, 42, 43, P), (48, 44, 49, B), (48, 50, 53, P), (48, 54, 65, B), (48, 66, 73, P),
    (48, 74, 77, B), (48, 78, 93, P), (48, 94, 97, B), (48, 98, 99, P), (48, 100, 107, B), (48, 108, 117, P),
    (48, 118, 121, B), (48, 122, 131, P), (48, 132, 135, B), (48, 136, 137, P), (48, 138, 139, B),
    (48, 140, 141, P), (48, 142, 145, B), (48, 146, 149, P), (48, 150, 161, B), (48, 162, 165, P),
    (48, 166, 169, B), (48, 170, 173, P), (48, 174, 177, B), (48, 178, 181, P), (48, 182, 185, B),
    (48, 186, 189, P), (48, 190, 193, B), (48, 194, 197, P), (48, 198, 201, B), (48, 202, 205, P),
    (48, 206, 209, B), (48, 210, 211, P), (48, 212, 223, B), (48, 224, 231, P), (48, 232, 237, B),
    (48, 238, 249, P), (48, 250, 253, B), (48, 254, 259, P), (48, 580, 581, P), (48, 582, 593, B),
    (48, 594, 597, P), (48, 598, 601, B), (48, 602, 605, P), (48, 606, 609, B), (48, 610, 613, P),
    (48, 614, 617, B), (48, 618, 629, P), (48, 630, 633, B), (48, 634, 637, P), (48, 638, 639, B),
    (49, 0, 1, B), (49, 2, 5, P), (49, 6, 9, B), (49, 10, 21, P), (49, 22, 25, B), (49, 26, 37, P),
    (49, 38, 41, B), (49, 42, 45, P), (49, 46, 49, B), (49, 50, 53, P), (49, 54, 57, B), (49, 58, 61, P),
    (49, 62, 65, B), (49, 66, 73, P), (49, 74, 77, B), (49, 78, 93, P), (49, 94, 97, B), (49, 98, 99, P),
    (49, 100, 103, B), (49, 104, 105, P), (49, 106, 109, B), (49, 110, 117, P), (49, 118, 121, B),
    (49, 122, 131, P), (49, 132, 135, B), (49, 136, 137, P), (49, 138, 139, B), (49, 140, 141, P),
    (49, 142, 145, B), (49, 146, 149, P), (49, 150, 161, B), (49, 162, 165, P), (49, 166, 169, B),
    (49, 170, 173, P), (49, 174, 177, B), (49, 178, 181, P), (49, 182, 193, B), (49, 194, 197, P),
    (49, 198, 201, B), (49, 202, 205, P), (49, 206, 209, B), (49, 210, 211, P), (49, 212, 221, B),
    (49, 222, 233, P), (49, 234, 239, B), (49, 240, 249, P), (49, 250, 253, B), (49, 254, 259, P),
    (49, 580, 581, P), (49, 582, 593, B), (49, 594, 597, P), (49, 598, 601, B), (49, 602, 605, P),
    (49, 606, 609, B), (49, 610, 613, P), (49, 614, 617, B), (49, 618, 629, P), (49, 630, 633, B),
    (49, 634, 637, P), (49, 638, 639, B), (50, 0, 1, B), (50, 2, 5, P), (50, 6, 9, B), (50, 10, 21, P),
    (50, 22, 25, B), (50, 26, 37, P), (50, 38, 41, B), (50, 42, 45, P), (50, 46, 49, B), (50, 50, 53, P),
    (50, 54, 57, B), (50, 58, 61, P), (50, 62, 65, B), (50, 66, 73, P), (50, 74, 77, B), (50, 78, 93, P),
    (50, 94, 97, B), (50, 98, 99, P), (50, 100, 103, B), (50, 104, 105, P), (50, 106, 109, B),
    (50, 110, 117, P), (50, 118, 121, B), (50, 122, 131, P), (50, 132, 135, B), (50, 136, 137, P),
    (50, 138, 139, B), (50, 140, 141, P), (50, 142, 145, B), (50, 146, 149, P), (50, 150, 161, B),
    (50, 162, 165, P), (50, 166, 169, B), (50, 170, 173, P), (50, 174, 177, B), (50, 178, 181, P),
    (50, 182, 193, B), (50, 194, 197, P), (50, 198, 201, B), (50, 202, 205, P), (50, 206, 209, B),
    (50, 210, 211, P), (50, 212, 221, B), (50, 222, 233, P), (50, 234, 239, B), (50, 240, 249, P),
    (50, 250, 253, B), (50, 254, 259, P), (50, 580, 581, P), (50, 582, 593, B), (50, 594, 597, P),
    (50, 598, 601, B), (50, 602, 605, P), (50, 606, 609, B), (50, 610, 613, P), (50, 614, 617, B),
    (50, 618, 629, P), (50, 630, 633, B), (50, 634, 637, P), (50, 638, 639, B), (51, 0, 1, B), (51, 2, 5, P),
    (51, 6, 9, B), (51, 10, 21, P), (51, 22, 25, B), (51, 26, 37, P), (51, 38, 41, B), (51, 42, 45, P),
    (51, 46, 49, B), (51, 50, 53, P), (51, 54, 57, B), (51, 58, 61, P), (51, 62, 65, B), (51, 66, 73, P),
    (51, 74, 77, B), (51, 78, 93, P), (51, 94, 97, B), (51, 98, 99, P), (51, 100, 103, B), (51, 104, 105, P),
    (51, 106, 109, B), (51, 110, 117, P), (51, 118, 121, B), (51, 122, 131, P), (51, 132, 135, B),
    (51, 136, 141, P), (51, 142, 145, B), (51, 146, 149, P), (51, 150, 153, B), (51, 154, 155, P),
    (51, 156, 161, B), (51, 162, 165, P), (51, 166, 169, B), (51, 170, 173, P), (51, 174, 177, B),
    (51, 178, 181, P), (51, 182, 191, B), (51, 192, 197, P), (51, 198, 201, B), (51, 202, 205, P),
    (51, 206, 209, B), (51, 210, 211, P), (51, 212, 215, B), (51, 216, 217, P), (51, 218, 221, B),
    (51, 222, 235, P), (51, 236, 241, B), (51, 242, 249, P), (51, 250, 253, B), (51, 254, 259, P),
    (51, 580, 581, P), (51, 582, 593, B), (51, 594, 597, P), (51, 598, 601, B), (51, 602, 605, P),
    (51, 606, 609, B), (51, 610, 613, P), (51, 614, 617, B), (51, 618, 629, P), (51, 630, 633, B),
    (51, 634, 637, P), (51, 638, 639, B), (52, 0, 1, B), (52, 2, 5, P), (52, 6, 9, B), (52, 10, 21, P),
    (52, 22, 25, B), (52, 26, 37, P), (52, 38, 41, B), (52, 42, 45, P), (52, 46, 49, B), (52, 50, 53, P),
    (52, 54, 57, B), (52, 58, 61, P), (52, 62, 65, B), (52, 66, 73, P), (52, 74, 77, B), (52, 78, 93, P),
    (52, 94, 97, B), (52, 98, 99, P), (52, 100, 103, B), (52, 104, 105, P), (52, 106, 109, B),
    (52, 110, 117, P), (52, 118, 121, B), (52, 122, 131, P), (52, 132, 135, B), (52, 136, 141, P),
    (52, 142, 145, B), (52, 146, 149, P), (52, 150, 153, B), (52, 154, 155, P), (52, 156, 161, B),
    (52, 162, 165, P), (52, 166, 169, B), (52, 170, 173, P), (52, 174, 177, B), (52, 178, 181, P),
    (52, 182, 191, B), (52, 192, 197, P), (52, 198, 201, B), (52, 202, 205, P), (52, 206, 209, B),
    (52, 210, 211, P), (52, 212, 215, B), (52, 216, 217, P), (52, 218, 221, B), (52, 222, 235, P),
    (52, 236, 241, B), (52, 242, 249, P), (52, 250, 253, B), (52, 254, 259, P), (52, 580, 581, P),
    (52, 582, 585, B), (52, 586, 589, P), (52, 590, 593, B), (52, 594, 597, P), (52, 598, 601, B),
    (52, 602, 605, P), (52, 606, 609, B), (52, 610, 613, P), (52, 614, 617, B), (52, 618, 621, P),
    (52, 622, 625, B), (52, 626, 629, P), (52, 630, 633, B), (52, 634, 637, P), (52, 638, 639, B),
    (53, 0, 1, B), (53, 2, 5, P), (53, 6, 9, B), (53, 10, 21, P), (53, 22, 25, B), (53, 26, 37, P),
    (53, 38, 41, B), (53, 42, 45, P), (53, 46, 49, B), (53, 50, 53, P), (53, 54, 57, B), (53, 58, 61, P),
    (53, 62, 65, B), (53, 66, 73, P), (53, 74, 77, B), (53, 78, 85, P), (53, 86, 89, B), (53, 90, 93, P),
    (53, 94, 97, B), (53, 98, 99, P), (53, 100, 103, B), (53, 104, 107, P), (53, 108, 111, B),
    (53, 112, 117, P), (53, 118, 121, B), (53, 122, 131, P), (53, 132, 135, B), (53, 136, 141, P),
    (53, 142, 145, B), (53, 146, 149, P), (53, 150, 153, B), (53, 154, 155, P), (53, 156, 161, B),
    (53, 162, 165, P), (53, 166, 169, B), (53, 170, 173, P), (53, 174, 177, B), (53, 178, 181, P),
    (53, 182, 185, B), (53, 186, 197, P), (53, 198, 201, B), (53, 202, 205, P), (53, 206, 209, B),
    (53, 210, 211, P), (53, 212, 215, B), (53, 216, 219, P), (53, 220, 223, B), (53, 224, 237, P),
    (53, 238, 241, B), (53, 242, 249, P), (53, 250, 253, B), (53, 254, 259, P), (53, 580, 581, P),
    (53, 582, 585, B), (53, 586, 589, P), (53, 590, 593, B), (53, 594, 597, P), (53, 598, 601, B),
    (53, 602, 605, P), (53, 606, 609, B), (53, 610, 613, P), (53, 614, 617, B), (53, 618, 621, P),
    (53, 622, 625, B), (53, 626, 629, P), (53, 630, 633, B), (53, 634, 637, P), (53, 638, 639, B),
    (54, 0, 1, B), (54, 2, 5, P), (54, 6, 9, B), (54, 10, 21, P), (54, 22, 25, B), (54, 26, 37, P),
    (54, 38, 41, B), (54, 42, 45, P), (54, 46, 49, B), (54, 50, 53, P), (54, 54, 57, B), (54, 58, 61, P),
    (54, 62, 65, B), (54, 66, 73, P), (54, 74, 77, B), (54, 78, 85, P), (54, 86, 89, B), (54, 90, 93, P),
    (54, 94, 97, B), (54, 98, 99, P), (54, 100, 103, B), (54, 104, 107, P), (54, 108, 111, B),
    (54, 112, 117, P), (54, 118, 121, B), (54, 122, 131, P), (54, 132, 135, B), (54, 136, 141, P),
    (54, 142, 145, B), (54, 146, 149, P), (54, 150, 153, B), (54, 154, 155, P), (54, 156, 161, B),
    (54, 162, 165, P), (54, 166, 169, B), (54, 170, 173, P), (54, 174, 177, B), (54, 178, 181, P),
    (54, 182, 185, B), (54, 186, 197, P), (54, 198, 201, B), (54, 202, 205, P), (54, 206, 209, B),
    (54, 210, 211, P), (54, 212, 215, B), (54, 216, 219, P), (54, 220, 223, B), (54, 224, 237, P),
    (54, 238, 241, B), (54, 242, 249, P), (54, 250, 253, B), (54, 254, 259, P), (54, 580, 581, P),
    (54, 582, 585, B), (54, 586, 589, P), (54, 590, 593, B), (54, 594, 597, P), (54, 598, 601, B),
    (54, 602, 605, P), (54, 606, 609, B), (54, 610, 613, P), (54, 614, 617, B), (54, 618, 621, P),
    (54, 622, 625, B), (54, 626, 629, P), (54, 630, 633, B), (54, 634, 635, P), (54, 636, 639, B),
    (55, 0, 1, B), (55, 2, 5, P), (55, 6, 9, B), (55, 10, 21, P), (55, 22, 25, B), (55, 26, 37, P),
    (55, 38, 41, B), (55, 42, 45, P), (55, 46, 49, B), (55, 50, 53, P), (55, 54, 57, B), (55, 58, 61, P),
    (55, 62, 65, B), (55, 66, 73, P), (55, 74, 77, B), (55, 78, 85, P), (55, 86, 89, B), (55, 90, 93, P),
    (55, 94, 97, B), (55, 98, 99, P), (55, 100, 103, B), (55, 104, 107, P), (55, 108, 111, B),
    (55, 112, 117, P), (55, 118, 121, B), (55, 122, 131, P), (55, 132, 135, B), (55, 136, 141, P),
    (55, 142, 145, B), (55, 146, 149, P), (55, 150, 153, B), (55, 154, 157, P), (55, 158, 161, B),
    (55, 162, 165, P), (55, 166, 169, B), (55, 170, 173, P), (55, 174, 177, B), (55, 178, 181, P),
    (55, 182, 185, B), (55, 186, 197, P), (55, 198, 201, B), (55, 202, 203, P), (55, 204, 205, B),
    (55, 206, 207, P), (55, 208, 209, B), (55, 210, 211, P), (55, 212, 215, B), (55, 216, 219, P),
    (55, 220, 223, B), (55, 224, 237, P), (55, 238, 241, B), (55, 242, 249, P), (55, 250, 253, B),
    (55, 254, 259, P), (55, 580, 581, P), (55, 582, 585, B), (55, 586, 589, P), (55, 590, 593, B),
    (55, 594, 597, P), (55, 598, 601, B), (55, 602, 605, P), (55, 606, 609, B), (55, 610, 613, P),
    (55, 614, 617, B), (55, 618, 621, P), (55, 622, 625, B), (55, 626, 629, P), (55, 630, 633, B),
    (55, 634, 635, P), (55, 636, 639, B), (56, 0, 1, B), (56, 2, 5, P), (56, 6, 9, B), (56, 10, 21, P),
    (56, 22, 25, B), (56, 26, 37, P), (56, 38, 41, B), (56, 42, 45, P), (56, 46, 49, B), (56, 50, 53, P),
    (56, 54, 57, B), (56, 58, 61, P), (56, 62, 65, B), (56, 66, 73, P), (56, 74, 77, B), (56, 78, 85, P),
    (56, 86, 89, B), (56, 90, 93, P), (56, 94, 97, B), (56, 98, 99, P), (56, 100, 103, B), (56, 104, 107, P),
    (56, 108, 111, B), (56, 112, 117, P), (56, 118, 121, B), (56, 122, 131, P), (56, 132, 135, B),
    (56, 136, 141, P), (56, 142, 145, B), (56, 146, 149, P), (56, 150, 153, B), (56, 154, 157, P),
    (56, 158, 161, B), (56, 162, 165, P), (56, 166, 169, B), (56, 170, 173, P), (56, 174, 177, B),
    (56, 178, 181, P), (56, 182, 185, B), (56, 186, 197, P), (56, 198, 201, B), (56, 202, 203, P),
    (56, 204, 205, B), (56, 206, 207, P), (56, 208, 209, B), (56, 210, 211, P), (56, 212, 215, B),
    (56, 216, 219, P), (56, 220, 223, B), (56, 224, 237, P), (56, 238, 241, B), (56, 242, 249, P),
    (56, 250, 253, B), (56, 254, 259, P), (56, 580, 581, P), (56, 582, 585, B), (56, 586, 589, P),
    (56, 590, 593, B), (56, 594, 597, P), (56, 598, 609, B), (56, 610, 613, P), (56, 614, 625, B),
    (56, 626, 629, P), (56, 630, 639, B), (57, 0, 5, P), (57, 6, 17, B), (57, 18, 21, P), (57, 22, 25, B),
    (57, 26, 37, P), (57, 38, 49, B), (57, 50, 53, P), (57, 54, 57, B), (57, 58, 61, P), (57, 62, 65, B),
    (57, 66, 69, P), (57, 70, 81, B), (57, 82, 85, P), (57, 86, 97, B), (57, 98, 99, P), (57, 100, 103, B),
    (57, 104, 109, P), (57, 110, 113, B), (57, 114, 117, P), (57, 118, 129, B), (57, 130, 131, P),
    (57, 132, 135, B), (57, 136, 141, P), (57, 142, 145, B), (57, 146, 149, P), (57, 150, 153, B),
    (57, 154, 157, P), (57, 158, 161, B), (57, 162, 165, P), (57, 166, 177, B), (57, 178, 181, P),
    (57, 182, 185, B), (57, 186, 197, P), (57, 198, 207, B), (57, 208, 211, P), (57, 212, 215, B),
    (57, 216, 221, P), (57, 222, 225, B), (57, 226, 229, P), (57, 230, 241, B), (57, 242, 249, P),
    (57, 250, 253, B), (57, 254, 259, P), (57, 580, 581, P), (57, 582, 585, B), (57, 586, 589, P),
    (57, 590, 593, B), (57, 594, 597, P), (57, 598, 609, B), (57, 610, 613, P), (57, 614, 625, B),
    (57, 626, 629, P), (57, 630, 639, B), (58, 0, 5, P), (58, 6, 17, B), (58, 18, 21, P), (58, 22, 25, B),
    (58, 26, 37, P), (58, 38, 49, B), (58, 50, 53, P), (58, 54, 57, B), (58, 58, 61, P), (58, 62, 65, B),
    (58, 66, 69, P), (58, 70, 81, B), (58, 82, 85, P), (58, 86, 97, B), (58, 98, 99, P), (58, 100, 103, B),
    (58, 104, 109, P), (58, 110, 113, B), (58, 114, 117, P), (58, 118, 129, B), (58, 130, 131, P),
    (58, 132, 135, B), (58, 136, 141, P), (58, 142, 145, B), (58, 146, 149, P), (58, 150, 153, B),
    (58, 154, 157, P), (58, 158, 161, B), (58, 162, 165, P), (58, 166, 177, B), (58, 178, 181, P),
    (58, 182, 185, B), (58, 186, 197, P), (58, 198, 207, B), (58, 208, 211, P), (58, 212, 215, B),
    (58, 216, 221, P), (58, 222, 225, B), (58, 226, 229, P), (58, 230, 241, B), (58, 242, 249, P),
    (58, 250, 253, B), (58, 254, 259, P), (58, 580, 581, P), (58, 582, 585, B), (58, 586, 589, P),
    (58, 590, 593, B), (58, 594, 597, P), (58, 598, 607, B), (58, 608, 615, P), (58, 616, 623, B),
    (58, 624, 629, P), (58, 630, 637, B), (58, 638, 639, P), (59, 0, 5, P), (59, 6, 17, B), (59, 18, 21, P),
    (59, 22, 25, B), (59, 26, 39, P), (59, 40, 47, B), (59, 48, 53, P), (59, 54, 57, B), (59, 58, 61, P),
    (59, 62, 65, B), (59, 66, 69, P), (59, 70, 81, B), (59, 82, 87, P), (59, 88, 95, B), (59, 96, 99, P),
    (59, 100, 103, B), (59, 104, 109, P), (59, 110, 113, B), (59, 114, 117, P), (59, 118, 129, B),
    (59, 130, 131, P), (59, 132, 135, B), (59, 136, 141, P), (59, 142, 145, B), (59, 146, 149, P),
    (59, 150, 153, B), (59, 154, 157, P), (59, 158, 161, B), (59, 162, 167, P), (59, 168, 175, B),
    (59, 176, 181, P), (59, 182, 185, B), (59, 186, 199, P), (59, 200, 203, B), (59, 204, 205, P),
    (59, 206, 209, B), (59, 210, 211, P), (59, 212, 215, B), (59, 216, 221, P), (59, 222, 225, B),
    (59, 226, 229, P), (59, 230, 239, B), (59, 240, 249, P), (59, 250, 253, B), (59, 254, 259, P),
    (59, 580, 581, P), (59, 582, 585, B), (59, 586, 589, P), (59, 590, 593, B), (59, 594, 597, P),
    (59, 598, 607, B), (59, 608, 615, P), (59, 616, 623, B), (59, 624, 629, P), (59, 630, 637, B),
    (59, 638, 639, P), (60, 0, 5, P), (60, 6, 17, B), (60, 18, 21, P), (60, 22, 25, B), (60, 26, 39, P),
    (60, 40, 47, B), (60, 48, 53, P), (60, 54, 57, B), (60, 58, 61, P), (60, 62, 65, B), (60, 66, 69, P),
    (60, 70, 81, B), (60, 82, 87, P), (60, 88, 95, B), (60, 96, 99, P), (60, 100, 103, B), (60, 104, 109, P),
    (60, 110, 113, B), (60, 114, 117, P), (60, 118, 129, B), (60, 130, 131, P), (60, 132, 135, B),
    (60, 136, 141, P), (60, 142, 145, B), (60, 146, 149, P), (60, 150, 153, B), (60, 154, 157, P),
    (60, 158, 161, B), (60, 162, 167, P), (60, 168, 175, B), (60, 176, 181, P), (60, 182, 185, B),
    (60, 186, 199, P), (60, 200, 203, B), (60, 204, 205, P), (60, 206, 209, B), (60, 210, 211, P),
    (60, 212, 215, B), (60, 216, 221, P), (60, 222, 225, B), (60, 226, 229, P), (60, 230, 239, B),
    (60, 240, 249, P), (60, 250, 253, B), (60, 254, 259, P), (60, 580, 639, P), (61, 0, 259, P),
    (61, 580, 639, P), (62, 0, 259, P), (62, 580, 639, P), (63, 0, 259, P), (63, 580, 639, P),
    (64, 0, 259, P),
];
/// T14H: grab (0,170) 640x40, record 312 of probe/e1.
#[rustfmt::skip]
const SP_T14H: &[Span] = &[
    (173, 10, 639, P), (174, 0, 639, P), (175, 0, 639, P), (176, 0, 639, P), (177, 0, 639, P),
    (178, 0, 639, P), (179, 0, 639, P), (180, 0, 639, P), (181, 0, 639, P), (182, 0, 639, P),
    (183, 0, 639, P), (184, 0, 639, P), (185, 0, 639, P), (186, 0, 639, P), (187, 0, 639, P),
    (188, 0, 639, P), (189, 0, 639, P), (190, 0, 639, P), (191, 0, 639, P), (192, 0, 639, P),
    (193, 0, 639, P), (194, 0, 639, P), (195, 0, 639, P), (196, 0, 639, P), (197, 0, 639, P),
    (198, 0, 639, P), (199, 0, 639, P), (200, 0, 639, P), (201, 0, 639, P), (202, 0, 639, P),
    (203, 0, 639, P), (204, 0, 639, P), (205, 0, 639, P), (206, 0, 639, P), (207, 0, 639, P),
    (208, 0, 639, P), (209, 0, 639, P),
];
/// T14L: grab (0,160) 64x50, record 316 of probe/e1.
#[rustfmt::skip]
const SP_T14L: &[Span] = &[
    (160, 0, 63, P), (161, 0, 63, P), (162, 0, 63, P), (163, 0, 63, P), (164, 0, 63, P), (165, 0, 63, P),
    (166, 0, 63, P), (167, 0, 63, P), (168, 0, 63, P), (169, 0, 63, P), (170, 0, 63, P), (171, 0, 63, P),
    (172, 0, 63, P), (173, 0, 63, P), (174, 0, 63, P), (175, 0, 19, P), (175, 20, 21, B), (175, 22, 23, P),
    (175, 24, 25, B), (175, 26, 63, P), (176, 0, 19, P), (176, 20, 21, B), (176, 22, 23, P), (176, 24, 25, B),
    (176, 26, 27, P), (176, 28, 39, B), (176, 40, 47, P), (176, 48, 51, B), (176, 52, 59, P),
    (176, 60, 63, B), (177, 0, 19, P), (177, 20, 21, B), (177, 22, 23, P), (177, 24, 25, B), (177, 26, 27, P),
    (177, 28, 39, B), (177, 40, 47, P), (177, 48, 51, B), (177, 52, 59, P), (177, 60, 63, B), (178, 0, 19, P),
    (178, 20, 21, B), (178, 22, 23, P), (178, 24, 25, B), (178, 26, 27, P), (178, 28, 39, B),
    (178, 40, 45, P), (178, 46, 53, B), (178, 54, 59, P), (178, 60, 63, B), (179, 0, 19, P), (179, 20, 21, B),
    (179, 22, 23, P), (179, 24, 25, B), (179, 26, 27, P), (179, 28, 39, B), (179, 40, 45, P),
    (179, 46, 53, B), (179, 54, 59, P), (179, 60, 63, B), (180, 0, 19, P), (180, 20, 21, B), (180, 22, 23, P),
    (180, 24, 25, B), (180, 26, 33, P), (180, 34, 37, B), (180, 38, 43, P), (180, 44, 55, B),
    (180, 56, 59, P), (180, 60, 63, B), (181, 0, 19, P), (181, 20, 21, B), (181, 22, 23, P), (181, 24, 25, B),
    (181, 26, 33, P), (181, 34, 37, B), (181, 38, 43, P), (181, 44, 55, B), (181, 56, 59, P),
    (181, 60, 63, B), (182, 0, 19, P), (182, 20, 21, B), (182, 22, 23, P), (182, 24, 25, B), (182, 26, 33, P),
    (182, 34, 37, B), (182, 38, 43, P), (182, 44, 47, B), (182, 48, 51, P), (182, 52, 55, B),
    (182, 56, 59, P), (182, 60, 63, B), (183, 0, 17, P), (183, 18, 21, B), (183, 22, 23, P), (183, 24, 25, B),
    (183, 26, 33, P), (183, 34, 37, B), (183, 38, 43, P), (183, 44, 47, B), (183, 48, 51, P),
    (183, 52, 55, B), (183, 56, 59, P), (183, 60, 63, B), (184, 0, 17, P), (184, 18, 21, B), (184, 22, 23, P),
    (184, 24, 25, B), (184, 26, 31, P), (184, 32, 35, B), (184, 36, 43, P), (184, 44, 47, B),
    (184, 48, 51, P), (184, 52, 55, B), (184, 56, 59, P), (184, 60, 63, B), (185, 0, 17, P), (185, 18, 21, B),
    (185, 22, 23, P), (185, 24, 25, B), (185, 26, 31, P), (185, 32, 35, B), (185, 36, 43, P),
    (185, 44, 47, B), (185, 48, 51, P), (185, 52, 55, B), (185, 56, 59, P), (185, 60, 63, B), (186, 0, 17, P),
    (186, 18, 21, B), (186, 22, 23, P), (186, 24, 25, B), (186, 26, 31, P), (186, 32, 35, B),
    (186, 36, 43, P), (186, 44, 47, B), (186, 48, 51, P), (186, 52, 55, B), (186, 56, 59, P),
    (186, 60, 63, B), (187, 0, 15, P), (187, 16, 21, B), (187, 22, 23, P), (187, 24, 25, B), (187, 26, 31, P),
    (187, 32, 35, B), (187, 36, 43, P), (187, 44, 47, B), (187, 48, 51, P), (187, 52, 55, B),
    (187, 56, 59, P), (187, 60, 63, B), (188, 0, 15, P), (188, 16, 21, B), (188, 22, 23, P), (188, 24, 25, B),
    (188, 26, 29, P), (188, 30, 33, B), (188, 34, 43, P), (188, 44, 55, B), (188, 56, 59, P),
    (188, 60, 63, B), (189, 0, 13, P), (189, 14, 19, B), (189, 20, 23, P), (189, 24, 25, B), (189, 26, 29, P),
    (189, 30, 33, B), (189, 34, 43, P), (189, 44, 55, B), (189, 56, 59, P), (189, 60, 63, B), (190, 0, 13, P),
    (190, 14, 19, B), (190, 20, 23, P), (190, 24, 25, B), (190, 26, 29, P), (190, 30, 33, B),
    (190, 34, 43, P), (190, 44, 55, B), (190, 56, 59, P), (190, 60, 63, B), (191, 0, 11, P), (191, 12, 19, B),
    (191, 20, 23, P), (191, 24, 25, B), (191, 26, 29, P), (191, 30, 33, B), (191, 34, 43, P),
    (191, 44, 55, B), (191, 56, 59, P), (191, 60, 63, B), (192, 0, 11, P), (192, 12, 19, B), (192, 20, 23, P),
    (192, 24, 25, B), (192, 26, 27, P), (192, 28, 31, B), (192, 32, 43, P), (192, 44, 47, B),
    (192, 48, 51, P), (192, 52, 55, B), (192, 56, 59, P), (192, 60, 63, B), (193, 0, 11, P), (193, 12, 17, B),
    (193, 18, 23, P), (193, 24, 25, B), (193, 26, 27, P), (193, 28, 31, B), (193, 32, 43, P),
    (193, 44, 47, B), (193, 48, 51, P), (193, 52, 55, B), (193, 56, 59, P), (193, 60, 63, B), (194, 0, 11, P),
    (194, 12, 17, B), (194, 18, 23, P), (194, 24, 25, B), (194, 26, 27, P), (194, 28, 31, B),
    (194, 32, 43, P), (194, 44, 47, B), (194, 48, 51, P), (194, 52, 55, B), (194, 56, 59, P),
    (194, 60, 63, B), (195, 0, 11, P), (195, 12, 17, B), (195, 18, 23, P), (195, 24, 25, B), (195, 26, 27, P),
    (195, 28, 31, B), (195, 32, 43, P), (195, 44, 47, B), (195, 48, 51, P), (195, 52, 55, B),
    (195, 56, 59, P), (195, 60, 63, B), (196, 0, 11, P), (196, 12, 17, B), (196, 18, 23, P), (196, 24, 25, B),
    (196, 26, 27, P), (196, 28, 39, B), (196, 40, 43, P), (196, 44, 47, B), (196, 48, 51, P),
    (196, 52, 55, B), (196, 56, 59, P), (196, 60, 63, B), (197, 0, 7, B), (197, 8, 11, P), (197, 12, 15, B),
    (197, 16, 23, P), (197, 24, 25, B), (197, 26, 27, P), (197, 28, 39, B), (197, 40, 43, P),
    (197, 44, 47, B), (197, 48, 51, P), (197, 52, 55, B), (197, 56, 59, P), (197, 60, 63, B), (198, 0, 7, B),
    (198, 8, 11, P), (198, 12, 15, B), (198, 16, 23, P), (198, 24, 25, B), (198, 26, 27, P), (198, 28, 39, B),
    (198, 40, 43, P), (198, 44, 47, B), (198, 48, 51, P), (198, 52, 55, B), (198, 56, 59, P),
    (198, 60, 63, B), (199, 0, 7, B), (199, 8, 11, P), (199, 12, 13, B), (199, 14, 23, P), (199, 24, 25, B),
    (199, 26, 27, P), (199, 28, 39, B), (199, 40, 43, P), (199, 44, 47, B), (199, 48, 51, P),
    (199, 52, 55, B), (199, 56, 59, P), (199, 60, 63, B), (200, 0, 7, B), (200, 8, 11, P), (200, 12, 13, B),
    (200, 14, 23, P), (200, 24, 25, B), (200, 26, 63, P), (201, 0, 63, P), (202, 0, 63, P), (203, 0, 63, P),
    (204, 0, 63, P), (205, 0, 63, P), (206, 0, 63, P), (207, 0, 63, P), (208, 0, 63, P), (209, 0, 63, P),
];

// ── the V30 run's cases ──────────────────────────────────────────────────

/// TVA: grab (70,200) 60x120, record 373 of `measurements/measurement-2026-11-06-v30/probe/e1`.
#[rustfmt::skip]
const SP_TVA: &[Span] = &[
    (253, 96, 100, P), (253, 101, 101, B), (254, 96, 101, B), (255, 96, 97, P), (255, 98, 98, B),
    (255, 99, 100, P), (255, 101, 101, B), (256, 96, 97, P), (256, 98, 98, B), (256, 99, 100, P),
    (256, 101, 101, B), (257, 96, 97, P), (257, 98, 98, B), (257, 99, 100, P), (257, 101, 101, B),
    (258, 96, 101, B), (259, 96, 100, P), (259, 101, 101, B), (260, 96, 96, B), (260, 97, 97, P),
    (260, 98, 101, B), (261, 96, 96, B), (261, 97, 97, P), (261, 98, 98, B), (261, 99, 99, P),
    (261, 100, 101, B), (262, 96, 96, B), (262, 97, 99, P), (262, 100, 101, B), (263, 96, 96, B),
    (263, 97, 99, P), (263, 100, 101, B), (264, 96, 96, P), (264, 97, 99, B), (264, 100, 100, P),
    (264, 101, 101, B), (265, 96, 100, P), (265, 101, 101, B), (266, 96, 96, B), (266, 97, 100, P),
    (266, 101, 101, B), (267, 96, 96, B), (267, 97, 97, P), (267, 98, 98, B), (267, 99, 100, P),
    (267, 101, 101, B), (268, 96, 96, B), (268, 97, 97, P), (268, 98, 98, B), (268, 99, 100, P),
    (268, 101, 101, B), (269, 96, 96, B), (269, 97, 97, P), (269, 98, 98, B), (269, 99, 100, P),
    (269, 101, 101, B), (270, 96, 101, B), (271, 96, 100, P), (271, 101, 101, B), (272, 96, 96, B),
    (272, 97, 99, P), (272, 100, 101, B), (273, 96, 96, B), (273, 97, 97, P), (273, 98, 98, B),
    (273, 99, 99, P), (273, 100, 101, B), (274, 96, 96, B), (274, 97, 97, P), (274, 98, 98, B),
    (274, 99, 99, P), (274, 100, 101, B), (275, 96, 96, B), (275, 97, 97, P), (275, 98, 98, B),
    (275, 99, 99, P), (275, 100, 101, B), (276, 96, 101, B), (277, 96, 100, P), (277, 101, 101, B),
    (278, 96, 96, P), (278, 97, 99, B), (278, 100, 100, P), (278, 101, 101, B), (279, 96, 96, B),
    (279, 97, 99, P), (279, 100, 101, B), (280, 96, 96, B), (280, 97, 99, P), (280, 100, 101, B),
    (281, 96, 96, B), (281, 97, 99, P), (281, 100, 101, B), (282, 96, 101, B), (283, 96, 100, P),
    (283, 101, 101, B), (284, 96, 96, B), (284, 97, 99, P), (284, 100, 101, B), (285, 96, 96, B),
    (285, 97, 99, P), (285, 100, 101, B), (286, 96, 96, B), (286, 97, 99, P), (286, 100, 101, B),
    (287, 96, 96, B), (287, 97, 99, P), (287, 100, 101, B), (288, 96, 96, P), (288, 97, 99, B),
    (288, 100, 100, P), (288, 101, 101, B), (289, 96, 100, P), (289, 101, 101, B), (290, 96, 96, P),
    (290, 97, 97, B), (290, 98, 98, P), (290, 99, 99, B), (290, 100, 100, P), (290, 101, 101, B),
    (291, 96, 96, B), (291, 97, 97, P), (291, 98, 98, B), (291, 99, 99, P), (291, 100, 101, B),
    (292, 96, 96, B), (292, 97, 97, P), (292, 98, 98, B), (292, 99, 99, P), (292, 100, 101, B),
    (293, 96, 96, B), (293, 97, 97, P), (293, 98, 98, B), (293, 99, 99, P), (293, 100, 101, B),
    (294, 96, 101, B), (295, 96, 100, P), (295, 101, 101, B), (296, 96, 96, P), (296, 97, 101, B),
    (297, 96, 96, B), (297, 97, 97, P), (297, 98, 98, B), (297, 99, 100, P), (297, 101, 101, B),
    (298, 96, 96, B), (298, 97, 97, P), (298, 98, 98, B), (298, 99, 100, P), (298, 101, 101, B),
    (299, 96, 96, B), (299, 97, 97, P), (299, 98, 98, B), (299, 99, 100, P), (299, 101, 101, B),
    (300, 96, 96, P), (300, 97, 101, B), (301, 101, 101, B),
];

/// TVB: grab (600,200) 40x120, record 374 of `measurements/measurement-2026-11-06-v30/probe/e1`.
#[rustfmt::skip]
const SP_TVB: &[Span] = &[
    (253, 638, 639, P), (254, 638, 639, B), (255, 638, 639, P), (256, 638, 639, P), (257, 638, 639, P),
    (258, 638, 639, B), (259, 638, 639, P), (260, 638, 638, B), (260, 639, 639, P), (261, 638, 638, B),
    (261, 639, 639, P), (262, 638, 638, B), (262, 639, 639, P), (263, 638, 638, B), (263, 639, 639, P),
    (264, 638, 638, P), (264, 639, 639, B), (265, 638, 639, P), (266, 638, 638, B), (266, 639, 639, P),
    (267, 638, 638, B), (267, 639, 639, P), (268, 638, 638, B), (268, 639, 639, P), (269, 638, 638, B),
    (269, 639, 639, P), (270, 638, 639, B), (271, 638, 639, P), (272, 638, 638, B), (272, 639, 639, P),
    (273, 638, 638, B), (273, 639, 639, P), (274, 638, 638, B), (274, 639, 639, P), (275, 638, 638, B),
    (275, 639, 639, P), (276, 638, 639, B), (277, 638, 639, P), (278, 638, 638, P), (278, 639, 639, B),
    (279, 638, 638, B), (279, 639, 639, P), (280, 638, 638, B), (280, 639, 639, P), (281, 638, 638, B),
    (281, 639, 639, P), (282, 638, 639, B), (283, 638, 639, P), (284, 638, 638, B), (284, 639, 639, P),
    (285, 638, 638, B), (285, 639, 639, P), (286, 638, 638, B), (286, 639, 639, P), (287, 638, 638, B),
    (287, 639, 639, P), (288, 638, 638, P), (288, 639, 639, B), (289, 638, 639, P), (290, 638, 638, P),
    (290, 639, 639, B), (291, 638, 638, B), (291, 639, 639, P), (292, 638, 638, B), (292, 639, 639, P),
    (293, 638, 638, B), (293, 639, 639, P), (294, 638, 639, B), (295, 638, 639, P), (296, 638, 638, P),
    (296, 639, 639, B), (297, 638, 638, B), (297, 639, 639, P), (298, 638, 638, B), (298, 639, 639, P),
    (299, 638, 638, B), (299, 639, 639, P), (300, 638, 638, P), (300, 639, 639, B),
];

/// TVBw: grab (0,200) 40x120, record 374 of `measurements/measurement-2026-11-06-v30/probe/e1`.
#[rustfmt::skip]
const SP_TVBW: &[Span] = &[
    (254, 0, 2, P), (254, 3, 3, B), (255, 0, 3, B), (256, 0, 0, B), (256, 1, 2, P), (256, 3, 3, B),
    (257, 0, 0, B), (257, 1, 2, P), (257, 3, 3, B), (258, 0, 0, B), (258, 1, 2, P), (258, 3, 3, B),
    (259, 0, 3, B), (260, 0, 2, P), (260, 3, 3, B), (261, 0, 3, B), (262, 0, 0, B), (262, 1, 1, P),
    (262, 2, 3, B), (263, 0, 1, P), (263, 2, 3, B), (264, 0, 1, P), (264, 2, 3, B), (265, 0, 1, B),
    (265, 2, 2, P), (265, 3, 3, B), (266, 0, 2, P), (266, 3, 3, B), (267, 0, 2, P), (267, 3, 3, B),
    (268, 0, 0, B), (268, 1, 2, P), (268, 3, 3, B), (269, 0, 0, B), (269, 1, 2, P), (269, 3, 3, B),
    (270, 0, 0, B), (270, 1, 2, P), (270, 3, 3, B), (271, 0, 3, B), (272, 0, 2, P), (272, 3, 3, B),
    (273, 0, 1, P), (273, 2, 3, B), (274, 0, 0, B), (274, 1, 1, P), (274, 2, 3, B), (275, 0, 0, B),
    (275, 1, 1, P), (275, 2, 3, B), (276, 0, 0, B), (276, 1, 1, P), (276, 2, 3, B), (277, 0, 3, B),
    (278, 0, 2, P), (278, 3, 3, B), (279, 0, 1, B), (279, 2, 2, P), (279, 3, 3, B), (280, 0, 1, P),
    (280, 2, 3, B), (281, 0, 1, P), (281, 2, 3, B), (282, 0, 1, P), (282, 2, 3, B), (283, 0, 3, B),
    (284, 0, 2, P), (284, 3, 3, B), (285, 0, 1, P), (285, 2, 3, B), (286, 0, 1, P), (286, 2, 3, B),
    (287, 0, 1, P), (287, 2, 3, B), (288, 0, 1, P), (288, 2, 3, B), (289, 0, 1, B), (289, 2, 2, P),
    (289, 3, 3, B), (290, 0, 2, P), (290, 3, 3, B), (291, 0, 0, P), (291, 1, 1, B), (291, 2, 2, P),
    (291, 3, 3, B), (292, 0, 0, B), (292, 1, 1, P), (292, 2, 3, B), (293, 0, 0, B), (293, 1, 1, P),
    (293, 2, 3, B), (294, 0, 0, B), (294, 1, 1, P), (294, 2, 3, B), (295, 0, 3, B), (296, 0, 2, P),
    (296, 3, 3, B), (297, 0, 3, B), (298, 0, 0, B), (298, 1, 2, P), (298, 3, 3, B), (299, 0, 0, B),
    (299, 1, 2, P), (299, 3, 3, B), (300, 0, 0, B), (300, 1, 2, P), (300, 3, 3, B), (301, 0, 3, B),
    (302, 3, 3, B),
];

/// TVC: grab (0,170) 60x50, record 375 of `measurements/measurement-2026-11-06-v30/probe/e1`.
#[rustfmt::skip]
const SP_TVC: &[Span] = &[
    (199, 0, 21, B), (200, 0, 2, B), (200, 3, 3, P), (200, 4, 7, B), (200, 8, 10, P), (200, 11, 14, B),
    (200, 15, 15, P), (200, 16, 16, B), (200, 17, 19, P), (200, 20, 20, B), (201, 0, 1, P), (201, 2, 2, B),
    (201, 3, 7, P), (201, 8, 8, B), (201, 9, 9, P), (201, 10, 10, B), (201, 11, 13, P), (201, 14, 14, B),
    (201, 15, 15, P), (201, 16, 16, B), (201, 17, 19, P), (201, 20, 20, B), (202, 0, 1, P), (202, 2, 2, B),
    (202, 3, 7, P), (202, 8, 8, B), (202, 9, 10, P), (202, 11, 14, B), (202, 15, 15, P), (202, 16, 20, B),
    (203, 0, 1, P), (203, 2, 2, B), (203, 3, 7, P), (203, 8, 8, B), (203, 9, 9, P), (203, 10, 10, B),
    (203, 11, 13, P), (203, 14, 14, B), (203, 15, 15, P), (203, 16, 16, B), (203, 17, 19, P),
    (203, 20, 20, B), (204, 0, 2, B), (204, 3, 3, P), (204, 4, 7, B), (204, 8, 10, P), (204, 11, 14, B),
    (204, 15, 16, P), (204, 17, 19, B), (204, 20, 20, P),
];

/// TVCw: grab (590,170) 50x50, record 375 of `measurements/measurement-2026-11-06-v30/probe/e1`.
#[rustfmt::skip]
const SP_TVCW: &[Span] = &[
    (198, 613, 639, B), (199, 613, 613, P), (199, 614, 614, B), (199, 615, 617, P), (199, 618, 618, B),
    (199, 619, 619, P), (199, 620, 623, B), (199, 624, 629, P), (199, 630, 630, B), (199, 631, 631, P),
    (199, 632, 636, B), (199, 637, 638, P), (199, 639, 639, B), (200, 613, 613, P), (200, 614, 614, B),
    (200, 615, 617, P), (200, 618, 618, B), (200, 619, 619, P), (200, 620, 620, B), (200, 621, 623, P),
    (200, 624, 624, B), (200, 625, 629, P), (200, 630, 630, B), (200, 631, 635, P), (200, 636, 636, B),
    (200, 637, 637, P), (200, 638, 638, B), (200, 639, 639, P), (201, 613, 613, P), (201, 614, 618, B),
    (201, 619, 619, P), (201, 620, 621, B), (201, 622, 623, P), (201, 624, 624, B), (201, 625, 626, P),
    (201, 627, 630, B), (201, 631, 632, P), (201, 633, 636, B), (201, 637, 637, P), (201, 638, 638, B),
    (201, 639, 639, P), (202, 613, 613, P), (202, 614, 614, B), (202, 615, 617, P), (202, 618, 618, B),
    (202, 619, 623, P), (202, 624, 624, B), (202, 625, 629, P), (202, 630, 630, B), (202, 631, 635, P),
    (202, 636, 636, B), (202, 637, 637, P), (202, 638, 638, B), (202, 639, 639, P), (203, 613, 613, P),
    (203, 614, 614, B), (203, 615, 617, P), (203, 618, 618, B), (203, 619, 619, P), (203, 620, 623, B),
    (203, 624, 625, P), (203, 626, 630, B), (203, 631, 631, P), (203, 632, 636, B), (203, 637, 638, P),
    (203, 639, 639, B),
];

/// TVD: grab (0,180) 40x80, record 376 of `measurements/measurement-2026-11-06-v30/probe/e1`.
#[rustfmt::skip]
const SP_TVD: &[Span] = &[
    (200, 0, 1, B), (200, 2, 2, P), (201, 0, 0, B), (201, 1, 1, P), (201, 2, 2, B), (202, 0, 0, B),
    (202, 1, 1, P), (202, 2, 2, B), (203, 0, 0, B), (203, 1, 1, P), (203, 2, 2, B), (204, 0, 1, B),
    (204, 2, 2, P), (205, 0, 2, P), (206, 0, 2, B), (207, 0, 0, B), (207, 1, 1, P), (207, 2, 2, B),
    (208, 0, 0, B), (208, 1, 1, P), (208, 2, 2, B), (209, 0, 0, B), (209, 1, 1, P), (209, 2, 2, B),
    (210, 0, 0, P), (210, 1, 1, B), (210, 2, 2, P), (211, 0, 2, P), (212, 0, 1, B), (212, 2, 2, P),
    (213, 0, 1, P), (213, 2, 2, B), (214, 0, 1, P), (214, 2, 2, B), (215, 0, 1, P), (215, 2, 2, B),
    (216, 0, 1, P), (216, 2, 2, B), (217, 0, 2, P), (218, 0, 2, B), (219, 0, 1, P), (219, 2, 2, B),
    (220, 0, 1, P), (220, 2, 2, B), (221, 0, 1, P), (221, 2, 2, B), (222, 0, 1, B), (222, 2, 2, P),
    (223, 0, 2, P), (224, 0, 2, B), (225, 0, 0, B), (225, 1, 1, P), (225, 2, 2, B), (226, 0, 0, B),
    (226, 1, 1, P), (226, 2, 2, B), (227, 0, 0, B), (227, 1, 1, P), (227, 2, 2, B), (228, 0, 1, P),
    (228, 2, 2, B), (229, 0, 2, P), (230, 0, 2, B), (231, 0, 0, B), (231, 1, 1, P), (231, 2, 2, B),
    (232, 0, 0, B), (232, 1, 1, P), (232, 2, 2, B), (233, 0, 0, B), (233, 1, 1, P), (233, 2, 2, B),
    (234, 0, 1, P), (234, 2, 2, B), (235, 0, 2, P), (236, 0, 1, B), (236, 2, 2, P), (237, 0, 1, P),
    (237, 2, 2, B), (238, 0, 1, P), (238, 2, 2, B), (239, 0, 0, B), (239, 1, 1, P), (239, 2, 2, B),
    (240, 0, 0, B), (240, 1, 1, P), (240, 2, 2, B), (241, 0, 2, P), (242, 0, 2, B), (243, 0, 0, B),
    (243, 1, 2, P), (244, 0, 0, B), (244, 1, 2, P), (245, 0, 0, B), (245, 1, 2, P), (246, 0, 2, B),
    (247, 0, 2, P),
];

/// TVDw: grab (600,180) 40x80, record 376 of `measurements/measurement-2026-11-06-v30/probe/e1`.
#[rustfmt::skip]
const SP_TVDW: &[Span] = &[
    (199, 637, 639, B), (200, 637, 637, B), (200, 638, 639, P), (201, 637, 637, B), (201, 638, 639, P),
    (202, 637, 637, B), (202, 638, 639, P), (203, 637, 639, B), (204, 637, 637, B), (204, 638, 639, P),
    (205, 637, 639, B), (206, 637, 638, B), (206, 639, 639, P), (207, 637, 638, B), (207, 639, 639, P),
    (208, 637, 638, B), (208, 639, 639, P), (209, 637, 637, B), (209, 638, 638, P), (209, 639, 639, B),
    (210, 637, 637, B), (210, 638, 639, P), (211, 637, 637, B), (211, 638, 638, P), (211, 639, 639, B),
    (212, 637, 638, B), (212, 639, 639, P), (213, 637, 638, B), (213, 639, 639, P), (214, 637, 638, B),
    (214, 639, 639, P), (215, 637, 638, B), (215, 639, 639, P), (216, 637, 637, B), (216, 638, 639, P),
    (217, 637, 639, B), (218, 637, 638, B), (218, 639, 639, P), (219, 637, 638, B), (219, 639, 639, P),
    (220, 637, 638, B), (220, 639, 639, P), (221, 637, 637, B), (221, 638, 638, P), (221, 639, 639, B),
    (222, 637, 637, B), (222, 638, 639, P), (223, 637, 639, B), (224, 637, 638, B), (224, 639, 639, P),
    (225, 637, 638, B), (225, 639, 639, P), (226, 637, 638, B), (226, 639, 639, P), (227, 637, 638, B),
    (227, 639, 639, P), (228, 637, 637, B), (228, 638, 639, P), (229, 637, 639, B), (230, 637, 637, B),
    (230, 638, 639, P), (231, 637, 637, B), (231, 638, 639, P), (232, 637, 637, B), (232, 638, 639, P),
    (233, 637, 637, B), (233, 638, 639, P), (234, 637, 637, B), (234, 638, 639, P), (235, 637, 637, B),
    (235, 638, 638, P), (235, 639, 639, B), (236, 637, 638, B), (236, 639, 639, P), (237, 637, 638, B),
    (237, 639, 639, P), (238, 637, 638, B), (238, 639, 639, P), (239, 637, 639, B), (240, 637, 637, B),
    (240, 638, 639, P), (241, 637, 639, B), (242, 637, 637, B), (242, 638, 639, P), (243, 637, 637, B),
    (243, 638, 639, P), (244, 637, 637, B), (244, 638, 639, P), (245, 637, 639, B), (246, 637, 637, B),
    (246, 638, 639, P), (247, 637, 637, B),
];

/// TVE: grab (70,200) 60x120, record 377 of `measurements/measurement-2026-11-06-v30/probe/e1`.
#[rustfmt::skip]
const SP_TVE: &[Span] = &[
    (280, 96, 96, B), (280, 97, 99, P), (280, 100, 101, B), (281, 96, 96, B), (281, 97, 99, P),
    (281, 100, 101, B), (282, 96, 101, B), (283, 96, 100, P), (283, 101, 101, B), (284, 96, 96, B),
    (284, 97, 99, P), (284, 100, 101, B), (285, 96, 96, B), (285, 97, 99, P), (285, 100, 101, B),
    (286, 96, 96, B), (286, 97, 99, P), (286, 100, 101, B), (287, 96, 96, B), (287, 97, 99, P),
    (287, 100, 101, B), (288, 96, 96, P), (288, 97, 99, B), (288, 100, 100, P), (288, 101, 101, B),
    (289, 96, 100, P), (289, 101, 101, B), (290, 96, 96, P), (290, 97, 97, B), (290, 98, 98, P),
    (290, 99, 99, B), (290, 100, 100, P), (290, 101, 101, B), (291, 96, 96, B), (291, 97, 97, P),
    (291, 98, 98, B), (291, 99, 99, P), (291, 100, 101, B), (292, 96, 96, B), (292, 97, 97, P),
    (292, 98, 98, B), (292, 99, 99, P), (292, 100, 101, B), (293, 96, 96, B), (293, 97, 97, P),
    (293, 98, 98, B), (293, 99, 99, P), (293, 100, 101, B), (294, 96, 101, B), (295, 96, 100, P),
    (295, 101, 101, B),
];

/// TVF: grab (201,100) 4x220, record 378 of `measurements/measurement-2026-11-06-v30/probe/e1`.
#[rustfmt::skip]
const SP_TVF: &[Span] = &[
    (173, 201, 204, P), (174, 201, 202, P), (174, 203, 204, B), (175, 201, 204, P), (176, 201, 202, P),
    (176, 203, 204, B), (177, 201, 204, P), (178, 201, 202, P), (178, 203, 204, B), (179, 201, 204, P),
    (180, 201, 202, P), (180, 203, 204, B), (181, 201, 204, P), (182, 201, 202, P), (182, 203, 204, B),
    (183, 201, 204, P), (184, 201, 202, P), (184, 203, 204, B), (185, 201, 204, P), (186, 201, 202, P),
    (186, 203, 204, B), (187, 201, 204, P), (188, 201, 202, P), (188, 203, 204, B), (189, 201, 204, P),
    (190, 201, 202, P), (190, 203, 204, B), (191, 201, 204, P), (192, 201, 202, P), (192, 203, 204, B),
    (193, 201, 204, P), (194, 201, 202, P), (194, 203, 204, B), (195, 201, 204, P), (196, 201, 202, P),
    (196, 203, 204, B), (197, 201, 204, P), (198, 201, 202, P), (198, 203, 204, B), (199, 201, 204, P),
    (200, 201, 202, P), (200, 203, 204, B), (201, 201, 204, P), (202, 201, 202, P), (202, 203, 204, B),
    (203, 201, 204, P), (204, 201, 202, P), (204, 203, 204, B), (205, 201, 204, P), (206, 201, 202, P),
    (206, 203, 204, B), (207, 201, 204, P), (208, 201, 202, P), (208, 203, 204, B), (209, 201, 204, P),
    (210, 201, 202, P), (210, 203, 204, B), (211, 201, 204, P), (212, 201, 202, P), (212, 203, 204, B),
    (213, 201, 204, P), (214, 201, 202, P), (214, 203, 204, B), (215, 201, 204, P), (216, 201, 202, P),
    (216, 203, 204, B), (217, 201, 204, P), (218, 201, 202, P), (218, 203, 204, B), (219, 201, 204, P),
    (220, 201, 202, P), (220, 203, 204, B), (221, 201, 204, P), (222, 201, 202, P), (222, 203, 204, B),
    (223, 201, 204, P), (224, 201, 202, P), (224, 203, 204, B), (225, 201, 204, P), (226, 201, 202, P),
    (226, 203, 204, B), (227, 201, 204, P), (228, 201, 202, P), (228, 203, 204, B), (229, 201, 204, P),
    (230, 201, 202, P), (230, 203, 204, B), (231, 201, 204, P), (232, 201, 202, P), (232, 203, 204, B),
    (233, 201, 204, P), (234, 201, 202, P), (234, 203, 204, B), (235, 201, 204, P), (236, 201, 202, P),
    (236, 203, 204, B), (237, 201, 204, P), (238, 201, 202, P), (238, 203, 204, B), (239, 201, 204, P),
    (240, 201, 202, P), (240, 203, 204, B), (241, 201, 204, P), (242, 201, 202, P), (242, 203, 204, B),
    (243, 201, 204, P), (244, 201, 202, P), (244, 203, 204, B), (245, 201, 204, P), (246, 201, 202, P),
    (246, 203, 204, B), (247, 201, 204, P), (248, 201, 202, P), (248, 203, 204, B), (249, 201, 204, P),
    (250, 201, 202, P), (250, 203, 204, B), (251, 201, 204, P), (252, 201, 202, P), (252, 203, 204, B),
    (253, 201, 204, P), (254, 201, 202, P), (254, 203, 204, B), (255, 201, 204, P), (256, 201, 202, P),
    (256, 203, 204, B), (257, 201, 204, P), (258, 201, 202, P), (258, 203, 204, B), (259, 201, 204, P),
    (260, 201, 202, P), (260, 203, 204, B), (261, 201, 204, P), (262, 201, 202, P), (262, 203, 204, B),
    (263, 201, 204, P), (264, 201, 202, P), (264, 203, 204, B), (265, 201, 204, P), (266, 201, 202, P),
    (266, 203, 204, B), (267, 201, 204, P), (268, 201, 202, P), (268, 203, 204, B), (269, 201, 204, P),
    (270, 201, 202, P), (270, 203, 204, B), (271, 201, 204, P), (272, 201, 202, P), (272, 203, 204, B),
    (273, 201, 204, P), (274, 201, 202, P), (274, 203, 204, B), (275, 201, 204, P), (276, 201, 202, P),
    (276, 203, 204, B), (277, 201, 204, P), (278, 201, 202, P), (278, 203, 204, B), (279, 201, 204, P),
    (280, 201, 202, P), (280, 203, 204, B), (281, 201, 204, P), (282, 201, 202, P), (282, 203, 204, B),
    (283, 201, 204, P), (284, 201, 202, P), (284, 203, 204, B), (285, 201, 204, P), (286, 201, 202, P),
    (286, 203, 204, B), (287, 201, 204, P), (288, 201, 202, P), (288, 203, 204, B), (289, 201, 204, P),
    (290, 201, 202, P), (290, 203, 204, B), (291, 201, 204, P), (292, 201, 202, P), (292, 203, 204, B),
    (293, 201, 204, P), (294, 201, 202, P), (294, 203, 204, B), (295, 201, 204, P), (296, 201, 202, P),
    (296, 203, 204, B), (297, 201, 204, P), (298, 201, 202, P), (298, 203, 204, B), (299, 201, 204, P),
    (300, 201, 202, P), (300, 203, 204, B), (301, 203, 204, P),
];

/// TVG: grab (296,80) 4x220, record 379 of `measurements/measurement-2026-11-06-v30/probe/e1`.
#[rustfmt::skip]
const SP_TVG: &[Span] = &[
    (100, 296, 299, P), (101, 296, 297, B), (101, 298, 299, P), (102, 296, 299, P), (103, 296, 297, B),
    (103, 298, 299, P), (104, 296, 299, P), (105, 296, 297, B), (105, 298, 299, P), (106, 296, 299, P),
    (107, 296, 297, B), (107, 298, 299, P), (108, 296, 299, P), (109, 296, 297, B), (109, 298, 299, P),
    (110, 296, 299, P), (111, 296, 297, B), (111, 298, 299, P), (112, 296, 299, P), (113, 296, 297, B),
    (113, 298, 299, P), (114, 296, 299, P), (115, 296, 297, B), (115, 298, 299, P), (116, 296, 299, P),
    (117, 296, 297, B), (117, 298, 299, P), (118, 296, 299, P), (119, 296, 297, B), (119, 298, 299, P),
    (120, 296, 299, P), (121, 296, 297, B), (121, 298, 299, P), (122, 296, 299, P), (123, 296, 297, B),
    (123, 298, 299, P), (124, 296, 299, P), (125, 296, 297, B), (125, 298, 299, P), (126, 296, 299, P),
    (127, 296, 297, B), (127, 298, 299, P), (128, 296, 299, P), (129, 296, 297, B), (129, 298, 299, P),
    (130, 296, 299, P), (131, 296, 297, B), (131, 298, 299, P), (132, 296, 299, P), (133, 296, 297, B),
    (133, 298, 299, P), (134, 296, 299, P), (135, 296, 297, B), (135, 298, 299, P), (136, 296, 299, P),
    (137, 296, 297, B), (137, 298, 299, P), (138, 296, 299, P), (139, 296, 297, B), (139, 298, 299, P),
    (140, 296, 299, P), (141, 296, 297, B), (141, 298, 299, P), (142, 296, 299, P), (143, 296, 297, B),
    (143, 298, 299, P), (144, 296, 299, P), (145, 296, 297, B), (145, 298, 299, P), (146, 296, 299, P),
    (147, 296, 297, B), (147, 298, 299, P), (148, 296, 299, P), (149, 296, 297, B), (149, 298, 299, P),
    (150, 296, 299, P), (151, 296, 297, B), (151, 298, 299, P), (152, 296, 299, P), (153, 296, 297, B),
    (153, 298, 299, P), (154, 296, 299, P), (155, 296, 297, B), (155, 298, 299, P), (156, 296, 299, P),
    (157, 296, 297, B), (157, 298, 299, P), (158, 296, 299, P), (159, 296, 297, B), (159, 298, 299, P),
    (160, 296, 299, P), (161, 296, 297, B), (161, 298, 299, P), (162, 296, 299, P), (163, 296, 297, B),
    (163, 298, 299, P), (164, 296, 299, P), (165, 296, 297, B), (165, 298, 299, P), (166, 296, 299, P),
    (167, 296, 297, B), (167, 298, 299, P), (168, 296, 299, P), (169, 296, 297, B), (169, 298, 299, P),
    (170, 296, 299, P), (171, 296, 297, B), (171, 298, 299, P), (172, 296, 299, P), (173, 296, 297, B),
    (173, 298, 299, P), (174, 296, 299, P), (175, 296, 297, B), (175, 298, 299, P), (176, 296, 299, P),
    (177, 296, 297, B), (177, 298, 299, P), (178, 296, 299, P), (179, 296, 297, B), (179, 298, 299, P),
    (180, 296, 299, P), (181, 296, 297, B), (181, 298, 299, P), (182, 296, 299, P), (183, 296, 297, B),
    (183, 298, 299, P), (184, 296, 299, P), (185, 296, 297, B), (185, 298, 299, P), (186, 296, 299, P),
    (187, 296, 297, B), (187, 298, 299, P), (188, 296, 299, P), (189, 296, 297, B), (189, 298, 299, P),
    (190, 296, 299, P), (191, 296, 297, B), (191, 298, 299, P), (192, 296, 299, P), (193, 296, 297, B),
    (193, 298, 299, P), (194, 296, 299, P), (195, 296, 297, B), (195, 298, 299, P), (196, 296, 299, P),
    (197, 296, 297, B), (197, 298, 299, P), (198, 296, 299, P), (199, 296, 297, B), (199, 298, 299, P),
    (200, 296, 299, P), (201, 296, 297, B), (201, 298, 299, P), (202, 296, 299, P), (203, 296, 297, B),
    (203, 298, 299, P), (204, 296, 299, P), (205, 296, 297, B), (205, 298, 299, P), (206, 296, 299, P),
    (207, 296, 297, B), (207, 298, 299, P), (208, 296, 299, P), (209, 296, 297, B), (209, 298, 299, P),
    (210, 296, 299, P), (211, 296, 297, B), (211, 298, 299, P), (212, 296, 299, P), (213, 296, 297, B),
    (213, 298, 299, P), (214, 296, 299, P), (215, 296, 297, B), (215, 298, 299, P), (216, 296, 299, P),
    (217, 296, 297, B), (217, 298, 299, P), (218, 296, 299, P), (219, 296, 297, B), (219, 298, 299, P),
    (220, 296, 299, P), (221, 296, 297, B), (221, 298, 299, P), (222, 296, 299, P), (223, 296, 297, B),
    (223, 298, 299, P), (224, 296, 299, P), (225, 296, 297, B), (225, 298, 299, P), (226, 296, 299, P),
    (227, 296, 297, B), (227, 298, 299, P), (228, 296, 297, P),
];

/// TVH: grab (300,96) 220x4, record 380 of `measurements/measurement-2026-11-06-v30/probe/e1`.
#[rustfmt::skip]
const SP_TVH: &[Span] = &[
    (96, 373, 373, P), (96, 374, 374, B), (96, 375, 375, P), (96, 376, 376, B), (96, 377, 377, P),
    (96, 378, 378, B), (96, 379, 379, P), (96, 380, 380, B), (96, 381, 381, P), (96, 382, 382, B),
    (96, 383, 383, P), (96, 384, 384, B), (96, 385, 385, P), (96, 386, 386, B), (96, 387, 387, P),
    (96, 388, 388, B), (96, 389, 389, P), (96, 390, 390, B), (96, 391, 391, P), (96, 392, 392, B),
    (96, 393, 393, P), (96, 394, 394, B), (96, 395, 395, P), (96, 396, 396, B), (96, 397, 397, P),
    (96, 398, 398, B), (96, 399, 399, P), (96, 400, 400, B), (96, 401, 401, P), (96, 402, 402, B),
    (96, 403, 403, P), (96, 404, 404, B), (96, 405, 405, P), (96, 406, 406, B), (96, 407, 407, P),
    (96, 408, 408, B), (96, 409, 409, P), (96, 410, 410, B), (96, 411, 411, P), (96, 412, 412, B),
    (96, 413, 413, P), (96, 414, 414, B), (96, 415, 415, P), (96, 416, 416, B), (96, 417, 417, P),
    (96, 418, 418, B), (96, 419, 419, P), (96, 420, 420, B), (96, 421, 421, P), (96, 422, 422, B),
    (96, 423, 423, P), (96, 424, 424, B), (96, 425, 425, P), (96, 426, 426, B), (96, 427, 427, P),
    (96, 428, 428, B), (96, 429, 429, P), (96, 430, 430, B), (96, 431, 431, P), (96, 432, 432, B),
    (96, 433, 433, P), (96, 434, 434, B), (96, 435, 435, P), (96, 436, 436, B), (96, 437, 437, P),
    (96, 438, 438, B), (96, 439, 439, P), (96, 440, 440, B), (96, 441, 441, P), (96, 442, 442, B),
    (96, 443, 443, P), (96, 444, 444, B), (96, 445, 445, P), (96, 446, 446, B), (96, 447, 447, P),
    (96, 448, 448, B), (96, 449, 449, P), (96, 450, 450, B), (96, 451, 451, P), (96, 452, 452, B),
    (96, 453, 453, P), (96, 454, 454, B), (96, 455, 455, P), (96, 456, 456, B), (96, 457, 457, P),
    (96, 458, 458, B), (96, 459, 459, P), (96, 460, 460, B), (96, 461, 461, P), (96, 462, 462, B),
    (96, 463, 463, P), (96, 464, 464, B), (96, 465, 465, P), (96, 466, 466, B), (96, 467, 467, P),
    (96, 468, 468, B), (96, 469, 469, P), (96, 470, 470, B), (96, 471, 471, P), (96, 472, 472, B),
    (96, 473, 473, P), (96, 474, 474, B), (96, 475, 475, P), (96, 476, 476, B), (96, 477, 477, P),
    (96, 478, 478, B), (96, 479, 479, P), (96, 480, 480, B), (96, 481, 481, P), (96, 482, 482, B),
    (96, 483, 483, P), (96, 484, 484, B), (96, 485, 485, P), (96, 486, 486, B), (96, 487, 487, P),
    (96, 488, 488, B), (96, 489, 489, P), (96, 490, 490, B), (96, 491, 491, P), (96, 492, 492, B),
    (96, 493, 493, P), (96, 494, 494, B), (96, 495, 495, P), (96, 496, 496, B), (96, 497, 497, P),
    (96, 498, 498, B), (96, 499, 499, P), (96, 500, 500, B), (96, 501, 501, P), (97, 373, 373, B),
    (97, 374, 374, P), (97, 375, 375, B), (97, 376, 376, P), (97, 377, 377, B), (97, 378, 378, P),
    (97, 379, 379, B), (97, 380, 380, P), (97, 381, 381, B), (97, 382, 382, P), (97, 383, 383, B),
    (97, 384, 384, P), (97, 385, 385, B), (97, 386, 386, P), (97, 387, 387, B), (97, 388, 388, P),
    (97, 389, 389, B), (97, 390, 390, P), (97, 391, 391, B), (97, 392, 392, P), (97, 393, 393, B),
    (97, 394, 394, P), (97, 395, 395, B), (97, 396, 396, P), (97, 397, 397, B), (97, 398, 398, P),
    (97, 399, 399, B), (97, 400, 400, P), (97, 401, 401, B), (97, 402, 402, P), (97, 403, 403, B),
    (97, 404, 404, P), (97, 405, 405, B), (97, 406, 406, P), (97, 407, 407, B), (97, 408, 408, P),
    (97, 409, 409, B), (97, 410, 410, P), (97, 411, 411, B), (97, 412, 412, P), (97, 413, 413, B),
    (97, 414, 414, P), (97, 415, 415, B), (97, 416, 416, P), (97, 417, 417, B), (97, 418, 418, P),
    (97, 419, 419, B), (97, 420, 420, P), (97, 421, 421, B), (97, 422, 422, P), (97, 423, 423, B),
    (97, 424, 424, P), (97, 425, 425, B), (97, 426, 426, P), (97, 427, 427, B), (97, 428, 428, P),
    (97, 429, 429, B), (97, 430, 430, P), (97, 431, 431, B), (97, 432, 432, P), (97, 433, 433, B),
    (97, 434, 434, P), (97, 435, 435, B), (97, 436, 436, P), (97, 437, 437, B), (97, 438, 438, P),
    (97, 439, 439, B), (97, 440, 440, P), (97, 441, 441, B), (97, 442, 442, P), (97, 443, 443, B),
    (97, 444, 444, P), (97, 445, 445, B), (97, 446, 446, P), (97, 447, 447, B), (97, 448, 448, P),
    (97, 449, 449, B), (97, 450, 450, P), (97, 451, 451, B), (97, 452, 452, P), (97, 453, 453, B),
    (97, 454, 454, P), (97, 455, 455, B), (97, 456, 456, P), (97, 457, 457, B), (97, 458, 458, P),
    (97, 459, 459, B), (97, 460, 460, P), (97, 461, 461, B), (97, 462, 462, P), (97, 463, 463, B),
    (97, 464, 464, P), (97, 465, 465, B), (97, 466, 466, P), (97, 467, 467, B), (97, 468, 468, P),
    (97, 469, 469, B), (97, 470, 470, P), (97, 471, 471, B), (97, 472, 472, P), (97, 473, 473, B),
    (97, 474, 474, P), (97, 475, 475, B), (97, 476, 476, P), (97, 477, 477, B), (97, 478, 478, P),
    (97, 479, 479, B), (97, 480, 480, P), (97, 481, 481, B), (97, 482, 482, P), (97, 483, 483, B),
    (97, 484, 484, P), (97, 485, 485, B), (97, 486, 486, P), (97, 487, 487, B), (97, 488, 488, P),
    (97, 489, 489, B), (97, 490, 490, P), (97, 491, 491, B), (97, 492, 492, P), (97, 493, 493, B),
    (97, 494, 494, P), (97, 495, 495, B), (97, 496, 496, P), (97, 497, 497, B), (97, 498, 498, P),
    (97, 499, 499, B), (97, 500, 500, P), (97, 501, 501, B), (98, 373, 500, P), (99, 373, 500, P),
];

/// TVI: grab (80,301) 160x4, record 381 of `measurements/measurement-2026-11-06-v30/probe/e1`.
#[rustfmt::skip]
const SP_TVI: &[Span] = &[
    (301, 100, 227, P), (302, 100, 227, P), (303, 100, 100, P), (303, 101, 101, B), (303, 102, 102, P),
    (303, 103, 103, B), (303, 104, 104, P), (303, 105, 105, B), (303, 106, 106, P), (303, 107, 107, B),
    (303, 108, 108, P), (303, 109, 109, B), (303, 110, 110, P), (303, 111, 111, B), (303, 112, 112, P),
    (303, 113, 113, B), (303, 114, 114, P), (303, 115, 115, B), (303, 116, 116, P), (303, 117, 117, B),
    (303, 118, 118, P), (303, 119, 119, B), (303, 120, 120, P), (303, 121, 121, B), (303, 122, 122, P),
    (303, 123, 123, B), (303, 124, 124, P), (303, 125, 125, B), (303, 126, 126, P), (303, 127, 127, B),
    (303, 128, 128, P), (303, 129, 129, B), (303, 130, 130, P), (303, 131, 131, B), (303, 132, 132, P),
    (303, 133, 133, B), (303, 134, 134, P), (303, 135, 135, B), (303, 136, 136, P), (303, 137, 137, B),
    (303, 138, 138, P), (303, 139, 139, B), (303, 140, 140, P), (303, 141, 141, B), (303, 142, 142, P),
    (303, 143, 143, B), (303, 144, 144, P), (303, 145, 145, B), (303, 146, 146, P), (303, 147, 147, B),
    (303, 148, 148, P), (303, 149, 149, B), (303, 150, 150, P), (303, 151, 151, B), (303, 152, 152, P),
    (303, 153, 153, B), (303, 154, 154, P), (303, 155, 155, B), (303, 156, 156, P), (303, 157, 157, B),
    (303, 158, 158, P), (303, 159, 159, B), (303, 160, 160, P), (303, 161, 161, B), (303, 162, 162, P),
    (303, 163, 163, B), (303, 164, 164, P), (303, 165, 165, B), (303, 166, 166, P), (303, 167, 167, B),
    (303, 168, 168, P), (303, 169, 169, B), (303, 170, 170, P), (303, 171, 171, B), (303, 172, 172, P),
    (303, 173, 173, B), (303, 174, 174, P), (303, 175, 175, B), (303, 176, 176, P), (303, 177, 177, B),
    (303, 178, 178, P), (303, 179, 179, B), (303, 180, 180, P), (303, 181, 181, B), (303, 182, 182, P),
    (303, 183, 183, B), (303, 184, 184, P), (303, 185, 185, B), (303, 186, 186, P), (303, 187, 187, B),
    (303, 188, 188, P), (303, 189, 189, B), (303, 190, 190, P), (303, 191, 191, B), (303, 192, 192, P),
    (303, 193, 193, B), (303, 194, 194, P), (303, 195, 195, B), (303, 196, 196, P), (303, 197, 197, B),
    (303, 198, 198, P), (303, 199, 199, B), (303, 200, 200, P), (303, 201, 201, B), (303, 202, 202, P),
    (303, 203, 203, B), (303, 204, 204, P), (303, 205, 205, B), (303, 206, 206, P), (303, 207, 207, B),
    (303, 208, 208, P), (303, 209, 209, B), (303, 210, 210, P), (303, 211, 211, B), (303, 212, 212, P),
    (303, 213, 213, B), (303, 214, 214, P), (303, 215, 215, B), (303, 216, 216, P), (303, 217, 217, B),
    (303, 218, 218, P), (303, 219, 219, B), (303, 220, 220, P), (303, 221, 221, B), (303, 222, 222, P),
    (303, 223, 223, B), (303, 224, 224, P), (303, 225, 225, B), (303, 226, 226, P), (303, 227, 227, B),
    (303, 228, 228, P), (304, 100, 100, B), (304, 101, 101, P), (304, 102, 102, B), (304, 103, 103, P),
    (304, 104, 104, B), (304, 105, 105, P), (304, 106, 106, B), (304, 107, 107, P), (304, 108, 108, B),
    (304, 109, 109, P), (304, 110, 110, B), (304, 111, 111, P), (304, 112, 112, B), (304, 113, 113, P),
    (304, 114, 114, B), (304, 115, 115, P), (304, 116, 116, B), (304, 117, 117, P), (304, 118, 118, B),
    (304, 119, 119, P), (304, 120, 120, B), (304, 121, 121, P), (304, 122, 122, B), (304, 123, 123, P),
    (304, 124, 124, B), (304, 125, 125, P), (304, 126, 126, B), (304, 127, 127, P), (304, 128, 128, B),
    (304, 129, 129, P), (304, 130, 130, B), (304, 131, 131, P), (304, 132, 132, B), (304, 133, 133, P),
    (304, 134, 134, B), (304, 135, 135, P), (304, 136, 136, B), (304, 137, 137, P), (304, 138, 138, B),
    (304, 139, 139, P), (304, 140, 140, B), (304, 141, 141, P), (304, 142, 142, B), (304, 143, 143, P),
    (304, 144, 144, B), (304, 145, 145, P), (304, 146, 146, B), (304, 147, 147, P), (304, 148, 148, B),
    (304, 149, 149, P), (304, 150, 150, B), (304, 151, 151, P), (304, 152, 152, B), (304, 153, 153, P),
    (304, 154, 154, B), (304, 155, 155, P), (304, 156, 156, B), (304, 157, 157, P), (304, 158, 158, B),
    (304, 159, 159, P), (304, 160, 160, B), (304, 161, 161, P), (304, 162, 162, B), (304, 163, 163, P),
    (304, 164, 164, B), (304, 165, 165, P), (304, 166, 166, B), (304, 167, 167, P), (304, 168, 168, B),
    (304, 169, 169, P), (304, 170, 170, B), (304, 171, 171, P), (304, 172, 172, B), (304, 173, 173, P),
    (304, 174, 174, B), (304, 175, 175, P), (304, 176, 176, B), (304, 177, 177, P), (304, 178, 178, B),
    (304, 179, 179, P), (304, 180, 180, B), (304, 181, 181, P), (304, 182, 182, B), (304, 183, 183, P),
    (304, 184, 184, B), (304, 185, 185, P), (304, 186, 186, B), (304, 187, 187, P), (304, 188, 188, B),
    (304, 189, 189, P), (304, 190, 190, B), (304, 191, 191, P), (304, 192, 192, B), (304, 193, 193, P),
    (304, 194, 194, B), (304, 195, 195, P), (304, 196, 196, B), (304, 197, 197, P), (304, 198, 198, B),
    (304, 199, 199, P), (304, 200, 200, B), (304, 201, 201, P), (304, 202, 202, B), (304, 203, 203, P),
    (304, 204, 204, B), (304, 205, 205, P), (304, 206, 206, B), (304, 207, 207, P), (304, 208, 208, B),
    (304, 209, 209, P), (304, 210, 210, B), (304, 211, 211, P), (304, 212, 212, B), (304, 213, 213, P),
    (304, 214, 214, B), (304, 215, 215, P), (304, 216, 216, B), (304, 217, 217, P), (304, 218, 218, B),
    (304, 219, 219, P), (304, 220, 220, B), (304, 221, 221, P), (304, 222, 222, B), (304, 223, 223, P),
    (304, 224, 224, B), (304, 225, 225, P), (304, 226, 226, B), (304, 227, 227, P), (304, 228, 228, B),
];

/// TVJ: grab (80,80) 120x40, record 382 of `measurements/measurement-2026-11-06-v30/probe/e1`.
#[rustfmt::skip]
const SP_TVJ: &[Span] = &[
    (96, 101, 103, B), (96, 106, 109, B), (96, 113, 116, B), (96, 118, 121, B), (96, 124, 128, B),
    (96, 130, 134, B), (96, 137, 140, B), (96, 142, 142, B), (96, 146, 146, B), (97, 100, 100, B),
    (97, 104, 104, B), (97, 106, 106, B), (97, 110, 110, B), (97, 112, 112, B), (97, 118, 118, B),
    (97, 122, 122, B), (97, 124, 124, B), (97, 130, 130, B), (97, 136, 136, B), (97, 142, 142, B),
    (97, 146, 146, B), (98, 100, 104, B), (98, 106, 109, B), (98, 112, 112, B), (98, 118, 118, B),
    (98, 122, 122, B), (98, 124, 127, B), (98, 130, 133, B), (98, 136, 136, B), (98, 139, 140, B),
    (98, 142, 146, B), (99, 100, 100, B), (99, 104, 104, B), (99, 106, 106, B), (99, 110, 110, B),
    (99, 112, 112, B), (99, 118, 118, B), (99, 122, 122, B), (99, 124, 124, B), (99, 130, 130, B),
    (99, 136, 136, B), (99, 140, 140, B), (99, 142, 142, B), (99, 146, 146, B), (100, 100, 100, B),
    (100, 104, 104, B), (100, 106, 109, B), (100, 113, 116, B), (100, 118, 121, B), (100, 124, 128, B),
    (100, 130, 130, B), (100, 137, 140, B), (100, 142, 142, B), (100, 146, 146, B), (101, 100, 148, B),
];

/// TVL: grab (80,80) 130x40, record 384 of `measurements/measurement-2026-11-06-v30/probe/e1`.
#[rustfmt::skip]
const SP_TVL: &[Span] = &[
    (95, 99, 99, P), (95, 100, 104, B), (95, 105, 106, P), (95, 107, 112, B), (95, 113, 115, P),
    (95, 116, 121, B), (95, 122, 122, P), (95, 123, 128, B), (95, 129, 130, P), (95, 131, 137, B),
    (95, 138, 138, P), (95, 139, 145, B), (95, 146, 147, P), (95, 148, 153, B), (95, 154, 154, P),
    (95, 155, 157, B), (95, 158, 158, P), (95, 159, 161, B), (95, 162, 162, P), (96, 99, 100, B),
    (96, 101, 103, P), (96, 104, 105, B), (96, 106, 106, P), (96, 107, 107, B), (96, 108, 111, P),
    (96, 112, 113, B), (96, 114, 114, P), (96, 115, 116, B), (96, 117, 120, P), (96, 121, 121, B),
    (96, 122, 122, P), (96, 123, 123, B), (96, 124, 127, P), (96, 128, 129, B), (96, 130, 130, P),
    (96, 131, 131, B), (96, 132, 136, P), (96, 137, 137, B), (96, 138, 138, P), (96, 139, 139, B),
    (96, 140, 144, P), (96, 145, 145, B), (96, 146, 146, P), (96, 147, 148, B), (96, 149, 152, P),
    (96, 153, 153, B), (96, 154, 154, P), (96, 155, 155, B), (96, 156, 156, P), (96, 157, 157, B),
    (96, 158, 158, P), (96, 159, 159, B), (96, 160, 160, P), (96, 161, 161, B), (96, 162, 162, P),
    (97, 99, 99, B), (97, 100, 100, P), (97, 101, 103, B), (97, 104, 104, P), (97, 105, 105, B),
    (97, 106, 106, P), (97, 107, 107, B), (97, 108, 108, P), (97, 109, 111, B), (97, 112, 112, P),
    (97, 113, 113, B), (97, 114, 114, P), (97, 115, 115, B), (97, 116, 116, P), (97, 117, 121, B),
    (97, 122, 122, P), (97, 123, 123, B), (97, 124, 124, P), (97, 125, 127, B), (97, 128, 128, P),
    (97, 129, 129, B), (97, 130, 130, P), (97, 131, 131, B), (97, 132, 132, P), (97, 133, 137, B),
    (97, 138, 138, P), (97, 139, 139, B), (97, 140, 140, P), (97, 141, 145, B), (97, 146, 146, P),
    (97, 147, 147, B), (97, 148, 148, P), (97, 149, 153, B), (97, 154, 154, P), (97, 155, 155, B),
    (97, 156, 156, P), (97, 157, 159, B), (97, 160, 160, P), (97, 161, 161, B), (97, 162, 162, P),
    (98, 99, 99, B), (98, 100, 104, P), (98, 105, 105, B), (98, 106, 106, P), (98, 107, 107, B),
    (98, 108, 111, P), (98, 112, 113, B), (98, 114, 114, P), (98, 115, 115, B), (98, 116, 116, P),
    (98, 117, 117, B), (98, 118, 122, P), (98, 123, 123, B), (98, 124, 124, P), (98, 125, 125, B),
    (98, 126, 126, P), (98, 127, 127, B), (98, 128, 128, P), (98, 129, 129, B), (98, 130, 130, P),
    (98, 131, 131, B), (98, 132, 135, P), (98, 136, 136, B), (98, 137, 138, P), (98, 139, 139, B),
    (98, 140, 143, P), (98, 144, 144, B), (98, 145, 146, P), (98, 147, 147, B), (98, 148, 148, P),
    (98, 149, 150, B), (98, 151, 152, P), (98, 153, 153, B), (98, 154, 154, P), (98, 155, 155, B),
    (98, 156, 160, P), (98, 161, 161, B), (98, 162, 162, P), (99, 99, 99, B), (99, 100, 100, P),
    (99, 101, 103, B), (99, 104, 104, P), (99, 105, 105, B), (99, 106, 106, P), (99, 107, 107, B),
    (99, 108, 108, P), (99, 109, 111, B), (99, 112, 112, P), (99, 113, 113, B), (99, 114, 114, P),
    (99, 115, 115, B), (99, 116, 116, P), (99, 117, 121, B), (99, 122, 122, P), (99, 123, 123, B),
    (99, 124, 124, P), (99, 125, 127, B), (99, 128, 128, P), (99, 129, 129, B), (99, 130, 130, P),
    (99, 131, 131, B), (99, 132, 132, P), (99, 133, 137, B), (99, 138, 138, P), (99, 139, 139, B),
    (99, 140, 140, P), (99, 141, 144, B), (99, 145, 146, P), (99, 147, 147, B), (99, 148, 148, P),
    (99, 149, 151, B), (99, 152, 152, P), (99, 153, 153, B), (99, 154, 154, P), (99, 155, 155, B),
    (99, 156, 156, P), (99, 157, 159, B), (99, 160, 160, P), (99, 161, 161, B), (99, 162, 162, P),
    (100, 99, 163, B), (101, 99, 101, B), (101, 102, 102, P), (101, 103, 105, B), (101, 106, 106, P),
    (101, 107, 112, B), (101, 113, 115, P), (101, 116, 121, B), (101, 122, 122, P), (101, 123, 128, B),
    (101, 129, 130, P), (101, 131, 137, B), (101, 138, 138, P), (101, 139, 141, B), (101, 142, 147, P),
    (101, 148, 153, B), (101, 154, 154, P), (101, 155, 157, B), (101, 158, 158, P), (101, 159, 161, B),
    (101, 162, 162, P), (102, 99, 162, P),
];

/// TVM: grab (80,80) 240x40, record 385 of `measurements/measurement-2026-11-06-v30/probe/e1`.
#[rustfmt::skip]
const SP_TVM: &[Span] = &[
    (96, 100, 100, P), (96, 101, 103, B), (96, 104, 105, P), (96, 106, 109, B), (96, 110, 117, P),
    (96, 194, 194, P), (96, 195, 198, B), (96, 199, 199, P), (96, 200, 203, B), (96, 204, 211, P),
    (96, 288, 292, B), (96, 293, 293, P), (96, 294, 298, B), (96, 299, 299, P), (97, 100, 100, B),
    (97, 101, 103, P), (97, 104, 104, B), (97, 105, 105, P), (97, 106, 106, B), (97, 107, 109, P),
    (97, 110, 110, B), (97, 111, 117, P), (97, 194, 194, B), (97, 195, 199, P), (97, 200, 200, B),
    (97, 201, 203, P), (97, 204, 204, B), (97, 205, 211, P), (97, 288, 288, B), (97, 289, 293, P),
    (97, 294, 294, B), (97, 295, 299, P), (98, 100, 104, B), (98, 105, 105, P), (98, 106, 109, B),
    (98, 110, 117, P), (98, 194, 194, B), (98, 195, 199, P), (98, 200, 200, B), (98, 201, 203, P),
    (98, 204, 204, B), (98, 205, 211, P), (98, 288, 291, B), (98, 292, 293, P), (98, 294, 297, B),
    (98, 298, 299, P), (99, 100, 100, B), (99, 101, 103, P), (99, 104, 104, B), (99, 105, 105, P),
    (99, 106, 106, B), (99, 107, 109, P), (99, 110, 110, B), (99, 111, 117, P), (99, 194, 194, B),
    (99, 195, 199, P), (99, 200, 200, B), (99, 201, 203, P), (99, 204, 204, B), (99, 205, 211, P),
    (99, 288, 288, B), (99, 289, 293, P), (99, 294, 294, B), (99, 295, 299, P), (100, 100, 100, B),
    (100, 101, 103, P), (100, 104, 104, B), (100, 105, 105, P), (100, 106, 109, B), (100, 110, 117, P),
    (100, 194, 194, P), (100, 195, 198, B), (100, 199, 199, P), (100, 200, 203, B), (100, 204, 211, P),
    (100, 288, 292, B), (100, 293, 293, P), (100, 294, 294, B), (100, 295, 299, P), (101, 100, 300, B),
];

/// TVN: grab (80,80) 240x40, record 386 of `measurements/measurement-2026-11-06-v30/probe/e1`.
#[rustfmt::skip]
const SP_TVN: &[Span] = &[
    (95, 99, 99, P), (95, 100, 104, B), (95, 105, 106, P), (95, 107, 112, B), (95, 113, 122, P),
    (95, 191, 191, P), (95, 192, 197, B), (95, 198, 198, P), (95, 199, 204, B), (95, 205, 214, P),
    (95, 283, 289, B), (95, 290, 290, P), (95, 291, 297, B), (95, 298, 298, P), (96, 99, 100, B),
    (96, 101, 103, P), (96, 104, 105, B), (96, 106, 106, P), (96, 107, 107, B), (96, 108, 111, P),
    (96, 112, 113, B), (96, 114, 122, P), (96, 191, 192, B), (96, 193, 196, P), (96, 197, 197, B),
    (96, 198, 198, P), (96, 199, 199, B), (96, 200, 203, P), (96, 204, 205, B), (96, 206, 214, P),
    (96, 283, 283, B), (96, 284, 288, P), (96, 289, 289, B), (96, 290, 290, P), (96, 291, 291, B),
    (96, 292, 296, P), (96, 297, 297, B), (96, 298, 298, P), (97, 99, 99, B), (97, 100, 100, P),
    (97, 101, 103, B), (97, 104, 104, P), (97, 105, 105, B), (97, 106, 106, P), (97, 107, 107, B),
    (97, 108, 108, P), (97, 109, 111, B), (97, 112, 112, P), (97, 113, 113, B), (97, 114, 122, P),
    (97, 191, 191, B), (97, 192, 192, P), (97, 193, 197, B), (97, 198, 198, P), (97, 199, 199, B),
    (97, 200, 200, P), (97, 201, 203, B), (97, 204, 204, P), (97, 205, 205, B), (97, 206, 214, P),
    (97, 283, 283, B), (97, 284, 284, P), (97, 285, 289, B), (97, 290, 290, P), (97, 291, 291, B),
    (97, 292, 292, P), (97, 293, 297, B), (97, 298, 298, P), (98, 99, 99, B), (98, 100, 104, P),
    (98, 105, 105, B), (98, 106, 106, P), (98, 107, 107, B), (98, 108, 111, P), (98, 112, 113, B),
    (98, 114, 122, P), (98, 191, 191, B), (98, 192, 192, P), (98, 193, 193, B), (98, 194, 198, P),
    (98, 199, 199, B), (98, 200, 200, P), (98, 201, 201, B), (98, 202, 202, P), (98, 203, 203, B),
    (98, 204, 204, P), (98, 205, 205, B), (98, 206, 214, P), (98, 283, 283, B), (98, 284, 287, P),
    (98, 288, 288, B), (98, 289, 290, P), (98, 291, 291, B), (98, 292, 295, P), (98, 296, 296, B),
    (98, 297, 298, P), (99, 99, 99, B), (99, 100, 100, P), (99, 101, 103, B), (99, 104, 104, P),
    (99, 105, 105, B), (99, 106, 106, P), (99, 107, 107, B), (99, 108, 108, P), (99, 109, 111, B),
    (99, 112, 112, P), (99, 113, 113, B), (99, 114, 122, P), (99, 191, 191, B), (99, 192, 192, P),
    (99, 193, 197, B), (99, 198, 198, P), (99, 199, 199, B), (99, 200, 200, P), (99, 201, 203, B),
    (99, 204, 204, P), (99, 205, 205, B), (99, 206, 214, P), (99, 283, 283, B), (99, 284, 284, P),
    (99, 285, 289, B), (99, 290, 290, P), (99, 291, 291, B), (99, 292, 292, P), (99, 293, 296, B),
    (99, 297, 298, P), (100, 99, 299, B), (101, 99, 101, B), (101, 102, 102, P), (101, 103, 105, B),
    (101, 106, 106, P), (101, 107, 112, B), (101, 113, 122, P), (101, 191, 191, P), (101, 192, 197, B),
    (101, 198, 198, P), (101, 199, 204, B), (101, 205, 214, P), (101, 283, 289, B), (101, 290, 290, P),
    (101, 291, 293, B), (101, 294, 298, P), (102, 99, 122, P), (102, 191, 214, P), (102, 283, 298, P),
];

/// TVQ: grab (60,80) 120x40, record 389 of `measurements/measurement-2026-11-06-v30/probe/e1`.
#[rustfmt::skip]
const SP_TVQ: &[Span] = &[
    (96, 100, 102, B), (96, 103, 103, P), (96, 104, 104, B), (96, 105, 107, P), (96, 108, 108, B),
    (96, 109, 109, P), (97, 100, 104, B), (97, 105, 107, P), (97, 108, 108, B), (97, 109, 109, P),
    (98, 100, 108, B), (98, 109, 109, P), (99, 100, 104, B), (99, 105, 107, P), (99, 108, 108, B),
    (99, 109, 109, P), (100, 100, 102, B), (100, 103, 103, P), (100, 104, 104, B), (100, 105, 107, P),
    (100, 108, 108, B), (100, 109, 109, P), (101, 100, 105, B), (101, 106, 109, P),
];

/// TVR: grab (560,80) 80x40, record 390 of `measurements/measurement-2026-11-06-v30/probe/e1`.
#[rustfmt::skip]
const SP_TVR: &[Span] = &[
    (96, 601, 603, B), (96, 606, 609, B), (96, 613, 616, B), (96, 618, 621, B), (96, 624, 628, B),
    (96, 630, 634, B), (96, 637, 639, B), (97, 600, 600, B), (97, 604, 604, B), (97, 606, 606, B),
    (97, 610, 610, B), (97, 612, 612, B), (97, 618, 618, B), (97, 622, 622, B), (97, 624, 624, B),
    (97, 630, 630, B), (97, 636, 636, B), (98, 600, 604, B), (98, 606, 609, B), (98, 612, 612, B),
    (98, 618, 618, B), (98, 622, 622, B), (98, 624, 627, B), (98, 630, 633, B), (98, 636, 636, B),
    (98, 639, 639, B), (99, 600, 600, B), (99, 604, 604, B), (99, 606, 606, B), (99, 610, 610, B),
    (99, 612, 612, B), (99, 618, 618, B), (99, 622, 622, B), (99, 624, 624, B), (99, 630, 630, B),
    (99, 636, 636, B), (100, 600, 600, B), (100, 604, 604, B), (100, 606, 609, B), (100, 613, 616, B),
    (100, 618, 621, B), (100, 624, 628, B), (100, 630, 630, B), (100, 637, 639, B), (101, 600, 639, B),
];

/// TVRw: grab (0,80) 80x40, record 390 of `measurements/measurement-2026-11-06-v30/probe/e1`.
#[rustfmt::skip]
const SP_TVRW: &[Span] = &[
    (97, 0, 0, B), (97, 2, 2, B), (97, 6, 6, B), (98, 2, 2, B), (98, 6, 6, B), (99, 0, 0, B), (99, 2, 6, B),
    (100, 0, 0, B), (100, 2, 2, B), (100, 6, 6, B), (101, 0, 0, B), (101, 2, 2, B), (101, 6, 6, B),
    (102, 0, 8, B),
];

/// TVS: grab (560,80) 80x40, record 391 of `measurements/measurement-2026-11-06-v30/probe/e1`.
#[rustfmt::skip]
const SP_TVS: &[Span] = &[
    (96, 601, 603, B), (96, 606, 609, B), (96, 613, 616, B), (96, 618, 621, B), (96, 624, 628, B),
    (96, 630, 634, B), (96, 637, 639, B), (97, 600, 600, B), (97, 604, 604, B), (97, 606, 606, B),
    (97, 610, 610, B), (97, 612, 612, B), (97, 618, 618, B), (97, 622, 622, B), (97, 624, 624, B),
    (97, 630, 630, B), (97, 636, 636, B), (98, 600, 604, B), (98, 606, 609, B), (98, 612, 612, B),
    (98, 618, 618, B), (98, 622, 622, B), (98, 624, 627, B), (98, 630, 633, B), (98, 636, 636, B),
    (98, 639, 639, B), (99, 600, 600, B), (99, 604, 604, B), (99, 606, 606, B), (99, 610, 610, B),
    (99, 612, 612, B), (99, 618, 618, B), (99, 622, 622, B), (99, 624, 624, B), (99, 630, 630, B),
    (99, 636, 636, B), (100, 600, 600, B), (100, 604, 604, B), (100, 606, 609, B), (100, 613, 616, B),
    (100, 618, 621, B), (100, 624, 628, B), (100, 630, 630, B), (100, 637, 639, B), (101, 600, 639, B),
];

/// TVSw: grab (0,80) 80x40, record 391 of `measurements/measurement-2026-11-06-v30/probe/e1`.
#[rustfmt::skip]
const SP_TVSW: &[Span] = &[
    (97, 0, 0, B), (97, 2, 2, B), (97, 6, 6, B), (98, 2, 2, B), (98, 6, 6, B), (99, 0, 0, B), (99, 2, 6, B),
    (100, 0, 0, B), (100, 2, 2, B), (100, 6, 6, B), (101, 0, 0, B), (101, 2, 2, B), (101, 6, 6, B),
    (102, 0, 8, B),
];

/// TVT: grab (560,80) 80x40, record 392 of `measurements/measurement-2026-11-06-v30/probe/e1`.
#[rustfmt::skip]
const SP_TVT: &[Span] = &[
    (96, 600, 600, B), (96, 604, 605, B), (96, 610, 612, B), (96, 617, 617, B), (96, 622, 623, B),
    (96, 629, 629, B), (96, 635, 636, B), (97, 601, 603, B), (97, 605, 605, B), (97, 607, 609, B),
    (97, 611, 611, B), (97, 613, 617, B), (97, 619, 621, B), (97, 623, 623, B), (97, 625, 629, B),
    (97, 631, 635, B), (97, 637, 639, B), (98, 605, 605, B), (98, 610, 611, B), (98, 613, 617, B),
    (98, 619, 621, B), (98, 623, 623, B), (98, 628, 629, B), (98, 634, 635, B), (98, 637, 638, B),
    (99, 601, 603, B), (99, 605, 605, B), (99, 607, 609, B), (99, 611, 611, B), (99, 613, 617, B),
    (99, 619, 621, B), (99, 623, 623, B), (99, 625, 629, B), (99, 631, 635, B), (99, 637, 639, B),
    (100, 601, 603, B), (100, 605, 605, B), (100, 610, 612, B), (100, 617, 617, B), (100, 622, 623, B),
    (100, 629, 629, B), (100, 631, 636, B), (101, 600, 639, B),
];

/// TVTw: grab (0,80) 80x40, record 392 of `measurements/measurement-2026-11-06-v30/probe/e1`.
#[rustfmt::skip]
const SP_TVTW: &[Span] = &[
    (97, 1, 1, B), (97, 3, 5, B), (97, 7, 7, B), (98, 0, 1, B), (98, 3, 5, B), (98, 7, 7, B), (99, 1, 1, B),
    (99, 7, 7, B), (100, 1, 1, B), (100, 3, 5, B), (100, 7, 7, B), (101, 1, 1, B), (101, 3, 5, B),
    (101, 7, 7, B), (102, 0, 7, B),
];

/// TVV: grab (560,60) 80x60, record 394 of `measurements/measurement-2026-11-06-v30/probe/e1`.
#[rustfmt::skip]
const SP_TVV: &[Span] = &[
    (80, 600, 639, P), (81, 600, 639, P), (82, 600, 639, P), (83, 600, 604, P), (83, 605, 607, B),
    (83, 608, 613, P), (83, 614, 621, B), (83, 622, 627, P), (83, 628, 633, B), (83, 634, 637, P),
    (83, 638, 639, B), (84, 600, 604, P), (84, 605, 607, B), (84, 608, 613, P), (84, 614, 621, B),
    (84, 622, 627, P), (84, 628, 633, B), (84, 634, 637, P), (84, 638, 639, B), (85, 600, 602, P),
    (85, 603, 608, B), (85, 609, 613, P), (85, 614, 622, B), (85, 623, 625, P), (85, 626, 634, B),
    (85, 635, 637, P), (85, 638, 639, B), (86, 600, 601, P), (86, 602, 610, B), (86, 611, 613, P),
    (86, 614, 616, B), (86, 617, 619, P), (86, 620, 622, B), (86, 623, 625, P), (86, 626, 628, B),
    (86, 629, 631, P), (86, 632, 634, B), (86, 635, 637, P), (86, 638, 639, B), (87, 600, 601, P),
    (87, 602, 610, B), (87, 611, 613, P), (87, 614, 616, B), (87, 617, 619, P), (87, 620, 622, B),
    (87, 623, 625, P), (87, 626, 628, B), (87, 629, 631, P), (87, 632, 634, B), (87, 635, 637, P),
    (87, 638, 639, B), (88, 600, 601, P), (88, 602, 604, B), (88, 605, 607, P), (88, 608, 610, B),
    (88, 611, 613, P), (88, 614, 616, B), (88, 617, 619, P), (88, 620, 622, B), (88, 623, 625, P),
    (88, 626, 628, B), (88, 629, 631, P), (88, 632, 634, B), (88, 635, 637, P), (88, 638, 639, B),
    (89, 600, 601, P), (89, 602, 604, B), (89, 605, 607, P), (89, 608, 610, B), (89, 611, 613, P),
    (89, 614, 622, B), (89, 623, 625, P), (89, 626, 628, B), (89, 629, 637, P), (89, 638, 639, B),
    (90, 600, 601, P), (90, 602, 604, B), (90, 605, 607, P), (90, 608, 610, B), (90, 611, 613, P),
    (90, 614, 622, B), (90, 623, 625, P), (90, 626, 628, B), (90, 629, 637, P), (90, 638, 639, B),
    (91, 600, 601, P), (91, 602, 604, B), (91, 605, 607, P), (91, 608, 610, B), (91, 611, 613, P),
    (91, 614, 621, B), (91, 622, 625, P), (91, 626, 628, B), (91, 629, 637, P), (91, 638, 639, B),
    (92, 600, 601, P), (92, 602, 610, B), (92, 611, 613, P), (92, 614, 616, B), (92, 617, 619, P),
    (92, 620, 622, B), (92, 623, 625, P), (92, 626, 628, B), (92, 629, 637, P), (92, 638, 639, B),
    (93, 600, 601, P), (93, 602, 610, B), (93, 611, 613, P), (93, 614, 616, B), (93, 617, 619, P),
    (93, 620, 622, B), (93, 623, 625, P), (93, 626, 628, B), (93, 629, 637, P), (93, 638, 639, B),
    (94, 600, 601, P), (94, 602, 610, B), (94, 611, 613, P), (94, 614, 616, B), (94, 617, 619, P),
    (94, 620, 622, B), (94, 623, 625, P), (94, 626, 628, B), (94, 629, 637, P), (94, 638, 639, B),
    (95, 600, 601, P), (95, 602, 604, B), (95, 605, 607, P), (95, 608, 610, B), (95, 611, 613, P),
    (95, 614, 616, B), (95, 617, 619, P), (95, 620, 622, B), (95, 623, 625, P), (95, 626, 628, B),
    (95, 629, 631, P), (95, 632, 634, B), (95, 635, 637, P), (95, 638, 639, B), (96, 600, 601, P),
    (96, 602, 604, B), (96, 605, 607, P), (96, 608, 610, B), (96, 611, 613, P), (96, 614, 616, B),
    (96, 617, 619, P), (96, 620, 622, B), (96, 623, 625, P), (96, 626, 628, B), (96, 629, 631, P),
    (96, 632, 634, B), (96, 635, 637, P), (96, 638, 639, B), (97, 600, 601, P), (97, 602, 604, B),
    (97, 605, 607, P), (97, 608, 610, B), (97, 611, 613, P), (97, 614, 616, B), (97, 617, 619, P),
    (97, 620, 622, B), (97, 623, 625, P), (97, 626, 628, B), (97, 629, 631, P), (97, 632, 634, B),
    (97, 635, 637, P), (97, 638, 639, B), (98, 600, 601, P), (98, 602, 604, B), (98, 605, 607, P),
    (98, 608, 610, B), (98, 611, 613, P), (98, 614, 622, B), (98, 623, 625, P), (98, 626, 634, B),
    (98, 635, 637, P), (98, 638, 639, B), (99, 600, 601, P), (99, 602, 604, B), (99, 605, 607, P),
    (99, 608, 610, B), (99, 611, 613, P), (99, 614, 622, B), (99, 623, 625, P), (99, 626, 634, B),
    (99, 635, 637, P), (99, 638, 639, B), (100, 600, 601, P), (100, 602, 604, B), (100, 605, 607, P),
    (100, 608, 610, B), (100, 611, 613, P), (100, 614, 621, B), (100, 622, 627, P), (100, 628, 633, B),
    (100, 634, 637, P), (100, 638, 639, B), (101, 600, 601, P), (101, 602, 604, B), (101, 605, 607, P),
    (101, 608, 610, B), (101, 611, 613, P), (101, 614, 621, B), (101, 622, 627, P), (101, 628, 633, B),
    (101, 634, 637, P), (101, 638, 639, B), (102, 600, 639, P), (103, 600, 639, P), (104, 600, 639, P),
];

/// TVVw: grab (0,60) 80x60, record 394 of `measurements/measurement-2026-11-06-v30/probe/e1`.
#[rustfmt::skip]
const SP_TVVW: &[Span] = &[
    (81, 0, 57, P), (82, 0, 57, P), (83, 0, 57, P), (84, 0, 4, B), (84, 5, 10, P), (84, 11, 19, B),
    (84, 20, 22, P), (84, 23, 31, B), (84, 32, 36, P), (84, 37, 44, B), (84, 45, 47, P), (84, 48, 50, B),
    (84, 51, 53, P), (84, 54, 56, B), (84, 57, 57, P), (85, 0, 4, B), (85, 5, 10, P), (85, 11, 19, B),
    (85, 20, 22, P), (85, 23, 31, B), (85, 32, 36, P), (85, 37, 44, B), (85, 45, 47, P), (85, 48, 50, B),
    (85, 51, 53, P), (85, 54, 56, B), (85, 57, 57, P), (86, 0, 5, B), (86, 6, 10, P), (86, 11, 19, B),
    (86, 20, 22, P), (86, 23, 31, B), (86, 32, 34, P), (86, 35, 44, B), (86, 45, 47, P), (86, 48, 50, B),
    (86, 51, 53, P), (86, 54, 56, B), (86, 57, 57, P), (87, 0, 1, B), (87, 2, 2, P), (87, 3, 7, B),
    (87, 8, 10, P), (87, 11, 13, B), (87, 14, 22, P), (87, 23, 25, B), (87, 26, 34, P), (87, 35, 37, B),
    (87, 38, 47, P), (87, 48, 50, B), (87, 51, 53, P), (87, 54, 56, B), (87, 57, 57, P), (88, 0, 1, B),
    (88, 2, 2, P), (88, 3, 7, B), (88, 8, 10, P), (88, 11, 13, B), (88, 14, 22, P), (88, 23, 25, B),
    (88, 26, 34, P), (88, 35, 37, B), (88, 38, 47, P), (88, 48, 50, B), (88, 51, 53, P), (88, 54, 56, B),
    (88, 57, 57, P), (89, 0, 1, B), (89, 2, 4, P), (89, 5, 7, B), (89, 8, 10, P), (89, 11, 13, B),
    (89, 14, 22, P), (89, 23, 25, B), (89, 26, 34, P), (89, 35, 37, B), (89, 38, 47, P), (89, 48, 50, B),
    (89, 51, 53, P), (89, 54, 56, B), (89, 57, 57, P), (90, 0, 1, B), (90, 2, 4, P), (90, 5, 7, B),
    (90, 8, 10, P), (90, 11, 17, B), (90, 18, 22, P), (90, 23, 30, B), (90, 31, 34, P), (90, 35, 37, B),
    (90, 38, 39, P), (90, 40, 44, B), (90, 45, 47, P), (90, 48, 56, B), (90, 57, 57, P), (91, 0, 1, B),
    (91, 2, 4, P), (91, 5, 7, B), (91, 8, 10, P), (91, 11, 17, B), (91, 18, 22, P), (91, 23, 30, B),
    (91, 31, 34, P), (91, 35, 37, B), (91, 38, 39, P), (91, 40, 44, B), (91, 45, 47, P), (91, 48, 56, B),
    (91, 57, 57, P), (92, 0, 1, B), (92, 2, 4, P), (92, 5, 7, B), (92, 8, 10, P), (92, 11, 17, B),
    (92, 18, 22, P), (92, 23, 30, B), (92, 31, 34, P), (92, 35, 37, B), (92, 38, 39, P), (92, 40, 44, B),
    (92, 45, 47, P), (92, 48, 56, B), (92, 57, 57, P), (93, 0, 1, B), (93, 2, 4, P), (93, 5, 7, B),
    (93, 8, 10, P), (93, 11, 13, B), (93, 14, 22, P), (93, 23, 25, B), (93, 26, 34, P), (93, 35, 37, B),
    (93, 38, 41, P), (93, 42, 44, B), (93, 45, 47, P), (93, 48, 50, B), (93, 51, 53, P), (93, 54, 56, B),
    (93, 57, 57, P), (94, 0, 1, B), (94, 2, 4, P), (94, 5, 7, B), (94, 8, 10, P), (94, 11, 13, B),
    (94, 14, 22, P), (94, 23, 25, B), (94, 26, 34, P), (94, 35, 37, B), (94, 38, 41, P), (94, 42, 44, B),
    (94, 45, 47, P), (94, 48, 50, B), (94, 51, 53, P), (94, 54, 56, B), (94, 57, 57, P), (95, 0, 1, B),
    (95, 2, 4, P), (95, 5, 7, B), (95, 8, 10, P), (95, 11, 13, B), (95, 14, 22, P), (95, 23, 25, B),
    (95, 26, 34, P), (95, 35, 37, B), (95, 38, 41, P), (95, 42, 44, B), (95, 45, 47, P), (95, 48, 50, B),
    (95, 51, 53, P), (95, 54, 56, B), (95, 57, 57, P), (96, 0, 1, B), (96, 2, 4, P), (96, 5, 7, B),
    (96, 8, 10, P), (96, 11, 13, B), (96, 14, 22, P), (96, 23, 25, B), (96, 26, 34, P), (96, 35, 37, B),
    (96, 38, 41, P), (96, 42, 44, B), (96, 45, 47, P), (96, 48, 50, B), (96, 51, 53, P), (96, 54, 56, B),
    (96, 57, 57, P), (97, 0, 1, B), (97, 2, 4, P), (97, 5, 7, B), (97, 8, 10, P), (97, 11, 13, B),
    (97, 14, 22, P), (97, 23, 25, B), (97, 26, 34, P), (97, 35, 37, B), (97, 38, 41, P), (97, 42, 44, B),
    (97, 45, 47, P), (97, 48, 50, B), (97, 51, 53, P), (97, 54, 56, B), (97, 57, 57, P), (98, 0, 1, B),
    (98, 2, 2, P), (98, 3, 7, B), (98, 8, 10, P), (98, 11, 13, B), (98, 14, 22, P), (98, 23, 25, B),
    (98, 26, 34, P), (98, 35, 37, B), (98, 38, 41, P), (98, 42, 44, B), (98, 45, 47, P), (98, 48, 50, B),
    (98, 51, 53, P), (98, 54, 56, B), (98, 57, 57, P), (99, 0, 5, B), (99, 6, 10, P), (99, 11, 19, B),
    (99, 20, 22, P), (99, 23, 25, B), (99, 26, 34, P), (99, 35, 44, B), (99, 45, 47, P), (99, 48, 50, B),
    (99, 51, 53, P), (99, 54, 56, B), (99, 57, 57, P), (100, 0, 5, B), (100, 6, 10, P), (100, 11, 19, B),
    (100, 20, 22, P), (100, 23, 25, B), (100, 26, 34, P), (100, 35, 44, B), (100, 45, 47, P),
    (100, 48, 50, B), (100, 51, 53, P), (100, 54, 56, B), (100, 57, 57, P), (101, 0, 4, B), (101, 5, 10, P),
    (101, 11, 19, B), (101, 20, 22, P), (101, 23, 25, B), (101, 26, 36, P), (101, 37, 42, B),
    (101, 43, 47, P), (101, 48, 50, B), (101, 51, 53, P), (101, 54, 56, B), (101, 57, 57, P), (102, 0, 4, B),
    (102, 5, 10, P), (102, 11, 19, B), (102, 20, 22, P), (102, 23, 25, B), (102, 26, 36, P), (102, 37, 42, B),
    (102, 43, 47, P), (102, 48, 50, B), (102, 51, 53, P), (102, 54, 56, B), (102, 57, 57, P), (103, 0, 57, P),
    (104, 0, 57, P), (105, 0, 57, P),
];

/// TVW: grab (0,130) 80x40, record 395 of `measurements/measurement-2026-11-06-v30/probe/e1`.
#[rustfmt::skip]
const SP_TVW: &[Span] = &[
    (146, 0, 0, B), (146, 1, 1, P), (146, 2, 6, B), (146, 7, 8, P), (146, 9, 12, B), (146, 13, 13, P),
    (146, 14, 14, B), (146, 15, 17, P), (146, 18, 18, B), (146, 19, 19, P), (147, 0, 1, P), (147, 2, 2, B),
    (147, 3, 7, P), (147, 8, 8, B), (147, 9, 13, P), (147, 14, 14, B), (147, 15, 17, P), (147, 18, 18, B),
    (147, 19, 19, P), (148, 0, 1, P), (148, 2, 5, B), (148, 6, 7, P), (148, 8, 8, B), (148, 9, 10, P),
    (148, 11, 12, B), (148, 13, 13, P), (148, 14, 18, B), (148, 19, 19, P), (149, 0, 1, P), (149, 2, 2, B),
    (149, 3, 7, P), (149, 8, 8, B), (149, 9, 11, P), (149, 12, 12, B), (149, 13, 13, P), (149, 14, 14, B),
    (149, 15, 17, P), (149, 18, 18, B), (149, 19, 19, P), (150, 0, 0, B), (150, 1, 1, P), (150, 2, 2, B),
    (150, 3, 8, P), (150, 9, 12, B), (150, 13, 13, P), (150, 14, 14, B), (150, 15, 17, P), (150, 18, 18, B),
    (150, 19, 19, P), (151, 0, 20, B),
];

/// TVWw: grab (560,130) 80x40, record 395 of `measurements/measurement-2026-11-06-v30/probe/e1`.
#[rustfmt::skip]
const SP_TVWW: &[Span] = &[
    (145, 612, 612, P), (145, 613, 615, B), (145, 616, 617, P), (145, 618, 621, B), (145, 622, 624, P),
    (145, 625, 628, B), (145, 629, 629, P), (145, 630, 633, B), (145, 634, 635, P), (145, 636, 639, B),
    (146, 612, 612, B), (146, 613, 615, P), (146, 616, 616, B), (146, 617, 617, P), (146, 618, 618, B),
    (146, 619, 621, P), (146, 622, 622, B), (146, 623, 623, P), (146, 624, 624, B), (146, 625, 629, P),
    (146, 630, 630, B), (146, 631, 633, P), (146, 634, 634, B), (146, 635, 635, P), (146, 636, 636, B),
    (146, 637, 639, P), (147, 612, 616, B), (147, 617, 617, P), (147, 618, 621, B), (147, 622, 623, P),
    (147, 624, 624, B), (147, 625, 629, P), (147, 630, 630, B), (147, 631, 633, P), (147, 634, 634, B),
    (147, 635, 635, P), (147, 636, 639, B), (148, 612, 612, B), (148, 613, 615, P), (148, 616, 616, B),
    (148, 617, 617, P), (148, 618, 618, B), (148, 619, 621, P), (148, 622, 622, B), (148, 623, 623, P),
    (148, 624, 624, B), (148, 625, 629, P), (148, 630, 630, B), (148, 631, 633, P), (148, 634, 634, B),
    (148, 635, 635, P), (148, 636, 636, B), (148, 637, 639, P), (149, 612, 612, B), (149, 613, 615, P),
    (149, 616, 616, B), (149, 617, 617, P), (149, 618, 621, B), (149, 622, 624, P), (149, 625, 628, B),
    (149, 629, 629, P), (149, 630, 633, B), (149, 634, 635, P), (149, 636, 639, B), (150, 612, 639, B),
];

/// TVX: grab (80,80) 120x40, record 396 of `measurements/measurement-2026-11-06-v30/probe/e1`.
#[rustfmt::skip]
const SP_TVX: &[Span] = &[
    (87, 100, 163, P), (88, 100, 163, P), (89, 100, 102, P), (89, 103, 104, B), (89, 105, 108, P),
    (89, 109, 113, B), (89, 114, 117, P), (89, 118, 121, B), (89, 122, 124, P), (89, 125, 128, B),
    (89, 129, 132, P), (89, 133, 138, B), (89, 139, 140, P), (89, 141, 146, B), (89, 147, 149, P),
    (89, 150, 154, B), (89, 155, 156, P), (89, 157, 158, B), (89, 159, 160, P), (89, 161, 162, B),
    (89, 163, 163, P), (90, 100, 101, P), (90, 102, 105, B), (90, 106, 108, P), (90, 109, 114, B),
    (90, 115, 116, P), (90, 117, 122, B), (90, 123, 124, P), (90, 125, 129, B), (90, 130, 132, P),
    (90, 133, 138, B), (90, 139, 140, P), (90, 141, 146, B), (90, 147, 148, P), (90, 149, 154, B),
    (90, 155, 156, P), (90, 157, 158, B), (90, 159, 160, P), (90, 161, 162, B), (90, 163, 163, P),
    (91, 100, 100, P), (91, 101, 106, B), (91, 107, 108, P), (91, 109, 110, B), (91, 111, 112, P),
    (91, 113, 114, B), (91, 115, 116, P), (91, 117, 118, B), (91, 119, 120, P), (91, 121, 122, B),
    (91, 123, 124, P), (91, 125, 126, B), (91, 127, 127, P), (91, 128, 130, B), (91, 131, 132, P),
    (91, 133, 134, B), (91, 135, 140, P), (91, 141, 142, B), (91, 143, 148, P), (91, 149, 150, B),
    (91, 151, 156, P), (91, 157, 158, B), (91, 159, 160, P), (91, 161, 162, B), (91, 163, 163, P),
    (92, 100, 100, P), (92, 101, 102, B), (92, 103, 104, P), (92, 105, 106, B), (92, 107, 108, P),
    (92, 109, 110, B), (92, 111, 112, P), (92, 113, 114, B), (92, 115, 116, P), (92, 117, 118, B),
    (92, 119, 120, P), (92, 121, 122, B), (92, 123, 124, P), (92, 125, 126, B), (92, 127, 128, P),
    (92, 129, 130, B), (92, 131, 132, P), (92, 133, 134, B), (92, 135, 140, P), (92, 141, 142, B),
    (92, 143, 148, P), (92, 149, 150, B), (92, 151, 156, P), (92, 157, 158, B), (92, 159, 160, P),
    (92, 161, 162, B), (92, 163, 163, P), (93, 100, 100, P), (93, 101, 102, B), (93, 103, 104, P),
    (93, 105, 106, B), (93, 107, 108, P), (93, 109, 114, B), (93, 115, 116, P), (93, 117, 118, B),
    (93, 119, 124, P), (93, 125, 126, B), (93, 127, 128, P), (93, 129, 130, B), (93, 131, 132, P),
    (93, 133, 137, B), (93, 138, 140, P), (93, 141, 145, B), (93, 146, 148, P), (93, 149, 150, B),
    (93, 151, 151, P), (93, 152, 154, B), (93, 155, 156, P), (93, 157, 162, B), (93, 163, 163, P),
    (94, 100, 100, P), (94, 101, 102, B), (94, 103, 104, P), (94, 105, 106, B), (94, 107, 108, P),
    (94, 109, 113, B), (94, 114, 116, P), (94, 117, 118, B), (94, 119, 124, P), (94, 125, 126, B),
    (94, 127, 128, P), (94, 129, 130, B), (94, 131, 132, P), (94, 133, 137, B), (94, 138, 140, P),
    (94, 141, 145, B), (94, 146, 148, P), (94, 149, 150, B), (94, 151, 151, P), (94, 152, 154, B),
    (94, 155, 156, P), (94, 157, 162, B), (94, 163, 163, P), (95, 100, 100, P), (95, 101, 106, B),
    (95, 107, 108, P), (95, 109, 110, B), (95, 111, 112, P), (95, 113, 114, B), (95, 115, 116, P),
    (95, 117, 118, B), (95, 119, 124, P), (95, 125, 126, B), (95, 127, 128, P), (95, 129, 130, B),
    (95, 131, 132, P), (95, 133, 134, B), (95, 135, 140, P), (95, 141, 142, B), (95, 143, 148, P),
    (95, 149, 150, B), (95, 151, 152, P), (95, 153, 154, B), (95, 155, 156, P), (95, 157, 158, B),
    (95, 159, 160, P), (95, 161, 162, B), (95, 163, 163, P), (96, 100, 100, P), (96, 101, 106, B),
    (96, 107, 108, P), (96, 109, 110, B), (96, 111, 112, P), (96, 113, 114, B), (96, 115, 116, P),
    (96, 117, 118, B), (96, 119, 124, P), (96, 125, 126, B), (96, 127, 128, P), (96, 129, 130, B),
    (96, 131, 132, P), (96, 133, 134, B), (96, 135, 140, P), (96, 141, 142, B), (96, 143, 148, P),
    (96, 149, 150, B), (96, 151, 152, P), (96, 153, 154, B), (96, 155, 156, P), (96, 157, 158, B),
    (96, 159, 160, P), (96, 161, 162, B), (96, 163, 163, P), (97, 100, 100, P), (97, 101, 102, B),
    (97, 103, 104, P), (97, 105, 106, B), (97, 107, 108, P), (97, 109, 110, B), (97, 111, 112, P),
    (97, 113, 114, B), (97, 115, 116, P), (97, 117, 118, B), (97, 119, 120, P), (97, 121, 122, B),
    (97, 123, 124, P), (97, 125, 126, B), (97, 127, 128, P), (97, 129, 130, B), (97, 131, 132, P),
    (97, 133, 134, B), (97, 135, 140, P), (97, 141, 142, B), (97, 143, 148, P), (97, 149, 150, B),
    (97, 151, 152, P), (97, 153, 154, B), (97, 155, 156, P), (97, 157, 158, B), (97, 159, 160, P),
    (97, 161, 162, B), (97, 163, 163, P), (98, 100, 100, P), (98, 101, 102, B), (98, 103, 104, P),
    (98, 105, 106, B), (98, 107, 108, P), (98, 109, 110, B), (98, 111, 112, P), (98, 113, 114, B),
    (98, 115, 116, P), (98, 117, 118, B), (98, 119, 120, P), (98, 121, 122, B), (98, 123, 124, P),
    (98, 125, 126, B), (98, 127, 127, P), (98, 128, 130, B), (98, 131, 132, P), (98, 133, 134, B),
    (98, 135, 140, P), (98, 141, 142, B), (98, 143, 148, P), (98, 149, 150, B), (98, 151, 152, P),
    (98, 153, 154, B), (98, 155, 156, P), (98, 157, 158, B), (98, 159, 160, P), (98, 161, 162, B),
    (98, 163, 163, P), (99, 100, 100, P), (99, 101, 102, B), (99, 103, 104, P), (99, 105, 106, B),
    (99, 107, 108, P), (99, 109, 114, B), (99, 115, 116, P), (99, 117, 122, B), (99, 123, 124, P),
    (99, 125, 129, B), (99, 130, 132, P), (99, 133, 138, B), (99, 139, 140, P), (99, 141, 142, B),
    (99, 143, 148, P), (99, 149, 154, B), (99, 155, 156, P), (99, 157, 158, B), (99, 159, 160, P),
    (99, 161, 162, B), (99, 163, 163, P), (100, 100, 100, P), (100, 101, 102, B), (100, 103, 104, P),
    (100, 105, 106, B), (100, 107, 108, P), (100, 109, 113, B), (100, 114, 117, P), (100, 118, 121, B),
    (100, 122, 124, P), (100, 125, 128, B), (100, 129, 132, P), (100, 133, 138, B), (100, 139, 140, P),
    (100, 141, 142, B), (100, 143, 149, P), (100, 150, 153, B), (100, 154, 156, P), (100, 157, 158, B),
    (100, 159, 160, P), (100, 161, 162, B), (100, 163, 163, P), (101, 100, 163, P), (102, 100, 163, P),
];

/// TVY: grab (560,80) 80x40, record 397 of `measurements/measurement-2026-11-06-v30/probe/e1`.
#[rustfmt::skip]
const SP_TVY: &[Span] = &[
    (87, 600, 639, P), (88, 600, 639, P), (89, 600, 602, P), (89, 603, 604, B), (89, 605, 608, P),
    (89, 609, 613, B), (89, 614, 617, P), (89, 618, 621, B), (89, 622, 624, P), (89, 625, 628, B),
    (89, 629, 632, P), (89, 633, 638, B), (89, 639, 639, P), (90, 600, 601, P), (90, 602, 605, B),
    (90, 606, 608, P), (90, 609, 614, B), (90, 615, 616, P), (90, 617, 622, B), (90, 623, 624, P),
    (90, 625, 629, B), (90, 630, 632, P), (90, 633, 638, B), (90, 639, 639, P), (91, 600, 600, P),
    (91, 601, 606, B), (91, 607, 608, P), (91, 609, 610, B), (91, 611, 612, P), (91, 613, 614, B),
    (91, 615, 616, P), (91, 617, 618, B), (91, 619, 620, P), (91, 621, 622, B), (91, 623, 624, P),
    (91, 625, 626, B), (91, 627, 627, P), (91, 628, 630, B), (91, 631, 632, P), (91, 633, 634, B),
    (91, 635, 639, P), (92, 600, 600, P), (92, 601, 602, B), (92, 603, 604, P), (92, 605, 606, B),
    (92, 607, 608, P), (92, 609, 610, B), (92, 611, 612, P), (92, 613, 614, B), (92, 615, 616, P),
    (92, 617, 618, B), (92, 619, 620, P), (92, 621, 622, B), (92, 623, 624, P), (92, 625, 626, B),
    (92, 627, 628, P), (92, 629, 630, B), (92, 631, 632, P), (92, 633, 634, B), (92, 635, 639, P),
    (93, 600, 600, P), (93, 601, 602, B), (93, 603, 604, P), (93, 605, 606, B), (93, 607, 608, P),
    (93, 609, 614, B), (93, 615, 616, P), (93, 617, 618, B), (93, 619, 624, P), (93, 625, 626, B),
    (93, 627, 628, P), (93, 629, 630, B), (93, 631, 632, P), (93, 633, 637, B), (93, 638, 639, P),
    (94, 600, 600, P), (94, 601, 602, B), (94, 603, 604, P), (94, 605, 606, B), (94, 607, 608, P),
    (94, 609, 613, B), (94, 614, 616, P), (94, 617, 618, B), (94, 619, 624, P), (94, 625, 626, B),
    (94, 627, 628, P), (94, 629, 630, B), (94, 631, 632, P), (94, 633, 637, B), (94, 638, 639, P),
    (95, 600, 600, P), (95, 601, 606, B), (95, 607, 608, P), (95, 609, 610, B), (95, 611, 612, P),
    (95, 613, 614, B), (95, 615, 616, P), (95, 617, 618, B), (95, 619, 624, P), (95, 625, 626, B),
    (95, 627, 628, P), (95, 629, 630, B), (95, 631, 632, P), (95, 633, 634, B), (95, 635, 639, P),
    (96, 600, 600, P), (96, 601, 606, B), (96, 607, 608, P), (96, 609, 610, B), (96, 611, 612, P),
    (96, 613, 614, B), (96, 615, 616, P), (96, 617, 618, B), (96, 619, 624, P), (96, 625, 626, B),
    (96, 627, 628, P), (96, 629, 630, B), (96, 631, 632, P), (96, 633, 634, B), (96, 635, 639, P),
    (97, 600, 600, P), (97, 601, 602, B), (97, 603, 604, P), (97, 605, 606, B), (97, 607, 608, P),
    (97, 609, 610, B), (97, 611, 612, P), (97, 613, 614, B), (97, 615, 616, P), (97, 617, 618, B),
    (97, 619, 620, P), (97, 621, 622, B), (97, 623, 624, P), (97, 625, 626, B), (97, 627, 628, P),
    (97, 629, 630, B), (97, 631, 632, P), (97, 633, 634, B), (97, 635, 639, P), (98, 600, 600, P),
    (98, 601, 602, B), (98, 603, 604, P), (98, 605, 606, B), (98, 607, 608, P), (98, 609, 610, B),
    (98, 611, 612, P), (98, 613, 614, B), (98, 615, 616, P), (98, 617, 618, B), (98, 619, 620, P),
    (98, 621, 622, B), (98, 623, 624, P), (98, 625, 626, B), (98, 627, 627, P), (98, 628, 630, B),
    (98, 631, 632, P), (98, 633, 634, B), (98, 635, 639, P), (99, 600, 600, P), (99, 601, 602, B),
    (99, 603, 604, P), (99, 605, 606, B), (99, 607, 608, P), (99, 609, 614, B), (99, 615, 616, P),
    (99, 617, 622, B), (99, 623, 624, P), (99, 625, 629, B), (99, 630, 632, P), (99, 633, 638, B),
    (99, 639, 639, P), (100, 600, 600, P), (100, 601, 602, B), (100, 603, 604, P), (100, 605, 606, B),
    (100, 607, 608, P), (100, 609, 613, B), (100, 614, 617, P), (100, 618, 621, B), (100, 622, 624, P),
    (100, 625, 628, B), (100, 629, 632, P), (100, 633, 638, B), (100, 639, 639, P), (101, 600, 639, P),
    (102, 600, 639, P),
];

/// TVYw: grab (0,80) 80x40, record 397 of `measurements/measurement-2026-11-06-v30/probe/e1`.
#[rustfmt::skip]
const SP_TVYW: &[Span] = &[
    (88, 0, 23, P), (89, 0, 23, P), (90, 0, 0, P), (90, 1, 6, B), (90, 7, 9, P), (90, 10, 14, B),
    (90, 15, 16, P), (90, 17, 18, B), (90, 19, 20, P), (90, 21, 22, B), (90, 23, 23, P), (91, 0, 0, P),
    (91, 1, 6, B), (91, 7, 8, P), (91, 9, 14, B), (91, 15, 16, P), (91, 17, 18, B), (91, 19, 20, P),
    (91, 21, 22, B), (91, 23, 23, P), (92, 0, 0, P), (92, 1, 2, B), (92, 3, 8, P), (92, 9, 10, B),
    (92, 11, 16, P), (92, 17, 18, B), (92, 19, 20, P), (92, 21, 22, B), (92, 23, 23, P), (93, 0, 0, P),
    (93, 1, 2, B), (93, 3, 8, P), (93, 9, 10, B), (93, 11, 16, P), (93, 17, 18, B), (93, 19, 20, P),
    (93, 21, 22, B), (93, 23, 23, P), (94, 0, 0, P), (94, 1, 5, B), (94, 6, 8, P), (94, 9, 10, B),
    (94, 11, 11, P), (94, 12, 14, B), (94, 15, 16, P), (94, 17, 22, B), (94, 23, 23, P), (95, 0, 0, P),
    (95, 1, 5, B), (95, 6, 8, P), (95, 9, 10, B), (95, 11, 11, P), (95, 12, 14, B), (95, 15, 16, P),
    (95, 17, 22, B), (95, 23, 23, P), (96, 0, 0, P), (96, 1, 2, B), (96, 3, 8, P), (96, 9, 10, B),
    (96, 11, 12, P), (96, 13, 14, B), (96, 15, 16, P), (96, 17, 18, B), (96, 19, 20, P), (96, 21, 22, B),
    (96, 23, 23, P), (97, 0, 0, P), (97, 1, 2, B), (97, 3, 8, P), (97, 9, 10, B), (97, 11, 12, P),
    (97, 13, 14, B), (97, 15, 16, P), (97, 17, 18, B), (97, 19, 20, P), (97, 21, 22, B), (97, 23, 23, P),
    (98, 0, 0, P), (98, 1, 2, B), (98, 3, 8, P), (98, 9, 10, B), (98, 11, 12, P), (98, 13, 14, B),
    (98, 15, 16, P), (98, 17, 18, B), (98, 19, 20, P), (98, 21, 22, B), (98, 23, 23, P), (99, 0, 0, P),
    (99, 1, 2, B), (99, 3, 8, P), (99, 9, 10, B), (99, 11, 12, P), (99, 13, 14, B), (99, 15, 16, P),
    (99, 17, 18, B), (99, 19, 20, P), (99, 21, 22, B), (99, 23, 23, P), (100, 0, 0, P), (100, 1, 2, B),
    (100, 3, 8, P), (100, 9, 14, B), (100, 15, 16, P), (100, 17, 18, B), (100, 19, 20, P), (100, 21, 22, B),
    (100, 23, 23, P), (101, 0, 0, P), (101, 1, 2, B), (101, 3, 9, P), (101, 10, 13, B), (101, 14, 16, P),
    (101, 17, 18, B), (101, 19, 20, P), (101, 21, 22, B), (101, 23, 23, P), (102, 0, 23, P), (103, 0, 23, P),
];

/// TWA: grab (201,90) 4x230, record 399 of `measurements/measurement-2026-11-06-v30/probe/e1`.
#[rustfmt::skip]
const SP_TWA: &[Span] = &[
    (90, 203, 203, B), (90, 204, 204, P), (91, 203, 203, P), (91, 204, 204, B), (92, 203, 203, B),
    (92, 204, 204, P), (93, 203, 203, P), (93, 204, 204, B), (94, 203, 203, B), (94, 204, 204, P),
    (95, 203, 203, P), (95, 204, 204, B), (96, 203, 203, B), (96, 204, 204, P), (97, 203, 203, P),
    (97, 204, 204, B), (98, 203, 203, B), (98, 204, 204, P), (99, 203, 203, P), (99, 204, 204, B),
    (100, 201, 202, P), (100, 203, 203, B), (100, 204, 204, P), (101, 201, 203, P), (101, 204, 204, B),
    (102, 201, 202, P), (102, 203, 203, B), (102, 204, 204, P), (103, 201, 203, P), (103, 204, 204, B),
    (104, 201, 202, P), (104, 203, 203, B), (104, 204, 204, P), (105, 201, 203, P), (105, 204, 204, B),
    (106, 201, 202, P), (106, 203, 203, B), (106, 204, 204, P), (107, 201, 203, P), (107, 204, 204, B),
    (108, 201, 202, P), (108, 203, 203, B), (108, 204, 204, P), (109, 201, 203, P), (109, 204, 204, B),
    (110, 201, 202, P), (110, 203, 203, B), (110, 204, 204, P), (111, 201, 203, P), (111, 204, 204, B),
    (112, 201, 202, P), (112, 203, 203, B), (112, 204, 204, P), (113, 201, 203, P), (113, 204, 204, B),
    (114, 201, 202, P), (114, 203, 203, B), (114, 204, 204, P), (115, 201, 203, P), (115, 204, 204, B),
    (116, 203, 203, B), (116, 204, 204, P), (117, 203, 203, P), (117, 204, 204, B), (118, 203, 203, B),
    (118, 204, 204, P), (119, 203, 203, P), (119, 204, 204, B), (120, 203, 203, B), (120, 204, 204, P),
    (121, 203, 203, P), (121, 204, 204, B), (122, 203, 203, B), (122, 204, 204, P), (123, 203, 203, P),
    (123, 204, 204, B), (124, 203, 203, B), (124, 204, 204, P), (125, 203, 203, P), (125, 204, 204, B),
    (126, 201, 202, P), (126, 203, 203, B), (126, 204, 204, P), (127, 201, 203, P), (127, 204, 204, B),
    (128, 201, 202, P), (128, 203, 203, B), (128, 204, 204, P), (129, 201, 203, P), (129, 204, 204, B),
    (130, 201, 202, P), (130, 203, 203, B), (130, 204, 204, P), (131, 201, 203, P), (131, 204, 204, B),
    (132, 201, 202, P), (132, 203, 203, B), (132, 204, 204, P), (133, 201, 203, P), (133, 204, 204, B),
    (134, 201, 202, P), (134, 203, 203, B), (134, 204, 204, P), (135, 201, 203, P), (135, 204, 204, B),
    (136, 201, 202, P), (136, 203, 203, B), (136, 204, 204, P), (137, 201, 203, P), (137, 204, 204, B),
    (138, 201, 202, P), (138, 203, 203, B), (138, 204, 204, P), (139, 201, 203, P), (139, 204, 204, B),
    (140, 201, 202, P), (140, 203, 203, B), (140, 204, 204, P), (141, 201, 203, P), (141, 204, 204, B),
    (142, 203, 203, B), (142, 204, 204, P), (143, 203, 203, P), (143, 204, 204, B), (144, 203, 203, B),
    (144, 204, 204, P), (145, 203, 203, P), (145, 204, 204, B), (146, 203, 203, B), (146, 204, 204, P),
    (147, 203, 203, P), (147, 204, 204, B), (148, 203, 203, B), (148, 204, 204, P), (149, 203, 203, P),
    (149, 204, 204, B), (150, 203, 203, B), (150, 204, 204, P), (151, 203, 203, P), (151, 204, 204, B),
    (152, 201, 202, P), (152, 203, 203, B), (152, 204, 204, P), (153, 201, 203, P), (153, 204, 204, B),
    (154, 201, 202, P), (154, 203, 203, B), (154, 204, 204, P), (155, 201, 203, P), (155, 204, 204, B),
    (156, 201, 202, P), (156, 203, 203, B), (156, 204, 204, P), (157, 201, 203, P), (157, 204, 204, B),
    (158, 201, 202, P), (158, 203, 203, B), (158, 204, 204, P), (159, 201, 203, P), (159, 204, 204, B),
    (160, 201, 202, P), (160, 203, 203, B), (160, 204, 204, P), (161, 201, 203, P), (161, 204, 204, B),
    (162, 201, 202, P), (162, 203, 203, B), (162, 204, 204, P), (163, 201, 203, P), (163, 204, 204, B),
    (164, 201, 202, P), (164, 203, 203, B), (164, 204, 204, P), (165, 201, 203, P), (165, 204, 204, B),
    (166, 201, 202, P), (166, 203, 203, B), (166, 204, 204, P), (167, 201, 203, P), (167, 204, 204, B),
    (168, 203, 203, B), (168, 204, 204, P), (169, 203, 203, P), (169, 204, 204, B), (170, 203, 203, B),
    (170, 204, 204, P), (171, 203, 203, P), (171, 204, 204, B), (172, 203, 203, B), (172, 204, 204, P),
    (173, 203, 203, P), (173, 204, 204, B), (174, 203, 203, B), (174, 204, 204, P), (175, 203, 203, P),
    (175, 204, 204, B), (176, 203, 203, B), (176, 204, 204, P), (177, 203, 203, P), (177, 204, 204, B),
    (178, 201, 202, P), (178, 203, 203, B), (178, 204, 204, P), (179, 201, 203, P), (179, 204, 204, B),
    (180, 201, 202, P), (180, 203, 203, B), (180, 204, 204, P), (181, 201, 203, P), (181, 204, 204, B),
    (182, 201, 202, P), (182, 203, 203, B), (182, 204, 204, P), (183, 201, 203, P), (183, 204, 204, B),
    (184, 201, 202, P), (184, 203, 203, B), (184, 204, 204, P), (185, 201, 203, P), (185, 204, 204, B),
    (186, 201, 202, P), (186, 203, 203, B), (186, 204, 204, P), (187, 201, 203, P), (187, 204, 204, B),
    (188, 201, 202, P), (188, 203, 203, B), (188, 204, 204, P), (189, 201, 203, P), (189, 204, 204, B),
    (190, 201, 202, P), (190, 203, 203, B), (190, 204, 204, P), (191, 201, 203, P), (191, 204, 204, B),
    (192, 201, 202, P), (192, 203, 203, B), (192, 204, 204, P), (193, 201, 203, P), (193, 204, 204, B),
    (194, 203, 203, B), (194, 204, 204, P), (195, 203, 203, P), (195, 204, 204, B), (196, 203, 203, B),
    (196, 204, 204, P), (197, 203, 203, P), (197, 204, 204, B), (198, 203, 203, B), (198, 204, 204, P),
    (199, 203, 203, P), (199, 204, 204, B), (200, 203, 203, B), (200, 204, 204, P), (201, 203, 203, P),
    (201, 204, 204, B), (202, 203, 203, B), (202, 204, 204, P), (203, 203, 203, P), (203, 204, 204, B),
    (204, 201, 202, P), (204, 203, 203, B), (204, 204, 204, P), (205, 201, 203, P), (205, 204, 204, B),
    (206, 201, 202, P), (206, 203, 203, B), (206, 204, 204, P), (207, 201, 203, P), (207, 204, 204, B),
    (208, 201, 202, P), (208, 203, 203, B), (208, 204, 204, P), (209, 201, 203, P), (209, 204, 204, B),
    (210, 201, 202, P), (210, 203, 203, B), (210, 204, 204, P), (211, 201, 203, P), (211, 204, 204, B),
    (212, 201, 202, P), (212, 203, 203, B), (212, 204, 204, P), (213, 201, 203, P), (213, 204, 204, B),
    (214, 201, 202, P), (214, 203, 203, B), (214, 204, 204, P), (215, 201, 203, P), (215, 204, 204, B),
    (216, 201, 202, P), (216, 203, 203, B), (216, 204, 204, P), (217, 201, 203, P), (217, 204, 204, B),
    (218, 201, 202, P), (218, 203, 203, B), (218, 204, 204, P), (219, 201, 203, P), (219, 204, 204, B),
    (220, 203, 203, B), (220, 204, 204, P), (221, 203, 203, P), (221, 204, 204, B), (222, 203, 203, B),
    (222, 204, 204, P), (223, 203, 203, P), (223, 204, 204, B), (224, 203, 203, B), (224, 204, 204, P),
    (225, 203, 203, P), (225, 204, 204, B), (226, 203, 203, B), (226, 204, 204, P), (227, 203, 203, P),
    (227, 204, 204, B), (228, 203, 203, B), (228, 204, 204, P), (229, 203, 203, P), (229, 204, 204, B),
    (230, 203, 203, B), (230, 204, 204, P), (231, 201, 203, P), (231, 204, 204, B), (232, 201, 202, P),
    (232, 203, 203, B), (232, 204, 204, P), (233, 201, 203, P), (233, 204, 204, B), (234, 201, 202, P),
    (234, 203, 203, B), (234, 204, 204, P), (235, 201, 203, P), (235, 204, 204, B), (236, 201, 202, P),
    (236, 203, 203, B), (236, 204, 204, P), (237, 201, 203, P), (237, 204, 204, B), (238, 201, 202, P),
    (238, 203, 203, B), (238, 204, 204, P), (239, 201, 203, P), (239, 204, 204, B), (240, 201, 202, P),
    (240, 203, 203, B), (240, 204, 204, P), (241, 201, 203, P), (241, 204, 204, B), (242, 201, 202, P),
    (242, 203, 203, B), (242, 204, 204, P), (243, 201, 203, P), (243, 204, 204, B), (244, 201, 202, P),
    (244, 203, 203, B), (244, 204, 204, P), (245, 201, 203, P), (245, 204, 204, B), (246, 201, 202, P),
    (246, 203, 203, B), (246, 204, 204, P), (247, 203, 203, P), (247, 204, 204, B), (248, 203, 203, B),
    (248, 204, 204, P), (249, 203, 203, P), (249, 204, 204, B), (250, 203, 203, B), (250, 204, 204, P),
    (251, 203, 203, P), (251, 204, 204, B), (252, 203, 203, B), (252, 204, 204, P), (253, 203, 203, P),
    (253, 204, 204, B), (254, 203, 203, B), (254, 204, 204, P), (255, 203, 203, P), (255, 204, 204, B),
    (256, 203, 203, B), (256, 204, 204, P), (257, 203, 203, P), (257, 204, 204, B), (258, 201, 202, P),
    (258, 203, 203, B), (258, 204, 204, P), (259, 201, 203, P), (259, 204, 204, B), (260, 201, 202, P),
    (260, 203, 203, B), (260, 204, 204, P), (261, 201, 203, P), (261, 204, 204, B), (262, 201, 202, P),
    (262, 203, 203, B), (262, 204, 204, P), (263, 201, 203, P), (263, 204, 204, B), (264, 201, 202, P),
    (264, 203, 203, B), (264, 204, 204, P), (265, 201, 203, P), (265, 204, 204, B), (266, 201, 202, P),
    (266, 203, 203, B), (266, 204, 204, P), (267, 201, 203, P), (267, 204, 204, B), (268, 201, 202, P),
    (268, 203, 203, B), (268, 204, 204, P), (269, 201, 203, P), (269, 204, 204, B), (270, 201, 202, P),
    (270, 203, 203, B), (270, 204, 204, P), (271, 201, 203, P), (271, 204, 204, B), (272, 201, 202, P),
    (272, 203, 203, B), (272, 204, 204, P), (273, 201, 203, P), (273, 204, 204, B), (274, 203, 203, B),
    (274, 204, 204, P), (275, 203, 203, P), (275, 204, 204, B), (276, 203, 203, B), (276, 204, 204, P),
    (277, 203, 203, P), (277, 204, 204, B), (278, 203, 203, B), (278, 204, 204, P), (279, 203, 203, P),
    (279, 204, 204, B), (280, 203, 203, B), (280, 204, 204, P), (281, 203, 203, P), (281, 204, 204, B),
    (282, 203, 203, B), (282, 204, 204, P), (283, 203, 203, P), (283, 204, 204, B), (284, 203, 203, B),
    (284, 204, 204, P), (285, 201, 203, P), (285, 204, 204, B), (286, 201, 202, P), (286, 203, 203, B),
    (286, 204, 204, P), (287, 201, 203, P), (287, 204, 204, B), (288, 201, 202, P), (288, 203, 203, B),
    (288, 204, 204, P), (289, 201, 203, P), (289, 204, 204, B), (290, 201, 202, P), (290, 203, 203, B),
    (290, 204, 204, P), (291, 201, 203, P), (291, 204, 204, B), (292, 201, 202, P), (292, 203, 203, B),
    (292, 204, 204, P), (293, 201, 203, P), (293, 204, 204, B), (294, 201, 202, P), (294, 203, 203, B),
    (294, 204, 204, P), (295, 201, 203, P), (295, 204, 204, B), (296, 201, 202, P), (296, 203, 203, B),
    (296, 204, 204, P), (297, 201, 203, P), (297, 204, 204, B), (298, 201, 202, P), (298, 203, 203, B),
    (298, 204, 204, P), (299, 201, 203, P), (299, 204, 204, B), (300, 201, 202, P), (300, 203, 203, B),
    (300, 204, 204, P), (301, 203, 203, P), (301, 204, 204, B),
];
