//! Two real-Pi-4 defects seen while running QED, both on the **undraw** side of
//! the AES: what the screen looks like once something the AES drew is taken
//! away again.
//!
//! 1. **A GEM app's background is white, not the desktop pattern.** TeraDesk
//!    launches a program with `wind_set(0, WF_NEWDESK, NULL, 0)` followed by
//!    `dsk_draw()` (`sdk/apps/teradesk/startprg.c:264,294`), and `dsk_draw()` is
//!    nothing but `form_dial(FMD_START)` + `form_dial(FMD_FINISH)` over the
//!    desktop rectangle (`sdk/apps/teradesk/icon.c:241-249`). EmuTOS' `fm_dial`
//!    answers FMD_FINISH with `w_drawdesk(pt)` (`aes/gemfmlib.c:375-378`), and
//!    `w_drawdesk` with no registered desktop tree draws `gl_wtree[ROOT]`
//!    (`aes/gemwmlib.c:340-359`), whose `ob_spec` `wm_init` copied from the
//!    AES's own built-in DESKTOP tree — `4419L` = `0x1143`, i.e. pattern 4 in
//!    colour 3 (`aes/gemwmlib.c:1269-1271`, `aes/gem_rsc.c:551-556`). That is
//!    the classic green dither, not white. The white screen belongs to
//!    `sh_draw(cmd, TRUE)`, which *temporarily* swaps the root's `ob_spec` for
//!    `CLEAR_SCREEN` and puts it straight back (`aes/gemshlib.c:269-286`).
//!
//! 2. **The file selector stays on screen after a file is picked.** EmuTOS'
//!    `fs_input` brackets the whole dialog with `fm_dial(FMD_START, …)` before
//!    the first `ob_draw` and `fm_dial(FMD_FINISH, &gl_rcenter, &gl_rfs)` after
//!    the loop (`aes/gemfslib.c:707-708,918-919`); the FMD_FINISH is what
//!    repaints the desktop under the dialog and re-asks every overlapped window
//!    for its content.
//!
//! Both are exercised here through the real AES trap dispatcher against a pixel
//! buffer. Harness shape (backend + host + parameter block) copied from
//! `aes_window_content.rs`, with a key queue added so the file selector's modal
//! loop can be ended with RETURN.
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
    keys: Mutex<VecDeque<u16>>,
    clock: AtomicU64,
}

impl ScriptHost {
    fn new(script: &[Packet]) -> Self {
        ScriptHost {
            raster: BufBackend::new(),
            script: Mutex::new(script.iter().copied().collect()),
            keys: Mutex::new(VecDeque::new()),
            clock: AtomicU64::new(0),
        }
    }
    /// Queue a key word (`scancode << 8 | ascii`) for `console_try_read_key`.
    fn push_key(&self, k: u16) {
        self.keys.lock().unwrap().push_back(k);
    }
}

