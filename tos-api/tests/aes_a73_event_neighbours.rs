//! A73 — three neighbours of the H02 fix, each driven against EmuTOS:
//!
//! 1. **`MU_M1` on the entry pass** (`gemevlib.c:231-265`, `geminput.c:568-587`):
//!    EmuTOS tests a rectangle wait exactly once at arm time (`amouse`, against
//!    the pointer position *at that instant*) and afterwards only through
//!    `post_mouse(gl_mowner, …)` (`geminput.c:390,446-456`) — i.e. only while
//!    the caller owns the mouse, or when ownership comes back to it
//!    (`set_mown`, `:220-235`). A drop-down track the control manager runs in
//!    that same pass (select on press: `gl_mowner` stays `ctl_pd`, `set_mown`
//!    is a no-op while the button is held) never posts to the application's
//!    `c_msleep`; `acancel` discards the unsatisfied arm when `MN_SELECTED`
//!    wakes the app (`gemevlib.c:264`, `gemasync.c:217-243`). Select on
//!    *release* is different: `set_mown(app)` runs before the app is
//!    dispatched and `post_mouse(app, xrat, yrat)` completes the arm against
//!    the live position (`gemctrl.c:441-442` → `gemwmlib.c:321-334` →
//!    `geminput.c:225-231`), so `MU_M1` *does* come with the message there.
//! 2. **`OUTSIDE_STATE`** (`gemmnlib.c:293-305,437-443`): leaving the bar and
//!    the open box does *not* close the drop-down; `mn_do` waits for re-entry
//!    (`MU_M2` on `cur_menu`) or a button change. Re-entering the box and
//!    pressing selects the item (`MN_SELECTED`); a press outside breaks the
//!    loop as a cancel (`:373-374`, cleanup `:482-511`) and is consumed by
//!    the control manager (`gl_ctmown`, `geminput.c:334`; `set_mown` no-op
//!    while held, `:222`).
//! 3. **`menu_popup`** (`gemmnext.c:304-356`): the whole call runs in the
//!    *application's* context under `wm_update(BEG_MCTRL)` (`fm_own`,
//!    `gemfmlib.c:82-96`: `gl_mntree = NULL`, `ct_chgown(rlr, &gl_rscreen)`)
//!    — no `ct_mouse`/`gl_ctmown` bracket at all. What keeps the selecting
//!    press from reaching the app's next `evnt_multi` is the trailing
//!    `ev_button(1, 0x00ff, 0x0000, rets)` (`:351`): `mn_popup` does not
//!    return until the button is up.
//!
//! Harness (raster stand-in + scripted host + parameter block) copied from
//! `aes_menu_click_ownership.rs`; the pointer advances only in
//! `Host::pump_input`, exactly as on the board.

#![allow(clippy::too_many_arguments)]

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

use tos_api::bdos::process::Abi;
use tos_api::host::Host;
use tos_api::vdi::backend::{ClipRect, MonoSrc, PatternFill, Pel, RasterBackend, Rect, WriteMode};

const SCREEN_W: u16 = 640;
const SCREEN_H: u16 = 480;

// ── a raster backend over a real pixel buffer ──────────────────────────────

struct BufBackend {
    px: Mutex<Vec<u32>>,
    palette: Mutex<[u16; 256]>,
    req: Mutex<[(u16, u16, u16); 256]>,
}

impl BufBackend {
    fn new() -> Self {
        BufBackend {
            px: Mutex::new(vec![0; SCREEN_W as usize * SCREEN_H as usize]),
            palette: Mutex::new([0; 256]),
            req: Mutex::new([(0, 0, 0); 256]),
        }
    }

    fn inside(x: i16, y: i16) -> bool {
        x >= 0 && y >= 0 && (x as i32) < SCREEN_W as i32 && (y as i32) < SCREEN_H as i32
    }

    fn idx(x: i16, y: i16) -> usize {
        y as usize * SCREEN_W as usize + x as usize
    }

    fn put(&self, x: i16, y: i16, v: u32, clip: ClipRect) {
        if !Self::inside(x, y) {
            return;
        }
        if let Some((x0, y0, x1, y1)) = clip.clamp_to(SCREEN_W, SCREEN_H) {
            let (xi, yi) = (x as i32, y as i32);
            if xi < x0 || xi > x1 || yi < y0 || yi > y1 {
                return;
            }
        }
        self.px.lock().unwrap()[Self::idx(x, y)] = v;
    }
}

