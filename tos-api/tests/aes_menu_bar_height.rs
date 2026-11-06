//! A20 — the menu-bar row is `gl_hbox = gl_hchar + 3`, derived from the system
//! font's cell height, not a literal 19.
//!
//! EmuTOS sets it once, in `gsx_start` (`references/emutos/aes/gemgraf.c:405-427`,
//! pinned tree `1a7484aba1bfba23e2c0899e243b6d8177fe41df`):
//!
//! ```c
//! gsx_textsize(&gl_wptschar, &gl_hptschar, &gl_wchar, &gl_hchar);  /* :405 */
//! gl_hbox = gl_hchar + 3;                                           /* :412 */
//! r_set(&gl_rfull, 0, gl_hbox, gl_width, (gl_height - gl_hbox));   /* :424 */
//! r_set(&gl_rmenu, 0, 0, gl_width, gl_hbox);                        /* :427 */
//! ```
//!
//! `gsx_textsize` is `vqt_attributes` on the AES workstation (`gemgsxif.c:505-512`,
//! `gl_hchar = ptsout[3]` = the current font's cell height), so `gl_hchar` is
//! whatever the VDI's default font is — 16 rows for the 8×16 face on a screen
//! of 400 lines or more, 8 for the 8×8 face below that (`vdi_text.c:612`).
//! One variable, three readers that this test pins together:
//!
//! | reader | EmuTOS |
//! |---|---|
//! | `graf_handle` → `int_out[4]` | `GR_HBOX = gl_hbox` (`gemsuper.c:291`) |
//! | `wind_get(0, WF_WXYWH)` → `y` | `gl_rfull.g_y` (`gemgraf.c:424`, `gemwmlib.c:1294`) |
//! | the menu-bar hitbox | `gl_rmenu` (`gemgraf.c:427`) — the pointer in that row with the buttons up hands the mouse to the Control Manager (`geminput.c:388-389`), which runs the drop-down tracker (`gemctrl.c:318-330`) |
//!
//! **What is switchable here and what is not.** The rule is checked for two
//! font sizes by feeding it the two real ring fonts' headers (`FON_8X16` →
//! 19, `FON_8X8` → 11, the cell height computed as `vqt_attributes` reports
//! it: `top + bottom + 1`). The three readers are pinned to each other and to
//! the rule at the TT and Pi sizes, which must still be the measured 19
//! (ticket acceptance 3: the 2026-09-09 Pi/TT measurement stays explained).
//! Since V32 the VDI picks `def_font` per `V_REZ_VT` as EmuTOS does
//! (`tos_api::vdi::font::text_init_def_font`), so a screen under 400 lines
//! switches the live `gl_hchar` to 8; `vdi_v32_default_font.rs` checks that
//! size through `graf_handle`.
//!
//! **Rig.** A raster backend that keeps width/height and *ignores* drawing
//! (the tracker draws the drop-down and saves the screen under it; nothing
//! here reads pixels back), plus a scripted host whose pointer moves only in
//! `Host::pump_input`, as on the board — the harness of
//! `aes_menu_click_ownership.rs`, reduced to what the hitbox check needs.
//!
//! Ticket: `TASK/A20.md`.

#![allow(clippy::too_many_arguments, unused_variables)]

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU16, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

use tos_api::bdos::process::Abi;
use tos_api::host::Host;
use tos_api::vdi::backend::{ClipRect, MonoSrc, PatternFill, Pel, RasterBackend, Rect, WriteMode};
use tos_api::vdi::font::{SysFont, FON_8X16, FON_8X8};

// ── the rule, fed with two real fonts ────────────────────────────────────

/// `vqt_attributes`' `ptsout[3]` — the cell height EmuTOS's `gsx_textsize`
/// stores in `gl_hchar` (`vdi_text.c:1018`: `top + bottom + 1`;
/// `tos-api/src/vdi/inquiry.rs::vqt_attributes`).
fn cell_height(f: &SysFont) -> i16 {
    f.top + f.bottom + 1
}

