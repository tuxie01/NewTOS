//! Reproduction rig for the real-Pi-4 **window-resize drawing artefact**.
//!
//! The gesture the user reports: dragging a TeraDesk window's *sizer* makes
//! square holes appear in the desktop background and travel with the drag;
//! dragging the same window by its *mover* is clean.
//!
//! The rig drives the AES exactly as the board does — a [`ScriptHost`] whose
//! `pump_input` is the only thing that advances the pointer (the USB-HID mouse
//! is polled from `Host::pump_input`, `kernel/src/host_env.rs:41`), feeding
//! packets through `tos_api::vdi::feed_relative` (our `mouse_int`) — and
//! snapshots the whole framebuffer once per drag frame, so every pixel the AES
//! writes during the gesture is recoverable and comparable.
//!
//! Harness shape (backend + host) copied from `aes_button_dispatch.rs`.

#![allow(clippy::too_many_arguments)]

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
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
    /// Op counter + "dump every Nth op to PPM" trip, so the *mid-frame*
    /// screen — what the display scans out while the AES is still drawing —
    /// can be looked at, not just the settled end-of-frame state.
    ops: AtomicUsize,
    op_dump: Mutex<Option<(String, usize)>>,
}

impl BufBackend {
    fn new() -> Self {
        BufBackend {
            px: Mutex::new(vec![0; SCREEN_W as usize * SCREEN_H as usize]),
            palette: Mutex::new([0; 256]),
            req: Mutex::new([(0, 0, 0); 256]),
            log: Mutex::new(Vec::new()),
            logging: AtomicBool::new(false),
            ops: AtomicUsize::new(0),
            op_dump: Mutex::new(None),
        }
    }

