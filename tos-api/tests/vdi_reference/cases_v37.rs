//! V37 (TASK/V37.md): the horizontal `v_justified` aligned against `max_x`.
//!
//! `gdp_justified` draws nothing itself: it hands its run to `output_text`
//! with `width = max_x` (`vdi/vdi_text.c:1288`). `output_text` takes the
//! alignment offset `delh` from `h_align` against that width, minus the
//! outline's start pixel (`:347-364`), adds the skew additive of the
//! vertical alignment (`d1`/`d2`, the font's `left_offset`/`right_offset`
//! when `F_SKEW` is set, `:332-341`, `:366-390`) and starts the run at
//! `DESTX = x - delh - outline` (`:410`); only a negative `max_x` falls back
//! to `calc_width` (`:305-306`). The underline starts at that `DESTX`
//! (`:412`). Before V37 NewTOS' horizontal path started at `x` and ignored
//! `delh` altogether (the rotated path already had it).
//!
//! Source of every expected value: the probe cases TVO/TVP (`tv`, V30), the
//! justified cases ZG002/ZG003/ZG008/ZG009 (`tz_misc`, V41) and T3700–T3741
//! (`t37`, added by V37) of `tosdump/probes/vdprobe.c` on EmuTOS 1a7484ab,
//! Hatari Falcon030 truecolor, run
//! `measurements/measurement-2026-11-07-v37/RUNINFO.TXT` (`probe/e1`): one
//! FNV-1a over each grab rectangle's screen words and the count of non-white
//! words (`method/gen-grabs.py`, the format of `cases_v33`). A failing grab
//! is looked at in the probe's maps (`vdprobe-compare.py --maps`).
//!
//! T37: `'AB CD EF'` interword at (300,100), length 200 unless named;
//! T3700–T3727 in the default font (6×6 here: `left_offset` 0, `right_offset`
//! 3), T3728–T3741 in the 8×16 font (1, 7). Skew is drawn transparent: the
//! replace-mode cell of a skewed glyph is TASK/V41, which is why ZG002/003/
//! 008/009 (skew in replace on a pen-2 ground) stay ignored here.
//!
//! Hand-written, like `cases_v20`..`cases_v35`.

use crate::harness::{reset_vdi, screen_word, workstation, Call, Pb, VDI_LOCK};