impl RasterBackend for BufBackend {
    fn width(&self) -> u16 {
        SCREEN_W
    }
    fn height(&self) -> u16 {
        SCREEN_H
    }
    fn num_colors(&self) -> u16 {
        256
    }
    fn planes(&self) -> u16 {
        16
    }
    fn set_color(&self, index: u16, r: u16, g: u16, b: u16) {
        if (index as usize) < 256 {
            self.req.lock().unwrap()[index as usize] = (r, g, b);
        }
    }
    fn query_color(&self, index: u16) -> (u16, u16, u16) {
        if (index as usize) < 256 {
            self.req.lock().unwrap()[index as usize]
        } else {
            (0, 0, 0)
        }
    }
    fn set_active_palette(&self, palette: &[u16; 256]) {
        *self.palette.lock().unwrap() = *palette;
    }
    fn pel_to_device16(&self, pel: Pel) -> u16 {
        self.palette.lock().unwrap()[pel as usize]
    }
    fn device_to_rgb565(&self, argb: u32) -> u16 {
        let (r, g, b) = ((argb >> 16) & 0xFF, (argb >> 8) & 0xFF, argb & 0xFF);
        (((r >> 3) << 11) | ((g >> 2) << 5) | (b >> 3)) as u16
    }
    fn rgb565_to_device(&self, px: u16) -> u32 {
        let (r5, g6, b5) = ((px >> 11) & 0x1F, (px >> 5) & 0x3F, px & 0x1F);
        let r = ((r5 << 3) | (r5 >> 2)) as u32;
        let g = ((g6 << 2) | (g6 >> 4)) as u32;
        let b = ((b5 << 3) | (b5 >> 2)) as u32;
        (r << 16) | (g << 8) | b
    }
    fn read_pixel(&self, x: i16, y: i16) -> Pel {
        if !Self::inside(x, y) {
            return 0;
        }
        let v = self.px.lock().unwrap()[Self::idx(x, y)];
        let want = self.device_to_rgb565(v);
        let pal = *self.palette.lock().unwrap();
        pal.iter().position(|&p| p == want).unwrap_or(0) as Pel
    }
    fn fill_rect(&self, x: i16, y: i16, w: i16, h: i16, pel: Pel, clip: ClipRect) {
        let v = self.rgb565_to_device(self.pel_to_device16(pel));
        for yy in y..y.saturating_add(h) {
            for xx in x..x.saturating_add(w) {
                self.put(xx, yy, v, clip);
            }
        }
    }
    fn abline(
        &self,
        x0: i16,
        y0: i16,
        x1: i16,
        y1: i16,
        pel: Pel,
        _mode: WriteMode,
        clip: ClipRect,
        mask: u16,
        _last: bool,
    ) -> u16 {
        let v = self.rgb565_to_device(self.pel_to_device16(pel));
        let (mut x, mut y) = (x0 as i32, y0 as i32);
        let (dx, dy) = ((x1 as i32 - x).abs(), -(y1 as i32 - y).abs());
        let (sx, sy) = (if x < x1 as i32 { 1 } else { -1 }, if y < y1 as i32 { 1 } else { -1 });
        let mut err = dx + dy;
        let mut m = mask;
        loop {
            if m & 0x8000 != 0 {
                self.put(x as i16, y as i16, v, clip);
            }
            m = m.rotate_left(1);
            if x == x1 as i32 && y == y1 as i32 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
        m
    }
    fn fill_pattern(&self, x: i16, y: i16, w: i16, h: i16, fill: PatternFill, clip: ClipRect) {
        let fg = self.rgb565_to_device(self.pel_to_device16(fill.fg));
        let bg = self.rgb565_to_device(self.pel_to_device16(fill.bg));
        for yy in y..y.saturating_add(h) {
            let row = fill.pattern[(yy.rem_euclid(16)) as usize % fill.pattern.len()];
            for xx in x..x.saturating_add(w) {
                let bit = row & (0x8000u16 >> (xx.rem_euclid(16) as u16));
                self.put(xx, yy, if bit != 0 { fg } else { bg }, clip);
            }
        }
    }
    fn draw_glyph_sized(
        &self,
        x: i16,
        y: i16,
        glyph: &[u8],
        cell_w: i16,
        cell_h: i16,
        fg: Pel,
        _mode: WriteMode,
        clip: ClipRect,
    ) {
        let v = self.rgb565_to_device(self.pel_to_device16(fg));
        for row in 0..cell_h {
            let byte = glyph.get(row as usize).copied().unwrap_or(0);
            for col in 0..cell_w.min(8) {
                if byte & (0x80 >> col) != 0 {
                    self.put(x + col, y + row, v, clip);
                }
            }
        }
    }
    fn blit_mono(&self, dst: Rect, src: MonoSrc, fg: Pel, bg: Pel, _m: WriteMode, clip: ClipRect) {
        let f = self.rgb565_to_device(self.pel_to_device16(fg));
        let b = self.rgb565_to_device(self.pel_to_device16(bg));
        for row in 0..dst.h {
            for col in 0..dst.w {
                let (sx, sy) = (src.x + col, src.y + row);
                let wi = (sy as usize) * src.stride_words as usize + (sx as usize) / 16;
                let w = src.bits.word(wi).unwrap_or(0);
                let bit = w & (0x8000u16 >> (sx.rem_euclid(16)) as u16);
                self.put(dst.x + col, dst.y + row, if bit != 0 { f } else { b }, clip);
            }
        }
    }
    fn copy_rect(&self, src: Rect, dx: i16, dy: i16, _op: u8, clip: ClipRect) {
        let mut tmp = Vec::with_capacity((src.w as usize) * (src.h as usize));
        {
            let px = self.px.lock().unwrap();
            for row in 0..src.h {
                for col in 0..src.w {
                    let (sx, sy) = (src.x + col, src.y + row);
                    tmp.push(if Self::inside(sx, sy) { px[Self::idx(sx, sy)] } else { 0 });
                }
            }
        }
        for row in 0..src.h {
            for col in 0..src.w {
                let v = tmp[(row as usize) * (src.w as usize) + col as usize];
                self.put(dx + col, dy + row, v, clip);
            }
        }
    }
    fn get_device_pixel(&self, x: i16, y: i16) -> u32 {
        if Self::inside(x, y) {
            self.px.lock().unwrap()[Self::idx(x, y)]
        } else {
            0
        }
    }
    fn put_device_pixel(&self, x: i16, y: i16, argb: u32) {
        if Self::inside(x, y) {
            self.px.lock().unwrap()[Self::idx(x, y)] = argb;
        }
    }
    fn flush_rect(&self, _x0: i16, _y0: i16, _x1: i16, _y1: i16) {}
}

// ── the host: input advances only in `pump_input`, as on the board ─────────

/// One synthetic mouse packet, in the form the VDI's `mouse_int` receives it.
#[derive(Copy, Clone, Debug)]
struct Packet {
    dx: i16,
    dy: i16,
    /// VDI convention: bit 0 = left, bit 1 = right.
    buttons: u8,
}

struct ScriptHost {
    raster: BufBackend,
    script: Mutex<VecDeque<Packet>>,
    /// Milliseconds; advanced by `pump_input` alone.
    clock: AtomicU64,
    /// Consecutive pumps that found the script empty. A tracking loop that
    /// never sees the event it waits for would otherwise spin forever inside
    /// the trap; the cap turns that into a failing assertion.
    idle: AtomicUsize,
}

/// Pumps on an empty script before the harness gives up. `count_clicks` may
/// legitimately run a whole double-click window (≤ 450 pumps) on an empty
/// script, so the cap sits well above that.
const IDLE_CAP: usize = 20_000;

impl ScriptHost {
    fn new() -> Self {
        ScriptHost {
            raster: BufBackend::new(),
            script: Mutex::new(VecDeque::new()),
            clock: AtomicU64::new(0),
            idle: AtomicUsize::new(0),
        }
    }
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
                    "the AES is still waiting after {n} pumps on an exhausted mouse script: \
                     the event it waits for never came (pointer {:?}, buttons {:#04x})",
                    tos_api::vdi::mouse_position(),
                    tos_api::vdi::mouse_buttons()
                );
            }
        }
    }
}

