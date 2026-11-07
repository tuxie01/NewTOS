//! V53 — what `v_opnwk` resets and what `v_clswk` switches off in the VDI
//! mouse (`TASK/V53.md`, audit VDI-WK-001/-002, probe `tosdump/probes/wkprobe.c`
//! cases W1, W2, C1).
//!
//! EmuTOS runs `timer_init` and `vdimouse_init` on **every** `v_opnwk`
//! (`references/emutos/vdi/vdi_control.c:562-563`):
//!
//! * `tim_addr = just_rts` (`vdi/vdi_misc.c:128`);
//! * `HIDE_CNT = 1` — without `cur_replace`, so a pointer that was visible
//!   leaves no save-under to restore (`vdi/vdi_mouse.c:581`);
//! * `GCURX`/`GCURY` = the centre (`:582-583`);
//! * `user_but = user_mot = just_rts`, `user_cur = mov_cur`,
//!   `user_wheel = just_rts` (`:585-589`);
//! * the default arrow form (`:593`), `MOUSE_BT = 0`, `cur_ms_stat = 0`
//!   (`:595-596`);
//! * `Initmous(1, …, mouse_int)`: packets reach `mouse_int` (`:605`).
//!
//! `v_clswk` runs `timer_exit` and `vdimouse_exit` (`vdi_control.c:587-588`):
//! `user_but = user_mot = user_cur = user_wheel = just_rts` (`vdi_mouse.c:
//! 624-629`) and `Initmous(0, 0, 0)` (`:634`), which sets the BIOS
//! `mousevec` to `just_rts` (`bios/mouse.c:47-48,119-121`): no packet moves
//! the pointer until the next `v_opnwk`. `tim_addr`, the hide count and the
//! extended-button vector `mousexvec` (`vdi_mouse.c:612`, never reset) stay.
//!
//! The expected values are the oracle's (`measurements/measurement-2026-11-07-
//! v18-t3/probe/e1/WKPROBE.LOG`): W1 `vq_mouse 0 319 239` and every `vex_*`
//! back at its default, W1b/W1c the arrow at the centre, W2b no remnant, C1
//! `vq_mouse` unchanged by a packet and the four mouse vectors `just_rts`.
//!
//! Everything runs on the physical handle (1), which `v_clswk` leaves open;
//! the harness' virtual workstation is closed by the first `v_clswk` and not
//! used here.

#[path = "vdi_reference/harness.rs"]
mod harness;

use tos_api::bdos::process::Abi;
use tos_api::bios::vectors::mousexvec;
use tos_api::vdi::mouse::PRESET_FORMS;

const PHYS: i16 = 1;

/// A parameter block whose `CONTRL` reaches LP64 pointer slot 1 (words
/// 11..14): the `vex_*` calls hand the old vector back there
/// (`sdk/gemlib/gemlib-0.44.0/gem_vdiP.h:177,248`); the harness block stops
/// at 12 words.
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
        self.contrl[6] = PHYS;
        // SAFETY: the five arrays outlive the call and are large enough for
        // every opcode used here (v_opnwk: 45 + 12 words out).
        unsafe {
            tos_api::vdi::dispatch(self.block.as_ptr() as u64, Abi::Lp64);
        }
    }
}

fn backend() -> &'static dyn tos_api::vdi::backend::RasterBackend {
    tos_api::host::host().unwrap().raster().unwrap()
}

/// `v_opnwk` with the AES's `work_in` (`aes/gemgsxif.c:180-184`, no mode
/// request).
fn opnwk() {
    let mut pb = WPb::new();
    for i in 0..10 {
        pb.intin[i] = 1;
    }
    pb.intin[10] = 2;
    pb.op(1, 0, 11);
    assert_eq!(pb.contrl[6], PHYS, "v_opnwk returns the physical handle");
}

fn clswk() {
    WPb::new().op(2, 0, 0);
}

fn show(flag: i16) {
    let mut pb = WPb::new();
    pb.intin[0] = flag;
    pb.op(122, 0, 1);
}

fn hide() {
    WPb::new().op(123, 0, 0);
}

fn vq_mouse() -> (i16, i16, i16) {
    let mut pb = WPb::new();
    pb.op(124, 0, 0);
    (pb.intout[0], pb.ptsout[0], pb.ptsout[1])
}

