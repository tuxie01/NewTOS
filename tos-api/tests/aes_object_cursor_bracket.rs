//! A65 — `objc_draw` / `objc_change` must hide the mouse cursor while they draw.
//!
//! The cursor is a **save-under sprite**: `mouse::draw` stores the 16×16 device
//! pixels it is about to cover and paints the arrow over them, `mouse::erase`
//! writes those stored pixels straight back (`tos-api/src/vdi/mouse.rs`; EmuTOS
//! `cur_display`/`cur_replace`, `vdi/vdi_mouse.c:975-985,1099`). The saved block
//! is only valid as long as nothing else draws into it. Draw into it between the
//! save and the restore, and the **next** cursor step writes the *pre-draw*
//! pixels back — a stale rectangle that stays on screen until something else
//! repaints it.
//!
//! EmuTOS closes that window by bracketing every object draw with
//! `gsx_moff()` / `gsx_mon()` — `ob_draw` at `aes/gemoblib.c:974` … `:976`,
//! `ob_change` at `:1275` … `:1319`. The bracket takes the sprite off the screen
//! first (releasing the save-under) and puts it back afterwards with a **fresh**
//! save-under taken from the newly drawn pixels.
//!
//! # What is measured
//!
//! Two runs of one identical sequence — paint a row, park the cursor on it,
//! change the row through the real AES trap, step the cursor, take the cursor
//! away — differing in nothing but who supplies the bracket: the test, or the
//! AES itself. If the AES brackets, the two screens are identical.
//!
//! The third case pins the **signature** rather than mere inequality. The stale
//! block is the save-under box, so it is
//!
//! * exactly **16 columns** wide, starting at the sprite's origin, and
//! * exactly **`16 − k`** rows tall, `k` = the cursor's distance from the row's
//!   upper edge — the part of the 16-pixel box that still falls inside a
//!   16-pixel row,
//!
//! and every pixel in it carries the **pre-change** colour. That is the shape
//! the defect photograph was read against (`TASK/A65.md`): sixteen bright
//! columns inside an inverted selection bar, one to two pixels high when the row
//! was clicked at its lower edge.
//!
//! # The profile
//!
//! 640×480 Falcon truecolor (16 planes, 256 pens) — the workstation shape the
//! defect was photographed on, and the one `init_wk` seeds a pseudo-palette for
//! (`vdi/workstation.rs`, `planes > 8`). The four-plane TT harness in
//! `aes_reference/` cannot serve here: its pseudo-palette is never filled, so
//! WHITE and BLACK resolve to the same device word and an inversion is
//! invisible. Harness shape (backend, host, parameter block) is
//! `aes_dialog_undraw.rs`'s.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use tos_api::aes::object::Object;
use tos_api::bdos::process::Abi;
use tos_api::host::Host;
use tos_api::vdi::backend::{ClipRect, MonoSrc, PatternFill, Pel, RasterBackend, Rect, WriteMode};
use tos_api::vdi::mouse;
use tos_api::vdi::{cursor_hide, cursor_show};

const SCREEN_W: u16 = 640;
const SCREEN_H: u16 = 480;

// ── a raster backend over a real pixel buffer ─────────────────────────────

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

struct TestHost {
    raster: BufBackend,
    clock: AtomicU64,
}

impl Host for TestHost {
    fn console_write_byte(&self, _byte: u8) {}
    fn raster(&self) -> Option<&dyn RasterBackend> {
        Some(&self.raster)
    }
    fn monotonic_millis(&self) -> u64 {
        self.clock.fetch_add(1, Ordering::Relaxed)
    }
    fn pump_input(&self) {
        self.clock.fetch_add(1, Ordering::Relaxed);
    }
}

// SAFETY: every field is behind a Mutex or an atomic.
unsafe impl Sync for TestHost {}
unsafe impl Send for TestHost {}

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
        // SAFETY: `block` holds six live pointers to arrays owned by `self`.
        unsafe { tos_api::aes::dispatch(self.block.as_ptr() as u64, ABI) };
    }
}

// ── the rig ───────────────────────────────────────────────────────────────

/// The AES keeps process-global state (window table, process descriptors, the
/// cursor), so the cases in this binary must not overlap.
static SERIAL: Mutex<()> = Mutex::new(());
static RIG: OnceLock<&'static TestHost> = OnceLock::new();

