//! V01–V15 (TASK/V01.md … V15.md): the thirteen `vdi-all.tsv` cases whose
//! TT reference differs from NewTOS, replayed against the **Falcon truecolor**
//! EmuTOS reference — the oracle for NewTOS' own profile (16 planes, 256
//! pens, ADR-024) — instead of the four-plane TT one.
//!
//! Hand-written, not generated: `mkrusttests` emits a whole table, and the
//! rest of `vdi-all.tsv` is already covered by `cases_r2.rs`/`cases_r3.rs`
//! against the TT dump. This module only pins the cells the V tickets name.
//!
//! **The reference.** `tosdump/reference/EMUTOS-1a7484ab-VDI-all-falcontc.BIN`:
//! the oracle, EmuTOS 1a7484ab (`docs/analysis/ORACLE.md`), byte-identical
//! to `EMUTOS14-VDI-all-falcontc.BIN`, which EmuTOS 1.4 (the release ROM
//! `etos512de.img`) produced the same way, on Hatari 2.6.1 `--machine
//! falcon --monitor vga`, video mode `0x001C` = `VIDEL_VGA | VIDEL_80COL |
//! VIDEL_TRUECOLOR` set through `EMUDESK.INF` `#E`, `$FF8266 = 0x0100`
//! (`SPS_HICOLOR`), `DEV_TAB 639,479,…,256,…,0,…`. Same dumper binary
//! (`tosdump/deploy-tt/DUMPVDI.PRG`, build 19b7), same table
//! (`vdi-all.tsv`, CRC D135ADAF), same reset. Run and diffs:
//! `measurements/measurement-2026-09-21-vdi-qemu/`.
//!
//! **What it settles.** EmuTOS itself, same ROM, TT-medium versus Falcon
//! truecolor, differs in exactly these thirteen cases and exactly these
//! cells (`DIFF-emutos-tt-vs-emutos-falcontc.txt`) — so every one of them is
//! a function of the device by EmuTOS' own arithmetic:
//!
//! | case | cell | rule | TT | Falcon TC | EmuTOS line |
//! |------|------|------|----|-----------|-------------|
//! | 33, 274 | `INTOUT[13]` | `(v_planes<8) ? 1<<v_planes : 256` | 16 | 256 | `vdi_control.c:206` |
//! | 33, 274 | `INTOUT[39]` | `get_palette()`: VIDEL non-compat, non-4bpp → 0 | 4096 | 0 | `bios/screen.c:831-849` |
//! | 34, 250, 252 | `INTOUT[4]` | `v_planes` | 4 | 16 | `vdi_control.c:210` |
//! | 34, 250, 252 | `INTOUT[5]` | `(v_planes==16 \|\| MON_MONO) ? 0 : 1` | 1 | 0 | `vdi_control.c:211-213` |
//! | 42 (`vsl_color 99`) | `INTOUT[0]` | `validate_color_index`: `< numcolors` | 1 | 99 | `vdi_control.c:224-230`, `vdi_line.c:141-149` |
//! | 113 (`vsm_color 99`) | `INTOUT[0]` | same | 1 | 99 | `vdi_marker.c:71-79` |
//! | 72 (`vq_color 255`) | `INTOUT[0]` | `colnum >= numcolors → -1` | -1 | 255 | `vdi_col.c:854-859` |
//! | 270, 272 | `INTOUT[0..1]` | `TRUECOLOR_MODE`: `0`, raw word; pen 1 = `palette[255]` = 0 | 15, 1 | 0, 0 | `vdi_fill.c:1104-1114`, `vdi_col.c:737` |
//! | 54, 160, 161 | `INTOUT[1]` | after `v_clrwk` (`memset 0xff`): raw word | 0 | **0xFFFF** | `vdi_control.c:610-618`, `vdi_fill.c:671-681` |
//!
//! NewTOS on QEMU `raspi4b` at 640×480 matched the Falcon truecolor column
//! in every cell but the last row, where it answered `0xFFDF` — the V11/V12
//! defect (VDI-OVL-001), pinned in `tos-api/tests/vdi_reference_overlay_bit.rs`
//! against the kernel's transcribed device-pixel model and fixed there
//! 2026-09-21 (the overlay bit lives in the X byte of the 32-bpp store). The
//! harness double used here shares that model since the same day; the three
//! cases are replayed so the rest of their records stays pinned.
//!
//! **Cells the harness excludes.** `harness::MACHINE_EXCLUDED` still applies
//! (it is keyed on opcode/index, not on the reference): for the excluded
//! slots the EmuTOS formula is asserted instead of the dump's word. Against
//! this reference the two agree, so nothing is lost; for `vsl_color` (17)
//! there is no exclusion and the dump's 99 is compared directly.
//!
//! **Geometry-derived cells** (`INTOUT[0..1]` = `V_REZ_HZ-1`/`V_REZ_VT-1`,
//! `vdi_control.c:203-204`; the clip rectangle `PTSOUT[0..3]`,
//! `vdi_control.c:239-250,643-648`; `vq_chcells` = `V_REZ_VT/16`,
//! `V_REZ_HZ/8`, `vdi_esc.c:62-67`, `bios/font.c:80-81`) differ only
//! between 640×480 and 1920×1080 (V01, V06, V13, V14); at the reference
//! geometry they are byte-equal on all three machines and are compared here
//! as plain words.

