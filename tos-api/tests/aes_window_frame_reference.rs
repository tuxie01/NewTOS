//! A95 — **the window frame, pixel for pixel against EmuTOS on a Falcon.**
//!
//! Reference: `measurements/measurement-2026-09-20-a79/hatari-falcon-reference/shots/
//! falcon-tc-window-c.png` — EmuTOS (the pinned source's own build) on a
//! Hatari Falcon030 in 640x480 truecolor, the topped `C:\` window at screen
//! `(16,96,304,192)`; and `falcon-tc-two-windows.png`, the **untopped** back
//! window at `(16,64,384,320)`. Every expectation below names the reference
//! pixel(s) it encodes, frame-relative `(dx,dy)` = screen `(16+dx, 96+dy)`
//! (resp. `(16+dx, 64+dy)` for the untopped shot); a negative `dx`/`dy` counts
//! from the right/bottom border pixel. The frame is EmuTOS's `W_ACTIVE` tree
//! (`w_bldactive`, `aes/gemwmlib.c:479-628`) drawn by `just_draw`
//! (`aes/gemoblib.c:471-953`).
//!
//! Six deltas found by `measurements/measurement-2026-09-20-a79/RUNINFO.TXT` § FINAL, one
//! case each (all red before the fix):
//!
//! 1. **Bevel width** — `add_3d_effect` draws two 1-px polylines inside the
//!    border, each stopping one pixel short of both ends (`gemoblib.c:238-276`).
//! 2. **Name-box bevel** — `W_NAME` is `FL3DACT` with a `NAME` (`gemwmlib.c:553`)
//!    and gets the same bevel.
//! 3. **Glyph placement** — a `G_BOXCHAR` glyph is centred in `gr_inside(3D-grown
//!    cell, th)` with `gr_just`'s `(w+1)/2`, `(h+1)/2` (`gemgraf.c:567-593`) after
//!    the activator's `y-1` nudge (`acttxtmove`, `gemoblib.c:735-757`).
//! 4. **Down arrow / track** — `W_VBAR` runs to the seam row it shares with
//!    `W_HBAR` (`gemwmlib.c:606-610`); the down arrow sits at `y+h+ADJ3DSTD-1`
//!    (`:424`), so its grown slot ends on the row above the seam and the
//!    `W_VSLIDE` track is one row longer.
//! 5. **Left / right arrows** — `W_HBAR` starts on the window's left border
//!    column (`x = 0`, `:612-616`); the left arrow's grown slot is columns
//!    `0..22`, the right arrow's (`x+w-1+ADJ3DSTD`, `:459`) ends on the column
//!    before the seam column.
//! 6. **`DISABLED` name** — untopped, `W_NAME.ob_state = DISABLED`
//!    (`gemwmlib.c:537`); `just_draw` lays `IP_4PATT` in WHITE, `MD_TRANS`, over
//!    the interior (`gemoblib.c:917-930`) **before** the bevel (`:949-950`).
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
/// Every gadget, `INFO` included — the frame with all nineteen elements (the
/// reference window has all of them too).
const KIND_ALL: i16 = NAME
    | CLOSER
    | FULLER
    | MOVER
    | INFO
    | SIZER
    | UPARROW
    | DNARROW
    | VSLIDE
    | LFARROW
    | RTARROW
    | HSLIDE;

const WF_NAME: i16 = 2;
const WF_INFO: i16 = 3;
const WF_TOP: i16 = 10;
const WF_NEWDESK: i16 = 14;
const WF_DCOLOR: i16 = 19;

// palette indices the AES paints with (`obdefs.h:109-124`), as the test
// palette below maps them
const WHITE: Pel = 0;
const BLACK: Pel = 1;
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

