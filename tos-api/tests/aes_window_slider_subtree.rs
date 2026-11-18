//! A98 — **`wind_set(WF_HSLIDE/WF_VSLIDE/WF_HSLSIZ/WF_VSLSIZ)` repaint only the
//! `W_HSLIDE`/`W_VSLIDE` subtree — track + elevator — never the arrow cells.**
//!
//! EmuTOS's `wm_set` names the slider object and walks exactly that subtree:
//!
//! ```c
//! case WF_VSLIDE:
//!     if (pwin->w_vslide != pinwds[0])    /* position changed? */
//!     {
//!         pwin->w_vslide = pinwds[0];     /* yes, save new posn */
//!         gadget = W_VSLIDE;              /* & redraw slider    */   /* gemwmlib.c:1711-1716 */
//!     }
//!     break;
//! ...
//! if (do_cpwalk)
//!     w_cpwalk(w_handle, gadget, MAX_DEPTH, TRUE);                   /* gemwmlib.c:1754-1755 */
//! ```
//!
//! `w_cpwalk` → `do_walk` → `ob_draw(tree, obj, depth)`, and `ob_draw` stops at
//! `obj`'s own subtree — `last = tree[obj].ob_next; everyobj(tree, obj, last,
//! just_draw, …)` (`aes/gemoblib.c:959-977`). The arrows are **siblings** of
//! the slider under the bar, not its children: `w_bldvbar` hangs `W_UPARROW`/
//! `W_DNARROW` and `W_VSLIDE` under `W_VBAR` and only `W_VELEV` under
//! `W_VSLIDE` (`w_adjust(W_VBAR, W_UPARROW, …)`, `w_adjust(W_VBAR, W_DNARROW,
//! …)`, `w_adjust(W_VBAR, W_VSLIDE, …)`, `w_adjust(W_VSLIDE, W_VELEV, …)`,
//! `aes/gemwmlib.c:412-436`; the horizontal bar likewise, `:450-474`). So a
//! slider redraw repaints the track and its elevator and **not one pixel of an
//! arrow cell**. NewTOS's `redraw_vslider`/`redraw_hslider` repainted the
//! arrows as well — same pixels afterwards, so only a per-pixel write mask
//! tells the two apart (`docs/api/aes/window-graphics.md`, "Redraw scope"
//! under `wind_set` (105), edge case 5).
//!
//! The backend of this binary records **which device pixels a call wrote**
//! (after clipping), on top of the A84 draw-op log. The arrow cells and the
//! track are located through the AES's own frame hit test (`gadget_name_at`
//! → `"up"`/`"dn"`/`"lf"`/`"rt"` for the arrow cells, `"vuppage"`/`"vdnpage"`/
//! `"velev"` resp. `"hlfpage"`/`"hrtpage"`/`"helev"` for the track), not by
//! re-deriving the layout in the test.
//!
//! Note on the shared row/column: EmuTOS places the down arrow at
//! `y+h+ADJ3DSTD-1` and the right arrow at `x+w-1+ADJ3DSTD` (`:424`, `:459`),
//! so the 3D-grown arrow slot overlaps the track's last row/column by one
//! pixel line — that line belongs to the `W_?SLIDE` object rect and is
//! legitimately repainted (with the track's own border colour, unchanged in
//! value). The hit-test cell of the arrow starts one line further in, so
//! "no arrow-cell pixel written" is exact for the cells the Control Manager
//! treats as the arrows, and the pixel-value probe below covers that line too.
//!
//! Cases (one per slider field, the four arms share `w_cpwalk`):
//!
//! * `wind_set(WF_VSLIDE, 500)` on a fresh window with both bars: the track
//!   (every pixel) and the elevator are written, no up/down-arrow pixel is
//!   written and every up/down-arrow pixel keeps its value — red before the
//!   fix (the arrows were repainted);
//! * the same for `WF_VSLSIZ`, `WF_HSLIDE` (left/right arrows), `WF_HSLSIZ`.
//!
//! Harness (raster stand-in + host + parameter block) after
//! `aes_window_slider_guard.rs` (A84), plus the touched mask.
#![allow(clippy::too_many_arguments)]

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use tos_api::bdos::process::Abi;
use tos_api::host::Host;
use tos_api::vdi::backend::{ClipRect, MonoSrc, PatternFill, Pel, RasterBackend, Rect, WriteMode};

