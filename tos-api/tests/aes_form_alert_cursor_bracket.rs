//! A66 — `bb_save` / `bb_restore` must take the mouse cursor off the screen
//! while they copy, so the pointer never enters the saved image.
//!
//! The cursor is a **save-under sprite** (`tos-api/src/vdi/mouse.rs`; EmuTOS
//! `cur_display`/`cur_replace`, `vdi/vdi_mouse.c:975-985,1099`). `bb_save` reads
//! the screen; if the sprite is drawn at that moment its pixels are read too,
//! and `bb_restore` writes them back as *background* — a **ghost arrow** at the
//! position the pointer had when the alert opened, which nothing repaints.
//!
//! EmuTOS closes that window **under** `bb_save`/`bb_restore`: both go through
//! `bb_set`, and `bb_set` brackets its `vro_cpyfm` with `gsx_moff()` /
//! `gsx_mon()` (`aes/gemgsxif.c:367,378`). That is why `fm_alert`
//! (`aes/gemfmalt.c:363,390`) carries no bracket of its own.
//!
//! # What is measured
//!
//! * **The buffer** (`bb_save` alone): a slate screen, the arrow shown at `P`,
//!   `bb_save` over a rectangle containing the sprite's 16×16 box. Every pixel
//!   of the saved image must be slate. Without the bracket the image carries
//!   the arrow **signature** — `mouse::draw` paints the ARROW form's data bits
//!   black and its remaining mask bits white, from the sprite origin, 16
//!   columns wide — and nothing else, pixel for pixel.
//! * **The screen** (`form_alert` end to end, through the AES trap): the arrow
//!   shown at `P` inside the alert, the alert opened and closed with Enter,
//!   the cursor taken away. The 16×16 box at `P` must be slate again, and the
//!   whole screen must equal the pre-alert screen. Without the bracket the box
//!   carries the same signature: the ghost arrow.
//!
//! The signature is checked, not mere inequality — sixteen columns from the
//! sprite origin, black where the ARROW data bits are, white where only the
//! mask bits are, slate everywhere else (`TASK/A65.md`'s test form).
//!
//! # The profile
//!
//! 640×480 Falcon truecolor (16 planes, 256 pens) with the pseudo-palette
//! `init_wk` seeds under truecolor, as in `aes_object_cursor_bracket.rs`. The
//! slate is a device value that **round-trips** the RGB565 wireformat
//! `bb_save` stores (`rgb565_to_device(SLATE565)`), so a lossless copy is
//! indistinguishable from the original and only the sprite can show up as a
//! difference; its green LSB is set, so no palette pen can produce it.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use tos_api::bdos::process::Abi;
use tos_api::host::Host;
use tos_api::vdi::backend::{ClipRect, MonoSrc, PatternFill, Pel, RasterBackend, Rect, WriteMode};
use tos_api::vdi::mouse;
use tos_api::vdi::{bb_restore, bb_save, bb_save_words, cursor_hide, cursor_show};

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
        let (ax, ay, mut bx, mut by) = if x1 < x0 { (x1, y1, x0, y0) } else { (x0, y0, x1, y1) };
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

/// A host whose console hands out queued key bytes — how the modal alert is
/// closed headlessly (`form.rs` `modal_loop`: Enter fires the DEFAULT button).
/// The first key read also snapshots the screen: at that moment the alert is
/// fully drawn and waiting, which is the only chance to see it from outside.
struct TestHost {
    raster: BufBackend,
    clock: AtomicU64,
    keys: Mutex<VecDeque<u8>>,
    while_open: Mutex<Option<Vec<u32>>>,
}

impl Host for TestHost {
    fn console_write_byte(&self, _byte: u8) {}
    fn console_try_read(&self) -> Option<u8> {
        let mut open = self.while_open.lock().unwrap();
        if open.is_none() {
            *open = Some(self.raster.snapshot());
        }
        self.keys.lock().unwrap().pop_front()
    }
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

/// The AES keeps process-global state (the cursor, the update lock, the menu
/// hold), so the cases in this binary must not overlap.
static SERIAL: Mutex<()> = Mutex::new(());
static RIG: OnceLock<&'static TestHost> = OnceLock::new();

fn rig() -> &'static TestHost {
    RIG.get_or_init(|| {
        let host: &'static TestHost = Box::leak(Box::new(TestHost {
            raster: BufBackend::new(),
            clock: AtomicU64::new(0),
            keys: Mutex::new(VecDeque::new()),
            while_open: Mutex::new(None),
        }));
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