/// The one host of this binary (`install_host` accepts a single install), with
/// the pseudo-palette the VDI dispatcher would have left behind
/// (`vdi/mod.rs`: `backend.set_active_palette(&ext.palette)` before every
/// drawing op; `init_wk` seeds `ext.palette` from the device's requested
/// colours under truecolor). Without it every pen resolves to the same device
/// word and an inverted row is indistinguishable from a normal one.
fn rig() -> &'static TestHost {
    RIG.get_or_init(|| {
        let host: &'static TestHost =
            Box::leak(Box::new(TestHost { raster: BufBackend::new(), clock: AtomicU64::new(0) }));
        let href: &'static &'static dyn Host = Box::leak(Box::new(host as &'static dyn Host));
        tos_api::host::install_host(href);
        seed_palette(&host.raster);
        host
    })
}

/// EmuTOS' `videl_palette1` (`vdi/vdi_col.c:130-136`) as the RGB555-in-565
/// words the pseudo-palette holds — pen 0 white (background), pen 1 black
/// (foreground), the sixteen the kernel backend seeds too.
fn seed_palette(b: &BufBackend) {
    #[rustfmt::skip]
    const VIDEL1: [(u16, u16, u16); 16] = [
        (1000, 1000, 1000), (0, 0, 0),       (1000, 0, 0),    (0, 1000, 0),
        (0, 0, 1000),       (0, 1000, 1000), (1000, 1000, 0), (1000, 0, 1000),
        (733, 733, 733),    (533, 533, 533), (667, 0, 0),     (0, 667, 0),
        (0, 0, 667),        (0, 667, 667),   (667, 667, 0),   (667, 0, 667),
    ];
    let mut pal = [0u16; 256];
    for (i, (r, g, bl)) in VIDEL1.iter().enumerate() {
        let c5 = |v: u16| ((v as u32 * 31 + 500) / 1000) as u16;
        pal[i] = (c5(*r) << 11) | (c5(*g) << 6) | c5(*bl);
    }
    *b.palette.lock().unwrap() = pal;
}

// ── the object under test ────────────────────────────────────────────────

/// `G_BOX` (`obdefs.h`). Its `ob_spec` is a colour word, not a pointer, so the
/// tree needs no backing storage and the fill is fully determined by the spec.
const G_BOX: u16 = 20;
/// `SELECTED` (`obdefs.h`).
const SELECTED: i16 = 0x0001;
/// `NIL` (`obdefs.h`).
const NIL: i16 = -1;
/// GEM pen numbers (`obdefs.h` `WHITE`/`BLACK`).
const PEN_BLACK: Pel = 1;

/// `ob_spec` = 0: border WHITE, thickness byte 0, inside pattern `IP_HOLLOW`,
/// inside colour WHITE. `just_draw` fills a hollow box **opaque white**
/// (`gr_rect`, EmuTOS `gemgraf.c:547-559`), and SELECTED turns the pattern
/// solid **black** (`just_draw`'s `changecol` arm, EmuTOS
/// `gemoblib.c:1290-1304`'s counterpart on the redraw side) — so every pixel of
/// the rectangle flips, which is what makes the damage countable to the pixel.
const SPEC_HOLLOW_WHITE: u64 = 0;

/// The row: one line of the 8×16 system font, as wide as a file-selector name
/// column. Height 16 is what makes `16 − k` falsifiable — a taller row would
/// clip nothing off the save-under box and always give 16.
const ROW_X: i16 = 100;
const ROW_Y: i16 = 100;
const ROW_W: i16 = 200;
const ROW_H: i16 = 16;

/// The arrow's hot spot is (0, 0) (`mouse::PRESET_FORMS[0]`), so the sprite's
/// 16×16 save-under box starts at the cursor position itself, `k` pixels below
/// the row's upper edge.
const K: i16 = 5;
const CUR_X: i16 = 120;
const CUR_Y: i16 = ROW_Y + K;

/// The save-under box is 16 device pixels square (`mouse::draw`).
const BOX: i16 = 16;

/// A device value no palette pen can produce, so "nothing drew here" is visible
/// as itself.
const SLATE: u32 = 0x0012_3456;

/// The predicted stale rectangle: the save-under box clipped to the row.
const DMG_X0: i16 = CUR_X;
const DMG_X1: i16 = CUR_X + BOX - 1;
const DMG_Y0: i16 = CUR_Y;
const DMG_Y1: i16 = ROW_Y + ROW_H - 1;

fn tree(state: i16) -> [Object; 1] {
    [Object {
        ob_next: NIL,
        ob_head: NIL,
        ob_tail: NIL,
        ob_type: G_BOX,
        ob_flags: 0,
        ob_state: state as u16,
        ob_spec: SPEC_HOLLOW_WHITE,
        ob_x: ROW_X,
        ob_y: ROW_Y,
        ob_width: ROW_W,
        ob_height: ROW_H,
    }]
}

// ── screen helpers ───────────────────────────────────────────────────────

/// Paint the whole screen a raw device value, bypassing every AES/VDI path.
fn slate(b: &BufBackend) {
    b.px.lock().unwrap().fill(SLATE);
}

fn at(px: &[u32], x: i16, y: i16) -> u32 {
    px[y as usize * SCREEN_W as usize + x as usize]
}

/// Every position at which two screens disagree.
fn differences(a: &[u32], b: &[u32]) -> Vec<(i16, i16)> {
    let mut d = Vec::new();
    for y in 0..SCREEN_H as i16 {
        for x in 0..SCREEN_W as i16 {
            if at(a, x, y) != at(b, x, y) {
                d.push((x, y));
            }
        }
    }
    d
}

/// `(xmin, ymin, xmax, ymax)` of a difference set.
fn bounds(d: &[(i16, i16)]) -> (i16, i16, i16, i16) {
    let (mut x0, mut y0, mut x1, mut y1) = (i16::MAX, i16::MAX, i16::MIN, i16::MIN);
    for &(x, y) in d {
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    }
    (x0, y0, x1, y1)
}

/// One screen row rendered as characters, for the failure message.
fn band(px: &[u32], white: u32, black: u32, y: i16, x0: i16, x1: i16) -> String {
    (x0..=x1)
        .map(|x| match at(px, x, y) {
            SLATE => '.',
            v if v == black => '#',
            v if v == white => ' ',
            _ => 'o',
        })
        .collect()
}

// ── the AES calls ────────────────────────────────────────────────────────

/// `objc_draw` (42): `int_in[0]` object, `[1]` depth, `[2..5]` clip
/// (0,0,0,0 = no clip = full screen, `gsx_sclip`'s zero convention).
fn objc_draw(pb: &mut Aespb, t: &[Object; 1]) {
    pb.intin.fill(0);
    pb.addrin.fill(0);
    pb.addrin[0] = t.as_ptr() as u64;
    pb.call(42, 6, 1, 1);
    assert_eq!(pb.intout[0], 1, "objc_draw refused the tree");
}

/// `objc_change` (47): `int_in[0]` object, `[2..5]` clip, `[6]` new state,
/// `[7]` redraw flag (`aesdefs.h:376-377`).
fn objc_change(pb: &mut Aespb, t: &[Object; 1], newstate: i16) {
    pb.intin.fill(0);
    pb.intin[6] = newstate;
    pb.intin[7] = 1; // redraw
    pb.addrin.fill(0);
    pb.addrin[0] = t.as_ptr() as u64;
    pb.call(47, 8, 1, 1);
    assert_eq!(pb.intout[0], 1, "objc_change refused the tree");
}

// ── the sequence ─────────────────────────────────────────────────────────

/// What the AES is asked to do over the visible cursor.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Op {
    /// Flip the row to SELECTED with `objc_change(…, redraw = 1)` (47).
    Change,
    /// Set the state directly and repaint with `objc_draw` (42).
    Draw,
}