const SCREEN_W: u16 = 640;
const SCREEN_H: u16 = 480;

// ── a raster backend that counts its draw operations ──────────────────────

struct CountingBackend {
    px: Mutex<Vec<u32>>,
    palette: Mutex<[u16; 256]>,
    req: Mutex<[(u16, u16, u16); 256]>,
    /// Every drawing entry point appends one line: `"fill_rect x=… y=… w=… h=…"`.
    ops: Mutex<Vec<String>>,
    /// One flag per device pixel, set by every write that reaches the
    /// buffer (after clipping) since the last [`Self::reset_ops`] — the
    /// "which pixels did the call touch" oracle, finer than the op log.
    touched: Mutex<Vec<bool>>,
}

impl CountingBackend {
    fn new() -> Self {
        CountingBackend {
            px: Mutex::new(vec![0; SCREEN_W as usize * SCREEN_H as usize]),
            palette: Mutex::new([0; 256]),
            req: Mutex::new([(0, 0, 0); 256]),
            ops: Mutex::new(Vec::new()),
            touched: Mutex::new(vec![false; SCREEN_W as usize * SCREEN_H as usize]),
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
        self.touched.lock().unwrap()[Self::idx(x, y)] = true;
    }

    fn log(&self, line: String) {
        self.ops.lock().unwrap().push(line);
    }

    /// Drop the op log and the touched mask; the next `take_ops` /
    /// [`Self::touched`] count from here.
    fn reset_ops(&self) {
        self.ops.lock().unwrap().clear();
        self.touched.lock().unwrap().fill(false);
    }

    /// Whether any write reached `(x, y)` since the last [`Self::reset_ops`].
    fn touched(&self, x: i16, y: i16) -> bool {
        Self::inside(x, y) && self.touched.lock().unwrap()[Self::idx(x, y)]
    }

    /// The operations issued since the last reset, oldest first.
    fn take_ops(&self) -> Vec<String> {
        std::mem::take(&mut *self.ops.lock().unwrap())
    }

    fn clear(&self) {
        self.px.lock().unwrap().fill(0);
    }
}

impl RasterBackend for CountingBackend {
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
        self.log(format!("fill_rect x={x} y={y} w={w} h={h} pel={pel}"));
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
        self.log(format!("abline ({x0},{y0})-({x1},{y1}) pel={pel}"));
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
        self.log(format!("fill_pattern x={x} y={y} w={w} h={h}"));
        let fg = self.rgb565_to_device(self.pel_to_device16(fill.fg));
        let bg = self.rgb565_to_device(self.pel_to_device16(fill.bg));
        for yy in y..y.saturating_add(h) {
            let row = fill.pattern[(yy.rem_euclid(16)) as usize % fill.pattern.len()];
            for xx in x..x.saturating_add(w) {
                let bit = row & (0x8000u16 >> (xx.rem_euclid(16) as u16));
                match fill.mode {
                    WriteMode::Replace => self.put(xx, yy, if bit != 0 { fg } else { bg }, clip),
                    _ => {
                        if bit != 0 {
                            self.put(xx, yy, fg, clip);
                        }
                    }
                }
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
        mode: WriteMode,
        clip: ClipRect,
    ) {
        self.log(format!("draw_glyph x={x} y={y}"));
        let v = self.rgb565_to_device(self.pel_to_device16(fg));
        let bg = self.rgb565_to_device(self.pel_to_device16(0));
        for row in 0..cell_h {
            let byte = glyph.get(row as usize).copied().unwrap_or(0);
            for col in 0..cell_w.min(8) {
                if byte & (0x80 >> col) != 0 {
                    self.put(x + col, y + row, v, clip);
                } else if mode == WriteMode::Replace {
                    self.put(x + col, y + row, bg, clip);
                }
            }
        }
    }
    fn blit_mono(&self, dst: Rect, src: MonoSrc, fg: Pel, bg: Pel, _m: WriteMode, clip: ClipRect) {
        self.log(format!("blit_mono x={} y={} w={} h={}", dst.x, dst.y, dst.w, dst.h));
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
        self.log(format!("copy_rect x={} y={} w={} h={} -> ({dx},{dy})", src.x, src.y, src.w, src.h));
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
        self.log(format!("put_device_pixel x={x} y={y}"));
        if Self::inside(x, y) {
            self.px.lock().unwrap()[Self::idx(x, y)] = argb;
            self.touched.lock().unwrap()[Self::idx(x, y)] = true;
        }
    }
    fn flush_rect(&self, _x0: i16, _y0: i16, _x1: i16, _y1: i16) {}
}

// ── the host: no input at all — every case is pure `wind_*` calls ──────────

struct QuietHost {
    raster: CountingBackend,
    clock: AtomicU64,
}

impl Host for QuietHost {
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
    }
}

// SAFETY: every mutable field is behind a Mutex / atomic.
unsafe impl Sync for QuietHost {}
unsafe impl Send for QuietHost {}

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
        // SAFETY: `block` holds six live pointers to arrays owned by `self`,
        // each larger than any opcode issued here declares.
        unsafe { tos_api::aes::dispatch(self.block.as_ptr() as u64, ABI) };
    }
}

