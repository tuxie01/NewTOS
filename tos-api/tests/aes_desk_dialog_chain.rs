//! **DESK08** — the dialog chain every desktop dialog now runs through, pinned
//! at the AES trap boundary: `form_center` (54) → `form_dial(FMD_START)` (51) →
//! `objc_draw` (42) → `form_dial(FMD_FINISH)` (51).
//!
//! EmuTOS' desktop wraps every one of its dialogs in `start_dialog` /
//! `end_dialog` (`desk/deskinf.c:349-390`), and `programs/desktop/src/` did not:
//! `dlg.rs`, `inf.rs` and `insapp.rs` each computed a centre of their own from
//! the screen size, drew with a full-screen clip, and had the caller repaint the
//! **whole** desktop afterwards (gap G7,
//! `docs/analysis/desk-aes-gaps-2026-09-14.md`). The two properties that fixes
//! are exactly the two asserted here, against a real pixel buffer driven through
//! the real AES dispatcher:
//!
//! 1. **Placement.** `form_center` puts the tree's ROOT where EmuTOS'
//!    `ob_center` puts it (`aes/gemobed.c:67-106`): horizontally centred, and
//!    vertically centred **below the menu bar** —
//!    `yd = gl_hbox + (gl_height - gl_hbox - hd) / 2`. The desktop's own
//!    `(screen_h - dialog_h) / 2` is a different number, and it is higher up the
//!    screen by half the menu row.
//!
//! 2. **Undraw.** `form_dial(FMD_FINISH)` restores **only** its rectangle, and
//!    restores it exactly: the screen outside the dialog is untouched
//!    throughout, and after the FINISH the whole screen is byte-identical to
//!    what it was before the dialog went up. EmuTOS' `fm_dial` does not
//!    photograph the screen at FMD_START (`aes/gemfmlib.c:365-369` is a bare
//!    `break`) — the restore is `w_drawdesk(pt)` + `w_update(DESKWH, pt, …)`
//!    at FINISH (`:375-379`), which is why "restore" here means "repaint",
//!    and why it is worth checking that the repaint really reproduces the
//!    previous pixels.
//!
//! The tree used is the desktop's own New-folder box
//! (`programs/desktop/src/dlg.rs::edit_name`): a 290x150 `G_BOX` at the **origin**
//! with a title `G_STRING`, an editable `G_BOXTEXT` and two `G_BUTTON`s — built
//! with no screen position of its own, exactly as a resource-file dialog is, so
//! that `form_center` is what places it.
//!
//! Harness (backend + host + parameter block + palette) is the one from
//! `aes_dialog_undraw.rs`.
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

// ── the desktop's New-folder dialog tree, built at the origin ─────────────

use tos_api::aes::object::{Object, TedInfo};

const G_BOX: u16 = 20;
const G_BOXTEXT: u16 = 22;
const G_BUTTON: u16 = 26;
const G_STRING: u16 = 28;

const SELECTABLE: u16 = 0x0001;
const DEFAULT: u16 = 0x0002;
const EXIT: u16 = 0x0004;
const EDITABLE: u16 = 0x0008;
const LASTOB: u16 = 0x0020;

const MAX_DEPTH: i16 = 8;
const ROOT: i16 = 0;

/// The dialog's nominal size — `programs/desktop/src/dlg.rs::edit_name`.
const DLG_W: i16 = 290;
const DLG_H: i16 = 150;

fn mk(ty: u16, fl: u16, spec: u64, x: i16, y: i16, w: i16, h: i16, next: i16) -> Object {
    Object {
        ob_next: next,
        ob_head: -1,
        ob_tail: -1,
        ob_type: ty,
        ob_flags: fl,
        ob_state: 0,
        ob_spec: spec,
        ob_x: x,
        ob_y: y,
        ob_width: w,
        ob_height: h,
    }
}

