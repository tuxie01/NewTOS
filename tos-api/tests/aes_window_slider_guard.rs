//! A84 — **`wind_set(WF_HSLIDE/WF_VSLIDE/WF_HSLSIZ/WF_VSLSIZ)` redraw the
//! slider only when the value changed.**
//!
//! EmuTOS's `wm_set` compares before it stores: `if (pwin->w_vslide !=
//! pinwds[0]) { pwin->w_vslide = pinwds[0]; gadget = W_VSLIDE; }`
//! (`references/emutos/aes/gemwmlib.c:1690-1716`, pinned tree
//! `1a7484aba1bfba23e2c0899e243b6d8177fe41df`). With the value unchanged
//! `gadget` stays `-1`, `do_cpwalk` stays `FALSE` and `w_cpwalk` is never
//! reached (`:1739-1755`) — the call issues **no draw operation at all**
//! (`docs/api/aes/window-graphics.md`, `wind_set` (105) field table "only if
//! changed", "Redraw scope", edge case 5).
//!
//! NewTOS stored and redrew unconditionally: the A59 survey's case T8
//! (`TASK/A84.md`) counted 32 draw operations for the second of two
//! `wind_set(h, WF_VSLIDE, 500)` calls where EmuTOS issues 0. The pixels
//! after the call are identical either way — only the draw-call count tells
//! the two apart, so this binary's backend **counts** every drawing entry
//! point and logs its arguments.
//!
//! Cases:
//!
//! * **T8** — `WF_VSLIDE 500` twice: the first call (0 → 500) draws, the
//!   second issues 0 operations. Red before the fix (the second call drew).
//! * The same for `WF_HSLIDE`, `WF_VSLSIZ` and `WF_HSLSIZ` — the four arms
//!   share the guard (`:1691`, `:1698`, `:1705`, `:1712`).
//! * The comparison happens **after** validation (`:1643-1656` clamps before
//!   the `switch`): `WF_VSLIDE 1500` after `WF_VSLIDE 1000` is "unchanged"
//!   (both clamp to 1000) and draws nothing; `WF_VSLSIZ -1` on a fresh window
//!   (default −1) draws nothing.
//! * A changed value still draws the slider, and the stored value is what
//!   `wind_get` reads back — the guard suppresses the redraw, not the store.
//! * A window that is created but not open stores without drawing regardless
//!   (`redraw_vslider` is gated on `w.open`; EmuTOS draws through the window's
//!   empty rectangle list) — a guard against the guard masking that path.
//!
//! Harness (raster stand-in + host + parameter block) after
//! `aes_window_color_words.rs`, reduced to what a draw-call count needs.

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
}

