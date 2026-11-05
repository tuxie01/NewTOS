//! Reproduction rig for the real-Pi-4 **"a window empties when it is clicked"**
//! defect: on TeraDesk, clicking a file inside an open directory window leaves
//! the window blank — the listing is gone and never comes back.
//!
//! The gesture reaches the AES as `wind_set(handle, WF_INFO, …)`: TeraDesk's
//! `dir_select()` → `dir_setnws(w, TRUE)` → `dir_info(w)` ends in
//! `wind_set_str(w->xw_handle, WF_INFO, w->info)`
//! (`sdk/apps/teradesk/dir.c:3110-3146,2923-2956,<dir_info>`), i.e. *every*
//! selection change rewrites the info line. It is also reachable through
//! `WF_TOP` and through a click on a window that is not on top.
//!
//! What EmuTOS does with a window's interior on any of those: **nothing**.
//! The window frame is an OBJECT tree (`gl_awind`) whose `W_BOX`, `W_DATA` and
//! `W_WORK` elements are `G_IBOX` (`aes/gemwmlib.c:62-69`) with `ob_spec`
//! `0x00001101` for `W_DATA`/`W_WORK` — border thickness 0 (`:87-88`) — and
//! `ob_draw` skips both the border (`if (th != 0) gr_box(...)`) and the interior
//! fill (`if (obtype != G_IBOX) gr_inside(...)`) for such an object
//! (`aes/gemoblib.c:663-670`). The work area belongs to the application; the AES
//! never paints it, it only asks for it back with `WM_REDRAW`.
//!
//! Harness shape (backend + host + parameter block) copied from
//! `aes_window_drag.rs`, which copied it from `aes_button_dispatch.rs`.

#![allow(clippy::too_many_arguments)]


use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

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

const MU_BUTTON: i16 = 0x0002;
const MU_MESAG: i16 = 0x0010;

/// Seed the backend's pseudo-palette the way the VDI dispatcher does before a
/// colour-sensitive op (`ACTIVE_PALETTE`, `kernel/src/vdi_backend.rs:11-21`):
/// EmuTOS' `videl_palette1` for pens 0..15, as RGB555-in-565 words. Without
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

fn drain(host: &'static ScriptHost, pb: &mut Aespb) {
    for _ in 0..64 {
        if host.script.lock().unwrap().is_empty() {
            return;
        }
        pb.call(79, 0, 5, 0);
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
        pb.call(79, 0, 5, 0); // graf_mkstate — ends in dsptch() → pump_input()
    }
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

/// The desktop-background tree an application registers with
/// `wind_set(0, WF_NEWDESK, tree, root)` — TeraDesk does exactly this
/// (`sdk/apps/teradesk/icon.c:2511`). Root = a patterned box over the whole
/// screen, plus three icon-sized boxes that sit *behind* the test window.
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



// ── pixel helpers ─────────────────────────────────────────────────────────

/// A marker colour written straight into the framebuffer, standing in for
/// "application content the owner drew into its work area". Not a palette
/// colour, so nothing the AES paints can produce it by accident.
const MARK_A: u32 = 0x0011_2233;
const MARK_B: u32 = 0x0044_5566;

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

    fn log_tail(&self, n: usize) -> Vec<String> {
        let l = self.log.lock().unwrap();
        l[l.len().saturating_sub(n)..].to_vec()
    }
}

/// `(x, y, w, h)` of `a` minus every row/column of `b` — used as "the part of
/// A that window B does not cover"; only the simple case the rig needs (B
/// overlaps A's bottom-right corner), so the strip above B and the strip left
/// of B are returned.
fn uncovered_strips(a: (i16, i16, i16, i16), b: (i16, i16, i16, i16)) -> [(i16, i16, i16, i16); 2] {
    let above = (a.0, a.1, a.2, (b.1 - a.1).max(0));
    let left = (a.0, a.1, (b.0 - a.0).max(0), a.3);
    [above, left]
}