/// The derivation follows the font: 8×16 → 19 (today's measured value), 8×8 →
/// 11 (what EmuTOS computes on a screen under 400 lines, where `def_font` is
/// the 8×8 face). A constant cannot pass both.
#[test]
fn gl_hbox_is_gl_hchar_plus_3_for_both_ring_fonts() {
    let big = cell_height(&FON_8X16);
    let small = cell_height(&FON_8X8);
    assert_eq!((big, small), (16, 8), "the ring fonts' cell heights (Fonthead top+bottom+1)");
    assert_eq!(tos_api::aes::hbox_of(big), 19, "gl_hbox for the 8x16 face (gemgraf.c:412)");
    assert_eq!(tos_api::aes::hbox_of(small), 11, "gl_hbox for the 8x8 face (gemgraf.c:412)");
    assert_ne!(
        tos_api::aes::hbox_of(big),
        tos_api::aes::hbox_of(small),
        "the box height must follow the cell height"
    );
}

// ── the metadata backend that ignores drawing ─────────────────────────────

/// The geometry the reference dump was measured at (Atari TT, TT-medium).
const TT: (u16, u16) = (640, 480);
/// The current hardware profile (Pi 4, `measurements/measurement-2026-09-09/run1-pi4/`).
const PI4: (u16, u16) = (1920, 1080);

struct Profile {
    width: AtomicU16,
    height: AtomicU16,
}

impl RasterBackend for Profile {
    fn width(&self) -> u16 {
        self.width.load(Ordering::Relaxed)
    }
    fn height(&self) -> u16 {
        self.height.load(Ordering::Relaxed)
    }
    fn planes(&self) -> u16 {
        16
    }
    fn num_colors(&self) -> u16 {
        256
    }
    fn set_color(&self, i: u16, r: u16, g: u16, b: u16) {}
    fn query_color(&self, i: u16) -> (u16, u16, u16) {
        (0, 0, 0)
    }
    fn set_active_palette(&self, p: &[u16; 256]) {}
    fn pel_to_device16(&self, p: Pel) -> u16 {
        0
    }
    fn device_to_rgb565(&self, p: u32) -> u16 {
        0
    }
    fn rgb565_to_device(&self, p: u16) -> u32 {
        0
    }
    fn read_pixel(&self, x: i16, y: i16) -> Pel {
        0
    }
    fn fill_rect(&self, x: i16, y: i16, w: i16, h: i16, p: Pel, c: ClipRect) {}
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
        mask
    }
    fn fill_pattern(&self, x: i16, y: i16, w: i16, h: i16, p: PatternFill, c: ClipRect) {}
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
    }
    fn blit_mono(&self, d: Rect, s: MonoSrc, f: Pel, b: Pel, m: WriteMode, c: ClipRect) {}
    fn copy_rect(&self, s: Rect, x: i16, y: i16, op: u8, c: ClipRect) {}
    fn get_device_pixel(&self, x: i16, y: i16) -> u32 {
        0
    }
    fn put_device_pixel(&self, x: i16, y: i16, p: u32) {}
    fn flush_rect(&self, x: i16, y: i16, xx: i16, yy: i16) {}
}

// ── the host: the pointer advances only in `pump_input` ──────────────────

/// One synthetic mouse packet (VDI convention: bit 0 = left button).
#[derive(Copy, Clone, Debug)]
struct Packet {
    dx: i16,
    dy: i16,
    buttons: u8,
}

struct ScriptHost {
    raster: Profile,
    script: Mutex<VecDeque<Packet>>,
    clock: AtomicU64,
    /// Consecutive pumps on an empty script; a wait that never sees its event
    /// would otherwise spin forever inside the trap.
    idle: AtomicUsize,
}

const IDLE_CAP: usize = 20_000;

impl ScriptHost {
    fn push(&self, p: Packet) {
        self.script.lock().unwrap().push_back(p);
    }
    fn clear(&self) {
        self.script.lock().unwrap().clear();
        self.idle.store(0, Ordering::Relaxed);
    }
}

