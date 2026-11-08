//! V54 — `vsc_form` only stores the new pointer form; a visible pointer keeps
//! its old image until the next redraw (`TASK/V54.md`, audit VDI-WK-003,
//! probe `tosdump/probes/wkprobe.c` cases F1, F2).
//!
//! EmuTOS `vdi_vsc_form` → `linea_transform_mouse` → `set_mouse_form`
//! (`references/emutos/vdi/vdi_mouse.c:513-516`, `:1138-1141`, `:450-492`)
//! copies hotspot, colours, mask and data into `mouse_cdb` between
//! `mouse_flag += 1` and `mouse_flag -= 1` and calls neither `cur_replace`
//! nor `cur_display`. The pointer on the screen and the save-under behind it
//! stay as they were; the new form appears at the next redraw — `vb_draw`
//! after a packet (`:668-686`), or `hide_cur` + `dis_cur` (`:127-171`).
//!
//! The AES installs a form through `gsx_mfset`, which brackets the
//! `vsc_form` with `gsx_moff` / `gsx_mon` (`references/emutos/aes/
//! gemgsxif.c:407-420`, `:440-455`): `v_hide_c` + `v_show_c(1)` around it,
//! so a shown pointer is redrawn at once in the new form. In NewTOS that is
//! `tos_api::vdi::mouse::set_form` / `set_preset` (`graf_mouse` USER_DEF and
//! the presets, `tos-api/src/aes/graf.rs`); the last test pins it.
//!
//! Expected values are the oracle's (`measurements/measurement-2026-11-07-
//! v18-t3/probe/maps/14-F1b.txt`, `17-F2b.txt`): F1b and F2b equal F1a / the
//! arrow before `vsc_form`, F1c/F2c the ground, F1d the new form.

#[path = "vdi_reference/harness.rs"]
mod harness;

use tos_api::bdos::process::Abi;
use tos_api::vdi::mouse::PRESET_FORMS;

/// A parameter block on the physical handle (the pointer is one per
/// machine; the handle only selects the workstation the fills draw on).
struct WPb {
    contrl: Box<[i16; 16]>,
    intin: Box<[i16; 64]>,
    ptsin: Box<[i16; 16]>,
    intout: Box<[i16; 64]>,
    ptsout: Box<[i16; 16]>,
    block: Box<[u64; 5]>,
}

impl WPb {
    fn new() -> WPb {
        let mut pb = WPb {
            contrl: Box::new([0; 16]),
            intin: Box::new([0; 64]),
            ptsin: Box::new([0; 16]),
            intout: Box::new([0; 64]),
            ptsout: Box::new([0; 16]),
            block: Box::new([0; 5]),
        };
        pb.block[0] = pb.contrl.as_ptr() as u64;
        pb.block[1] = pb.intin.as_ptr() as u64;
        pb.block[2] = pb.ptsin.as_ptr() as u64;
        pb.block[3] = pb.intout.as_ptr() as u64;
        pb.block[4] = pb.ptsout.as_ptr() as u64;
        pb
    }

    fn op(&mut self, opcode: i16, n_pts: i16, n_int: i16) {
        self.contrl.fill(0);
        self.contrl[0] = opcode;
        self.contrl[1] = n_pts;
        self.contrl[3] = n_int;
        self.contrl[6] = harness::workstation().phys;
        // SAFETY: the five arrays outlive the call and are large enough for
        // every opcode used here.
        unsafe {
            tos_api::vdi::dispatch(self.block.as_ptr() as u64, Abi::Lp64);
        }
    }
}

fn backend() -> &'static dyn tos_api::vdi::backend::RasterBackend {
    tos_api::host::host().unwrap().raster().unwrap()
}

fn show(flag: i16) {
    let mut pb = WPb::new();
    pb.intin[0] = flag;
    pb.op(122, 0, 1);
}

fn hide() {
    WPb::new().op(123, 0, 0);
}

/// `vsc_form` (111) with the 37-WORD MFORM.
fn vsc_form(hx: i16, hy: i16, bg: i16, fg: i16, mask: &[u16; 16], data: &[u16; 16]) {
    let mut pb = WPb::new();
    pb.intin[..5].copy_from_slice(&[hx, hy, 1, bg, fg]);
    for r in 0..16 {
        pb.intin[5 + r] = mask[r] as i16;
        pb.intin[21 + r] = data[r] as i16;
    }
    pb.op(111, 0, 37);
}

/// The probe's `arrow()`: the AES arrow, hotspot 0,0, mask pen 0, data pen 1.
fn arrow() {
    let (_, _, mask, data) = PRESET_FORMS[0];
    vsc_form(0, 0, 0, 1, &mask, &data);
}

