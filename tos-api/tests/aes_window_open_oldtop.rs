//! A94 — **`wind_open` of a second window repaints the old top *untopped*,
//! in every piece of its own rectangle list.**
//!
//! EmuTOS `wm_open` (`aes/gemwmlib.c:1410-1432`) links the new window at the
//! top of `W_TREE` and calls `draw_change(w_handle, &t)` (`:1424-1426`).
//! `draw_change` rebuilds every rectangle list (`:1088`), makes the new
//! window `gl_wtop` (`:1090-1091`), and — because the previous top is no
//! longer `ob_tail` and the new top's `WS_PREV` (= the `gl_rzero` a created
//! window's `WS_CURR` is, `wm_create`, `:1377`) is the zero rectangle — draws
//! the old top "covered": `w_cpwalk(oldtop, 0, MAX_DEPTH, TRUE)`
//! (`:1194-1207`). `w_cpwalk` runs `w_bldactive(oldtop)` with
//! `istop = (gl_wtop == w_handle)` now **false** (`:495`, name `DISABLED`
//! `:537`, untopped colour words `:382-397`) and `do_walk`s the old top's own
//! rectangle list clipped to its `WS_TRUE` (`:363-379`, `:286-312`) — so the
//! whole visible frame of the old top is repainted deactivated, not only the
//! part inside the new window's damage rectangle. That call is gated on
//! `oldtop != NIL`, `oldtop != w_handle` and the zero `WS_PREV` (`:1203-1206`)
//! — **not** on any overlap. `w_update(w_handle, &c, DESKWH, FALSE)`
//! afterwards (`:1222`) visits the new window alone (`bottom == top`,
//! `:1014-1043`).
//!
//! Reference picture: `measurements/measurement-2026-09-20-a79/hatari-falcon-reference/
//! shots/falcon-tc-two-windows.png` — after the second window opened at
//! boot the first one's name band is white, no `IP_2PATT`, name `DISABLED`.
//! NewTOS before this ticket (`measurements/measurement-2026-09-20-a79/screendumps/
//! final-desk-B-two.png`): the old top kept its topped stipple and bevels
//! outside the new window's rectangle.
//!
//! Cases:
//!
//! * **partial overlap** — `h2` covers `h1`'s lower right; `h1`'s title band
//!   lies wholly outside `h2` and must come out hollow white (the untopped
//!   word `0x1101`, no stipple), its closer with border, bevel and white
//!   face; `h2`'s band carries the topped stipple. Red before the fix.
//! * **no overlap** — `h2` beside `h1`: `h1` is repainted untopped all the
//!   same (`:1203-1206` has no overlap test). Red before the fix.
//! * **fully covered** — `h1` under `h2` entirely (shadow included): the
//!   old top's rectangle list is empty, `do_walk` draws nothing, and the
//!   screen is pixel-identical to opening `h2` alone.
//!
//! The `DISABLED` overlay on the untopped name (A95) is not asserted: the
//! glyph cells are only required to be black/white.
//!
//! Harness (raster stand-in + host + parameter block) copied from
//! `aes_window_color_words.rs`.