fn install(host: &'static QuietHost) {
    let href: &'static &'static dyn Host = Box::leak(Box::new(host as &'static dyn Host));
    tos_api::host::install_host(href);
}

// ── AES constants ─────────────────────────────────────────────────────────

// window kinds (`include/aesdefs.h:519-531`)
const NAME: i16 = 0x0001;
const CLOSER: i16 = 0x0002;
const FULLER: i16 = 0x0004;
const MOVER: i16 = 0x0008;
const SIZER: i16 = 0x0020;
const UPARROW: i16 = 0x0040;
const DNARROW: i16 = 0x0080;
const VSLIDE: i16 = 0x0100;
const LFARROW: i16 = 0x0200;
const RTARROW: i16 = 0x0400;
const HSLIDE: i16 = 0x0800;
/// The A59 survey window kind: everything but `INFO`.
const KIND_0FEF: i16 =
    NAME | CLOSER | FULLER | MOVER | SIZER | UPARROW | DNARROW | VSLIDE | LFARROW | RTARROW | HSLIDE;

const OP_WIND_CREATE: i16 = 100;
const OP_WIND_OPEN: i16 = 101;
const OP_WIND_GET: i16 = 104;
const OP_WIND_SET: i16 = 105;
const OP_WIND_NEW: i16 = 109;

/// `WF_CXYWH` = `WF_CURRXYWH` (`include/aesdefs.h:538`).
const WF_CURRXYWH: i16 = 5;
const WF_HSLIDE: i16 = 8;
const WF_VSLIDE: i16 = 9;
const WF_HSLSIZ: i16 = 15;
const WF_VSLSIZ: i16 = 16;

// ── the rig ───────────────────────────────────────────────────────────────

/// The AES window table and the installed host are process-wide statics, so
/// the cases in this binary must not run concurrently.
static SERIAL: Mutex<()> = Mutex::new(());
static RIG: OnceLock<&'static QuietHost> = OnceLock::new();

fn rig() -> &'static QuietHost {
    RIG.get_or_init(|| {
        let host: &'static QuietHost = Box::leak(Box::new(QuietHost {
            raster: CountingBackend::new(),
            clock: AtomicU64::new(0),
        }));
        install(host);
        seed_palette(&host.raster);
        host
    })
}