/// The desktop's New-folder tree, ROOT at (0,0) — `form_center` places it.
fn new_folder_tree(ted: &TedInfo) -> Vec<Object> {
    static EMPTY: [u8; 1] = [0];
    static PROMPT: &[u8] = b"Name of new folder:\0";
    static OK: &[u8] = b" OK \0";
    static CANCEL: &[u8] = b" Cancel \0";
    let _ = &EMPTY;
    let box_spec: u64 = (1 << 16) | 0x1070;
    let mut t = vec![
        mk(G_BOX, 0, box_spec, 0, 0, DLG_W, DLG_H, -1),
        mk(G_STRING, 0, PROMPT.as_ptr() as u64, 24, 18, DLG_W - 48, 16, 2),
        mk(G_BOXTEXT, EDITABLE, ted as *const TedInfo as u64, 24, 46, 200, 24, 3),
        mk(G_BUTTON, SELECTABLE | EXIT | DEFAULT, OK.as_ptr() as u64, 28, 104, 90, 28, 4),
        mk(G_BUTTON, SELECTABLE | EXIT | LASTOB, CANCEL.as_ptr() as u64, 172, 104, 90, 28, 0),
    ];
    t[0].ob_head = 1;
    t[0].ob_tail = 4;
    t
}

fn ted_for(buf: &mut [u8]) -> TedInfo {
    static EMPTY: [u8; 1] = [0];
    TedInfo {
        te_ptext: buf.as_mut_ptr() as u64,
        te_ptmplt: EMPTY.as_ptr() as u64,
        te_pvalid: EMPTY.as_ptr() as u64,
        te_font: 3,
        te_junk1: 0,
        te_just: 0,
        te_color: 1 << 8,
        te_junk2: 0,
        te_thickness: 1,
        te_txtlen: 13,
        te_tmplen: 0,
    }
}

/// `form_center(tree)` — opcode 54, `addr_in[0]` the tree, the outer rectangle
/// in `int_out[1..4]`.
fn form_center(pb: &mut Aespb, tree: *const Object) -> (i16, i16, i16, i16) {
    pb.intin.fill(0);
    pb.addrin[0] = tree as u64;
    pb.call(54, 0, 5, 1);
    (pb.intout[1], pb.intout[2], pb.intout[3], pb.intout[4])
}

/// `objc_draw(tree, ROOT, MAX_DEPTH, x, y, w, h)` — opcode 42.
fn objc_draw(pb: &mut Aespb, tree: *const Object, r: (i16, i16, i16, i16)) {
    pb.intin.fill(0);
    pb.addrin[0] = tree as u64;
    pb.intin[0] = ROOT;
    pb.intin[1] = MAX_DEPTH;
    pb.intin[2] = r.0;
    pb.intin[3] = r.1;
    pb.intin[4] = r.2;
    pb.intin[5] = r.3;
    pb.call(42, 6, 1, 1);
}

/// `wind_get(0, WF_WXYWH)` — opcode 104; its `y` is `gl_hbox`, the menu row
/// `ob_center` centres below.
fn desk_grect(pb: &mut Aespb) -> (i16, i16, i16, i16) {
    pb.intin.fill(0);
    pb.intin[0] = 0; // DESKWH
    pb.intin[1] = 4; // WF_WXYWH
    pb.call(104, 2, 5, 0);
    (pb.intout[1], pb.intout[2], pb.intout[3], pb.intout[4])
}

fn inside(r: (i16, i16, i16, i16), x: usize, y: usize) -> bool {
    let (x, y) = (x as i16, y as i16);
    x >= r.0 && x < r.0 + r.2 && y >= r.1 && y < r.1 + r.3
}