#![allow(clippy::too_many_arguments)]

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

    fn clear(&self) {
        self.px.lock().unwrap().fill(0);
    }

    /// A copy of every device pixel — the "same picture" oracle.
    fn snapshot(&self) -> Vec<u32> {
        self.px.lock().unwrap().clone()
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
    /// Honours `Replace`: clear bits of the `cell_w`×`cell_h` cell are painted
    /// in palette colour 0 (the VDI background pen), set bits in `fg`; every
    /// other mode paints the set bits only.
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

// ── the host: no input at all — every case is pure `wind_*` calls ──────────

struct QuietHost {
    raster: BufBackend,
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


// ── AES constants ─────────────────────────────────────────────────────────

// window kinds (`include/aesdefs.h:519-531`)
const NAME: i16 = 0x0001;
const CLOSER: i16 = 0x0002;
const FULLER: i16 = 0x0004;
const MOVER: i16 = 0x0008;
const INFO: i16 = 0x0010;
const SIZER: i16 = 0x0020;
const UPARROW: i16 = 0x0040;
const DNARROW: i16 = 0x0080;
const VSLIDE: i16 = 0x0100;
const LFARROW: i16 = 0x0200;
const RTARROW: i16 = 0x0400;
const HSLIDE: i16 = 0x0800;
/// Every gadget, `INFO` included — the frame with all nineteen elements.
const KIND_ALL: i16 =
    NAME | CLOSER | FULLER | MOVER | INFO | SIZER | UPARROW | DNARROW | VSLIDE | LFARROW | RTARROW | HSLIDE;

const WF_NAME: i16 = 2;
const WF_CXYWH: i16 = 5;
const WF_INFO: i16 = 3;
const WF_NEWDESK: i16 = 14;
const WF_DCOLOR: i16 = 19;

// palette indices the AES paints with (`obdefs.h:109-124`), as the test
// palette below maps them
const WHITE: Pel = 0;
const BLACK: Pel = 1;
const LWHITE: Pel = 8;
const LBLACK: Pel = 9;

// frame metrics for the 8×16 system font on a ≥ 640-wide screen
// (`gl_wbox`/`gl_hbox` = 19, `ADJ3DSTD` = 2, `adj_wbox`/`adj_hbox` = 23)
const GL_WBOX: i16 = 19;
const GL_HBOX: i16 = 19;
const ADJ3DSTD: i16 = 2;
const ADJ_WBOX: i16 = GL_WBOX + 2 * ADJ3DSTD;
const ADJ_HBOX: i16 = GL_HBOX + 2 * ADJ3DSTD;


// ── the rig ───────────────────────────────────────────────────────────────

/// The AES window table and the installed host are process-wide statics, so
/// the cases in this binary must not run concurrently.
static SERIAL: Mutex<()> = Mutex::new(());
static RIG: OnceLock<&'static QuietHost> = OnceLock::new();

fn rig() -> &'static QuietHost {
    RIG.get_or_init(|| {
        let host: &'static QuietHost =
            Box::leak(Box::new(QuietHost { raster: BufBackend::new(), clock: AtomicU64::new(0) }));
        install(host);
        seed_palette(&host.raster);
        host
    })
}

/// Seed the backend's pseudo-palette the way the VDI dispatcher does before a
/// colour-sensitive op (`ACTIVE_PALETTE`, `kernel/src/vdi_backend.rs:11-21`).
/// Index 2 is RED (1000, 0, 0) — the colour T7's `ic=2` names.
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

/// Fresh AES window state (`wind_new`, EmuTOS `wm_new`, `aes/gemwmlib.c:
/// 1875-1893`), a cleared framebuffer, the registered desktop background
/// tree, and the compiled default colour words restored.
fn fresh() -> (&'static QuietHost, Aespb) {
    let host = rig();
    let mut pb = Aespb::new();
    pb.call(10, 0, 1, 0); // appl_init
    pb.intin.fill(0);
    pb.call(109, 0, 1, 0); // wind_new
    host.raster.clear();

    let tree = desk_tree();
    pb.intin.fill(0);
    pb.intin[0] = 0; // DESKWH
    pb.intin[1] = WF_NEWDESK;
    pb.intin[2] = 0; // root object
    pb.addrin[0] = tree.as_ptr() as u64;
    pb.call(105, 6, 1, 1); // wind_set
    pb.addrin[0] = 0;

    // `WF_DCOLOR` is process-wide state; put every slot back to `wm_start`'s
    // values so a case that changed a default cannot leak into the next.
    for g in 0..19 {
        let (t, b) = match g {
            3 => (0x11a1, 0x1101),
            12 | 17 => (0x1111, 0x1111),
            _ => (0x1101, 0x1101),
        };
        set_dcolor(&mut pb, g, t, b);
    }
    (host, pb)
}

/// `wind_create(kind)` + `wind_open(r)`, returning the handle.
fn open_window(pb: &mut Aespb, kind: i16, r: (i16, i16, i16, i16)) -> i16 {
    pb.intin.fill(0);
    pb.intin[0] = kind;
    pb.intin[1] = 0;
    pb.intin[2] = 0;
    pb.intin[3] = SCREEN_W as i16;
    pb.intin[4] = SCREEN_H as i16;
    pb.call(100, 5, 1, 0); // wind_create
    let h = pb.intout[0];
    assert!(h > 0, "wind_create must give a handle, got {h}");
    pb.intin.fill(0);
    pb.intin[0] = h;
    pb.intin[1] = r.0;
    pb.intin[2] = r.1;
    pb.intin[3] = r.2;
    pb.intin[4] = r.3;
    pb.call(101, 5, 1, 0); // wind_open
    assert_eq!(pb.intout[0], 1, "wind_open({h})");
    h
}

/// `wind_create(kind)` alone — a created but **unopened** window, whose
/// `WS_CURR` is `gl_rzero` (`wm_create`, `gemwmlib.c:1377`).
fn create_window(pb: &mut Aespb, kind: i16) -> i16 {
    pb.intin.fill(0);
    pb.intin[0] = kind;
    pb.intin[1] = 0;
    pb.intin[2] = 0;
    pb.intin[3] = SCREEN_W as i16;
    pb.intin[4] = SCREEN_H as i16;
    pb.call(100, 5, 1, 0); // wind_create
    let h = pb.intout[0];
    assert!(h > 0, "wind_create must give a handle, got {h}");
    h
}

/// `wind_set(h, WF_CXYWH, r)` — on an unopened window this is `draw_change`
/// storing `WS_PREV := WS_CURR`, `WS_CURR := r`, `WS_WORK` and returning at
/// the `VF_ISOPEN` test without drawing (`gemwmlib.c:1069-1086`). It is the
/// only way a window arrives at `wind_open` with a **non-zero** old
/// `WS_CURR` (A104).
fn set_cxywh(pb: &mut Aespb, handle: i16, r: (i16, i16, i16, i16)) {
    pb.intin.fill(0);
    pb.intin[0] = handle;
    pb.intin[1] = WF_CXYWH;
    pb.intin[2] = r.0;
    pb.intin[3] = r.1;
    pb.intin[4] = r.2;
    pb.intin[5] = r.3;
    pb.call(105, 6, 1, 0); // wind_set
    assert_eq!(pb.intout[0], 1, "wind_set({handle}, WF_CXYWH)");
}

/// `wind_open(h, r)` on an already-created window.
fn open_at(pb: &mut Aespb, handle: i16, r: (i16, i16, i16, i16)) {
    pb.intin.fill(0);
    pb.intin[0] = handle;
    pb.intin[1] = r.0;
    pb.intin[2] = r.1;
    pb.intin[3] = r.2;
    pb.intin[4] = r.3;
    pb.call(101, 5, 1, 0); // wind_open
    assert_eq!(pb.intout[0], 1, "wind_open({handle})");
}

/// `wind_set(h, WF_NAME | WF_INFO, string)` in the native (`addr_in[0]`) form.
fn set_string(pb: &mut Aespb, handle: i16, field: i16, s: &'static [u8]) {
    assert_eq!(s.last(), Some(&0), "string must be NUL-terminated");
    pb.intin.fill(0);
    pb.intin[0] = handle;
    pb.intin[1] = field;
    pb.addrin[0] = s.as_ptr() as u64;
    pb.call(105, 6, 1, 1);
    pb.addrin[0] = 0;
    assert_eq!(pb.intout[0], 1, "wind_set({handle}, {field}, string)");
}

/// `wind_set(0, WF_DCOLOR, gadget, tcol, bcol)`.
fn set_dcolor(pb: &mut Aespb, gadget: i16, tcol: i16, bcol: i16) {
    pb.intin.fill(0);
    pb.intin[0] = 0;
    pb.intin[1] = WF_DCOLOR;
    pb.intin[2] = gadget;
    pb.intin[3] = tcol;
    pb.intin[4] = bcol;
    pb.call(105, 6, 1, 0);
    assert_eq!(pb.intout[0], 1, "wind_set(WF_DCOLOR, {gadget})");
}



#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct R {
    x: i16,
    y: i16,
    w: i16,
    h: i16,
}