/// Seed the backend's pseudo-palette the way the VDI dispatcher does before a
/// colour-sensitive op (`ACTIVE_PALETTE`, `kernel/src/vdi_backend.rs:11-21`).
fn seed_palette(b: &CountingBackend) {
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

/// Fresh AES window state (`wind_new`, EmuTOS `wm_new`, `aes/gemwmlib.c:
/// 1875-1893`) and a cleared framebuffer.
fn fresh() -> (&'static QuietHost, Aespb) {
    let host = rig();
    let mut pb = Aespb::new();
    pb.call(10, 0, 1, 0); // appl_init
    pb.intin.fill(0);
    pb.call(OP_WIND_NEW, 0, 1, 0);
    host.raster.clear();
    host.raster.reset_ops();
    (host, pb)
}

/// `wind_create(kind)` over the whole screen, returning the handle.
fn create_window(pb: &mut Aespb, kind: i16) -> i16 {
    pb.intin.fill(0);
    pb.intin[0] = kind;
    pb.intin[1] = 0;
    pb.intin[2] = 0;
    pb.intin[3] = SCREEN_W as i16;
    pb.intin[4] = SCREEN_H as i16;
    pb.call(OP_WIND_CREATE, 5, 1, 0);
    let h = pb.intout[0];
    assert!(h > 0, "wind_create must give a handle, got {h}");
    h
}

/// `wind_open(h, r)`.
fn open(pb: &mut Aespb, h: i16, r: (i16, i16, i16, i16)) {
    pb.intin.fill(0);
    pb.intin[0] = h;
    pb.intin[1] = r.0;
    pb.intin[2] = r.1;
    pb.intin[3] = r.2;
    pb.intin[4] = r.3;
    pb.call(OP_WIND_OPEN, 5, 1, 0);
    assert_eq!(pb.intout[0], 1, "wind_open({h})");
}

/// `wind_create` + `wind_open` — the A59 survey window `(30,40,320,240)`.
fn open_window(pb: &mut Aespb, kind: i16) -> i16 {
    let h = create_window(pb, kind);
    open(pb, h, (30, 40, 320, 240));
    h
}

/// `wind_set(h, field, value)` for a one-word field; returns `int_out[0]`.
fn set_int(pb: &mut Aespb, handle: i16, field: i16, value: i16) -> i16 {
    pb.intin.fill(0);
    pb.intin[0] = handle;
    pb.intin[1] = field;
    pb.intin[2] = value;
    pb.call(OP_WIND_SET, 6, 1, 0);
    pb.intout[0]
}

/// `wind_get(h, field)` → `int_out[1]`.
fn get_int(pb: &mut Aespb, handle: i16, field: i16) -> i16 {
    pb.intin.fill(0);
    pb.intin[0] = handle;
    pb.intin[1] = field;
    pb.call(OP_WIND_GET, 2, 5, 0);
    assert_eq!(pb.intout[0], 1, "wind_get({handle}, {field})");
    pb.intout[1]
}

/// Issue `wind_set(h, field, value)` and return the draw operations it caused.
fn set_and_count(host: &QuietHost, pb: &mut Aespb, handle: i16, field: i16, value: i16) -> Vec<String> {
    host.raster.reset_ops();
    assert_eq!(set_int(pb, handle, field, value), 1, "wind_set({handle}, {field}, {value})");
    host.raster.take_ops()
}


fn field_name(field: i16) -> &'static str {
    match field {
        WF_HSLIDE => "WF_HSLIDE",
        WF_VSLIDE => "WF_VSLIDE",
        WF_HSLSIZ => "WF_HSLSIZ",
        WF_VSLSIZ => "WF_VSLSIZ",
        _ => "?",
    }
}

/// Every point inside `curr` that the AES's own frame hit test classifies as
/// one of `names` (`gadget_name_at`, the `ob_find(gl_awind, W_BOX, …)` of
/// `hctl_window`, `aes/gemctrl.c:188`).
fn cells(wh: i16, curr: (i16, i16, i16, i16), names: &[&str]) -> Vec<(i16, i16)> {
    let (x0, y0, w, h) = curr;
    let mut out = Vec::new();
    for y in y0..y0 + h {
        for x in x0..x0 + w {
            if names.contains(&tos_api::aes::window::gadget_name_at(wh, x, y)) {
                out.push((x, y));
            }
        }
    }
    out
}

/// `wind_get(h, WF_CURRXYWH)` → `(x, y, w, h)`.
fn wind_get_curr(pb: &mut Aespb, handle: i16) -> (i16, i16, i16, i16) {
    pb.intin.fill(0);
    pb.intin[0] = handle;
    pb.intin[1] = WF_CURRXYWH;
    pb.call(OP_WIND_GET, 2, 5, 0);
    assert_eq!(pb.intout[0], 1, "wind_get({handle}, WF_CURRXYWH)");
    (pb.intout[1], pb.intout[2], pb.intout[3], pb.intout[4])
}