/// The probe's block: full mask, checker data (`wkprobe.c` `build_forms`).
fn block() -> ([u16; 16], [u16; 16]) {
    let mask = [0xFFFF; 16];
    let data: [u16; 16] = core::array::from_fn(|i| if i & 1 != 0 { 0x5555 } else { 0xAAAA });
    (mask, data)
}

/// Put the pointer at `(x, y)` through the packet path (top-left clamp,
/// then out).
fn setpos(x: i16, y: i16) {
    tos_api::vdi::feed_relative(-4000, -4000, 0, backend());
    tos_api::vdi::feed_relative(x, y, 0, backend());
    assert_eq!(tos_api::vdi::mouse_position(), (x, y), "pointer at {x},{y}");
}

/// `vr_recfl` solid in `pen`.
fn recfl(pen: i16, x1: i16, y1: i16, x2: i16, y2: i16) {
    let mut pb = WPb::new();
    pb.intin[0] = 1;
    pb.op(23, 0, 1); // vsf_interior solid
    pb.intin[0] = pen;
    pb.op(25, 0, 1); // vsf_color
    pb.ptsin[..4].copy_from_slice(&[x1, y1, x2, y2]);
    pb.op(114, 2, 0); // vr_recfl
}

/// The probe's `pattern`: pen 6 ground, pen 2 / pen 4 stripes every 4 px.
fn pattern(x1: i16, y1: i16, x2: i16, y2: i16) {
    recfl(6, x1, y1, x2, y2);
    let mut x = x1;
    while x < x2 {
        recfl(if x & 4 != 0 { 2 } else { 4 }, x, y1, x + 1, y2);
        x += 4;
    }
}

/// The screen words of the rectangle `(x, y, w, h)`, row by row.
fn grab(x: i16, y: i16, w: i16, h: i16) -> Vec<u16> {
    let mut v = Vec::with_capacity(w as usize * h as usize);
    for yy in y..y + h {
        for xx in x..x + w {
            v.push(harness::screen_word(xx, yy));
        }
    }
    v
}

fn differing(a: &[u16], b: &[u16]) -> usize {
    assert_eq!(a.len(), b.len());
    a.iter().zip(b).filter(|(x, y)| x != y).count()
}

/// The device word a pen paints, read back from a 1-pixel `vr_recfl` far
/// from every case.
fn pen_word(pen: i16) -> u16 {
    recfl(pen, 630, 470, 630, 470);
    harness::screen_word(630, 470)
}

/// Whether the form `(hx, hy, mask, data)` in pens `bg`/`fg` stands with its
/// hotspot at `(x, y)`: every data pixel in `fg`, every mask-only pixel in
/// `bg`.
#[allow(clippy::too_many_arguments)]
fn form_at(x: i16, y: i16, hx: i16, hy: i16, mask: &[u16; 16], data: &[u16; 16], bg: i16, fg: i16) -> bool {
    let (fw, bw) = (pen_word(fg), pen_word(bg));
    for r in 0..16i16 {
        for c in 0..16i16 {
            let bit = 0x8000u16 >> c;
            let w = harness::screen_word(x - hx + c, y - hy + r);
            if data[r as usize] & bit != 0 && w != fw {
                return false;
            }
            if data[r as usize] & bit == 0 && mask[r as usize] & bit != 0 && w != bw {
                return false;
            }
        }
    }
    true
}

/// A known state: the physical workstation's pointer hidden once (the probe's
/// `norm`: `v_show_c(0)` + `v_hide_c`), the screen cleared.
fn norm() {
    let _ = harness::workstation();
    show(0);
    hide();
    WPb::new().op(3, 0, 0); // v_clrwk
}

/// F1: arrow shown at (110,260) on a pattern, then `vsc_form` (block, pens
/// 2/3). EmuTOS: F1b = F1a (the arrow stays), F1c the ground, F1d the block.
#[test]
fn f1_vsc_form_while_shown_keeps_the_old_image() {
    let _guard = harness::VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    norm();
    pattern(90, 240, 153, 303);
    let ground = grab(90, 240, 64, 64);
    arrow();
    setpos(110, 260);
    show(0);
    let f1a = grab(90, 240, 64, 64);
    let (am, ad) = (PRESET_FORMS[0].2, PRESET_FORMS[0].3);
    assert!(form_at(110, 260, 0, 0, &am, &ad, 0, 1), "F1a: the arrow is shown");

    let (bm, bd) = block();
    vsc_form(0, 0, 2, 3, &bm, &bd);
    let f1b = grab(90, 240, 64, 64);
    assert_eq!(
        differing(&f1a, &f1b),
        0,
        "F1b: set_mouse_form only stores (vdi_mouse.c:450-492) -- the arrow stays on the screen"
    );

    hide();
    assert_eq!(differing(&grab(90, 240, 64, 64), &ground), 0, "F1c: v_hide_c puts the ground back");

    show(0);
    assert!(form_at(110, 260, 0, 0, &bm, &bd, 2, 3), "F1d: the next show draws the block");
    assert_ne!(differing(&grab(90, 240, 64, 64), &f1a), 0, "F1d differs from F1a");
    hide();
    assert_eq!(differing(&grab(90, 240, 64, 64), &ground), 0, "the block's ground too");
}