impl R {
    fn inset(self, n: i16) -> R {
        R { x: self.x + n, y: self.y + n, w: self.w - 2 * n, h: self.h - 2 * n }
    }
    fn contains(self, x: i16, y: i16) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

/// The `W_NAME` object rect for a window whose kind has `CLOSER | FULLER |
/// NAME` — EmuTOS `w_bldactive` (`gemwmlib.c:515-537`): the closer and the
/// fuller each take a full `adj_wbox` of the title box, the name sits
/// `ADJ3DSTD` inside what is left, `gl_hbox` tall at `y = ADJ3DSTD`.
fn name_rect(b: R) -> R {
    R {
        x: b.x + ADJ_WBOX + ADJ3DSTD,
        y: b.y + ADJ3DSTD,
        w: b.w - 2 * ADJ_WBOX - 2 * ADJ3DSTD,
        h: GL_HBOX,
    }
}

/// The 3D-grown rect `just_draw` paints an `FL3DACT` object on
/// (`gemoblib.c:503-508`: `ADJ3DSTD` out on every side).
fn grown(r: R) -> R {
    r.inset(-ADJ3DSTD)
}

/// First work row for a title-bar window without `INFO` (`band_top`).
fn top_of(b: R, has_info: bool) -> i16 {
    b.y + ADJ_HBOX + if has_info { GL_HBOX } else { 0 }
}

fn px(host: &QuietHost, x: i16, y: i16) -> Pel {
    host.raster.read_pixel(x, y)
}

/// Every pixel of `r`, as `(x, y, pel)`.
fn pixels(host: &QuietHost, r: R) -> Vec<(i16, i16, Pel)> {
    let mut v = Vec::with_capacity(r.w as usize * r.h as usize);
    for y in r.y..r.y + r.h {
        for x in r.x..r.x + r.w {
            v.push((x, y, px(host, x, y)));
        }
    }
    v
}

fn count(host: &QuietHost, r: R, pel: Pel) -> usize {
    pixels(host, r).iter().filter(|p| p.2 == pel).count()
}

/// Assert every pixel of `r` is `pel`, naming the first offender.
fn assert_all(host: &QuietHost, r: R, pel: Pel, what: &str) {
    for (x, y, p) in pixels(host, r) {
        assert_eq!(p, pel, "{what}: pixel ({x},{y}) in {r:?}");
    }
}

/// Assert the 1px ring of `r` is `pel`.
fn assert_ring(host: &QuietHost, r: R, pel: Pel, what: &str) {
    assert_all(host, R { x: r.x, y: r.y, w: r.w, h: 1 }, pel, &format!("{what} top row"));
    assert_all(host, R { x: r.x, y: r.y + r.h - 1, w: r.w, h: 1 }, pel, &format!("{what} bottom row"));
    assert_all(host, R { x: r.x, y: r.y, w: 1, h: r.h }, pel, &format!("{what} left column"));
    assert_all(host, R { x: r.x + r.w - 1, y: r.y, w: 1, h: r.h }, pel, &format!("{what} right column"));
}



/// The `IP_2PATT` dither (`vdi/vdi_fill.c` DITHER row 2, `[0x0000, 0x5555,
/// 0x0000, 0x5555]`) at screen-aligned `(x, y)`: ink on odd rows at odd
/// columns. Reference: `falcon-tc-window-c.png` name band, rows 99/101/…
/// carry `#` at x = 41, 43, … (odd), rows 98/100/… none.
fn ip2_is_ink(x: i16, y: i16) -> bool {
    y.rem_euclid(4) % 2 == 1 && x.rem_euclid(16) % 2 == 1
}

/// Assert the bevel `add_3d_effect` leaves inside a 1px border: WHITE along
/// the top and left, LBLACK along the bottom and right (`gemoblib.c:238-276`,
/// colours fixed at `:252,270-272`; reference `falcon-tc-window-c.png`
/// closer: white at x=17 / y=97, `(140,138,140)` at x=37 / y=117).
///
/// Samples the outermost pixel row/column of each edge, two pixels clear of
/// the corners; the exact 1-px width and the corner ownership are pinned by
/// `aes_window_frame_reference.rs` (A95).
fn assert_bevel(host: &QuietHost, ci: R, what: &str) {
    assert_all(host, R { x: ci.x + 2, y: ci.y, w: ci.w - 4, h: 1 }, WHITE, &format!("{what} bevel top"));
    assert_all(host, R { x: ci.x, y: ci.y + 2, w: 1, h: ci.h - 4 }, WHITE, &format!("{what} bevel left"));
    assert_all(host, R { x: ci.x + 2, y: ci.y + ci.h - 1, w: ci.w - 4, h: 1 }, LBLACK, &format!("{what} bevel bottom"));
    assert_all(host, R { x: ci.x + ci.w - 1, y: ci.y + 2, w: 1, h: ci.h - 4 }, LBLACK, &format!("{what} bevel right"));
}

// ── what an untopped / a topped title band looks like ─────────────────────

/// The glyph cells of a centred `n`-character name in the `W_NAME` rect —
/// left out of the interior assertions (the `DISABLED` overlay thins them
/// when untopped, A95 (6); the space cell is opaque white under `md=REPLACE`
/// when topped).
fn name_cells(nr: R, n: i16) -> R {
    let nlen = n * 8;
    let nx = nr.x + ((nr.w - nlen) / 2).max(2);
    R { x: nx, y: nr.y + 1, w: nlen, h: 16 }
}

/// `b`'s title band as `w_bldactive` builds it for a window that is **not**
/// `gl_wtop` (`gemwmlib.c:495,537`): the untopped `W_NAME` word `0x1101` =
/// hollow white inside a black ring, no `IP_2PATT`, glyphs black or white
/// only; the closer (`0x1101`, `FL3DACT`) with border, bevel and white face.
/// Reference: `falcon-tc-two-windows.png`, the back window's title.
fn assert_untopped_title(host: &QuietHost, b: R, name_chars: i16, what: &str) {
    // W_TITLE ring (the W_BOX ring's top row and its first `adj_hbox` rows
    // of both columns are the same pixels); the rest of the W_BOX ring may
    // lie under the new window and is not this function's business.
    assert_ring(host, R { x: b.x, y: b.y, w: b.w, h: ADJ_HBOX }, BLACK, &format!("{what}: W_TITLE border"));

    let closer = grown(R { x: b.x + ADJ3DSTD, y: b.y + ADJ3DSTD, w: GL_WBOX, h: GL_HBOX });
    assert_ring(host, closer, BLACK, &format!("{what}: W_CLOSER border"));
    let ci = closer.inset(1);
    assert_bevel(host, ci, &format!("{what}: W_CLOSER"));
    let face = ci.inset(2);
    assert!(count(host, face, BLACK) > 0, "{what}: W_CLOSER glyph");
    assert_eq!(
        count(host, face, BLACK) + count(host, face, WHITE),
        face.w as usize * face.h as usize,
        "{what}: W_CLOSER face is black/white only"
    );

    let nr = name_rect(b);
    let nbox = grown(nr);
    assert_ring(host, nbox, BLACK, &format!("{what}: W_NAME border"));
    // The `FL3DACT` name box carries the bevel (`add_3d_effect`,
    // `gemoblib.c:949-950`; reference `falcon-tc-two-windows.png` row 85
    // LBLACK) — the hollow-white claim holds inside it (A95 (2)).
    assert_bevel(host, nbox.inset(1), &format!("{what}: W_NAME"));
    let cells = name_cells(nr, name_chars);
    let inner = nbox.inset(2);
    for (x, y, p) in pixels(host, inner) {
        if cells.contains(x, y) {
            continue;
        }
        assert_eq!(p, WHITE, "{what}: untopped W_NAME is hollow white outside the glyphs, at ({x},{y})");
    }
    assert!(count(host, cells, BLACK) > 0, "{what}: W_NAME glyphs");
    assert_eq!(count(host, inner, LWHITE), 0, "{what}: untopped W_NAME has no grey");
    assert_eq!(
        count(host, inner, BLACK) + count(host, inner, WHITE),
        inner.w as usize * inner.h as usize,
        "{what}: untopped W_NAME is black/white only"
    );

    // W_INFO: `0x1101` topped and untopped alike — black ring, white inside.
    let info = R { x: b.x, y: b.y + ADJ_HBOX, w: b.w, h: GL_HBOX };
    assert_ring(host, info, BLACK, &format!("{what}: W_INFO border"));
    let ii = info.inset(1);
    assert_all(host, R { x: ii.x + ii.w - 40, y: ii.y, w: 40, h: ii.h }, WHITE, &format!("{what}: W_INFO interior"));
}

/// `b`'s name band as built for `gl_wtop`: the topped word `0x11a1` —
/// `IP_2PATT` in black inside the ring (sampled two pixels inside the
/// border, clear of the name-box bevel EmuTOS draws there, A95).
fn assert_topped_name(host: &QuietHost, b: R, name_chars: i16, what: &str) {
    let nr = name_rect(b);
    let nbox = grown(nr);
    assert_ring(host, nbox, BLACK, &format!("{what}: W_NAME border"));
    let cells = name_cells(nr, name_chars);
    for (x, y, p) in pixels(host, nbox.inset(3)) {
        if cells.contains(x, y) {
            continue;
        }
        let want = if ip2_is_ink(x, y) { BLACK } else { WHITE };
        assert_eq!(p, want, "{what}: topped W_NAME IP_2PATT at ({x},{y})");
    }
}

// ── the cases ─────────────────────────────────────────────────────────────

/// `h2` opens over `h1`'s lower right. `h1`'s title band (rows `60..83`) lies
/// wholly above `h2` (from row 160) and outside the damage rectangle
/// `wind_open` repaints for `h2`; EmuTOS repaints it anyway, untopped, via
/// `w_cpwalk(oldtop, 0, MAX_DEPTH, TRUE)` (`gemwmlib.c:1207`) clipped to
/// `h1`'s own rectangle list. Where `h1` *is* under `h2`, nothing of `h1`
/// may show — its list has no piece there.
#[test]
fn second_open_repaints_the_old_top_untopped_outside_the_new_window() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    let b1 = R { x: 40, y: 60, w: 360, h: 260 };
    let b2 = R { x: 200, y: 160, w: 380, h: 280 };
    let h1 = open_window(&mut pb, KIND_ALL, (b1.x, b1.y, b1.w, b1.h));
    set_string(&mut pb, h1, WF_NAME, b"A A\0");
    set_string(&mut pb, h1, WF_INFO, b"i\0");
    // The single open window is `gl_wtop`: topped stipple on its name.
    assert_topped_name(host, b1, 3, "h1 alone");

