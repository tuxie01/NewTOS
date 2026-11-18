//! `wind_set(WF_TOP)` on the **desktop handle** and on a **created-but-unopened
//! handle** — what `wm_mktop` does when its target is not an open window (A80;
//! spec `docs/api/aes/window-graphics.md` § `wind_set`, "`WF_TOP` on the
//! desktop handle and on a created-but-unopened handle").
//!
//! The reference, `aes/gemwmlib.c`:
//!
//! ```c
//! case WF_TOP:
//!     if (w_handle != gl_wtop)                    /* :1680 */
//!         wm_mktop(w_handle);                     /* :1682 */
//!     break;
//! ...
//! static void wm_mktop(WORD w_handle)             /* :1605 */
//! {
//!     ob_order(gl_wtree, w_handle, NIL);          /* :1609 — refuses ROOT, gemoblib.c:1209-1210 */
//!     w_getsize(WS_PREV, w_handle, &p);
//!     w_getsize(WS_CURR, w_handle, &t);
//!     draw_change(w_handle, &t);                  /* :1612 */
//!     w_setsize(WS_PREV, w_handle, &p);           /* :1613 */
//! }
//! ...
//! static void draw_change(WORD w_handle, GRECT *pt)   /* :1058 */
//!     ...
//!     w_setsize(WS_CURR, w_handle, pt);           /* :1073 */
//!     pw = w_getxptr(WS_WORK, w_handle);
//!     wm_calc(WC_WORK, D.w_win[w_handle].w_kind, ..., &pw->g_x, ...);  /* :1075-1077 */
//!     if ((D.w_win[w_handle].w_flags & VF_ISOPEN) == 0)               /* :1085 */
//!         return;                                                      /* :1086 */
//! ```
//!
//! * **T5** `wind_set(0, WF_TOP)` with a window open on top: `gl_wtop` is
//!   never 0, so `wm_mktop(0)` always runs; `ob_order` refuses `ROOT`, the
//!   desktop window is `VF_INUSE` but never `VF_ISOPEN` (`w_setup`,
//!   `:205-213,1290`), so `draw_change` stores the geometry and returns before
//!   the tree walk and before any drawing. Net: **0 pixels, no message, the
//!   top window unchanged, and `WS_WORK(0) := wm_calc(WC_WORK, NONE,
//!   WS_CURR(0))` = `(1, 1, W−2, H−2)`** (`:1807-1866`, every border 1 px for
//!   `kind = NONE`) instead of the `gl_rfull` `wm_init` stored (`:1294`).
//!   NewTOS before the fix: 2775 pixels changed (the top window's frame drawn
//!   deactivated + a 1-px frame around the desktop work area), `WS_WORK(0)`
//!   unchanged. **T5b** the same with *no* window open: the comparison is
//!   against the raw `gl_wtop` (`NIL`), so the desktop handle still qualifies.
//! * **T6** `wind_set(h2, WF_TOP)` on a handle that was created but never
//!   opened, while `h` is open: undefined in EmuTOS (`ob_order` → `get_par`
//!   walks `W_TREE[-1]`, `gemobjop.c:145-160`). NewTOS contract: `TRUE`, no
//!   repaint, no message, no z-order change, the window's rectangles left
//!   alone. Before the fix: 1215 pixels (the top window's frame deactivated)
//!   and a stacking value burnt on a closed slot.
//! * **T7** guard: `wind_set(h2, WF_TOP)` on an **open** `h2` under `h` still
//!   tops it — the frames repaint and `h2` is asked to redraw (the open-window
//!   path is covered in depth by `aes_window_content.rs::
//!   clicking_a_window_never_erases_its_content`, `aes_window_rectlist_zorder.rs::
//!   topping_a_window_moves_it_in_the_subtraction_order_like_a_later_open` and
//!   `aes_window_zorder_counter.rs`; this case only pins that the new guard
//!   does not swallow it).
//!
//! Harness shape (backend + host + parameter block) copied from
//! `aes_window_move_blit.rs` (A78).

