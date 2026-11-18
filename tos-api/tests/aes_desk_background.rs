//! Reproduction rig for the **desktop-background over-paint**: the AES repaints
//! `w_drawdesk` over the whole damage rectangle instead of over the desktop
//! window's owner rectangle list, so it paints the background *inside* windows
//! it must never touch.
//!
//! What EmuTOS does. `w_drawdesk(pc)` is one call to
//! `do_walk(DESKWH, tree, root, depth, pc)` (`aes/gemwmlib.c:340-359`), and
//! `do_walk` (`:286-312`) is:
//!
//! ```c
//! if (pc) rc_intersect(&gl_rfull, pc); else pc = &gl_rfull;
//! for (po = D.w_win[wh].w_rlist; po; po = po->o_link) {
//!     rc_copy(&po->o_gr, &t);
//!     if (rc_intersect(pc, &t)) { gsx_sclip(&t); ob_draw(tree, obj, depth); }
//! }
//! ```
//!
//! Two clips, not one:
//!
//! * `gl_rfull` — the screen **minus the menu row** (`aes/gemgraf.c:423-424`,
//!   set unconditionally at `gsx_start`, menu bar installed or not). The AES
//!   never repaints the menu strip from `w_drawdesk`; removing a menu bar does
//!   not repaint it either (`mn_bar`, `aes/gemmnlib.c:724-728`).
//! * `D.w_win[DESKWH].w_rlist` — the desktop window's **owner rectangle list**:
//!   its own rectangle with every window above it subtracted, rebuilt by
//!   `newrect`/`mkrect`/`brkrct` (`aes/gemwrect.c:64-205`) at the top of every
//!   `draw_change` (`aes/gemwmlib.c:1129`) *before* anything is drawn. A window
//!   that is open owns its rectangle; the desktop does not, so `w_drawdesk`
//!   cannot paint into it. What the window shows is restored by its owner in
//!   answer to the `WM_REDRAW` that `w_update`/`w_redraw` post
//!   (`:1004-1048,887-907`) — the AES never paints a window's work area.
//!
//! The five entries into the background repaint are exercised here, one test
//! each: `wind_open`, `wind_close`, `wind_set(WF_CURRXYWH)`, the mover drag and
//! `form_dial(FMD_FINISH)`. Every one asserts the same two things — *nothing*
//! changed inside a covering window's work area, and the bare desktop inside
//! the damage rectangle *was* repainted (so a fix cannot be "paint less").
//!
//! Harness shape (backend + host + parameter block) copied from
//! `aes_window_content.rs`, which copied it from `aes_window_drag.rs`.

#![allow(clippy::too_many_arguments)]


use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use tos_api::bdos::process::Abi;
use tos_api::host::Host;
use tos_api::vdi::backend::{ClipRect, MonoSrc, PatternFill, Pel, RasterBackend, Rect, WriteMode};

const SCREEN_W: u16 = 640;
const SCREEN_H: u16 = 480;

// ── a raster backend over a real pixel buffer, with an op log ──────────────

struct BufBackend {
    px: Mutex<Vec<u32>>,
    palette: Mutex<[u16; 256]>,
    req: Mutex<[(u16, u16, u16); 256]>,
    log: Mutex<Vec<String>>,
    logging: AtomicBool,
}

impl BufBackend {
    fn new() -> Self {
        BufBackend {
            px: Mutex::new(vec![0; SCREEN_W as usize * SCREEN_H as usize]),
            palette: Mutex::new([0; 256]),
            req: Mutex::new([(0, 0, 0); 256]),
            log: Mutex::new(Vec::new()),
            logging: AtomicBool::new(false),
        }
    }

