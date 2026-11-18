//! `wind_set(WF_CXYWH)` on an **open** window — the message and blit contract
//! of EmuTOS `draw_change` (`aes/gemwmlib.c:1058-1223`), driven through the
//! dispatcher (A78; spec `docs/api/aes/window-graphics.md` § `wind_set`,
//! "`WF_CXYWH` on an open window").
//!
//! What the reference does, case by case:
//!
//! * **T1** pure move of the top window → `w_move` (`:938-996`) **blits**
//!   `WS_PREV + shadow` to `WS_TRUE` on screen (`bb_screen`, `:970`) and
//!   returns `moved = TRUE`; `w_update` then skips `gl_wtop` (`:1025`), so the
//!   mover gets **no** `WM_REDRAW` — its pixels are already at the new place.
//! * **T2** pure shrink (same corner, subset) → `stop = w_handle`, `moved =
//!   TRUE` (`:1141-1146`): again no `WM_REDRAW` to the window itself.
//! * **T3** grow (superset) → one `WM_REDRAW` with `(pt + shadow) ∩
//!   hull(rlist)` (`:1160-1167,1222`, `w_redraw` `:887-907`) — the regression
//!   guard for the branch that was right before.
//! * **T4** move of a window that is *not* on top → no blit; `w_update` walks
//!   the z-order **from the top down** (`:1021-1043`), so the covering window's
//!   message is queued before the mover's.
//! * **T5** move of a window that reaches past the right screen edge, toward
//!   that edge's opposite (leftwards) → the part that becomes visible was never
//!   on screen, so `w_move` refuses to blit (`:951-957`), `stop = DESKWH` and
//!   the mover **does** get a `WM_REDRAW` for the union. **T5b**: the same
//!   window moved further right *is* blitted (both halves of the `&&`).
//! * **T6** the same rectangle again → `draw_change` returns before drawing
//!   (`:1108-1116`): nothing painted, nothing sent.
//!
//! Harness shape (backend + host + parameter block) copied from
//! `aes_window_content.rs`.

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

const WF_WXYWH: i16 = 4;
const WF_CXYWH: i16 = 5;
const WF_NEWDESK: i16 = 14;
const WM_REDRAW: i16 = 20;

/// The window drop-shadow size, from the one place the AES keeps it (EmuTOS
/// `DROP_SHADOW_SIZE`, `aes/gemwmlib.c:49`).
const S: i16 = tos_api::aes::window::DROP_SHADOW_SIZE;

/// A marker colour written straight into the framebuffer, standing in for
/// "content the owner drew into its work area". Not a palette colour, so
/// nothing the AES paints can produce it by accident.
const MARK: u32 = 0x0011_2233;

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

/// Fresh AES window state (`wind_new`, EmuTOS `wm_new`, `aes/gemwmlib.c:
/// 1875-1893`) + the registered desktop background tree; returns the app id.
fn fresh() -> (&'static ScriptHost, Aespb, i16) {
    let host = rig();
    let mut pb = Aespb::new();
    pb.call(10, 0, 1, 0); // appl_init
    let pid = pb.intout[0];
    pb.intin.fill(0);
    pb.call(109, 0, 1, 0); // wind_new
    tos_api::aes::window::test_clear_msg(pid);

    let tree = desk_tree();
    pb.intin.fill(0);
    pb.intin[0] = 0; // DESKWH
    pb.intin[1] = WF_NEWDESK;
    pb.intin[2] = 0; // root object
    pb.addrin[0] = tree.as_ptr() as u64;
    pb.call(105, 6, 1, 1); // wind_set
    pb.addrin[0] = 0;
    (host, pb, pid)
}