/// `vex_*(new)` → the previous vector, read from LP64 pointer slot 1.
fn vex(op: i16, new: u64) -> u64 {
    let mut pb = WPb::new();
    pb.contrl.fill(0);
    pb.contrl[0] = op;
    pb.contrl[6] = PHYS;
    // SAFETY: CONTRL words 7..10 (slot 0) and 11..14 (slot 1) lie inside the
    // 16-word array; the slots are WORD-aligned only.
    unsafe {
        (pb.contrl[7..].as_mut_ptr() as *mut u64).write_unaligned(new);
        tos_api::vdi::dispatch(pb.block.as_ptr() as u64, Abi::Lp64);
        (pb.contrl[11..].as_ptr() as *const u64).read_unaligned()
    }
}

const VEX_BUTV: i16 = 125;
const VEX_MOTV: i16 = 126;
const VEX_CURV: i16 = 127;
const VEX_TIMV: i16 = 118;
const VEX_WHEELV: i16 = 134;
/// The order of the probe's `vex_ops` (`tosdump/probes/wkprobe.c`).
const VEX_OPS: [i16; 5] = [VEX_BUTV, VEX_MOTV, VEX_CURV, VEX_TIMV, VEX_WHEELV];

/// A routine address no default can be: the probe's own `xnop`.
const OWN: u64 = 0x0000_0000_0042_1230;

/// The 16×16 cross the probe installs (`wkprobe.c` `cross_mask`/`cross_data`
/// shape: hotspot 5,5, mask pen 4, data pen 1).
fn cross_form() {
    let mut pb = WPb::new();
    pb.intin[0] = 5;
    pb.intin[1] = 5;
    pb.intin[2] = 1;
    pb.intin[3] = 4;
    pb.intin[4] = 1;
    for r in 0..16 {
        pb.intin[5 + r] = if (4..=6).contains(&r) { 0xFFFFu16 as i16 } else { 0x0E00 };
        pb.intin[21 + r] = if r == 5 { 0x7FFE } else { 0x0400 };
    }
    pb.op(111, 0, 37);
}

/// Put the pointer at `(x, y)` through the packet path: walk into the
/// top-left clamp, then out.
fn setpos(x: i16, y: i16) {
    tos_api::vdi::feed_relative(-4000, -4000, 0, backend());
    tos_api::vdi::feed_relative(x, y, 0, backend());
    assert_eq!(tos_api::vdi::mouse_position(), (x, y), "pointer at {x},{y}");
}