    fn note(&self, s: String) {
        if self.logging.load(Ordering::Relaxed) {
            self.log.lock().unwrap().push(s);
        }
        let n = self.ops.fetch_add(1, Ordering::Relaxed);
        let dump = self.op_dump.lock().unwrap().clone();
        if let Some((dir, every)) = dump {
            if every > 0 && n.is_multiple_of(every) {
                dump_ppm(&format!("{dir}/op-{n:05}.ppm"), &self.snapshot());
            }
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

    /// Stamp `r` with a raw device value, straight into the buffer — stands in
    /// for the content a window owner has drawn into its work area, so a later
    /// comparison can tell "the AES left it alone" from "the AES painted over
    /// it". Not a backend op: it must not show up in the op log.
    fn paint(&self, r: (i16, i16, i16, i16), v: u32) {
        let mut px = self.px.lock().unwrap();
        for y in r.1..r.1 + r.3 {
            for x in r.0..r.0 + r.2 {
                if Self::inside(x, y) {
                    px[Self::idx(x, y)] = v;
                }
            }
        }
    }

    /// How many pixels of `r` differ from `before`, and the first one found.
    fn diff_in(&self, before: &[u32], r: (i16, i16, i16, i16)) -> (usize, Option<(i16, i16)>) {
        let px = self.px.lock().unwrap();
        let mut n = 0usize;
        let mut first = None;
        for y in r.1..r.1 + r.3 {
            for x in r.0..r.0 + r.2 {
                if Self::inside(x, y) && px[Self::idx(x, y)] != before[Self::idx(x, y)] {
                    n += 1;
                    if first.is_none() {
                        first = Some((x, y));
                    }
                }
            }
        }
        (n, first)
    }
}

/// A device value no palette pen maps to, used to stamp window content.
const MARK: u32 = 0x00FF_00FF;

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
    /// One framebuffer snapshot per `pump_input` while `filming` is set — a
    /// drag frame is exactly one loop turn of `drag_size`/`drag_move`.
    film: Mutex<Vec<(Vec<u32>, i16, i16)>>,
    filming: AtomicBool,
}

impl ScriptHost {
    fn new(script: &[Packet]) -> Self {
        ScriptHost {
            raster: BufBackend::new(),
            script: Mutex::new(script.iter().copied().collect()),
            clock: AtomicU64::new(0),
            film: Mutex::new(Vec::new()),
            filming: AtomicBool::new(false),
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
        // The kernel's trap-side pump starts with the frame tick
        // (`kernel/src/vbl.rs`, TASK/V55): a pointer moved by the previous
        // packet is drawn now. Here every pump stands for a new frame.
        tos_api::vdi::vb_draw(&self.raster);
        if self.filming.load(Ordering::Relaxed) {
            let s = self.raster.snapshot();
            let (mx, my) = tos_api::vdi::mouse_position();
            self.film.lock().unwrap().push((s, mx, my));
        }
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

/// Seed the backend's pseudo-palette the way the VDI dispatcher does before a
/// colour-sensitive op (`ACTIVE_PALETTE`, `kernel/src/vdi_backend.rs:11-21`):
/// EmuTOS' `videl_palette1` for pens 0..15, as RGB555-in-565 words. Without
/// this every pen resolves to 0 and the whole frame renders black.
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

fn dump_ppm(path: &str, px: &[u32]) {
    use std::io::Write;
    let mut f = std::fs::File::create(path).unwrap();
    write!(f, "P6\n{} {}\n255\n", SCREEN_W, SCREEN_H).unwrap();
    let mut buf = Vec::with_capacity(px.len() * 3);
    for &v in px {
        buf.push(((v >> 16) & 0xFF) as u8);
        buf.push(((v >> 8) & 0xFF) as u8);
        buf.push((v & 0xFF) as u8);
    }
    f.write_all(&buf).unwrap();
}

fn dump_dir() -> Option<String> {
    std::env::var("DRAG_DUMP_DIR").ok()
}

/// Consume every packet still queued, so a later `move_to` is not driving the
/// pointer through a backlog.
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
    // The frame after the last packet draws the pointer where it now is
    // (`vb_draw`, TASK/V55).
    tos_api::vdi::vb_draw(&host.raster);
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

struct Rig {
    host: &'static ScriptHost,
    pb: Aespb,
    wh: i16,
}

/// The window under test, in border coordinates.
const WX: i16 = 120;
const WY: i16 = 80;
const WW: i16 = 320;
const WH: i16 = 220;

/// The AES window table, the desktop tree registration and the installed host
/// are process-wide statics, so the cases in this binary must not run
/// concurrently (same guard as `aes_desk_background.rs`).
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
/// window and drains the update locks, so each case starts from the same table.
fn fresh() -> (&'static ScriptHost, Aespb) {
    let host = rig();
    let mut pb = Aespb::new();
    pb.call(10, 0, 1, 0); // appl_init
    host.script.lock().unwrap().clear();
    host.film.lock().unwrap().clear();
    pb.intin.fill(0);
    pb.call(109, 0, 1, 0); // wind_new

    // Register the desktop background tree, then let the AES paint it.
    let tree = desk_tree();
    pb.intin.fill(0);
    pb.intin[0] = 0; // DESKWH
    pb.intin[1] = 14; // WF_NEWDESK
    pb.intin[2] = 0; // root object
    pb.addrin[0] = tree.as_ptr() as u64;
    pb.call(105, 6, 1, 1); // wind_set
    pb.addrin[0] = 0;
    (host, pb)
}

/// `wind_create` + `wind_open` at `r`, returning the handle. Opening order is
/// z order: the window opened last ends up on top.
fn open_window(pb: &mut Aespb, kind: i16, r: (i16, i16, i16, i16)) -> i16 {
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
    pb.intin[1] = r.0;
    pb.intin[2] = r.1;
    pb.intin[3] = r.2;
    pb.intin[4] = r.3;
    pb.call(101, 5, 1, 0); // wind_open
    wh
}

fn mouse_on(pb: &mut Aespb) {
    pb.intin.fill(0);
    pb.intin[0] = 257; // graf_mouse(M_ON)
    pb.call(78, 1, 1, 0);
}

fn setup(kind: i16) -> Rig {
    let (host, mut pb) = fresh();
    let wh = open_window(&mut pb, kind, (WX, WY, WW, WH));
    mouse_on(&mut pb);
    Rig { host, pb, wh }
}

fn evnt_multi(pb: &mut Aespb) {
    pb.intin.fill(0);
    pb.intin[0] = MU_MESAG | MU_BUTTON | MU_KEYBD;
    pb.intin[1] = 0x0102;
    pb.intin[2] = 3;
    pb.intin[3] = 0;
    pb.addrin[0] = 0;
    pb.call(25, 16, 7, 1);
}

/// The blocking `evnt_multi` the drag runs inside — `MU_BUTTON` armed for
/// "both buttons up", so the call cannot return while the button is held and
/// the Control Manager's `intercept` gets to run the drag — but with a real
/// message buffer, so the `WM_MOVED` it posts is handed back instead of being
/// dequeued into a null pointer.
fn evnt_multi_drag(pb: &mut Aespb) -> [i16; 8] {
    let mut buf = Box::new([0i16; 8]);
    pb.intin.fill(0);
    pb.intin[0] = MU_MESAG | MU_BUTTON | MU_KEYBD;
    pb.intin[1] = 0x0102;
    pb.intin[2] = 3;
    pb.intin[3] = 0;
    pb.addrin[0] = buf.as_mut_ptr() as u64;
    pb.call(25, 16, 7, 1);
    pb.addrin[0] = 0;
    *buf
}

/// `evnt_multi` with a real message buffer and a **zero-millisecond timer**, so
/// the call always comes back: `MU_TIMER` fires on the first pass when no
/// message is queued (`msg[0]` stays 0). Used to read what the AES posted
/// without arming `MU_BUTTON`, which would block until the button changes.
fn evnt_multi_msg(pb: &mut Aespb) -> [i16; 8] {
    let mut buf = Box::new([0i16; 8]);
    pb.intin.fill(0);
    pb.intin[0] = MU_MESAG | MU_TIMER;
    pb.intin[14] = 0; // timer lo
    pb.intin[15] = 0; // timer hi
    pb.addrin[0] = buf.as_mut_ptr() as u64;
    pb.call(25, 16, 7, 1);
    pb.addrin[0] = 0;
    *buf
}

/// Empty the owner FIFO so a later `evnt_multi` reports the message the drag
/// posted and not a `WM_REDRAW` left over from `wind_open`. Safe with the
/// button up: `intercept` returns at once (`window.rs` `intercept`, the
/// `mouse_buttons() & 0x01 == 0` guard).
fn drain_messages(pb: &mut Aespb) {
    for _ in 0..16 {
        if evnt_multi_msg(pb)[0] == 0 {
            return;
        }
    }
}


/// EmuTOS' sizer feedback is two XOR outlines and nothing else: `gr_rubwind`
/// (`aes/gemgrlib.c:199-220`) loops `gr_clamp` + `gr_wait`, and `gr_wait` draws
/// `gr_draw` (`:155-166`: the rubber box, plus the work-area box offset by
/// `poff`), waits, and complements it away again. Nothing is repainted and the
/// window is not resized — the owner does that when it answers `WM_SIZED`.
///
/// The regression this pins down: the former live-resize repainted the desktop
/// background (`w_drawdesk` — the background box **and every icon**) over the
/// whole window rectangle on every drag frame, then redrew the frame on top.
/// The window's area therefore showed the background-behind-the-window, icons
/// included, for the length of each frame's redraw — the square "holes" that
/// travel with the drag, reported from a real Pi 4 with TeraDesk. Here that
/// shows up as ~33 000 changed pixels per frame inside the window, all of them
/// off the rubber band.
#[test]
fn sizer_drag_draws_only_the_rubber_band() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let kind = NAME | CLOSER | FULLER | MOVER | SIZER;
    let mut rig = setup(kind);
    let host = rig.host;

    let dd = dump_dir();
    if let Some(d) = &dd {
        dump_ppm(&format!("{d}/00-open.ppm"), &host.raster.snapshot());
    }

    // The two rectangles the rubber band is built from: the window's border
    // rect (WF_CURRXYWH) and its work rect (WF_WORKXYWH); `hctl_window` derives
    // `poff = work - border` from exactly these (`aes/gemctrl.c:247-251`).
    let curr = wind_get_rect(&mut rig.pb, rig.wh, 5);
    let work = wind_get_rect(&mut rig.pb, rig.wh, 4);
    let poff = (work.0 - curr.0, work.1 - curr.1, work.2 - curr.2, work.3 - curr.3);
    // Minimum size (`gemctrl.c:252-257`): the window has top gadgets, so
    // `wm = gl_wbox * 4`; it has no vertical gadgets, so `hm = gl_hchar`.
    let (wmin, hmin) = (19 * 4, 16);

    move_to(host, &mut rig.pb, WX + WW - 4, WY + WH - 4, 0);
    host.push(Packet { dx: 0, dy: 0, buttons: 1 });
    rig.pb.call(79, 0, 5, 0); // graf_mkstate — observe the press

    let before = host.raster.snapshot();
    let (start_mx, start_my) = tos_api::vdi::mouse_position();
    if let Some(d) = &dd {
        dump_ppm(&format!("{d}/01-armed.ppm"), &before);
    }

    // The press is in (observed above); the drag arrives *after* the
    // application has armed its wait. `ev_multi` drains the fork ring first
    // (`chkkbd(); forker();`, `aes/gemevlib.c:169-170` — one `pump_input` at
    // the head of `evnt_multi`, A92), so one held packet stands in for that
    // drain and the first movement is what the wait itself sees.
    host.push(Packet { dx: 0, dy: 0, buttons: 1 });
    for _ in 0..6 {
        host.push(Packet { dx: 10, dy: 8, buttons: 1 });
    }
    for _ in 0..6 {
        host.push(Packet { dx: -10, dy: -8, buttons: 1 });
    }
    host.push(Packet { dx: 0, dy: 0, buttons: 0 }); // release
    for _ in 0..8 {
        host.push(Packet { dx: 0, dy: 0, buttons: 0 });
    }

    host.filming.store(true, Ordering::Relaxed);
    if let Some(d) = &dd {
        std::fs::create_dir_all(format!("{d}/size")).unwrap();
        *host.raster.op_dump.lock().unwrap() = Some((format!("{d}/size"), 4));
    }
    evnt_multi(&mut rig.pb);
    *host.raster.op_dump.lock().unwrap() = None;
    host.filming.store(false, Ordering::Relaxed);

    let film: Vec<(Vec<u32>, i16, i16)> = host.film.lock().unwrap().drain(..).collect();
    eprintln!("== sizer drag: {} frames", film.len());
    let changed = report("size", &before, &film, &dd);
    assert!(film.len() > 4, "the scripted drag must produce several frames");

    // Every pixel that differs from the pre-drag screen must sit on one of the
    // two rubber-band outlines, or under the mouse sprite (16x16 from the
    // hotspot, which `gr_wait`'s `gsx_mon` redraws on top of the band).
    // Every position the mouse sprite occupied during the gesture, including
    // the one it was drawn at in `before`: those 16x16 boxes hold sprite
    // pixels in one image and background in the other, which is the sprite
    // moving, not a repaint. (ARROW's hotspot is (0,0), `aes/mform.rsc`.)
    let mut sprite: Vec<(i16, i16)> = vec![(start_mx, start_my)];
    sprite.extend(film.iter().map(|(_, mx, my)| (*mx, *my)));
    let mut stray: Vec<(usize, i16, i16)> = Vec::new();
    for (fi, x, y) in changed {
        let (mx, my) = (film[fi].1, film[fi].2);
        let bw = (mx - curr.0 + 1).max(wmin);
        let bh = (my - curr.1 + 1).max(hmin);
        let on_band = on_outline(x, y, curr.0, curr.1, bw, bh)
            || on_outline(x, y, curr.0 + poff.0, curr.1 + poff.1, bw + poff.2, bh + poff.3);
        let on_cursor = sprite.iter().any(|(sx, sy)| x >= *sx && x < *sx + 16 && y >= *sy && y < *sy + 16);
        if !on_band && !on_cursor {
            stray.push((fi, x, y));
        }
    }
    if !stray.is_empty() {
        for e in stray.iter().take(20) {
            eprintln!("  stray pixel: frame {} ({},{})", e.0, e.1, e.2);
        }
    }
    assert!(
        stray.is_empty(),
        "the sizer drag painted {} pixels that are not part of the XOR rubber band: \
         EmuTOS `gr_rubwind`/`gr_wait` (aes/gemgrlib.c:169-220) touches nothing but the \
         two outline boxes — no desktop background, no window frame, no work area",
        stray.len()
    );

    // `gr_wait` complements the band away before it returns, and nothing else
    // was drawn: on release the screen is bit-identical to what it was.
    let after = host.raster.snapshot();
    let diff = after.iter().zip(before.iter()).filter(|(a, b)| a != b).count();
    if let Some(d) = &dd {
        dump_ppm(&format!("{d}/99-after.ppm"), &after);
    }
    assert_eq!(
        diff, 0,
        "after the button came up the rubber band must be fully erased and the \
         screen unchanged (the window is resized by the owner's wind_set(WF_CURRXYWH) \
         in answer to WM_SIZED, not by the drag)"
    );

    // And the window itself is untouched by the drag — EmuTOS' AES never
    // resizes it; `ct_msgup` only reports the dragged size to the owner.
    let after_curr = wind_get_rect(&mut rig.pb, rig.wh, 5);
    assert_eq!(
        after_curr, curr,
        "gr_rubwind must not move or resize the window (aes/gemgrlib.c:199-220 \
         writes only *pwend/*phend); the owner does that on WM_SIZED"
    );
}

/// `wind_get(handle, field)` → `(x, y, w, h)`.
fn wind_get_rect(pb: &mut Aespb, handle: i16, field: i16) -> (i16, i16, i16, i16) {
    pb.intin.fill(0);
    pb.intin[0] = handle;
    pb.intin[1] = field;
    pb.call(104, 2, 5, 0);
    (pb.intout[1], pb.intout[2], pb.intout[3], pb.intout[4])
}

/// The 1-pixel outline of `(x, y, w, h)`, as [`set`] membership.
fn on_outline(px: i16, py: i16, x: i16, y: i16, w: i16, h: i16) -> bool {
    if w <= 0 || h <= 0 {
        return false;
    }
    let (xr, yb) = (x + w - 1, y + h - 1);
    let in_x = px >= x && px <= xr;
    let in_y = py >= y && py <= yb;
    (in_x && in_y) && (px == x || px == xr || py == y || py == yb)
}

/// Per drag frame: how many pixels changed against the pre-drag screen, and
/// where. `film` entries are `(screen, mouse_x, mouse_y)` sampled at the top
/// of each `pump_input`, i.e. once per drag-loop turn.
fn report(tag: &str, before: &[u32], film: &[(Vec<u32>, i16, i16)], dd: &Option<String>) -> Vec<(usize, i16, i16)> {
    let mut all_changed = Vec::new();
    for (i, (f, mx, my)) in film.iter().enumerate() {
        let mut changed = 0usize;
        let (mut x0, mut y0, mut x1, mut y1) = (i16::MAX, i16::MAX, i16::MIN, i16::MIN);
        for y in 0..SCREEN_H as i16 {
            for x in 0..SCREEN_W as i16 {
                let k = y as usize * SCREEN_W as usize + x as usize;
                if f[k] != before[k] {
                    changed += 1;
                    all_changed.push((i, x, y));
                    x0 = x0.min(x);
                    y0 = y0.min(y);
                    x1 = x1.max(x);
                    y1 = y1.max(y);
                }
            }
        }
        eprintln!(
            "  [{tag}] frame {i:2}: mouse=({mx},{my}) changed={changed:6} bbox=({x0},{y0})-({x1},{y1})"
        );
        if let Some(d) = dd {
            dump_ppm(&format!("{d}/{tag}-frame-{i:02}.ppm"), f);
        }
    }
    all_changed
}

// ── mover ─────────────────────────────────────────────────────────────────

/// EmuTOS `rc_constrain` (`util/rectfunc.c:32-42`) — pull `t` back inside `c`
/// without changing its size.
fn rc_constrain(c: (i16, i16, i16, i16), t: (i16, i16, i16, i16)) -> (i16, i16, i16, i16) {
    let (mut x, mut y, w, h) = t;
    if x < c.0 {
        x = c.0;
    }
    if y < c.1 {
        y = c.1;
    }
    if x + w > c.0 + c.2 {
        x = (c.0 + c.2) - w;
    }
    if y + h > c.1 + c.3 {
        y = (c.1 + c.3) - h;
    }
    (x, y, w, h)
}

/// The constraining rectangle `hctl_window`'s `W_NAME` arm builds for
/// `gr_dragbox`, 3D branch (`aes/gemctrl.c:229-233`):
/// `r_set(&f, 0, gl_hbox, gl_rscreen.g_w + w - gl_wbox - 6 - 2*ADJ3DSTD, MAX_COORDINATE)`
/// with `gl_hbox = gl_wbox = 19` (`gemgraf.c:412-415` for the 8×16 font),
/// `ADJ3DSTD = 2` (`obdefs.h:136`) and `MAX_COORDINATE = 10000`
/// (`include/sysconf.h:63`).
fn mover_bounds(win_w: i16) -> (i16, i16, i16, i16) {
    (0, 19, SCREEN_W as i16 + win_w - 19 - 6 - 2 * 2, 10000)
}

/// EmuTOS' mover feedback is one XOR outline and nothing else: `gr_dragbox`
/// (`aes/gemgrlib.c:243-267`) takes the grab offset once with
/// `gr_clamp(sx+1, sy+1, 0, 0)`, then loops "box origin = mouse − offset,
/// `rc_constrain`, `gr_wait`". `gr_wait` (`:169-191`) draws the box, waits for
/// the mouse to move or the button to come up, and complements it away again.
/// The window is **not** moved: `hctl_window` reports the dragged origin in
/// `WM_MOVED` (`ct_msgup`, `aes/gemctrl.c:237-238,314`) and the owner applies
/// it with `wind_set(WF_CURRXYWH)`.
#[test]
fn mover_drag_draws_only_the_rubber_band() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let kind = NAME | CLOSER | FULLER | MOVER | SIZER;
    let mut rig = setup(kind);
    let host = rig.host;
    let dd = dump_dir();

    let curr = wind_get_rect(&mut rig.pb, rig.wh, 5);
    drain_messages(&mut rig.pb);

    // Grab the title bar between the closer and the fuller — `ob_find` there
    // answers `W_NAME`, the mover (`aes/gemctrl.c:225`).
    move_to(host, &mut rig.pb, WX + WW / 2, WY + 5, 0);
    host.push(Packet { dx: 0, dy: 0, buttons: 1 });
    rig.pb.call(79, 0, 5, 0); // graf_mkstate — observe the press

    let before = host.raster.snapshot();
    let (start_mx, start_my) = tos_api::vdi::mouse_position();
    // `gr_clamp(sx+1, sy+1, 0, 0, &offx, &offy)` → `max(mouse − origin, 0)`.
    let offx = (start_mx - curr.0).max(0);
    let offy = (start_my - curr.1).max(0);
    let bounds = mover_bounds(curr.2);

    // The press is in (observed above); the drag arrives *after* the
    // application has armed its wait. `ev_multi` drains the fork ring first
    // (`chkkbd(); forker();`, `aes/gemevlib.c:169-170` — one `pump_input` at
    // the head of `evnt_multi`, A92), so one held packet stands in for that
    // drain and the first movement is what the wait itself sees.
    host.push(Packet { dx: 0, dy: 0, buttons: 1 });
    for _ in 0..6 {
        host.push(Packet { dx: 12, dy: 10, buttons: 1 });
    }
    for _ in 0..6 {
        host.push(Packet { dx: -8, dy: 6, buttons: 1 });
    }
    host.push(Packet { dx: 0, dy: 0, buttons: 0 }); // release
    for _ in 0..8 {
        host.push(Packet { dx: 0, dy: 0, buttons: 0 });
    }

    host.filming.store(true, Ordering::Relaxed);
    let msg = evnt_multi_drag(&mut rig.pb);
    host.filming.store(false, Ordering::Relaxed);

    let film: Vec<(Vec<u32>, i16, i16)> = host.film.lock().unwrap().drain(..).collect();
    eprintln!("== mover drag: {} frames", film.len());
    let changed = report("move", &before, &film, &dd);
    assert!(film.len() > 4, "the scripted drag must produce several frames");

    let mut sprite: Vec<(i16, i16)> = vec![(start_mx, start_my)];
    sprite.extend(film.iter().map(|(_, mx, my)| (*mx, *my)));
    let mut stray: Vec<(usize, i16, i16)> = Vec::new();
    for (fi, x, y) in changed {
        let (mx, my) = (film[fi].1, film[fi].2);
        let o = rc_constrain(bounds, (mx - offx, my - offy, curr.2, curr.3));
        let on_band = on_outline(x, y, o.0, o.1, o.2, o.3);
        let on_cursor =
            sprite.iter().any(|(sx, sy)| x >= *sx && x < *sx + 16 && y >= *sy && y < *sy + 16);
        if !on_band && !on_cursor {
            stray.push((fi, x, y));
        }
    }
    if !stray.is_empty() {
        for e in stray.iter().take(20) {
            eprintln!("  stray pixel: frame {} ({},{})", e.0, e.1, e.2);
        }
    }
    assert!(
        stray.is_empty(),
        "the mover drag painted {} pixels that are not part of the XOR outline: \
         EmuTOS `gr_dragbox`/`gr_wait` (aes/gemgrlib.c:169-191,243-267) touches nothing \
         but that one box — no window blit, no desktop background, no frame",
        stray.len()
    );

    // Walk the (now released) pointer back to where it was, so the only thing
    // that could still differ — the mouse sprite, which `gr_wait`'s `gsx_mon`
    // legitimately redraws wherever the pointer is — is back on its own start
    // pixels too. Everything else must be bit-identical.
    let (end_mx, end_my) = tos_api::vdi::mouse_position();
    move_to(host, &mut rig.pb, start_mx, start_my, 0);
    let after = host.raster.snapshot();
    let diff = after.iter().zip(before.iter()).filter(|(a, b)| a != b).count();
    assert_eq!(
        diff, 0,
        "after the button came up the outline must be fully erased and the screen \
         unchanged (the window is moved by the owner's wind_set(WF_CURRXYWH) in \
         answer to WM_MOVED, not by the drag)"
    );

    let after_curr = wind_get_rect(&mut rig.pb, rig.wh, 5);
    assert_eq!(
        after_curr, curr,
        "gr_dragbox must not move the window (aes/gemgrlib.c:243-267 writes only \
         *pdx/*pdy); the owner does that on WM_MOVED"
    );

    // And the reported rectangle is the dragged origin with the *unchanged*
    // size (`ct_msgup(message, owner, wh, x, y, w, h)`, `aes/gemctrl.c:314`),
    // taken from where the pointer was when the button came up.
    let want = rc_constrain(bounds, (end_mx - offx, end_my - offy, curr.2, curr.3));
    assert_eq!(msg[0], 28, "the drag must post WM_MOVED (28), got {msg:?}");
    assert_eq!(
        (msg[3], msg[4], msg[5], msg[6], msg[7]),
        (rig.wh, want.0, want.1, curr.2, curr.3),
        "WM_MOVED must carry the constrained drag origin and the original size"
    );
    // The sender: `ap_sendmsg` stamps `msg[1] = rlr->p_pid` (`gemwmlib.c:855`),
    // and `ct_msgup` runs inside `ctlmgr()` — the Screen Manager, pid 1
    // (`aes/geminit.c:888`, `gemctrl.c:238`; `docs/api/aes/application.md`
    // § "AES process identity and pid", contract (b), A56). Not the
    // application's own 0.
    assert_eq!(msg[1], 1, "WM_MOVED msg[1] is the Screen Manager's pid, 1; got {msg:?}");
}

// Two-window scene: `LOW` is opened first, so it sits below; the window under
// test starts above it and is dragged straight down across it.
const LOW: (i16, i16, i16, i16) = (60, 300, 500, 170);

/// The gesture the user reported from the Pi 4: drag a window across another
/// window. EmuTOS leaves the lower window's pixels alone — the only thing on
/// the screen during the drag is the XOR outline, which is complemented away
/// again (`gr_wait`, `aes/gemgrlib.c:169-191`).
#[test]
fn mover_drag_across_another_window_never_writes_it() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let kind = NAME | CLOSER | FULLER | MOVER | SIZER;
    let (host, mut pb) = fresh();
    let low = open_window(&mut pb, kind, LOW);
    let wh = open_window(&mut pb, kind, (WX, WY, WW, WH));
    mouse_on(&mut pb);
    let dd = dump_dir();