    fn note(&self, s: String) {
        if self.logging.load(Ordering::Relaxed) {
            self.log.lock().unwrap().push(s);
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
        self.note(format!("fill_rect x={x} y={y} w={w} h={h} pel={pel} clip={:?}", (clip.xmn, clip.ymn, clip.xmx, clip.ymx)));
        let v = self.rgb565_to_device(self.pel_to_device16(pel));
        for yy in y..y.saturating_add(h) {
            for xx in x..x.saturating_add(w) {
                self.put(xx, yy, v, clip);
            }
        }
    }
    /// Honours the VDI write mode, like the board's backend
    /// (`kernel/src/vdi_backend.rs` `abline`/`combine`/`xor_pixel`): XOR
    /// feedback (the AES rubber band) only restores the screen if the second
    /// draw really complements what the first one wrote.
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
        // Always draw left→right (EmuTOS `abline:2300`).
        let (ax, ay, mut bx, mut by) = if x1 < x0 { (x1, y1, x0, y0) } else { (x0, y0, x1, y1) };
        // XOR `LSTLIN` kludge (`abline:2319`): shorten all but the last segment
        // so a shared vertex is not complemented twice.
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
        // Horizontal: screen-aligned dash, `LN_MASK` not advanced.
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
        self.note(format!("fill_pattern x={x} y={y} w={w} h={h} fg={} bg={}", fill.fg, fill.bg));
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
        self.note(format!("copy_rect src=({},{},{},{}) -> ({dx},{dy})", src.x, src.y, src.w, src.h));
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
    fn push(&self, p: Packet) {
        self.script.lock().unwrap().push_back(p);
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

// window kinds
const NAME: i16 = 0x0001;
const CLOSER: i16 = 0x0002;
const FULLER: i16 = 0x0004;
const MOVER: i16 = 0x0008;
const SIZER: i16 = 0x0020;


const MU_KEYBD: i16 = 0x0001;
const MU_BUTTON: i16 = 0x0002;
const MU_MESAG: i16 = 0x0010;
const MU_TIMER: i16 = 0x0020;

// AES opcodes used here.
const OP_APPL_INIT: i16 = 10;
const OP_EVNT_MULTI: i16 = 25;
const OP_FORM_DIAL: i16 = 51;
const OP_GRAF_MKSTATE: i16 = 79;
const OP_WIND_CREATE: i16 = 100;
const OP_WIND_OPEN: i16 = 101;
const OP_WIND_CLOSE: i16 = 102;
const OP_WIND_GET: i16 = 104;
const OP_WIND_SET: i16 = 105;
const OP_WIND_NEW: i16 = 109;

const WF_WXYWH: i16 = 4;
const WF_CXYWH: i16 = 5;
const WF_NEWDESK: i16 = 14;

const FMD_FINISH: i16 = 3;

/// `WM_REDRAW` (`include/aesdefs.h:229`).
const WM_REDRAW: i16 = 20;

/// EmuTOS `gl_hbox` for the 8x16 system font: `gl_hchar + 3`
/// (`aes/gemgraf.c:412`), and therefore the height of `gl_rmenu` and the top
/// edge of `gl_rfull` (`:423-424,427`).
const GL_HBOX: i16 = 19;

/// A marker colour written straight into the framebuffer, standing in for
/// "application content the owner drew into its work area". Not a palette
/// colour, so nothing the AES paints can produce it by accident.
const MARK: u32 = 0x0011_2233;

/// Seed the backend's pseudo-palette the way the VDI dispatcher does before a
/// colour-sensitive op (`ACTIVE_PALETTE`, `kernel/src/vdi_backend.rs:11-21`):
/// EmuTOS' `videl_palette1` for pens 0..15, as RGB555-in-565 words.
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

/// A GEM `OBJECT` at the kernel/LP64 record width (`tos-api/src/aes/object.rs`).
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

/// The desktop-background tree an application registers with
/// `wind_set(0, WF_NEWDESK, tree, root)` — TeraDesk does exactly this
/// (`sdk/apps/teradesk/icon.c:2511`). Root = a patterned box over the whole
/// screen, plus three icon-sized boxes, i.e. the tree whose icons showed up as
/// travelling square holes when the AES painted it where it must not.
fn desk_tree() -> &'static [Obj] {
    // colour word: (border<<12)|(text<<8)|(tmode<<7)|(pattern<<4)|interior
    let bg = 0x0000_0043u64; // pattern 4, interior 3 (green) — a dithered desktop
    let ic = 0x0000_0002u64; // solid interior 2 (red) — an "icon"
    Box::leak(Box::new([
        Obj { next: -1, head: 1, tail: 3, otype: G_BOX, flags: 0, state: 0, spec: bg, x: 0, y: 0, w: SCREEN_W as i16, h: SCREEN_H as i16 },
        Obj { next: 2, head: -1, tail: -1, otype: G_BOX, flags: 0, state: 0, spec: ic, x: 150, y: 110, w: 32, h: 32 },
        Obj { next: 3, head: -1, tail: -1, otype: G_BOX, flags: 0, state: 0, spec: ic, x: 250, y: 180, w: 32, h: 32 },
        Obj { next: -1, head: -1, tail: -1, otype: G_BOX, flags: 0, state: 0, spec: ic, x: 350, y: 240, w: 32, h: 32 },
    ]))
}

impl BufBackend {
    /// Fill `(x, y, w, h)` with a raw device value, bypassing the AES — the
    /// stand-in for the owner's own content drawing.
    fn paint(&self, r: (i16, i16, i16, i16), argb: u32) {
        let mut px = self.px.lock().unwrap();
        for y in r.1..r.1 + r.3 {
            for x in r.0..r.0 + r.2 {
                if Self::inside(x, y) {
                    px[Self::idx(x, y)] = argb;
                }
            }
        }
    }

    /// Number of pixels inside `r` that differ from `before`, and the first
    /// such pixel (for the failure message).
    fn diff_in(&self, before: &[u32], r: (i16, i16, i16, i16)) -> (usize, Option<(i16, i16, u32, u32)>) {
        let px = self.px.lock().unwrap();
        let mut n = 0usize;
        let mut first = None;
        for y in r.1..r.1 + r.3 {
            for x in r.0..r.0 + r.2 {
                if !Self::inside(x, y) {
                    continue;
                }
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

    /// Number of pixels inside `r` still holding [`MARK`] — used as
    /// "the AES did *not* repaint here".
    fn marked_in(&self, r: (i16, i16, i16, i16)) -> usize {
        let px = self.px.lock().unwrap();
        let mut n = 0;
        for y in r.1..r.1 + r.3 {
            for x in r.0..r.0 + r.2 {
                if Self::inside(x, y) && px[Self::idx(x, y)] == MARK {
                    n += 1;
                }
            }
        }
        n
    }
}

// ── the rig ───────────────────────────────────────────────────────────────

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

/// Fresh AES window state + the registered desktop background tree. `wind_new`
/// (EmuTOS `wm_new`, `aes/gemwmlib.c:1875-1893`) closes and deletes every
/// window and drains the update locks, so each test starts from the same table.
fn fresh(host: &'static ScriptHost) -> Aespb {
    let mut pb = Aespb::new();
    pb.call(OP_APPL_INIT, 0, 1, 0);
    host.script.lock().unwrap().clear();
    pb.intin.fill(0);
    pb.call(OP_WIND_NEW, 0, 1, 0);

    let tree = desk_tree();
    pb.intin.fill(0);
    pb.intin[0] = 0; // DESKWH
    pb.intin[1] = WF_NEWDESK;
    pb.intin[2] = 0; // root object
    pb.addrin[0] = tree.as_ptr() as u64;
    pb.call(OP_WIND_SET, 6, 1, 1);
    pb.addrin[0] = 0;
    pb
}

/// `wind_create` + `wind_open` at `r`, returning the handle.
fn open_window(pb: &mut Aespb, kind: i16, r: (i16, i16, i16, i16)) -> i16 {
    pb.intin.fill(0);
    pb.intin[0] = kind;
    pb.intin[1] = 0;
    pb.intin[2] = 0;
    pb.intin[3] = SCREEN_W as i16;
    pb.intin[4] = SCREEN_H as i16;
    pb.call(OP_WIND_CREATE, 5, 1, 0);
    let h = pb.intout[0];
    assert!(h > 0, "wind_create must give a handle, got {h}");
    pb.intin.fill(0);
    pb.intin[0] = h;
    pb.intin[1] = r.0;
    pb.intin[2] = r.1;
    pb.intin[3] = r.2;
    pb.intin[4] = r.3;
    pb.call(OP_WIND_OPEN, 5, 1, 0);
    h
}

/// `wind_get(handle, field)` → `(x, y, w, h)`.
fn wind_get_rect(pb: &mut Aespb, handle: i16, field: i16) -> (i16, i16, i16, i16) {
    pb.intin.fill(0);
    pb.intin[0] = handle;
    pb.intin[1] = field;
    pb.call(OP_WIND_GET, 2, 5, 0);
    (pb.intout[1], pb.intout[2], pb.intout[3], pb.intout[4])
}

/// Pull the next message out of the calling application's FIFO, or `None` when
/// the queue is empty.
///
/// `evnt_multi` blocks on an empty queue, so `MU_TIMER` is armed alongside
/// `MU_MESAG`: the call returns either a message or a timer event and can never
/// hang. The [`ScriptHost`] clock advances one millisecond per input pump, so a
/// small deadline is a small, bounded number of idle passes.
fn next_mesag(pb: &mut Aespb) -> Option<[i16; 8]> {
    let mut buf = Box::new([0i16; 8]);
    pb.intin.fill(0);
    pb.intin[0] = MU_MESAG | MU_TIMER;
    pb.intin[14] = 20; // ms
    pb.addrin[0] = buf.as_mut_ptr() as u64;
    pb.call(OP_EVNT_MULTI, 16, 7, 1);
    pb.addrin[0] = 0;
    ((pb.intout[0] & MU_MESAG) != 0).then_some(*buf)
}

fn form_dial(pb: &mut Aespb, fmd: i16, little: (i16, i16, i16, i16), big: (i16, i16, i16, i16)) {
    pb.intin.fill(0);
    pb.intin[0] = fmd;
    pb.intin[1] = little.0;
    pb.intin[2] = little.1;
    pb.intin[3] = little.2;
    pb.intin[4] = little.3;
    pb.intin[5] = big.0;
    pb.intin[6] = big.1;
    pb.intin[7] = big.2;
    pb.intin[8] = big.3;
    pb.call(OP_FORM_DIAL, 9, 1, 0);
}

/// Consume every packet still queued, so a later `move_to` is not driving the
/// pointer through a backlog.
fn drain(host: &'static ScriptHost, pb: &mut Aespb) {
    for _ in 0..64 {
        if host.script.lock().unwrap().is_empty() {
            return;
        }
        pb.call(OP_GRAF_MKSTATE, 0, 5, 0);
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
        pb.call(OP_GRAF_MKSTATE, 0, 5, 0);
    }
}

// ── geometry ──────────────────────────────────────────────────────────────

/// Window A — opened first, so it ends up *below* B.
const RA: (i16, i16, i16, i16) = (40, 60, 400, 300);
/// Window B — opened second, overlapping A's bottom-right corner and reaching
/// out onto the bare desktop to A's right.
const RB: (i16, i16, i16, i16) = (240, 180, 340, 250);
/// A patch of bare desktop inside RB but outside RA, clear of the background
/// tree's icon boxes: what the AES *must* repaint.
const BARE: (i16, i16, i16, i16) = (460, 300, 110, 120);

const KIND: i16 = NAME | CLOSER | FULLER | MOVER | SIZER;

/// `r` shrunk by `n` pixels on every side — the interior of a work area, so a
/// 3D frame's inner bevel row is never counted as "the work area was painted".
fn inset(r: (i16, i16, i16, i16), n: i16) -> (i16, i16, i16, i16) {
    (r.0 + n, r.1 + n, r.2 - 2 * n, r.3 - 2 * n)
}

/// Assert that not one pixel of `work` changed, and that `bare` did change —
/// the two halves of `do_walk`'s clip (`aes/gemwmlib.c:286-312`).
fn assert_clipped(
    b: &BufBackend,
    before: &[u32],
    work: (i16, i16, i16, i16),
    bare: (i16, i16, i16, i16),
    what: &str,
) {
    let (n, first) = b.diff_in(before, work);
    assert_eq!(
        n, 0,
        "{what}: {n} of {} pixels inside the covering window's work area changed \
         (first {first:?}); EmuTOS clips w_drawdesk to the DESKTOP window's owner \
         rectangle list, which does not contain a rectangle another open window \
         owns (do_walk, aes/gemwmlib.c:286-312; newrect/mkrect, aes/gemwrect.c:136-205). \
         The window's content is restored by its owner on WM_REDRAW, not by the AES \
         (w_update/w_redraw, aes/gemwmlib.c:1004-1048,887-907)",
        work.2 as i32 * work.3 as i32,
    );
    let still = b.marked_in(bare);
    assert_eq!(
        still, 0,
        "{what}: {still} of {} pixels of *bare desktop* inside the damage rectangle \
         were left unpainted; the desktop window owns them, so w_drawdesk must draw \
         the background tree there (aes/gemwmlib.c:340-359)",
        bare.2 as i32 * bare.3 as i32,
    );
}

// ── 1. wind_close ─────────────────────────────────────────────────────────

/// Closing the top window repaints the desktop it uncovered — and nothing else.
///
/// `wm_close` → `draw_change` (`aes/gemwmlib.c:1438-1459,1058-1223`) rebuilds
/// the rectangle lists first (`:1129`), so by the time `w_drawdesk(&c)` runs
/// (`:1217-1219`) the closed window's rectangle has gone back to whoever is
/// underneath: the desktop where nothing else covers it, window A where A does.
/// A gets its part back as a `WM_REDRAW` (`w_update`/`w_redraw`,
/// `:1004-1048,887-907`), not as a background repaint.
#[test]
fn closing_a_window_does_not_paint_the_window_below() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let host = rig();
    let b = &host.raster;
    let mut pb = fresh(host);

    let ha = open_window(&mut pb, KIND, RA);
    let hb = open_window(&mut pb, KIND, RB);
    let work_a = wind_get_rect(&mut pb, ha, WF_WXYWH);
    assert!(work_a.2 > 0 && work_a.3 > 0, "A must have a work area");

    // A's owner has drawn its content; the bare desktop is marked so the test
    // can tell "repainted" from "left alone".
    b.paint(inset(work_a, 2), MARK);
    b.paint(BARE, MARK);
    let before = b.snapshot();

    pb.intin.fill(0);
    pb.intin[0] = hb;
    pb.call(OP_WIND_CLOSE, 1, 1, 0);

    assert_clipped(b, &before, inset(work_a, 2), BARE, "wind_close");
}

// ── 2. wind_open ──────────────────────────────────────────────────────────

/// Opening a window paints its frame and asks the owner for the inside; it
/// never paints the inside itself.
///
/// On an open `draw_change` takes the "grow that is a superset or an open"
/// branch (`aes/gemwmlib.c:1160-1170`), which leaves `start == w_handle`, so
/// the `if (start == DESKWH) w_drawdesk(&c)` at `:1217-1219` does not even run.
/// `w_update` then draws the frame (`w_cpwalk`) and posts `WM_REDRAW`
/// (`:1004-1048`). The work area keeps whatever was on the screen until the
/// owner answers.
#[test]
fn opening_a_window_does_not_paint_its_work_area() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let host = rig();
    let b = &host.raster;
    let mut pb = fresh(host);

    let _ha = open_window(&mut pb, KIND, RA);
    // Mark the whole rectangle B is about to occupy.
    b.paint(RB, MARK);
    let before = b.snapshot();

    let hb = open_window(&mut pb, KIND, RB);
    let work_b = wind_get_rect(&mut pb, hb, WF_WXYWH);
    assert!(work_b.2 > 0 && work_b.3 > 0, "B must have a work area");

    let (n, first) = b.diff_in(&before, inset(work_b, 2));
    assert_eq!(
        n, 0,
        "wind_open painted {n} of {} work-area pixels (first {first:?}); on an open \
         EmuTOS never calls w_drawdesk at all — `start` stays at the opening window \
         (aes/gemwmlib.c:1160-1170,1217-1219) — and w_cpwalk draws only the frame \
         objects, W_WORK being a borderless G_IBOX (aes/gemwmlib.c:67-68,87-88; \
         aes/gemoblib.c:663-670)",
        work_b.2 as i32 * work_b.3 as i32,
    );
}

// ── 3. wind_set(WF_CURRXYWH) ──────────────────────────────────────────────

/// The programmatic move: the window leaves, the desktop it uncovered is
/// repainted, the window it uncovered is not.
#[test]
fn a_programmatic_move_does_not_paint_the_window_below() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let host = rig();
    let b = &host.raster;
    let mut pb = fresh(host);

    let ha = open_window(&mut pb, KIND, RA);
    let hb = open_window(&mut pb, KIND, RB);
    let work_a = wind_get_rect(&mut pb, ha, WF_WXYWH);

    b.paint(inset(work_a, 2), MARK);
    b.paint((RB.0 + 5, RA.1 + RA.3 + 5, 50, 60), MARK);
    let before = b.snapshot();

    // Move B down-right, off A entirely; the damage is old ∪ new.
    pb.intin.fill(0);
    pb.intin[0] = hb;
    pb.intin[1] = WF_CXYWH;
    pb.intin[2] = 300;
    pb.intin[3] = 220;
    pb.intin[4] = RB.2;
    pb.intin[5] = RB.3;
    pb.call(OP_WIND_SET, 6, 1, 0);

    // B's new rectangle covers part of A again; only the part of A's work area
    // that no window covers is asserted untouched. The bare-desktop probe has
    // to be somewhere the *new* B does not cover either: the bottom-left slice
    // of B's old rectangle, which is below A and left of the new B.
    let uncovered = (work_a.0, work_a.1, (300 - work_a.0).max(0), work_a.3);
    let bare = (RB.0 + 5, RA.1 + RA.3 + 5, 50, 60);
    assert_clipped(b, &before, inset(uncovered, 2), bare, "wind_set(WF_CURRXYWH)");
}

// ── 4. form_dial(FMD_FINISH) ──────────────────────────────────────────────

/// Taking a dialog down repaints the desktop under it, not the windows under
/// it: `fm_dial(FMD_FINISH)` is `w_drawdesk(pt)` + `w_update(DESKWH, pt, …)`
/// (`aes/gemfmlib.c:375-378`), i.e. the same `do_walk` clip plus a `WM_REDRAW`
/// fan-out to every window the dialog covered.
#[test]
fn form_dial_finish_does_not_paint_the_windows_under_the_dialog() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let host = rig();
    let b = &host.raster;
    let mut pb = fresh(host);

    let ha = open_window(&mut pb, KIND, RA);
    let work_a = wind_get_rect(&mut pb, ha, WF_WXYWH);

    b.paint(inset(work_a, 2), MARK);
    b.paint(BARE, MARK);
    let before = b.snapshot();

    // A dialog rectangle that spans A and the bare desktop to its right.
    form_dial(&mut pb, FMD_FINISH, (300, 220, 40, 40), (20, 40, 600, 420));

    assert_clipped(b, &before, inset(work_a, 2), BARE, "form_dial(FMD_FINISH)");
}

/// `do_walk`'s *first* clip: `rc_intersect(&gl_rfull, pc)`
/// (`aes/gemwmlib.c:294-298`). `gl_rfull` is the screen minus the menu row
/// (`aes/gemgraf.c:423-424`, set at `gsx_start` whether or not a menu bar is
/// installed), so `w_drawdesk` never touches the top `gl_hbox` rows — not on a
/// full-screen `form_dial(FMD_FINISH)` either. Removing a menu bar does not
/// repaint that strip (`mn_bar`, `aes/gemmnlib.c:724-728`); installing one
/// does.
#[test]
fn the_background_repaint_stops_at_the_menu_row() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let host = rig();
    let b = &host.raster;
    let mut pb = fresh(host);