// ── the geometry ─────────────────────────────────────────────────────────

/// The pointer: at the screen centre, which is inside every centred alert
/// (`do_alert` centres the box below the menu bar; a one-line alert is
/// 112×94 px at 640×480, so a 16×16 box at the centre sits well inside its
/// saved rectangle). The ARROW's hot spot is (0, 0) (`mouse::PRESET_FORMS[0]`),
/// so the sprite's save-under box starts at the pointer itself.
const CUR_X: i16 = 320;
const CUR_Y: i16 = 240;

/// The save-under box is 16 device pixels square (`mouse::draw`).
const BOX: i16 = 16;

/// The slate as a 16-bit RGB565 word: green LSB (bit 5) set, so it is no
/// RGB555-in-565 pseudo-palette entry and no pen resolves to it; expanded to
/// the device by the backend it round-trips `bb_save`'s wireformat exactly.
const SLATE565: u16 = 0x1234;

/// A rectangle around the pointer for the buffer case — wider than the
/// sprite on every side, so a partial capture cannot be mistaken for a clean
/// one.
const SAVE_X: i16 = CUR_X - 24;
const SAVE_Y: i16 = CUR_Y - 24;
const SAVE_W: i16 = 70;
const SAVE_H: i16 = 60;

/// Pen numbers (`obdefs.h` `WHITE`/`BLACK`) — `mouse::draw`'s mask/data pens.
const PEN_WHITE: Pel = 0;
const PEN_BLACK: Pel = 1;

// ── screen helpers ───────────────────────────────────────────────────────

fn slate_device(b: &BufBackend) -> u32 {
    b.rgb565_to_device(SLATE565)
}

/// Paint the whole screen the slate, bypassing every AES/VDI path.
fn slate(b: &BufBackend) {
    let v = slate_device(b);
    b.px.lock().unwrap().fill(v);
}

fn at(px: &[u32], x: i16, y: i16) -> u32 {
    px[y as usize * SCREEN_W as usize + x as usize]
}

/// The ARROW sprite as `mouse::draw` paints it over a uniform `ground`: data
/// bits black, remaining mask bits white, everything else the ground —
/// the **signature** a captured pointer leaves, from the sprite origin.
fn arrow_signature(b: &BufBackend, ground: u32) -> [[u32; BOX as usize]; BOX as usize] {
    let white = b.rgb565_to_device(b.pel_to_device16(PEN_WHITE));
    let black = b.rgb565_to_device(b.pel_to_device16(PEN_BLACK));
    let (hx, hy, mask, data) = &mouse::PRESET_FORMS[0];
    assert_eq!((*hx, *hy), (0, 0), "the ARROW hot spot is expected at the sprite origin");
    let mut sig = [[ground; BOX as usize]; BOX as usize];
    for (dy, row) in sig.iter_mut().enumerate() {
        for (dx, px) in row.iter_mut().enumerate() {
            let bit = 0x8000u16 >> dx;
            if data[dy] & bit != 0 {
                *px = black;
            } else if mask[dy] & bit != 0 {
                *px = white;
            }
        }
    }
    sig
}

/// One row of a 16×16 box as characters: `#` black, ` ` white, `.` the
/// ground, `o` anything else.
fn band(row: &[u32], white: u32, black: u32, ground: u32) -> String {
    row.iter()
        .map(|&v| {
            if v == ground {
                '.'
            } else if v == black {
                '#'
            } else if v == white {
                ' '
            } else {
                'o'
            }
        })
        .collect()
}