impl Host for ScriptHost {
    fn console_write_byte(&self, _byte: u8) {}
    fn console_try_read_key(&self) -> Option<u16> {
        self.keys.lock().unwrap().pop_front()
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

// ── shared rig ────────────────────────────────────────────────────────────

use std::sync::OnceLock;

/// The AES's global state (window table, background registration, mouse) is a
/// set of statics, so the two tests in this binary must not run at the same
/// time.
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

/// Fill `(x, y, w, h)` with a raw device value, bypassing every AES/VDI path —
/// the stand-in for "whatever happened to be on the screen before".
fn paint(b: &BufBackend, r: (i16, i16, i16, i16), argb: u32) {
    let mut px = b.px.lock().unwrap();
    for y in r.1..r.1 + r.3 {
        for x in r.0..r.0 + r.2 {
            if BufBackend::inside(x, y) {
                px[BufBackend::idx(x, y)] = argb;
            }
        }
    }
}

/// A value no palette pen can produce, so "the AES did not paint here" is
/// visible as itself.
const UNPAINTED: u32 = 0x0012_3456;

const FULL: (i16, i16, i16, i16) = (0, 0, SCREEN_W as i16, SCREEN_H as i16);

const FMD_START: i16 = 0;
const FMD_FINISH: i16 = 3;

/// `form_dial(fmd, little, big)` — opcode 51, `int_in[0]` type, `[1..4]` the
/// small rectangle, `[5..8]` the big one.
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
    pb.call(51, 9, 1, 0);
}

/// The pen the AES's own DESKTOP root paints with, per pixel: `ob_spec` `4419L`
/// = `0x1143` (`references/emutos/aes/gem_rsc.c:551-556`), cracked by `gr_crack`
/// (`aes/gemgraf.c:612-628`) into pattern 4 / interior colour 3, and drawn by
/// `gr_rect` as `FIS_PATTERN` in `MD_REPLACE` (`aes/gemgraf.c:547-559`) — the
/// 50 % dither `0xAAAA/0x5555/0xAAAA/0x5555`, green on white.
fn desktop_pen(x: i16, y: i16) -> Pel {
    const PAT: [u16; 4] = [0xAAAA, 0x5555, 0xAAAA, 0x5555];
    let row = PAT[(y.rem_euclid(16) as usize) % 4];
    if row & (0x8000u16 >> (x.rem_euclid(16) as u16)) != 0 {
        3
    } else {
        0
    }
}

// ── 1. the desktop background under a launched GEM app ────────────────────

/// TeraDesk hands the screen to a GEM application by un-registering its own
/// background tree and asking the AES to repaint the desktop rectangle:
/// `wind_set(0, WF_NEWDESK, NULL, 0)` (`sdk/apps/teradesk/startprg.c:264`) then
/// `dsk_draw()` = `form_dial(FMD_START)` + `form_dial(FMD_FINISH)`
/// (`sdk/apps/teradesk/icon.c:241-249`, called at `startprg.c:294`).
///
/// EmuTOS answers the FMD_FINISH with `w_drawdesk(pt)`
/// (`aes/gemfmlib.c:375-378`). With `gl_newdesk == NULL` that draws
/// `gl_wtree[ROOT]` at depth 0 (`aes/gemwmlib.c:340-359`), and `wm_init` gave
/// that root the built-in DESKTOP tree's `ob_spec` — `4419L` = `0x1143`
/// (`aes/gemwmlib.c:1269-1271`, `aes/gem_rsc.c:551-556`). So the background a
/// launched application sits on is the green 50 % dither, *not* white: the white
/// screen is `sh_draw(cmd, TRUE)`'s one-shot `CLEAR_SCREEN`, which swaps the
/// root's `ob_spec` and puts it straight back (`aes/gemshlib.c:269-286`).
///
/// It is the desktop rectangle, not the screen: `w_drawdesk` is one
/// `do_walk(DESKWH, …)`, which starts with `rc_intersect(&gl_rfull, pc)`
/// (`aes/gemwmlib.c:294-298`). `gl_rfull` is the screen **minus the menu row**
/// (`aes/gemgraf.c:423-424`, set at `gsx_start` whether or not a menu bar is
/// installed), so even a full-screen FMD_FINISH leaves the top `gl_hbox` rows
/// exactly as they were — removing a menu bar does not repaint them either
/// (`mn_bar`, `aes/gemmnlib.c:724-728`); installing one does. NewTOS' desktop
/// repaints that strip with `menu::install_menu_bar` after its own whole-screen
/// composite (`programs/desktop/src/redraw.rs:206-213`), exactly as EmuTOS' desktop
/// does with `menu_bar()`.
#[test]
fn the_desktop_background_is_the_aes_desktop_tree_not_white() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let host = rig();
    let mut pb = Aespb::new();
    pb.call(10, 0, 1, 0); // appl_init

    // No desktop tree registered — the state TeraDesk leaves behind.
    pb.intin.fill(0);
    pb.intin[0] = 0; // DESKWH
    pb.intin[1] = WF_NEWDESK;
    pb.intin[2] = 0;
    pb.addrin[0] = 0;
    pb.call(105, 6, 1, 1); // wind_set(0, WF_NEWDESK, NULL, 0)

    paint(&host.raster, FULL, UNPAINTED);
    form_dial(&mut pb, FMD_START, FULL, FULL);
    form_dial(&mut pb, FMD_FINISH, FULL, FULL);

    let mut wrong = 0usize;
    let mut first = None;
    for y in GL_HBOX..SCREEN_H as i16 {
        for x in 0..SCREEN_W as i16 {
            let got = host.raster.read_pixel(x, y);
            let want = desktop_pen(x, y);
            if got != want {
                if first.is_none() {
                    first = Some((x, y, want, got));
                }
                wrong += 1;
            }
        }
    }
    assert_eq!(
        wrong,
        0,
        "form_dial(FMD_FINISH) must repaint the desktop with the AES DESKTOP \
         root's ob_spec 0x1143 (pattern 4, colour 3), as EmuTOS w_drawdesk does \
         (gemwmlib.c:340-359, 1269-1271); {wrong} px differ, first {:?} \
         (x, y, want_pen, got_pen)",
        first
    );