    let curr = wind_get_rect(&mut pb, wh, 5);
    let low_work = wind_get_rect(&mut pb, low, 4); // WF_WORKXYWH
    assert!(low_work.2 > 0 && low_work.3 > 0, "the lower window must have a work area");
    drain_messages(&mut pb);

    move_to(host, &mut pb, WX + WW / 2, WY + 5, 0);
    host.push(Packet { dx: 0, dy: 0, buttons: 1 });
    pb.call(79, 0, 5, 0);

    // The lower window's owner has drawn its content; nothing may overwrite it.
    host.raster.paint(low_work, MARK);
    let before = host.raster.snapshot();
    let (start_mx, start_my) = tos_api::vdi::mouse_position();
    let offx = (start_mx - curr.0).max(0);
    let offy = (start_my - curr.1).max(0);
    let bounds = mover_bounds(curr.2);

    // Straight down, deep into the lower window and back up a little.
    // The press is in (observed above); the drag arrives *after* the
    // application has armed its wait. `ev_multi` drains the fork ring first
    // (`chkkbd(); forker();`, `aes/gemevlib.c:169-170` — one `pump_input` at
    // the head of `evnt_multi`, A92), so one held packet stands in for that
    // drain and the first movement is what the wait itself sees.
    host.push(Packet { dx: 0, dy: 0, buttons: 1 });
    for _ in 0..8 {
        host.push(Packet { dx: 4, dy: 20, buttons: 1 });
    }
    for _ in 0..3 {
        host.push(Packet { dx: -4, dy: -14, buttons: 1 });
    }
    host.push(Packet { dx: 0, dy: 0, buttons: 0 });
    for _ in 0..8 {
        host.push(Packet { dx: 0, dy: 0, buttons: 0 });
    }