/// `wind_set(h, WF_TOP)`.
fn set_top(pb: &mut Aespb, handle: i16) {
    pb.intin.fill(0);
    pb.intin[0] = handle;
    pb.intin[1] = WF_TOP;
    pb.call(105, 6, 1, 0);
    assert_eq!(pb.intout[0], 1, "wind_set({handle}, WF_TOP)");
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

// ── frame geometry the cases read pixels at ───────────────────────────────

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
    /// One row of `self`, `dy` from its top (negative: from its bottom row).
    fn row(self, dy: i16, x0: i16, x1: i16) -> R {
        R { x: self.fx(x0), y: self.fy(dy), w: self.fx(x1) - self.fx(x0) + 1, h: 1 }
    }
    /// One column of `self`, `dx` from its left (negative: from its right column).
    fn col(self, dx: i16, y0: i16, y1: i16) -> R {
        R { x: self.fx(dx), y: self.fy(y0), w: 1, h: self.fy(y1) - self.fy(y0) + 1 }
    }
    /// Frame-relative → screen x (`dx < 0` counts from the right border pixel).
    fn fx(self, dx: i16) -> i16 {
        if dx < 0 { self.x + self.w + dx } else { self.x + dx }
    }
    /// Frame-relative → screen y (`dy < 0` counts from the bottom border pixel).
    fn fy(self, dy: i16) -> i16 {
        if dy < 0 { self.y + self.h + dy } else { self.y + dy }
    }
}

/// The 3D-grown rect `just_draw` paints an `FL3DACT` object on
/// (`gemoblib.c:503-508`: `ADJ3DSTD` out on every side).
fn grown(r: R) -> R {
    r.inset(-ADJ3DSTD)
}

/// First work row for a `KIND_ALL` window (title + info bands, `band_top`).
fn work_top(b: R) -> i16 {
    b.y + ADJ_HBOX + GL_HBOX
}

