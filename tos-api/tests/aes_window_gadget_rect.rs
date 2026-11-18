//! A77 — **`WM_CLOSED` / `WM_FULLED` carry the window's `WS_CURR` rect in
//! `msg[4..7]`.**
//!
//! EmuTOS's Control Manager fetches the clicked window's current border rect
//! once, before the gadget switch, and every message it posts from there goes
//! out with those four words unless the arm overwrote them:
//!
//! ```c
//! w_getsize(WS_CURR, w_handle, &t);                  /* aes/gemctrl.c:196 */
//! r_get(&t, &x, &y, &w, &h);                         /* aes/gemctrl.c:197 */
//! kind = pwin->w_kind;
//! ...
//! switch(cpt)
//! {
//! case W_CLOSER:
//! #if CONF_WITH_PCGEM
//!     if (kind & HOTCLOSE)
//!     {
//!         message = WM_CLOSED;                       /* aes/gemctrl.c:212 */
//!         break;
//!     }
//!     FALLTHROUGH;
//! #endif
//! case W_FULLER:
//!     if (gr_watchbox(gl_awind, gadget, selected, normal))
//!         message = (cpt == W_CLOSER) ? WM_CLOSED : WM_FULLED;  /* :220 */
//!     break;
//! ...
//! }
//! ...
//! ct_msgup(message, pwin->w_owner, w_handle, x, y, w, h);  /* aes/gemctrl.c:314 */
//! ```
//!
//! The closer and fuller arms leave `x, y, w, h` alone, so `WM_CLOSED` (both
//! the `gr_watchbox` form and the `HOTCLOSE` press form) and `WM_FULLED`
//! deliver `msg[4] = x, msg[5] = y, msg[6] = w, msg[7] = h` of `WS_CURR`.
//! NewTOS posted `0, 0, 0, 0` for all three.
//!
//! Three probes, driven exactly as the board drives the AES (a scripted host
//! whose `pump_input` is the only thing that advances the pointer):
//!
//! * [`hotclose_wm_closed_carries_ws_curr`] — `CLOSER | HOTCLOSE`, press on
//!   the closer: the `WM_CLOSED` `msg[4..7]` equals `wind_get(WF_CURRXYWH)`;
//! * [`plain_closer_wm_closed_carries_ws_curr`] — `CLOSER` alone, press and
//!   release inside: same words on the `gr_watchbox` form;
//! * [`fuller_wm_fulled_carries_ws_curr`] — `FULLER`, press and release inside:
//!   `WM_FULLED` with the same four words.
//!
//! Each probe first moves the window off its `wind_open` rect with
//! `wind_set(WF_CURRXYWH)` so that the expected words are neither the
//! creation rect nor zero, and `wind_get(WF_CURRXYWH)` is the oracle.
//!
//! Harness (raster stand-in + scripted host + parameter block + helpers)
//! copied from `aes_window_hotclose.rs` (A49).

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
/// (`ACTIVE_PALETTE`, `kernel/src/vdi_backend.rs:11-21`).
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

// ── AES constants ──────────────────────────────────────────────────────────

// window kinds (`include/aesdefs.h:519-531`)
const NAME: i16 = 0x0001;
const CLOSER: i16 = 0x0002;
const FULLER: i16 = 0x0004;
/// `HOTCLOSE 0x1000 /* PC-GEM feature */` (`include/aesdefs.h:531`).
const HOTCLOSE: i16 = 0x1000;

const MU_MESAG: i16 = 0x0010;
const MU_TIMER: i16 = 0x0020;
const WM_CLOSED: i16 = 22;
const WM_FULLED: i16 = 23;
/// `WF_CXYWH` = `WF_CURRXYWH` (`include/aesdefs.h:538`).
const WF_CURRXYWH: i16 = 5;

/// The window under test, in border coordinates, as `wind_open` gets it.
const WX: i16 = 120;
const WY: i16 = 80;
const WW: i16 = 320;
const WH: i16 = 220;
/// Where each probe moves the window afterwards (`wind_set(WF_CURRXYWH)`), so
/// the expected words are neither the `wind_open` rect nor zero.
const MOVED: (i16, i16, i16, i16) = (176, 118, 296, 206);
/// Empty desktop, well away from the window in both positions.
const PARK: (i16, i16) = (40, 420);

/// The installed host, window table and AES state are process-wide statics,
/// so the cases in this binary run one at a time.
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

/// Fresh application state: every window closed (`wind_new`), the pointer
/// shown and parked on the desktop with the buttons up, nothing queued.
fn fresh() -> (&'static ScriptHost, Aespb) {
    let host = rig();
    host.clear();
    let mut pb = Aespb::new();
    pb.call(10, 0, 1, 0); // appl_init
    pb.intin.fill(0);
    pb.call(109, 0, 1, 0); // wind_new: closes every window, drains the update locks

    pb.intin.fill(0);
    pb.intin[0] = 257; // graf_mouse(M_ON)
    pb.call(78, 1, 1, 0);

    move_to(host, &mut pb, PARK.0, PARK.1, 0);
    drain_messages(&mut pb);
    (host, pb)
}

