//! A49 — **`HOTCLOSE` (0x1000): the closer fires `WM_CLOSED` on the press.**
//!
//! With `CONF_WITH_PCGEM` (1 in the 512 profile, `include/config.h:1320`; only
//! `TARGET_192` turns it off, `:353`) the Control Manager treats a window whose
//! `kind` carries `HOTCLOSE` (`include/aesdefs.h:531`) differently at the
//! closer — no `gr_watchbox`, no wait for the release:
//!
//! ```c
//! case W_CLOSER:                                     /* aes/gemctrl.c:208 */
//! #if CONF_WITH_PCGEM
//!     if (kind & HOTCLOSE)
//!     {
//!         message = WM_CLOSED;
//!         break;
//!     }
//!     FALLTHROUGH;
//! #endif
//! case W_FULLER:
//!     if (gr_watchbox(gl_awind, gadget, selected, normal))
//!         message = (cpt == W_CLOSER) ? WM_CLOSED : WM_FULLED;
//! ```
//!
//! and `ct_msgup`, having posted it, does not park in its `while (button &
//! 0x0001)` wait either but **fakes the button up** so the still-held press
//! cannot re-trigger the Control Manager or reach the application as a press:
//!
//! ```c
//! if ((message == WM_CLOSED) && (D.w_win[wh].w_kind & HOTCLOSE))
//! {                                                  /* aes/gemctrl.c:90 */
//!     button = 0;     /* we fake button up, otherwise a slow click will */
//!     return;         /*  cause us to stay in ctlmgr(), eating keys     */
//! }
//! ```
//!
//! Three probes, driven exactly as the board drives the AES (a scripted host
//! whose `pump_input` is the only thing that advances the pointer):
//!
//! * [`hotclose_press_delivers_wm_closed_before_the_release`] — `CLOSER |
//!   HOTCLOSE`, pointer on the closer, press: `evnt_multi` answers `WM_CLOSED`
//!   without consuming a single further packet;
//! * [`hotclose_press_is_consumed_and_the_closer_rearms_on_the_next_press`] —
//!   the `button = 0` half: the held press is neither a second `WM_CLOSED` nor
//!   a `MU_BUTTON` for a TeraDesk-style wait; after the release a fresh press
//!   fires again;
//! * [`plain_closer_fires_only_after_the_release_inside`] — regression: with
//!   `CLOSER` alone the message waits for the release inside the gadget
//!   (`gr_watchbox`, `docs/api/aes/window-graphics.md` § `graf_watchbox`).
//!
//! Harness (raster stand-in + scripted host + parameter block) copied from
//! `aes_menu_click_ownership.rs`; `open_window` from `aes_window_drag.rs`.

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
/// `HOTCLOSE 0x1000 /* PC-GEM feature */` (`include/aesdefs.h:531`).
const HOTCLOSE: i16 = 0x1000;

const MU_KEYBD: i16 = 0x0001;
const MU_BUTTON: i16 = 0x0002;
const MU_MESAG: i16 = 0x0010;
const MU_TIMER: i16 = 0x0020;
const WM_CLOSED: i16 = 22;

/// The window under test, in border coordinates.
const WX: i16 = 120;
const WY: i16 = 80;
const WW: i16 = 320;
const WH: i16 = 220;
/// A point inside the closer cell: the `gl_wbox` × `gl_hbox` gadget inset
/// `ADJ3DSTD` (2) from the border's top-left corner (`draw_frame`, the layout
/// `gadget_at` hit-tests against).
const ON_CLOSER: (i16, i16) = (WX + 8, WY + 8);
/// Empty desktop, well away from the window.
const PARK: (i16, i16) = (320, 400);

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