/// The overlap of two rectangles, or `None` when they do not touch.
fn overlap(a: (i16, i16, i16, i16), b: (i16, i16, i16, i16)) -> Option<(i16, i16, i16, i16)> {
    let x0 = a.0.max(b.0);
    let y0 = a.1.max(b.1);
    let x1 = (a.0 + a.2).min(b.0 + b.2);
    let y1 = (a.1 + a.3).min(b.1 + b.3);
    (x1 > x0 && y1 > y0).then_some((x0, y0, x1 - x0, y1 - y0))
}

// ── the drop shadow, as a parameter ───────────────────────────────────────

/// The window drop-shadow size, taken from the **one** place the AES keeps it
/// (`tos_api::aes::window::DROP_SHADOW_SIZE`, EmuTOS `DROP_SHADOW_SIZE`,
/// `aes/gemwmlib.c:49`). Every expectation below that depends on the shadow is
/// expressed in terms of `S`, never as a literal, so raising the size for a
/// larger screen profile stays a one-line change instead of a test round.
const S: i16 = tos_api::aes::window::DROP_SHADOW_SIZE;

/// The border thickness the shadow is `2 * th` of — `W_BOX`'s `ob_spec`
/// `0x00011101` gives EmuTOS `th = 1` (`aes/gemwmlib.c:63,84`, decoded in
/// `ob_sst`, `aes/gemobjop.c:54-57`), matching `DROP_SHADOW_SIZE = 2`. Derived
/// from `S` here for the same reason: one value, not two that can drift apart.
const TH: i16 = S / 2;

/// The two bands `ob_draw` paints for a `SHADOWED` object, in terms of the
/// window's border rectangle (`aes/gemoblib.c:889-901`, with `t` the rectangle
/// `gr_inside(&t, th)` left and `2*th = S`):
///
/// - bottom `(x + th, y + h, w - th, S)`
/// - right  `(x + w, y + th, S, h + th)`
///
/// The `th`-wide notch at the bottom left and the one at the top right are
/// painted by neither band; that is EmuTOS behaviour and no assertion may
/// demand a colour there.
fn shadow_bands(r: (i16, i16, i16, i16)) -> [(i16, i16, i16, i16); 2] {
    let bottom = (r.0 + TH, r.1 + r.3, r.2 - TH, S);
    let right = (r.0 + r.2, r.1 + TH, S, r.3 + TH);
    [bottom, right]
}

// ── the rig ───────────────────────────────────────────────────────────────

/// Window A — opened first, so it ends up *below* B.
const RA: (i16, i16, i16, i16) = (60, 60, 340, 250);
/// Window B — opened second, overlapping A's bottom-right corner.
const RB: (i16, i16, i16, i16) = (240, 180, 340, 250);

struct Rig {
    host: &'static ScriptHost,
    pb: Aespb,
    pid: i16,
    ha: i16,
    hb: i16,
}

/// `wind_create` + `wind_open` at `r`, returning the handle.
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
    h
}

/// `wind_get(handle, field)` → `(x, y, w, h)`.
fn wind_get_rect(pb: &mut Aespb, handle: i16, field: i16) -> (i16, i16, i16, i16) {
    pb.intin.fill(0);
    pb.intin[0] = handle;
    pb.intin[1] = field;
    pb.call(104, 2, 5, 0);
    (pb.intout[1], pb.intout[2], pb.intout[3], pb.intout[4])
}

/// `wind_get(0, WF_TOP)` — the handle of the top window.
fn top_handle(pb: &mut Aespb) -> i16 {
    pb.intin.fill(0);
    pb.intin[0] = 0;
    pb.intin[1] = WF_TOP;
    pb.call(104, 2, 5, 0);
    pb.intout[1]
}

/// `wind_set(handle, field, …)` with up to four value words.
fn wind_set(pb: &mut Aespb, handle: i16, field: i16, v: [i16; 4]) {
    pb.intin.fill(0);
    pb.intin[0] = handle;
    pb.intin[1] = field;
    pb.intin[2] = v[0];
    pb.intin[3] = v[1];
    pb.intin[4] = v[2];
    pb.intin[5] = v[3];
    pb.addrin[0] = 0;
    pb.call(105, 6, 1, 0);
}