    let h2 = open_window(&mut pb, KIND_ALL, (b2.x, b2.y, b2.w, b2.h));
    set_string(&mut pb, h2, WF_NAME, b"B\0");
    set_string(&mut pb, h2, WF_INFO, b"j\0");

    // The old top, outside the new window: deactivated decorations.
    assert_untopped_title(host, b1, 3, "h1 after h2 opened");
    // The new top: activated.
    assert_topped_name(host, b2, 1, "h2");

    // `h1`'s vertical bar column (x 377..400) crosses `h2`'s title and info
    // bands (rows 160..202); `h1` owns no rectangle there, so its black bar
    // borders / IP_1PATT track must not appear: `h2`'s name band is the
    // pure topped stipple across that column (asserted above) and its info
    // band interior is white there. (`h2`'s *work area* is not asserted:
    // `w_update` draws the new window's frame only, `:1029`, and leaves the
    // work area to the owner's `WM_REDRAW` — `h1`'s earlier pixels
    // legitimately survive there until then, in EmuTOS as here.)
    let info2 = R { x: b2.x, y: b2.y + ADJ_HBOX, w: b2.w, h: GL_HBOX }.inset(1);
    let strip = R { x: b1.x + b1.w - ADJ_WBOX - 1, y: info2.y, w: ADJ_WBOX + 4, h: info2.h };
    assert_all(host, strip, WHITE, "h2's info band where h1's bar column crosses it");
}