/// Run the sequence once and hand back the screen.
///
/// `outer_bracket` supplies the `gsx_moff`/`gsx_mon` pair *from outside* — what
/// the AES is supposed to do for itself. The two runs differ in nothing else.
fn run(op: Op, outer_bracket: bool) -> Vec<u32> {
    let h = rig();
    let b = &h.raster;
    let mut pb = Aespb::new();

    // A known cursor state. `show_reset` runs `ensure_init` (which installs the
    // ARROW form and, on its first run, re-parks the cursor — it must not do
    // that after we have placed it); `remove` then takes the sprite off the
    // screen and leaves the hide depth at exactly 1.
    mouse::show_reset(b);
    mouse::remove(b);
    slate(b);

    // 1. the row, unselected: an opaque white bar on the slate.
    let mut t = tree(0);
    objc_draw(&mut pb, &t);

    // 2. the cursor onto the row, then visible — this `draw` takes the
    //    save-under copy of the *white* row.
    mouse::set_locator_pos(CUR_X, CUR_Y);
    cursor_show(b);

    // 3. the operation under test, drawing straight through the save-under box.
    if outer_bracket {
        cursor_hide(b);
    }
    match op {
        Op::Change => objc_change(&mut pb, &t, SELECTED),
        Op::Draw => {
            t[0].ob_state = SELECTED as u16;
            objc_draw(&mut pb, &t);
        }
    }
    if outer_bracket {
        cursor_show(b);
    }

    // 4. one cursor step: `erase` writes the save-under back, `draw` re-takes it
    //    two pixels on — the packet through `feed_relative` (the IKBD
    //    decoder's entry), drawn in the next frame by `vb_draw` (TASK/V55).
    tos_api::vdi::feed_relative(2, 0, 0, b);
    tos_api::vdi::vb_draw(b);

    // 5. sprite away again, so what is left on the screen is only the damage.
    cursor_hide(b);

    b.snapshot()
}