/// The grown slots of the `FL3DACT` gadgets whose position **no** delta of
/// this ticket moves: closer, fuller, up arrow, sizer, vertical elevator (a
/// default `-1` thumb at slide 0: `gl_hbox - 2*ADJ3DSTD - 1` tall, grown by
/// `ADJ3DSTD`, its top border on the track's, `w_bldvbar`, `gemwmlib.c:431-436`).
/// Reference slots: closer `(0,0)+23x23`, fuller `(-23,0)`, up arrow `(-23,42)`,
/// sizer `(-23,-23)`, elevator `(-23,65)+23x18`.
fn fixed_slots(b: R) -> [(R, &'static str); 5] {
    let top = work_top(b);
    [
        (grown(R { x: b.x + ADJ3DSTD, y: b.y + ADJ3DSTD, w: GL_WBOX, h: GL_HBOX }), "W_CLOSER"),
        (grown(R { x: b.x + b.w - ADJ_WBOX + ADJ3DSTD, y: b.y + ADJ3DSTD, w: GL_WBOX, h: GL_HBOX }), "W_FULLER"),
        (grown(R { x: b.x + b.w - ADJ_WBOX + ADJ3DSTD, y: top + ADJ3DSTD, w: GL_WBOX, h: GL_HBOX }), "W_UPARROW"),
        (
            grown(R { x: b.x + b.w - GL_WBOX - ADJ3DSTD, y: b.y + b.h - GL_HBOX - ADJ3DSTD, w: GL_WBOX, h: GL_HBOX }),
            "W_SIZER",
        ),
        (
            grown(R {
                x: b.x + b.w - ADJ_WBOX + ADJ3DSTD,
                y: top + ADJ_HBOX + ADJ3DSTD,
                w: GL_WBOX,
                h: GL_HBOX - 2 * ADJ3DSTD - 1,
            }),
            "W_VELEV",
        ),
    ]
}

/// The `W_NAME` object rect for a window with `CLOSER | FULLER | NAME` —
/// `w_bldactive` (`gemwmlib.c:515-537`): the closer and the fuller each take a
/// full `adj_wbox` of the title box, the name sits `ADJ3DSTD` inside what is
/// left, `gl_hbox` tall at `y = ADJ3DSTD`.
fn name_rect(b: R) -> R {
    R { x: b.x + ADJ_WBOX + ADJ3DSTD, y: b.y + ADJ3DSTD, w: b.w - 2 * ADJ_WBOX - 2 * ADJ3DSTD, h: GL_HBOX }
}

/// The glyph cells of the centred `n`-character name in the `W_NAME` rect —
/// `gr_just` on `gr_inside(grown, 1)` after the activator's `y-1`
/// (`gemoblib.c:735-757`, `gemgraf.c:567-589`; A105 (2)): `x = c.x +
/// (c.w - n·8 + 1) / 2` with `c = (nr.x - 1, nr.y - 2, nr.w + 2, 21)`,
/// `y = c.y + (21 - 16 + 1) / 2 = nr.y + 1`.
fn name_cells(nr: R, n: i16) -> R {
    let nlen = n * 8;
    let (cx, cw) = (nr.x - 1, nr.w + 2);
    let slack = cw - nlen;
    let nx = if slack > 0 { cx + (slack + 1) / 2 } else { cx };
    R { x: nx, y: nr.y + 1, w: nlen, h: 16 }
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

/// Bounding box `(x0, x1, y0, y1)` of the BLACK pixels in `r`, or `None`.
fn ink_bbox(host: &QuietHost, r: R) -> Option<(i16, i16, i16, i16)> {
    let ink: Vec<(i16, i16)> = pixels(host, r).into_iter().filter(|p| p.2 == BLACK).map(|p| (p.0, p.1)).collect();
    if ink.is_empty() {
        return None;
    }
    Some((
        ink.iter().map(|p| p.0).min().unwrap(),
        ink.iter().map(|p| p.0).max().unwrap(),
        ink.iter().map(|p| p.1).min().unwrap(),
        ink.iter().map(|p| p.1).max().unwrap(),
    ))
}

/// The `IP_1PATT` dither (`vdi/vdi_fill.c` DITHER row 1, `[0x0000, 0x4444,
/// 0x0000, 0x1111]`) at screen-aligned `(x, y)`: row 1 inks columns 1, 5, 9,
/// 13; row 3 inks 3, 7, 11, 15.
fn ip1_is_ink(x: i16, y: i16) -> bool {
    let row = match y.rem_euclid(4) {
        1 => 0x4444u16,
        3 => 0x1111u16,
        _ => 0,
    };
    row & (0x8000 >> (x.rem_euclid(16) as u16)) != 0
}

/// The `IP_2PATT` dither (`[0x0000, 0x5555, 0x0000, 0x5555]`): ink on odd rows
/// at odd columns.
fn ip2_is_ink(x: i16, y: i16) -> bool {
    y.rem_euclid(4) % 2 == 1 && x.rem_euclid(16) % 2 == 1
}

/// The `IP_4PATT` dither (`[0xAAAA, 0x5555, 0xAAAA, 0x5555]`): ink where
/// column and row parity agree. Reference (`falcon-tc-two-windows.png`, the
/// untopped name): `(186,69)` black, `(187,69)` white, `(187,70)` black.
fn ip4_is_ink(x: i16, y: i16) -> bool {
    let row = if y.rem_euclid(2) == 0 { 0xAAAAu16 } else { 0x5555u16 };
    row & (0x8000 >> (x.rem_euclid(16) as u16)) != 0
}

/// Assert the bevel `add_3d_effect` leaves inside a 1px border (`gemoblib.c:
/// 238-276`) on the grown slot `s`, **exactly one pixel wide**: with
/// `r = gr_inside(s, 1)`, WHITE from `(r.x, r.y+r.h-2)` up to `(r.x, r.y)` and
/// on to `(r.x+r.w-2, r.y)`; LBLACK from `(r.x+1, r.y+r.h-1)` to
/// `(r.x+r.w-1, r.y+r.h-1)` up to `(r.x+r.w-1, r.y+1)`. The two far corners
/// `(r.x, r.y+r.h-1)` and `(r.x+r.w-1, r.y)` are untouched face (white), and
/// nothing inside `r` carries LBLACK.
///
/// Reference (`falcon-tc-window-c.png`, closer): LBLACK at `x=37` rows
/// `98..117` and row `117` columns `18..37`; `x=36` rows `98..116` and row
/// `116` columns `18..36` white (face); `(17,117)` and `(37,97)` white.
fn assert_bevel_1px(host: &QuietHost, s: R, what: &str) {
    let r = s.inset(1);
    assert_all(host, R { x: r.x, y: r.y, w: 1, h: r.h - 1 }, WHITE, &format!("{what} bevel left (white)"));
    assert_all(host, R { x: r.x, y: r.y, w: r.w - 1, h: 1 }, WHITE, &format!("{what} bevel top (white)"));
    assert_all(host, R { x: r.x + 1, y: r.y + r.h - 1, w: r.w - 1, h: 1 }, LBLACK, &format!("{what} bevel bottom (LBLACK)"));
    assert_all(host, R { x: r.x + r.w - 1, y: r.y + 1, w: 1, h: r.h - 1 }, LBLACK, &format!("{what} bevel right (LBLACK)"));
    assert_eq!(px(host, r.x, r.y + r.h - 1), WHITE, "{what}: bottom-left corner is untouched face");
    assert_eq!(px(host, r.x + r.w - 1, r.y), WHITE, "{what}: top-right corner is untouched face");
    let inner = r.inset(1);
    assert_eq!(count(host, inner, LBLACK), 0, "{what}: no second LBLACK line inside the 1-px bevel {inner:?}");
}

/// The default topped `KIND_ALL` window at the survey rect, name "A A", info "i".
const B: R = R { x: 30, y: 40, w: 320, h: 240 };

fn topped_window(pb: &mut Aespb) -> i16 {
    let h = open_window(pb, KIND_ALL, (B.x, B.y, B.w, B.h));
    set_string(pb, h, WF_NAME, b"A A\0");
    set_string(pb, h, WF_INFO, b"i\0");
    h
}

// ── (1) bevel width ───────────────────────────────────────────────────────

/// `add_3d_effect` (`gemoblib.c:238-276`): two 1-px polylines inside the
/// border, WHITE top/left and LBLACK bottom/right, each stopping one pixel
/// short of both ends. Reference: closer LBLACK only at column `21` (rows
/// `2..21`) and row `21` (columns `2..21`) — screen `x=37`/`y=117`; column
/// `20` and row `20` are face white. Same on the fuller (column `-2`, row
/// `21`), the up arrow (column `-2` rows `44..63`, row `63`), the sizer
/// (column `-2`, row `-2`) and the elevator (column `-2`, row `81`).
#[test]
fn bevel_is_one_pixel_on_every_fl3dact_gadget() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    topped_window(&mut pb);
    for (slot, what) in fixed_slots(B) {
        assert_ring(host, slot, BLACK, &format!("{what} border"));
        assert_bevel_1px(host, slot, what);
    }
}

// ── (2) the name box's bevel ──────────────────────────────────────────────

/// `W_NAME` is `FL3DACT` when the window has a `NAME` (`w_bldactive`,
/// `gemwmlib.c:553`) — `just_draw` grows it (`gemoblib.c:503-508`) and ends
/// with `add_3d_effect` (`:949-950`). Reference (`falcon-tc-window-c.png`):
/// row `97` white from `x=40` to `294` and column `40` white rows `97..116`
/// (the white polyline); row `117` LBLACK from `x=41` to `295` and column
/// `295` LBLACK rows `98..117`; `(295,97)` is the `IP_2PATT` stipple (black
/// there), `(40,117)` the stipple too (white there) — the untouched corners.
#[test]
fn topped_name_box_carries_the_fl3dact_bevel() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    topped_window(&mut pb);
    let nbox = grown(name_rect(B));
    assert_ring(host, nbox, BLACK, "W_NAME border");
    let r = nbox.inset(1);
    assert_all(host, R { x: r.x, y: r.y, w: 1, h: r.h - 1 }, WHITE, "W_NAME bevel left (ref column 40)");
    assert_all(host, R { x: r.x, y: r.y, w: r.w - 1, h: 1 }, WHITE, "W_NAME bevel top (ref row 97)");
    assert_all(host, R { x: r.x + 1, y: r.y + r.h - 1, w: r.w - 1, h: 1 }, LBLACK, "W_NAME bevel bottom (ref row 117)");
    assert_all(host, R { x: r.x + r.w - 1, y: r.y + 1, w: 1, h: r.h - 1 }, LBLACK, "W_NAME bevel right (ref column 295)");
    // The two untouched corners show the interior: `IP_2PATT` in black.
    for (x, y) in [(r.x + r.w - 1, r.y), (r.x, r.y + r.h - 1)] {
        let want = if ip2_is_ink(x, y) { BLACK } else { WHITE };
        assert_eq!(px(host, x, y), want, "W_NAME bevel corner ({x},{y}) shows the stipple");
    }
    // Inside the bevel the stipple is untouched (outside the glyph cells).
    let cells = name_cells(name_rect(B), 3);
    for (x, y, p) in pixels(host, r.inset(1)) {
        if cells.contains(x, y) {
            continue;
        }
        let want = if ip2_is_ink(x, y) { BLACK } else { WHITE };
        assert_eq!(p, want, "W_NAME IP_2PATT at ({x},{y})");
    }
}