/// `h2` beside `h1`, no overlap at all (the shadow included): the old top
/// is repainted untopped all the same — `draw_change` reaches the
/// `w_cpwalk(oldtop, …)` of `gemwmlib.c:1207` on `oldtop != NIL`,
/// `oldtop != w_handle` and a zero `WS_PREV` (`:1203-1206`); no overlap
/// test is involved.
#[test]
fn non_overlapping_second_open_still_repaints_the_old_top_untopped() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    let b1 = R { x: 10, y: 40, w: 300, h: 200 };
    let b2 = R { x: 330, y: 40, w: 300, h: 200 };
    let h1 = open_window(&mut pb, KIND_ALL, (b1.x, b1.y, b1.w, b1.h));
    set_string(&mut pb, h1, WF_NAME, b"A A\0");
    set_string(&mut pb, h1, WF_INFO, b"i\0");
    assert_topped_name(host, b1, 3, "h1 alone");

    let h2 = open_window(&mut pb, KIND_ALL, (b2.x, b2.y, b2.w, b2.h));
    set_string(&mut pb, h2, WF_NAME, b"B\0");
    set_string(&mut pb, h2, WF_INFO, b"j\0");

    assert_untopped_title(host, b1, 3, "h1 after h2 opened beside it");
    assert_topped_name(host, b2, 1, "h2");
}