/// `(name, (x, y, w, h), FNV-1a of the words, non-white words)` of `method/gen-grabs.py`.
type GrabHash = (&'static str, (i16, i16, i16, i16), u64, u32);

#[rustfmt::skip]
const GRABS: &[GrabHash] = &[
    ("TVO", (150, 80, 320, 40), 0x2ebf7e0be2c7297d, 441),
    ("TVP", (80, 80, 320, 40), 0xee601684f54d8efd, 441),
    ("ZG002", (0, 60, 320, 80), 0xd2d0b330670b3b7f, 25600),
    ("ZG003", (0, 60, 320, 80), 0x104ed4a813abb14f, 25600),
    ("ZG008", (0, 60, 320, 80), 0x1744b744a9162dbf, 25600),
    ("ZG009", (0, 60, 320, 80), 0x91e1d3bd71e38a79, 25600),
    ("T3700", (60, 60, 520, 64), 0x98f602445e7c6873, 288),
    ("T3701", (60, 60, 520, 64), 0x14da55d48a6c9663, 83),
    ("T3702", (60, 60, 520, 64), 0xd0b4741e7c92477f, 512),
    ("T3703", (60, 60, 520, 64), 0x1107c772cd1cc83d, 441),
    ("T3704", (60, 60, 520, 64), 0xc46a40eac4f52f4f, 281),
    ("T3705", (60, 60, 520, 64), 0xd3aca89b072a175f, 649),
    ("T3706", (60, 60, 520, 64), 0xe87cc1ccced3c0b3, 288),
    ("T3707", (60, 60, 520, 64), 0x6ed7793b9b37bc23, 83),
    ("T3708", (60, 60, 520, 64), 0x648c5f00a28c5eaf, 512),
    ("T3709", (60, 60, 520, 64), 0x3b6749594920973d, 441),
    ("T3710", (60, 60, 520, 64), 0x423b37cdd711688f, 281),
    ("T3711", (60, 60, 520, 64), 0x1734aaa7ed20020f, 649),
    ("T3712", (60, 60, 520, 64), 0x3f7fc358ff3786b3, 288),
    ("T3713", (60, 60, 520, 64), 0xed873493d9ace9e3, 83),
    ("T3714", (60, 60, 520, 64), 0xa5057ef46df5b39f, 512),
    ("T3715", (60, 60, 520, 64), 0x65329963b99abd7d, 441),
    ("T3716", (60, 60, 520, 64), 0x845d40e634b633cf, 281),
    ("T3717", (60, 60, 520, 64), 0xa2b04e574c4063bf, 649),
    ("T3718", (60, 60, 520, 64), 0xeb4cd76d1a6d0083, 83),
    ("T3719", (60, 60, 520, 64), 0x6e9aeee8765c9fb3, 83),
    ("T3720", (60, 60, 520, 64), 0x44ec67fe329397a3, 83),
    ("T3721", (60, 60, 520, 64), 0x44ec67fe329397a3, 83),
    ("T3722", (60, 60, 520, 64), 0x6e9aeee8765c9fb3, 83),
    ("T3723", (60, 60, 520, 64), 0x1bc0527dbfc2b6c1, 288),
    ("T3724", (60, 60, 520, 64), 0xce38a0d8d6b07dc1, 288),
    ("T3725", (60, 60, 520, 64), 0x2b9c343cb954b6cf, 329),
    ("T3726", (60, 60, 520, 64), 0xfb6d03a7bb83949d, 310),
    ("T3727", (60, 60, 520, 64), 0x56d4c4bd483945f3, 320),
    ("T3728", (60, 60, 520, 64), 0x27b1a0e5f62a12cf, 289),
    ("T3729", (60, 60, 520, 64), 0x7f279226f60a7ad9, 482),
    ("T3730", (60, 60, 520, 64), 0xab81f5ff7acf6b5b, 1561),
    ("T3731", (60, 60, 520, 64), 0x341e4f66ef0cfa0f, 289),
    ("T3732", (60, 60, 520, 64), 0xbe466bb429a8d959, 482),
    ("T3733", (60, 60, 520, 64), 0x3106b1278839b56b, 1561),
    ("T3734", (60, 60, 520, 64), 0xf696b24bbdbef74f, 289),
    ("T3735", (60, 60, 520, 64), 0xf115276a715819d9, 482),
    ("T3736", (60, 60, 520, 64), 0x17f6517a8700c07b, 1561),
    ("T3737", (60, 60, 520, 64), 0x9672527954f50dff, 289),
    ("T3738", (60, 60, 520, 64), 0x1ca7c54e9429a48f, 289),
    ("T3739", (60, 60, 520, 64), 0x5fc33510e27893bf, 289),
    ("T3740", (60, 60, 520, 64), 0x9ac68256ea2cb3cf, 289),
    ("T3741", (60, 60, 520, 64), 0x5ab9c6ee472f96bf, 289),
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

fn leak(v: Vec<i16>) -> &'static [i16] {
    Box::leak(v.into_boxed_slice())
}

/// One VDI call with owned arrays (the probe's `i1`/`i2`/`p1`/`justified`).
fn call(opcode: i16, sub: i16, intin: &[i16], ptsin: &[i16]) -> Call {
    Call { opcode, sub, intin: leak(intin.to_vec()), ptsin: leak(ptsin.to_vec()) }
}

/// The probe's `justified(x, y, len, iw, ic, s)`: `v_justified`, `PTSIN` (x, y) (len, 0).
fn justified(x: i16, y: i16, len: i16, iw: i16, ic: i16, s: &str) -> Call {
    let mut intin = vec![iw, ic];
    intin.extend(s.bytes().map(i16::from));
    call(11, 10, &intin, &[x, y, len, 0])
}

/// The probe's `tbegin` (the dumper's reset with the text part), `calls` in order, then [`compare`].
fn run(name: &str, calls: &[Call]) {
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
    compare(name);
}

/// The probe's `t37_case`: `vst_height` (0 = the default font), `vst_alignment(h, v)`,
/// `vst_effects(eff)`, `vswr_mode(mode)`, `v_justified`, `vswr_mode(1)`.
#[allow(clippy::too_many_arguments)]
fn t37(name: &str, height: i16, h: i16, v: i16, eff: i16, mode: i16, len: i16, iw: i16, ic: i16, s: &str) {
    let mut calls = Vec::new();
    if height != 0 {
        calls.push(call(12, 0, &[], &[0, height]));
    }
    calls.push(call(39, 0, &[h, v], &[]));
    calls.push(call(106, 0, &[eff], &[]));
    calls.push(call(32, 0, &[mode], &[]));
    calls.push(justified(300, 100, len, iw, ic, s));
    calls.push(call(32, 0, &[1], &[]));
    run(name, &calls);
}

const SKEW: i16 = 4;
const OUTLINE: i16 = 16;
const UNDER: i16 = 8;

/// `h_align` × effects in the default font (T3700–T3717): skew transparent, the rest in replace.
fn t37_matrix(first: usize, h: i16) {
    for (k, eff) in [0, SKEW, OUTLINE, UNDER, SKEW | UNDER, OUTLINE | UNDER].into_iter().enumerate() {
        let mode = if eff & SKEW != 0 { 2 } else { 1 };
        t37(&format!("T37{:02}", first + k), 0, h, 0, eff, mode, 200, 1, 0, "AB CD EF");
    }
}

/// Left alignment: `delh` 0, the run starts at `x` (the default font has no `d1`).
#[test]
fn v37_probe_t3700_t3705_left() {
    t37_matrix(0, 0);
}

/// Centre: `delh = max_x / 2 - outline` (`vdi_text.c:358-360`), not the natural width.
#[test]
fn v37_probe_t3706_t3711_centre_against_max_x() {
    t37_matrix(6, 1);
}

/// Right: `delh = max_x - 2 * outline` (`:361-363`).
#[test]
fn v37_probe_t3712_t3717_right_against_max_x() {
    t37_matrix(12, 2);
}

/// The skew additive of each `v_align` (`:366-390`) with centre alignment, default font:
/// half `half*d2/top`, ascent `ascent*d2/top`, bottom 0, descent `descent*d1/bottom`, top `d1+d2`.
#[test]
fn v37_probe_t3718_t3722_skew_additive_per_v_align() {
    for v in 1..=5 {
        t37(&format!("T37{:02}", 17 + v), 0, 1, v, SKEW, 2, 200, 1, 0, "AB CD EF");
    }
}

/// Interchar justification, centre and right, and right with skew + underline.
#[test]
fn v37_probe_t3723_t3725_interchar() {
    t37("T3723", 0, 1, 0, 0, 1, 200, 0, 1, "ABCDEFGH");
    t37("T3724", 0, 2, 0, 0, 1, 200, 0, 1, "ABCDEFGH");
    t37("T3725", 0, 2, 0, SKEW | UNDER, 2, 200, 0, 1, "ABCDEFGH");
}

/// A negative `max_x` aligns against `calc_width` (`:305-306`).
#[test]
fn v37_probe_t3726_negative_max_x_uses_calc_width() {
    t37("T3726", 0, 1, 0, UNDER, 1, -40, 1, 0, "AB CD EF");
}

/// `max_x` 0 is not negative: right alignment is `0 - 2 * outline`.
#[test]
fn v37_probe_t3727_zero_max_x() {
    t37("T3727", 0, 2, 0, OUTLINE | UNDER, 1, 0, 1, 0, "AB CD EF");
}

/// The 8×16 font (`left_offset` 1): the baseline's skew additive `d1` moves the run one pixel left
/// at every `h_align` (T3728–T3736; `delh += d1`, `:369-370`), skew transparent, outline in replace.
#[test]
fn v37_probe_t3728_t3736_8x16_skew_d1() {
    for h in 0..3 {
        for (k, eff) in [SKEW, SKEW | UNDER, OUTLINE | UNDER].into_iter().enumerate() {
            let mode = if eff & SKEW != 0 { 2 } else { 1 };
            t37(&format!("T37{:02}", 28 + 3 * h as usize + k), 13, h, 0, eff, mode, 200, 1, 0, "AB CD EF");
        }
    }
}

/// The 8×16 font, left alignment: the skew additive of each `v_align` alone (T3737–T3741).
#[test]
fn v37_probe_t3737_t3741_8x16_skew_additive_per_v_align() {
    for v in 1..=5 {
        t37(&format!("T37{:02}", 36 + v), 13, 0, v, SKEW, 2, 200, 1, 0, "AB CD EF");
    }
}

/// TVO (V30): underline, centre, 'AB CD EF' at (300,100), length 200, default font.
#[test]
fn v37_probe_tvo_centre_underline() {
    run("TVO", &[call(106, 0, &[UNDER], &[]), call(39, 0, &[1, 0], &[]), justified(300, 100, 200, 1, 0, "AB CD EF")]);
}

/// TVP (V30): underline, right, 'AB CD EF' at (300,100), length 200.
#[test]
fn v37_probe_tvp_right_underline() {
    run("TVP", &[call(106, 0, &[UNDER], &[]), call(39, 0, &[2, 0], &[]), justified(300, 100, 200, 1, 0, "AB CD EF")]);
}

/// The probe's `tz_misc` variants 2/3 (ZG002/003 skew, ZG008/009 bold+skew+outline): 8×16,
/// a pen-2 ground (0,60) 320×80, replace mode, `v_justified` at (20,100) length 280.
fn zg(name: &str, eff: i16, iw: i16, ic: i16) {
    run(
        name,
        &[
            call(12, 0, &[], &[0, 13]),
            call(25, 0, &[2], &[]),
            call(114, 0, &[], &[0, 60, 319, 139]),
            call(25, 0, &[1], &[]),
            call(106, 0, &[eff], &[]),
            justified(20, 100, 280, iw, ic, "AB CD EF"),
        ],
    );
}

#[test]
#[ignore = "skew in replace mode: the cell background of TASK/V41 (R1 and others), not V37's d1"]
fn v37_probe_zg002_skew_replace_interword() {
    zg("ZG002", SKEW, 1, 0);
}

#[test]
#[ignore = "skew in replace mode: the cell background of TASK/V41 (R1 and others), not V37's d1"]
fn v37_probe_zg003_skew_replace_interchar() {
    zg("ZG003", SKEW, 0, 1);
}

#[test]
#[ignore = "bold+skew+outline in replace mode: TASK/V41 (R1, R2, R7), not V37's d1"]
fn v37_probe_zg008_all_effects_replace_interword() {
    zg("ZG008", 1 | SKEW | OUTLINE, 1, 0);
}

#[test]
#[ignore = "bold+skew+outline in replace mode: TASK/V41 (R1, R2, R7), not V37's d1"]
fn v37_probe_zg009_all_effects_replace_interchar() {
    zg("ZG009", 1 | SKEW | OUTLINE, 0, 1);
}