impl Host for ScriptHost {
    fn console_write_byte(&self, _byte: u8) {}
    fn console_try_read_key(&self) -> Option<u16> {
        None
    }
    fn raster(&self) -> Option<&dyn RasterBackend> {
        Some(&self.raster)
    }
    fn monotonic_millis(&self) -> u64 {
        self.clock.load(Ordering::Relaxed)
    }
    fn pump_input(&self) {
        self.clock.fetch_add(1, Ordering::Relaxed);
        let next = self.script.lock().unwrap().pop_front();
        match next {
            Some(p) => {
                self.idle.store(0, Ordering::Relaxed);
                tos_api::vdi::feed_relative(p.dx, p.dy, p.buttons, &self.raster);
            }
            None => {
                let n = self.idle.fetch_add(1, Ordering::Relaxed) + 1;
                assert!(
                    n < IDLE_CAP,
                    "the AES is still waiting after {n} pumps on an exhausted mouse script \
                     (pointer {:?}, buttons {:#04x})",
                    tos_api::vdi::mouse_position(),
                    tos_api::vdi::mouse_buttons()
                );
            }
        }
    }
}

static HOST: ScriptHost = ScriptHost {
    raster: Profile { width: AtomicU16::new(TT.0), height: AtomicU16::new(TT.1) },
    script: Mutex::new(VecDeque::new()),
    clock: AtomicU64::new(0),
    idle: AtomicUsize::new(0),
};
static HOST_REF: &dyn Host = &HOST;

/// The installed host, the menu tree and the AES state are process-wide.
static SERIAL: Mutex<()> = Mutex::new(());
static INSTALL: OnceLock<()> = OnceLock::new();

// ── AES parameter block ──────────────────────────────────────────────────

struct Aespb {
    contrl: Box<[i16; 8]>,
    global: Box<[i16; 16]>,
    intin: Box<[i16; 16]>,
    intout: Box<[i16; 16]>,
    addrin: Box<[u64; 4]>,
    addrout: Box<[u64; 4]>,
    block: Box<[u64; 6]>,
}

impl Aespb {
    fn new() -> Aespb {
        let mut pb = Aespb {
            contrl: Box::new([0; 8]),
            global: Box::new([0; 16]),
            intin: Box::new([0; 16]),
            intout: Box::new([0; 16]),
            addrin: Box::new([0; 4]),
            addrout: Box::new([0; 4]),
            block: Box::new([0; 6]),
        };
        pb.block[0] = pb.contrl.as_ptr() as u64;
        pb.block[1] = pb.global.as_ptr() as u64;
        pb.block[2] = pb.intin.as_ptr() as u64;
        pb.block[3] = pb.intout.as_ptr() as u64;
        pb.block[4] = pb.addrin.as_ptr() as u64;
        pb.block[5] = pb.addrout.as_ptr() as u64;
        pb
    }

    /// One AES call, exactly as the GEM-trap handler issues it.
    fn call(&mut self, opcode: i16, in_len: i16, out_len: i16, ain_len: i16) {
        self.contrl[0] = opcode;
        self.contrl[1] = in_len;
        self.contrl[2] = out_len;
        self.contrl[3] = ain_len;
        self.contrl[4] = 0;
        self.intout.fill(0);
        // SAFETY: `block` holds six live pointers to arrays owned by `self`,
        // each larger than any opcode issued here declares.
        assert!(unsafe { tos_api::aes::dispatch(self.block.as_ptr() as u64, Abi::Lp64) }.is_none());
    }
}

const OP_APPL_INIT: i16 = 10;
const OP_EVNT_MULTI: i16 = 25;
const OP_MENU_BAR: i16 = 30;
const OP_GRAF_HANDLE: i16 = 77;
const OP_GRAF_MOUSE: i16 = 78;
const OP_GRAF_MKSTATE: i16 = 79;
const OP_WIND_GET: i16 = 104;
const OP_WIND_NEW: i16 = 109;

const WF_WXYWH: i16 = 4;
const DESKWH: i16 = 0;

const MU_KEYBD: i16 = 0x0001;
const MU_BUTTON: i16 = 0x0002;
const MU_MESAG: i16 = 0x0010;
const MU_TIMER: i16 = 0x0020;
const MN_SELECTED: i16 = 10;

