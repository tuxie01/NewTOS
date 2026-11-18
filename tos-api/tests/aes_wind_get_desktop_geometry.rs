//! `wind_get(0, WF_WXYWH / WF_CXYWH / WF_PXYWH / WF_FXYWH)` — the desktop
//! window's four rectangles are a function of the **device**, not constants.
//!
//! EmuTOS derives them once the resolution is known, in `gsx_start`
//! (`references/emutos/aes/gemgraf.c:423-424`, pinned tree
//! `1a7484aba1bfba23e2c0899e243b6d8177fe41df`):
//!
//! ```c
//! r_set(&gl_rscreen, 0, 0, gl_width, gl_height);
//! r_set(&gl_rfull, 0, gl_hbox, gl_width, (gl_height - gl_hbox));
//! ```
//!
//! and `wm_init` copies them into the desktop window, handle 0 (`DESKWH`),
//! (`aes/gemwmlib.c:1290-1294`):
//!
//! ```c
//! w_setup(ppd, DESKWH, NONE);
//! w_setsize(WS_CURR, DESKWH, &gl_rscreen);
//! w_setsize(WS_PREV, DESKWH, &gl_rscreen);
//! w_setsize(WS_FULL, DESKWH, &gl_rfull);
//! w_setsize(WS_WORK, DESKWH, &gl_rfull);
//! ```
//!
//! `wm_get` answers the four fields with a plain `w_getsize` of those slots
//! (`:1528-1541`, `:236-262`) — nothing is recomputed on the way out. So on
//! **any** device the desktop window answers:
//!
//! | field | rect | EmuTOS |
//! |---|---|---|
//! | `WF_WXYWH` (4) | `(0, hbox, W, H − hbox)` | `WS_WORK = gl_rfull` |
//! | `WF_CXYWH` (5) | `(0, 0, W, H)` | `WS_CURR = gl_rscreen` |
//! | `WF_PXYWH` (6) | `(0, 0, W, H)` | `WS_PREV = gl_rscreen` |
//! | `WF_FXYWH` (7) | `(0, hbox, W, H − hbox)` | `WS_FULL = gl_rfull` |
//!
//! where `W`/`H` are the workstation's pixel extent (`gl_width`/`gl_height`,
//! `gemgraf.c:393-394`) and `hbox` is `gl_hbox`, the gadget box height that is
//! also the menu-bar row (`gl_rmenu`, `gemgraf.c:427`). `gl_hbox` is not a
//! private number: `graf_handle` reports it in `int_out[4]`
//! (`aes/gemsuper.c:287-291`), and that is where this test takes it from — the
//! AES's own reading, so the rule holds whatever `gl_hbox` is derived from
//! (today a constant, TASK/A20; after A20 the font metric — the test does not
//! care, it only requires the two API outputs to agree the way the one
//! variable in EmuTOS makes them agree).
//!
//! **Why this exists next to the reference suite.** `tests/aes_reference/`
//! replays measured cases 155–158 of `tosdump/cases/aes-r1.tsv` against the
//! TT dump — at the TT's 640×480 only, and as a number-for-number comparison.
//! That proves the values at one profile; it cannot tell "derived from the
//! device" from "the TT's numbers, hard-coded". The Pi 4 run
//! (`measurements/measurement-2026-09-09/run1-pi4/AES.LOG`) answered 1920/1080 and
//! 1061 for the same cases, which the tickets classify as a device difference,
//! not a defect — provided the rule holds. This file checks the *rule*, at two
//! profiles (the TT's 640×480 and the Pi 4's 1920×1080), and checks that the
//! check itself is not vacuous: a deliberately wrong expectation is reported.
//!
//! **What the rig is.** A metadata-only raster backend whose width/height are
//! switchable at run time; nothing here draws, and the backend panics if
//! anything tries. After switching the profile the test runs
//! `tos_api::aes::window::wm_init()` — the same entry the kernel's shell loop
//! calls before each program (`kernel/src/shell_loop.rs:84`, EmuTOS
//! `gemshlib.c:717`) — which is what seeds the desktop window from the live
//! screen. EmuTOS never changes resolution at run time, so there is no trap
//! that re-seeds it; calling the AES start entry is the honest equivalent.
//!
//! Tickets: `TASK/A01.md`, `TASK/A02.md`, `TASK/A03.md`, `TASK/A04.md`.

