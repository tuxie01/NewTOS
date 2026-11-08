//! V55 — a pointer that follows a mouse packet is drawn in the next frame,
//! not in the packet; `vq_mouse` reports the new place at once
//! (`TASK/V55.md`, audit VDI-WK-004, probe `tosdump/probes/wkprobe.c` cases
//! M1, M2).
//!
//! EmuTOS `mouse_int` (`references/emutos/vdi/vdi_asm.S:240-372`) stores the
//! clipped `GCURX`/`GCURY` and, only for a packet that carried motion, calls
//! `user_cur` = `mov_cur` (`:302-322`, `:367-368`), which stores
//! `newx`/`newy` and raises `draw_flag` unless the pointer is hidden
//! (`:496-532`). `vb_draw` (`vdi/vdi_mouse.c:668-686`, `vblqueue[0]`, `:602`)
//! redraws in the next VBL, at `newx`/`newy`, unless `mouse_flag` or
//! `HIDE_CNT` is set (`:673`). `dis_cur`/`hide_cur` draw and erase at once
//! and clear `draw_flag` (`:127-171`).
//!
//! NewTOS: `tos_api::vdi::feed_relative` (`mouse_int`), `vb_draw` — which the
//! kernel's frame tick calls (`kernel/src/vbl.rs`); here each test calls it
//! where a frame would begin. Expected values are the oracle's
//! (`measurements/measurement-2026-11-07-v18-t3/probe/maps/22-M1a.txt`,
//! `24-M2a.txt`): M1a and M2a equal the screen before the packet.

#[path = "vdi_reference/harness.rs"]
mod harness;

use tos_api::bdos::process::Abi;
use tos_api::vdi::mouse::PRESET_FORMS;

/// A parameter block on the physical handle.
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

/// `vq_mouse` (124): buttons, x, y.
fn vq_mouse() -> (i16, i16, i16) {
    let mut pb = WPb::new();
    pb.op(124, 0, 0);
    (pb.intout[0], pb.ptsout[0], pb.ptsout[1])
}

/// The probe's `arrow()`: `vsc_form` with the AES arrow, hotspot 0,0, mask
/// pen 0, data pen 1.
fn arrow() {
    let (_, _, mask, data) = PRESET_FORMS[0];
    let mut pb = WPb::new();
    pb.intin[..5].copy_from_slice(&[0, 0, 1, 0, 1]);
    for r in 0..16 {
        pb.intin[5 + r] = mask[r] as i16;
        pb.intin[21 + r] = data[r] as i16;
    }
    pb.op(111, 0, 37);
}

/// The probe's `setpos`: `vsin_mode(locator, sample)` + `v_locator`, which
/// sets `GCURX`/`GCURY` without drawing (`vdi/vdi_mouse.c:286-287`).
fn setpos(x: i16, y: i16) {
    let mut pb = WPb::new();
    pb.intin[0] = 1;
    pb.intin[1] = 2;
    pb.op(33, 0, 2);
    pb.ptsin[0] = x;
    pb.ptsin[1] = y;
    pb.op(28, 1, 0);
    assert_eq!(tos_api::vdi::mouse_position(), (x, y), "pointer at {x},{y}");
}

/// A packet: buttons in VDI convention (bit 0 left).
fn packet(dx: i16, dy: i16, buttons: u8) {
    tos_api::vdi::feed_relative(dx, dy, buttons, backend());
}

/// The frame tick: EmuTOS' VBL running `vb_draw`.
fn frame() {
    tos_api::vdi::vb_draw(backend());
}

/// `vr_recfl` solid in `pen` (the probe's `bar` draws the same with `v_bar`).
fn recfl(pen: i16, x1: i16, y1: i16, x2: i16, y2: i16) {
    let mut pb = WPb::new();
    pb.intin[0] = 1;
    pb.op(23, 0, 1); // vsf_interior solid
    pb.intin[0] = pen;
    pb.op(25, 0, 1); // vsf_color
    pb.ptsin[..4].copy_from_slice(&[x1, y1, x2, y2]);
    pb.op(114, 2, 0); // vr_recfl
}