/// TeraDesk's main-loop wait as a poll: `MU_MESAG | MU_BUTTON | MU_KEYBD`,
/// `ev_mbclicks = 0x102` (bflag 1 = wait for a **press**, up to 2 clicks),
/// `ev_mbmask = 3`, `ev_mbstate = 0` (`sdk/apps/teradesk/main.c:1588-1610`),
/// plus a zero-millisecond `MU_TIMER` so the call always comes back. Returns
/// `(fired, EV_BRET, msg)`.
fn evnt_multi_teradesk_poll(pb: &mut Aespb) -> (i16, i16, [i16; 8]) {
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

/// Empty the owner FIFO (the `WM_REDRAW` `wind_open` posts, and anything else).
fn drain_messages(pb: &mut Aespb) {
    for _ in 0..16 {
        let (fired, _) = evnt_multi_mesag_poll(pb);
        if fired & MU_MESAG == 0 {
            return;
        }
    }
}

/// Open a window of `kind`, put the pointer on its closer with the buttons up
/// and empty the pipe. Returns the handle.
fn on_closer(host: &'static ScriptHost, pb: &mut Aespb, kind: i16) -> i16 {
    let wh = open_window(pb, kind);
    drain_messages(pb);
    move_to(host, pb, ON_CLOSER.0, ON_CLOSER.1, 0);
    assert_eq!(
        tos_api::aes::window::gadget_name_at(wh, ON_CLOSER.0, ON_CLOSER.1),
        "closer",
        "the pointer must sit on the closer of window {wh}"
    );
    drain_messages(pb);
    wh
}

/// **Probe 1 — the press alone.** `CLOSER | HOTCLOSE`: the press is followed
/// by four more packets (three held, one release) that stay *queued*; the very
/// next `evnt_multi` must already answer `WM_CLOSED` for the window without
/// waiting on any of them, i.e. before the release (`gemctrl.c:208-214`:
/// `message = WM_CLOSED; break;`, no `gr_watchbox`). `ev_multi` opens with
/// `chkkbd(); forker();` (`aes/gemevlib.c:169-170`) — one `pump_input` at
/// the head of `evnt_multi` (A92) — and every AES call ends with one
/// `dsptch()` → one `pump_input` (`aes/gemsuper.c:452-464`,
/// `tos-api/src/aes/mod.rs` `dsptch`), so a call that fired on its entry pass
/// consumes exactly **two** packets (both held ones) — two of the four
/// remain, the release among them. A wait for the release would have pumped
/// through all four.
#[test]
fn hotclose_press_delivers_wm_closed_before_the_release() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    let wh = on_closer(host, &mut pb, NAME | CLOSER | HOTCLOSE);

    button_here(host, &mut pb, 1);
    for _ in 0..3 {
        host.push(Packet { dx: 0, dy: 0, buttons: 1 });
    }
    host.push(Packet { dx: 0, dy: 0, buttons: 0 });
    let t0 = host.clock.load(Ordering::Relaxed);

    let (fired, msg) = evnt_multi_mesag_poll(&mut pb);
    let pumps = host.clock.load(Ordering::Relaxed) - t0;
    assert_eq!(
        fired & MU_MESAG,
        MU_MESAG,
        "a press on a HOTCLOSE closer must deliver a message at once; fired = {fired:#06x}"
    );
    assert_eq!(
        (msg[0], msg[3]),
        (WM_CLOSED, wh),
        "the message must be WM_CLOSED (22) for the window; msg = {msg:?}"
    );
    let left = host.script.lock().unwrap().len();
    assert_eq!(
        (pumps, left),
        (2, 2),
        "HOTCLOSE fires on the press: the call must pump twice (the head forker(), \
         aes/gemevlib.c:169-170, and its closing dsptch) and leave two of the four \
         packets queued after the press (one held, the release) untouched — pumped \
         {pumps}, {left} remain: the Control Manager waited for the release \
         (gr_watchbox) instead of answering on the press (aes/gemctrl.c:208-214)"
    );

    // Tidy: let the queued release through.
    drain(host, &mut pb);
    drain_messages(&mut pb);
}