/// `wind_create` + `wind_open` at `r`, returning the handle. Opening order is
/// z order: the window opened last ends up on top.
fn open_window(pb: &mut Aespb, r: (i16, i16, i16, i16)) -> i16 {
    pb.intin.fill(0);
    pb.intin[0] = KIND;
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

/// `wind_set(handle, WF_CXYWH, x, y, w, h)` → `int_out[0]`.
fn set_cxywh(pb: &mut Aespb, handle: i16, r: (i16, i16, i16, i16)) -> i16 {
    pb.intin.fill(0);
    pb.intin[0] = handle;
    pb.intin[1] = WF_CXYWH;
    pb.intin[2] = r.0;
    pb.intin[3] = r.1;
    pb.intin[4] = r.2;
    pb.intin[5] = r.3;
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

/// The `WM_REDRAW`s among `msgs` as `(handle, (x, y, w, h))`.
fn redraws(msgs: &[[i16; 8]]) -> Vec<(i16, (i16, i16, i16, i16))> {
    msgs.iter().filter(|m| m[0] == WM_REDRAW).map(|m| (m[3], (m[4], m[5], m[6], m[7]))).collect()
}

impl BufBackend {
    /// Fill `r` with a raw device value, bypassing the AES — the stand-in for
    /// the owner's own content drawing.
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

    /// Number of pixels inside `r` that hold `MARK`.
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

    /// Number of pixels inside `r` that differ from `before`, and the first
    /// such pixel.
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

    /// The `copy_rect` calls in the op log, as `(src, (dx, dy))`.
    fn blits(&self) -> Vec<String> {
        self.log.lock().unwrap().iter().filter(|l| l.starts_with("copy_rect")).cloned().collect()
    }

    fn start_log(&self) {
        self.log.lock().unwrap().clear();
        self.logging.store(true, Ordering::Relaxed);
    }

    fn stop_log(&self) {
        self.logging.store(false, Ordering::Relaxed);
    }
}

fn area(r: (i16, i16, i16, i16)) -> usize {
    r.2 as usize * r.3 as usize
}

// ── T1: pure move of the top window ───────────────────────────────────────

const R1: (i16, i16, i16, i16) = (30, 40, 320, 240);

/// The window's content travels with it, and the owner is not asked to
/// redraw: `w_move` blits `WS_PREV + shadow` (`aes/gemwmlib.c:945-948,970`)
/// to the new place and `w_update` skips `gl_wtop` because `moved` is `TRUE`
/// (`:994,1025`).
#[test]
fn t1_pure_move_of_the_top_window_blits_and_sends_no_redraw() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb, pid) = fresh();
    let b = &host.raster;
    let h = open_window(&mut pb, R1);
    let work = wind_get_rect(&mut pb, h, WF_WXYWH);
    drain(&mut pb, pid);
    b.paint(work, MARK);

    b.start_log();
    let ret = set_cxywh(&mut pb, h, (60, 80, R1.2, R1.3));
    b.stop_log();
    assert_eq!(ret, 1);

    // The blit: `bb_screen(s.x, s.y, d.x, d.y, s.w, s.h)` with `s = WS_PREV +
    // shadow` and `d = WS_TRUE` (`:945-948,970`), clipped to `gl_rfull` (`:969`).
    let blits = b.blits();
    assert_eq!(
        blits,
        vec![format!("copy_rect src=({},{},{},{}) -> (60,80)", R1.0, R1.1, R1.2 + S, R1.3 + S)],
        "w_move blits the vacated WS_PREV+shadow rectangle to the new WS_TRUE \
         position, once (aes/gemwmlib.c:945-948,970); got {blits:?}"
    );

    // The content is at the new place — every pixel of the new work area.
    let new_work = wind_get_rect(&mut pb, h, WF_WXYWH);
    assert_eq!(new_work, (work.0 + 30, work.1 + 40, work.2, work.3));
    let carried = b.marked_in(new_work);
    assert_eq!(
        carried,
        area(new_work),
        "after a pure move of the top window its whole work area must show the \
         content it had before (the blit carried it): {carried} of {} pixels",
        area(new_work)
    );

    // The strip it vacated belongs to the desktop again and was repainted by
    // `w_drawdesk` (`:1217-1219`): no content left there.
    let vacated_left = (work.0, work.1, 60 - work.0, work.3);
    assert_eq!(b.marked_in(vacated_left), 0, "the vacated strip must be desktop again");

    // …and the owner was not asked for anything.
    let msgs = drain(&mut pb, pid);
    assert!(
        redraws(&msgs).is_empty(),
        "a pure move of the top window is blitted (w_move, aes/gemwmlib.c:938-996) \
         and w_update skips gl_wtop when moved == TRUE (:1025): no WM_REDRAW to \
         the mover. Got {msgs:?}"
    );
}

// ── T2: pure shrink of the top window ─────────────────────────────────────