    b.paint((0, 0, SCREEN_W as i16, SCREEN_H as i16), MARK);
    let before = b.snapshot();

    form_dial(&mut pb, FMD_FINISH, (300, 220, 40, 40), (0, 0, SCREEN_W as i16, SCREEN_H as i16));

    let menu = (0, 0, SCREEN_W as i16, GL_HBOX);
    let (n, first) = b.diff_in(&before, menu);
    assert_eq!(
        n, 0,
        "form_dial(FMD_FINISH) painted {n} pixels of the menu row (first {first:?}); \
         do_walk intersects its clip with gl_rfull first (aes/gemwmlib.c:294-298), and \
         gl_rfull starts at gl_hbox = {GL_HBOX} (aes/gemgraf.c:412,423-424)",
    );
    // Below the menu row the background *is* repainted.
    let still = b.marked_in((0, GL_HBOX, SCREEN_W as i16, SCREEN_H as i16 - GL_HBOX));
    assert_eq!(still, 0, "everything below the menu row must be repainted ({still} px left)");
}

// ── 5. the mover drag ─────────────────────────────────────────────────────

/// Window A, and window B straddling A's left edge: the strip B vacates when it
/// is dragged to the right falls half on the bare desktop and half on A.
const DA: (i16, i16, i16, i16) = (220, 40, 390, 400);
const DB: (i16, i16, i16, i16) = (200, 150, 260, 180);
/// How far the drag moves B to the right.
const DRAG: i16 = 40;

/// Moving a window by its mover repaints the strip it vacated — but only the
/// part of that strip the desktop still owns.
///
/// The mover drag itself paints nothing at all: it is an XOR outline
/// (`gr_dragbox`, `aes/gemgrlib.c:243-267`) that `gr_wait` complements away
/// again, and the window is moved **once, at the end**, when the owner answers
/// `WM_MOVED` with `wind_set(WF_CURRXYWH)` (`hctl_window` → `ct_msgup`,
/// `aes/gemctrl.c:237-238,314`). That `wm_set` runs `draw_change`
/// (`aes/gemwmlib.c:1673-1675`), whose `w_drawdesk(&c)` (`:1217-1219`) comes
/// *after* the rectangle lists were rebuilt for the new position (`:1129`) and
/// is clipped by `do_walk` to what the desktop window owns. Half this strip is
/// desktop and gets the background; the other half is window A's and must be
/// left for A's `WM_REDRAW`.
#[test]
fn the_mover_drag_repaints_only_desktop_it_uncovered() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let host = rig();
    let b = &host.raster;
    let mut pb = fresh(host);

    let ha = open_window(&mut pb, KIND, DA);
    let hb = open_window(&mut pb, KIND, DB);
    let work_b = wind_get_rect(&mut pb, hb, WF_WXYWH);
    assert!(
        DB.1 > DA.1 && DB.1 + DB.3 < DA.1 + DA.3 && DB.0 < DA.0 && DA.0 < DB.0 + DRAG,
        "B must straddle A's left edge and sit inside A vertically"
    );

    // Grab the mover well to the right of the strip B will vacate, so the
    // mouse sprite never lands in the measured region.
    let (gx, gy) = (DB.0 + DB.2 - 40, DB.1 + 6);
    move_to(host, &mut pb, gx, gy, 0);
    host.push(Packet { dx: 0, dy: 0, buttons: 1 });
    pb.call(OP_GRAF_MKSTATE, 0, 5, 0); // observe the press

    // B's owner has drawn its content, so a background repaint into the strip
    // B leaves behind is unmistakable.
    b.paint(work_b, MARK);
    let before = b.snapshot();

    // The press is already in (observed above); the drag movement arrives
    // *after* the application has armed its wait. `ev_multi` drains the fork
    // ring first (`chkkbd(); forker();`, `aes/gemevlib.c:169-170` — one
    // `pump_input` at the head of `evnt_multi`, A92), so one idle packet
    // stands in for that drain and the DRAG packet is the first thing the
    // wait itself sees, with the pointer still on the mover.
    host.push(Packet { dx: 0, dy: 0, buttons: 1 });
    host.push(Packet { dx: DRAG, dy: 0, buttons: 1 });
    host.push(Packet { dx: 0, dy: 0, buttons: 0 }); // release
    for _ in 0..8 {
        host.push(Packet { dx: 0, dy: 0, buttons: 0 });
    }

    // The first call runs the drag (the button is down, so it cannot return
    // until `intercept` has handled it); the loop then walks past whatever
    // `wind_open` left in the FIFO to the `WM_MOVED` the drag posted.
    let mut msg = Box::new([0i16; 8]);
    for _ in 0..8 {
        pb.intin.fill(0);
        pb.intin[0] = MU_MESAG | MU_BUTTON | MU_KEYBD;
        pb.intin[1] = 0x0102;
        pb.intin[2] = 3;
        pb.intin[3] = 0;
        pb.addrin[0] = msg.as_mut_ptr() as u64;
        pb.call(OP_EVNT_MULTI, 16, 7, 1);
        pb.addrin[0] = 0;
        if msg[0] == 28 {
            break;
        }
    }
    assert_eq!(msg[0], 28, "the drag must post WM_MOVED (28), got {msg:?}");

    // Nothing may have been painted yet — the drag draws the XOR outline and
    // erases it again (`gr_wait`, aes/gemgrlib.c:169-191).
    let (during, at) = b.diff_in(&before, work_b);
    assert_eq!(
        during, 0,
        "the mover drag itself changed {during} pixels of the window's work area \
         (first {at:?}); `gr_dragbox` moves nothing and paints nothing"
    );

    // The owner answers WM_MOVED — this is where the screen changes.
    pb.intin.fill(0);
    pb.intin[0] = hb;
    pb.intin[1] = WF_CXYWH;
    pb.intin[2] = msg[4];
    pb.intin[3] = msg[5];
    pb.intin[4] = msg[6];
    pb.intin[5] = msg[7];
    pb.call(OP_WIND_SET, 6, 1, 0);

    // The strip B vacated: the columns left of B's *new* border rectangle,
    // restricted to B's former work area. Left of A's border it is the
    // desktop's; from A's *work* area on it is A's. A's border columns belong to
    // neither half — the AES draws its frame there itself (`w_cpwalk` per window
    // inside the damage rectangle, `aes/gemwmlib.c:1029-1030`).
    let work_a = wind_get_rect(&mut pb, ha, WF_WXYWH);
    let desk_strip = (work_b.0, work_b.1, DA.0 - work_b.0, work_b.3);
    let a_strip = (work_a.0, work_b.1, DB.0 + DRAG - work_a.0, work_b.3);
    assert!(desk_strip.2 > 0 && a_strip.2 > 0, "both halves of the strip must be non-empty");

    let (n, first) = b.diff_in(&before, a_strip);
    assert_eq!(
        n, 0,
        "the move painted {n} of {} pixels into the half of the vacated strip \
         that window A owns (first {first:?}). w_drawdesk walks the DESKTOP window's \
         owner rectangle list, which after `newrect` no longer contains anything an \
         open window covers (aes/gemwmlib.c:286-312,340-359; aes/gemwrect.c:136-205); \
         A gets that strip back as a WM_REDRAW, not as a background repaint",
        a_strip.2 as i32 * a_strip.3 as i32,
    );

    let still = b.marked_in(desk_strip);
    assert_eq!(
        still, 0,
        "{still} of {} pixels of the vacated strip that the *desktop* owns were left \
         unpainted; that half is exactly what w_drawdesk is for \
         (aes/gemwmlib.c:340-359)",
        desk_strip.2 as i32 * desk_strip.3 as i32,
    );
}