/// `wind_create` + `wind_open` at the test rect, returning the handle.
fn open_window(pb: &mut Aespb, kind: i16) -> i16 {
    pb.intin.fill(0);
    pb.intin[0] = kind;
    pb.intin[1] = 0;
    pb.intin[2] = 0;
    pb.intin[3] = SCREEN_W as i16;
    pb.intin[4] = SCREEN_H as i16;
    pb.call(100, 5, 1, 0); // wind_create
    let wh = pb.intout[0];
    assert!(wh > 0, "wind_create must give a handle, got {wh}");

    pb.intin.fill(0);
    pb.intin[0] = wh;
    pb.intin[1] = WX;
    pb.intin[2] = WY;
    pb.intin[3] = WW;
    pb.intin[4] = WH;
    pb.call(101, 5, 1, 0); // wind_open
    assert_eq!(pb.intout[0], 1, "wind_open must succeed");
    wh
}

/// `wind_get(handle, field)` → `(x, y, w, h)` (`aes_window_drag.rs`).
fn wind_get_rect(pb: &mut Aespb, handle: i16, field: i16) -> (i16, i16, i16, i16) {
    pb.intin.fill(0);
    pb.intin[0] = handle;
    pb.intin[1] = field;
    pb.call(104, 2, 5, 0);
    (pb.intout[1], pb.intout[2], pb.intout[3], pb.intout[4])
}

/// `wind_set(handle, WF_CURRXYWH, x, y, w, h)`.
fn wind_set_curr(pb: &mut Aespb, handle: i16, r: (i16, i16, i16, i16)) {
    pb.intin.fill(0);
    pb.intin[0] = handle;
    pb.intin[1] = WF_CURRXYWH;
    pb.intin[2] = r.0;
    pb.intin[3] = r.1;
    pb.intin[4] = r.2;
    pb.intin[5] = r.3;
    pb.call(105, 6, 1, 0);
    assert_eq!(pb.intout[0], 1, "wind_set(WF_CURRXYWH) must succeed");
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

/// One button packet at the current position, consumed at once (the VDI's
/// `mouse_int` → `b_click`, `feed_relative`): `buttons` = 1 is the press,
/// 0 the release.
fn button_here(host: &'static ScriptHost, pb: &mut Aespb, buttons: u8) {
    host.push(Packet { dx: 0, dy: 0, buttons });
    graf_mkstate(pb);
    assert_eq!(tos_api::vdi::mouse_buttons() & 1, buttons & 1, "the button packet must be live");
}

/// `evnt_multi(MU_MESAG | MU_TIMER(0))` — a poll that always returns: the
/// Control Manager runs on the entry pass, a queued message is dequeued, the
/// zero timer fires. Returns `(fired, msg)`. Never arms `MU_BUTTON`, so a held
/// button cannot satisfy it.
fn evnt_multi_mesag_poll(pb: &mut Aespb) -> (i16, [i16; 8]) {
    let mut buf = Box::new([0i16; 8]);
    pb.intin.fill(0);
    pb.intin[0] = MU_MESAG | MU_TIMER;
    pb.intin[14] = 0;
    pb.intin[15] = 0;
    pb.addrin[0] = buf.as_mut_ptr() as u64;
    pb.call(25, 16, 7, 1);
    pb.addrin[0] = 0;
    (pb.intout[0], *buf)
}

/// Empty the owner FIFO (the `WM_REDRAW`s `wind_open` and `wind_set` post,
/// and anything else).
fn drain_messages(pb: &mut Aespb) {
    for _ in 0..16 {
        let (fired, _) = evnt_multi_mesag_poll(pb);
        if fired & MU_MESAG == 0 {
            return;
        }
    }
}

/// Open a window of `kind` at the `wind_open` rect, move it to [`MOVED`] the
/// way an owner does (`wind_set(WF_CURRXYWH)`), empty the pipe and read back
/// the current border rect through `wind_get(WF_CURRXYWH)` — the oracle for
/// `msg[4..7]`. Returns `(handle, ws_curr)`.
fn open_and_move(pb: &mut Aespb, kind: i16) -> (i16, (i16, i16, i16, i16)) {
    let wh = open_window(pb, kind);
    drain_messages(pb);
    wind_set_curr(pb, wh, MOVED);
    drain_messages(pb);
    let curr = wind_get_rect(pb, wh, WF_CURRXYWH);
    assert_eq!(curr, MOVED, "wind_get(WF_CURRXYWH) must report the moved rect");
    assert_ne!(curr, (0, 0, 0, 0), "the oracle rect must not be the zero rect");
    (wh, curr)
}

/// A point inside the closer cell: the `gl_wbox` × `gl_hbox` gadget inset
/// `ADJ3DSTD` (2) from the border's top-left corner (`draw_frame`, the layout
/// `gadget_at` hit-tests against).
fn closer_point(curr: (i16, i16, i16, i16)) -> (i16, i16) {
    (curr.0 + 8, curr.1 + 8)
}

/// A point inside the fuller cell: the same-sized gadget at the right end of
/// the title band (`gad_rect(Fuller)`: `x + w - adj_wbox() + ADJ3DSTD`).
fn fuller_point(curr: (i16, i16, i16, i16)) -> (i16, i16) {
    (curr.0 + curr.2 - 8, curr.1 + 8)
}

/// Put the pointer on `(px, py)` with the buttons up and check it sits on the
/// named gadget of `wh`; empty the pipe afterwards.
fn on_gadget(host: &'static ScriptHost, pb: &mut Aespb, wh: i16, name: &str, (px, py): (i16, i16)) {
    move_to(host, pb, px, py, 0);
    assert_eq!(
        tos_api::aes::window::gadget_name_at(wh, px, py),
        name,
        "the pointer must sit on the {name} of window {wh}"
    );
    drain_messages(pb);
}

/// The four rect words of `msg`.
fn rect_of(msg: &[i16; 8]) -> (i16, i16, i16, i16) {
    (msg[4], msg[5], msg[6], msg[7])
}

/// **Probe 1 — `HOTCLOSE`, the press form.** `CLOSER | HOTCLOSE`: the press
/// on the closer delivers `WM_CLOSED` at once (`aes/gemctrl.c:208-214`, A49)
/// and its `msg[4..7]` are the window's `WS_CURR` `x, y, w, h`
/// (`:196-197`, `:314`).
#[test]
fn hotclose_wm_closed_carries_ws_curr() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    let (wh, curr) = open_and_move(&mut pb, NAME | CLOSER | HOTCLOSE);
    on_gadget(host, &mut pb, wh, "closer", closer_point(curr));

    button_here(host, &mut pb, 1);
    let (fired, msg) = evnt_multi_mesag_poll(&mut pb);
    assert_eq!(
        (fired & MU_MESAG, msg[0], msg[3]),
        (MU_MESAG, WM_CLOSED, wh),
        "the press on a HOTCLOSE closer must deliver WM_CLOSED for the window; \
         fired = {fired:#06x}, msg = {msg:?}"
    );
    assert_eq!(
        rect_of(&msg),
        curr,
        "WM_CLOSED (HOTCLOSE form) must carry the window's WS_CURR rect in \
         msg[4..7] (aes/gemctrl.c:196-197 r_get(WS_CURR), :314 ct_msgup(…, x, y, w, h)); \
         msg = {msg:?}"
    );

    // Tidy: the physical release.
    button_here(host, &mut pb, 0);
    drain_messages(&mut pb);
}