/// `wind_set(handle, WF_INFO, str)` the way a classic-AES caller packs it:
/// the pointer goes into `int_in[2..]` (gemlib `wind_set_str`), `ain_len` 0.
fn wind_set_str(pb: &mut Aespb, handle: i16, field: i16, s: *const u8) {
    pb.intin.fill(0);
    pb.intin[0] = handle;
    pb.intin[1] = field;
    let p = s as u64;
    for i in 0..4 {
        pb.intin[2 + i] = ((p >> (16 * i)) & 0xFFFF) as u16 as i16;
    }
    pb.addrin[0] = 0;
    pb.call(105, 6, 1, 0);
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

fn setup() -> Rig {
    let host: &'static ScriptHost = Box::leak(Box::new(ScriptHost::new(&[])));
    install(host);
    let mut pb = Aespb::new();
    pb.call(10, 0, 1, 0); // appl_init
    let pid = pb.intout[0];
    seed_palette(&host.raster);

    // Desktop background tree, as TeraDesk registers one.
    let tree = desk_tree();
    pb.intin.fill(0);
    pb.intin[0] = 0; // DESKWH
    pb.intin[1] = WF_NEWDESK;
    pb.intin[2] = 0; // root object
    pb.addrin[0] = tree.as_ptr() as u64;
    pb.call(105, 6, 1, 1); // wind_set
    pb.addrin[0] = 0;

    let kind = NAME | CLOSER | FULLER | MOVER | SIZER | INFO;
    let ha = open_window(&mut pb, kind, RA);
    let hb = open_window(&mut pb, kind, RB);
    // The cursor stays hidden for the whole run: its save-under sprite would
    // otherwise show up in the pixel diffs as "content that changed".
    Rig { host, pb, pid, ha, hb }
}

const INFO: i16 = 0x0010;
const WF_INFO: i16 = 3;
const WF_WXYWH: i16 = 4;
const WF_CXYWH: i16 = 5;
const WF_TOP: i16 = 10;
const WF_NEWDESK: i16 = 14;
const WM_REDRAW: i16 = 20;
const WM_TOPPED: i16 = 21;
const WM_UNTOPPED: i16 = 30;
const MU_TIMER: i16 = 0x0020;

// ── the test ──────────────────────────────────────────────────────────────

/// Three ways to touch a window, none of which may cost it its content.
///
/// 1. `wind_set(WF_INFO)` — the reported gesture (TeraDesk rewrites the info
///    line on every selection change). EmuTOS redraws the `W_INFO` gadget only
///    (`wm_set` → `w_cpwalk(w_handle, W_INFO, MAX_DEPTH, TRUE)`,
///    `aes/gemwmlib.c:1663-1670,1754-1755`); `W_WORK` is a borderless `G_IBOX`
///    and is not drawn at all (`:67-68,87-88`, `aes/gemoblib.c:663-670`).
/// 2. `wind_set(WF_TOP)` on the window that is *already* on top. EmuTOS:
///    `if (w_handle != gl_wtop) wm_mktop(...)` — the whole call is skipped, so
///    not one pixel changes and not one message is sent
///    (`aes/gemwmlib.c:1676-1680`).
/// 3. A click in a window that is *not* on top. EmuTOS's Control Manager sends
///    `WM_UNTOPPED` to the old top's owner and `WM_TOPPED` to the clicked
///    window's owner and **returns without topping anything**
///    (`aes/gemctrl.c:170-178`); the owner tops it with `wind_set(WF_TOP)`,
///    which redraws the two frames and sends the newly-topped window a
///    `WM_REDRAW` for its work area (`draw_change` top branch,
///    `aes/gemwmlib.c:1112-1136` → `w_update`/`w_redraw`, `:1004-1048,887-907`).
#[test]
fn clicking_a_window_never_erases_its_content() {
    let mut rig = setup();
    let b = &rig.host.raster;
    let (ha, hb, pid) = (rig.ha, rig.hb, rig.pid);
    let work_a = wind_get_rect(&mut rig.pb, ha, WF_WXYWH);
    let work_b = wind_get_rect(&mut rig.pb, hb, WF_WXYWH);
    assert!(work_a.2 > 0 && work_b.2 > 0, "both windows must have a work area");
    b.logging.store(true, Ordering::Relaxed);

    // ── 1. the reported gesture: the info line is rewritten ───────────────
    b.paint(work_b, MARK_B);
    tos_api::aes::window::test_clear_msg(pid);
    let before = b.snapshot();
    let info = b"3 items, 12 KB\0";
    wind_set_str(&mut rig.pb, hb, WF_INFO, info.as_ptr());
    let (n, first) = b.diff_in(&before, work_b);
    assert_eq!(
        n, 0,
        "wind_set(WF_INFO) overwrote {n} of {} work-area pixels (first {:?}); EmuTOS \
         redraws only the W_INFO gadget and never paints W_WORK, a borderless G_IBOX \
         (aes/gemwmlib.c:1663-1670,67-68,87-88; aes/gemoblib.c:663-670). Last backend ops:\n{}",
        work_b.2 as i32 * work_b.3 as i32,
        first,
        b.log_tail(12).join("\n")
    );

    // ── 2. wind_set(WF_TOP) on the window that is already on top ──────────
    b.paint(work_a, MARK_A);
    b.paint(work_b, MARK_B);
    tos_api::aes::window::test_clear_msg(pid);
    let before = b.snapshot();
    assert_eq!(top_handle(&mut rig.pb), hb, "B is the top window");
    wind_set(&mut rig.pb, hb, WF_TOP, [hb, 0, 0, 0]);
    let (n, first) = b.diff_in(&before, (0, 0, SCREEN_W as i16, SCREEN_H as i16));
    assert_eq!(
        n, 0,
        "wind_set(WF_TOP) on the already-top window changed {n} screen pixels \
         (first {:?}); EmuTOS skips the whole call: `if (w_handle != gl_wtop)` \
         (aes/gemwmlib.c:1676-1680)",
        first
    );
    assert_eq!(
        tos_api::aes::window::test_msg_count(pid),
        0,
        "a no-op WF_TOP must not queue a message either (aes/gemwmlib.c:1676-1680)"
    );

    // ── 3. a click in the covered window A ────────────────────────────────
    b.paint(work_a, MARK_A);
    b.paint(work_b, MARK_B);
    tos_api::aes::window::test_clear_msg(pid);
    // A point inside A's work area that B does not cover.
    let (cx, cy) = (work_a.0 + 20, work_a.1 + 20);
    move_to(rig.host, &mut rig.pb, cx, cy, 1);
    // The press is in; the release arrives *after* the application has armed
    // its wait. `ev_multi` drains the fork ring first (`chkkbd(); forker();`,
    // `aes/gemevlib.c:169-170` — one `pump_input` at the head of `evnt_multi`,
    // A92), so one held packet stands in for that drain.
    rig.host.push(Packet { dx: 0, dy: 0, buttons: 1 });
    rig.host.push(Packet { dx: 0, dy: 0, buttons: 0 }); // the release
    let before = b.snapshot();

    let mut mbuf = [0i16; 8];
    rig.pb.intin.fill(0);
    rig.pb.intin[0] = MU_MESAG | MU_BUTTON | MU_TIMER;
    rig.pb.intin[1] = 0x0102; // one click, left button
    rig.pb.intin[2] = 1;
    rig.pb.intin[3] = 1; // wait for a press — the button is already down
    rig.pb.intin[14] = 200; // 200 ms timer, so the call cannot hang
    rig.pb.addrin[0] = mbuf.as_mut_ptr() as u64;
    rig.pb.call(25, 16, 7, 1); // evnt_multi
    rig.pb.addrin[0] = 0;

    assert_eq!(
        top_handle(&mut rig.pb),
        hb,
        "the Control Manager must NOT top the window itself — it sends WM_TOPPED \
         and returns (aes/gemctrl.c:170-178); the owner does the topping"
    );
    assert_eq!(
        [mbuf[0], mbuf[3]],
        [WM_UNTOPPED, hb],
        "first message: WM_UNTOPPED for the old top (aes/gemctrl.c:146-153,174)"
    );
    let m = next_msg(&mut rig.pb, pid).expect("WM_TOPPED must follow WM_UNTOPPED");
    assert_eq!(
        [m[0], m[3]],
        [WM_TOPPED, ha],
        "second message: WM_TOPPED for the clicked window (aes/gemctrl.c:177)"
    );
    assert_eq!(tos_api::aes::window::test_msg_count(pid), 0, "no further message yet");

    // The owner answers, as TeraDesk's `wd_type_topped` does
    // (`sdk/apps/teradesk/window.c:2375-2378` → `xw_set_topbot(w, WF_TOP)`).
    wind_set(&mut rig.pb, ha, WF_TOP, [ha, 0, 0, 0]);
    assert_eq!(top_handle(&mut rig.pb), ha, "the owner's WF_TOP tops A");

    // A's content outside B keeps every pixel: the topping repaints frames,
    // not interiors, and above all does not repaint the desktop background
    // over them (EmuTOS's top branch never calls `w_drawdesk` — `start` stays
    // `w_handle`, `aes/gemwmlib.c:1102-1136,1215-1219`).
    for strip in uncovered_strips(work_a, RB) {
        if strip.2 <= 0 || strip.3 <= 0 {
            continue;
        }
        let (n, first) = b.diff_in(&before, strip);
        assert_eq!(
            n, 0,
            "topping A erased {n} pixels of its already-visible content in {strip:?} \
             (first {:?})",
            first
        );
    }
    // B's content outside A likewise survives losing the top spot — **except**
    // under A's drop shadow, which is part of A and is now on top of B.
    //
    // Derivation, not adjustment: `wm_init` puts `SHADOWED` on the root of the
    // window element tree (`W_ACTIVE[ROOT].ob_state = SHADOWED`,
    // `aes/gemwmlib.c:1284`), so every `w_cpwalk`→`do_walk`→`ob_draw` of a
    // window frame paints the two bands of `aes/gemoblib.c:889-901`, and the
    // owner rectangle list lets them through because `newrect` starts from
    // `w_getsize(WS_TRUE, …)` (`aes/gemwrect.c:190,203`, `gemwmlib.c:254-261`).
    // Topping A redraws A's frame with A's list unbroken, so A's right band
    // lands squarely in the part of B we probe here. EmuTOS sends B no
    // `WM_REDRAW` for it either: the top branch's `w_update` runs a single turn
    // for the topped window (`:1109-1136,1004-1048`).
    //
    // So the region splits in two and both halves are asserted — a shadow that
    // is *claimed* by the rectangle list but never painted is exactly the
    // half-fix this assertion exists to catch.
    let strip = (RA.0 + RA.2, work_b.1, work_b.0 + work_b.2 - (RA.0 + RA.2), work_b.3);
    let [bottom_band, right_band] = shadow_bands(RA);
    assert_eq!(
        overlap(bottom_band, strip),
        None,
        "rig check: A's bottom shadow band {bottom_band:?} ends at column \
         {} and must not reach the probed strip {strip:?}",
        RA.0 + RA.2 - 1
    );
    let shaded = overlap(right_band, strip).unwrap_or_else(|| {
        panic!(
            "rig check: A's right shadow band {right_band:?} must cross the probed \
             strip {strip:?}; with DROP_SHADOW_SIZE = {S} it does not, so this \
             assertion would prove nothing"
        )
    });

    // (a) the shadow really is painted, over its whole extent, in the shadow
    //     colour: `W_BOX`'s border colour — `just_draw` fills the `SHADOWED`
    //     bands with the `bcol` it cracked from the `G_IBOX`'s word
    //     (`aes/gemoblib.c:614,894-900`, `gr_crack` bits 15..12,
    //     `aes/gemgraf.c:612-615`). A's `W_BOX` word is the compiled default
    //     `0x1101` (`gl_wtcolor`/`gl_wbcolor`, `aes/gemwmlib.c:1347-1350`), so
    //     pen 1 = BLACK (A131; NewTOS painted pen 8 until the user revoked
    //     that on 2026-11-05).
    const SHADOW: Pel = 1;
    let mut unpainted = Vec::new();
    for y in shaded.1..shaded.1 + shaded.3 {
        for x in shaded.0..shaded.0 + shaded.2 {
            if b.read_pixel(x, y) != SHADOW {
                unpainted.push((x, y, b.read_pixel(x, y)));
            }
        }
    }
    assert!(
        unpainted.is_empty(),
        "{} of {} pixels of A's drop shadow in {shaded:?} were left unpainted \
         (first {:?}). The owner rectangle list claims that strip for A \
         (`w_getsize(WS_TRUE)`, aes/gemwmlib.c:254-261), so nobody else paints \
         it — if A's frame does not paint it either, a strip of stale pixels \
         stays at the window edge. Band from `ob_draw`, aes/gemoblib.c:889-901.",
        unpainted.len(),
        shaded.2 as i32 * shaded.3 as i32,
        unpainted.first()
    );

    // (b) and it stops there: right of the band, B keeps every pixel.
    let beyond = (shaded.0 + shaded.2, strip.1, strip.0 + strip.2 - (shaded.0 + shaded.2), strip.3);
    let (n, first) = b.diff_in(&before, beyond);
    assert_eq!(
        n, 0,
        "untopping B erased {n} pixels of its content in {beyond:?}, outside A's \
         {S}-pixel drop shadow (first {:?})",
        first
    );

    // And A is told to repaint the part it could not see before — its whole
    // `WS_TRUE` rectangle, clipped to the screen.
    //
    // Derived from the source, not adjusted to the observed value:
    //
    // * `wm_set(WF_TOP)` → `wm_mktop` → `draw_change`, top branch
    //   (`aes/gemwmlib.c:1676-1680,1605-1615,1112-1136`). `c` starts as the
    //   window's `WS_CURR` (`:1069-1070`); the `:1198-1201` union with
    //   `gl_wtop`'s current rectangle is a no-op because `gl_wtop` *is* A; the
    //   tail then adds the drop shadow unconditionally (`:1213-1215`), so the
    //   damage rectangle handed to `w_update` is `curr(A)` grown by `S`.
    // * `w_update` clips that to `gl_rfull` (`:1010`) and calls `w_redraw` once
    //   for the topped window (`:1024-1030`).
    // * `w_redraw` (`:887-907`): `rc_intersect(&t, &d)` writes into **d**
    //   (`util/rectfunc.c:61-78`) and only asks whether the work area is
    //   touched at all; `w_union(w_rlist, &d)` (`:869-884`) then **overwrites**
    //   `d` with the hull of the **raw** owner rectangle list — the `WS_TRUE`
    //   remainder pieces `newrect` links in (`aes/gemwrect.c:190,203`), nothing
    //   work-clipped. What goes out is `d ∩ t`.
    // * A is on top and nothing covers it, so its list is the single rectangle
    //   `WS_TRUE(A)` and the hull is that rectangle.
    //
    // The message rectangle is therefore `WS_TRUE(A) ∩ gl_rfull`: work area
    // *plus* frame *plus* drop shadow. Both inputs are read back at run time and
    // the shadow enters as `S`, so nothing here is a frozen number.
    let curr_a = wind_get_rect(&mut rig.pb, ha, WF_CXYWH);
    let true_a = (curr_a.0, curr_a.1, curr_a.2 + S, curr_a.3 + S);
    // `gl_rfull` is where every later reader takes it from: the desktop
    // window's work rectangle (`wm_init`, `aes/gemwmlib.c:1284-1300`).
    let full = wind_get_rect(&mut rig.pb, 0, WF_WXYWH);
    let want = overlap(true_a, full).expect("A must be on screen");
    assert_ne!(
        want, work_a,
        "rig check: with DROP_SHADOW_SIZE = {S} the expected rectangle {want:?} must \
         differ from A's work area {work_a:?}, or this assertion cannot tell the raw \
         owner-list hull from the work-clipped one"
    );
    let m = next_msg(&mut rig.pb, pid).expect("WF_TOP must post a WM_REDRAW for A");
    assert_eq!(
        [m[0], m[3], m[4], m[5], m[6], m[7]],
        [WM_REDRAW, ha, want.0, want.1, want.2, want.3],
        "w_redraw sends `pt ∩ union(w_rlist)`, and `w_rlist` holds the RAW `WS_TRUE` \
         pieces — for the newly topped, unobstructed A that is `WS_TRUE(A) ∩ gl_rfull` \
         = {want:?}, i.e. its work area {work_a:?} plus frame and drop shadow \
         (aes/gemwmlib.c:887-907,869-884,1004-1048; util/rectfunc.c:61-78; \
         aes/gemwrect.c:190,203)"
    );
}
