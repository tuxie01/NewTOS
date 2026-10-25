//! A111 — the `objc_edit` text cursor is EmuTOS `curfld`'s XOR line.
//!
//! **Soll (`aes/gemobed.c:194-218` `curfld`, `:335-463` `ob_edit`).** With
//! `dist == 0` `curfld` sets `gsx_attr(FALSE, MD_XOR, BLACK)`, widens the
//! `pxl_rect` cell to `y - 3` / `gl_hchar + 6` rows and draws ONE vertical
//! `gsx_cline` there — a 1-pixel solid line in XOR mode (line type 7, `udsty`
//! 0xffff, width 1: `gemgraf.c:420-422`). XOR is its own inverse, so `ob_edit`
//! uses the same call to switch the caret off and on:
//!
//! * `EDINIT` — index to the end, then `curfld(…, 0)`: caret ON, no field draw.
//! * `EDCHAR` — `curfld(start, 0)` (OFF at the old place), edit, and only when
//!   the text changed `curfld(min(start, nstart), dist)` = `ob_draw` of the
//!   field clipped to the changed columns; then `curfld(new, 0)` (ON).
//! * `EDEND`  — only the final `curfld(…, 0)`: caret OFF.
//!
//! The invariant tested here: after every call of an `EDINIT … EDCHAR* …
//! EDEND` sequence the screen equals a clean draw of the dialog plus exactly
//! one complemented `gl_hchar + 6`-pixel column at the caret (none after
//! `EDEND`). Spec: `docs/api/aes/object-form.md` `objc_edit`.

#![allow(clippy::too_many_arguments)]

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
                if mode == WriteMode::Xor {
                    // Complement the stored pixel (`kernel/src/vdi_backend.rs`
                    // `combine`/`xor_pixel`, EmuTOS `*addr = ~*addr`).
                    if Self::inside(x as i16, y as i16) {
                        let old = self.px.lock().unwrap()[Self::idx(x as i16, y as i16)];
                        self.put(x as i16, y as i16, old ^ 0x00FF_FFFF, clip);
                    }
                } else {
                    self.put(x as i16, y as i16, v, clip);
                }
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
        let v = self.rgb565_to_device(self.pel_to_device16(fg));
        // `Replace` (opaque text) also paints the clear bits with pen 0.
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

// ── the host: a screen and nothing else ────────────────────────────────────

struct KeyHost {
    raster: BufBackend,
}

impl Host for KeyHost {
    fn console_write_byte(&self, _byte: u8) {}
    fn console_try_read_key(&self) -> Option<u16> {
        None
    }
    fn raster(&self) -> Option<&dyn RasterBackend> {
        Some(&self.raster)
    }
}

// SAFETY: every mutable field is behind a Mutex.
unsafe impl Sync for KeyHost {}
unsafe impl Send for KeyHost {}

// ── AES parameter block ────────────────────────────────────────────────────

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

    /// Issue one AES call, exactly as the GEM-trap handler does.
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