/// The shared body: on a fresh window with both bars, `wind_set(field, value)`
/// repaints the whole track + elevator (`track` names) and neither writes nor
/// changes one pixel of the arrow cells (`arrows` names).
fn slider_redraw_is_the_slide_subtree(field: i16, value: i16, arrows: [&str; 2], track: [&str; 3]) {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    let h = open_window(&mut pb, KIND_0FEF);
    let curr = wind_get_curr(&mut pb, h);
    let name = field_name(field);

    let arrow_px = cells(h, curr, &arrows);
    assert!(!arrow_px.is_empty(), "the window must have {arrows:?} cells");
    let before: Vec<u32> = arrow_px.iter().map(|&(x, y)| host.raster.get_device_pixel(x, y)).collect();

    let ops = set_and_count(host, &mut pb, h, field, value);
    assert!(!ops.is_empty(), "{name} {value} on a fresh window is a change and must redraw the slider");

    // The `W_?SLIDE` subtree: the track (page areas either side of the thumb)
    // and the elevator, every pixel written (`gr_rect` fills in MD_REPLACE).
    let track_px = cells(h, curr, &track);
    assert!(!track_px.is_empty(), "the window must have {track:?} cells");
    let track_missed: Vec<(i16, i16)> =
        track_px.iter().copied().filter(|&(x, y)| !host.raster.touched(x, y)).collect();
    assert!(
        track_missed.is_empty(),
        "A98: {name} {value} must repaint the whole {track:?} subtree; {} of {} pixels untouched, first {:?}",
        track_missed.len(),
        track_px.len(),
        track_missed.first()
    );

    // The arrows are siblings under `W_?BAR`, outside the walked subtree
    // (`gemwmlib.c:412-436,450-474`, `gemoblib.c:959-977`): not written …
    let arrow_written: Vec<(i16, i16)> =
        arrow_px.iter().copied().filter(|&(x, y)| host.raster.touched(x, y)).collect();
    assert!(
        arrow_written.is_empty(),
        "A98: {name} {value} wrote {} of {} {arrows:?} pixels, EmuTOS's `w_cpwalk(w_handle, W_?SLIDE, …)` \
         walks the slider subtree alone (gemoblib.c:959-977; arrows are siblings, gemwmlib.c:412-436,450-474); \
         first {:?}; {} draw ops: {:?}",
        arrow_written.len(),
        arrow_px.len(),
        arrow_written.first(),
        ops.len(),
        ops.first()
    );
    // … and unchanged in value (the pixel probe, not only the write mask).
    let after: Vec<u32> = arrow_px.iter().map(|&(x, y)| host.raster.get_device_pixel(x, y)).collect();
    let changed = before.iter().zip(after.iter()).filter(|(a, b)| a != b).count();
    assert_eq!(changed, 0, "A98: {changed} {arrows:?} pixels changed value across {name} {value}");
    assert_eq!(get_int(&mut pb, h, field), value, "{name} stored value");
}

// ── cases ─────────────────────────────────────────────────────────────────

const V_ARROWS: [&str; 2] = ["up", "dn"];
const V_TRACK: [&str; 3] = ["vuppage", "vdnpage", "velev"];
const H_ARROWS: [&str; 2] = ["lf", "rt"];
const H_TRACK: [&str; 3] = ["hlfpage", "hrtpage", "helev"];

/// `WF_VSLIDE` (`gemwmlib.c:1711-1716`, `gadget = W_VSLIDE`).
#[test]
fn vslide_redraw_is_the_vslide_subtree() {
    slider_redraw_is_the_slide_subtree(WF_VSLIDE, 500, V_ARROWS, V_TRACK);
}

/// `WF_VSLSIZ` (`gemwmlib.c:1697-1702`, `gadget = W_VSLIDE`).
#[test]
fn vslsiz_redraw_is_the_vslide_subtree() {
    slider_redraw_is_the_slide_subtree(WF_VSLSIZ, 250, V_ARROWS, V_TRACK);
}

/// `WF_HSLIDE` (`gemwmlib.c:1704-1709`, `gadget = W_HSLIDE`).
#[test]
fn hslide_redraw_is_the_hslide_subtree() {
    slider_redraw_is_the_slide_subtree(WF_HSLIDE, 500, H_ARROWS, H_TRACK);
}

/// `WF_HSLSIZ` (`gemwmlib.c:1690-1695`, `gadget = W_HSLIDE`).
#[test]
fn hslsiz_redraw_is_the_hslide_subtree() {
    slider_redraw_is_the_slide_subtree(WF_HSLSIZ, 250, H_ARROWS, H_TRACK);
}