// ── (3) glyph placement ───────────────────────────────────────────────────

/// `G_BOXCHAR`: `gr_gtext(TE_CNTR, IBM, ch, gr_inside(&t, th))` on the grown
/// cell, `c.g_y--` for an unselected activator (`acttxtmove`, `gemoblib.c:
/// 735-757`), `gr_just` adding `(w+1)/2` and `(h+1)/2` (`gemgraf.c:567-593`)
/// — the glyph cell of a `23x23` slot starts at `(8,3)`. Reference ink
/// bounding boxes (black pixels, frame-relative, `falcon-tc-window-c.png`):
/// closer columns `8..14` rows `5..15`; fuller `-15..-9` / `5..15`; up arrow
/// `-15..-8` / `48..57`; sizer `-15..-9` / `-18..-8`; left arrow `8..15` /
/// `-18..-9`. NewTOS drew every glyph one column further left (`r.x +
/// (r.w-8)/2` on the un-grown cell).
#[test]
fn gadget_glyphs_are_centred_in_the_grown_cell() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    topped_window(&mut pb);
    let top = work_top(B);
    let face = |s: R| s.inset(2);
    let closer = grown(R { x: B.x + ADJ3DSTD, y: B.y + ADJ3DSTD, w: GL_WBOX, h: GL_HBOX });
    let fuller = grown(R { x: B.x + B.w - ADJ_WBOX + ADJ3DSTD, y: B.y + ADJ3DSTD, w: GL_WBOX, h: GL_HBOX });
    let up = grown(R { x: B.x + B.w - ADJ_WBOX + ADJ3DSTD, y: top + ADJ3DSTD, w: GL_WBOX, h: GL_HBOX });
    let sizer =
        grown(R { x: B.x + B.w - GL_WBOX - ADJ3DSTD, y: B.y + B.h - GL_HBOX - ADJ3DSTD, w: GL_WBOX, h: GL_HBOX });
    let lf = grown(R { x: B.x + ADJ3DSTD, y: B.y + B.h - ADJ_HBOX + ADJ3DSTD, w: GL_WBOX, h: GL_HBOX });
    let want = [
        (closer, "W_CLOSER", (B.fx(8), B.fx(14), B.fy(5), B.fy(15))),
        (fuller, "W_FULLER", (B.fx(-15), B.fx(-9), B.fy(5), B.fy(15))),
        (up, "W_UPARROW", (B.fx(-15), B.fx(-8), top + 6, top + 15)),
        (sizer, "W_SIZER", (B.fx(-15), B.fx(-9), B.fy(-18), B.fy(-8))),
        (lf, "W_LFARROW", (B.fx(8), B.fx(15), B.fy(-18), B.fy(-9))),
    ];
    for (slot, what, bbox) in want {
        assert_eq!(ink_bbox(host, face(slot)), Some(bbox), "{what} glyph ink bounding box (x0,x1,y0,y1)");
    }
}