/// A shrink that is a pure subset: `stop = w_handle`, `moved = TRUE`
/// (`aes/gemwmlib.c:1141-1146`) — the window keeps what it has and is not
/// asked to redraw; nothing is blitted (the size branch has no `w_move`).
#[test]
fn t2_pure_shrink_of_the_top_window_sends_no_redraw() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb, pid) = fresh();
    let b = &host.raster;
    let h = open_window(&mut pb, R1);
    let work = wind_get_rect(&mut pb, h, WF_WXYWH);
    drain(&mut pb, pid);
    b.paint(work, MARK);
    let before = b.snapshot();

    b.start_log();
    let ret = set_cxywh(&mut pb, h, (R1.0, R1.1, 200, 160));
    b.stop_log();
    assert_eq!(ret, 1);
    assert!(b.blits().is_empty(), "a size change is never blitted; got {:?}", b.blits());

    // The kept part of the work area is untouched (the AES paints no window
    // interior, and there is no message asking the owner to).
    let new_work = wind_get_rect(&mut pb, h, WF_WXYWH);
    let inner = (new_work.0 + 2, new_work.1 + 2, new_work.2 - 4, new_work.3 - 4);
    let (n, first) = b.diff_in(&before, inner);
    assert_eq!(n, 0, "the shrunk window's remaining work area changed {n} pixels (first {first:?})");

    let msgs = drain(&mut pb, pid);
    assert!(
        redraws(&msgs).is_empty(),
        "a pure-subset shrink sets stop = w_handle, moved = TRUE (aes/gemwmlib.c:\
         1141-1146), so w_update skips the window (:1025): no WM_REDRAW. Got {msgs:?}"
    );
}

// ── T3: grow of the top window (regression) ───────────────────────────────

/// A superset grow: `c = pt` (`aes/gemwmlib.c:1160-1167`), the tail adds the
/// shadow (`:1213-1215`) and `w_update(w_handle, c, DESKWH, FALSE)` visits the
/// window: one `WM_REDRAW` with `c ∩ hull(rlist)` = its `WS_TRUE` (`:887-907`).
#[test]
fn t3_grow_of_the_top_window_sends_one_redraw_for_the_full_rect() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb, pid) = fresh();
    let b = &host.raster;
    let h = open_window(&mut pb, R1);
    drain(&mut pb, pid);

    b.start_log();
    let ret = set_cxywh(&mut pb, h, (R1.0, R1.1, 400, 300));
    b.stop_log();
    assert_eq!(ret, 1);
    assert!(b.blits().is_empty(), "a size change is never blitted; got {:?}", b.blits());

    let msgs = drain(&mut pb, pid);
    assert_eq!(
        redraws(&msgs),
        vec![(h, (R1.0, R1.1, 400 + S, 300 + S))],
        "a superset grow posts exactly one WM_REDRAW for (pt + shadow) ∩ hull(rlist) \
         = WS_TRUE (aes/gemwmlib.c:1160-1167,1213-1222,887-907). Got {msgs:?}"
    );
}

// ── T4: move of a window that is not on top ───────────────────────────────

const R4_LOW: (i16, i16, i16, i16) = (30, 40, 320, 240);
const R4_TOP: (i16, i16, i16, i16) = (200, 150, 300, 200);

/// No blit for a window that is not `gl_wtop` (`aes/gemwmlib.c:1174`); the
/// damage is `(pt ∪ c) + shadow` (`:1187,1214-1215`) and `w_update` walks the
/// z-order from the top down (`:1021-1043`): the covering window's `WM_REDRAW`
/// comes first, the mover's second.
#[test]
fn t4_move_of_a_lower_window_fans_out_top_down() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb, pid) = fresh();
    let b = &host.raster;
    let h1 = open_window(&mut pb, R4_LOW);
    let h2 = open_window(&mut pb, R4_TOP);
    drain(&mut pb, pid);

    b.start_log();
    let ret = set_cxywh(&mut pb, h1, (50, 60, R4_LOW.2, R4_LOW.3));
    b.stop_log();
    assert_eq!(ret, 1);
    assert!(b.blits().is_empty(), "only gl_wtop is ever blitted (aes/gemwmlib.c:1174); got {:?}", b.blits());

    // damage = (30,40,340,260) + S; h2's hull is its WS_TRUE (200,150,302,202);
    // h1's hull is its new WS_TRUE (50,60,322,242) — see the ticket (A78, T4).
    let msgs = drain(&mut pb, pid);
    assert_eq!(
        redraws(&msgs),
        vec![(h2, (200, 150, 172, 152)), (h1, (50, 60, 320 + S, 240 + S))],
        "w_update visits the windows from W_TREE[ROOT].ob_tail (top) downwards \
         (aes/gemwmlib.c:1021-1043): h2 first, then h1. Got {msgs:?}"
    );
}