// ── 6. the rectangle-list iterator and the window handle ──────────────────

const WF_FIRSTXYWH: i16 = 11;
const WF_NEXTXYWH: i16 = 12;
/// Out of the table's range (`NUM_WIN` is 16, handles 0..=15).
const OUT_OF_RANGE: i16 = -1;
/// Inside the table but not occupied by any window — the second half of
/// `get_pwin`'s test (`VF_INUSE`). The rig opens two windows, so the last slot
/// is free; the test asserts that before it relies on it.
const FREE_SLOT: i16 = 15;

/// `wind_get(handle, field)` → `(ret, (x, y, w, h))`.
fn wind_get_ret_rect(pb: &mut Aespb, handle: i16, field: i16) -> (i16, (i16, i16, i16, i16)) {
    pb.intin.fill(0);
    pb.intin[0] = handle;
    pb.intin[1] = field;
    pb.call(OP_WIND_GET, 2, 5, 0);
    (pb.intout[0], (pb.intout[1], pb.intout[2], pb.intout[3], pb.intout[4]))
}

/// The `WF_FIRSTXYWH`/`WF_NEXTXYWH` iterator belongs to a *window*, so a query
/// on a handle no window occupies must be refused **before** the list is
/// touched — otherwise it damages the iteration another window is in the
/// middle of.
///
/// EmuTOS `wm_get` looks the `WINDOW` up with `get_pwin` and returns FALSE for
/// every field except `WF_TOP`, `WF_SCREEN` and `WF_DCOLOR` when the handle is
/// out of range or the slot has no `VF_INUSE` (`aes/gemwmlib.c:1513-1524`,
/// `get_pwin` `:1391-1404`). Only past that does it reach `w_owns`, the one
/// place that reads `w_rlist` and writes `w_rnext` (`:1230-1240,1556-1561`).
/// Handle 0 is a legal handle (the desktop window, `:1290` with
/// `w_setup:205-228`), and so is a created-but-closed one.
///
/// Two levels that this test deliberately keeps apart: `ret = FALSE` is the
/// *handler's* answer. It does not mean the caller keeps its four previous
/// words — the dispatcher copies `min(OUT_LEN, O_SIZE)` words back from its own
/// local buffer either way (`aes/gemsuper.c:432-445`;
/// `tos-api/src/aes/mod.rs:132-140,263`). The assertions below are therefore
/// about the return value and about the *iterator state*, and say nothing about
/// `int_out[1..4]` of a refused call.
#[test]
fn an_invalid_handle_does_not_disturb_the_rectangle_list() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let host = rig();
    let mut pb = fresh(host);

    let ha = open_window(&mut pb, KIND, RA);
    let _hb = open_window(&mut pb, KIND, RB); // covers A's bottom-right corner

    // Both stand-ins really are invalid handles, by the same `get_pwin` rule
    // the measured TT cases 175-178 exercise on a rectangle field.
    for bad in [OUT_OF_RANGE, FREE_SLOT] {
        let (ret, _) = wind_get_ret_rect(&mut pb, bad, WF_WXYWH);
        assert_eq!(
            ret, 0,
            "wind_get({bad}, WF_WXYWH) must be FALSE — the handle is out of range or \
             its slot is not in use (aes/gemwmlib.c:1391-1404,1521-1523)"
        );
    }

    // The undisturbed sequence. B breaks A, so A owns at least two pieces.
    let (r0, p0) = wind_get_ret_rect(&mut pb, ha, WF_FIRSTXYWH);
    let (r1, p1) = wind_get_ret_rect(&mut pb, ha, WF_NEXTXYWH);
    assert_eq!((r0, r1), (1, 1), "FIRST/NEXT on a valid handle are TRUE");
    assert!(
        p0.2 > 0 && p0.3 > 0 && p1.2 > 0 && p1.3 > 0,
        "the rig must leave A with at least two visible pieces, got {p0:?} then {p1:?}"
    );

    // 1. FIRST on an invalid handle must not rebuild the list. If it does, the
    //    iteration window A is in the middle of is emptied.
    let (ret, again) = wind_get_ret_rect(&mut pb, ha, WF_FIRSTXYWH);
    assert_eq!((ret, again), (1, p0), "FIRST must restart A's own list");
    let (bad_first, _) = wind_get_ret_rect(&mut pb, OUT_OF_RANGE, WF_FIRSTXYWH);
    assert_eq!(
        bad_first, 0,
        "wind_get({OUT_OF_RANGE}, WF_FIRSTXYWH) must be FALSE (aes/gemwmlib.c:1521-1523)"
    );
    let (ret, after_bad_first) = wind_get_ret_rect(&mut pb, ha, WF_NEXTXYWH);
    assert_eq!(
        (ret, after_bad_first),
        (1, p1),
        "FIRST on an invalid handle threw away A's rectangle list: A's next piece is \
         {after_bad_first:?} instead of {p1:?}. `wm_get` refuses the handle before \
         `w_owns` sees a list at all (aes/gemwmlib.c:1513-1524,1556-1561)"
    );

    // 2. NEXT on an invalid handle must not advance the cursor — it would eat
    //    a piece belonging to another window.
    let (ret, again) = wind_get_ret_rect(&mut pb, ha, WF_FIRSTXYWH);
    assert_eq!((ret, again), (1, p0), "FIRST must restart A's own list");
    let (bad_next, _) = wind_get_ret_rect(&mut pb, FREE_SLOT, WF_NEXTXYWH);
    assert_eq!(
        bad_next, 0,
        "wind_get({FREE_SLOT}, WF_NEXTXYWH) must be FALSE — the slot is not in use \
         (aes/gemwmlib.c:1391-1404,1521-1523)"
    );
    let (ret, after_bad_next) = wind_get_ret_rect(&mut pb, ha, WF_NEXTXYWH);
    assert_eq!(
        (ret, after_bad_next),
        (1, p1),
        "NEXT on an invalid handle consumed one of A's pieces: A's next piece is \
         {after_bad_next:?} instead of {p1:?}. `w_owns` is only reached with a window \
         (aes/gemwmlib.c:1230-1240,1513-1524)"
    );
}

// ── 7. the order the rectangle list breaks a window into ──────────────────

/// The window that gets broken here. Opened first, so it ends up *below*.
const ORD_UNDER: (i16, i16, i16, i16) = (40, 100, 400, 300);
/// The window that breaks it — opened second, so it is above. Placed so that
/// exactly **three** of the four possible remainder pieces exist: its top edge
/// lies *above* the other window's, so there is **no TOP piece**, while LEFT,
/// RIGHT and BOTTOM all survive.
///
/// That missing TOP piece is the whole point. `brkrct` emits the pieces in the
/// order of its constants, and TOP is the first of them — so as long as a TOP
/// piece exists, every candidate order starts with it and the order is
/// unobservable. Take TOP away and the first reported piece is LEFT in EmuTOS
/// and BOTTOM in a TOP/BOTTOM/LEFT/RIGHT implementation.
///
/// One breaking window means at most four pieces, well under the list's
/// capacity, so nothing here depends on the truncation behaviour (A21) or on
/// the order of several breaking windows (A22).
const ORD_OVER: (i16, i16, i16, i16) = (140, 60, 200, 200);

/// EmuTOS's four piece kinds, under their own constant values — `TOP 0`,
/// `LEFT 1`, `RIGHT 2`, `BOTTOM 3` (`aes/gemwrect.c:30-33`). `brkrct` fills
/// `have_piece[]` indexed by them and appends with `for (i = 0; i < 4; i++)`
/// (`:113-125`), so *ascending index order* **is** the emission order.
const P_TOP: usize = 0;
const P_LEFT: usize = 1;
const P_RIGHT: usize = 2;
const P_BOTTOM: usize = 3;
const P_NAME: [&str; 4] = ["TOP", "LEFT", "RIGHT", "BOTTOM"];