    // …and the menu row is left alone: `do_walk` clips to `gl_rfull` first.
    let mut touched = 0usize;
    for y in 0..GL_HBOX {
        for x in 0..SCREEN_W as i16 {
            if host.raster.get_device_pixel(x, y) != UNPAINTED {
                touched += 1;
            }
        }
    }
    assert_eq!(
        touched, 0,
        "w_drawdesk must not touch the menu row: do_walk intersects its clip with \
         gl_rfull, which starts at gl_hbox = {GL_HBOX} \
         (aes/gemwmlib.c:294-298, aes/gemgraf.c:412,423-424); {touched} px changed"
    );
}

// ── 2. the file selector undraws itself ───────────────────────────────────

/// QED opens a file through cflib's `select_file`, which calls
/// `fsel_exinput`/`fsel_input` (`sdk/cflib/cflib-21/filesel.c:255-257`). EmuTOS'
/// `fs_input` brackets the dialog with `fm_dial(FMD_START, &gl_rcenter, &gl_rfs)`
/// before the first `ob_draw` and `fm_dial(FMD_FINISH, &gl_rcenter, &gl_rfs)`
/// after the loop (`aes/gemfslib.c:707-708,918-919`). Without the FMD_FINISH
/// nothing repaints the desktop under the dialog, and the selector stays on the
/// screen even though the call returned the picked file.
///
/// The dialog is ended the way EmuTOS' `fm_do` ends it on RETURN: one queued key
/// on the first pass of the modal loop, so what is measured is purely
/// draw-then-undraw.
#[test]
fn the_file_selector_undraws_itself() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let host = rig();
    let mut pb = Aespb::new();
    pb.call(10, 0, 1, 0); // appl_init

    pb.intin.fill(0);
    pb.intin[0] = 0;
    pb.intin[1] = WF_NEWDESK;
    pb.intin[2] = 0;
    pb.addrin[0] = 0;
    pb.call(105, 6, 1, 1); // wind_set(0, WF_NEWDESK, NULL, 0)

    // Start from a screen the AES itself painted, so "restored" is exactly
    // "identical to before".
    paint(&host.raster, FULL, UNPAINTED);
    form_dial(&mut pb, FMD_FINISH, FULL, FULL);
    let before = host.raster.snapshot();

    let mut path = [0u8; 128];
    path[..6].copy_from_slice(b"C:\\*.*");
    let mut sel = [0u8; 32];
    host.push_key(b'\r' as u16);

    pb.intin.fill(0);
    pb.addrin[0] = path.as_mut_ptr() as u64;
    pb.addrin[1] = sel.as_mut_ptr() as u64;
    pb.call(90, 0, 2, 2); // fsel_input
    pb.addrin[0] = 0;
    pb.addrin[1] = 0;
    assert_eq!(pb.intout[0], 1, "fsel_input must report that it ran");
    assert_eq!(pb.intout[1], 1, "RETURN in the selector is the OK button");

    let after = host.raster.snapshot();
    let mut wrong = 0usize;
    let mut first = None;
    for y in 0..SCREEN_H as i16 {
        for x in 0..SCREEN_W as i16 {
            let k = BufBackend::idx(x, y);
            if after[k] != before[k] {
                if first.is_none() {
                    first = Some((x, y, before[k], after[k]));
                }
                wrong += 1;
            }
        }
    }
    assert_eq!(
        wrong,
        0,
        "fsel_input must undraw itself: EmuTOS fs_input ends with \
         fm_dial(FMD_FINISH, &gl_rcenter, &gl_rfs) (gemfslib.c:918-919), which \
         repaints the desktop under the dialog; {wrong} px still differ, first \
         {:?} (x, y, before, after)",
        first
    );
}

const WF_NEWDESK: i16 = 14;

/// EmuTOS `gl_hbox` for the 8x16 system font: `gl_hchar + 3`
/// (`aes/gemgraf.c:412`), hence the menu-bar height and the top edge of
/// `gl_rfull` (`:423-424,427`).
const GL_HBOX: i16 = 19;