/// Compare a 16×16 box against the ground, and if it differs, say whether the
/// difference **is** the arrow signature (the ghost) — pixel for pixel.
fn check_box(b: &BufBackend, what: &str, got: &[[u32; BOX as usize]; BOX as usize], ground: u32) {
    let white = b.rgb565_to_device(b.pel_to_device16(PEN_WHITE));
    let black = b.rgb565_to_device(b.pel_to_device16(PEN_BLACK));
    let sig = arrow_signature(b, ground);
    let differing = got.iter().flatten().filter(|&&v| v != ground).count();
    if differing == 0 {
        return;
    }
    let sig_pixels = sig.iter().flatten().filter(|&&v| v != ground).count();
    let is_ghost = *got == sig;
    let mut msg = format!(
        "{what}: {differing} of {} pixels in the 16x16 box at the sprite origin ({CUR_X},{CUR_Y}) \
         are not the ground; the ARROW signature has {sig_pixels} \
         ({}).\n        got                expected\n",
        BOX * BOX,
        if is_ghost {
            "the box IS the ghost arrow, pixel for pixel"
        } else {
            "the box is NOT the exact ghost arrow"
        },
    );
    for dy in 0..BOX as usize {
        msg.push_str(&format!(
            "y+{dy:2}  |{}|  |{}|\n",
            band(&got[dy], white, black, ground),
            band(&sig[dy], white, black, ground),
        ));
    }
    panic!("{msg}");
}

/// The 16×16 box at the sprite origin, read from a screen snapshot.
fn box_from_screen(px: &[u32]) -> [[u32; BOX as usize]; BOX as usize] {
    let mut out = [[0u32; BOX as usize]; BOX as usize];
    for dy in 0..BOX {
        for dx in 0..BOX {
            out[dy as usize][dx as usize] = at(px, CUR_X + dx, CUR_Y + dy);
        }
    }
    out
}

/// A known cursor state: form installed, sprite off the screen, hide depth 1.
fn reset_cursor(b: &BufBackend) {
    mouse::show_reset(b);
    mouse::remove(b);
}

// ── the buffer: bb_save alone ────────────────────────────────────────────

/// The pointer must not be in the image `bb_save` stores. EmuTOS `bb_set`
/// hides it around the copy (`gemgsxif.c:367`); without that the sprite is
/// read like any other screen content.
#[test]
fn bb_save_does_not_capture_the_cursor() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let h = rig();
    let b = &h.raster;
    let ground = slate_device(b);
    assert_eq!(b.device_to_rgb565(ground), SLATE565, "the slate must round-trip RGB565");

    reset_cursor(b);
    slate(b);
    mouse::set_locator_pos(CUR_X, CUR_Y);
    cursor_show(b);

    let words = bb_save_words(SAVE_W, SAVE_H);
    let mut buf = vec![0u32; words];
    // SAFETY: `buf` holds `bb_save_words(SAVE_W, SAVE_H)` u32 words.
    unsafe { bb_save(b, buf.as_mut_ptr(), SAVE_X, SAVE_Y, SAVE_W, SAVE_H) };

    // The bracket must have put the sprite back: hide depth 0, sprite drawn.
    let shown = b.snapshot();
    let black = b.rgb565_to_device(b.pel_to_device16(PEN_BLACK));
    assert_eq!(at(&shown, CUR_X + 1, CUR_Y + 1), black, "the sprite must be back on screen after bb_save");

    // Decode the saved image: 16-bit pixels, rows padded to 16 px
    // (`docs/api/vdi/raster/_raster.md`, memory-MFDB stride `fd_wdwidth × 16`).
    let stride = (SAVE_W as usize).div_ceil(16) * 16;
    let pixel = |col: usize, row: usize| -> u32 {
        let idx = row * stride + col;
        // SAFETY: `idx < stride * SAVE_H <= words * 2`.
        let w = unsafe { (buf.as_ptr() as *const u16).add(idx).read_unaligned() };
        b.rgb565_to_device(w)
    };
    // Outside the sprite box every saved pixel is the slate — the copy itself
    // is exact.
    for row in 0..SAVE_H as usize {
        for col in 0..SAVE_W as usize {
            let (x, y) = (SAVE_X as usize + col, SAVE_Y as usize + row);
            let in_box = (CUR_X as usize..CUR_X as usize + BOX as usize).contains(&x)
                && (CUR_Y as usize..CUR_Y as usize + BOX as usize).contains(&y);
            if !in_box {
                assert_eq!(pixel(col, row), ground, "saved pixel ({x},{y}) outside the sprite box is not the slate");
            }
        }
    }
    let mut got = [[0u32; BOX as usize]; BOX as usize];
    for (dy, line) in got.iter_mut().enumerate() {
        for (dx, px) in line.iter_mut().enumerate() {
            let col = (CUR_X - SAVE_X) as usize + dx;
            let row = (CUR_Y - SAVE_Y) as usize + dy;
            *px = pixel(col, row);
        }
    }
    cursor_hide(b);
    check_box(b, "bb_save stored the pointer", &got, ground);
}