// SAFETY: every mutable field is behind a Mutex / atomic.
unsafe impl Sync for ScriptHost {}
unsafe impl Send for ScriptHost {}

// ── AES parameter block ────────────────────────────────────────────────────

const ABI: Abi = Abi::Lp64;

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

    /// Issue one AES call, exactly as the GEM-trap handler does.
    fn call(&mut self, opcode: i16, in_len: i16, out_len: i16, ain_len: i16) {
        self.contrl[0] = opcode;
        self.contrl[1] = in_len;
        self.contrl[2] = out_len;
        self.contrl[3] = ain_len;
        self.contrl[4] = 0;
        self.intout.fill(0);
        // SAFETY: `block` holds six live pointers to arrays owned by `self`,
        // each larger than any opcode issued here declares.
        unsafe { tos_api::aes::dispatch(self.block.as_ptr() as u64, ABI) };
    }
}

fn install(host: &'static ScriptHost) {
    let href: &'static &'static dyn Host = Box::leak(Box::new(host as &'static dyn Host));
    tos_api::host::install_host(href);
}

/// Seed the backend's pseudo-palette the way the VDI dispatcher does
/// (`ACTIVE_PALETTE`, `kernel/src/vdi_backend.rs:11-21`): EmuTOS'
/// `videl_palette1` for pens 0..15 as RGB555-in-565 words, so `read_pixel`
/// round-trips the pens the menu draws with.
fn seed_palette(b: &BufBackend) {
    const VIDEL1: [(u16, u16, u16); 16] = [
        (1000, 1000, 1000), (0, 0, 0), (1000, 0, 0), (0, 1000, 0),
        (0, 0, 1000), (0, 1000, 1000), (1000, 1000, 0), (1000, 0, 1000),
        (733, 733, 733), (533, 533, 533), (667, 0, 0), (0, 667, 0),
        (0, 0, 667), (0, 667, 667), (667, 667, 0), (667, 0, 667),
    ];
    let mut pal = [0u16; 256];
    for (i, (r, g, bl)) in VIDEL1.iter().enumerate() {
        let c5 = |v: u16| ((v as u32 * 31 + 500) / 1000) as u16;
        pal[i] = (c5(*r) << 11) | (c5(*g) << 6) | c5(*bl);
    }
    *b.palette.lock().unwrap() = pal;
}

// ── the menu tree ──────────────────────────────────────────────────────────

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

/// Object indices the cases steer by.
const TITLE_FILE: i16 = 4;
const ITEM_OPEN: i16 = 12;

/// Bar geometry: titles are 56 px wide in the 19 px bar row; the *File*
/// drop-down box sits at (56,19) and holds three 19 px items.
const BAR_H: i16 = 19;
const FILE_X: i16 = 56;
const ITEM_H: i16 = 19;

/// The desktop's menu shape (Desk/File/Edit with a drop-down each), the same
/// tree the objc selftest draws (`kernel/src/selftest_objc.rs:500-540`).
fn menu_tree() -> *mut Obj {
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

    let t = |ty, fl, spec: u64, x, y, w, h, next, head, tail| Obj {
        next, head, tail, otype: ty, flags: fl, state: 0, spec, x, y, w, h,
    };
    let s = |p: &[u8], y, next, last| {
        t(G_STRING, if last { LASTOB } else { 0 }, p.as_ptr() as u64, 0, y, 112, ITEM_H, next, -1, -1)
    };
    let tree: Box<[Obj; 17]> = Box::new([
        /* 0 ROOT      */ t(G_IBOX, 0, 0, 0, 0, 640, 480, -1, 1, 6),
        /* 1 THEBAR    */ t(G_BOX, 0, 0, 0, 0, 640, BAR_H, 6, 2, 2),
        /* 2 THEACTIVE */ t(G_IBOX, 0, 0, 0, 0, 640, BAR_H, 1, 3, 5),
        /* 3 Desk      */ t(G_TITLE, 0, DESK.as_ptr() as u64, 0, 0, 56, BAR_H, 4, -1, -1),
        /* 4 File      */ t(G_TITLE, 0, FILE.as_ptr() as u64, FILE_X, 0, 56, BAR_H, 5, -1, -1),
        /* 5 Edit      */ t(G_TITLE, 0, EDIT.as_ptr() as u64, 112, 0, 56, BAR_H, 2, -1, -1),
        /* 6 themenus  */ t(G_IBOX, 0, 0, 0, 0, 640, 480, 0, 7, 14),
        /* 7 Desk-box  */ t(G_BOX, 0, 1 << 16, 0, BAR_H, 112, 2 * ITEM_H, 10, 8, 9),
        /* 8  About    */ s(ABOUT, 0, 9, false),
        /* 9  Info     */ s(INFO, ITEM_H, 7, false),
        /*10 File-box  */ t(G_BOX, 0, 1 << 16, FILE_X, BAR_H, 112, 3 * ITEM_H, 14, 11, 13),
        /*11  New      */ s(NEW, 0, 12, false),
        /*12  Open     */ s(OPEN, ITEM_H, 13, false),
        /*13  Quit     */ s(QUIT, 2 * ITEM_H, 10, false),
        /*14 Edit-box  */ t(G_BOX, 0, 1 << 16, 112, BAR_H, 112, 2 * ITEM_H, 6, 15, 16),
        /*15  Cut      */ s(CUT, 0, 16, false),
        /*16  Copy     */ s(COPY, ITEM_H, 14, true),
    ]);
    Box::leak(tree).as_mut_ptr()
}

// ── the rig ────────────────────────────────────────────────────────────────

const MU_KEYBD: i16 = 0x0001;
const MU_BUTTON: i16 = 0x0002;
const MU_MESAG: i16 = 0x0010;
const MU_TIMER: i16 = 0x0020;
const MN_SELECTED: i16 = 10;

/// The installed host, menu tree and AES state are process-wide statics, so
/// the cases in this binary run one at a time.
static SERIAL: Mutex<()> = Mutex::new(());
static RIG: OnceLock<&'static ScriptHost> = OnceLock::new();

fn rig() -> &'static ScriptHost {
    RIG.get_or_init(|| {
        let host: &'static ScriptHost = Box::leak(Box::new(ScriptHost::new()));
        install(host);
        seed_palette(&host.raster);
        host
    })
}

/// Fresh application state with the menu bar installed and the pointer parked
/// on the desktop, buttons up, nothing queued.
fn fresh() -> (&'static ScriptHost, Aespb) {
    let host = rig();
    host.clear();
    let mut pb = Aespb::new();
    pb.call(10, 0, 1, 0); // appl_init
    pb.intin.fill(0);
    pb.call(109, 0, 1, 0); // wind_new: closes every window, drains the update locks

    pb.intin.fill(0);
    pb.intin[0] = 1; // show
    pb.addrin[0] = menu_tree() as u64;
    pb.call(30, 1, 1, 1); // menu_bar
    pb.addrin[0] = 0;
    assert_eq!(pb.intout[0], 1, "menu_bar must install the tree");

    pb.intin.fill(0);
    pb.intin[0] = 257; // graf_mouse(M_ON)
    pb.call(78, 1, 1, 0);

    // Park the pointer on empty desktop, buttons up, and empty the pipe.
    move_to(host, &mut pb, 320, 300, 0);
    drain_messages(&mut pb);
    (host, pb)
}

/// `graf_mkstate` — one AES call, hence one `dsptch()` → one `pump_input`.
fn graf_mkstate(pb: &mut Aespb) -> (i16, i16, i16) {
    pb.call(79, 0, 5, 0);
    (pb.intout[1], pb.intout[2], pb.intout[3])
}

/// Consume every packet still queued.
fn drain(host: &'static ScriptHost, pb: &mut Aespb) {
    for _ in 0..64 {
        if host.script.lock().unwrap().is_empty() {
            return;
        }
        graf_mkstate(pb);
    }
}

/// Walk the pointer to `(tx, ty)` with `buttons`, one packet per AES call.
fn move_to(host: &'static ScriptHost, pb: &mut Aespb, tx: i16, ty: i16, buttons: u8) {
    drain(host, pb);
    for _ in 0..200 {
        let (cx, cy) = tos_api::vdi::mouse_position();
        if cx == tx && cy == ty {
            break;
        }
        let dx = (tx - cx).clamp(-32, 32);
        let dy = (ty - cy).clamp(-32, 32);
        host.push(Packet { dx, dy, buttons });
        graf_mkstate(pb);
    }
    let (cx, cy) = tos_api::vdi::mouse_position();
    assert_eq!((cx, cy), (tx, ty), "move_to did not reach its target");
}

/// TeraDesk's main-loop wait: `MU_MESAG | MU_BUTTON | MU_KEYBD`, `ev_mbclicks
/// = 0x102` (bflag 1 = wait for a press, up to 2 clicks), `ev_mbmask = 3`,
/// `ev_mbstate = 0`, no timer (`sdk/apps/teradesk/main.c:1588-1610`), with a
/// real message buffer. Returns `(fired, EV_BRET, EV_MB, msg)`.
fn evnt_multi_teradesk(pb: &mut Aespb) -> (i16, i16, i16, [i16; 8]) {
    let mut buf = Box::new([0i16; 8]);
    pb.intin.fill(0);
    pb.intin[0] = MU_MESAG | MU_BUTTON | MU_KEYBD;
    pb.intin[1] = 0x0102;
    pb.intin[2] = 3;
    pb.intin[3] = 0;
    pb.addrin[0] = buf.as_mut_ptr() as u64;
    pb.call(25, 16, 7, 1);
    pb.addrin[0] = 0;
    (pb.intout[0], pb.intout[6], pb.intout[3], *buf)
}

/// Empty the owner FIFO (`MU_MESAG | MU_TIMER(0)` polls until none is left).
fn drain_messages(pb: &mut Aespb) {
    for _ in 0..16 {
        let mut buf = Box::new([0i16; 8]);
        pb.intin.fill(0);
        pb.intin[0] = MU_MESAG | MU_TIMER;
        pb.addrin[0] = buf.as_mut_ptr() as u64;
        pb.call(25, 16, 7, 1);
        pb.addrin[0] = 0;
        if pb.intout[0] & MU_MESAG == 0 {
            return;
        }
    }
}

/// Pointer positions: over the *File* title, and over its *Open* item.
const ON_FILE: (i16, i16) = (FILE_X + 28, 8);
const ON_OPEN: (i16, i16) = (FILE_X + 28, BAR_H + ITEM_H + 9);

// ── A73 helpers ────────────────────────────────────────────────────────────

const MU_M1: i16 = 0x0004;

/// The desktop below the bar: every drop-down item lies inside it, the bar
/// row does not.
const BELOW_BAR: (i16, i16, i16, i16) = (0, BAR_H, SCREEN_W as i16, SCREEN_H as i16 - BAR_H);

/// TeraDesk's main-loop wait plus an `MU_M1` **enter** rectangle
/// (`MO_FLAGS = 0`). Returns `(fired, EV_BRET, EV_MX, EV_MY, msg)`.
fn evnt_multi_with_m1(pb: &mut Aespb, rect: (i16, i16, i16, i16), timer_ms: Option<u16>) -> (i16, i16, i16, i16, [i16; 8]) {
    let mut buf = Box::new([0i16; 8]);
    pb.intin.fill(0);
    pb.intin[0] = MU_MESAG | MU_BUTTON | MU_KEYBD | MU_M1 | timer_ms.map_or(0, |_| MU_TIMER);
    pb.intin[1] = 0x0102;
    pb.intin[2] = 3;
    pb.intin[3] = 0;
    pb.intin[4] = 0; // MMO1_FLAGS: enter
    pb.intin[5] = rect.0;
    pb.intin[6] = rect.1;
    pb.intin[7] = rect.2;
    pb.intin[8] = rect.3;
    pb.intin[14] = timer_ms.unwrap_or(0) as i16;
    pb.intin[15] = 0;
    pb.addrin[0] = buf.as_mut_ptr() as u64;
    pb.call(25, 16, 7, 1);
    pb.addrin[0] = 0;
    (pb.intout[0], pb.intout[6], pb.intout[1], pb.intout[2], *buf)
}

/// TeraDesk's wait with a real (non-zero) timer: the loop pumps the script
/// until the deadline and reports `MU_TIMER` alone if nothing else fired.
fn evnt_multi_timed(pb: &mut Aespb, ms: u16) -> (i16, i16, [i16; 8]) {
    let mut buf = Box::new([0i16; 8]);
    pb.intin.fill(0);
    pb.intin[0] = MU_MESAG | MU_BUTTON | MU_KEYBD | MU_TIMER;
    pb.intin[1] = 0x0102;
    pb.intin[2] = 3;
    pb.intin[3] = 0;
    pb.intin[14] = ms as i16;
    pb.intin[15] = 0;
    pb.addrin[0] = buf.as_mut_ptr() as u64;
    pb.call(25, 16, 7, 1);
    pb.addrin[0] = 0;
    (pb.intout[0], pb.intout[6], *buf)
}

/// Queue a vertical glide of the pointer by `dy_total` in 10 px packets,
/// `buttons` held throughout.
fn glide_y(host: &ScriptHost, dy_total: i16, buttons: u8) {
    let step = if dy_total < 0 { -10 } else { 10 };
    let mut left = dy_total;
    while left.abs() > 10 {
        host.push(Packet { dx: 0, dy: step, buttons });
        left -= step;
    }
    host.push(Packet { dx: 0, dy: left, buttons });
}

/// A press held for a few packets, then released, then idle.
fn click(host: &ScriptHost) {
    host.push(Packet { dx: 0, dy: 0, buttons: 1 });
    for _ in 0..3 {
        host.push(Packet { dx: 0, dy: 0, buttons: 1 });
    }
    host.push(Packet { dx: 0, dy: 0, buttons: 0 });
    for _ in 0..8 {
        host.push(Packet { dx: 0, dy: 0, buttons: 0 });
    }
}

/// `ev_multi` opens with `chkkbd(); forker();` (`aes/gemevlib.c:169-170`):
/// whatever is queued when the application calls is drained *before* the
/// quick checks and the `abutton`/`amouse` arms — NewTOS pumps once at the
/// head of `evnt_multi` (A92). The cases below model input that arrives
/// **after** the arm (the pointer rests on the title, buttons up, when the
/// app arms), so they queue one idle packet for that drain to consume;
/// without it the first scripted packet would be pre-arm input — for 1b a
/// press already down at arm time, which `abutton`'s ownership-blind level
/// test answers with an immediate `MU_BUTTON` (`aes/geminput.c:549-551`;
/// `docs/api/aes/menu.md` "The one documented exception").
fn arm_gap(host: &ScriptHost) {
    host.push(Packet { dx: 0, dy: 0, buttons: 0 });
}

// ── Point 1: MU_M1 on the entry pass ───────────────────────────────────────

/// **Point 1a — select on press, track in the entry pass.** The app arms
/// `MU_M1` = *enter the desktop below the bar* while the pointer rests on the
/// *File* title (buttons up) — so the control manager's track runs in the
/// very pass that arms the wait. The track slides onto *Open* (inside the
/// rectangle) and selects on the press.
///
/// EmuTOS: `amouse` tested the rectangle against the arm-time position (in
/// the bar → not satisfied) and parked the wait on the app's `c_msleep`;
/// every move of the track went to `post_mouse(ctl_pd, …)`; the press left
/// `gl_mowner` at `ctl_pd` (`gl_ctmown`), `ct_msgup` dispatches the app while
/// the button is still held, so `set_mown` never ran for it; `acancel`
/// discards the arm. The call returns **`MU_MESAG` alone**. The *next* call
/// re-arms with the pointer already inside the rectangle and `amouse` fires
/// it at once — the `MU_M1` arrives one call later, not together with the
/// message.
///
/// The press is held **past the double-click window** (A127 D1): the track
/// acts on the published press, as `mn_do`'s `ev_multi` does, and while any
/// multi-click wait is pending (`gl_bpend` — this binary's earlier cases park
/// TeraDesk's `0x0102` waits and `acancel` never takes the count back) the
/// press opens the window instead of being published (`b_click`,
/// `aes/geminput.c:133-140`). A release inside the window would be published
/// together with the press when it closes (`b_delay`, `:155-171`), the track
/// would select with the button up and the case would be 1b's. Held past it,
/// the press is published while still down, whatever `gl_bpend` stands at.
#[test]
fn m1_armed_before_press_track_is_not_delivered_with_mn_selected() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();

    move_to(host, &mut pb, ON_FILE.0, ON_FILE.1, 0);
    assert!(ON_FILE.1 < BAR_H, "arm-time position must be in the bar row");

    arm_gap(host);
    glide_y(host, ON_OPEN.1 - ON_FILE.1, 0);
    // Press, held for 500 pumps = 500 ms of the harness clock, beyond the
    // longest window (450 ms, `gl_dcrates[0]`, `aes/gemevlib.c:33`), then
    // release and idle.
    for _ in 0..500 {
        host.push(Packet { dx: 0, dy: 0, buttons: 1 });
    }
    host.push(Packet { dx: 0, dy: 0, buttons: 0 });
    for _ in 0..8 {
        host.push(Packet { dx: 0, dy: 0, buttons: 0 });
    }

    let (fired, bret, mx, my, msg) = evnt_multi_with_m1(&mut pb, BELOW_BAR, None);
    eprintln!("1a: fired={fired:#06x} bret={bret} at=({mx},{my}) msg={msg:?}");
    assert_eq!(fired & MU_MESAG, MU_MESAG, "the selection must arrive as MU_MESAG; fired = {fired:#06x}");
    assert_eq!(msg[0], MN_SELECTED, "msg = {msg:?}");
    assert_eq!((msg[3], msg[4]), (TITLE_FILE, ITEM_OPEN), "msg = {msg:?}");
    assert_eq!(fired & MU_BUTTON, 0, "H02 regression: the selecting press leaked as MU_BUTTON; fired = {fired:#06x}");
    assert_eq!(
        fired & MU_M1,
        0,
        "MU_M1 was delivered next to MN_SELECTED for a rectangle the pointer entered \
         only *during* the control manager's track: EmuTOS arms the wait against the \
         arm-time position (amouse, aes/geminput.c:568-587), posts rectangle events \
         only to gl_mowner = ctl_pd during the track (mchange, :388-390), and acancel \
         discards the unsatisfied arm when MN_SELECTED wakes the app (aes/gemevlib.c:264); \
         fired = {fired:#06x}, pointer now at ({mx},{my})"
    );

    // The next arm sees the pointer inside the rectangle: `amouse` completes it
    // immediately (`geminput.c:575-576`), ownership-blind. Release first so the
    // long-press exception does not add MU_BUTTON.
    while !host.script.lock().unwrap().is_empty() {
        drain(host, &mut pb);
    }
    assert_eq!(tos_api::vdi::mouse_buttons() & 1, 0, "the release must have been pumped");
    let (fired2, _, _, _, msg2) = evnt_multi_with_m1(&mut pb, BELOW_BAR, Some(0));
    eprintln!("1a next call: fired={fired2:#06x} msg={msg2:?}");
    assert_eq!(fired2 & MU_M1, MU_M1, "the re-armed M1 fires at arm time (pointer inside); fired = {fired2:#06x}");
    assert_eq!(fired2 & (MU_MESAG | MU_BUTTON), 0, "nothing else; fired = {fired2:#06x}, msg = {msg2:?}");
}