/// **Probe 2 — the press is consumed, the closer re-arms.** After the
/// `WM_CLOSED`, `ct_msgup` fakes the button up (`gemctrl.c:90-94`). With the
/// press still physically held, a TeraDesk-style wait must therefore see
/// neither a second `WM_CLOSED` (the Control Manager is not re-triggered by a
/// level it has already acted on) nor `MU_BUTTON` (the AES's `button` is 0);
/// after the release, a fresh press on the same closer fires again.
#[test]
fn hotclose_press_is_consumed_and_the_closer_rearms_on_the_next_press() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    let wh = on_closer(host, &mut pb, NAME | CLOSER | HOTCLOSE);

    button_here(host, &mut pb, 1);
    let (fired, msg) = evnt_multi_mesag_poll(&mut pb);
    assert_eq!((fired & MU_MESAG, msg[0], msg[3]), (MU_MESAG, WM_CLOSED, wh), "msg = {msg:?}");

    // Still held. Two polls, as an application re-arming its main wait would.
    for i in 0..2 {
        let (fired, bret, msg) = evnt_multi_teradesk_poll(&mut pb);
        assert_eq!(
            fired & MU_MESAG,
            0,
            "poll {i}: the held press must not produce a second message \
             (aes/gemctrl.c:92, `button = 0`); msg = {msg:?}"
        );
        assert_eq!(
            fired & MU_BUTTON,
            0,
            "poll {i}: the held press must not reach the application as MU_BUTTON \
             (EV_BRET = {bret}) — ct_msgup faked the button up (aes/gemctrl.c:90-94), \
             so ev_multi's quick check sees button == 0; fired = {fired:#06x}"
        );
        assert_eq!(fired, MU_TIMER, "poll {i}: only the zero timer may fire; fired = {fired:#06x}");
    }

    // Release: a transition to up is not a press either.
    button_here(host, &mut pb, 0);
    let (fired, _, msg) = evnt_multi_teradesk_poll(&mut pb);
    assert_eq!(fired, MU_TIMER, "the release must produce nothing; fired = {fired:#06x}, msg = {msg:?}");

    // A fresh press on the same closer: the Control Manager acts again.
    button_here(host, &mut pb, 1);
    let (fired, msg) = evnt_multi_mesag_poll(&mut pb);
    assert_eq!(
        (fired & MU_MESAG, msg[0], msg[3]),
        (MU_MESAG, WM_CLOSED, wh),
        "a new press after the release must fire WM_CLOSED again; fired = {fired:#06x}, msg = {msg:?}"
    );

    button_here(host, &mut pb, 0);
    drain_messages(&mut pb);
}

/// **Probe 3 — regression, `CLOSER` without `HOTCLOSE`.** The same press with
/// the same four queued packets: the message comes only once the release has
/// been consumed (`gr_watchbox`, `gemctrl.c:216-220`), so the queue is empty
/// by the time `evnt_multi` returns, and the clock has moved by at least the
/// four pumps it took.
#[test]
fn plain_closer_fires_only_after_the_release_inside() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    let wh = on_closer(host, &mut pb, NAME | CLOSER);

    button_here(host, &mut pb, 1);
    for _ in 0..3 {
        host.push(Packet { dx: 0, dy: 0, buttons: 1 });
    }
    host.push(Packet { dx: 0, dy: 0, buttons: 0 });
    let t0 = host.clock.load(Ordering::Relaxed);

    let (fired, msg) = evnt_multi_mesag_poll(&mut pb);
    assert_eq!(
        (fired & MU_MESAG, msg[0], msg[3]),
        (MU_MESAG, WM_CLOSED, wh),
        "a plain CLOSER released inside must fire WM_CLOSED; fired = {fired:#06x}, msg = {msg:?}"
    );
    let left = host.script.lock().unwrap().len();
    assert_eq!(
        left, 0,
        "without HOTCLOSE the Control Manager waits for the release inside the \
         gadget (gr_watchbox): every queued packet must have been consumed, {left} remain"
    );
    let pumps = host.clock.load(Ordering::Relaxed) - t0;
    assert!(pumps >= 4, "the wait must have pumped through the release, pumped {pumps}");

    // And the button is up now: a TeraDesk-style poll sees nothing.
    let (fired, _, _) = evnt_multi_teradesk_poll(&mut pb);
    assert_eq!(fired, MU_TIMER, "after the release only the zero timer fires; fired = {fired:#06x}");
    drain_messages(&mut pb);
}