fn install(host: &'static KeyHost) {
    let href: &'static &'static dyn Host = Box::leak(Box::new(host as &'static dyn Host));
    tos_api::host::install_host(href);
}

/// Seed the backend's pseudo-palette the way the VDI dispatcher does
/// (`ACTIVE_PALETTE`, `kernel/src/vdi_backend.rs:11-21`): EmuTOS'
/// `videl_palette1` for pens 0..15 as RGB555-in-565 words, so `read_pixel`
/// round-trips the pens the menu draws with.
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

const G_FBOXTEXT: u16 = 30;
const EDITABLE: u16 = 0x0008;
const LASTOB: u16 = 0x0020;


/// Byte layout of the LP64 `TEDINFO`: three 8-byte pointers, then eight WORDs.
#[repr(C)]
struct Ted {
    ptext: u64,
    ptmplt: u64,
    pvalid: u64,
    font: i16,
    fontid: i16,
    just: i16,
    color: i16,
    fontsize: i16,
    thickness: i16,
    txtlen: i16,
    tmplen: i16,
}

static SERIAL: Mutex<()> = Mutex::new(());
static RIG: OnceLock<&'static KeyHost> = OnceLock::new();

fn rig() -> &'static KeyHost {
    RIG.get_or_init(|| {
        let host: &'static KeyHost = Box::leak(Box::new(KeyHost { raster: BufBackend::new() }));
        install(host);
        seed_palette(&host.raster);
        host
    })
}

/// `gl_wchar` / `gl_hchar` of the system font.
const WCHAR: i16 = 8;
const HCHAR: i16 = 16;
/// The field: exactly one text row high, so the caret's `-3`/`+6` extent
/// reaches three rows into the parent box above and below it.
const FIELD: (i16, i16, i16, i16) = (100, 100, 200, HCHAR);
// The parent is a red dither (pattern 4, pen 2, `ob_spec` 0x1142), so the
// caret's rows outside the field complement two different colours.
/// A ground no draw produces, so every pixel a call touches shows.
const GROUND: u32 = 0x00AB_CDEF;

const ED_INIT: i16 = 1;
const ED_CHAR: i16 = 2;
const ED_END: i16 = 3;

/// A dialog with one editable `G_FBOXTEXT` (`TE_LEFT`, template `________`,
/// validation `X` = anything, `te_txtlen` 9, no border) holding "AB". Boxed,
/// so its white interior is filled on every draw: a redraw of a few columns
/// gives the same pixels as a full one.
struct Dialog {
    text: Box<[u8; 9]>,
    _ted: Box<Ted>,
    tree: Box<[Obj; 2]>,
}

impl Dialog {
    fn new() -> Dialog {
        let mut text = Box::new(*b"AB\0\0\0\0\0\0\0");
        let tmpl: &'static [u8] = b"________\0";
        let valid: &'static [u8] = b"XXXXXXXX\0";
        let ted = Box::new(Ted {
            ptext: text.as_mut_ptr() as u64,
            ptmplt: tmpl.as_ptr() as u64,
            pvalid: valid.as_ptr() as u64,
            font: 3,
            fontid: 0,
            just: 0,
            color: 0x1180,
            fontsize: 0,
            thickness: 0,
            txtlen: 9,
            tmplen: 9,
        });
        let (fx, fy, fw, fh) = FIELD;
        let tree = Box::new([
            Obj { next: -1, head: 1, tail: 1, otype: 20, flags: 0, state: 0, spec: 0x0000_1142, x: 0, y: 0, w: 640, h: 480 },
            Obj {
                next: 0,
                head: -1,
                tail: -1,
                otype: G_FBOXTEXT,
                flags: EDITABLE | LASTOB,
                state: 0,
                spec: &*ted as *const Ted as u64,
                x: fx,
                y: fy,
                w: fw,
                h: fh,
            },
        ]);
        Dialog { text, _ted: ted, tree }
    }

    fn text(&self) -> &[u8] {
        let n = self.text.iter().position(|&b| b == 0).unwrap_or(self.text.len());
        &self.text[..n]
    }

    /// `objc_draw(tree, ROOT, MAX_DEPTH, full screen)`.
    fn draw(&mut self, pb: &mut Aespb) {
        pb.intin.fill(0);
        pb.intin[1] = 8;
        pb.intin[4] = SCREEN_W as i16;
        pb.intin[5] = SCREEN_H as i16;
        pb.addrin[0] = self.tree.as_mut_ptr() as u64;
        pb.call(42, 6, 1, 1);
    }

    /// `objc_edit(tree, 1, ch, idx, kind)` → the new index.
    fn edit(&mut self, pb: &mut Aespb, ch: i16, idx: i16, kind: i16) -> i16 {
        pb.intin.fill(0);
        pb.intin[0] = 1;
        pb.intin[1] = ch;
        pb.intin[2] = idx;
        pb.intin[3] = kind;
        pb.addrin[0] = self.tree.as_mut_ptr() as u64;
        pb.call(46, 4, 2, 1);
        assert_eq!(pb.intout[0], 1, "ob_edit returns TRUE");
        pb.intout[1]
    }
}

fn screen(host: &KeyHost) -> Vec<u32> {
    host.raster.px.lock().unwrap().clone()
}

/// The screen a clean draw of the dialog gives, with the caret at raw index
/// `caret` complemented in (`None` = no caret). The live screen is put back.
fn expected(host: &KeyHost, pb: &mut Aespb, d: &mut Dialog, caret: Option<i16>) -> Vec<u32> {
    let live = screen(host);
    host.raster.px.lock().unwrap().fill(GROUND);
    d.draw(pb);
    let mut exp = screen(host);
    if let Some(idx) = caret {
        // Template "________": find_pos is the identity; TE_LEFT, no vertical
        // slack (field height == gl_hchar), so pxl_rect = (fx + idx*8, fy).
        let (fx, fy, ..) = FIELD;
        let x = fx + idx * WCHAR;
        for y in (fy - 3)..(fy - 3 + HCHAR + 6) {
            exp[y as usize * SCREEN_W as usize + x as usize] ^= 0x00FF_FFFF;
        }
    }
    *host.raster.px.lock().unwrap() = live;
    exp
}

fn diff(a: &[u32], b: &[u32]) -> Vec<(i16, i16)> {
    a.iter()
        .zip(b.iter())
        .enumerate()
        .filter(|(_, (p, q))| p != q)
        .map(|(i, _)| ((i % SCREEN_W as usize) as i16, (i / SCREEN_W as usize) as i16))
        .collect()
}

/// `EDINIT` on a drawn dialog changes exactly the caret column: `gl_hchar + 6`
/// pixels from `pxl_rect.y - 3`, each complemented (`MD_XOR`), and nothing of
/// the field itself (`ob_edit` `EDINIT` does not redraw, `gemobed.c:363-365`).
#[test]
fn ed_init_draws_only_the_xor_caret() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let host = rig();
    let mut pb = Aespb::new();
    pb.call(10, 0, 1, 0); // appl_init
    let mut d = Dialog::new();
    host.raster.px.lock().unwrap().fill(GROUND);
    d.draw(&mut pb);
    let before = screen(host);
    let idx = d.edit(&mut pb, 0, 0, ED_INIT);
    assert_eq!(idx, 2, "EDINIT puts the index at the end of the text");
    let after = screen(host);
    let changed = diff(&before, &after);
    let (fx, fy, ..) = FIELD;
    let x = fx + 2 * WCHAR;
    let want: Vec<(i16, i16)> = ((fy - 3)..(fy + HCHAR + 3)).map(|y| (x, y)).collect();
    assert_eq!(changed, want, "EDINIT: one column at x={x}, rows {}..={}", fy - 3, fy + HCHAR + 2);
    for &(cx, cy) in &changed {
        let i = cy as usize * SCREEN_W as usize + cx as usize;
        assert_eq!(after[i], before[i] ^ 0x00FF_FFFF, "caret pixel ({cx},{cy}) is the complement");
    }
}