#![allow(clippy::too_many_arguments, unused_variables)]

use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::{Mutex, Once};

use tos_api::bdos::process::Abi;
use tos_api::host::Host;
use tos_api::vdi::backend::{ClipRect, MonoSrc, PatternFill, Pel, RasterBackend, Rect, WriteMode};

// ── the two profiles ─────────────────────────────────────────────────────

/// The geometry the reference dump was measured at — the Atari TT in
/// TT-medium (`tosdump/reference/EMUTOS14-AES.BIN`).
const TT: (u16, u16) = (640, 480);
/// The current hardware profile — the Pi 4 run
/// (`measurements/measurement-2026-09-09/run1-pi4/`). A test profile, not a new video mode:
/// the backend below only *reports* it.
const PI4: (u16, u16) = (1920, 1080);

// ── the switchable metadata-only backend ─────────────────────────────────

struct Profile {
    width: AtomicU16,
    height: AtomicU16,
}

static PROFILE: Profile = Profile { width: AtomicU16::new(TT.0), height: AtomicU16::new(TT.1) };

/// Set while the fixture runs `gsx_init`: its `v_opnwk` clears the VT52
/// console (`esc_init`, `vdi_control.c:564`, CON04), the one drawing the
/// fixture itself does — screen words through the row interface and their
/// flush. Every other moment still panics on any drawing.
static CONSOLE_SETUP: AtomicBool = AtomicBool::new(false);

fn console_setup() -> bool {
    CONSOLE_SETUP.load(Ordering::Relaxed)
}

impl RasterBackend for Profile {
    fn width(&self) -> u16 {
        self.width.load(Ordering::Relaxed)
    }
    fn height(&self) -> u16 {
        self.height.load(Ordering::Relaxed)
    }
    fn planes(&self) -> u16 {
        4
    }
    fn num_colors(&self) -> u16 {
        16
    }
    fn set_color(&self, i: u16, r: u16, g: u16, b: u16) {
        panic!("unexpected palette write")
    }
    fn query_color(&self, i: u16) -> (u16, u16, u16) {
        panic!("unexpected palette read")
    }
    fn set_active_palette(&self, _p: &[u32; 256]) {
        // Every VDI call on an open workstation selects its pseudo-palette
        // (`tos-api/src/vdi/mod.rs`); the AES's own `gsx_init` set-up calls
        // (`gemgraf.c:405-422`) are such calls. Nothing is drawn with it.
    }
    fn pel_to_device16(&self, p: Pel) -> u16 {
        panic!("unexpected pixel conversion")
    }
    fn device_to_rgb565(&self, p: u32) -> u16 {
        if console_setup() {
            return p as u16;
        }
        panic!("unexpected pixel conversion")
    }
    fn rgb565_to_device(&self, p: u16) -> u32 {
        if console_setup() {
            return p as u32;
        }
        panic!("unexpected pixel conversion")
    }
    fn read_pixel(&self, x: i16, y: i16) -> Pel {
        panic!("unexpected pixel read")
    }
    fn fill_rect(&self, x: i16, y: i16, w: i16, h: i16, p: Pel, c: ClipRect) {
        panic!("unexpected drawing")
    }
    fn abline(
        &self,
        x: i16,
        y: i16,
        xx: i16,
        yy: i16,
        p: Pel,
        m: WriteMode,
        c: ClipRect,
        mask: u16,
        last: bool,
    ) -> u16 {
        panic!("unexpected drawing")
    }
    fn fill_pattern(&self, x: i16, y: i16, w: i16, h: i16, p: PatternFill, c: ClipRect) {
        panic!("unexpected drawing")
    }
    fn draw_glyph_sized(
        &self,
        x: i16,
        y: i16,
        g: &[u8],
        w: i16,
        h: i16,
        p: Pel,
        m: WriteMode,
        c: ClipRect,
    ) {
        panic!("unexpected drawing")
    }
    fn blit_mono(&self, d: Rect, s: MonoSrc, f: Pel, b: Pel, m: WriteMode, c: ClipRect) {
        panic!("unexpected drawing")
    }
    fn copy_rect(&self, s: Rect, x: i16, y: i16, op: u8, c: ClipRect) {
        panic!("unexpected drawing")
    }
    fn get_device_pixel(&self, x: i16, y: i16) -> u32 {
        if console_setup() {
            return 0;
        }
        panic!("unexpected pixel read")
    }
    fn put_device_pixel(&self, x: i16, y: i16, p: u32) {
        if console_setup() {
            return;
        }
        panic!("unexpected drawing")
    }
    fn flush_rect(&self, x: i16, y: i16, xx: i16, yy: i16) {
        if console_setup() {
            return;
        }
        panic!("unexpected flush")
    }
}