// ── (4) the down arrow and the track ──────────────────────────────────────

/// `W_VBAR` is `work_h + 2` tall from the first work row — down to and
/// including the seam row it shares with `W_HBAR` (`gemwmlib.c:606-610`).
/// `w_bldvbar` puts the down arrow at `y+h+ADJ3DSTD-1` (`:424`): its grown
/// slot is rows `-46..-24`, bottom border directly above the seam row `-23`;
/// `W_VSLIDE` (`:429`) spans rows `65..146` (`-46`) — its bottom border is the
/// arrow slot's top border. Reference (`falcon-tc-window-c.png`): row `241`
/// (`-47`) is `IP_1PATT` track interior, rows `242` (`-46`), `264` (`-24`)
/// and `265` (`-23`) are black across the bar column; the arrow's ink is
/// columns `-15..-8`, rows `-40..-31`; LBLACK at column `-2` rows `-44..-25`
/// and row `-25` columns `-21..-2`. NewTOS drew the slot one row higher with
/// a white row at `-24`.
#[test]
fn down_arrow_slot_ends_on_the_row_above_the_seam() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    topped_window(&mut pb);
    let dn = R { x: B.fx(-23), y: B.fy(-46), w: ADJ_WBOX, h: ADJ_HBOX };
    assert_ring(host, dn, BLACK, "W_DNARROW border (ref rows 242/264, columns 281/303)");
    assert_bevel_1px(host, dn, "W_DNARROW");
    assert_eq!(
        ink_bbox(host, dn.inset(2)),
        Some((B.fx(-15), B.fx(-8), B.fy(-40), B.fy(-31))),
        "W_DNARROW glyph ink bounding box"
    );
    // The seam row below the slot: `W_HBAR`'s top border (`W_VBAR`'s bottom).
    assert_all(host, B.row(-23, -23, -1), BLACK, "seam row -23 (ref row 265)");
    // The track's last interior row `-47` is `IP_1PATT`, not a border.
    for (x, y, p) in pixels(host, B.row(-47, -22, -2)) {
        let want = if ip1_is_ink(x, y) { BLACK } else { WHITE };
        assert_eq!(p, want, "W_VSLIDE IP_1PATT on its last interior row at ({x},{y}) (ref row 241)");
    }
}