/// `graf_handle` → `(gl_wchar, gl_hchar, gl_wbox, gl_hbox)` from `int_out[1..4]`
/// (`gemsuper.c:288-291`).
fn graf_handle(pb: &mut Aespb) -> (i16, i16, i16, i16) {
    pb.call(OP_GRAF_HANDLE, 0, 5, 0);
    assert!(pb.intout[0] > 0, "graf_handle returned no handle ({})", pb.intout[0]);
    (pb.intout[1], pb.intout[2], pb.intout[3], pb.intout[4])
}

/// `wind_get(0, WF_WXYWH)` → `(x, y, w, h)`.
fn desk_work(pb: &mut Aespb) -> (i16, i16, i16, i16) {
    pb.intin.fill(0);
    pb.intin[0] = DESKWH;
    pb.intin[1] = WF_WXYWH;
    pb.call(OP_WIND_GET, 2, 5, 0);
    assert_eq!(pb.intout[0], 1, "wind_get(0, WF_WXYWH) returned FALSE");
    (pb.intout[1], pb.intout[2], pb.intout[3], pb.intout[4])
}

/// `graf_mkstate` — one AES call, hence one `dsptch()` → one `pump_input`.
fn graf_mkstate(pb: &mut Aespb) {
    pb.call(OP_GRAF_MKSTATE, 0, 5, 0);
}

/// Consume every packet still queued.
fn drain(pb: &mut Aespb) {
    for _ in 0..64 {
        if HOST.script.lock().unwrap().is_empty() {
            return;
        }
        graf_mkstate(pb);
    }
}

/// Walk the pointer to `(tx, ty)` with `buttons`, one packet per AES call.
fn move_to(pb: &mut Aespb, tx: i16, ty: i16, buttons: u8) {
    drain(pb);
    for _ in 0..200 {
        let (cx, cy) = tos_api::vdi::mouse_position();
        if cx == tx && cy == ty {
            break;
        }
        let dx = (tx - cx).clamp(-32, 32);
        let dy = (ty - cy).clamp(-32, 32);
        HOST.push(Packet { dx, dy, buttons });
        graf_mkstate(pb);
    }
    assert_eq!(tos_api::vdi::mouse_position(), (tx, ty), "move_to did not reach its target");
}

/// Empty the owner FIFO (`MU_MESAG | MU_TIMER(0)` polls until none is left).
fn drain_messages(pb: &mut Aespb) {
    for _ in 0..16 {
        let mut buf = Box::new([0i16; 8]);
        pb.intin.fill(0);
        pb.intin[0] = MU_MESAG | MU_TIMER;
        pb.addrin[0] = buf.as_mut_ptr() as u64;
        pb.call(OP_EVNT_MULTI, 16, 7, 1);
        pb.addrin[0] = 0;
        if pb.intout[0] & MU_MESAG == 0 {
            return;
        }
    }
}

/// An application's main-loop wait — `MU_MESAG | MU_BUTTON | MU_KEYBD`, the
/// button term set to return on one press: `ev_mbclicks = 0x101` (bflag 1 =
/// return when the buttons *leave* the state, `gemevlib.c:79-82`), mask 1,
/// state 0 (left button up). Returns `(fired, msg)`.
fn evnt_multi_wait_press(pb: &mut Aespb) -> (i16, [i16; 8]) {
    let mut buf = Box::new([0i16; 8]);
    pb.intin.fill(0);
    pb.intin[0] = MU_MESAG | MU_BUTTON | MU_KEYBD;
    pb.intin[1] = 0x0101;
    pb.intin[2] = 1;
    pb.intin[3] = 0;
    pb.addrin[0] = buf.as_mut_ptr() as u64;
    pb.call(OP_EVNT_MULTI, 16, 7, 1);
    pb.addrin[0] = 0;
    (pb.intout[0], *buf)
}

// ── the menu tree, built from the AES's own gl_hbox ──────────────────────

/// A GEM `OBJECT` at the kernel/LP64 record width (`tos-api/src/aes/object.rs:31`).
#[repr(C)]
#[derive(Copy, Clone)]
struct Obj {
    next: i16,
    head: i16,
    tail: i16,
    otype: u16,
    flags: u16,
    state: u16,
    spec: u64,
    x: i16,
    y: i16,
    w: i16,
    h: i16,
}

const G_BOX: u16 = 20;
const G_IBOX: u16 = 25;
const G_STRING: u16 = 28;
const G_TITLE: u16 = 32;
const LASTOB: u16 = 0x0020;