use crate::harness::{check, reset_vdi, workstation, Call, Case, Pb, VDI_LOCK};

static FALCON_TC: &[u8] = include_bytes!("../../../tosdump/reference/EMUTOS-1a7484ab-VDI-all-falcontc.BIN");

// ── V01 / V02 / V03: vq_extnd(0) — DEV_TAB ────────────────────────────────

#[test]
fn v01_v02_v03_case_0033_vq_extnd_0_dev_tab_on_the_falcon_truecolor_reference() {
    check(FALCON_TC, &Case {
        prelude: &[],
        id: 33,
        rec: 4,
        opcode: 102,
        sub: 0,
        intin: &[0],
        ptsin: &[],
        comment: "vq_extnd 0 SIZ_TAB and DEV_TAB",
    });
}

// ── V04 / V05 / V06: vq_extnd(1) — INQ_TAB and the clip rectangle ─────────

#[test]
fn v04_v05_v06_case_0034_vq_extnd_1_inq_tab_and_full_screen_clip() {
    check(FALCON_TC, &Case {
        prelude: &[],
        id: 34,
        rec: 5,
        opcode: 102,
        sub: 0,
        intin: &[1],
        ptsin: &[],
        comment: "vq_extnd 1 clipping and extended info",
    });
}

#[test]
fn v04_v05_v06_case_0250_vq_extnd_1_after_vs_clip_on() {
    check(FALCON_TC, &Case {
        prelude: &[Call { opcode: 129, sub: 0, intin: &[1], ptsin: &[10, 20, 300, 400] }],
        id: 250,
        rec: 169,
        opcode: 102,
        sub: 0,
        intin: &[1],
        ptsin: &[],
        comment: "vq_extnd 1 after vs_clip on",
    });
}

#[test]
fn v04_v05_v06_case_0252_vq_extnd_1_after_vs_clip_off() {
    check(FALCON_TC, &Case {
        prelude: &[Call { opcode: 129, sub: 0, intin: &[0], ptsin: &[] }],
        id: 252,
        rec: 170,
        opcode: 102,
        sub: 0,
        intin: &[1],
        ptsin: &[],
        comment: "vq_extnd 1 after vs_clip off",
    });
}

// ── V07 / V08 / V09: pen range from numcolors ─────────────────────────────

#[test]
fn v07_case_0042_vsl_color_99_is_a_valid_pen_on_256_colours() {
    check(FALCON_TC, &Case {
        prelude: &[],
        id: 42,
        rec: 12,
        opcode: 17,
        sub: 0,
        intin: &[99],
        ptsin: &[],
        comment: "vsl_color out of range",
    });
}

#[test]
fn v08_case_0113_vsm_color_99_is_a_valid_pen_on_256_colours() {
    check(FALCON_TC, &Case {
        prelude: &[],
        id: 113,
        rec: 74,
        opcode: 20,
        sub: 0,
        intin: &[99],
        ptsin: &[],
        comment: "vsm_color out of range",
    });
}

#[test]
fn v09_case_0072_vq_color_255_is_a_valid_pen_on_256_colours() {
    check(FALCON_TC, &Case {
        prelude: &[],
        id: 72,
        rec: 35,
        opcode: 26,
        sub: 0,
        intin: &[255, 0],
        ptsin: &[],
        comment: "vq_color pen 255 requested",
    });
}

