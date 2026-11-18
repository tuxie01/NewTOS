//! A87 — the mouse goes to the Control Manager only inside `gl_ctwait` =
//! `THEACTIVE`'s rectangle of the registered bar, not anywhere in the bar row
//! — and A88 — `menu_popup`'s box placement (`clamp_ypos`) and `mn_keystate`
//! through the whole AES call (the pool-less branch a host process can reach;
//! the tracked half and the `BEG_MCTRL` bracket are unit-tested in
//! `src/aes/menu.rs` `popup_wait_tests`).
//!
//! EmuTOS: `mn_bar(tree, TRUE)` sets the Control Manager's wait rectangle to
//! `THEACTIVE`'s absolute extent — `ob_actxywh(tree, THEACTIVE,
//! &gl_ctwait.m_gr)` (`aes/gemmnlib.c:717`) — i.e. the titles' width, while
//! `mn_bar(…, FALSE)` puts `gl_rmenu` back (`:727`). The hand-over on a move
//! with the buttons up is `if ((gl_mowner != ctl_pd) && (button == 0) &&
//! gl_mntree && in_mrect(&gl_ctwait)) gl_mowner = ctl_pd;` (`mchange`,
//! `aes/geminput.c:388-389`), and the Control Manager's own `MU_M1` wait is
//! over the same `gl_ctwait` (`ctlmgr`, `aes/gemctrl.c:456-460`), armed only
//! `if (gl_mntree)` (`:451-452`). So:
//!
//! * a pointer **moving** in the bar row right of the last title never leaves
//!   the application: its `MU_M1`/`MU_M2` waits complete there, and no track
//!   starts — the application keeps running (timer, keys, messages);
//! * a pointer on a title still starts the drop-down track;
//! * with no bar installed (`gl_mntree == NULL`) no move hands the mouse over
//!   at all, whatever `gl_ctwait` holds.
//!
//! A **press** in the bar row is a different rule and is *not* the
//! application's anywhere in the row: `bchange` → `chkown` → `chk_ctrl`
//! answers `-1` for `inside(mx, my, &gl_rmenu)` (`aes/geminput.c:96-97,
//! 316-322, 334-335`), so `gl_mowner = ctl_pd`, `post_button(ctl_pd, …)` never
//! visits the application's parked wait, and the Control Manager's own
//! `MU_BUTTON` finds no window there (`wm_find` = 0, `aes/gemctrl.c:467-470`)
//! and drops it; after the release `w_setactive` → `set_mown(app)` posts
//! `button == 0` (`aes/geminput.c:225-231`), which satisfies no press-wait.
//! The click right of the titles is therefore **swallowed** on EmuTOS — the
//! cases below assert exactly that, not a delivered `MU_BUTTON` (the ticket's
//! draft expectation was checked against the source and corrected, A87).
//!
//! NewTOS tested `my < gl_hbox` — the whole row — in both `mchange`
//! (`input.rs`) and the inline Control Manager (`event.rs menu_intercept`),
//! so a pointer resting right of the titles put the application into the
//! control manager's track, which returned only when the pointer left the
//! row: the application's timer, keys and rectangle waits were dead meanwhile.
//!
//! The tree here has `THEACTIVE` 168 px wide (three 56 px titles) in a 640 px
//! bar, as an RSC menu does (`THEACTIVE`'s width is the titles' — EmuTOS
//! desktop `adjust_menu`, `desk/deskmain.c:1528-1534`); the A73 harness tree
//! had it screen-wide, which is why none of those cases could see this.
//!
//! Harness (raster stand-in + scripted host + parameter block) copied from
//! `aes_a73_event_neighbours.rs`; the pointer advances only in
//! `Host::pump_input`, exactly as on the board.