const TITLE_FILE: i16 = 4;
const ITEM_OPEN: i16 = 12;
const TITLE_W: i16 = 56;
const FILE_X: i16 = TITLE_W;

/// Desk/File/Edit with a drop-down each. The bar row and the drop-down boxes
/// are laid out from `bar_h` — the `gl_hbox` the AES reported — so the tree
/// follows the AES, not a number of its own.
fn menu_tree(sw: i16, sh: i16, bar_h: i16) -> *mut Obj {
    static DESK: &[u8] = b"Desk\0";
    static FILE: &[u8] = b"File\0";
    static EDIT: &[u8] = b"Edit\0";
    static ABOUT: &[u8] = b"About\0";
    static INFO: &[u8] = b"Info\0";
    static NEW: &[u8] = b"New\0";
    static OPEN: &[u8] = b"Open\0";
    static QUIT: &[u8] = b"Quit\0";
    static CUT: &[u8] = b"Cut\0";
    static COPY: &[u8] = b"Copy\0";

    let item_h = bar_h;
    let t = |ty, fl, spec: u64, x, y, w, h, next, head, tail| Obj {
        next, head, tail, otype: ty, flags: fl, state: 0, spec, x, y, w, h,
    };
    let s = |p: &[u8], y, next, last| {
        t(G_STRING, if last { LASTOB } else { 0 }, p.as_ptr() as u64, 0, y, 112, item_h, next, -1, -1)
    };
    let tree: Box<[Obj; 17]> = Box::new([
        /* 0 ROOT      */ t(G_IBOX, 0, 0, 0, 0, sw, sh, -1, 1, 6),
        /* 1 THEBAR    */ t(G_BOX, 0, 0, 0, 0, sw, bar_h, 6, 2, 2),
        /* 2 THEACTIVE */ t(G_IBOX, 0, 0, 0, 0, sw, bar_h, 1, 3, 5),
        /* 3 Desk      */ t(G_TITLE, 0, DESK.as_ptr() as u64, 0, 0, TITLE_W, bar_h, 4, -1, -1),
        /* 4 File      */ t(G_TITLE, 0, FILE.as_ptr() as u64, FILE_X, 0, TITLE_W, bar_h, 5, -1, -1),
        /* 5 Edit      */ t(G_TITLE, 0, EDIT.as_ptr() as u64, 2 * TITLE_W, 0, TITLE_W, bar_h, 2, -1, -1),
        /* 6 themenus  */ t(G_IBOX, 0, 0, 0, 0, sw, sh, 0, 7, 14),
        /* 7 Desk-box  */ t(G_BOX, 0, 1 << 16, 0, bar_h, 112, 2 * item_h, 10, 8, 9),
        /* 8  About    */ s(ABOUT, 0, 9, false),
        /* 9  Info     */ s(INFO, item_h, 7, false),
        /*10 File-box  */ t(G_BOX, 0, 1 << 16, FILE_X, bar_h, 112, 3 * item_h, 14, 11, 13),
        /*11  New      */ s(NEW, 0, 12, false),
        /*12  Open     */ s(OPEN, item_h, 13, false),
        /*13  Quit     */ s(QUIT, 2 * item_h, 10, false),
        /*14 Edit-box  */ t(G_BOX, 0, 1 << 16, 2 * TITLE_W, bar_h, 112, 2 * item_h, 6, 15, 16),
        /*15  Cut      */ s(CUT, 0, 16, false),
        /*16  Copy     */ s(COPY, item_h, 14, true),
    ]);
    Box::leak(tree).as_mut_ptr()
}

// ── the rig ──────────────────────────────────────────────────────────────

fn lock() -> std::sync::MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|e| e.into_inner())
}