struct TestHost;

impl Host for TestHost {
    fn console_write_byte(&self, b: u8) {
        panic!("unexpected console output")
    }
    fn raster(&self) -> Option<&dyn RasterBackend> {
        Some(&PROFILE)
    }
}

static HOST: TestHost = TestHost;
static HOST_REF: &dyn Host = &HOST;

/// The AES window table and the installed host are process-wide; the tests of
/// this binary run one at a time.
static AES_LOCK: Mutex<()> = Mutex::new(());
static INSTALL: Once = Once::new();

// ── the calls ────────────────────────────────────────────────────────────

const OP_APPL_INIT: i16 = 10;
const OP_GRAF_HANDLE: i16 = 77;
const OP_WIND_GET: i16 = 104;

const WF_WXYWH: i16 = 4;
const WF_CXYWH: i16 = 5;
const WF_PXYWH: i16 = 6;
const WF_FXYWH: i16 = 7;

/// The desktop window, EmuTOS `DESKWH`.
const DESKWH: i16 = 0;

const POISON: i16 = 0x5a3c;

struct Call {
    global: [i16; 15],
    input: [i16; 16],
    output: [i16; 7],
    addrin: [u64; 3],
    addrout: [u64; 1],
}

impl Call {
    fn new() -> Self {
        Self {
            global: [POISON; 15],
            input: [POISON; 16],
            output: [POISON; 7],
            addrin: [0; 3],
            addrout: [0; 1],
        }
    }

    fn invoke(&mut self, op: i16, ni: i16, no: i16) -> i16 {
        let mut control = [op, ni, no, 0, 0];
        let raw = [
            control.as_mut_ptr() as u64,
            self.global.as_mut_ptr() as u64,
            self.input.as_mut_ptr() as u64,
            self.output.as_mut_ptr() as u64,
            self.addrin.as_mut_ptr() as u64,
            self.addrout.as_mut_ptr() as u64,
        ];
        // SAFETY: all six arrays are live, LP64, and cover the declared lengths.
        assert!(unsafe { tos_api::aes::dispatch(raw.as_ptr() as u64, Abi::Lp64) }.is_none());
        self.output[0]
    }
}

type R = (i16, i16, i16, i16);

/// `wind_get(handle, field)` → the rect in `int_out[1..4]`; `int_out[0]` must
/// be TRUE — handle 0 is a real window (`wm_init`, `gemwmlib.c:1290`).
fn wind_get_rect(handle: i16, field: i16) -> R {
    let mut c = Call::new();
    c.input[0] = handle;
    c.input[1] = field;
    let ok = c.invoke(OP_WIND_GET, 2, 5);
    assert_eq!(ok, 1, "wind_get({handle}, {field}) returned FALSE");
    (c.output[1], c.output[2], c.output[3], c.output[4])
}

/// `graf_handle` → `gl_hbox`, `int_out[4]` (`aes/gemsuper.c:291`).
fn gl_hbox() -> i16 {
    let mut c = Call::new();
    let handle = c.invoke(OP_GRAF_HANDLE, 0, 5);
    assert!(handle > 0, "graf_handle returned no handle ({handle})");
    c.output[4]
}

// ── the rule ─────────────────────────────────────────────────────────────

/// The four desktop-window rectangles as `wind_get(0, …)` answers them.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
struct Desk {
    work: R,
    curr: R,
    prev: R,
    full: R,
}