/// Through a whole edit — insert, arrows, backspace, delete, escape, insert,
/// end — the screen is always the clean dialog plus one XOR caret, and after
/// `EDEND` the clean dialog alone: every caret is taken off where it was put.
#[test]
fn edit_sequence_leaves_exactly_one_caret() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let host = rig();
    let mut pb = Aespb::new();
    pb.call(10, 0, 1, 0); // appl_init
    let mut d = Dialog::new();
    host.raster.px.lock().unwrap().fill(GROUND);
    d.draw(&mut pb);

    let mut idx = d.edit(&mut pb, 0, 0, ED_INIT);
    assert_eq!(idx, 2);
    let exp = expected(host, &mut pb, &mut d, Some(idx));
    assert_eq!(diff(&screen(host), &exp), vec![], "after EDINIT");

    // (key word, text after, index after)
    let steps: [(i16, &[u8], i16, &str); 8] = [
        (0x2E43, b"ABC", 3, "insert C"),
        (0x4B00, b"ABC", 2, "arrow left"),
        (0x4B00, b"ABC", 1, "arrow left"),
        (0x4D00, b"ABC", 2, "arrow right"),
        (0x0E08, b"AC", 1, "backspace"),
        (0x537F, b"A", 1, "delete"),
        (0x011B, b"", 0, "escape"),
        (0x2C5A, b"Z", 1, "insert Z"),
    ];
    for (key, text, want_idx, what) in steps {
        idx = d.edit(&mut pb, key, idx, ED_CHAR);
        assert_eq!((d.text(), idx), (text, want_idx), "{what}: text and index");
        let exp = expected(host, &mut pb, &mut d, Some(idx));
        assert_eq!(diff(&screen(host), &exp), vec![], "{what}: clean field + one XOR caret at index {idx}");
    }

    let end = d.edit(&mut pb, 0, idx, ED_END);
    assert_eq!(end, idx, "EDEND keeps the index");
    let exp = expected(host, &mut pb, &mut d, None);
    assert_eq!(diff(&screen(host), &exp), vec![], "after EDEND: the clean dialog, no caret left behind");
}