// ── T5: move of a window that reaches past the screen edge ────────────────

const R5: (i16, i16, i16, i16) = (400, 100, 320, 240); // x + w = 720 > 640

/// The source reaches past the right edge and the move goes **left**: the
/// columns that come into view were never on screen, so `w_move` unions the
/// two rectangles instead of blitting, `stop = DESKWH` (`aes/gemwmlib.c:
/// 951-957`) and returns `FALSE` (`:994`) — `w_update` visits every window,
/// the mover included, with damage `(s ∪ d) ∩ gl_rfull + shadow`.
#[test]
fn t5_move_leftwards_from_past_the_right_edge_is_not_blitted() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb, pid) = fresh();
    let b = &host.raster;
    let h = open_window(&mut pb, R5);
    drain(&mut pb, pid);

    b.start_log();
    let ret = set_cxywh(&mut pb, h, (300, R5.1, R5.2, R5.3));
    b.stop_log();
    assert_eq!(ret, 1);
    assert!(
        b.blits().is_empty(),
        "source past the right edge and moving left: no blit (aes/gemwmlib.c:951-957); got {:?}",
        b.blits()
    );

    // damage = ((400,100,322,242) ∪ (300,100,322,242)) ∩ gl_rfull + S
    //        = (300,100,340,242) + S = (300,100,342,244); ∩ hull(rlist) = new WS_TRUE.
    let msgs = drain(&mut pb, pid);
    assert_eq!(
        redraws(&msgs),
        vec![(h, (300, R5.1, R5.2 + S, R5.3 + S))],
        "the no-blit branch leaves moved == FALSE, so the mover is redrawn for \
         (s ∪ d) ∩ gl_rfull + shadow ∩ hull(rlist) (aes/gemwmlib.c:954,988,1222). Got {msgs:?}"
    );
}

/// The same window moved further **right**: what stays visible was visible
/// before, so the blit goes ahead with the source clipped to `gl_rfull`
/// (`w_mvfix`, `aes/gemwmlib.c:915-930,963`) and the mover gets no message.
#[test]
fn t5b_move_rightwards_from_past_the_right_edge_is_blitted() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb, pid) = fresh();
    let b = &host.raster;
    let h = open_window(&mut pb, R5);
    drain(&mut pb, pid);

    b.start_log();
    let ret = set_cxywh(&mut pb, h, (420, R5.1, R5.2, R5.3));
    b.stop_log();
    assert_eq!(ret, 1);
    // s = (400,100,322,242) ∩ gl_rfull = (400,100,240,242); d.x = 420.
    let blits = b.blits();
    assert_eq!(
        blits,
        vec![format!("copy_rect src=(400,100,{},{}) -> (420,100)", SCREEN_W as i16 - 400, R5.3 + S)],
        "source past the right edge but moving right: blit the on-screen part \
         (aes/gemwmlib.c:951-970); got {blits:?}"
    );
    let msgs = drain(&mut pb, pid);
    assert!(redraws(&msgs).is_empty(), "blitted → moved == TRUE → no WM_REDRAW to the mover; got {msgs:?}");
}

// ── T6: the same rectangle again ──────────────────────────────────────────

/// `wind_set(WF_CXYWH)` with the rectangle the window already has: same
/// corner, same size, and the window is not being topped — `draw_change`
/// returns before drawing anything (`aes/gemwmlib.c:1108-1116`).
#[test]
fn t6_setting_the_same_rectangle_paints_and_sends_nothing() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (host, mut pb, pid) = fresh();
    let b = &host.raster;
    let h = open_window(&mut pb, R1);
    let work = wind_get_rect(&mut pb, h, WF_WXYWH);
    drain(&mut pb, pid);
    b.paint(work, MARK);
    let before = b.snapshot();

    b.start_log();
    let ret = set_cxywh(&mut pb, h, R1);
    b.stop_log();
    assert_eq!(ret, 1);
    assert!(b.blits().is_empty());
    let (n, first) = b.diff_in(&before, (0, 0, SCREEN_W as i16, SCREEN_H as i16));
    assert_eq!(n, 0, "{n} pixels changed (first {first:?}); draw_change returns at :1115-1116");
    let msgs = drain(&mut pb, pid);
    assert!(msgs.is_empty(), "no message for an unchanged rectangle; got {msgs:?}");
}