#![allow(clippy::too_many_arguments)]
#![allow(dead_code)]

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
    fn set_active_palette(&self, palette: &[u32; 256]) {
        *self.palette.lock().unwrap() = palette.map(|p| p as u16);
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
/// drop-down box sits at (56,19) and holds three 19 px items. `THEACTIVE`
/// spans the three titles only — `ACTIVE_W` = 168 of the 640 px bar — so the
/// row right of *Edit* is bar without a title.
const BAR_H: i16 = 19;
const FILE_X: i16 = 56;
const ITEM_H: i16 = 19;
const ACTIVE_W: i16 = 3 * 56;

/// The desktop's menu shape (Desk/File/Edit with a drop-down each), the tree
/// the objc selftest draws (`kernel/src/selftest_objc.rs:500-540`) with
/// `THEACTIVE` sized to its titles.
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
        /* 2 THEACTIVE */ t(G_IBOX, 0, 0, 0, 0, ACTIVE_W, BAR_H, 1, 3, 5),
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

/// Consume every packet still queued (a rest of 300 packets may be left
/// over from a timed wait, so the bound is generous).
fn drain(host: &'static ScriptHost, pb: &mut Aespb) {
    for _ in 0..2000 {
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
/// head of `evnt_multi` (A92). The click cases below model a press that
/// arrives **after** the arm (the pointer rests in the row, buttons up,
/// when the app arms), so they queue one idle packet for that drain;
/// without it the press would be pre-arm input — a level already down at
/// arm time, which `abutton`'s ownership-blind test answers with an
/// immediate `MU_BUTTON` in EmuTOS too (`aes/geminput.c:549-551`;
/// `docs/api/aes/menu.md` "The one documented exception").
fn arm_gap(host: &ScriptHost) {
    host.push(Packet { dx: 0, dy: 0, buttons: 0 });
}

// ── A87 cases ──────────────────────────────────────────────────────────────

/// Bar row, right of `THEACTIVE`: x well past the three titles, y inside the
/// `gl_hbox` row.
const RIGHT_OF_TITLES: (i16, i16) = (400, 8);

/// The bar row right of the titles as an `MU_M1` rectangle: from
/// `THEACTIVE`'s right edge to the screen edge, the full row height.
const ROW_RIGHT_OF_ACTIVE: (i16, i16, i16, i16) = (ACTIVE_W, 0, SCREEN_W as i16 - ACTIVE_W, BAR_H);

/// The bar row plus the tallest drop-down, for "the track drew nothing"
/// comparisons.
fn bar_and_boxes_pixels(host: &ScriptHost) -> Vec<u32> {
    let px = host.raster.px.lock().unwrap();
    px[..(SCREEN_W as usize) * (BAR_H as usize + 3 * ITEM_H as usize)].to_vec()
}

/// Let the pointer rest where it is for `n` packets.
fn dwell(host: &ScriptHost, n: usize) {
    for _ in 0..n {
        host.push(Packet { dx: 0, dy: 0, buttons: 0 });
    }
}

/// TeraDesk's main-loop wait plus a timer, measuring the pumped time.
fn evnt_multi_timed_elapsed(host: &ScriptHost, pb: &mut Aespb, ms: u16) -> (i16, i16, [i16; 8], u64) {
    let t0 = host.clock.load(Ordering::Relaxed);
    let (fired, bret, msg) = evnt_multi_timed(pb, ms);
    (fired, bret, msg, host.clock.load(Ordering::Relaxed) - t0)
}

/// **The defect.** Bar installed; the pointer rests in the bar row right of
/// the last title, buttons up, and stays there for 300 packets before it
/// leaves. TeraDesk's main-loop wait plus a 100 ms timer. EmuTOS: the pointer
/// is outside `gl_ctwait`, nothing is handed over, the Control Manager's
/// `MU_M1` does not fire, and the application's timer goes off on time with
/// the pointer still in the row. NewTOS took the whole row for the bar:
/// `menu_intercept` entered the track, which waited for the pointer to leave
/// the row (300+ packets) before the application saw its timer.
#[test]
fn a87_the_app_keeps_running_while_the_pointer_rests_right_of_theactive() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();

    move_to(host, &mut pb, RIGHT_OF_TITLES.0, RIGHT_OF_TITLES.1, 0);
    assert!(RIGHT_OF_TITLES.1 < BAR_H && RIGHT_OF_TITLES.0 >= ACTIVE_W, "the position must be bar row, right of THEACTIVE");
    let before = bar_and_boxes_pixels(host);

    dwell(host, 300);
    glide_y(host, 100 - RIGHT_OF_TITLES.1, 0); // then leave the row

    let (fired, bret, msg, elapsed) = evnt_multi_timed_elapsed(host, &mut pb, 100);
    let (mx, my) = tos_api::vdi::mouse_position();
    eprintln!("a87 rest right-of-titles: fired={fired:#06x} bret={bret} at=({mx},{my}) msg={msg:?} elapsed={elapsed}");
    assert_eq!(fired, MU_TIMER, "only the timer; fired = {fired:#06x}, msg = {msg:?}");
    assert!(
        elapsed < 150,
        "the application was parked in the control manager's track for {elapsed} ms while the \
         pointer rested in the bar row right of THEACTIVE: EmuTOS hands the mouse to ctl_pd only \
         inside gl_ctwait = THEACTIVE's rectangle (mn_bar, aes/gemmnlib.c:717; mchange, \
         aes/geminput.c:388-389), and the Control Manager's MU_M1 covers the same rectangle \
         (aes/gemctrl.c:456-460) — the timer must fire at ~100 ms"
    );
    assert!(my < BAR_H && mx >= ACTIVE_W, "the pointer is still in the row at the timer, at ({mx},{my})");
    assert_eq!(bar_and_boxes_pixels(host), before, "no drop-down / title highlight may have been drawn");
}

/// **`mchange` alone.** No click: the application waits for `MU_M1` = *enter
/// the bar row right of the titles* (plus a timer) while the pointer glides
/// up from the desktop into that rectangle. EmuTOS: the pointer never enters
/// `gl_ctwait`, so `gl_mowner` stays the application and `post_mouse(app, …)`
/// completes the rectangle wait on entry. NewTOS's `mchange` handed the mouse
/// to the Control Manager on entering the row, so the live `MU_M1` test (owner
/// only, `aes/gemevlib.c:185`) never ran and the interception ate the moves.
#[test]
fn a87_entering_the_bar_row_right_of_theactive_keeps_the_mouse_with_the_app() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();

    move_to(host, &mut pb, RIGHT_OF_TITLES.0, 60, 0);
    // Up into the row in 10 px steps (the last step lands at y = 8), a few
    // idle packets, then out again — so a track that swallowed the entry has
    // a way to end before the timer.
    glide_y(host, RIGHT_OF_TITLES.1 - 60, 0);
    dwell(host, 3);
    glide_y(host, 60 - RIGHT_OF_TITLES.1, 0);

    let (fired, _bret, mx, my, msg) = evnt_multi_with_m1(&mut pb, ROW_RIGHT_OF_ACTIVE, Some(400));
    eprintln!("a87 enter right-of-titles: fired={fired:#06x} at=({mx},{my}) msg={msg:?}");
    assert_eq!(
        fired & MU_M1,
        MU_M1,
        "the pointer entered the bar row right of THEACTIVE while the application owned the \
         mouse: EmuTOS's hand-over needs in_mrect(&gl_ctwait) (aes/geminput.c:388-389) and \
         gl_ctwait is THEACTIVE's rectangle (aes/gemmnlib.c:717); fired = {fired:#06x}"
    );
    assert!(my < BAR_H && mx >= ACTIVE_W, "the M1 completed on entry, at ({mx},{my})");
    assert_eq!(fired & (MU_MESAG | MU_BUTTON), 0, "nothing else; msg = {msg:?}");
}

/// **A press there is `ctl_pd`'s, and blocks nothing.** Same rest right of
/// the titles, with a click before the 300 idle packets. EmuTOS: the press
/// makes `gl_mowner = ctl_pd` (`chk_ctrl`'s `gl_rmenu` test,
/// `aes/geminput.c:96-97`), the Control Manager's `MU_BUTTON` finds no window
/// (`wm_find` = 0, `aes/gemctrl.c:467-470`) and drops it, the release gives the
/// mouse back (`set_mown`, `:225-231`) with `button == 0` — the application
/// sees no `MU_BUTTON`, no message, and its timer on time. NewTOS parked it in
/// the track for the whole rest.
#[test]
fn a87_a_click_right_of_theactive_is_the_control_managers_and_blocks_nothing() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();

    move_to(host, &mut pb, RIGHT_OF_TITLES.0, RIGHT_OF_TITLES.1, 0);
    let before = bar_and_boxes_pixels(host);

    arm_gap(host);
    click(host);
    dwell(host, 300);
    glide_y(host, 100 - RIGHT_OF_TITLES.1, 0);

    let (fired, bret, msg, elapsed) = evnt_multi_timed_elapsed(host, &mut pb, 100);
    let (mx, my) = tos_api::vdi::mouse_position();
    eprintln!("a87 click right-of-titles: fired={fired:#06x} bret={bret} at=({mx},{my}) msg={msg:?} elapsed={elapsed}");
    assert_eq!(
        fired & MU_BUTTON,
        0,
        "a press in the bar row is ctl_pd's (chk_ctrl: inside gl_rmenu → -1, aes/geminput.c:96-97; \
         bchange, :334-335) and never reaches the application; fired = {fired:#06x}, EV_BRET = {bret}"
    );
    assert_eq!(fired, MU_TIMER, "only the timer; fired = {fired:#06x}, msg = {msg:?}");
    assert!(
        elapsed < 150,
        "the click and the rest right of THEACTIVE parked the application in the track for \
         {elapsed} ms (the timer was 100 ms)"
    );
    assert!(my < BAR_H && mx >= ACTIVE_W, "the pointer is still in the row, at ({mx},{my})");
    assert_eq!(bar_and_boxes_pixels(host), before, "no drop-down / title highlight may have been drawn");
}