    host.filming.store(true, Ordering::Relaxed);
    evnt_multi(&mut pb);
    host.filming.store(false, Ordering::Relaxed);

    let film: Vec<(Vec<u32>, i16, i16)> = host.film.lock().unwrap().drain(..).collect();
    eprintln!("== mover drag across a window: {} frames", film.len());
    let changed = report("cross", &before, &film, &dd);
    assert!(film.len() > 4, "the scripted drag must produce several frames");

    let mut sprite: Vec<(i16, i16)> = vec![(start_mx, start_my)];
    sprite.extend(film.iter().map(|(_, mx, my)| (*mx, *my)));
    let mut inlow = 0usize;
    let mut first: Option<(usize, i16, i16)> = None;
    for (fi, x, y) in changed {
        let inside_low = x >= low_work.0
            && x < low_work.0 + low_work.2
            && y >= low_work.1
            && y < low_work.1 + low_work.3;
        if !inside_low {
            continue;
        }
        let (mx, my) = (film[fi].1, film[fi].2);
        let o = rc_constrain(bounds, (mx - offx, my - offy, curr.2, curr.3));
        let on_band = on_outline(x, y, o.0, o.1, o.2, o.3);
        let on_cursor =
            sprite.iter().any(|(sx, sy)| x >= *sx && x < *sx + 16 && y >= *sy && y < *sy + 16);
        if !on_band && !on_cursor {
            inlow += 1;
            first.get_or_insert((fi, x, y));
        }
    }
    assert_eq!(
        inlow, 0,
        "dragging a window across the one below it wrote {inlow} pixels into that \
         window's work area (first {first:?}). The AES draws the XOR outline and \
         nothing else while the mouse is down (`gr_dragbox`, aes/gemgrlib.c:243-267); \
         a window's content is only ever restored by its owner on WM_REDRAW \
         (w_update/w_redraw, aes/gemwmlib.c:1004-1048,887-907)"
    );