/// Start the AES on a `w`×`h` screen: install the host once, switch the
/// profile, run the window manager's start entry (the kernel's shell loop
/// calls it before each program, `kernel/src/shell_loop.rs:84`; EmuTOS
/// `gemshlib.c:717`), and return a parameter block after `appl_init`.
fn start_at(w: u16, h: u16) -> Aespb {
    INSTALL.get_or_init(|| tos_api::host::install_host(&HOST_REF));
    HOST.clear();
    HOST.raster.width.store(w, Ordering::Relaxed);
    HOST.raster.height.store(h, Ordering::Relaxed);
    // The session's workstation open (`gsx_init`, `geminit.c:675`), whose
    // handle `graf_handle` reports — on the screen just switched to.
    tos_api::aes::gsxif::gsx_init();
    tos_api::aes::window::wm_init();
    let mut pb = Aespb::new();
    pb.call(OP_APPL_INIT, 0, 1, 0);
    assert!(pb.intout[0] >= 0, "appl_init failed ({})", pb.intout[0]);
    pb
}

/// Install a menu bar laid out from the AES's own `gl_hbox`, show the pointer,
/// park it on empty desktop with the buttons up, and empty the message pipe.
fn with_menu(pb: &mut Aespb, hbox: i16) {
    pb.intin.fill(0);
    pb.call(OP_WIND_NEW, 0, 1, 0);

    let (sw, sh) = (HOST.raster.width() as i16, HOST.raster.height() as i16);
    pb.intin.fill(0);
    pb.intin[0] = 1;
    pb.addrin[0] = menu_tree(sw, sh, hbox) as u64;
    pb.call(OP_MENU_BAR, 1, 1, 1);
    pb.addrin[0] = 0;
    assert_eq!(pb.intout[0], 1, "menu_bar must install the tree");

    pb.intin.fill(0);
    pb.intin[0] = 257; // graf_mouse(M_ON)
    pb.call(OP_GRAF_MOUSE, 1, 1, 0);

    move_to(pb, sw / 2, sh / 2, 0);
    drain_messages(pb);
}

/// Park the pointer at `(x, y)` with the buttons up, then script a press and
/// a release there and run the application's wait. Returns `(fired, msg)`.
fn press_at(pb: &mut Aespb, x: i16, y: i16) -> (i16, [i16; 8]) {
    move_to(pb, x, y, 0);
    HOST.push(Packet { dx: 0, dy: 0, buttons: 1 });
    HOST.push(Packet { dx: 0, dy: 0, buttons: 1 });
    HOST.push(Packet { dx: 0, dy: 0, buttons: 0 });
    for _ in 0..4 {
        HOST.push(Packet { dx: 0, dy: 0, buttons: 0 });
    }
    let r = evnt_multi_wait_press(pb);
    drain(pb);
    r
}

/// Park the pointer at `(x, y)` with the buttons up — inside the bar over the
/// *File* title if `y` is in the bar row — then script the gesture that picks
/// *Open*: slide down `dy` rows, press, release. Returns `(fired, msg)`.
fn press_open_from(pb: &mut Aespb, x: i16, y: i16, dy: i16) -> (i16, [i16; 8]) {
    move_to(pb, x, y, 0);
    // The gesture starts *after* the application has armed its wait with the
    // pointer parked at `(x, y)`. `ev_multi` drains the fork ring first
    // (`chkkbd(); forker();`, `aes/gemevlib.c:169-170` — one `pump_input` at
    // the head of `evnt_multi`, A92), so one idle packet stands in for that
    // drain and the slide is the first thing the wait itself sees.
    HOST.push(Packet { dx: 0, dy: 0, buttons: 0 });
    let steps = dy / 10;
    for _ in 0..steps {
        HOST.push(Packet { dx: 0, dy: 10, buttons: 0 });
    }
    HOST.push(Packet { dx: 0, dy: dy - steps * 10, buttons: 0 });
    HOST.push(Packet { dx: 0, dy: 0, buttons: 1 });
    for _ in 0..3 {
        HOST.push(Packet { dx: 0, dy: 0, buttons: 1 });
    }
    HOST.push(Packet { dx: 0, dy: 0, buttons: 0 });
    for _ in 0..8 {
        HOST.push(Packet { dx: 0, dy: 0, buttons: 0 });
    }
    let r = evnt_multi_wait_press(pb);
    drain(pb);
    r
}

// ── the three readers, pinned to the one variable ────────────────────────