/// The rule: `gl_rscreen`/`gl_rfull` (`gemgraf.c:423-424`) as `wm_init`
/// stores them in handle 0 (`gemwmlib.c:1291-1294`).
fn expected(w: i16, h: i16, hbox: i16) -> Desk {
    let rscreen = (0, 0, w, h);
    let rfull = (0, hbox, w, h - hbox);
    Desk { work: rfull, curr: rscreen, prev: rscreen, full: rfull }
}

/// Which of the four fields disagree with the rule, by name. Empty = the rule
/// holds. A list rather than a bool, so the negative tests can show that each
/// field is compared on its own and that a wrong `hbox` reaches exactly the two
/// `gl_rfull` fields.
fn mismatches(got: Desk, want: Desk) -> Vec<&'static str> {
    let mut bad = Vec::new();
    for (name, g, w) in [
        ("WF_WXYWH", got.work, want.work),
        ("WF_CXYWH", got.curr, want.curr),
        ("WF_PXYWH", got.prev, want.prev),
        ("WF_FXYWH", got.full, want.full),
    ] {
        if g != w {
            bad.push(name);
        }
    }
    bad
}

// ── the rig ──────────────────────────────────────────────────────────────

/// Start the AES on a screen of `w`×`h`: switch the backend, then run the
/// window manager's start entry so handle 0 is seeded from the live screen
/// (`gsx_start` → `wm_init`; `kernel/src/shell_loop.rs:84`). Returns the four
/// rectangles handle 0 answers and the `gl_hbox` `graf_handle` reports.
fn start_at(w: u16, h: u16) -> (Desk, i16) {
    INSTALL.call_once(|| {
        tos_api::host::install_host(&HOST_REF);
        let mut c = Call::new();
        let ap_id = c.invoke(OP_APPL_INIT, 0, 1);
        assert!(ap_id >= 0, "appl_init failed ({ap_id})");
    });
    PROFILE.width.store(w, Ordering::Relaxed);
    PROFILE.height.store(h, Ordering::Relaxed);
    // The session's workstation open (`gsx_init`, `geminit.c:675`), whose
    // handle `graf_handle` reports — on the screen just switched to.
    CONSOLE_SETUP.store(true, Ordering::Relaxed);
    tos_api::aes::gsxif::gsx_init();
    CONSOLE_SETUP.store(false, Ordering::Relaxed);
    tos_api::aes::window::wm_init();

    let hbox = gl_hbox();
    // The rule would be satisfied trivially by `hbox == 0` (`gl_rfull` would
    // equal `gl_rscreen`) — the menu row is a real, positive strip that leaves
    // room below it (`gl_rmenu`, `gemgraf.c:427`).
    assert!(hbox > 0 && (hbox as i32) < h as i32, "gl_hbox {hbox} is not a menu-bar height on a {w}x{h} screen");

    let desk = Desk {
        work: wind_get_rect(DESKWH, WF_WXYWH),
        curr: wind_get_rect(DESKWH, WF_CXYWH),
        prev: wind_get_rect(DESKWH, WF_PXYWH),
        full: wind_get_rect(DESKWH, WF_FXYWH),
    };
    (desk, hbox)
}

fn lock() -> std::sync::MutexGuard<'static, ()> {
    AES_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// One field, both profiles: the rect handle 0 answers is the one the rule
/// derives from the backend's own width/height and the AES's own `gl_hbox`.
fn check_field(name: &str, pick: fn(Desk) -> R) {
    let _g = lock();
    for (w, h) in [TT, PI4] {
        let (got, hbox) = start_at(w, h);
        let want = expected(w as i16, h as i16, hbox);
        assert_eq!(
            pick(got),
            pick(want),
            "wind_get(0, {name}) at {w}x{h} (gl_hbox {hbox}): got {:?}, the rule says {:?} \
             (gemgraf.c:423-424, gemwmlib.c:1291-1294)",
            pick(got),
            pick(want)
        );
    }
}

// ── A01–A04: the rule holds at both profiles ─────────────────────────────