// ── (5) the left and right arrows ─────────────────────────────────────────

/// `W_HBAR` sits at `x = 0` of the data area — on the window's left border
/// column — `work_w + 2` wide (`gemwmlib.c:612-616`); the left arrow at
/// `x+ADJ3DSTD` (`:452`) has its grown slot on columns `0..22`, `W_HSLIDE`
/// starts on column `23` (`:466`) and ends on `-46`, the right arrow at
/// `x+w-1+ADJ3DSTD` (`:459`) fills `-46..-24` with its right border directly
/// left of the seam column `-23`. Reference (`falcon-tc-window-c.png`, rows
/// `-22..-2`): columns `22` and `23` black (arrow border + track border),
/// column `21` LBLACK, column `20` white; columns `-24` and `-23` black
/// (arrow border + seam), `-25` LBLACK, `-26` white, `-46` black (track
/// border = arrow border); the arrows' ink is columns `8..15` resp.
/// `-38..-31`, rows `-18..-9`. NewTOS drew both slots one column inward.
#[test]
fn horizontal_arrow_slots_sit_on_the_band_edges() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    topped_window(&mut pb);
    let lf = R { x: B.fx(0), y: B.fy(-23), w: ADJ_WBOX, h: ADJ_HBOX };
    let rt = R { x: B.fx(-46), y: B.fy(-23), w: ADJ_WBOX, h: ADJ_HBOX };
    assert_ring(host, lf, BLACK, "W_LFARROW border (ref columns 16/38)");
    assert_bevel_1px(host, lf, "W_LFARROW");
    assert_ring(host, rt, BLACK, "W_RTARROW border (ref columns 274/296)");
    assert_bevel_1px(host, rt, "W_RTARROW");
    assert_eq!(
        ink_bbox(host, lf.inset(2)),
        Some((B.fx(8), B.fx(15), B.fy(-18), B.fy(-9))),
        "W_LFARROW glyph ink bounding box"
    );
    assert_eq!(
        ink_bbox(host, rt.inset(2)),
        Some((B.fx(-38), B.fx(-31), B.fy(-18), B.fy(-9))),
        "W_RTARROW glyph ink bounding box"
    );
    // The track's borders: column 23 (left) and column -46 (shared with the
    // right arrow's slot); the seam column -23.
    assert_all(host, B.col(23, -23, -1), BLACK, "W_HSLIDE left border (ref column 39)");
    assert_all(host, B.col(-23, -23, -1), BLACK, "seam column -23 (ref column 297)");
    // Column -47 is the track's last interior column: `IP_1PATT` (the default
    // elevator sits at the left end of the track and does not reach it).
    for (x, y, p) in pixels(host, B.col(-47, -22, -2)) {
        let want = if ip1_is_ink(x, y) { BLACK } else { WHITE };
        assert_eq!(p, want, "W_HSLIDE IP_1PATT on its last interior column at ({x},{y})");
    }
}

