//! A133 — TEDINFO text (`G_TEXT`, `G_BOXTEXT`, `G_FTEXT`, `G_FBOXTEXT`) is
//! written in the VDI mode EmuTOS takes from the colour word.
//!
//! EmuTOS `just_draw` cracks `te_color` (`gr_crack`, `aes/gemgraf.c:621`:
//! bit 7 → `MD_REPLACE`, else `MD_TRANS`), switches a hollow-white 3D text to
//! `MD_TRANS` (`aes/gemoblib.c:586-588`) and a SELECTED colour-changing one
//! too (`:702-706`), then sets it with `gsx_attr(TRUE, tmode, tcol)` (`:709`)
//! before `gr_gtext` (`:757`). The VDI's replace-mode text paints each
//! character cell's clear pixels in pen 0 (`vdi/vdi_textblit.c:766,1044`).
//! Spec: `docs/api/aes/object-form.md`, "Text writing mode (A133)".
//!
//! Harness (backend + host + parameter block) from
//! `aes_object_3d_text_button.rs`, its glyph blit made mode-aware. 640×480,
//! Falcon pseudo-palette: pen 0 white, 1 black, 2 red, 8 LWHITE.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use tos_api::aes::object::{Object, TedInfo};
use tos_api::bdos::process::Abi;
use tos_api::host::Host;
use tos_api::vdi::backend::{ClipRect, MonoSrc, PatternFill, Pel, RasterBackend, Rect, WriteMode};
use tos_api::vdi::mouse;

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
        mode: WriteMode,
        clip: ClipRect,
    ) {
        // The writing mode matters here (A133): `Replace` paints the cell's
        // clear pixels in pen 0 like the kernel backend
        // (`kernel/src/vdi_backend.rs` `draw_glyph_sized`), `Transparent`
        // leaves them alone. The AES text path uses no other mode.
        let v = self.rgb565_to_device(self.pel_to_device16(fg));
        let bg = self.rgb565_to_device(self.pel_to_device16(0));
        for row in 0..cell_h {
            let byte = glyph.get(row as usize).copied().unwrap_or(0);
            for col in 0..cell_w.min(8) {
                let set = byte & (0x80 >> col) != 0;
                match mode {
                    WriteMode::Replace => self.put(x + col, y + row, if set { v } else { bg }, clip),
                    WriteMode::Transparent => {
                        if set {
                            self.put(x + col, y + row, v, clip)
                        }
                    }
                    _ => panic!("AES text drawn with {mode:?}"),
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

static SERIAL: Mutex<()> = Mutex::new(());
static RIG: OnceLock<&'static TestHost> = OnceLock::new();

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

/// EmuTOS' `videl_palette1` (`vdi/vdi_col.c:130-136`): pen 0 white, 1 black,
/// 8 LWHITE (733), 9 LBLACK (533) — the four classes the expected rows use.
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

// ── objects ───────────────────────────────────────────────────────────────

const G_BOX: u16 = 20;
const G_TEXT: u16 = 21;
const G_BOXTEXT: u16 = 22;
const G_FTEXT: u16 = 29;
const G_FBOXTEXT: u16 = 30;
const SELECTED: u16 = 0x0001;
const LASTOB: u16 = 0x0020;
const FL3DIND: u16 = 0x0200;
const FL3DBAK: u16 = 0x0400;
const NIL: i16 = -1;

const TE_LEFT: i16 = 0;
const TE_CNTR: i16 = 2;

// Pens of the seeded palette.
const WHITE: Pel = 0;
const BLACK: Pel = 1;
const RED: Pel = 2;
const LWHITE: Pel = 8;

/// `te_color` words (`gr_crack`, `gemgraf.c:612-628`): border 1, text 1
/// (black), inside pattern 7 (solid), inside colour 2 (red); bit 7 =
/// MD_REPLACE.
const TE_SOLID_RED_REPLACE: i16 = 0x11F2;
const TE_SOLID_RED_TRANS: i16 = 0x1172;
/// Border 1, text 1, hollow, white, bit 7 = MD_REPLACE (EmuDESK's 4480).
const TE_HOLLOW_WHITE_REPLACE: i16 = 0x1180;

/// The ground: a solid red `G_BOX` without border (`ob_spec` colour word
/// `0x0072`: pattern 7, inside colour 2, thickness byte 0).
const ROOT_SPEC: u64 = 0x0000_0072;

/// A device value no palette pen produces: "nothing drew here".
const SLATE: u32 = 0x0012_3456;

// The root box and the text child's place inside it.
const RX: i16 = 200;
const RY: i16 = 100;
const RW: i16 = 240;
const RH: i16 = 64;
const CX: i16 = 16; // child, relative to the root
const CY: i16 = 16;

/// ROOT (solid red G_BOX) → one TEDINFO child of `ob_type`/`flags`/`state`
/// at `(CX, CY, w, h)`.
fn tree(ob_type: u16, flags: u16, state: u16, spec: u64, w: i16, h: i16) -> [Object; 2] {
    [
        Object {
            ob_next: NIL,
            ob_head: 1,
            ob_tail: 1,
            ob_type: G_BOX,
            ob_flags: 0,
            ob_state: 0,
            ob_spec: ROOT_SPEC,
            ob_x: RX,
            ob_y: RY,
            ob_width: RW,
            ob_height: RH,
        },
        Object {
            ob_next: 0,
            ob_head: NIL,
            ob_tail: NIL,
            ob_type,
            ob_flags: flags | LASTOB,
            ob_state: state,
            ob_spec: spec,
            ob_x: CX,
            ob_y: CY,
            ob_width: w,
            ob_height: h,
        },
    ]
}

/// A leaked TEDINFO: IBM font, `text`, template `tmplt`, `just`, `color`,
/// `thickness`.
fn ted(text: &'static [u8], tmplt: &'static [u8], just: i16, color: i16, thickness: i16) -> u64 {
    static EMPTY: &[u8] = b"\0";
    let t = Box::leak(Box::new(TedInfo {
        te_ptext: text.as_ptr() as u64,
        te_ptmplt: tmplt.as_ptr() as u64,
        te_pvalid: EMPTY.as_ptr() as u64,
        te_font: 3,
        te_junk1: 0,
        te_just: just,
        te_color: color,
        te_junk2: 0,
        te_thickness: thickness,
        te_txtlen: text.len() as i16,
        te_tmplen: tmplt.len() as i16,
    }));
    t as *const TedInfo as u64
}

/// `objc_draw` (42) of the whole tree, no clip.
fn objc_draw(pb: &mut Aespb, t: &[Object; 2]) {
    pb.intin.fill(0);
    pb.intin[1] = 8;
    pb.addrin.fill(0);
    pb.addrin[0] = t.as_ptr() as u64;
    pb.call(42, 6, 1, 1);
    assert_eq!(pb.intout[0], 1, "objc_draw refused the tree");
}

/// Draw `t` on a slate screen with the cursor out of the way, hand back the
/// backend.
fn draw(t: &[Object; 2]) -> &'static BufBackend {
    let h = rig();
    let b = &h.raster;
    let mut pb = Aespb::new();
    mouse::show_reset(b);
    mouse::remove(b);
    b.px.lock().unwrap().fill(SLATE);
    objc_draw(&mut pb, t);
    b
}

fn pen(b: &BufBackend, x: i16, y: i16) -> Option<Pel> {
    if b.get_device_pixel(x, y) == SLATE {
        None
    } else {
        Some(b.read_pixel(x, y))
    }
}

/// Every pixel of the character cells of `text` (IBM font, cell top-left
/// `(x, y)`): BLACK where the glyph has a bit, `ground` where it has none.
/// Collects every mismatch into one panic message.
fn expect_cells(b: &BufBackend, x: i16, y: i16, text: &[u8], ground: Pel, what: &str) {
    expect_cells_ink(b, x, y, text, BLACK, ground, what)
}

/// [`expect_cells`] with the glyph colour `ink`.
fn expect_cells_ink(b: &BufBackend, x: i16, y: i16, text: &[u8], ink: Pel, ground: Pel, what: &str) {
    let mut bad = Vec::new();
    for (i, &c) in text.iter().enumerate() {
        let g = tos_api::vdi::font::glyph(c as u16);
        for row in 0..16i16 {
            for col in 0..8i16 {
                let (px, py) = (x + 8 * i as i16 + col, y + row);
                let want = if g[row as usize] & (0x80 >> col) != 0 { ink } else { ground };
                let got = pen(b, px, py);
                if got != Some(want) {
                    bad.push(format!("({px},{py}) {got:?} want {want}"));
                }
            }
        }
    }
    assert!(bad.is_empty(), "{what}: {} cell pixels differ, first: {:?}", bad.len(), &bad[..bad.len().min(8)]);
}

/// The pixels right of the cells, still inside the child (`x .. x + w`, rows
/// `y .. y + 16`), are all `ground`.
fn expect_span(b: &BufBackend, x: i16, y: i16, w: i16, ground: Pel, what: &str) {
    for py in y..y + 16 {
        for px in x..x + w {
            assert_eq!(pen(b, px, py), Some(ground), "{what}: ({px},{py}) beside the cells");
        }
    }
}

/// The four TEDINFO kinds with their text and template: the formatted kinds
/// merge `te_ptext` into `te_ptmplt` (`ob_format`), so all four draw "AB".
fn kinds() -> [(u16, &'static [u8], &'static [u8], &'static str); 4] {
    [
        (G_TEXT, b"AB\0", b"\0", "G_TEXT"),
        (G_BOXTEXT, b"AB\0", b"\0", "G_BOXTEXT"),
        (G_FTEXT, b"AB\0", b"__\0", "G_FTEXT"),
        (G_FBOXTEXT, b"AB\0", b"__\0", "G_FBOXTEXT"),
    ]
}

// ── the writing mode from the colour word ─────────────────────────────────

/// Bit 7 set (MD_REPLACE, `gr_crack` `gemgraf.c:621`; `gsx_attr(TRUE,
/// tmode, tcol)` `gemoblib.c:709`): every character cell is painted, the
/// glyph in the text colour and every other cell pixel in VDI pen 0 (white;
/// `vdi/vdi_textblit.c:766,1044`). Beside the cells the ground (the red box
/// fill, or for G_TEXT/G_FTEXT the red parent) stays.
#[test]
fn replace_mode_text_paints_its_cells_pen_0_for_all_tedinfo_kinds() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    for (ty, text, tmplt, name) in kinds() {
        let t = tree(ty, 0, 0, ted(text, tmplt, TE_LEFT, TE_SOLID_RED_REPLACE, 0), 64, 16);
        let b = draw(&t);
        let (x, y) = (RX + CX, RY + CY);
        expect_cells(b, x, y, b"AB", WHITE, name);
        expect_span(b, x + 16, y, 48, RED, name);
    }
}

/// Bit 7 clear (MD_TRANS): the glyph only; the red ground shows between the
/// strokes.
#[test]
fn transparent_mode_text_leaves_its_cells_ground_for_all_tedinfo_kinds() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    for (ty, text, tmplt, name) in kinds() {
        let t = tree(ty, 0, 0, ted(text, tmplt, TE_LEFT, TE_SOLID_RED_TRANS, 0), 64, 16);
        let b = draw(&t);
        let (x, y) = (RX + CX, RY + CY);
        expect_cells(b, x, y, b"AB", RED, name);
        expect_span(b, x + 16, y, 48, RED, name);
    }
}

/// The 3D switch (`gemoblib.c:582-599`): under any 3D flag a hollow-white
/// MD_REPLACE text gets the solid `backgrcol` fill **and** `tmode =
/// MD_TRANS` (`:588`), so the fill shows between the glyphs — for the
/// promoted G_TEXT/G_FTEXT and for the boxed kinds alike.
#[test]
fn fl3dbak_hollow_white_replace_text_is_written_transparent_over_backgrcol() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    for (ty, text, tmplt, name) in kinds() {
        let t = tree(ty, FL3DBAK, 0, ted(text, tmplt, TE_LEFT, TE_HOLLOW_WHITE_REPLACE, 0), 64, 16);
        let b = draw(&t);
        let (x, y) = (RX + CX, RY + CY);
        expect_cells(b, x, y, b"AB", LWHITE, name);
        expect_span(b, x + 16, y, 48, LWHITE, name);
    }
}

/// SELECTED with `changecol` (an FL3DIND object, `indcolchange = 1`):
/// `tmode = MD_TRANS` and the text colour complemented (`gemoblib.c:702-706`,
/// `xor_color(BLACK)` = WHITE on a 256-colour screen, `vdi16comp`), the
/// interior swapped (`xor_color(RED)`, `:689`). In MD_TRANS the swapped face
/// shows between the white glyph strokes; MD_REPLACE would paint those
/// pixels pen 0 too, i.e. the whole cell white. FL3DIND grows the object by
/// ADJ3DSTD on every side (the 16-px text sits in a 20-px box, `gr_just`
/// centres it back onto the nominal y), `indtxtmove = 0` keeps it still;
/// TE_CNTR keeps the cells off the bevel at the grown edge.
#[test]
fn selected_changecol_replace_text_is_written_transparent() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let t = tree(G_BOXTEXT, FL3DIND, SELECTED, ted(b"AB\0", b"\0", TE_CNTR, TE_SOLID_RED_REPLACE, 0), 64, 16);
    let b = draw(&t);
    // Centred in the grown 68-px box, clear of its bevel: x - 2 + (68 - 16) / 2.
    let (x, y) = (RX + CX - 2 + 26, RY + CY);
    let face = pen(b, x - 8, y + 8).expect("the selected face was drawn");
    assert!(face != RED && face != WHITE, "the SELECTED interior takes xor_color(RED), got {face}");
    expect_cells_ink(b, x, y, b"AB", WHITE, face, "G_BOXTEXT FL3DIND SELECTED");
}