/// **Point 1b — select on release (regression guard for the fix).** Same
/// arm, but the user presses on the *File* title, drags onto *Open* and
/// releases there. Now `mn_do` returns with the button up, `ct_msgup` does
/// not wait, `ctlmgr` loops into `w_setactive` → `set_mown(app)` →
/// `post_mouse(app, xrat, yrat)` **before** the app is dispatched
/// (`gemctrl.c:441-442`, `geminput.c:225-231`): the parked arm is completed
/// against the live position, and the call returns **`MU_MESAG | MU_M1`**.
/// A fix that simply froze the pass-0 rectangle test at the arm-time position
/// would break this case; the model is "arm-time test, then live test iff the
/// caller owns the mouse again".
#[test]
fn m1_armed_before_release_track_is_delivered_with_mn_selected() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();

    move_to(host, &mut pb, ON_FILE.0, ON_FILE.1, 0);

    arm_gap(host);
    host.push(Packet { dx: 0, dy: 0, buttons: 1 }); // press on the title
    host.push(Packet { dx: 0, dy: 0, buttons: 1 });
    glide_y(host, ON_OPEN.1 - ON_FILE.1, 1); // drag down, held
    host.push(Packet { dx: 0, dy: 0, buttons: 1 }); // dwell on *Open*
    host.push(Packet { dx: 0, dy: 0, buttons: 0 }); // release → the selection
    for _ in 0..8 {
        host.push(Packet { dx: 0, dy: 0, buttons: 0 });
    }

    let (fired, bret, mx, my, msg) = evnt_multi_with_m1(&mut pb, BELOW_BAR, None);
    eprintln!("1b: fired={fired:#06x} bret={bret} at=({mx},{my}) msg={msg:?}");
    assert_eq!(fired & MU_MESAG, MU_MESAG, "fired = {fired:#06x}");
    assert_eq!((msg[0], msg[3], msg[4]), (MN_SELECTED, TITLE_FILE, ITEM_OPEN), "msg = {msg:?}");
    assert_eq!(fired & MU_BUTTON, 0, "H02 regression: the selecting release leaked as MU_BUTTON; fired = {fired:#06x}");
    assert_eq!(
        fired & MU_M1,
        MU_M1,
        "select-on-release: set_mown(app) → post_mouse(app, xrat, yrat) completes the \
         parked M1 against the live position before the app runs (aes/geminput.c:225-231, \
         aes/gemctrl.c:441-442); fired = {fired:#06x}"
    );
}