/// A01 — `WF_WXYWH` on handle 0 is `gl_rfull`: `(0, hbox, W, H − hbox)`.
#[test]
fn a01_wf_wxywh_of_the_desktop_is_the_screen_below_the_menu_row() {
    check_field("WF_WXYWH", |d| d.work);
}

/// A02 — `WF_CXYWH` on handle 0 is `gl_rscreen`: `(0, 0, W, H)`.
#[test]
fn a02_wf_cxywh_of_the_desktop_is_the_whole_screen() {
    check_field("WF_CXYWH", |d| d.curr);
}

/// A03 — `WF_PXYWH` on handle 0 is `gl_rscreen` in the initial state:
/// `(0, 0, W, H)`.
#[test]
fn a03_wf_pxywh_of_the_desktop_is_the_whole_screen() {
    check_field("WF_PXYWH", |d| d.prev);
}

/// A04 — `WF_FXYWH` on handle 0 is `gl_rfull`: `(0, hbox, W, H − hbox)`.
#[test]
fn a04_wf_fxywh_of_the_desktop_is_the_screen_below_the_menu_row() {
    check_field("WF_FXYWH", |d| d.full);
}

/// The four together, and that the two profiles really produce two different
/// answers — i.e. the rectangles follow the backend, they are not one frozen
/// set that happens to match the TT.
#[test]
fn the_desktop_rectangles_follow_the_backend_from_one_profile_to_the_other() {
    let _g = lock();
    let (tt, hbox_tt) = start_at(TT.0, TT.1);
    assert_eq!(mismatches(tt, expected(TT.0 as i16, TT.1 as i16, hbox_tt)), Vec::<&str>::new(), "{tt:?}");
    let (pi, hbox_pi) = start_at(PI4.0, PI4.1);
    assert_eq!(mismatches(pi, expected(PI4.0 as i16, PI4.1 as i16, hbox_pi)), Vec::<&str>::new(), "{pi:?}");
    assert_ne!(tt, pi, "switching the screen from {TT:?} to {PI4:?} must change every rectangle");
    // And back: the seed is taken from the live screen every time the AES
    // starts, not latched at the first start.
    let (tt2, _) = start_at(TT.0, TT.1);
    assert_eq!(tt2, tt, "returning to {TT:?} must restore the TT rectangles");
}

// ── the negative cases: the check is not vacuous ─────────────────────────

/// A wrong **width** in the expectation is reported for all four fields
/// (every one of them carries `W`): backend 640×480, expectation 1920 wide.
#[test]
fn a_wrong_width_is_detected_in_all_four_fields() {
    let _g = lock();
    let (got, hbox) = start_at(TT.0, TT.1);
    let bad = mismatches(got, expected(PI4.0 as i16, TT.1 as i16, hbox));
    assert_eq!(bad, ["WF_WXYWH", "WF_CXYWH", "WF_PXYWH", "WF_FXYWH"], "{got:?}");
}

/// A wrong **height** in the expectation is reported for all four fields
/// (`H` in the two screen rects, `H − hbox` in the two full rects): backend
/// 640×480, expectation 1080 tall.
#[test]
fn a_wrong_height_is_detected_in_all_four_fields() {
    let _g = lock();
    let (got, hbox) = start_at(TT.0, TT.1);
    let bad = mismatches(got, expected(TT.0 as i16, PI4.1 as i16, hbox));
    assert_eq!(bad, ["WF_WXYWH", "WF_CXYWH", "WF_PXYWH", "WF_FXYWH"], "{got:?}");
}

/// A wrong **menu-bar height** reaches exactly the two `gl_rfull` fields and
/// leaves the two `gl_rscreen` fields alone — the `hbox` term is live in the
/// rule, not a constant the comparison ignores. Checked at the Pi 4 profile so
/// the number is not the TT's.
#[test]
fn a_wrong_menu_bar_height_is_detected_in_the_two_gl_rfull_fields_only() {
    let _g = lock();
    let (got, hbox) = start_at(PI4.0, PI4.1);
    let bad = mismatches(got, expected(PI4.0 as i16, PI4.1 as i16, hbox + 1));
    assert_eq!(bad, ["WF_WXYWH", "WF_FXYWH"], "{got:?}");
}