    let (n, at) = host.raster.diff_in(&before, low_work);
    assert_eq!(
        n, 0,
        "after the button came up {n} pixels of the lower window's work area were \
         still different (first {at:?}) — `gr_wait`'s second `gr_draw` complements \
         the outline away, so the screen is exactly what it was"
    );
}

/// The settle: the owner answers `WM_MOVED` with `wind_set(WF_CURRXYWH)`, and
/// **that** is where the screen changes. EmuTOS `wm_set` routes it into
/// `draw_change` (`aes/gemwmlib.c:1673-1675,1058-1223`), which repaints the
/// uncovered desktop (`w_drawdesk(&c)`, `:1217-1219`) and then walks every
/// window with `w_update(start, &c, stop, moved)` (`:1222`), posting a
/// `WM_REDRAW` to each one the damage rectangle reaches (`w_redraw`,
/// `:887-907`). Without that fan-out the window the moved one uncovers keeps
/// showing the moved window's old pixels.
#[test]
fn moving_a_window_off_another_redraws_the_one_underneath() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let kind = NAME | CLOSER | FULLER | MOVER | SIZER;
    let (_host, mut pb) = fresh();
    // The lower window is opened first and is then *covered* by the upper one.
    let low = open_window(&mut pb, kind, LOW);
    let wh = open_window(&mut pb, kind, (WX, 200, WW, 220));
    mouse_on(&mut pb);
    let low_work = wind_get_rect(&mut pb, low, 4);
    let curr = wind_get_rect(&mut pb, wh, 5);
    // The two really do overlap, else the case proves nothing.
    assert!(curr.1 + curr.3 > low_work.1, "the windows must overlap");
    drain_messages(&mut pb);