// ── Point 2: OUTSIDE_STATE keeps the drop-down open ────────────────────────

/// Below the *File* box (y 19..76) on empty desktop.
const BELOW_FILE_BOX: (i16, i16) = (ON_OPEN.0, 200);

/// **Point 2a — leave the box, come back, press: it is a selection.** The
/// pointer opens *File* by hovering, slides down through the box and out onto
/// the desktop (`OUTSIDE_STATE`: the box stays down, the title stays hilited,
/// the item is un-hilited — `gemmnlib.c:437-443,451-459`), then back up onto
/// *Open* (`MU_M2` on `cur_menu` → `INITEM_STATE`, `:294-296,422-426`) and
/// presses. EmuTOS returns `(File, Open)` → one `MN_SELECTED`, no `MU_BUTTON`.
#[test]
fn leaving_the_box_keeps_it_open_and_reentry_press_selects() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();

    move_to(host, &mut pb, ON_FILE.0, ON_FILE.1, 0);

    arm_gap(host);
    glide_y(host, BELOW_FILE_BOX.1 - ON_FILE.1, 0); // down through the box, out
    for _ in 0..3 {
        host.push(Packet { dx: 0, dy: 0, buttons: 0 }); // dwell outside
    }
    glide_y(host, ON_OPEN.1 - BELOW_FILE_BOX.1, 0); // back up onto *Open*
    host.push(Packet { dx: 0, dy: 0, buttons: 0 });
    click(host); // press → the selection (hover mode: buparm 0x00010101)

    let (fired, bret, _mb, msg) = evnt_multi_teradesk(&mut pb);
    let (mx, my) = tos_api::vdi::mouse_position();
    eprintln!("2a: fired={fired:#06x} bret={bret} at=({mx},{my}) msg={msg:?}");
    assert_eq!(
        fired & MU_BUTTON,
        0,
        "a press inside a drop-down that EmuTOS still holds open (OUTSIDE_STATE, \
         aes/gemmnlib.c:293-305) reached the application as MU_BUTTON (EV_BRET = {bret}) \
         at ({mx},{my}): NewTOS closed the box when the pointer left bar + box \
         (menu.rs track_owned) and handed the mouse back; fired = {fired:#06x}, msg = {msg:?}"
    );
    assert_eq!(fired & MU_MESAG, MU_MESAG, "the press must be the selection; fired = {fired:#06x}");
    assert_eq!((msg[0], msg[3], msg[4]), (MN_SELECTED, TITLE_FILE, ITEM_OPEN), "msg = {msg:?}");
}