/// **Guard — a title still starts the track.** Hover onto *File*, slide onto
/// *Open*, press: one `MN_SELECTED (File, Open)`, no `MU_BUTTON` — the
/// A73/H02 contract, now with `THEACTIVE` narrower than the bar.
#[test]
fn a87_a_title_inside_theactive_still_starts_the_track() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();

    move_to(host, &mut pb, ON_FILE.0, ON_FILE.1, 0);
    assert!(ON_FILE.0 < ACTIVE_W, "File lies inside THEACTIVE");
    glide_y(host, ON_OPEN.1 - ON_FILE.1, 0);
    click(host);

    let (fired, bret, _mb, msg) = evnt_multi_teradesk(&mut pb);
    eprintln!("a87 title track: fired={fired:#06x} bret={bret} msg={msg:?}");
    assert_eq!(fired & MU_MESAG, MU_MESAG, "fired = {fired:#06x}");
    assert_eq!((msg[0], msg[3], msg[4]), (MN_SELECTED, TITLE_FILE, ITEM_OPEN), "msg = {msg:?}");
    assert_eq!(fired & MU_BUTTON, 0, "the selecting press is the Control Manager's; fired = {fired:#06x}");
}

/// **Guard — `THEACTIVE`'s right edge is the boundary.** Resting on the last
/// pixel column of *Edit* (x = 167) starts the track (the *Edit* box comes
/// down and stays until the pointer leaves the bar and the box, then a press
/// outside cancels it — A73 point 2b: consumed, timer only, late); one column
/// further (x = 168) the same rest is the application's: timer on time,
/// nothing drawn.
#[test]
fn a87_theactive_right_edge_is_the_boundary() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();

    // x = ACTIVE_W: first column right of THEACTIVE.
    move_to(host, &mut pb, ACTIVE_W, 8, 0);
    let before = bar_and_boxes_pixels(host);
    dwell(host, 300);
    glide_y(host, 100 - 8, 0);
    let (fired, bret, msg, elapsed) = evnt_multi_timed_elapsed(host, &mut pb, 100);
    eprintln!("a87 edge x={ACTIVE_W}: fired={fired:#06x} bret={bret} msg={msg:?} elapsed={elapsed}");
    assert_eq!(fired, MU_TIMER, "x = {ACTIVE_W} is outside THEACTIVE; fired = {fired:#06x}, msg = {msg:?}");
    assert!(elapsed < 150, "x = {ACTIVE_W} is outside THEACTIVE, no track may park the application ({elapsed} ms)");
    assert_eq!(bar_and_boxes_pixels(host), before, "nothing drawn at x = {ACTIVE_W}");

    // x = ACTIVE_W - 1: last column of Edit — the Control Manager's.
    drain(host, &mut pb);
    drain_messages(&mut pb);
    move_to(host, &mut pb, 300, 100, 0);
    move_to(host, &mut pb, ACTIVE_W - 1, 8, 0); // hover in → the track starts on Edit
    // Down through the Edit box (y 19..57) and out onto the desktop, then a
    // press there: OUTSIDE_STATE cancel, consumed (A73 2b).
    glide_y(host, 200 - 8, 0);
    dwell(host, 3);
    click(host);
    let (fired2, bret2, msg2, elapsed2) = evnt_multi_timed_elapsed(host, &mut pb, 100);
    eprintln!("a87 edge x={}: fired={fired2:#06x} bret={bret2} msg={msg2:?} elapsed={elapsed2}", ACTIVE_W - 1);
    assert_eq!(fired2, MU_TIMER, "x = {} is Edit's last column: the track ran and consumed the cancel; fired = {fired2:#06x}, msg = {msg2:?}", ACTIVE_W - 1);
    assert_ne!(bar_and_boxes_pixels(host), before, "the Edit box must have been down: the track ran at x = {}", ACTIVE_W - 1);
}