// ── the file selector's FTITLE ────────────────────────────────────────────

/// `FTITLE` as `fs_start` leaves it (`aes/gem_rsc.c:37-40,379-383`,
/// `aes/gemfslib.c:78-82`): G_BOXTEXT, FL3DBAK, colour `0x11C1` (bit 7 set,
/// IP_4PATT, inside colour 1), `te_thickness = -1`, TE_CNTR, 20 characters
/// wide and `1 char + 2*ADJ3DSTD` high, text `*.*`. The pattern is not
/// hollow, so the 3D switch does not fire and the text stays MD_REPLACE:
/// the three cells are white under black glyphs, the 50 % checker shows
/// only beside them (`measurements/measurement-2026-11-04-a132/RUNINFO.TXT`:
/// 140 checker pixels inside the cells before A133).
#[test]
fn ftitle_mask_cells_are_white_like_emutos() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let t = tree(G_BOXTEXT, FL3DBAK, 0, ted(b"*.*\0", b"\0", TE_CNTR, 0x11C1u16 as i16, -1), 160, 20);
    let b = draw(&t);
    // gr_just: slack 160 - 24 = 136 → x + 68; 20 - 16 = 4 → y + 2.
    let (x, y) = (RX + CX + 68, RY + CY + 2);
    expect_cells(b, x, y, b"*.*", WHITE, "FTITLE");
    // Beside the cells the checker: both black and white pixels.
    let (mut blk, mut wht) = (0, 0);
    for py in y..y + 16 {
        for px in x + 24..x + 40 {
            match pen(b, px, py) {
                Some(BLACK) => blk += 1,
                Some(WHITE) => wht += 1,
                other => panic!("FTITLE: ({px},{py}) {other:?} beside the cells, want the checker"),
            }
        }
    }
    assert!(blk > 0 && wht > 0, "FTITLE: the IP_4PATT checker beside the cells ({blk} black, {wht} white)");
}