/// A solid pen-2 box `(x1, y1)-(x2, y2)` (`vsf_interior` solid, `vsf_color`
/// 2, `vr_recfl`) — a ground the pointer's save-under is taken from.
fn box_fill(x1: i16, y1: i16, x2: i16, y2: i16) {
    let mut pb = WPb::new();
    pb.intin[0] = 1;
    pb.op(23, 0, 1); // vsf_interior solid
    pb.intin[0] = 2;
    pb.op(25, 0, 1); // vsf_color 2
    pb.ptsin[..4].copy_from_slice(&[x1, y1, x2, y2]);
    pb.op(114, 2, 0); // vr_recfl
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

/// `a == b` word for word; on failure the count of differing words, not the
/// two whole rectangles.
fn assert_same(a: &[u16], b: &[u16], what: &str) {
    let n = a.iter().zip(b).filter(|(x, y)| x != y).count();
    assert!(n == 0 && a.len() == b.len(), "{what}: {n} words differ");
}

/// The device word a pen paints, read back from a 1-pixel `vr_recfl`.
fn pen_word(pen: i16) -> u16 {
    let mut pb = WPb::new();
    pb.intin[0] = 1;
    pb.op(23, 0, 1);
    pb.intin[0] = pen;
    pb.op(25, 0, 1);
    pb.ptsin[..4].copy_from_slice(&[630, 470, 630, 470]);
    pb.op(114, 2, 0);
    harness::screen_word(630, 470)
}

/// A clean starting point: the physical workstation closed and opened again,
/// the AES's sequence at a session start.
fn reopen() {
    let _ = harness::workstation();
    clswk();
    opnwk();
}

/// Whether the 16×16 default arrow (`PRESET_FORMS[0]`, hotspot 0,0) stands
/// with its hotspot at `(x, y)`: every data pixel in pen 1, every mask-only
/// pixel in pen 0.
fn arrow_at(x: i16, y: i16) -> bool {
    let (hx, hy, mask, data) = PRESET_FORMS[0];
    let (fg, bg) = (pen_word(1), pen_word(0));
    for r in 0..16i16 {
        for c in 0..16i16 {
            let bit = 0x8000u16 >> c;
            let w = harness::screen_word(x - hx + c, y - hy + r);
            if data[r as usize] & bit != 0 && w != fg {
                return false;
            }
            if data[r as usize] & bit == 0 && mask[r as usize] & bit != 0 && w != bg {
                return false;
            }
        }
    }
    true
}

/// W1: cross at (100,100), hide count 3, own vectors in all five slots, left
/// button down — then `v_clswk` + `v_opnwk`. EmuTOS: `vq_mouse 0 319 239`,
/// every vector back at its default.
#[test]
fn w1_v_opnwk_resets_position_buttons_and_vectors() {
    let _guard = harness::VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reopen();
    cross_form();
    setpos(100, 100);
    hide();
    hide(); // hide count 3 (1 after the open, +2)
    for op in VEX_OPS {
        vex(op, OWN);
    }
    tos_api::vdi::feed_relative(0, 0, 0x01, backend()); // left button down
    assert_eq!(vq_mouse(), (1, 100, 100), "the state before the reopen");

    clswk();
    opnwk();

    assert_eq!(vq_mouse(), (0, 319, 239), "v_opnwk: buttons 0, the centre (vdi_mouse.c:582-583,595)");
    let mut old = [0u64; 5];
    for (i, op) in VEX_OPS.iter().enumerate() {
        old[i] = vex(*op, OWN);
        assert_ne!(old[i], OWN, "vex {op}: v_opnwk puts the default back (vdi_mouse.c:585-589, vdi_misc.c:128)");
        vex(*op, old[i]);
    }
    // just_rts in butv/motv/timv/wheelv, mov_cur in curv.
    assert_eq!(old[0], old[1], "user_but and user_mot share just_rts");
    assert_eq!(old[0], old[3], "tim_addr is just_rts too");
    assert_eq!(old[0], old[4], "user_wheel is just_rts too");
    assert_ne!(old[2], old[0], "user_cur is mov_cur, not just_rts");
    tos_api::vdi::feed_relative(0, 0, 0, backend());
}

/// W1b/W1c: after the reopen the hide count is 1 — one counted `v_show_c`
/// shows the pointer — and the form is the default arrow at the centre.
#[test]
fn w1bc_v_opnwk_resets_hide_count_and_form() {
    let _guard = harness::VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reopen();
    cross_form();
    setpos(100, 100);
    hide();
    hide(); // hide count 3
    clswk();
    opnwk();

    let before = grab(80, 80, 256, 176);
    show(1); // W1b: count 1 -> 0 on EmuTOS
    assert!(arrow_at(319, 239), "W1b: one v_show_c(1) shows the arrow at the centre");
    hide();
    assert_same(&grab(80, 80, 256, 176), &before, "v_hide_c puts the ground back");
    show(0); // W1c
    assert!(arrow_at(319, 239), "W1c: the default arrow, not the cross");
    hide();
    assert_same(&grab(80, 80, 256, 176), &before, "nothing of the cross anywhere");
}

/// W2: the cross shown at (100,100) over a box, then `v_clswk` + `v_opnwk`
/// (both clear the screen). EmuTOS sets `HIDE_CNT = 1` without
/// `cur_replace`: the next `v_hide_c` restores nothing.
#[test]
fn w2_v_opnwk_drops_a_visible_pointer_without_restoring_it() {
    let _guard = harness::VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reopen();
    box_fill(90, 90, 149, 149);
    cross_form();
    setpos(100, 100);
    show(0);
    clswk();
    opnwk();

    let after_open = grab(80, 80, 256, 176);
    hide(); // W2b
    assert_same(&grab(80, 80, 256, 176), &after_open, "W2b: no 16x16 remnant of the box");
    show(0); // W2c
    assert!(arrow_at(319, 239), "W2c: the arrow at the centre");
    hide();
}

/// C1: the arrow shown at (150,150) over a box, own butv/motv/timv/wheelv,
/// then `v_clswk`. EmuTOS: the four mouse vectors are `just_rts` (`curv`
/// too), `tim_addr` stays, and a packet neither moves nor draws the pointer.
#[test]
fn c1_v_clswk_switches_the_mouse_off() {
    let _guard = harness::VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reopen();
    box_fill(130, 130, 193, 193);
    let (hx, hy, mask, data) = PRESET_FORMS[0];
    let mut pb = WPb::new();
    pb.intin[..5].copy_from_slice(&[hx, hy, 1, 0, 1]);
    for r in 0..16 {
        pb.intin[5 + r] = mask[r] as i16;
        pb.intin[21 + r] = data[r] as i16;
    }
    pb.op(111, 0, 37); // the arrow
    setpos(150, 150);
    show(0);
    for op in VEX_OPS {
        if op != VEX_CURV {
            vex(op, OWN);
        }
    }
    clswk();

    let c_cur = vex(VEX_CURV, 0);
    let c_but = vex(VEX_BUTV, 0);
    let c_mot = vex(VEX_MOTV, 0);
    let c_wheel = vex(VEX_WHEELV, 0);
    let c_tim = vex(VEX_TIMV, 0);
    assert_eq!(c_tim, OWN, "timer_exit leaves tim_addr standing");
    assert_ne!(c_but, OWN, "vdimouse_exit: user_but = just_rts (vdi_mouse.c:624)");
    assert_eq!(c_mot, c_but, "user_mot = just_rts (:625)");
    assert_eq!(c_cur, c_but, "user_cur = just_rts, not mov_cur (:626)");
    assert_eq!(c_wheel, c_but, "user_wheel = just_rts (:628)");
    for (op, v) in [(VEX_CURV, c_cur), (VEX_BUTV, c_but), (VEX_MOTV, c_mot), (VEX_WHEELV, c_wheel), (VEX_TIMV, c_tim)] {
        vex(op, v);
    }

    let screen = grab(120, 120, 128, 128);
    tos_api::vdi::feed_relative(40, 20, 0, backend()); // C1b
    assert_eq!(vq_mouse(), (0, 150, 150), "Initmous(0): the packet does not reach mouse_int");
    assert_same(&grab(120, 120, 128, 128), &screen, "C1b/C1c: nothing drawn");
    tos_api::vdi::feed_relative(0, 0, 0x01, backend());
    assert_eq!(vq_mouse().0, 0, "no button either");

    // The extended buttons come through `mousexvec`, which vdimouse_exit
    // leaves installed (vdi_mouse.c:612, :622-645): MOUSE_BT still follows.
    mousexvec(0x37);
    assert_eq!(vq_mouse().0 & 0x04, 0x04, "button 3 through mousexvec after v_clswk");
    mousexvec(0xB7);
    assert_eq!(vq_mouse().0 & 0x04, 0, "button 3 released");

    hide(); // C1d: the pointer was still shown; its ground comes back
    assert!(!arrow_at(150, 150), "C1d: v_hide_c erases the pointer");

    // The next v_opnwk switches the mouse on again.
    opnwk();
    tos_api::vdi::feed_relative(5, 5, 0, backend());
    assert_eq!(vq_mouse(), (0, 324, 244), "after v_opnwk packets move the pointer again");
}

/// `v_opnvwk` runs `init_wk`, not `vdimouse_init` (`vdi_control.c:407-460`):
/// opening a virtual workstation leaves the pointer alone.
#[test]
fn v_opnvwk_leaves_the_mouse_alone() {
    let _guard = harness::VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    reopen();
    setpos(100, 120);
    hide();
    hide(); // hide count 3
    let mut pb = WPb::new();
    for i in 0..10 {
        pb.intin[i] = 1;
    }
    pb.intin[10] = 2;
    pb.op(100, 0, 11);
    let h = pb.contrl[6];
    assert!(h > PHYS, "v_opnvwk opened a virtual workstation ({h})");
    assert_eq!(vq_mouse(), (0, 100, 120), "the position stays");
    show(1);
    show(1);
    assert!(!arrow_at(100, 120), "the hide count stays: two shows leave it at 1");
    show(1);
    assert!(arrow_at(100, 120), "the third show uncovers it");
    hide();
}