/// Number of pixels of `a` and `b` that differ, split into inside / outside `r`.
fn diff_split(a: &[u32], b: &[u32], r: (i16, i16, i16, i16)) -> (usize, usize) {
    let (mut ins, mut outs) = (0, 0);
    for (i, (p, q)) in a.iter().zip(b.iter()).enumerate() {
        if p == q {
            continue;
        }
        let (x, y) = (i % SCREEN_W as usize, i / SCREEN_W as usize);
        if inside(r, x, y) {
            ins += 1;
        } else {
            outs += 1;
        }
    }
    (ins, outs)
}

#[test]
fn form_center_places_the_desktop_dialog_below_the_menu_bar() {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let host = rig();
    let mut pb = Aespb::new();

    let mut buf = [0u8; 16];
    let ted = ted_for(&mut buf);
    let tree = new_folder_tree(&ted);

    let t = form_center(&mut pb, tree.as_ptr());

    // The ROOT was moved by the AES, not by the caller: the tree went in at
    // (0,0) (`ob_center` writes `xd`/`yd` into the ROOT, `gemobed.c:81-82`).
    assert_eq!(
        (tree[0].ob_x, tree[0].ob_y),
        (t.0, t.1),
        "form_center must write the centred position into the ROOT"
    );
    // No OUTLINED / SHADOWED state on this box, so the reported outer rectangle
    // is the ROOT rectangle itself.
    assert_eq!((t.2, t.3), (DLG_W, DLG_H), "outer rect of a plain box is the box");

    // EmuTOS `ob_center`: horizontally centred, vertically centred **below the
    // menu bar**.
    let bar = desk_grect(&mut pb).1;
    assert!(bar > 0, "the desktop work area must start below a menu row");
    let want_x = (SCREEN_W as i16 - DLG_W) / 2;
    let want_y = bar + (SCREEN_H as i16 - bar - DLG_H) / 2;
    assert_eq!((t.0, t.1), (want_x, want_y));

    // The formula `programs/desktop/src/` used before DESK08 — `(screen_h - h) / 2`,
    // the menu row ignored — is a *different* position. Pinned so the two are
    // not silently conflated again.
    let desk_own_y = (SCREEN_H as i16 - DLG_H) / 2;
    assert_ne!(t.1, desk_own_y, "the menu-bar offset is what makes the two differ");
    assert_eq!(t.1 - desk_own_y, bar / 2, "and it is exactly half the menu row");

    let _ = host;
}

#[test]
fn fmd_finish_restores_only_the_dialog_rectangle() {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let host = rig();
    let mut pb = Aespb::new();
    pb.call(10, 0, 1, 0); // appl_init

    // No application desktop tree registered: `w_drawdesk` then draws the AES's
    // own built-in DESKTOP root (`aes/gemwmlib.c:340-359`).
    pb.intin.fill(0);
    pb.intin[0] = 0; // DESKWH
    pb.intin[1] = 14; // WF_NEWDESK
    pb.intin[2] = 0;
    pb.addrin[0] = 0;
    pb.call(105, 6, 1, 1);

    // A screen that is not the desktop, so "the AES painted here" is visible.
    paint(&host.raster, FULL, UNPAINTED);
    // Put the AES desktop up over the whole screen — the state a desktop dialog
    // is opened on top of. The top `gl_hbox` rows stay as they are: `w_drawdesk`
    // clips to `gl_rfull` first (`aes/gemwmlib.c:294-298`), which is why the
    // check below starts at the desktop work area.
    form_dial(&mut pb, FMD_FINISH, (0, 0, 0, 0), FULL);
    let bar = desk_grect(&mut pb).1;
    let before = host.raster.snapshot();
    assert!(
        before[(bar as usize) * SCREEN_W as usize..].iter().all(|&p| p != UNPAINTED),
        "the desktop work area must be the AES desktop before the dialog"
    );

    let mut buf = [0u8; 16];
    let ted = ted_for(&mut buf);
    let tree = new_folder_tree(&ted);

    // start_dialog: centre, reserve, draw (`deskinf.c:349-357`).
    let t = form_center(&mut pb, tree.as_ptr());
    form_dial(&mut pb, FMD_START, (0, 0, 0, 0), t);
    objc_draw(&mut pb, tree.as_ptr(), t);

    let during = host.raster.snapshot();
    let (ins, outs) = diff_split(&before, &during, t);
    assert!(ins > 0, "the dialog must have drawn something inside its rectangle");
    assert_eq!(outs, 0, "nothing outside the dialog rectangle may change: {outs} px did");

    // end_dialog: release + repaint (`deskinf.c:363-369`).
    form_dial(&mut pb, FMD_FINISH, (0, 0, 0, 0), t);

    let after = host.raster.snapshot();
    let (ins, outs) = diff_split(&before, &after, t);
    assert_eq!(outs, 0, "FMD_FINISH must not touch anything outside its rectangle");
    assert_eq!(ins, 0, "FMD_FINISH must restore its rectangle exactly: {ins} px left over");
    assert!(after == before, "the screen must be byte-identical to before the dialog");
}