/// `h1` wholly under `h2` (its drop shadow too): the old top's rectangle
/// list is empty, so `w_cpwalk(oldtop, …)`'s `do_walk` (`gemwmlib.c:
/// 286-312`) draws nothing. `h1`'s top border row (100) lies in `h2`'s
/// info band interior: it stays `h2`'s white, not `h1`'s black, and
/// outside `h2`'s work area the picture equals opening `h2` alone. Inside the work area `h1`'s earlier pixels survive until `h2`'s
/// `WM_REDRAW` (`w_update` draws the frame only, `:1029`) — in EmuTOS as
/// here — so only differences *there* are allowed.
#[test]
fn fully_covered_old_top_paints_nothing_outside() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let b1 = R { x: 100, y: 100, w: 200, h: 150 };
    let b2 = R { x: 60, y: 60, w: 400, h: 300 };

    let (host, mut pb) = fresh();
    let h2 = open_window(&mut pb, KIND_ALL, (b2.x, b2.y, b2.w, b2.h));
    set_string(&mut pb, h2, WF_NAME, b"B\0");
    set_string(&mut pb, h2, WF_INFO, b"j\0");
    let alone = host.raster.snapshot();

    let (host, mut pb) = fresh();
    let h1 = open_window(&mut pb, KIND_ALL, (b1.x, b1.y, b1.w, b1.h));
    set_string(&mut pb, h1, WF_NAME, b"A A\0");
    set_string(&mut pb, h1, WF_INFO, b"i\0");
    let h2 = open_window(&mut pb, KIND_ALL, (b2.x, b2.y, b2.w, b2.h));
    set_string(&mut pb, h2, WF_NAME, b"B\0");
    set_string(&mut pb, h2, WF_INFO, b"j\0");
    let covered = host.raster.snapshot();

    // `h2`'s work area: right of its left border, below its info band, left
    // of its vertical bar column, above its horizontal band.
    let work2 = R {
        x: b2.x + 1,
        y: top_of(b2, true),
        w: b2.w - 1 - ADJ_WBOX,
        h: b2.y + b2.h - ADJ_HBOX - top_of(b2, true),
    };
    let outside: Vec<(i16, i16)> = (0..SCREEN_H as i16)
        .flat_map(|y| (0..SCREEN_W as i16).map(move |x| (x, y)))
        .filter(|&(x, y)| !work2.contains(x, y))
        .filter(|&(x, y)| alone[BufBackend::idx(x, y)] != covered[BufBackend::idx(x, y)])
        .collect();
    assert!(
        outside.is_empty(),
        "a fully covered old top leaves no pixel of its own outside h2's work area: {} differ, first {:?}, last {:?}",
        outside.len(),
        outside.first(),
        outside.last()
    );
    // `h1`'s top border row (100) lies inside `h2`'s info band interior
    // (rows 84..100; row 101 is the band's own black bottom border): white,
    // as `h2` painted it.
    assert_all(host, R { x: b1.x, y: b1.y, w: b1.w + 2, h: 1 }, WHITE, "h2's info band over h1's top border");
    assert_topped_name(host, b2, 1, "h2");
}