#![allow(clippy::too_many_arguments)]

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
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

    fn snapshot(&self) -> Vec<u32> {
        self.px.lock().unwrap().clone()
    }

    /// Number of pixels of the whole screen that differ from `before`, and
    /// the first such pixel.
    fn diff_all(&self, before: &[u32]) -> (usize, Option<(i16, i16, u32, u32)>) {
        let px = self.px.lock().unwrap();
        let mut n = 0usize;
        let mut first = None;
        for y in 0..SCREEN_H as i16 {
            for x in 0..SCREEN_W as i16 {
                let k = Self::idx(x, y);
                if px[k] != before[k] {
                    if first.is_none() {
                        first = Some((x, y, before[k], px[k]));
                    }
                    n += 1;
                }
            }
        }
        (n, first)
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
    /// Honours the VDI write mode, like the board's backend
    /// (`kernel/src/vdi_backend.rs` `abline`/`combine`/`xor_pixel`).
    fn abline(
        &self,
        x0: i16,
        y0: i16,
        x1: i16,
        y1: i16,
        pel: Pel,
        mode: WriteMode,
        clip: ClipRect,
        mask: u16,
        last: bool,
    ) -> u16 {
        let fg = self.rgb565_to_device(self.pel_to_device16(pel));
        let bg = self.rgb565_to_device(self.pel_to_device16(0));
        let (ax, ay, mut bx, mut by) = if x1 < x0 { (x1, y1, x0, y0) } else { (x0, y0, x1, y1) };
        if mode == WriteMode::Xor && !last {
            if ax != bx {
                bx -= 1;
            } else if ay != by {
                by -= 1;
            }
        }
        let combine = |x: i16, y: i16, set: bool| match mode {
            WriteMode::Replace => self.put(x, y, if set { fg } else { bg }, clip),
            WriteMode::Transparent => {
                if set {
                    self.put(x, y, fg, clip)
                }
            }
            WriteMode::Xor => {
                if set {
                    let v = self.get_device_pixel(x, y);
                    self.put(x, y, v ^ 0x00FF_FFFF, clip);
                }
            }
            WriteMode::RevTransparent => {
                if !set {
                    self.put(x, y, fg, clip)
                }
            }
        };
        if ay == by {
            let (xlo, xhi) = (ax.min(bx), ax.max(bx));
            for x in xlo..=xhi {
                combine(x, ay, mask & (0x8000u16 >> (x & 0xf)) != 0);
            }
            return mask;
        }
        let (mut x, mut y) = (ax as i32, ay as i32);
        let (xe, ye) = (bx as i32, by as i32);
        let (dx, dy) = ((xe - x).abs(), -(ye - y).abs());
        let sy = if y < ye { 1 } else { -1 };
        let mut err = dx + dy;
        let mut m = mask;
        loop {
            combine(x as i16, y as i16, m & 0x8000 != 0);
            m = m.rotate_left(1);
            if x == xe && y == ye {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += 1;
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

// ── the host ──────────────────────────────────────────────────────────────

#[derive(Copy, Clone, Debug)]
struct Packet {
    dx: i16,
    dy: i16,
    buttons: u8,
}

struct ScriptHost {
    raster: BufBackend,
    script: Mutex<VecDeque<Packet>>,
    clock: AtomicU64,
}

impl ScriptHost {
    fn new(script: &[Packet]) -> Self {
        ScriptHost {
            raster: BufBackend::new(),
            script: Mutex::new(script.iter().copied().collect()),
            clock: AtomicU64::new(0),
        }
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
        if let Some(p) = next {
            tos_api::vdi::feed_relative(p.dx, p.dy, p.buttons, &self.raster);
        }
    }
}

unsafe impl Sync for ScriptHost {}
unsafe impl Send for ScriptHost {}

// ── AES parameter block ───────────────────────────────────────────────────

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

    fn call(&mut self, opcode: i16, in_len: i16, out_len: i16, ain_len: i16) {
        self.contrl[0] = opcode;
        self.contrl[1] = in_len;
        self.contrl[2] = out_len;
        self.contrl[3] = ain_len;
        self.contrl[4] = 0;
        self.intout.fill(0);
        unsafe { tos_api::aes::dispatch(self.block.as_ptr() as u64, ABI) };
    }
}

fn install(host: &'static ScriptHost) {
    let href: &'static &'static dyn Host = Box::leak(Box::new(host as &'static dyn Host));
    tos_api::host::install_host(href);
}

// ── AES constants ─────────────────────────────────────────────────────────

const NAME: i16 = 0x0001;
const CLOSER: i16 = 0x0002;
const FULLER: i16 = 0x0004;
const MOVER: i16 = 0x0008;
const INFO: i16 = 0x0010;
const SIZER: i16 = 0x0020;
const KIND: i16 = NAME | CLOSER | FULLER | MOVER | INFO | SIZER;

const DESKWH: i16 = 0;
const WF_WXYWH: i16 = 4;
const WF_CXYWH: i16 = 5;
const WF_PXYWH: i16 = 6;
const WF_TOP: i16 = 10;
const WF_NEWDESK: i16 = 14;
const WM_REDRAW: i16 = 20;

// ── the rig ───────────────────────────────────────────────────────────────

/// The AES window table and the installed host are process-wide statics, so
/// the cases in this binary must not run concurrently.
static SERIAL: Mutex<()> = Mutex::new(());
static RIG: OnceLock<&'static ScriptHost> = OnceLock::new();

/// The one host for this binary (`install_host` accepts a single install).
fn rig() -> &'static ScriptHost {
    RIG.get_or_init(|| {
        let host: &'static ScriptHost = Box::leak(Box::new(ScriptHost::new(&[])));
        install(host);
        seed_palette(&host.raster);
        host
    })
}

/// Seed the backend's pseudo-palette the way the VDI dispatcher does before a
/// colour-sensitive op (`ACTIVE_PALETTE`, `kernel/src/vdi_backend.rs:11-21`).
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

/// A desktop-background tree, as an application registers one with
/// `wind_set(0, WF_NEWDESK, tree, root)`: one patterned box over the screen.
fn desk_tree() -> &'static [Obj] {
    let bg = 0x0000_0043u64; // pattern 4, interior 3 (green) — a dithered desktop
    Box::leak(Box::new([Obj {
        next: -1,
        head: -1,
        tail: -1,
        otype: G_BOX,
        flags: 0,
        state: 0,
        spec: bg,
        x: 0,
        y: 0,
        w: SCREEN_W as i16,
        h: SCREEN_H as i16,
    }]))
}

/// Fresh Window Manager state — `wm_init` (EmuTOS `gemwmlib.c:1247-1310`),
/// which is the one thing that puts the desktop window's `WS_WORK` back to
/// `gl_rfull` (`:1294`; `wm_new` skips handle 0, `:1886`) — plus `wind_new`
/// and the registered desktop background tree; returns the app id. Every case
/// in this binary may leave `WS_WORK(0)` changed, so each starts from here.
fn fresh() -> (&'static ScriptHost, Aespb, i16) {
    let host = rig();
    let mut pb = Aespb::new();
    pb.call(10, 0, 1, 0); // appl_init
    let pid = pb.intout[0];
    tos_api::aes::window::wm_init();
    pb.intin.fill(0);
    pb.call(109, 0, 1, 0); // wind_new
    tos_api::aes::window::test_clear_msg(pid);

    let tree = desk_tree();
    pb.intin.fill(0);
    pb.intin[0] = DESKWH;
    pb.intin[1] = WF_NEWDESK;
    pb.intin[2] = 0; // root object
    pb.addrin[0] = tree.as_ptr() as u64;
    pb.call(105, 6, 1, 1); // wind_set
    pb.addrin[0] = 0;
    (host, pb, pid)
}

/// `wind_create(KIND, full screen)` → handle. The window is created *closed*.
fn create_window(pb: &mut Aespb) -> i16 {
    pb.intin.fill(0);
    pb.intin[0] = KIND;
    pb.intin[1] = 0;
    pb.intin[2] = 0;
    pb.intin[3] = SCREEN_W as i16;
    pb.intin[4] = SCREEN_H as i16;
    pb.call(100, 5, 1, 0); // wind_create
    let h = pb.intout[0];
    assert!(h > 0, "wind_create must give a handle, got {h}");
    h
}

/// `wind_create` + `wind_open` at `r`, returning the handle. Opening order is
/// z order: the window opened last ends up on top.
fn open_window(pb: &mut Aespb, r: (i16, i16, i16, i16)) -> i16 {
    let h = create_window(pb);
    pb.intin.fill(0);
    pb.intin[0] = h;
    pb.intin[1] = r.0;
    pb.intin[2] = r.1;
    pb.intin[3] = r.2;
    pb.intin[4] = r.3;
    pb.call(101, 5, 1, 0); // wind_open
    assert_eq!(pb.intout[0], 1, "wind_open({h}) failed");
    h
}

/// `wind_get(handle, field)` → `(x, y, w, h)`.
fn wind_get_rect(pb: &mut Aespb, handle: i16, field: i16) -> (i16, i16, i16, i16) {
    pb.intin.fill(0);
    pb.intin[0] = handle;
    pb.intin[1] = field;
    pb.call(104, 2, 5, 0);
    assert_eq!(pb.intout[0], 1, "wind_get({handle}, {field}) failed");
    (pb.intout[1], pb.intout[2], pb.intout[3], pb.intout[4])
}

/// `wind_get(0, WF_TOP)` → the handle of the top window (`w_top()`).
fn top_handle(pb: &mut Aespb) -> i16 {
    pb.intin.fill(0);
    pb.intin[0] = DESKWH;
    pb.intin[1] = WF_TOP;
    pb.call(104, 2, 5, 0);
    pb.intout[1]
}

/// `wind_set(handle, WF_TOP, 0, 0, 0, 0)` → `int_out[0]`.
fn set_top(pb: &mut Aespb, handle: i16) -> i16 {
    pb.intin.fill(0);
    pb.intin[0] = handle;
    pb.intin[1] = WF_TOP;
    pb.addrin[0] = 0;
    pb.call(105, 6, 1, 0);
    pb.intout[0]
}

/// Pop one message off the app's FIFO (`evnt_mesag`); `None` when empty — the
/// call would block on an empty queue, so the count is checked first.
fn next_msg(pb: &mut Aespb, pid: i16) -> Option<[i16; 8]> {
    if tos_api::aes::window::test_msg_count(pid) == 0 {
        return None;
    }
    let mut buf = [0i16; 8];
    pb.intin.fill(0);
    pb.addrin[0] = buf.as_mut_ptr() as u64;
    pb.call(23, 0, 1, 1); // evnt_mesag
    pb.addrin[0] = 0;
    Some(buf)
}

/// Every message queued for `pid`, in FIFO order.
fn drain(pb: &mut Aespb, pid: i16) -> Vec<[i16; 8]> {
    let mut out = Vec::new();
    while let Some(m) = next_msg(pb, pid) {
        out.push(m);
        if out.len() > 32 {
            break;
        }
    }
    out
}

/// The stacking value the AES hands out next, read without changing it (the
/// probe for "no `bump_z` happened").
fn next_z() -> i16 {
    let z = tos_api::aes::window::test_set_next_z(1);
    tos_api::aes::window::test_set_next_z(z);
    z
}

/// `wm_calc(WC_WORK, NONE, r)`: every border is 1 px (`aes/gemwmlib.c:1815`,
/// none of the `kind` tests at `:1818-1841` fires), so the work rect is the
/// border rect shrunk by one on each side (`:1864-1867`).
fn work_of_none(r: (i16, i16, i16, i16)) -> (i16, i16, i16, i16) {
    (r.0 + 1, r.1 + 1, r.2 - 2, r.3 - 2)
}

const R1: (i16, i16, i16, i16) = (30, 40, 320, 240);

// ── T5: wind_set(0, WF_TOP) with a window open on top ─────────────────────

/// `wm_mktop(0)`: nothing is drawn, nothing is sent, the top window stays,
/// and the desktop window's work rect is recomputed from its border rect with
/// `kind = NONE` (`aes/gemwmlib.c:1679-1683,1605-1613,1069-1086,1807-1866`).
#[test]
fn t5_topping_the_desktop_handle_paints_nothing_and_recomputes_its_work_rect() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb, pid) = fresh();
    let b = &host.raster;
    let h = open_window(&mut pb, R1);
    drain(&mut pb, pid);
    assert_eq!(top_handle(&mut pb), h, "rig check: h is on top");

    // The desktop window's four rectangles as `wm_init` left them
    // (`gemwmlib.c:1291-1294`): CURR/PREV = gl_rscreen, WORK = gl_rfull.
    let curr0 = wind_get_rect(&mut pb, DESKWH, WF_CXYWH);
    let prev0 = wind_get_rect(&mut pb, DESKWH, WF_PXYWH);
    let work0 = wind_get_rect(&mut pb, DESKWH, WF_WXYWH);
    assert_eq!(curr0, (0, 0, SCREEN_W as i16, SCREEN_H as i16), "rig check: WS_CURR(0) = gl_rscreen");
    assert_eq!(prev0, curr0, "rig check: WS_PREV(0) = gl_rscreen");
    let want = work_of_none(curr0);
    assert_ne!(
        work0, want,
        "rig check: gl_rfull {work0:?} must differ from wm_calc(WC_WORK, NONE, gl_rscreen) \
         {want:?}, or the recomputation is not observable"
    );
    let z_h = tos_api::aes::window::test_window_z(h);
    let nz = next_z();
    let before = b.snapshot();

    let ret = set_top(&mut pb, DESKWH);
    assert_eq!(ret, 1, "wm_set returns TRUE for WF_TOP (aes/gemwmlib.c:1737 is not reached)");

    // Not one pixel: `draw_change` returns before the tree walk and before any
    // drawing because the desktop window is not `VF_ISOPEN` (`:1085-1086`).
    let (n, first) = b.diff_all(&before);
    assert_eq!(
        n, 0,
        "wind_set(0, WF_TOP) must not paint: wm_mktop(0) never reaches a draw \
         (aes/gemwmlib.c:1605-1613,1079-1086), but {n} pixels changed (first {first:?})"
    );
    // No reorder: `ob_order` refuses ROOT (`gemoblib.c:1209-1210`).
    assert_eq!(top_handle(&mut pb), h, "the top window is unchanged");
    assert_eq!(tos_api::aes::window::test_window_z(h), z_h, "h keeps its stacking value");
    assert_eq!(next_z(), nz, "no stacking value was handed out (no bump_z)");
    // No message: `w_update`/`w_redraw` are never reached.
    let msgs = drain(&mut pb, pid);
    assert!(msgs.is_empty(), "wind_set(0, WF_TOP) must queue nothing, got {msgs:?}");

    // The geometry store did run (`:1069-1077`): WS_WORK(0) is now
    // wm_calc(WC_WORK, NONE, WS_CURR(0)) = (1, 1, W-2, H-2); CURR is unchanged
    // (it was stored onto itself) and PREV was restored by wm_mktop (`:1613`).
    let work1 = wind_get_rect(&mut pb, DESKWH, WF_WXYWH);
    assert_eq!(
        work1, want,
        "after wind_set(0, WF_TOP) the desktop window's WS_WORK is wm_calc(WC_WORK, NONE, \
         WS_CURR(0)) (aes/gemwmlib.c:1073-1077,1807-1866), not the gl_rfull wm_init stored \
         (:1294)"
    );
    assert_eq!(wind_get_rect(&mut pb, DESKWH, WF_CXYWH), curr0, "WS_CURR(0) unchanged");
    assert_eq!(wind_get_rect(&mut pb, DESKWH, WF_PXYWH), prev0, "WS_PREV(0) restored (:1613)");

    // And the open window's own rectangles were not touched.
    assert_eq!(wind_get_rect(&mut pb, h, WF_CXYWH), R1, "h's WS_CURR unchanged");
}

// ── T5b: the same with nothing open ───────────────────────────────────────

/// `gl_wtop` is `NIL` while no window is open, and `wm_set` compares the raw
/// variable (`aes/gemwmlib.c:1680`), so `0 != NIL` and `wm_mktop(0)` runs
/// here too. (`wind_get(0, WF_TOP)` folds `NIL` to 0, `w_top()`, `:315-318` —
/// comparing against *that* would silently skip this case.)
#[test]
fn t5b_topping_the_desktop_handle_with_nothing_open_recomputes_its_work_rect() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb, pid) = fresh();
    let b = &host.raster;
    drain(&mut pb, pid);
    assert_eq!(top_handle(&mut pb), DESKWH, "rig check: nothing is open");
    let curr0 = wind_get_rect(&mut pb, DESKWH, WF_CXYWH);
    let want = work_of_none(curr0);
    assert_ne!(wind_get_rect(&mut pb, DESKWH, WF_WXYWH), want, "rig check: gl_rfull differs");
    let before = b.snapshot();

    assert_eq!(set_top(&mut pb, DESKWH), 1);

    let (n, first) = b.diff_all(&before);
    assert_eq!(n, 0, "no pixel may change (first {first:?})");
    assert!(drain(&mut pb, pid).is_empty(), "no message");
    assert_eq!(
        wind_get_rect(&mut pb, DESKWH, WF_WXYWH),
        want,
        "wm_set compares against the raw gl_wtop (NIL, aes/gemwmlib.c:1680), so the \
         desktop handle is topped — and its WS_WORK recomputed — with nothing open too"
    );
}