/// F2: only the hotspot changes while shown (arrow 0,0 -> 8,8) at (300,260).
/// EmuTOS: F2b = the arrow where it was, F2c the ground; the next show draws
/// it 8 px up and left.
#[test]
fn f2_vsc_form_hotspot_while_shown_keeps_the_old_place() {
    let _guard = harness::VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    norm();
    pattern(280, 240, 343, 303);
    let ground = grab(280, 240, 64, 64);
    arrow();
    setpos(300, 260);
    show(0);
    let f2a = grab(280, 240, 64, 64);

    let (am, ad) = (PRESET_FORMS[0].2, PRESET_FORMS[0].3);
    vsc_form(8, 8, 0, 1, &am, &ad);
    assert_eq!(
        differing(&grab(280, 240, 64, 64), &f2a),
        0,
        "F2b: the arrow stays at the old place until the next redraw"
    );

    hide();
    assert_eq!(differing(&grab(280, 240, 64, 64), &ground), 0, "F2c: the ground");

    show(0);
    assert!(form_at(300, 260, 8, 8, &am, &ad, 0, 1), "the next show uses hotspot 8,8");
    hide();
    assert_eq!(differing(&grab(280, 240, 64, 64), &ground), 0, "and leaves no remnant");
}

/// The next redraw after a stored form is the pointer's move: the old image
/// comes off (its save-under is still the one taken when it was drawn), the
/// new form goes on at the new place. Afterwards nothing of either remains.
#[test]
fn packet_after_vsc_form_redraws_in_the_new_form_without_remnant() {
    let _guard = harness::VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    norm();
    pattern(380, 240, 459, 319);
    let ground = grab(380, 240, 80, 80);
    arrow();
    setpos(400, 260);
    show(0);
    let (bm, bd) = block();
    vsc_form(4, 4, 2, 3, &bm, &bd);

    tos_api::vdi::feed_relative(20, 10, 0, backend());
    assert_eq!(tos_api::vdi::mouse_position(), (420, 270));
    assert!(form_at(420, 270, 4, 4, &bm, &bd, 2, 3), "the block at the new place");
    let (am, ad) = (PRESET_FORMS[0].2, PRESET_FORMS[0].3);
    assert!(!form_at(400, 260, 0, 0, &am, &ad, 0, 1), "the arrow is gone from the old place");

    hide();
    assert_eq!(differing(&grab(380, 240, 80, 80), &ground), 0, "no remnant of the arrow or the block");
}

/// The AES path keeps its immediate redraw: `gsx_mfset` brackets the
/// `vsc_form` with `gsx_moff`/`gsx_mon` (`aes/gemgsxif.c:407-420`), i.e.
/// `v_hide_c` + `v_show_c(1)`. NewTOS: `set_form` (`graf_mouse` USER_DEF) and
/// `set_preset` (the presets) redraw a shown pointer at once.
#[test]
fn aes_form_install_while_shown_redraws_at_once() {
    let _guard = harness::VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    norm();
    pattern(90, 340, 153, 403);
    let ground = grab(90, 340, 64, 64);
    tos_api::vdi::mouse::set_preset(0, backend());
    setpos(110, 360);
    show(0);
    let (am, ad) = (PRESET_FORMS[0].2, PRESET_FORMS[0].3);
    assert!(form_at(110, 360, 0, 0, &am, &ad, 0, 1), "the arrow is shown");

    let (bm, bd) = block();
    tos_api::vdi::mouse::set_form(3, 3, &bm, &bd, backend());
    assert!(form_at(110, 360, 3, 3, &bm, &bd, 0, 1), "gsx_mfset: the block at once");

    let (hx, hy, hm, hd) = PRESET_FORMS[2];
    tos_api::vdi::mouse::set_preset(2, backend());
    assert!(form_at(110, 360, hx, hy, &hm, &hd, 0, 1), "a preset at once (graf_mouse HOURGLASS)");

    hide();
    assert_eq!(differing(&grab(90, 340, 64, 64), &ground), 0, "no remnant of any of the three");
    tos_api::vdi::mouse::set_preset(0, backend());
}