/// Which of `mkpiece`'s four kinds the reported piece `p` is, decided only by
/// which side of the covering window `over` it lies on — never by comparing
/// coordinates against a frozen expectation.
///
/// `mkpiece` (`aes/gemwrect.c:64-98`) puts TOP strictly above the covering
/// rectangle (`h = new.y - old.y`), LEFT strictly left of it
/// (`w = new.x - old.x`), RIGHT at `x = new.x + new.w` and BOTTOM at
/// `y = new.y + new.h`. Clipping a piece to the work area on the way out
/// (`w_owns`, `aes/gemwmlib.c:1230-1240`) can only shrink it, so it cannot move
/// a piece to the other side of `over`; and a later drop-shadow enlargement of
/// the covering rectangle only pushes RIGHT further right and BOTTOM further
/// down. The classification therefore survives the remaining fix stages, which
/// a coordinate expectation would not.
fn piece_kind(p: (i16, i16, i16, i16), over: (i16, i16, i16, i16)) -> usize {
    let mut hit = Vec::new();
    if p.1 + p.3 <= over.1 {
        hit.push(P_TOP);
    }
    if p.0 + p.2 <= over.0 {
        hit.push(P_LEFT);
    }
    if p.0 >= over.0 + over.2 {
        hit.push(P_RIGHT);
    }
    if p.1 >= over.1 + over.3 {
        hit.push(P_BOTTOM);
    }
    assert_eq!(
        hit.len(),
        1,
        "piece {p:?} is not on exactly one side of the covering window {over:?} \
         (matched {:?}) — the rig's geometry no longer isolates the four \
         `mkpiece` kinds (aes/gemwrect.c:64-98)",
        hit.iter().map(|&i| P_NAME[i]).collect::<Vec<_>>()
    );
    hit[0]
}

/// The pieces a broken window owns come out in one fixed order: **TOP, LEFT,
/// RIGHT, BOTTOM**.
///
/// EmuTOS decides this in `brkrct` (`aes/gemwrect.c:100-135`): it fills
/// `have_piece[TOP|LEFT|RIGHT|BOTTOM]` and then appends the pieces it has in
/// an ascending `for (i = 0; i < 4; i++)` loop, with the constants fixed at
/// `TOP 0`, `LEFT 1`, `RIGHT 2`, `BOTTOM 3` (`:30-33`). The pieces replace the
/// broken rectangle *in place* in the list (`brkrct` re-links `p->o_link`,
/// `mkrect` `:136-162`), so the relative order of everything else is kept and
/// this is the order the list is read in.
///
/// Every reader sees it: the draw walk `do_walk` (`aes/gemwmlib.c:286-312`) —
/// which also fixes the call order of a `G_USERDEF` object — the public
/// `WF_FIRSTXYWH`/`WF_NEXTXYWH` iteration through `w_owns` (`:1230-1240`,
/// `:1556-1561`), and the redraw union `w_union` (`:869-884`). It is algorithm
/// semantics, not a device parameter: the constants do not depend on the
/// resolution, the font or the shadow size.
///
/// The test names the pieces by the side of the covering window they lie on,
/// so it freezes no coordinate — only the order, which is what is under test.
#[test]
fn the_owner_rectangle_list_is_broken_in_top_left_right_bottom_order() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let host = rig();
    let mut pb = fresh(host);

    let under = open_window(&mut pb, KIND, ORD_UNDER);
    let over = open_window(&mut pb, KIND, ORD_OVER);

    // Take the geometry the AES actually gave the two windows, not the request.
    let u = wind_get_rect(&mut pb, under, WF_CXYWH);
    let o = wind_get_rect(&mut pb, over, WF_CXYWH);

    // The rig's precondition, in `brkrct`'s own terms (`aes/gemwrect.c:113-118`):
    // `have_piece[TOP] = (new.y > old.y)` must be false, the other three true.
    // Adding a drop shadow later grows both rectangles' w/h by the same amount,
    // so none of these four comparisons changes sign.
    assert!(
        o.1 <= u.1,
        "rig broken: the covering window must start at or above the other one \
         ({o:?} vs {u:?}), otherwise a TOP piece exists and hides the order"
    );
    assert!(o.0 > u.0, "rig broken: no LEFT piece for {o:?} in {u:?}");
    assert!(o.0 + o.2 < u.0 + u.2, "rig broken: no RIGHT piece for {o:?} in {u:?}");
    assert!(o.1 + o.3 < u.1 + u.3, "rig broken: no BOTTOM piece for {o:?} in {u:?}");

    // Walk the whole list through the public iterator.
    let mut order: Vec<usize> = Vec::new();
    let mut seen: Vec<(i16, i16, i16, i16)> = Vec::new();
    let (mut ret, mut p) = wind_get_ret_rect(&mut pb, under, WF_FIRSTXYWH);
    for _ in 0..20 {
        if ret == 0 || p.2 <= 0 || p.3 <= 0 {
            break;
        }
        seen.push(p);
        order.push(piece_kind(p, o));
        let step = wind_get_ret_rect(&mut pb, under, WF_NEXTXYWH);
        ret = step.0;
        p = step.1;
    }

    let named: Vec<&str> = order.iter().map(|&i| P_NAME[i]).collect();
    assert!(
        !order.contains(&P_TOP),
        "rig broken: a TOP piece turned up ({named:?}, {seen:?}) — with one the \
         order cannot be told apart"
    );
    assert_eq!(
        order,
        vec![P_LEFT, P_RIGHT, P_BOTTOM],
        "the owner rectangle list came out as {named:?} ({seen:?}). `brkrct` \
         appends the pieces it has in ascending constant order TOP(0), LEFT(1), \
         RIGHT(2), BOTTOM(3) (aes/gemwrect.c:30-33,113-125); with no TOP piece \
         that is LEFT, RIGHT, BOTTOM"
    );
    assert!(
        order.windows(2).all(|w| w[0] < w[1]),
        "the pieces must come out in ascending `brkrct` constant order, got {named:?}"
    );
}

// ── 8. the rectangle list and its cursor belong to one window ─────────────

/// Walk `handle`'s owner rectangle list from the start through the public
/// iterator and collect every piece, stopping at the end sentinel.
///
/// EmuTOS `w_owns` reports the end by zeroing only `g_w`/`g_h`
/// (`aes/gemwmlib.c:1240`), so an empty piece — not `ret` — is the terminator.
fn walk_owner_list(pb: &mut Aespb, handle: i16) -> Vec<(i16, i16, i16, i16)> {
    let mut out = Vec::new();
    let (mut ret, mut p) = wind_get_ret_rect(pb, handle, WF_FIRSTXYWH);
    for _ in 0..20 {
        if ret == 0 || p.2 <= 0 || p.3 <= 0 {
            break;
        }
        out.push(p);
        let step = wind_get_ret_rect(pb, handle, WF_NEXTXYWH);
        ret = step.0;
        p = step.1;
    }
    out
}

/// Is `p` completely inside `outer`?
fn contains_rect(outer: (i16, i16, i16, i16), p: (i16, i16, i16, i16)) -> bool {
    p.0 >= outer.0 && p.1 >= outer.1 && p.0 + p.2 <= outer.0 + outer.2 && p.1 + p.3 <= outer.1 + outer.3
}

/// Do `a` and `b` share at least one pixel?
fn rects_overlap(a: (i16, i16, i16, i16), b: (i16, i16, i16, i16)) -> bool {
    a.0 < b.0 + b.2 && b.0 < a.0 + a.2 && a.1 < b.1 + b.3 && b.1 < a.1 + a.3
}

/// The intersection of `a` and `b`, or `None` when they do not touch — EmuTOS
/// `rc_intersect` reduced to the half a test needs (`util/rectfunc.c:61-78`).
fn overlap(a: (i16, i16, i16, i16), b: (i16, i16, i16, i16)) -> Option<(i16, i16, i16, i16)> {
    let x0 = a.0.max(b.0);
    let y0 = a.1.max(b.1);
    let x1 = (a.0 + a.2).min(b.0 + b.2);
    let y1 = (a.1 + a.3).min(b.1 + b.3);
    (x1 > x0 && y1 > y0).then_some((x0, y0, x1 - x0, y1 - y0))
}

/// The rectangle list and the cursor into it are **per window**: starting the
/// iteration on one window must not disturb an iteration another window is in
/// the middle of.
///
/// EmuTOS keeps both inside the window: `ORECT *w_rlist` (the owner rectangle
/// list) and `ORECT *w_rnext` (the search-next cursor) are fields of the
/// `WINDOW` struct (`aes/gemlib.h:84-85`). `wm_get` picks the *requested*
/// window with `get_pwin` (`aes/gemwmlib.c:1513`) and takes `pwin->w_rlist`
/// for `WF_FIRSTXYWH` or `pwin->w_rnext` for `WF_NEXTXYWH` (`:1556-1561`);
/// `w_owns` advances that same window's link (`:1230-1240`). A `FIRST` on B
/// therefore cannot touch A's cursor.
///
/// This is ownership, not geometry: it depends on neither the resolution, nor
/// a drop shadow, nor the order the pieces are emitted in. The test accordingly
/// freezes no coordinate — it compares the interleaved sequence against each
/// window's own undisturbed sequence **from the same run**, exactly as the
/// invalid-handle test in section 6 does.
#[test]
fn the_rectangle_list_and_its_cursor_belong_to_one_window() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let host = rig();
    let mut pb = fresh(host);

    // Two valid, overlapping windows: B covers A's bottom-right corner, so A is
    // broken into more than one piece while B, being on top, keeps one whole
    // piece. Both handles are valid, so the handle check of section 6 cannot be
    // what answers here.
    let ha = open_window(&mut pb, KIND, RA);
    let hb = open_window(&mut pb, KIND, RB);

    let work_a = wind_get_rect(&mut pb, ha, WF_WXYWH);
    let work_b = wind_get_rect(&mut pb, hb, WF_WXYWH);
    let curr_b = wind_get_rect(&mut pb, hb, WF_CXYWH);

    // Each window's own sequence, taken without interference. This is the
    // yardstick — no expected number appears anywhere in this test.
    let solo_a = walk_owner_list(&mut pb, ha);
    let solo_b = walk_owner_list(&mut pb, hb);

    assert!(
        solo_a.len() >= 2,
        "rig broken: B must break A into at least two pieces, otherwise there is no \
         second piece for NEXT(A) to return; A's list is {solo_a:?}"
    );
    assert!(!solo_b.is_empty(), "rig broken: B must own at least one visible piece");

    // Whose piece is whose, told from the geometry rather than from a literal:
    // a piece of A lies in A's work area and clear of B (B is above it), a piece
    // of B lies in B's work area. `w_owns` clips every piece it hands out to the
    // window's own work rectangle (`aes/gemwmlib.c:1556-1561`), and A's pieces
    // are what is left of it after the window above was subtracted.
    for p in &solo_a {
        assert!(
            contains_rect(work_a, *p) && !rects_overlap(*p, curr_b),
            "piece {p:?} reported for A is not inside A's work area {work_a:?} clear \
             of the window above it {curr_b:?}"
        );
    }
    for p in &solo_b {
        assert!(
            contains_rect(work_b, *p),
            "piece {p:?} reported for B is not inside B's work area {work_b:?}"
        );
    }

    // The interleaved sequence: start A, start B, then continue A.
    let (ret_a0, a0) = wind_get_ret_rect(&mut pb, ha, WF_FIRSTXYWH);
    let (ret_b0, b0) = wind_get_ret_rect(&mut pb, hb, WF_FIRSTXYWH);
    let (ret_a1, a1) = wind_get_ret_rect(&mut pb, ha, WF_NEXTXYWH);

    assert_eq!(
        (ret_a0, ret_b0, ret_a1),
        (1, 1, 1),
        "FIRST/NEXT on two valid handles are all TRUE (aes/gemwmlib.c:1513-1524)"
    );
    assert_eq!(
        a0, solo_a[0],
        "FIRST(A) must start A's own list, got {a0:?} instead of {:?}",
        solo_a[0]
    );
    assert_eq!(
        b0, solo_b[0],
        "FIRST(B) must answer with B's own first piece, got {b0:?} instead of {:?}",
        solo_b[0]
    );
    assert_eq!(
        a1, solo_a[1],
        "NEXT(A) after a FIRST(B) in between returned {a1:?} instead of A's second \
         piece {:?}. A rectangle list of {solo_a:?} for A and {solo_b:?} for B: \
         {} — the list and the cursor are fields of the window, so starting B's \
         iteration cannot reach A's (`w_rlist`/`w_rnext`, aes/gemlib.h:84-85; \
         `wm_get` picks the requested window, aes/gemwmlib.c:1513,1556-1561; \
         `w_owns` advances that window's link, :1230-1240)",
        solo_a[1],
        if a1.2 == 0 && a1.3 == 0 {
            "that is the end-of-list sentinel, so B's FIRST overwrote A's list or cursor"
        } else if solo_b.contains(&a1) {
            "that is one of B's pieces, so A's query is reading B's list"
        } else {
            "that is neither A's second piece nor B's"
        }
    );
}