// ── T6: wind_set(h2, WF_TOP) on a created-but-unopened h2 ─────────────────

/// Undefined in EmuTOS (`ob_order` → `get_par` reads `W_TREE[-1]`,
/// `gemobjop.c:145-160`). NewTOS contract: `TRUE`, no repaint, no message, no
/// z-order change, rectangles untouched.
#[test]
fn t6_topping_a_created_but_unopened_handle_does_nothing() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb, pid) = fresh();
    let b = &host.raster;
    let h = open_window(&mut pb, R1);
    let h2 = create_window(&mut pb);
    assert_ne!(h2, h);
    drain(&mut pb, pid);
    assert_eq!(top_handle(&mut pb), h, "rig check: h is on top");
    assert_eq!(tos_api::aes::window::test_window_z(h2), None, "rig check: h2 is not open");

    let z_h = tos_api::aes::window::test_window_z(h);
    let nz = next_z();
    let curr2 = wind_get_rect(&mut pb, h2, WF_CXYWH);
    let work2 = wind_get_rect(&mut pb, h2, WF_WXYWH);
    let work0 = wind_get_rect(&mut pb, DESKWH, WF_WXYWH);
    let before = b.snapshot();

    let ret = set_top(&mut pb, h2);
    assert_eq!(ret, 1, "a valid (in-use) handle is accepted; wm_set returns TRUE");

    let (n, first) = b.diff_all(&before);
    assert_eq!(
        n, 0,
        "topping a window that is not open must not paint (in particular not the top \
         window's frame as inactive), but {n} pixels changed (first {first:?})"
    );
    assert_eq!(top_handle(&mut pb), h, "the top window is unchanged");
    assert_eq!(tos_api::aes::window::test_window_z(h), z_h, "h keeps its stacking value");
    assert_eq!(tos_api::aes::window::test_window_z(h2), None, "h2 is still closed");
    assert_eq!(next_z(), nz, "no stacking value was burnt on the closed slot (no bump_z)");
    let msgs = drain(&mut pb, pid);
    assert!(msgs.is_empty(), "nothing is queued, got {msgs:?}");
    assert_eq!(wind_get_rect(&mut pb, h2, WF_CXYWH), curr2, "h2's WS_CURR untouched");
    assert_eq!(wind_get_rect(&mut pb, h2, WF_WXYWH), work2, "h2's WS_WORK untouched");
    assert_eq!(wind_get_rect(&mut pb, DESKWH, WF_WXYWH), work0, "the desktop's WS_WORK untouched");
}