/// **Guard — no bar, no hand-over at all.** `menu_bar(…, 0)` (`gl_mntree =
/// NULL`, `gl_ctwait = gl_rmenu`, `aes/gemmnlib.c:726-727`): `mchange` and
/// `ctlmgr` both gate on `gl_mntree` (`aes/geminput.c:388`,
/// `aes/gemctrl.c:451-452`), so entering the row where *File* was completes
/// the application's `MU_M1`, and a rest there costs it nothing. A press
/// there is still `ctl_pd`'s (`gl_rmenu`, `:96-97`): swallowed, timer on time.
#[test]
fn a87_with_no_bar_no_move_hands_the_mouse_over() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();

    pb.intin.fill(0);
    pb.intin[0] = 0; // hide
    pb.addrin[0] = 0;
    pb.call(30, 1, 1, 1); // menu_bar(NULL, 0)
    assert_eq!(pb.intout[0], 1);

    // Enter the row where File was: the M1 completes for the application.
    move_to(host, &mut pb, ON_FILE.0, 60, 0);
    glide_y(host, ON_FILE.1 - 60, 0);
    dwell(host, 3);
    let (fired, _bret, mx, my, msg) = evnt_multi_with_m1(&mut pb, (0, 0, SCREEN_W as i16, BAR_H), Some(400));
    eprintln!("a87 no bar, enter row: fired={fired:#06x} at=({mx},{my}) msg={msg:?}");
    assert_eq!(fired & MU_M1, MU_M1, "no bar: entering the row is the application's; fired = {fired:#06x}");
    assert!(my < BAR_H, "at ({mx},{my})");

    // A click and a rest there: swallowed (gl_rmenu → ctl_pd), timer on time.
    drain(host, &mut pb);
    arm_gap(host);
    click(host);
    dwell(host, 300);
    glide_y(host, 100 - ON_FILE.1, 0);
    let (fired2, bret2, msg2, elapsed) = evnt_multi_timed_elapsed(host, &mut pb, 100);
    eprintln!("a87 no bar, click: fired={fired2:#06x} bret={bret2} msg={msg2:?} elapsed={elapsed}");
    assert_eq!(fired2, MU_TIMER, "no bar: a press in the row is ctl_pd's and dropped; fired = {fired2:#06x}, msg = {msg2:?}");
    assert!(elapsed < 150, "no bar: nothing may park the application ({elapsed} ms)");

    // Re-install for the cases that follow in this process.
    drain(host, &mut pb);
    pb.intin.fill(0);
    pb.intin[0] = 1;
    pb.addrin[0] = menu_tree() as u64;
    pb.call(30, 1, 1, 1);
    pb.addrin[0] = 0;
    assert_eq!(pb.intout[0], 1);
}