// ── 9. the breaking rectangle includes the drop shadow ────────────────────

/// The window that gets broken. Opened first, so it ends up *below*.
const SHD_UNDER: (i16, i16, i16, i16) = (40, 60, 440, 360);

/// The window drop-shadow size, taken from the **one** place the AES keeps it
/// (`tos_api::aes::window::DROP_SHADOW_SIZE`, EmuTOS `DROP_SHADOW_SIZE`,
/// `aes/gemwmlib.c:49`). The expectations below are written in terms of `S`,
/// never as a literal, so changing the size for another screen profile does not
/// turn into a test round.
const S: i16 = tos_api::aes::window::DROP_SHADOW_SIZE;

/// A window's drop shadow is part of what it takes away from the windows below
/// it: the rectangle that breaks their owner rectangle lists is `WS_TRUE`, not
/// `WS_CURR`.
///
/// `newrect` uses `w_getsize(WS_TRUE, wh, …)` twice — once for the rectangle
/// this window breaks everybody else's list with (`aes/gemwrect.c:190`) and
/// once for its own starting rectangle (`:203`) — and `w_getsize` grows `WS_TRUE`
/// by `DROP_SHADOW_SIZE` in `g_w`/`g_h` only, leaving `g_x`/`g_y` alone
/// (`aes/gemwmlib.c:253-261`). So the shadow is charged on exactly two of the
/// four sides, and the remainder pieces `mkpiece` cuts say which
/// (`aes/gemwrect.c:64-98`):
///
/// | piece | `mkpiece` edge | with the shadow |
/// |---|---|---|
/// | TOP    | `h = new.y - old.y`                     | ends at `over.y`, unchanged |
/// | LEFT   | `w = new.x - old.x`                     | ends at `over.x`, unchanged |
/// | RIGHT  | `x = new.x + new.w`                     | starts `S` further right |
/// | BOTTOM | `y = new.y + new.h`                     | starts `S` further down |
///
/// Testing all four is what makes this a statement about the *relation* rather
/// than about two numbers: a fix that simply enlarged every rectangle would
/// move the TOP and LEFT edges too and fail here.
///
/// The reported pieces are the work-area intersection (`w_owns`,
/// `aes/gemwmlib.c:1230-1240`), which can only shrink a piece, never move it
/// across the covering window — the covering window is placed strictly inside
/// the other one's work area so no edge under test is the one that gets
/// clipped. Nothing here freezes a coordinate that the remaining fix stages
/// change: the pieces are named by the side of the covering window they lie on
/// (`piece_kind`, section 7), the geometry is read back from the AES rather
/// than assumed, and only pieces *before* the end of the list are read, so the
/// end-of-list words (A15) stay out of it.
#[test]
fn the_breaking_rectangle_of_a_covering_window_includes_its_drop_shadow() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let host = rig();
    let mut pb = fresh(host);

    let under = open_window(&mut pb, KIND, SHD_UNDER);
    // Place the covering window strictly inside the other one's *work* area, so
    // that all four remainder pieces exist and none of the four edges under
    // test is cut away by `w_owns`' work-rectangle clip. Read the work area
    // back from the AES instead of computing it from the font metrics.
    let wu = wind_get_rect(&mut pb, under, WF_WXYWH);
    let inset = 60 + S;
    let over_at = (wu.0 + inset, wu.1 + inset, wu.2 - 2 * inset, wu.3 - 2 * inset);
    assert!(
        over_at.2 > 0 && over_at.3 > 0,
        "rig broken: {SHD_UNDER:?} gives the work area {wu:?}, too small to hold a \
         covering window inset by {inset} on every side"
    );
    let over = open_window(&mut pb, KIND, over_at);

    // The geometry the AES actually handed out, not the request.
    let u = wind_get_rect(&mut pb, under, WF_CXYWH);
    let o = wind_get_rect(&mut pb, over, WF_CXYWH);

    // `brkrct`'s own precondition for all four pieces (`aes/gemwrect.c:113-118`),
    // with the shadow already charged on the two sides that get it.
    assert!(o.1 > u.1, "rig broken: no TOP piece for {o:?} in {u:?}");
    assert!(o.0 > u.0, "rig broken: no LEFT piece for {o:?} in {u:?}");
    assert!(o.0 + o.2 + S < wu.0 + wu.2, "rig broken: no RIGHT piece for {o:?} in {wu:?}");
    assert!(o.1 + o.3 + S < wu.1 + wu.3, "rig broken: no BOTTOM piece for {o:?} in {wu:?}");

    let pieces = walk_owner_list(&mut pb, under);
    let mut by_kind: Vec<Option<(i16, i16, i16, i16)>> = vec![None; 4];
    for p in &pieces {
        let k = piece_kind(*p, o);
        assert!(
            by_kind[k].is_none(),
            "two {} pieces in {pieces:?} — one covering window can produce at most \
             one of each (`have_piece[]`, aes/gemwrect.c:107-125)",
            P_NAME[k]
        );
        by_kind[k] = Some(*p);
    }
    let named: Vec<&str> = (0..4).filter(|&i| by_kind[i].is_some()).map(|i| P_NAME[i]).collect();
    assert_eq!(
        named,
        vec![P_NAME[P_TOP], P_NAME[P_LEFT], P_NAME[P_RIGHT], P_NAME[P_BOTTOM]],
        "rig broken: all four remainder pieces must exist, got {named:?} from {pieces:?}"
    );

    let top = by_kind[P_TOP].unwrap();
    let left = by_kind[P_LEFT].unwrap();
    let right = by_kind[P_RIGHT].unwrap();
    let bottom = by_kind[P_BOTTOM].unwrap();

    // The two sides the shadow does not grow: `w_getsize` touches `g_w`/`g_h`
    // only, so `new.x` and `new.y` — the edges TOP and LEFT are cut at — are
    // the plain `WS_CURR` ones (`aes/gemwmlib.c:253-261`).
    assert_eq!(
        top.1 + top.3,
        o.1,
        "the TOP piece {top:?} must end where the covering window starts ({}), \
         `mkpiece` TOP: `h = new.y - old.y` (aes/gemwrect.c:79-81). The drop \
         shadow grows `g_h` only and must not move this edge \
         (`w_getsize`, aes/gemwmlib.c:253-261)",
        o.1
    );
    assert_eq!(
        left.0 + left.2,
        o.0,
        "the LEFT piece {left:?} must end where the covering window starts ({}), \
         `mkpiece` LEFT: `w = new.x - old.x` (aes/gemwrect.c:82-84). The drop \
         shadow grows `g_w` only and must not move this edge",
        o.0
    );

    // The two sides it does grow — the point of the test.
    assert_eq!(
        right.0,
        o.0 + o.2 + S,
        "the RIGHT piece {right:?} starts at {}, so the rectangle that broke the \
         list was {} wide instead of {} — the covering window's bare `WS_CURR` \
         rect {o:?} rather than its `WS_TRUE` one. `newrect` breaks with \
         `w_getsize(WS_TRUE, …)` (aes/gemwrect.c:190) and `WS_TRUE` is `WS_CURR` \
         plus DROP_SHADOW_SIZE = {S} (aes/gemwmlib.c:49,253-261); `mkpiece` RIGHT \
         puts the piece at `new.x + new.w` (aes/gemwrect.c:85-88). Reporting the \
         strip as free while the window paints its shadow into it makes the \
         window below draw over that shadow.",
        right.0,
        right.0 - o.0,
        o.2 + S
    );
    assert_eq!(
        bottom.1,
        o.1 + o.3 + S,
        "the BOTTOM piece {bottom:?} starts at {}, so the breaking rectangle was \
         {} tall instead of {} — same relation as for RIGHT, `mkpiece` BOTTOM \
         puts the piece at `new.y + new.h` (aes/gemwrect.c:89-92)",
        bottom.1,
        bottom.1 - o.1,
        o.3 + S
    );
}