/// `graf_handle`'s `gl_hbox` is the rule applied to `graf_handle`'s own
/// `gl_hchar` — and, for the 8×16 face the VDI opens with, the measured 19
/// (`measurements/measurement-2026-09-09`, TT and Pi 4 alike; ticket acceptance 3).
#[test]
fn graf_handle_reports_gl_hbox_derived_from_its_own_gl_hchar() {
    let _g = lock();
    let mut pb = start_at(TT.0, TT.1);
    let (wchar, hchar, wbox, hbox) = graf_handle(&mut pb);
    assert_eq!(
        hbox,
        tos_api::aes::hbox_of(hchar),
        "graf_handle: int_out[4] (gl_hbox) is not gl_hchar + 3 of int_out[2] (gemgraf.c:412, gemsuper.c:289-291)"
    );
    assert_eq!(hchar, cell_height(&FON_8X16), "gl_hchar is the VDI's default face's cell height");
    assert_eq!(hbox, 19, "the measured menu-bar height for the 8x16 face must not move");
    assert!(wchar > 0 && wbox >= wchar + 4, "gl_wbox >= gl_wchar + 4 (gemgraf.c:413-415)");
}

/// `wind_get(0, WF_WXYWH).y` is the same `gl_hbox` `graf_handle` reports —
/// `gl_rfull.g_y` (`gemgraf.c:424`) — on both profiles.
#[test]
fn the_desktop_work_area_starts_at_graf_handles_gl_hbox() {
    let _g = lock();
    for (w, h) in [TT, PI4] {
        let mut pb = start_at(w, h);
        let (_, _, _, hbox) = graf_handle(&mut pb);
        let (x, y, ww, hh) = desk_work(&mut pb);
        assert_eq!(
            (x, y, ww, hh),
            (0, hbox, w as i16, h as i16 - hbox),
            "wind_get(0, WF_WXYWH) at {w}x{h} is not (0, gl_hbox, W, H - gl_hbox) with gl_hbox = {hbox}"
        );
    }
}

/// The menu-bar hitbox ends where `graf_handle` says: the pointer on row
/// `gl_hbox - 1` (buttons up) is in the bar — the Control Manager takes the
/// mouse and the gesture down onto *Open* ends in `MN_SELECTED`; one row
/// lower, `gl_hbox`, the same press is the application's own `MU_BUTTON` and
/// no menu opens.
#[test]
fn the_menu_bar_hitbox_ends_at_graf_handles_gl_hbox() {
    let _g = lock();
    let mut pb = start_at(TT.0, TT.1);
    let (_, _, _, hbox) = graf_handle(&mut pb);
    with_menu(&mut pb, hbox);
    let x = FILE_X + TITLE_W / 2;

    // One row below the bar: not the menu's business.
    let (fired, msg) = press_at(&mut pb, x, hbox);
    assert_eq!(
        fired & MU_BUTTON,
        MU_BUTTON,
        "a press on row gl_hbox = {hbox} (first row under the bar) must reach the application as MU_BUTTON; fired = {fired:#06x}, msg = {msg:?}"
    );
    assert_eq!(
        fired & MU_MESAG,
        0,
        "row gl_hbox = {hbox} is below the bar: no menu may open there; fired = {fired:#06x}, msg = {msg:?}"
    );
    drain_messages(&mut pb);

    // The bar's last row: the tracker runs, *File* opens, *Open* is picked.
    let open_row = hbox + hbox + hbox / 2; // middle of the second item of File's box
    let (fired, msg) = press_open_from(&mut pb, x, hbox - 1, open_row - (hbox - 1));
    assert_eq!(
        fired & MU_MESAG,
        MU_MESAG,
        "the pointer on row gl_hbox - 1 = {} is in the bar (gl_rmenu, gemgraf.c:427): the gesture must end in MN_SELECTED; fired = {fired:#06x}, msg = {msg:?}",
        hbox - 1
    );
    assert_eq!(
        (msg[0], msg[3], msg[4]),
        (MN_SELECTED, TITLE_FILE, ITEM_OPEN),
        "MN_SELECTED must carry (File, Open); msg = {msg:?}"
    );
    assert_eq!(fired & MU_BUTTON, 0, "the selecting press must not leak as MU_BUTTON; fired = {fired:#06x}");
    drain_messages(&mut pb);
}
