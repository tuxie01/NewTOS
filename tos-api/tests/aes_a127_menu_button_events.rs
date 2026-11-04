//! A127 D1 — the drop-down track takes the left button as **events**, not as
//! a sample of the level once per pass.
//!
//! **Contract** (`docs/api/aes/menu.md`, "Mouse ownership over the track",
//! point 9): each round of `mn_do` waits with `ev_multi(MU_BUTTON | …,
//! buparm)` (`aes/gemmnlib.c:356`); the wait is completed by the first
//! published transition it accepts (`post_button`, `aes/geminput.c:463-483`),
//! or answered at once by the quick check on the recorded state
//! (`(mtrans > 1) && downorup(pr_button, buparm)`, `aes/gemevlib.c:190`). So a
//! press and its release that arrive between two passes of the tracker — two
//! mouse reports decoded in one `pump_input` — act exactly like a slow click:
//! `INITEM_STATE` selects, `OUTSIDE_STATE` cancels, `INTITLE_STATE` flips
//! `buparm` twice and nothing shows. Before A127 D1 `menu::track` compared
//! `vdi::mouse_buttons()` once per pass, and such a click was lost: the menu
//! stayed open (`measurements/measurement-2026-11-04-a127/RUNINFO.TXT`,
//! run C).
//!
//! Harness copied from `aes_menu_click_ownership.rs`; one script entry per
//! `pump_input`, an entry may carry several packets (`push_burst`).

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
    /// One entry per `pump_input`: every packet in it is fed in that one pump.
    script: Mutex<VecDeque<Vec<Packet>>>,
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
        self.script.lock().unwrap().push_back(vec![p]);
    }
    /// Several packets decoded in **one** `pump_input` — two mouse reports
    /// between two passes of the tracker (the A127 run C case).
    fn push_burst(&self, ps: &[Packet]) {
        self.script.lock().unwrap().push_back(ps.to_vec());
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
            Some(ps) => {
                self.idle.store(0, Ordering::Relaxed);
                for p in ps {
                    tos_api::vdi::feed_relative(p.dx, p.dy, p.buttons, &self.raster);
                }
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
/// The tree [`fresh`] installed with `menu_bar` (ZVIEW04).
static LAST_TREE: AtomicUsize = AtomicUsize::new(0);

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
    let tree = menu_tree();
    LAST_TREE.store(tree as usize, Ordering::Relaxed); // ZVIEW04: for msg[5..6]
    pb.addrin[0] = tree as u64;
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

/// The same wait with a zero-millisecond timer: a poll that always returns.
fn evnt_multi_poll(pb: &mut Aespb) -> (i16, i16, [i16; 8]) {
    let mut buf = Box::new([0i16; 8]);
    pb.intin.fill(0);
    pb.intin[0] = MU_MESAG | MU_BUTTON | MU_KEYBD | MU_TIMER;
    pb.intin[1] = 0x0102;
    pb.intin[2] = 3;
    pb.intin[3] = 0;
    pb.intin[14] = 0;
    pb.intin[15] = 0;
    pb.addrin[0] = buf.as_mut_ptr() as u64;
    pb.call(25, 16, 7, 1);
    pb.addrin[0] = 0;
    (pb.intout[0], pb.intout[6], *buf)
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

/// Pointer positions: over the *File* title, its *Open* item, and no man's
/// land right of the *File* box (below the bar, outside every box).
const ON_FILE: (i16, i16) = (FILE_X + 28, 8);
const ON_OPEN: (i16, i16) = (FILE_X + 28, BAR_H + ITEM_H + 9);
const NO_MANS_LAND: (i16, i16) = (300, BAR_H + ITEM_H + 9);
const ON_QUIT: (i16, i16) = (FILE_X + 28, BAR_H + 2 * ITEM_H + 9);

/// The *File* drop-down box.
const FILE_BOX: i16 = 10;
const SELECTED: u16 = 0x0001;

fn state_of(obj: i16) -> u16 {
    let tree = LAST_TREE.load(Ordering::Relaxed) as *const Obj;
    // SAFETY: `fresh` leaked the tree; `obj` is one of its 17 records.
    unsafe { (*tree.add(obj as usize)).state }
}

/// Queue a walk from the current scripted position `from` to `to`, buttons
/// `b`, one packet per pump, at most 10 px per axis and packet.
fn walk(host: &ScriptHost, from: (i16, i16), to: (i16, i16), b: u8) {
    let (mut x, mut y) = from;
    while (x, y) != to {
        let dx = (to.0 - x).clamp(-10, 10);
        let dy = (to.1 - y).clamp(-10, 10);
        host.push(Packet { dx, dy, buttons: b });
        x += dx;
        y += dy;
    }
}

fn idle(host: &ScriptHost, n: usize) {
    for _ in 0..n {
        host.push(Packet { dx: 0, dy: 0, buttons: 0 });
    }
}

/// The selecting click, fast or slow: `true` = press and release in **one**
/// pump, `false` = press, three held packets, release.
fn click(host: &ScriptHost, fast: bool) {
    if fast {
        host.push_burst(&[Packet { dx: 0, dy: 0, buttons: 1 }, Packet { dx: 0, dy: 0, buttons: 0 }]);
    } else {
        for _ in 0..4 {
            host.push(Packet { dx: 0, dy: 0, buttons: 1 });
        }
        host.push(Packet { dx: 0, dy: 0, buttons: 0 });
    }
}

fn assert_mn_selected_open(fired: i16, msg: &[i16; 8], case: &str) {
    assert_eq!(fired & MU_MESAG, MU_MESAG, "{case}: the pick must arrive as MU_MESAG; fired = {fired:#06x}");
    assert_eq!(
        (msg[0], msg[3], msg[4]),
        (MN_SELECTED, TITLE_FILE, ITEM_OPEN),
        "{case}: MN_SELECTED (File, Open) expected; msg = {msg:?}"
    );
    assert_eq!(
        fired & MU_BUTTON,
        0,
        "{case}: the selecting click leaked to the application as MU_BUTTON; fired = {fired:#06x}"
    );
    assert_eq!(state_of(ITEM_OPEN) & SELECTED, 0, "{case}: the picked item's SELECTED bit is cleared (gemmnlib.c:493)");
}

/// After the pick: the consumed press and release reach the application's
/// next wait neither as `MU_BUTTON` nor as a second message.
fn assert_nothing_after(host: &'static ScriptHost, pb: &mut Aespb, case: &str) {
    drain(host, pb);
    let (fired, bret, msg) = evnt_multi_poll(pb);
    assert_eq!(
        fired, MU_TIMER,
        "{case}: only the 0-ms timer may fire after the pick; fired = {fired:#06x} bret = {bret} msg = {msg:?}"
    );
}

/// **INITEM, button up → select on the press.** Hover *File*, slide onto
/// *Open*, click. The fast click (press + release in one pump) must select
/// like the slow one.
fn initem_press(fast: bool) {
    let (host, mut pb) = fresh();
    move_to(host, &mut pb, ON_FILE.0, ON_FILE.1, 0);
    walk(host, ON_FILE, ON_OPEN, 0);
    host.push(Packet { dx: 0, dy: 0, buttons: 0 }); // dwell: INITEM armed with the button up
    click(host, fast);
    idle(host, 8);
    let case = if fast { "INITEM fast click" } else { "INITEM slow click" };
    let (fired, _bret, _mb, msg) = evnt_multi_teradesk(&mut pb);
    eprintln!("{case}: fired={fired:#06x} msg={msg:?}");
    assert_mn_selected_open(fired, &msg, case);
    assert_nothing_after(host, &mut pb, case);
}

#[test]
fn initem_fast_click_selects_like_a_slow_click() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    initem_press(false);
    initem_press(true);
}

/// **INITEM, button held → select on the release.** Press on *File*, drag
/// onto *Open* held; then the release and a fresh press arrive in **one**
/// pump (the second report already down again). `buparm` was armed for the
/// release (`gemmnlib.c:314`), so the release selects *Open*; the level after
/// the pump (down) is the same as before it, which a per-pass sample could
/// not tell from "nothing happened" — it went on with the button held onto
/// *Quit* and picked that at the final release.
#[test]
fn initem_held_release_and_repress_in_one_pump_selects() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    move_to(host, &mut pb, ON_FILE.0, ON_FILE.1, 0);
    host.push(Packet { dx: 0, dy: 0, buttons: 0 });
    host.push(Packet { dx: 0, dy: 0, buttons: 1 }); // press on the title
    host.push(Packet { dx: 0, dy: 0, buttons: 1 });
    walk(host, ON_FILE, ON_OPEN, 1);
    host.push(Packet { dx: 0, dy: 0, buttons: 1 }); // dwell held: INITEM armed for the release
    host.push_burst(&[Packet { dx: 0, dy: 0, buttons: 0 }, Packet { dx: 0, dy: 0, buttons: 1 }]);
    walk(host, ON_OPEN, ON_QUIT, 1); // held onto *Quit*
    host.push(Packet { dx: 0, dy: 0, buttons: 0 });
    idle(host, 8);
    let case = "INITEM held, release+press in one pump";
    let (fired, _bret, _mb, msg) = evnt_multi_teradesk(&mut pb);
    eprintln!("{case}: fired={fired:#06x} msg={msg:?}");
    assert_mn_selected_open(fired, &msg, case);
    assert_nothing_after(host, &mut pb, case);
}

/// **OUTSIDE → cancel.** Hover *File*, onto *Open*, out to no man's land
/// right of the box (the box stays down, `OUTSIDE_STATE`), click. Fast and
/// slow must both cancel: no message, the title and the item un-hilited,
/// and the application's own button wait sees nothing of the click.
fn outside_cancel(fast: bool) {
    let (host, mut pb) = fresh();
    move_to(host, &mut pb, ON_FILE.0, ON_FILE.1, 0);
    walk(host, ON_FILE, ON_OPEN, 0);
    walk(host, ON_OPEN, NO_MANS_LAND, 0);
    host.push(Packet { dx: 0, dy: 0, buttons: 0 }); // dwell: OUTSIDE armed
    click(host, fast);
    idle(host, 8);
    let case = if fast { "OUTSIDE fast click" } else { "OUTSIDE slow click" };
    // MU_MESAG | MU_BUTTON | MU_TIMER(400 ms): the track runs inline, ends
    // on the cancel, then only the timer may end the call.
    let mut buf = Box::new([0i16; 8]);
    pb.intin.fill(0);
    pb.intin[0] = MU_MESAG | MU_BUTTON | MU_TIMER;
    pb.intin[1] = 0x0102;
    pb.intin[2] = 3;
    pb.intin[3] = 0;
    pb.intin[14] = 400;
    pb.intin[15] = 0;
    pb.addrin[0] = buf.as_mut_ptr() as u64;
    pb.call(25, 16, 7, 1);
    pb.addrin[0] = 0;
    let fired = pb.intout[0];
    eprintln!("{case}: fired={fired:#06x} msg={buf:?} left={}", host.script.lock().unwrap().len());
    assert_eq!(fired, MU_TIMER, "{case}: the cancel posts nothing and the click is not the application's; fired = {fired:#06x} msg = {buf:?}");
    assert!(host.script.lock().unwrap().is_empty(), "{case}: the track ended on the click and the call ran out the script");
    assert_eq!(state_of(TITLE_FILE) & SELECTED, 0, "{case}: the cancel un-hilites the title (gemmnlib.c:509-510)");
    assert_eq!(state_of(ITEM_OPEN) & SELECTED, 0, "{case}: no item stays hilited");
    assert_eq!(state_of(FILE_BOX) & SELECTED, 0, "{case}: the box is not selected");
}

#[test]
fn outside_fast_click_cancels_like_a_slow_click() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    outside_cancel(false);
    outside_cancel(true);
}