// ── 10. the words written at the end of the rectangle list ────────────────

/// The window whose rectangle list runs out. Opened first, so it ends up
/// *below* and is the one that gets broken.
const END_UNDER: (i16, i16, i16, i16) = (40, 60, 400, 300);

/// `w_owns` has **two** different ends and they do not write the same number of
/// words (`aes/gemwmlib.c:1230-1241`):
///
/// ```c
/// static void w_owns(WINDOW *pwin, ORECT *po, GRECT *pt, GRECT *poutwds)
/// {
///     while (po)
///     {
///         rc_copy(&po->o_gr, poutwds);
///         pwin->w_rnext = po = po->o_link;
///         if (rc_intersect(pt, poutwds))
///             return;
///     }
///     poutwds->g_w = poutwds->g_h = 0;
/// }
/// ```
///
/// * **P1 — the loop is never entered** (`po == NULL` on the way in, i.e. the
///   cursor already stands past the last piece). Only `g_w`/`g_h` are written;
///   `g_x`/`g_y` keep whatever the dispatcher's `WORD int_out[O_SIZE]` held,
///   and that array is a plain C `auto` (`aes/gemsuper.c:432`, copied out at
///   `:444-445`). **There is no expected x/y here** — A15 traced the values the
///   TT delivers (31/63 and 0/19) to the call history of the *caller*, not to
///   the window state, over all fifteen records. This test therefore asserts
///   nothing about x/y in that case, on purpose.
/// * **P2 — the loop runs, but no piece survives the work-area clip.**
///   `rc_intersect` writes all four components *even when it returns FALSE*
///   (`util/rectfunc.c:63-78`: the four stores happen before the return
///   expression), so after the last failed attempt `g_x = max(piece.x, work.x)`
///   and `g_y = max(piece.y, work.y)` are still standing when `:1240` zeroes
///   `g_w`/`g_h`. All four words are defined, and x/y are those of the **last
///   piece actually examined**.
///
/// The rig builds P2 and then walks straight into P1 with the next call, so a
/// single run shows both ends and the difference between them.
///
/// **Why no piece can survive here.** `brkrct` cuts the *complement* of the
/// covering rectangle out of the one below (`aes/gemwrect.c:100-135`), so every
/// remainder piece is disjoint from the covering rectangle. A covering `WS_TRUE`
/// rectangle that contains the whole work area of the window below therefore
/// proves — without enumerating the pieces — that none of them can meet that
/// work area.
///
/// **Which piece is the last one.** `brkrct` appends the pieces it has in the
/// ascending constant order TOP(0), LEFT(1), RIGHT(2), BOTTOM(3)
/// (`aes/gemwrect.c:30-33,113-125`). The covering window shares its top-left
/// corner with the one below, so `have_piece[TOP]` and `have_piece[LEFT]` are
/// both false and BOTTOM is the last piece `w_owns` examines. Its origin comes
/// from `mkpiece` (`:64-98`): the common calc keeps the broken rectangle's
/// `g_x` (`:73`), the BOTTOM override puts `g_y` at `new.y + new.h` (`:89-92`)
/// — and `new` is the covering window's `WS_TRUE` rect (`:190`), so the drop
/// shadow is part of that origin.
///
/// **No frozen coordinate.** Both geometries are read back from the AES, the
/// shadow enters as `S` ([`DROP_SHADOW_SIZE`](tos_api::aes::window)), and the
/// expectation is written as the two `mkpiece`/`rc_intersect` formulas over
/// those values.
#[test]
fn the_end_of_the_rectangle_list_writes_the_last_piece_it_examined() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let host = rig();
    let mut pb = fresh(host);

    let under = open_window(&mut pb, KIND, END_UNDER);
    let cu = wind_get_rect(&mut pb, under, WF_CXYWH);
    let wu = wind_get_rect(&mut pb, under, WF_WXYWH);

    // `WS_TRUE` of the window below — `WS_CURR` grown by DROP_SHADOW_SIZE in
    // `g_w`/`g_h` only (`w_getsize`, aes/gemwmlib.c:253-261). That is the
    // rectangle `newrect` starts its owner list from (`aes/gemwrect.c:203`).
    let true_right_u = cu.0 + cu.2 + S;
    let true_bottom_u = cu.1 + cu.3 + S;
    let work_right_u = wu.0 + wu.2;
    let work_bottom_u = wu.1 + wu.3;

    assert!(
        work_right_u < true_right_u && work_bottom_u < true_bottom_u,
        "rig broken: the window below needs room between its work area \
         {wu:?} and its own WS_TRUE rect {:?} for a RIGHT and a BOTTOM piece",
        (cu.0, cu.1, cu.2 + S, cu.3 + S)
    );
    assert!(
        wu.0 > cu.0,
        "rig broken: the work area {wu:?} must start right of the border rect \
         {cu:?}, otherwise `rc_intersect`'s `x = max(piece.x, work.x)` cannot be \
         told from `x = piece.x`"
    );

    // The covering window: the same top-left corner as the one below — so
    // `have_piece[TOP]` and `have_piece[LEFT]` are false (`aes/gemwrect.c:107-108`)
    // — and a `WS_TRUE` rectangle whose right and bottom edges land halfway
    // between the work area of the window below and its own `WS_TRUE` edge: far
    // enough to cover the whole work area, short enough to leave a RIGHT and a
    // BOTTOM piece.
    let over_at = (
        cu.0,
        cu.1,
        (work_right_u + true_right_u) / 2 - S - cu.0,
        (work_bottom_u + true_bottom_u) / 2 - S - cu.1,
    );
    assert!(
        over_at.2 > 0 && over_at.3 > 0,
        "rig broken: {END_UNDER:?} leaves no room for a covering window, got {over_at:?}"
    );
    let over = open_window(&mut pb, KIND, over_at);

    // The geometry the AES actually handed out, not the request.
    let co = wind_get_rect(&mut pb, over, WF_CXYWH);
    let true_right_o = co.0 + co.2 + S;
    let true_bottom_o = co.1 + co.3 + S;

    // `brkrct`'s own four `have_piece[]` tests (`aes/gemwrect.c:107-110`), on the
    // two `WS_TRUE` rectangles it works with (`:190` the breaker, `:203` ours).
    assert!(co.1 <= cu.1, "rig broken: {co:?} in {cu:?} leaves a TOP piece");
    assert!(co.0 <= cu.0, "rig broken: {co:?} in {cu:?} leaves a LEFT piece");
    assert!(
        true_right_o < true_right_u,
        "rig broken: no RIGHT piece — the covering window reaches {true_right_o} \
         and the window below ends at {true_right_u}"
    );
    assert!(
        true_bottom_o < true_bottom_u,
        "rig broken: no BOTTOM piece — the covering window reaches {true_bottom_o} \
         and the window below ends at {true_bottom_u}"
    );

    // The covering `WS_TRUE` rect contains the whole work area below it, so no
    // remainder piece can meet that work area: this is the P2 case.
    assert!(
        co.0 <= wu.0
            && co.1 <= wu.1
            && true_right_o >= work_right_u
            && true_bottom_o >= work_bottom_u,
        "rig broken: the covering WS_TRUE rect {:?} does not contain the work area \
         {wu:?} below it, so some piece would still be visible and the call would \
         not reach the end of the list at all",
        (co.0, co.1, co.2 + S, co.3 + S)
    );

    // P2. BOTTOM is the last piece examined; `mkpiece` gives it the broken
    // rectangle's `g_x` and an origin `new.y + new.h` below the covering
    // `WS_TRUE` rect, and the failing `rc_intersect` clamps both against the
    // work area.
    let last_piece_x = cu.0;
    let last_piece_y = true_bottom_o;
    let want = (last_piece_x.max(wu.0), last_piece_y.max(wu.1), 0, 0);

    let (ret, got) = wind_get_ret_rect(&mut pb, under, WF_FIRSTXYWH);
    assert_eq!(
        ret, 1,
        "wind_get(WF_FIRSTXYWH) on a valid handle is TRUE whatever the list \
         contains — `wm_get` only returns FALSE for an unknown handle or an \
         unknown field (aes/gemwmlib.c:1513-1524,1589-1592)"
    );
    assert_eq!(
        got,
        want,
        "the list of the window below is not empty — it holds the RIGHT piece at \
         x = {true_right_o} and the BOTTOM piece at y = {true_bottom_o} — but none \
         of them meets its work area {wu:?}. `w_owns` walks them all and every \
         `rc_intersect` writes its four components even on FALSE \
         (util/rectfunc.c:63-78), so the words left standing are those of the \
         last piece examined, x = max({last_piece_x}, {}) and y = \
         max({last_piece_y}, {}), with w/h zeroed by aes/gemwmlib.c:1240. \
         Got {got:?}. All four zeroes mean the pieces that fail the clip were \
         dropped when the list was built instead of being walked.",
        wu.0,
        wu.1
    );

    // P1. The cursor now stands past the last piece, so the next call does not
    // enter the loop at all: `g_w`/`g_h` are written, `g_x`/`g_y` are not.
    // **Nothing is asserted about x/y here** — on EmuTOS they are whatever the
    // dispatcher's uninitialised `int_out` held (`aes/gemsuper.c:432,444-445`),
    // which A15 showed to be the caller's own call history rather than any
    // window state. Freezing a value here would invent a contract.
    let (ret_p1, got_p1) = wind_get_ret_rect(&mut pb, under, WF_NEXTXYWH);
    assert_eq!(ret_p1, 1, "wind_get(WF_NEXTXYWH) on a valid handle is TRUE");
    assert_eq!(
        (got_p1.2, got_p1.3),
        (0, 0),
        "past the end of the list `w_owns` never enters its loop and sets \
         `g_w = g_h = 0` (aes/gemwmlib.c:1240); got {got_p1:?}"
    );
}