/// The boundaries the tickets ask for, per profile, straight from
/// `validate_color_index` (`vdi_control.c:224-230`: `< 0 || >= numcolors`
/// → 1) and `vdi_vq_color` (`vdi_col.c:854-859`: same test → -1) with
/// `numcolors = DEV_TAB[13] = 256` (`vdi_defs.h:98`, `vdi_control.c:206`).
/// On the TT (16 pens) the same inputs 16 and 255 are the first invalid and
/// an invalid pen; here 255 is the last valid one and 256 the first invalid.
#[test]
fn v07_v08_v09_pen_boundaries_follow_numcolors() {
    let _guard = VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let wk = workstation();
    let mut pb = Pb::new();
    reset_vdi(&mut pb, wk);

    let numcolors = 256i16;
    // (pen, vsl_color/vsm_color echo, vq_color INTOUT[0])
    let table = [
        (-1, 1, -1),
        (0, 0, 0),
        (numcolors - 1, numcolors - 1, numcolors - 1),
        (numcolors, 1, -1),
        (999, 1, -1),
    ];
    for (pen, want_set, want_vq) in table {
        for (opcode, name) in [(17, "vsl_color"), (20, "vsm_color")] {
            pb.contrl.fill(0);
            pb.contrl[0] = opcode;
            pb.contrl[3] = 1;
            pb.contrl[6] = wk.handle;
            pb.intin[0] = pen;
            pb.call();
            assert_eq!(pb.intout[0], want_set, "{name}({pen}) on numcolors={numcolors}");
        }
        pb.contrl.fill(0);
        pb.contrl[0] = 26;
        pb.contrl[3] = 2;
        pb.contrl[6] = wk.handle;
        pb.intin[0] = pen;
        pb.intin[1] = 0;
        pb.call();
        assert_eq!(pb.intout[0], want_vq, "vq_color({pen}, 0) on numcolors={numcolors}");
    }
}

// ── V10 / V11: v_get_pixel under TRUECOLOR_MODE ───────────────────────────

#[test]
fn v11_case_0054_v_get_pixel_background_after_v_clrwk_is_the_raw_word() {
    check(FALCON_TC, &Case {
        prelude: &[],
        id: 54,
        rec: 23,
        opcode: 105,
        sub: 0,
        intin: &[],
        ptsin: &[5, 5],
        comment: "v_get_pixel background pixel after v_clrwk",
    });
}

#[test]
fn v11_case_0160_v_get_pixel_top_left_corner_is_the_raw_word() {
    check(FALCON_TC, &Case {
        prelude: &[],
        id: 160,
        rec: 117,
        opcode: 105,
        sub: 0,
        intin: &[],
        ptsin: &[0, 0],
        comment: "v_get_pixel top left corner",
    });
}

#[test]
fn v11_case_0161_v_get_pixel_bottom_right_corner_is_the_raw_word() {
    check(FALCON_TC, &Case {
        prelude: &[],
        id: 161,
        rec: 118,
        opcode: 105,
        sub: 0,
        intin: &[],
        ptsin: &[639, 479],
        comment: "v_get_pixel bottom right corner",
    });
}

#[test]
fn v10_case_0270_v_get_pixel_inside_a_pen_1_box_is_zero_and_the_black_word() {
    check(FALCON_TC, &Case {
        prelude: &[
            Call { opcode: 23, sub: 0, intin: &[1], ptsin: &[] },
            Call { opcode: 11, sub: 1, intin: &[], ptsin: &[40, 40, 140, 120] },
        ],
        id: 270,
        rec: 179,
        opcode: 105,
        sub: 0,
        intin: &[],
        ptsin: &[80, 80],
        comment: "v_get_pixel inside the drawn box",
    });
}

#[test]
fn v10_case_0272_v_get_pixel_on_a_pen_1_line_is_zero_and_the_black_word() {
    check(FALCON_TC, &Case {
        prelude: &[Call { opcode: 6, sub: 0, intin: &[], ptsin: &[0, 0, 200, 200] }],
        id: 272,
        rec: 180,
        opcode: 105,
        sub: 0,
        intin: &[],
        ptsin: &[100, 100],
        comment: "v_get_pixel on the drawn line",
    });
}

// ── V13: vq_chcells ───────────────────────────────────────────────────────

#[test]
fn v13_case_0173_vq_chcells_rows_and_columns_at_640x480() {
    check(FALCON_TC, &Case {
        prelude: &[],
        id: 173,
        rec: 128,
        opcode: 5,
        sub: 1,
        intin: &[],
        ptsin: &[],
        comment: "vq_chcells rows and columns",
    });
}

// ── V14 / V15: v_opnvwk — the second workstation's tables and its handle ──

// Record layout, `tosdump/core/tosdump.h:92-101` (record format 2).
const OFF_RET: usize = 4;
const OFF_CONTRL: usize = 8;
const OFF_INTOUT: usize = 800;
const OFF_PTSOUT: usize = 1056;
const OFF_CONTRL_BEFORE: usize = 1568;