// ── A88: menu_popup through the AES call (pool-less branch) ────────────────

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

/// One `menu_popup(36)` call with the pop-up's start item `item` at
/// `(x, y)`; returns `(rc, out MENU)`.
fn menu_popup(pb: &mut Aespb, tree: *mut Obj, item: i16, x: i16, y: i16) -> (i16, MenuRec) {
    let mut min = Box::new(MenuRec { mn_tree: tree as u64, mn_menu: 0, mn_item: item, mn_scroll: 7, mn_keystate: 0x55 });
    let mut mout = Box::new(MenuRec { mn_tree: 0, mn_menu: 0, mn_item: -1, mn_scroll: 0, mn_keystate: 0x55 });
    pb.intin.fill(0);
    pb.intin[0] = x; // MPOP_XPOS
    pb.intin[1] = y; // MPOP_YPOS
    pb.addrin[0] = &mut *min as *mut MenuRec as u64;
    pb.addrin[1] = &mut *mout as *mut MenuRec as u64;
    pb.call(36, 2, 1, 2);
    pb.addrin[0] = 0;
    pb.addrin[1] = 0;
    (pb.intout[0], *mout)
}

/// **`clamp_ypos` through the call, and `mn_keystate` on no pick.** The
/// buttons are up, so `mn_popup`'s first wait returns at once; the box is
/// positioned and clamped (`gemmnext.c:320-324`) *before* `popup_blit`. The
/// host has no pool, but since A89 `popup_blit_save` keeps the pixels on the
/// heap there (as `bb_save` does without a screen buffer), so the pop-up
/// **is** tracked: each call gets a click with the pointer parked outside
/// the box — `handle_popup` returns on the press with nothing under it
/// (`:290-291`), the no-pick branch: `rc = 0`, `mn_keystate = 0`
/// (`:345-349`). The box's `ob_y` in the caller's tree shows the clamp:
///
/// * start item 2 (*Beta*, ob_y 19) requested at y = 0 → box −19 → pushed
///   below the bar row in `gl_hchar` steps: −3, 13, **29**;
/// * requested at y = 479 (screen bottom) → box 460 → pulled up until it
///   fits (`480 − 57 = 423`): 444, 428, **412**;
/// * x is `xpos` as given — EmuTOS does not clamp it (`ob_x = xpos`, `:322`).
#[test]
fn a88_menu_popup_clamps_the_box_below_the_bar_and_onto_the_screen() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    let tree = popup_tree();
    // SAFETY: `tree` is the leaked 4-object array above.
    let box_y = |t: *mut Obj| unsafe { (*t).y };
    let box_x = |t: *mut Obj| unsafe { (*t).x };

    // The pointer rests at (320,300), outside both boxes below: a click there
    // cancels each pop-up (consumed inside the call, gemmnext.c:290-291,351).
    click(host);
    let (rc, out) = menu_popup(&mut pb, tree, 2, 600, 0);
    eprintln!("a88 y=0: rc={rc} box=({},{}) out={out:?}", box_x(tree), box_y(tree));
    assert_eq!(rc, 0, "a press outside the box → the no-pick branch");
    assert_eq!(out.mn_keystate, 0, "TOS always sets mn_keystate — 0 on no pick (gemmnext.c:347); out = {out:?}");
    assert_eq!(box_y(tree), 29, "start item 2 at y = 0: box −19 → −3 → 13 → 29 (clamp_ypos, gemmnext.c:124-126)");
    assert_eq!(box_x(tree), 600, "x is not clamped (ob_x = xpos, gemmnext.c:322)");

    drain(host, &mut pb);
    click(host);
    let (rc, out) = menu_popup(&mut pb, tree, 2, 100, SCREEN_H as i16 - 1);
    eprintln!("a88 y=bottom: rc={rc} box=({},{}) out={out:?}", box_x(tree), box_y(tree));
    assert_eq!(rc, 0);
    assert_eq!(out.mn_keystate, 0, "out = {out:?}");
    assert_eq!(box_y(tree), 412, "start item 2 at y = 479: box 460 → 444 → 428 → 412 (clamp_ypos, gemmnext.c:119-121)");
    assert_eq!(box_x(tree), 100);

    drain(host, &mut pb);
}