// ── 11. the rectangle `w_redraw` puts into the WM_REDRAW message ──────────

/// The window that stays partly visible. Opened first, so it is the one the
/// second window breaks.
const MSG_UNDER: (i16, i16, i16, i16) = (40, 100, 400, 300);

/// `w_redraw` sends `damage ∩ hull(**raw** owner rectangle list)` — the work
/// rectangle is a gate, not a clip (`aes/gemwmlib.c:887-907`):
///
/// ```c
/// static void w_redraw(WORD w_handle, GRECT *pt)
/// {
///     GRECT t, d;
///     rc_copy(pt, &t);
///     w_getsize(WS_WORK, w_handle, &d);
///     if (rc_intersect(&t, &d))                       /* :896 */
///         if (w_union(D.w_win[w_handle].w_rlist, &d)) /* :899 */
///             if (rc_intersect(&d, &t))               /* :902 */
///                 ap_sendmsg(..., t.g_x, t.g_y, t.g_w, t.g_h);
/// }
/// ```
///
/// Three separate statements, and the order of the two `rc_intersect` calls is
/// what makes the difference:
///
/// * `rc_intersect(&t, &d)` writes its result into **`d`**, not into `t`
///   (`util/rectfunc.c:61-78` — the second argument is the one that is
///   overwritten). `t` therefore still holds the whole damage rectangle
///   afterwards, and the return value is only asked whether the window's work
///   area is touched at all.
/// * `w_union(w_rlist, &d)` (`:869-884`) then **overwrites `d`** with the hull
///   of the window's owner rectangle list. That list holds the **raw**
///   `WS_TRUE` remainder pieces `newrect` links in (`aes/gemwrect.c:190,203`);
///   nothing clips them to the work area on this path — that clip belongs to
///   `w_owns` and thus to `WF_FIRSTXYWH`/`WF_NEXTXYWH` alone
///   (`gemwmlib.c:1230-1240,1556-1561`), which is how the owner keeps its own
///   painting inside its work area (`desk/desksupp.c:223-229`).
/// * what goes out is `d ∩ t`, i.e. `damage ∩ hull(raw list)`.
///
/// So the rectangle the application is told to repaint reaches over its work
/// area's edge, across its frame and its drop shadow — and it is the hull taken
/// **before** any work clip, which is in general strictly larger than the hull
/// of the clipped pieces.
///
/// The rig separates the two rules on both counts at once. The covering window
/// swallows the whole TOP, LEFT and RIGHT of the window below, so `brkrct`
/// leaves exactly **one** piece, the BOTTOM one (`aes/gemwrect.c:107-125`):
///
/// * the hull is that piece and therefore *not* the whole `WS_TRUE` rectangle
///   of the window below — a run that answered `WS_TRUE ∩ damage` would fail
///   here too;
/// * the piece reaches past the work area on its left, right and bottom edge —
///   a run that answered the work-clipped hull would fail as well.
///
/// The damage rectangle is supplied directly, as the "big" rectangle of
/// `form_dial(FMD_FINISH)` — EmuTOS `fm_dial` (`aes/gemfmlib.c:375-379`) is
/// `w_drawdesk(pt)` + `w_update(DESKWH, pt, DESKWH, FALSE)`, i.e. the very
/// `w_update`→`w_redraw` fan-out under test, over a rectangle of the caller's
/// choosing.
///
/// **No frozen coordinate.** Both window geometries are read back from the AES,
/// the covering window is placed relative to those, the shadow enters as `S`
/// ([`DROP_SHADOW_SIZE`](tos_api::aes::window)) and `gl_rfull` is read as the
/// desktop window's work rectangle. The expectation is the `mkpiece` BOTTOM
/// formula over those values.
#[test]
fn the_redraw_message_spans_the_raw_owner_rectangle_not_the_work_clipped_one() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let host = rig();
    let mut pb = fresh(host);

    let under = open_window(&mut pb, KIND, MSG_UNDER);
    let cu = wind_get_rect(&mut pb, under, WF_CXYWH);
    let wu = wind_get_rect(&mut pb, under, WF_WXYWH);
    assert!(wu.2 > 0 && wu.3 > 0, "the window below must have a work area");

    // `WS_TRUE` of the window below: `WS_CURR` with the drop shadow added to
    // `g_w`/`g_h` only (`w_getsize`, aes/gemwmlib.c:253-261). This is the
    // rectangle `newrect` starts its owner list from (`aes/gemwrect.c:203`).
    let true_u = (cu.0, cu.1, cu.2 + S, cu.3 + S);

    // The covering window: `margin` past the top, the left and the right edge
    // of `true_u`, and with its own `WS_TRUE` bottom edge halfway down the work
    // area below — far enough that only a BOTTOM piece is left, short enough
    // that this piece still meets the work area (`w_redraw`'s gate, `:896`).
    let margin = 8;
    let over_bottom = wu.1 + wu.3 / 2;
    let over_at = (
        cu.0 - margin,
        cu.1 - margin,
        (cu.0 + cu.2 + margin) - (cu.0 - margin),
        over_bottom - S - (cu.1 - margin),
    );
    assert!(
        over_at.0 >= 0 && over_at.1 >= 0 && over_at.2 > 0 && over_at.3 > 0,
        "rig broken: {MSG_UNDER:?} leaves no room for a covering window, got {over_at:?}"
    );
    let over = open_window(&mut pb, KIND, over_at);
    let co = wind_get_rect(&mut pb, over, WF_CXYWH);
    let true_o = (co.0, co.1, co.2 + S, co.3 + S);

    // `brkrct`'s own four `have_piece[]` tests (`aes/gemwrect.c:107-110`) on the
    // two `WS_TRUE` rectangles it works with — `:190` the breaker, `:203` ours.
    assert!(true_u.1 >= true_o.1, "rig broken: {true_o:?} in {true_u:?} leaves a TOP piece");
    assert!(true_u.0 >= true_o.0, "rig broken: {true_o:?} in {true_u:?} leaves a LEFT piece");
    assert!(
        true_u.0 + true_u.2 <= true_o.0 + true_o.2,
        "rig broken: {true_o:?} in {true_u:?} leaves a RIGHT piece"
    );
    assert!(
        true_u.1 + true_u.3 > true_o.1 + true_o.3,
        "rig broken: {true_o:?} covers {true_u:?} completely — no BOTTOM piece, \
         so the window below owns nothing and gets no message at all"
    );

    // The one surviving piece, from `mkpiece` (`aes/gemwrect.c:64-98`): the
    // common calc keeps the broken rectangle's `g_x`/`g_w` (`:71-74`), the
    // BOTTOM override puts `g_y` at `new.y + new.h` and takes the rest of the
    // height (`:89-92`) — `new` being the covering window's `WS_TRUE` rect.
    let piece = (
        true_u.0,
        true_o.1 + true_o.3,
        true_u.2,
        (true_u.1 + true_u.3) - (true_o.1 + true_o.3),
    );

    // Both halves of what this test separates, checked before it runs.
    assert_ne!(
        piece, true_u,
        "rig check: the hull {piece:?} must differ from the whole WS_TRUE rect \
         {true_u:?}, or the test cannot tell `hull(w_rlist)` from `WS_TRUE`"
    );
    let clipped = overlap(piece, wu).unwrap_or_else(|| {
        panic!(
            "rig check: the surviving piece {piece:?} must meet the work area {wu:?}, \
             otherwise `w_redraw`'s gate (aes/gemwmlib.c:896) closes and no message \
             is sent at all"
        )
    });
    assert_ne!(
        clipped, piece,
        "rig check: the piece {piece:?} must reach past the work area {wu:?}, or the \
         test cannot tell the raw hull from the work-clipped one"
    );

    // Whatever the two `wind_open`s queued: taken out first, because same-window
    // redraws coalesce by rectangle union in the FIFO (EmuTOS `doq`,
    // `aes/gemqueue.c:45-56`) and a leftover would be unioned into the one under
    // test.
    for _ in 0..16 {
        if next_mesag(&mut pb).is_none() {
            break;
        }
    }

    // `gl_rfull` — the screen minus the menu row, where every reader takes it
    // from: the desktop window's work rectangle (`wm_init`, `gemwmlib.c:1284-1300`).
    let full = wind_get_rect(&mut pb, 0, WF_WXYWH);
    let want = overlap(piece, full).expect("the surviving piece must be on screen");

    // `w_update(DESKWH, pt, DESKWH, FALSE)` over the whole screen: the damage
    // rectangle is deliberately larger than anything below, so what comes back
    // is the hull itself and not an artefact of the damage.
    let big = (0, 0, SCREEN_W as i16, SCREEN_H as i16);
    form_dial(&mut pb, FMD_FINISH, big, big);

    let mut got = None;
    for _ in 0..8 {
        match next_mesag(&mut pb) {
            Some(m) if m[0] == WM_REDRAW && m[3] == under => {
                got = Some(m);
                break;
            }
            Some(_) => continue,
            None => break,
        }
    }
    let m = got.unwrap_or_else(|| {
        panic!(
            "the window below owns the piece {piece:?} inside the damage rectangle \
             {big:?}, so `w_update`/`w_redraw` must post it a WM_REDRAW \
             (aes/gemwmlib.c:1004-1048,887-907)"
        )
    });
    assert_eq!(
        (m[4], m[5], m[6], m[7]),
        want,
        "the WM_REDRAW rectangle is `damage ∩ w_union(w_rlist)` = {want:?} — the hull \
         of the RAW owner list, taken before any work clip (aes/gemwmlib.c:899-905, \
         869-884; aes/gemwrect.c:190,203). The work-clipped hull would be {:?} and \
         the window's whole WS_TRUE rect {true_u:?}; neither is what EmuTOS sends. \
         `rc_intersect(&t, &d)` at `:896` writes into `d` and only gates the call \
         (util/rectfunc.c:61-78)",
        overlap(clipped, full)
    );
}