/// Count the pixels **outside** `r` that are no longer [`UNPAINTED`], i.e. that
/// the AES drew on.
fn touched_outside(px: &[u32], r: (i16, i16, i16, i16)) -> usize {
    px.iter()
        .enumerate()
        .filter(|(i, &p)| {
            p != UNPAINTED && !inside(r, i % SCREEN_W as usize, i / SCREEN_W as usize)
        })
        .count()
}

/// The scope of the restore, measured rather than assumed: with a sentinel
/// value on the screen, only the pixels the AES actually drew on stop being the
/// sentinel.
///
/// This is the half of G7 that pixels alone do not show on a plain background —
/// a whole-screen `form_dial(FMD_FINISH)` repaints the same pattern and so
/// leaves the same picture. What it does **not** leave the same is everything
/// the desktop composite pulls in behind it: `desk::redraw::redraw_desktop`
/// follows its full-screen FMD_FINISH with `win_shwall` (every window's content
/// re-laid) and `install_menu_bar`, and the AES's own `w_update` fans a
/// `WM_REDRAW` at every window on screen instead of the ones the dialog
/// covered. Bounding the drawn area to the dialog's rectangle is what removes
/// all of that.
#[test]
fn the_dialog_restore_draws_only_inside_its_own_rectangle() {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let host = rig();
    let mut pb = Aespb::new();
    pb.call(10, 0, 1, 0); // appl_init

    pb.intin.fill(0);
    pb.intin[0] = 0; // DESKWH
    pb.intin[1] = 14; // WF_NEWDESK
    pb.intin[2] = 0;
    pb.addrin[0] = 0;
    pb.call(105, 6, 1, 1);

    let mut buf = [0u8; 16];
    let ted = ted_for(&mut buf);
    let tree = new_folder_tree(&ted);

    // Sentinel screen: every pixel that is still UNPAINTED afterwards was never
    // drawn on.
    paint(&host.raster, FULL, UNPAINTED);

    let t = form_center(&mut pb, tree.as_ptr());
    form_dial(&mut pb, FMD_START, (0, 0, 0, 0), t);
    objc_draw(&mut pb, tree.as_ptr(), t);
    form_dial(&mut pb, FMD_FINISH, (0, 0, 0, 0), t);

    let px = host.raster.snapshot();
    let outside = touched_outside(&px, t);
    assert_eq!(
        outside, 0,
        "the whole dialog chain must draw inside its rectangle only; {outside} px \
         outside {t:?} were drawn on"
    );
    // And it did draw: the rectangle itself is no longer the sentinel.
    let repainted = (t.3 as usize * t.2 as usize) - px
        .iter()
        .enumerate()
        .filter(|(i, &p)| {
            p == UNPAINTED && inside(t, i % SCREEN_W as usize, i / SCREEN_W as usize)
        })
        .count();
    assert_eq!(
        repainted,
        t.2 as usize * t.3 as usize,
        "FMD_FINISH must repaint the whole dialog rectangle"
    );
}