// ── A104 — `wind_open` of a window whose old `WS_CURR` is not zero ─────────
//
// `wind_set(WF_CXYWH, r1)` on a created-but-unopened window leaves `WS_CURR =
// r1` (`draw_change` stores it and returns at the `VF_ISOPEN` test,
// `gemwmlib.c:1069-1086`). The `wind_open(r2)` that follows therefore enters
// `draw_change` with a **non-zero** `c`, and EmuTOS's branch tree
// (`:1104-1191`) splits by how `r2` relates to `r1`:
//
// | `r2` vs. `r1` | branch | what `draw_change` does |
// |---|---|---|
// | equal | "top request" `:1110-1136` | `w_cpwalk(oldtop, 0, MAX_DEPTH, TRUE)` — the old top untopped, in full (`:1118-1123`) — and, when neither window is `VF_BROKEN`, `w_cpwalk(gl_wtop, …)` + `return` (`:1129-1135`) |
// | same origin, other size | "size request" `:1138-1154` | damage = union of the two sizes + shadow; `start = DESKWH` on a shrink |
// | superset, other origin | "grow or open" `:1160-1167` | `c = pt` — identical to the plain-open branch |
// | same size, other origin | "move of top guy" `:1173-1177` | `w_move` **blits** the screen rectangle at `r1` to `r2` |
// | anything else | "other moves and shrinks" `:1184-1190` | damage = union of `r1` and `r2`, `start = DESKWH` |
//
// Only the first branch differs from ours in **pixels**, and it is the same
// defect A94 fixed one gate further on: `:1203-1206`'s `WS_PREV == gl_rzero`
// test keeps the old-top repaint from firing here, but `:1118-1123` runs it
// unconditionally. That gate is what the first case pins. The size/move/shrink
// branches only widen the damage rectangle over ground the window never
// occupied, so they repaint pixels that are already correct — the second and
// third cases measure exactly that. The `w_move` blit is the one place where
// EmuTOS would put *other* pixels on the screen; that one is a named
// deviation, see `docs/api/aes/window-graphics.md` § `wind_open`.

/// `h2` is created, given `b2` with `wind_set(WF_CXYWH)` and then **opened at
/// the same rectangle**. `pt == c` with `c != gl_rzero` is `draw_change`'s
/// top-request branch, and it is not the early `return` of `:1113-1116`: the
/// window just became `W_TREE[ROOT].ob_tail` (`w_obadd`, `:1425`) and was not
/// `oldtop`, because it was not open. So `w_cpwalk(oldtop, 0, MAX_DEPTH, TRUE)`
/// at `:1120` repaints `h1` deactivated through its own rectangle list — the
/// same call A94 wired for the plain-open branch, reached by a different gate.
#[test]
fn a104_open_at_the_preset_rect_repaints_the_old_top_untopped() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    let b1 = R { x: 10, y: 40, w: 300, h: 200 };
    let b2 = R { x: 330, y: 40, w: 300, h: 200 };
    let h1 = open_window(&mut pb, KIND_ALL, (b1.x, b1.y, b1.w, b1.h));
    set_string(&mut pb, h1, WF_NAME, b"A A\0");
    set_string(&mut pb, h1, WF_INFO, b"i\0");
    assert_topped_name(host, b1, 3, "h1 alone");

    let h2 = create_window(&mut pb, KIND_ALL);
    set_cxywh(&mut pb, h2, (b2.x, b2.y, b2.w, b2.h));
    open_at(&mut pb, h2, (b2.x, b2.y, b2.w, b2.h));
    set_string(&mut pb, h2, WF_NAME, b"B\0");
    set_string(&mut pb, h2, WF_INFO, b"j\0");

    assert_untopped_title(host, b1, 3, "h1 after h2 opened at its preset rect");
    assert_topped_name(host, b2, 1, "h2");
}

/// The counter-case, and the reason the gate cannot simply be dropped: when
/// the open rectangle is a **superset at another origin**, `draw_change` takes
/// the "grow that is a superset or an open" branch (`:1160-1167`) and reaches
/// `:1203-1206`, whose `w_getsize(WS_PREV, gl_wtop, &pprev)` is the preset
/// rectangle — not `gl_rzero` — so `w_cpwalk(oldtop, …)` is **skipped**. `h1`
/// keeps the topped decorations it was painted with, outside the damage
/// rectangle, exactly as in EmuTOS.
#[test]
fn a104_open_at_a_superset_of_the_preset_rect_leaves_the_old_top_alone() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    let b1 = R { x: 10, y: 40, w: 300, h: 200 };
    let preset = (350, 60, 200, 120);
    let b2 = R { x: 330, y: 40, w: 300, h: 200 };
    let h1 = open_window(&mut pb, KIND_ALL, (b1.x, b1.y, b1.w, b1.h));
    set_string(&mut pb, h1, WF_NAME, b"A A\0");
    set_string(&mut pb, h1, WF_INFO, b"i\0");
    assert_topped_name(host, b1, 3, "h1 alone");

    let h2 = create_window(&mut pb, KIND_ALL);
    set_cxywh(&mut pb, h2, preset);
    open_at(&mut pb, h2, (b2.x, b2.y, b2.w, b2.h));
    set_string(&mut pb, h2, WF_NAME, b"B\0");
    set_string(&mut pb, h2, WF_INFO, b"j\0");

    assert_topped_name(host, b1, 3, "h1 after a grow-branch open (:1204-1206 skips the repaint)");
    assert_topped_name(host, b2, 1, "h2");
}