/// **Probe 2 — `CLOSER` alone, the `gr_watchbox` form.** Press and release
/// inside the closer: `WM_CLOSED` (`aes/gemctrl.c:216-220`) with the same
/// four `WS_CURR` words.
#[test]
fn plain_closer_wm_closed_carries_ws_curr() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    let (wh, curr) = open_and_move(&mut pb, NAME | CLOSER);
    on_gadget(host, &mut pb, wh, "closer", closer_point(curr));

    button_here(host, &mut pb, 1);
    host.push(Packet { dx: 0, dy: 0, buttons: 1 });
    host.push(Packet { dx: 0, dy: 0, buttons: 0 });
    let (fired, msg) = evnt_multi_mesag_poll(&mut pb);
    assert_eq!(
        (fired & MU_MESAG, msg[0], msg[3]),
        (MU_MESAG, WM_CLOSED, wh),
        "a plain CLOSER released inside must fire WM_CLOSED; fired = {fired:#06x}, msg = {msg:?}"
    );
    assert_eq!(
        rect_of(&msg),
        curr,
        "WM_CLOSED (gr_watchbox form) must carry the window's WS_CURR rect in \
         msg[4..7] (aes/gemctrl.c:196-197, :314); msg = {msg:?}"
    );
    drain(host, &mut pb);
    drain_messages(&mut pb);
}

/// **Probe 3 — `FULLER`.** Press and release inside the fuller: `WM_FULLED`
/// (`aes/gemctrl.c:217-220`) with the `WS_CURR` words — the same `x, y, w, h`
/// the closer arm inherits, the fuller arm does not touch them either.
#[test]
fn fuller_wm_fulled_carries_ws_curr() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    let (wh, curr) = open_and_move(&mut pb, NAME | CLOSER | FULLER);
    on_gadget(host, &mut pb, wh, "fuller", fuller_point(curr));

    button_here(host, &mut pb, 1);
    host.push(Packet { dx: 0, dy: 0, buttons: 1 });
    host.push(Packet { dx: 0, dy: 0, buttons: 0 });
    let (fired, msg) = evnt_multi_mesag_poll(&mut pb);
    assert_eq!(
        (fired & MU_MESAG, msg[0], msg[3]),
        (MU_MESAG, WM_FULLED, wh),
        "a FULLER released inside must fire WM_FULLED; fired = {fired:#06x}, msg = {msg:?}"
    );
    assert_eq!(
        rect_of(&msg),
        curr,
        "WM_FULLED must carry the window's WS_CURR rect in msg[4..7] \
         (aes/gemctrl.c:196-197, :314); msg = {msg:?}"
    );
    drain(host, &mut pb);
    drain_messages(&mut pb);
}