// ── T7: guard — an open window under the top one is still topped ──────────

/// The open-window path (`wm_mktop` → `draw_change` top branch,
/// `aes/gemwmlib.c:1112-1136`) is untouched: frames repaint and the newly
/// topped window is asked to redraw. In depth: `aes_window_content.rs`,
/// `aes_window_rectlist_zorder.rs`, `aes_window_zorder_counter.rs`.
#[test]
fn t7_topping_an_open_window_under_the_top_one_still_repaints_and_redraws() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb, pid) = fresh();
    let b = &host.raster;
    let h2 = open_window(&mut pb, (200, 150, 300, 200));
    let h = open_window(&mut pb, R1); // opened last → on top, overlapping h2
    drain(&mut pb, pid);
    assert_eq!(top_handle(&mut pb), h, "rig check: h is on top");
    let z_h = tos_api::aes::window::test_window_z(h).expect("h is open");
    let before = b.snapshot();

    assert_eq!(set_top(&mut pb, h2), 1);

    let (n, _) = b.diff_all(&before);
    assert!(n > 0, "topping an open window repaints the two frames");
    assert_eq!(top_handle(&mut pb), h2, "h2 is on top now");
    assert!(
        tos_api::aes::window::test_window_z(h2).expect("h2 is open") > z_h,
        "h2 was moved above h"
    );
    let msgs = drain(&mut pb, pid);
    assert!(
        msgs.iter().any(|m| m[0] == WM_REDRAW && m[3] == h2),
        "the newly topped, previously covered h2 gets a WM_REDRAW (w_update/w_redraw, \
         aes/gemwmlib.c:1004-1048,887-907); got {msgs:?}"
    );
}