// ── (6) the untopped name is DISABLED ─────────────────────────────────────

/// `w_bldactive` sets `W_NAME.ob_state = DISABLED` for a window that is not
/// `gl_wtop` (`gemwmlib.c:537`); `just_draw` then fills the interior with
/// `IP_4PATT` in WHITE, `MD_TRANS` (`gemoblib.c:917-930`) — every glyph pixel
/// on a pattern-set position turns white — and draws the bevel afterwards
/// (`:949-950`), so the LBLACK edge stays solid. Reference
/// (`falcon-tc-two-windows.png`, the back window at `(16,64)`): the name's
/// glyphs are a checkerboard — `(186,69)` black, `(187,69)` white, `(187,70)`
/// black — row `85` (`+21`) is solid LBLACK from `x=41`, and the closer's
/// glyph (not `DISABLED`) is solid at `(25,69)…`.
#[test]
fn untopped_name_is_drawn_disabled() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb) = fresh();
    // Two side-by-side windows; `h1` opened last (on top), then `WF_TOP` on
    // `h2` repaints `h1` "covered with deactivated borders" (`draw_change`,
    // `gemwmlib.c:1120-1122`).
    let b1 = R { x: 10, y: 40, w: 300, h: 200 };
    let b2 = R { x: 330, y: 40, w: 300, h: 200 };
    let h2 = open_window(&mut pb, KIND_ALL, (b2.x, b2.y, b2.w, b2.h));
    let h1 = open_window(&mut pb, KIND_ALL, (b1.x, b1.y, b1.w, b1.h));
    set_string(&mut pb, h1, WF_NAME, b"A A\0");
    set_string(&mut pb, h1, WF_INFO, b"i\0");
    set_top(&mut pb, h2);

    let nr = name_rect(b1);
    let nbox = grown(nr);
    assert_ring(host, nbox, BLACK, "untopped W_NAME border");
    assert_bevel_1px(host, nbox, "untopped W_NAME");
    let inner = nbox.inset(2);
    // Every pattern-set pixel of the interior is white — glyph or not.
    for (x, y, p) in pixels(host, inner) {
        if ip4_is_ink(x, y) {
            assert_eq!(p, WHITE, "DISABLED overlay: IP_4PATT pixel ({x},{y}) must be white");
        }
    }
    // The other half of the glyph ink survives.
    let cells = name_cells(nr, 3);
    let a_cell = R { x: cells.x, y: cells.y, w: 8, h: 16 };
    let survivors = count(host, a_cell, BLACK);
    assert!(survivors > 0, "the 'A' keeps its ink on the pattern-clear pixels (got {survivors})");
    let ink_on_pattern = pixels(host, a_cell).iter().filter(|p| p.2 == BLACK && ip4_is_ink(p.0, p.1)).count();
    assert_eq!(ink_on_pattern, 0, "no glyph ink on a pattern-set pixel");
    // Outside the glyph cells the interior is white (hollow `0x1101`).
    for (x, y, p) in pixels(host, inner) {
        if cells.contains(x, y) {
            continue;
        }
        assert_eq!(p, WHITE, "untopped W_NAME interior at ({x},{y})");
    }
    // The closer is not DISABLED: its glyph keeps ink on pattern-set pixels.
    let closer = grown(R { x: b1.x + ADJ3DSTD, y: b1.y + ADJ3DSTD, w: GL_WBOX, h: GL_HBOX });
    let closer_ink_on_pattern =
        pixels(host, closer.inset(2)).iter().filter(|p| p.2 == BLACK && ip4_is_ink(p.0, p.1)).count();
    assert!(closer_ink_on_pattern > 0, "the untopped closer's glyph is not dithered");
    // The topped window's name has no overlay: its stipple is intact two
    // pixels inside the border.
    let nr2 = name_rect(b2);
    let cells2 = name_cells(nr2, 0);
    for (x, y, p) in pixels(host, grown(nr2).inset(2)) {
        if cells2.contains(x, y) {
            continue;
        }
        let want = if ip2_is_ink(x, y) { BLACK } else { WHITE };
        assert_eq!(p, want, "topped W_NAME IP_2PATT at ({x},{y})");
    }
}