fn check(op: Op) {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let b = &rig().raster;
    let white = b.rgb565_to_device(b.pel_to_device16(0));
    let black = b.rgb565_to_device(b.pel_to_device16(PEN_BLACK));
    assert_ne!(white, black, "the palette cannot tell an inverted row apart");

    let bracketed = run(op, true);
    let bare = run(op, false);

    // Precondition: the row really was drawn, and drawn black once SELECTED.
    assert_eq!(
        at(&bracketed, ROW_X + 1, ROW_Y + 1),
        black,
        "{op:?}: the SELECTED row is not the inverted fill"
    );

    let d = differences(&bracketed, &bare);
    if d.is_empty() {
        return; // the AES brackets — nothing stale survives the cursor step
    }

    let (x0, y0, x1, y1) = bounds(&d);
    let predicted = ((DMG_X1 - DMG_X0 + 1) as usize) * ((DMG_Y1 - DMG_Y0 + 1) as usize);
    let mut msg = format!(
        "{op:?}: the AES drew through the cursor's save-under box.\n\
         {} pixels differ; damage ({x0},{y0})..({x1},{y1}) = {}x{}; \
         predicted ({DMG_X0},{DMG_Y0})..({DMG_X1},{DMG_Y1}) = {}x{} = {predicted} px \
         (16 columns by 16-k = 16-{K} rows)\n\
         A = bracketed, B = as the AES left it; '#' black, ' ' white, '.' untouched\n",
        d.len(),
        x1 - x0 + 1,
        y1 - y0 + 1,
        DMG_X1 - DMG_X0 + 1,
        DMG_Y1 - DMG_Y0 + 1,
    );
    for y in (DMG_Y0 - 1)..=(DMG_Y1 + 1) {
        msg.push_str(&format!(
            "y={y:3}  A |{}|  B |{}|\n",
            band(&bracketed, white, black, y, DMG_X0 - 2, DMG_X1 + 2),
            band(&bare, white, black, y, DMG_X0 - 2, DMG_X1 + 2),
        ));
    }
    panic!("{msg}");
}

#[test]
fn objc_change_does_not_leave_the_cursor_save_under_stale() {
    check(Op::Change);
}

#[test]
fn objc_draw_does_not_leave_the_cursor_save_under_stale() {
    check(Op::Draw);
}

/// The damage has a shape, and the shape is the save-under box. This case
/// measures the **sprite**, not the AES: the row is repainted behind the
/// cursor's back with a raw backend write, so no bracket anywhere in the AES
/// can suppress it, and the geometry the other two cases predict stays pinned
/// after the fix.
#[test]
fn the_save_under_box_is_sixteen_wide_and_sixteen_minus_k_tall() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let h = rig();
    let b = &h.raster;
    let mut pb = Aespb::new();

    mouse::show_reset(b);
    mouse::remove(b);
    slate(b);

    let t = tree(0);
    objc_draw(&mut pb, &t);
    let unselected = b.snapshot();

    mouse::set_locator_pos(CUR_X, CUR_Y);
    cursor_show(b);

    // Invert the row with the very write `just_draw`'s solid fill ends up
    // making, but straight at the backend — the AES never sees it.
    let black = b.rgb565_to_device(b.pel_to_device16(PEN_BLACK));
    for y in ROW_Y..ROW_Y + ROW_H {
        for x in ROW_X..ROW_X + ROW_W {
            b.put_device_pixel(x, y, black);
        }
    }

    // What the screen owes the viewer once the cursor is gone again: the
    // inverted row, the slate around it, no sprite anywhere. Built from the
    // pre-cursor snapshot so it cannot inherit anything the cursor did.
    let mut ideal = unselected.clone();
    for y in ROW_Y..ROW_Y + ROW_H {
        for x in ROW_X..ROW_X + ROW_W {
            ideal[y as usize * SCREEN_W as usize + x as usize] = black;
        }
    }

    tos_api::vdi::feed_relative(2, 0, 0, b);
    cursor_hide(b);
    let damaged = b.snapshot();

    let d = differences(&ideal, &damaged);
    assert!(!d.is_empty(), "the cursor step wrote nothing back at all");
    assert_eq!(
        bounds(&d),
        (DMG_X0, DMG_Y0, DMG_X1, DMG_Y1),
        "save-under damage: expected 16 columns from the sprite origin and \
         16-k = 16-{K} rows down to the row's lower edge"
    );
    assert_eq!(
        d.len(),
        (BOX as usize) * ((BOX - K) as usize),
        "every pixel of the clipped save-under box must carry the stale colour"
    );
    // And what comes back is the pre-change content, pixel for pixel.
    for &(x, y) in &d {
        assert_eq!(
            at(&damaged, x, y),
            at(&unselected, x, y),
            "({x},{y}) must hold the colour the save-under captured"
        );
    }
}