fn be16(b: &[u8], off: usize) -> i16 {
    (((b[off] as u16) << 8) | b[off + 1] as u16) as i16
}

/// Replays the last case of the table by hand rather than through `check`,
/// because V15 says what `check` cannot: the handle a `v_opnvwk` returns is
/// an **identity**, not a number to be byte-compared. On the dumper's
/// machines it was 3 (physical 1, the dumper's own 2, this one 3 — TT,
/// Falcon truecolor and NewTOS/QEMU alike, `measurements/measurement-2026-09-21-vdi-qemu/
/// handles-vdi.txt`); in this test binary `cases_r3.rs` case 145 opens a
/// third workstation too and never closes it, so the absolute number here
/// depends on test order and is deliberately not asserted.
///
/// What is asserted, per V15: (1) `contrl[6]` went in as the caller's own
/// handle and the reference agrees; (2) the call answered with a **new**
/// handle, distinct from the physical and the calling one; (3) the new
/// workstation's `DEV_TAB`/`SIZ_TAB` (`INTOUT[0..45]`, `PTSOUT[0..12]`), the
/// output counts and `d0` are what the Falcon truecolor reference recorded —
/// V14's "W/H, pen count, precision agree with V01–V03"; (4) the handle is
/// live until `v_clsvwk` and dead after it (the dispatcher treats an unknown
/// handle as a VDI no-op that writes nothing, `tos-api/src/vdi/mod.rs`,
/// `vwk::resolve` → `None`).
#[test]
fn v14_v15_case_0274_v_opnvwk_second_workstation_tables_and_handle_identity() {
    let _guard = VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let wk = workstation();
    let mut pb = Pb::new();
    reset_vdi(&mut pb, wk);

    let rec = &FALCON_TC[crate::harness::HDR_SIZE + 181 * crate::harness::REC_SIZE..]
        [..crate::harness::REC_SIZE];
    assert_eq!(be16(rec, 0), 274, "record 181 of the Falcon truecolor dump is case 274");

    // (1) the handle word going in: ours, and the reference's.
    assert_eq!(be16(rec, OFF_CONTRL_BEFORE + 6 * 2), wk.handle, "case 274 went in on the dumper's own handle");

    // Exactly as `check` (and the dumper) issue a call: everything poisoned,
    // then only the words the caller owns are written.
    pb.poison();
    pb.contrl[0] = 100;
    pb.contrl[1] = 0;
    pb.contrl[3] = 11;
    pb.contrl[5] = 0;
    pb.contrl[6] = wk.handle;
    pb.intin[..11].copy_from_slice(&[1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2]);
    let d0 = pb.call();

    // (2) a new, distinct handle came back.
    let h3 = pb.contrl[6];
    assert!(h3 > 0 && h3 != wk.phys && h3 != wk.handle, "v_opnvwk answered with a fresh handle, got {h3}");

    // (3) everything else in the record: d0, contrl except the handle,
    // DEV_TAB → INTOUT[0..45], SIZ_TAB → PTSOUT[0..12].
    assert_eq!(d0, be16(rec, OFF_RET + 2), "d0 (flip_y) of v_opnvwk");
    for i in (0..12).filter(|&i| i != 6) {
        assert_eq!(pb.contrl[i], be16(rec, OFF_CONTRL + i * 2), "contrl[{i}] after v_opnvwk");
    }
    for i in 0..45 {
        assert_eq!(
            pb.intout[i],
            be16(rec, OFF_INTOUT + i * 2),
            "DEV_TAB[{i}] of the second workstation vs the Falcon truecolor reference"
        );
    }
    for i in 0..12 {
        assert_eq!(
            pb.ptsout[i],
            be16(rec, OFF_PTSOUT + i * 2),
            "SIZ_TAB[{i}] of the second workstation vs the Falcon truecolor reference"
        );
    }

    // (4) lifetime: live now, dead after v_clsvwk.
    let probe = |pb: &mut Pb| -> bool {
        pb.poison();
        pb.contrl.fill(0);
        pb.contrl[0] = 102; // vq_extnd 0 on h3
        pb.contrl[3] = 1;
        pb.contrl[6] = h3;
        pb.intin[0] = 0;
        pb.call();
        pb.intout[0] != crate::harness::POISON
    };
    assert!(probe(&mut pb), "the new handle {h3} answers vq_extnd while open");
    pb.contrl.fill(0);
    pb.contrl[0] = 101; // v_clsvwk
    pb.contrl[6] = h3;
    pb.call();
    assert!(!probe(&mut pb), "the closed handle {h3} is a VDI no-op after v_clsvwk");
}