// ── the pair: bb_save / bb_restore as the form manager uses them ─────────

/// The sequence `fm_alert` runs, at the `bb_set` level: save, draw over the
/// area under a bracket of its own, restore. After the cursor is taken away
/// the area must be the slate again — no ghost.
#[test]
fn bb_save_bb_restore_round_trip_leaves_no_ghost() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let h = rig();
    let b = &h.raster;
    let ground = slate_device(b);

    reset_cursor(b);
    slate(b);
    let before = b.snapshot();
    mouse::set_locator_pos(CUR_X, CUR_Y);
    cursor_show(b);

    let words = bb_save_words(SAVE_W, SAVE_H);
    let mut buf = vec![0u32; words];
    // SAFETY: `buf` holds `bb_save_words(SAVE_W, SAVE_H)` u32 words.
    unsafe { bb_save(b, buf.as_mut_ptr(), SAVE_X, SAVE_Y, SAVE_W, SAVE_H) };

    // "The alert": the area painted under a bracket, as `ob_draw` does.
    cursor_hide(b);
    b.fill_rect(SAVE_X, SAVE_Y, SAVE_W, SAVE_H, 8, ClipRect::full(SCREEN_W, SCREEN_H));
    cursor_show(b);

    // The restore under a bracket of the caller's — the shape `form.rs` had
    // before A66 (hide → restore → show). It is what turns the captured
    // pointer into a *ghost*: the erase before the restore puts the sprite's
    // own save-under back, the restore then writes the captured arrow, and the
    // show re-takes a save-under of that arrow. Nested and inert once
    // `bb_restore` brackets itself (the hide depth counts, EmuTOS `gl_moff`).
    cursor_hide(b);
    // SAFETY: `buf` holds the image `bb_save` stored for this rectangle.
    unsafe {
        bb_restore(b, buf.as_ptr(), SAVE_X, SAVE_Y, SAVE_W, SAVE_H, ClipRect::full(SCREEN_W, SCREEN_H));
    }
    cursor_show(b);
    cursor_hide(b);
    let after = b.snapshot();

    check_box(b, "bb_restore wrote the pointer back", &box_from_screen(&after), ground);
    assert_eq!(before, after, "the restored screen differs from the original outside the sprite box");
}

// ── the screen: form_alert end to end ────────────────────────────────────

/// `form_alert` (52): `int_in[0]` = default button, `addr_in[0]` = the string.
fn form_alert(pb: &mut Aespb, s: &[u8], defbut: i16) -> i16 {
    pb.intin.fill(0);
    pb.intin[0] = defbut;
    pb.addrin.fill(0);
    pb.addrin[0] = s.as_ptr() as u64;
    pb.call(52, 1, 1, 1);
    pb.intout[0]
}

/// A real alert over the visible pointer, opened and closed: nothing of the
/// pointer may survive at the opening position (`TASK/A66.md`, acceptance).
#[test]
fn form_alert_leaves_no_ghost_arrow_at_the_opening_position() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let h = rig();
    let b = &h.raster;
    let ground = slate_device(b);
    let mut pb = Aespb::new();

    reset_cursor(b);
    slate(b);
    let before = b.snapshot();
    mouse::set_locator_pos(CUR_X, CUR_Y);
    cursor_show(b);

    // Enter fires the DEFAULT button (`modal_loop`); the string ends in NUL for
    // the GEM contract.
    *h.while_open.lock().unwrap() = None;
    h.keys.lock().unwrap().push_back(b'\r');
    let chosen = form_alert(&mut pb, b"[1][Hello][OK]\0", 1);
    assert_eq!(chosen, 1, "Enter must fire the default button");

    // Precondition: the alert really covered the pointer while it was open —
    // the pixel just left of the sprite box was repainted by the alert.
    let open = h.while_open.lock().unwrap().take().expect("the modal loop never asked for a key");
    assert_ne!(at(&open, CUR_X - 1, CUR_Y), ground, "the alert did not cover the pointer position");

    cursor_hide(b);
    let after = b.snapshot();

    check_box(b, "form_alert left a ghost arrow", &box_from_screen(&after), ground);
    assert_eq!(before, after, "the screen after the alert differs from the screen before it");
}