impl CountingBackend {
    fn new() -> Self {
        CountingBackend {
            px: Mutex::new(vec![0; SCREEN_W as usize * SCREEN_H as usize]),
            palette: Mutex::new([0; 256]),
            req: Mutex::new([(0, 0, 0); 256]),
            ops: Mutex::new(Vec::new()),
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

    fn log(&self, line: String) {
        self.ops.lock().unwrap().push(line);
    }

    /// Drop the op log; the next `take_ops` counts from here.
    fn reset_ops(&self) {
        self.ops.lock().unwrap().clear();
    }

    /// The operations issued since the last reset, oldest first.
    fn take_ops(&self) -> Vec<String> {
        std::mem::take(&mut *self.ops.lock().unwrap())
    }

    fn clear(&self) {
        self.px.lock().unwrap().fill(0);
    }

    /// FNV-1a over every device pixel — the "same picture" oracle.
    fn hash(&self) -> u64 {
        let px = self.px.lock().unwrap();
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for &v in px.iter() {
            for b in v.to_le_bytes() {
                h ^= b as u64;
                h = h.wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
        h
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

/// The shared body of the four per-field cases: a change draws, the same value
/// again draws nothing and leaves the picture as it was, and the stored value
/// survives both calls.
fn changed_draws_unchanged_does_not(field: i16, first: i16, second_same: i16) {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    let h = open_window(&mut pb, KIND_0FEF);
    let name = field_name(field);

    let ops1 = set_and_count(host, &mut pb, h, field, first);
    assert!(
        !ops1.is_empty(),
        "A84: {name} {first} on a fresh window is a change and must redraw the slider; drew nothing"
    );
    let before = host.raster.hash();

    // T8: the identical value again — EmuTOS `gemwmlib.c:1690-1716` compares
    // first and leaves `gadget == -1`, so nothing is drawn.
    let ops2 = set_and_count(host, &mut pb, h, field, second_same);
    assert!(
        ops2.is_empty(),
        "A84: {name} {second_same} again issued {} draw operations, EmuTOS issues 0 \
         (`if (pwin->w_… != pinwds[0])`, gemwmlib.c:1690-1716); first op: {:?}",
        ops2.len(),
        ops2.first()
    );
    assert_eq!(host.raster.hash(), before, "the unchanged call must leave every pixel as it was");
    assert_eq!(get_int(&mut pb, h, field), first, "{name} stored value after the two calls");
}

// ── cases ─────────────────────────────────────────────────────────────────

/// **T8** — `wind_set(h, WF_VSLIDE, 500)` twice: the second call draws nothing.
#[test]
fn t8_vslide_set_twice_draws_only_once() {
    changed_draws_unchanged_does_not(WF_VSLIDE, 500, 500);
}

#[test]
fn hslide_set_twice_draws_only_once() {
    changed_draws_unchanged_does_not(WF_HSLIDE, 500, 500);
}

#[test]
fn vslsiz_set_twice_draws_only_once() {
    changed_draws_unchanged_does_not(WF_VSLSIZ, 250, 250);
}

#[test]
fn hslsiz_set_twice_draws_only_once() {
    changed_draws_unchanged_does_not(WF_HSLSIZ, 250, 250);
}

/// The guard compares the **validated** value (`gemwmlib.c:1643-1656` clamps
/// before the `switch`): `1500` after `1000` is the same slider position.
#[test]
fn the_guard_compares_after_clamping() {
    changed_draws_unchanged_does_not(WF_VSLIDE, 1000, 1500);
}

/// `WF_VSLSIZ −1` ("default size") on a fresh window: `w_setup` already left
/// `w_vslsiz = -1` (`gemwmlib.c:217`), so this is not a change and draws
/// nothing; the value reads back as −1 (the getter round-trips it).
#[test]
fn default_size_on_a_fresh_window_is_not_a_change() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    let h = open_window(&mut pb, KIND_0FEF);
    let ops = set_and_count(host, &mut pb, h, WF_VSLSIZ, -1);
    assert!(ops.is_empty(), "A84: WF_VSLSIZ -1 on a fresh window drew {} ops", ops.len());
    assert_eq!(get_int(&mut pb, h, WF_VSLSIZ), -1);
    let ops = set_and_count(host, &mut pb, h, WF_HSLSIZ, -1);
    assert!(ops.is_empty(), "A84: WF_HSLSIZ -1 on a fresh window drew {} ops", ops.len());
}

/// A change after an unchanged call still draws: the guard suppresses the
/// redraw, never the store — `500 → 500 → 700` draws on the third call and
/// `wind_get` reads 700.
#[test]
fn a_later_change_still_draws_and_stores() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    let h = open_window(&mut pb, KIND_0FEF);
    set_and_count(host, &mut pb, h, WF_VSLIDE, 500);
    assert!(set_and_count(host, &mut pb, h, WF_VSLIDE, 500).is_empty());
    let ops = set_and_count(host, &mut pb, h, WF_VSLIDE, 700);
    assert!(!ops.is_empty(), "A84: WF_VSLIDE 500 → 700 must redraw the slider");
    assert_eq!(get_int(&mut pb, h, WF_VSLIDE), 700);
}

/// A created-but-unopened window stores the value and draws nothing either
/// way (there is no frame on screen); the guard must not turn that into a
/// draw and a later `wind_open` shows the stored position.
#[test]
fn an_unopened_window_stores_without_drawing() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    let h = create_window(&mut pb, KIND_0FEF);
    let ops = set_and_count(host, &mut pb, h, WF_VSLIDE, 500);
    assert!(ops.is_empty(), "unopened window: WF_VSLIDE drew {} ops", ops.len());
    let ops = set_and_count(host, &mut pb, h, WF_VSLIDE, 500);
    assert!(ops.is_empty(), "unopened window: WF_VSLIDE again drew {} ops", ops.len());
    assert_eq!(get_int(&mut pb, h, WF_VSLIDE), 500);
    host.raster.reset_ops();
    open(&mut pb, h, (30, 40, 320, 240));
    assert!(!host.raster.take_ops().is_empty(), "wind_open must draw the frame");
}