/// **Point 2b — a press outside cancels and is consumed.** Same glide out of
/// the box, then a click on the desktop there. In `OUTSIDE_STATE` the
/// `MU_BUTTON` breaks `mn_do` (`gemmnlib.c:369-374`) → cleanup restores the
/// box and un-hilites the title, `mn_do` returns FALSE (no message). The press
/// happened under `gl_ctmown`, `post_button(ctl_pd, …)` never visits the
/// app's list, `set_mown` stays a no-op until the release, and the release
/// satisfies no press-wait: the app sees nothing but its timer.
#[test]
fn press_outside_the_open_box_is_consumed_as_cancel() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();

    move_to(host, &mut pb, ON_FILE.0, ON_FILE.1, 0);

    arm_gap(host);
    glide_y(host, BELOW_FILE_BOX.1 - ON_FILE.1, 0); // down through the box, out
    for _ in 0..3 {
        host.push(Packet { dx: 0, dy: 0, buttons: 0 });
    }
    click(host); // press on the desktop → cancel, consumed

    let (fired, bret, msg) = evnt_multi_timed(&mut pb, 300);
    let (mx, my) = tos_api::vdi::mouse_position();
    eprintln!("2b: fired={fired:#06x} bret={bret} at=({mx},{my}) msg={msg:?}");
    assert_eq!(
        fired & MU_BUTTON,
        0,
        "the click that cancels an open drop-down is the control manager's \
         (OUTSIDE_STATE → MU_BUTTON → break, aes/gemmnlib.c:373-374; gl_ctmown keeps \
         gl_mowner = ctl_pd, aes/geminput.c:334) — NewTOS delivered it to the app as \
         MU_BUTTON (EV_BRET = {bret}) at ({mx},{my}); fired = {fired:#06x}"
    );
    assert_eq!(fired, MU_TIMER, "only the timer may fire; fired = {fired:#06x}, msg = {msg:?}");
}