/// Pen 6 ground with pen 2 / pen 4 stripes every 4 px, so a save-under that
/// comes back at the wrong place shows.
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

/// The device word a pen paints, read back from a 1-pixel fill far from
/// every case.
fn pen_word(pen: i16) -> u16 {
    recfl(pen, 630, 470, 630, 470);
    harness::screen_word(630, 470)
}

/// Whether the arrow stands with its hotspot at `(x, y)`: every data pixel in
/// pen 1, every mask-only pixel in pen 0.
fn arrow_at(x: i16, y: i16) -> bool {
    let (_, _, mask, data) = PRESET_FORMS[0];
    let (fw, bw) = (pen_word(1), pen_word(0));
    for r in 0..16i16 {
        for c in 0..16i16 {
            let bit = 0x8000u16 >> c;
            let w = harness::screen_word(x + c, y + r);
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

/// A known state: the pointer hidden once, no frame redraw pending, the
/// screen cleared (the probe's `norm`).
fn norm() {
    let _ = harness::workstation();
    show(0);
    hide();
    frame();
    WPb::new().op(3, 0, 0); // v_clrwk
    arrow();
}

/// M1: arrow shown at (200,350), packet +20,+8. Right after it `vq_mouse`
/// reports (220,358) and the screen still shows the arrow at (200,350)
/// (M1a); after the frame the arrow stands at (220,358) and the old place is
/// the ground again (M1b).
#[test]
fn m1_packet_while_shown_is_drawn_in_the_next_frame() {
    let _guard = harness::VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    norm();
    pattern(192, 344, 255, 383);
    let ground = grab(192, 344, 64, 40);
    setpos(200, 350);
    show(0);
    assert!(arrow_at(200, 350), "the arrow is shown");
    let before = grab(192, 344, 64, 40);

    packet(20, 8, 0);
    assert_eq!(vq_mouse(), (0, 220, 358), "vq_mouse reports the new place at once");
    assert_eq!(
        differing(&grab(192, 344, 64, 40), &before),
        0,
        "M1a: mov_cur only records the place (vdi_asm.S:496-532) -- the screen is unchanged"
    );

    frame();
    assert!(arrow_at(220, 358), "M1b: vb_draw draws the arrow at newx/newy");
    assert!(!arrow_at(200, 350), "M1b: and took it off the old place");
    frame();
    assert!(arrow_at(220, 358), "a second frame without a packet changes nothing");

    hide();
    assert_eq!(differing(&grab(192, 344, 64, 40), &ground), 0, "no remnant");
}

/// M2: a bar drawn over the shown arrow, then packet +30,0. Right after it
/// the bar is whole (M2a); in the next frame `cur_replace` puts the old
/// save-under back — the ground from before the bar, the "mouse crumb" — and
/// the arrow goes on at (380,350) over the bar (M2b).
#[test]
fn m2_bar_over_the_pointer_stays_whole_until_the_frame() {
    let _guard = harness::VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    norm();
    let ground = grab(336, 336, 96, 56);
    setpos(350, 350);
    show(0);
    recfl(2, 340, 340, 389, 389);
    let barred = grab(336, 336, 96, 56);

    packet(30, 0, 0);
    assert_eq!(vq_mouse(), (0, 380, 350));
    assert_eq!(differing(&grab(336, 336, 96, 56), &barred), 0, "M2a: the bar is whole, no hole, no new arrow");

    frame();
    assert!(arrow_at(380, 350), "M2b: the arrow over the bar");
    for r in 0..16i16 {
        for c in 0..16i16 {
            let (x, y) = (350 + c, 350 + r);
            let i = ((y - 336) * 96 + (x - 336)) as usize;
            assert_eq!(harness::screen_word(x, y), ground[i], "M2b: the old cell is the ground from before the bar at {x},{y}");
        }
    }
    hide();
}

/// Several packets before one frame: one redraw, at the last place; the
/// save-under taken there is the screen of that place.
#[test]
fn packets_in_one_frame_give_one_redraw_at_the_last_place() {
    let _guard = harness::VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    norm();
    pattern(88, 88, 167, 135);
    let ground = grab(88, 88, 80, 48);
    setpos(100, 100);
    show(0);
    let before = grab(88, 88, 80, 48);
    packet(10, 0, 0);
    packet(10, 4, 0);
    packet(10, 0, 0);
    assert_eq!(vq_mouse(), (0, 130, 104));
    assert_eq!(differing(&grab(88, 88, 80, 48), &before), 0, "nothing drawn before the frame");
    frame();
    assert!(arrow_at(130, 104), "drawn once, at the last place");
    hide();
    assert_eq!(differing(&grab(88, 88, 80, 48), &ground), 0, "no remnant of any intermediate place");
}

/// A packet while hidden records nothing (`mov_cur` tests `HIDE_CNT`); the
/// next `v_show_c` draws at `GCURX`/`GCURY` itself.
#[test]
fn packet_while_hidden_is_drawn_by_the_next_show() {
    let _guard = harness::VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    norm();
    let ground = grab(492, 344, 40, 40);
    setpos(500, 350);
    packet(10, 10, 0);
    frame();
    assert_eq!(differing(&grab(492, 344, 40, 40), &ground), 0, "M3a: nothing drawn while hidden");
    show(0);
    assert!(arrow_at(510, 360), "M3b: v_show_c draws at the new place");
    hide();
    assert_eq!(differing(&grab(492, 344, 40, 40), &ground), 0);
}

/// `hide_cur` and `dis_cur` drop a pending redraw (`draw_flag = 0`,
/// `vdi_mouse.c:139`, `:167`): a frame after them draws nothing more.
#[test]
fn hide_and_show_drop_a_pending_redraw() {
    let _guard = harness::VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    norm();
    pattern(88, 200, 167, 247);
    let ground = grab(88, 200, 80, 48);
    setpos(100, 210);
    show(0);
    packet(20, 0, 0);
    hide();
    assert_eq!(differing(&grab(88, 200, 80, 48), &ground), 0, "v_hide_c takes the arrow off its drawn place");
    frame();
    assert_eq!(differing(&grab(88, 200, 80, 48), &ground), 0, "no frame redraw while hidden");
    show(0);
    assert!(arrow_at(120, 210), "v_show_c draws at GCURX/GCURY");
    let shown = grab(88, 200, 80, 48);
    frame();
    assert_eq!(differing(&grab(88, 200, 80, 48), &shown), 0, "dis_cur dropped the pending redraw");
    hide();
    assert_eq!(differing(&grab(88, 200, 80, 48), &ground), 0, "no remnant");
}

/// A packet without motion (a button change) calls no `mov_cur`
/// (`vdi_asm.S:302-311`): nothing is redrawn, so a bar over the pointer keeps
/// no hole; `vq_mouse` reports the buttons at once.
#[test]
fn button_packet_redraws_nothing() {
    let _guard = harness::VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    norm();
    setpos(450, 300);
    show(0);
    recfl(2, 440, 290, 489, 339);
    let barred = grab(436, 286, 64, 64);
    packet(0, 0, 1);
    assert_eq!(vq_mouse(), (1, 450, 300), "the button at once");
    assert_eq!(differing(&grab(436, 286, 64, 64), &barred), 0, "no redraw in the packet");
    frame();
    assert_eq!(differing(&grab(436, 286, 64, 64), &barred), 0, "and none in the frame");
    packet(0, 0, 0);
    frame();
    assert_eq!(vq_mouse(), (0, 450, 300));
    assert_eq!(differing(&grab(436, 286, 64, 64), &barred), 0, "the release neither");
    hide();
}