/// The size/shrink/move branches take a **larger** damage rectangle than the
/// bare `pt` we repaint — the union of the preset and the open rectangle plus
/// the drop shadow (`:1152-1154`, `:1184-1190`) — and on a shrink they start
/// the walk at the desktop so `w_drawdesk` repaints the background over it
/// (`:1217-1219`). The extra ground is the rectangle the window was *assigned*
/// but never shown at, so repainting it puts back what is already there: the
/// screen after `create + wind_set(WF_CXYWH) + wind_open` must be pixel-
/// identical to the screen after a plain `create + wind_open` at the same
/// rectangle.
///
/// Both presets are checked with no other window open, so the old-top repaint
/// (the case above) plays no part: what is compared is the damage extent alone.
#[test]
fn a104_a_preset_the_window_never_showed_at_repaints_to_the_same_pixels() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let b = R { x: 120, y: 80, w: 360, h: 260 };

    let (host, mut pb) = fresh();
    let h = open_window(&mut pb, KIND_ALL, (b.x, b.y, b.w, b.h));
    set_string(&mut pb, h, WF_NAME, b"B\0");
    set_string(&mut pb, h, WF_INFO, b"j\0");
    let plain = host.raster.snapshot();

    // `:1138-1154` (same origin, other size — a shrink in both axes) and
    // `:1184-1190` (a diagonal move to another size).
    for (what, preset) in [
        ("same-origin shrink (size request, :1138-1154)", (b.x, b.y, b.w + 120, b.h + 80)),
        ("diagonal move to another size (:1184-1190)", (40, 300, 240, 150)),
    ] {
        let (host, mut pb) = fresh();
        let h = create_window(&mut pb, KIND_ALL);
        set_cxywh(&mut pb, h, preset);
        open_at(&mut pb, h, (b.x, b.y, b.w, b.h));
        set_string(&mut pb, h, WF_NAME, b"B\0");
        set_string(&mut pb, h, WF_INFO, b"j\0");
        let preset_run = host.raster.snapshot();

        let differ: Vec<usize> =
            (0..plain.len()).filter(|&i| plain[i] != preset_run[i]).take(8).collect();
        assert!(
            differ.is_empty(),
            "{what}: a preset rectangle must not change one pixel of the opened window's screen; \
             {} of the first 8 differing indices: {:?}",
            differ.len(),
            differ
        );
    }
}

/// The one branch whose pixels EmuTOS would genuinely put somewhere else:
/// when the open rectangle has the **same size at another origin**,
/// `draw_change` calls `w_move` (`:1173-1177`), which blits `WS_PREV + shadow`
/// — the preset rectangle — to the new place (`gemwmlib.c:946-970`), leaves
/// the window out of the frame repaint (`moved`, `:1025`) and out of the
/// `WM_REDRAW` fan-out. For a *move* that is right: the pixels being carried
/// are the window's own. For an **open** they are whatever happened to be on
/// the screen where the window never was — the desktop, or another window.
///
/// NewTOS paints the window instead; this case states that, and is the
/// receipt for the deviation recorded in
/// `docs/api/aes/window-graphics.md` § `wind_open`.
#[test]
fn a104_open_at_a_same_size_move_of_the_preset_paints_the_window_not_a_blit() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let b = R { x: 120, y: 80, w: 360, h: 260 };

    let (host, mut pb) = fresh();
    let h = open_window(&mut pb, KIND_ALL, (b.x, b.y, b.w, b.h));
    set_string(&mut pb, h, WF_NAME, b"B\0");
    set_string(&mut pb, h, WF_INFO, b"j\0");
    let plain = host.raster.snapshot();

    let (host, mut pb) = fresh();
    let h = create_window(&mut pb, KIND_ALL);
    set_cxywh(&mut pb, h, (40, 300, b.w, b.h)); // same size, other origin
    open_at(&mut pb, h, (b.x, b.y, b.w, b.h));
    set_string(&mut pb, h, WF_NAME, b"B\0");
    set_string(&mut pb, h, WF_INFO, b"j\0");
    let moved = host.raster.snapshot();

    let differ: Vec<usize> = (0..plain.len()).filter(|&i| plain[i] != moved[i]).take(8).collect();
    assert!(
        differ.is_empty(),
        "the window must be painted at its open rectangle, not blitted from ground it never \
         occupied; {} of the first 8 differing indices: {:?}",
        differ.len(),
        differ
    );
    assert_topped_name(host, b, 1, "the window opened after a same-size preset");
}