// ── Point 3: menu_popup returns only after the button is up ────────────────

/// A three-item pop-up: root `G_BOX` 112×57, items at y 0/19/38.
fn popup_tree() -> *mut Obj {
    static A: &[u8] = b"Alpha\0";
    static B: &[u8] = b"Beta\0";
    static C: &[u8] = b"Gamma\0";
    let s = |p: &[u8], y, next, last| Obj {
        next,
        head: -1,
        tail: -1,
        otype: G_STRING,
        flags: if last { LASTOB } else { 0 },
        state: 0,
        spec: p.as_ptr() as u64,
        x: 0,
        y,
        w: 112,
        h: ITEM_H,
    };
    let tree: Box<[Obj; 4]> = Box::new([
        Obj { next: -1, head: 1, tail: 3, otype: G_BOX, flags: 0, state: 0, spec: 1 << 16, x: 0, y: 0, w: 112, h: 3 * ITEM_H },
        s(A, 0, 2, false),
        s(B, ITEM_H, 3, false),
        s(C, 2 * ITEM_H, 0, true),
    ]);
    Box::leak(tree).as_mut_ptr()
}

/// The `MENU` struct at the LP64 layout (`mn_tree` @0, four WORDs @8).
#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct MenuRec {
    mn_tree: u64,
    mn_menu: i16,
    mn_item: i16,
    mn_scroll: i16,
    mn_keystate: i16,
}