/// **INTITLE → nothing, the wait flips twice.** A click on the *File* title
/// itself selects and cancels nothing (`buparm ^= 1`, `gemmnlib.c:375`, once
/// for the press and once for the release); the box stays down, and a
/// following click on *Open* selects — within the same `evnt_multi`, so the
/// track never ended in between.
fn intitle_then_item(fast: bool) {
    let (host, mut pb) = fresh();
    move_to(host, &mut pb, ON_FILE.0, ON_FILE.1, 0);
    host.push(Packet { dx: 0, dy: 0, buttons: 0 }); // INTITLE armed
    click(host, fast); // on the title
    idle(host, 2);
    walk(host, ON_FILE, ON_OPEN, 0);
    host.push(Packet { dx: 0, dy: 0, buttons: 0 });
    click(host, false); // a slow click on *Open*
    idle(host, 8);
    let case = if fast { "INTITLE fast click, then item" } else { "INTITLE slow click, then item" };
    let (fired, _bret, _mb, msg) = evnt_multi_teradesk(&mut pb);
    eprintln!("{case}: fired={fired:#06x} msg={msg:?}");
    assert_mn_selected_open(fired, &msg, case);
    assert_nothing_after(host, &mut pb, case);
}

#[test]
fn intitle_fast_click_flips_like_a_slow_click() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    intitle_then_item(false);
    intitle_then_item(true);
}