    // The owner answers WM_MOVED: move the window up and out of the way.
    let moved = (curr.0, 40, curr.2, curr.3);
    pb.intin.fill(0);
    pb.intin[0] = wh;
    pb.intin[1] = 5; // WF_CURRXYWH
    pb.intin[2] = moved.0;
    pb.intin[3] = moved.1;
    pb.intin[4] = moved.2;
    pb.intin[5] = moved.3;
    pb.call(105, 6, 1, 0); // wind_set

    // Collect what the AES posted.
    let mut msgs: Vec<[i16; 8]> = Vec::new();
    for _ in 0..16 {
        let m = evnt_multi_msg(&mut pb);
        if m[0] == 0 {
            break;
        }
        msgs.push(m);
    }
    eprintln!("== after WF_CURRXYWH: {msgs:?}");
    let for_low: Vec<&[i16; 8]> =
        msgs.iter().filter(|m| m[0] == 20 && m[3] == low).collect(); // WM_REDRAW
    assert!(
        !for_low.is_empty(),
        "moving a window off the one below it must post WM_REDRAW to the lower \
         window's owner — EmuTOS `draw_change` ends in `w_update(start, &c, stop, \
         moved)` (aes/gemwmlib.c:1222), which calls `w_redraw` for every window the \
         damage rectangle reaches (:887-907). Got {msgs:?}"
    );
    // ...covering the part the moved window vacated.
    let r = for_low[0];
    let (rx, ry, rw, rh) = (r[4], r[5], r[6], r[7]);
    let want_y = curr.1.max(low_work.1);
    assert!(
        rw > 0 && rh > 0 && ry <= want_y && ry + rh > want_y,
        "the WM_REDRAW rectangle ({rx},{ry},{rw},{rh}) must cover the uncovered \
         part of the lower window (from y={want_y})"
    );
}