/// **Point 3 — `menu_popup` must not return while the selecting press is
/// held.** The app calls `menu_popup` with the button still down (the click
/// that opened it), the script releases, glides one item down, presses.
/// EmuTOS: `ev_button(1, 0x00ff, 0x0000)` before (`gemmnext.c:314`),
/// `handle_popup` returns on the press (`:290-291`), then **`ev_button(1,
/// 0x00ff, 0x0000)` again** (`:351`) — `mn_popup` returns with the button
/// up, so the app's next `evnt_multi` (wait for a press) finds nothing: the
/// selecting press is consumed inside `menu_popup`. Not `gl_ctmown`: the
/// whole call is the app's own context under `wm_update(BEG_MCTRL)`.
///
/// **Host note.** `mn_popup_owned` saves the screen under the box through
/// `gsxif::popup_blit_save` — on the board a pool block from
/// `bdos::mem::traps::mxalloc`, which no host process can map (the AES
/// re-derives the `Long` pool key through the kernel high-half alias). Until
/// A89 the host had no pool and the save returned `None`, so this case could
/// not run here (`rc = 0` without a track, the D-086 path) and was ignored;
/// since A89 (2026-09-21) `popup_blit_save` keeps the pixels on the heap when
/// there is no pool at all (as `bb_save` does without a screen buffer), so the
/// pop-up is tracked on the host and the case runs. No shipped application
/// calls `menu_popup` (cflib's `cf_menu_popup` falls back to its own tracker
/// because `appl_getinfo` (130) is absent), so this is the only proof of the
/// bracket besides the `menu::popup_wait_tests` unit cases.
#[test]
fn menu_popup_waits_for_the_selecting_press_to_be_released() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();

    // The click that opens the pop-up: pressed when the app calls menu_popup.
    host.push(Packet { dx: 0, dy: 0, buttons: 1 });
    graf_mkstate(&mut pb);
    assert_eq!(tos_api::vdi::mouse_buttons() & 1, 1, "the opening press must be down");
    let (px, py) = tos_api::vdi::mouse_position();

    // Script consumed inside menu_popup: release, glide one item down, press
    // (the selection), hold, release, idle.
    host.push(Packet { dx: 0, dy: 0, buttons: 0 });
    host.push(Packet { dx: 0, dy: 0, buttons: 0 });
    glide_y(host, ITEM_H, 0);
    host.push(Packet { dx: 0, dy: 0, buttons: 0 });
    click(host);

    let tree = popup_tree();
    let mut min = Box::new(MenuRec { mn_tree: tree as u64, mn_menu: 0, mn_item: 1, mn_scroll: 0, mn_keystate: 0 });
    let mut mout = Box::new(MenuRec { mn_tree: 0, mn_menu: 0, mn_item: -1, mn_scroll: 0, mn_keystate: 0 });
    pb.intin.fill(0);
    pb.intin[0] = px; // MPOP_XPOS: the start item sits under the pointer
    pb.intin[1] = py;
    pb.addrin[0] = &mut *min as *mut MenuRec as u64;
    pb.addrin[1] = &mut *mout as *mut MenuRec as u64;
    pb.call(36, 2, 1, 2); // menu_popup
    pb.addrin[0] = 0;
    pb.addrin[1] = 0;
    let rc = pb.intout[0];
    let mb_after = tos_api::vdi::mouse_buttons();
    let (mx, my) = tos_api::vdi::mouse_position();
    eprintln!("3: menu_popup rc={rc} out={mout:?} buttons after={mb_after:#04x} at=({mx},{my})");
    assert_eq!(rc, 1, "an item was picked");
    assert_eq!(mout.mn_item, 2, "the item one row below the start item; out = {mout:?}");

    // (a) the level: EmuTOS waits for button-up before returning (gemmnext.c:351).
    assert_eq!(
        mb_after & 1,
        0,
        "menu_popup returned with the selecting press still held: EmuTOS' mn_popup \
         calls ev_button(1, 0x00ff, 0x0000, rets) after handle_popup (aes/gemmnext.c:351) \
         and returns only once the button is up"
    );

    // (b) what the app sees next: its usual wait for a press must not be
    // satisfied by the press that picked the pop-up item.
    let (fired, bret, msg) = evnt_multi_timed(&mut pb, 50);
    eprintln!("3 next call: fired={fired:#06x} bret={bret} msg={msg:?}");
    assert_eq!(
        fired & MU_BUTTON,
        0,
        "the press that picked the pop-up item leaked to the application's next \
         evnt_multi as MU_BUTTON (EV_BRET = {bret}): in EmuTOS the button is already \
         up when menu_popup returns (aes/gemmnext.c:351), so neither the quick check \
         nor abutton (aes/geminput.c:549) sees a press; fired = {fired:#06x}"
    );
    assert_eq!(fired, MU_TIMER, "only the timer; fired = {fired:#06x}, msg = {msg:?}");
}
