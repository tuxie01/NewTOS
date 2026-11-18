//! A85 — `just_draw`'s SELECTED rendering per object type × 3D flag class
//! (`docs/api/aes/object-form.md` item 8, EmuTOS `aes/gemoblib.c`).
//!
//! The EmuTOS rule (`gemoblib.c:504-525`, `:681-690`, `:699-706`,
//! `:932-943`, `:950-951`):
//!
//! * `FL3DOBJ` (`FL3DIND`/`FL3DACT`, `obdefs.h:92`): `changecol` is the
//!   `objc_sysvar` variable (`indcolchange`, default 1 / `actcolchange`,
//!   default 0, `init_3d` `:1329-1333`); `movetext` likewise
//!   (`indtxtmove` 0 / `acttxtmove` 1). A SELECTED object with `changecol`
//!   swaps its interior (`hollow → solid BLACK`, else `xor_color(icol)`) and
//!   its text colour (`:681-690`, `:699-706`, `:807-808`); one without keeps
//!   both. **Never** the trailing XOR (`:936`). `add_3d_effect` sinks the
//!   bevel (`:950-951`, `:252`, `:270`).
//! * Not `FL3DOBJ` (no 3D **or `FL3DBAK`**, `obdefs.h:97`): `changecol =
//!   !xor_is_ok(obtype, flags, spec)` (`:524`); `xor_is_ok` (`:107-148`) is
//!   TRUE for `G_IBOX`/`G_STRING`/`G_TITLE`, FALSE for `G_IMAGE`, and for the
//!   box/text families TRUE unless (at > 16 colours) the *spec's* interior
//!   or text colour is beyond BLACK. `G_BUTTON`/`G_USERDEF`/icons take the
//!   `default:` → TRUE. With `changecol` FALSE the object is drawn as usual
//!   and then the whole rect — inset by `th` when `th > 0` (`:889-892`) —
//!   is XOR-inverted (`bb_fill(MD_XOR, …)`, `:940`): in the 16-bit VDI a
//!   bitwise complement of every device pixel (`vdi_line.c:484-495`).
//!
//! # The profile
//!
//! 640×480 Falcon truecolor, pseudo-palette seeded like `init_wk` does
//! (`aes_object_3d_box_border.rs`). Class letters: `.` white, `#` black,
//! `r` red, `g` green, `l` LWHITE, `d` LBLACK, `c` LCYAN, `L` = the 24-bit
//! complement of LWHITE (what an XOR over a `backgrcol` face yields on this
//! device — no palette pen), `_` the slate ("nothing drew here"). Every
//! object sits at `(220, 100, 24, 16)` like the sibling tests; the
//! "background" cells first paint `(200, 90, 70, 40)` in a pen so an XOR
//! over the *underlying* pixels is tellable from a fill.

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

    fn clipped(x: i16, y: i16, clip: ClipRect) -> bool {
        if !Self::inside(x, y) {
            return true;
        }
        if let Some((x0, y0, x1, y1)) = clip.clamp_to(SCREEN_W, SCREEN_H) {
            let (xi, yi) = (x as i32, y as i32);
            if xi < x0 || xi > x1 || yi < y0 || yi > y1 {
                return true;
            }
        }
        false
    }

    fn put(&self, x: i16, y: i16, v: u32, clip: ClipRect) {
        if Self::clipped(x, y, clip) {
            return;
        }
        self.px.lock().unwrap()[Self::idx(x, y)] = v;
    }

    /// VDI `WM_XOR`: complement the 24-bit RGB of the destination (EmuTOS
    /// `vdi_line.c:494` `*work = ~temp`; kernel `VdiScreen::xor_pixel`).
    fn xor(&self, x: i16, y: i16, clip: ClipRect) {
        if Self::clipped(x, y, clip) {
            return;
        }
        let mut px = self.px.lock().unwrap();
        let i = Self::idx(x, y);
        px[i] ^= 0x00FF_FFFF;
    }

    fn dev(&self, pel: Pel) -> u32 {
        self.rgb565_to_device(self.pel_to_device16(pel))
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
    /// The kernel's convention (`VdiScreen::device_to_rgb565`): the wire
    /// word is RGB555-in-565 — green in bits 6-10, bit 5 the overlay bit —
    /// as `seed_palette` packs it (EmuTOS `set_color16`, `vdi_col.c`).
    fn device_to_rgb565(&self, argb: u32) -> u16 {
        let r5 = ((argb >> 19) & 0x1f) as u16;
        let g5 = ((argb >> 11) & 0x1f) as u16;
        let b5 = ((argb >> 3) & 0x1f) as u16;
        (r5 << 11) | (g5 << 6) | b5
    }
    /// The kernel's `pal555_to_xrgb`: 5→8-bit gun replication, so WHITE is
    /// `0xFFFFFF` and BLACK `0x000000` — the XOR of one is the other,
    /// exactly as on the device (a 6-bit green decode of the 5-bit word
    /// would leave WHITE at `0xFFFBFF` and no b/w XOR cell could match).
    fn rgb565_to_device(&self, px: u16) -> u32 {
        let x = |c5: u16| ((c5 << 3) | (c5 >> 2)) as u32;
        (x((px >> 11) & 0x1F) << 16) | (x((px >> 6) & 0x1F) << 8) | x(px & 0x1F)
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
        let v = self.dev(pel);
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
        let fg = self.dev(pel);
        let bg = self.dev(0);
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
                    self.xor(x, y, clip);
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
    /// Mode-aware (the sibling harnesses paint every pattern opaque): the
    /// XOR cells of this file depend on `WriteMode::Xor` complementing the
    /// destination exactly like the kernel's `combine`.
    fn fill_pattern(&self, x: i16, y: i16, w: i16, h: i16, fill: PatternFill, clip: ClipRect) {
        let fg = self.dev(fill.fg);
        let bg = self.dev(fill.bg);
        for yy in y..y.saturating_add(h) {
            let row = fill.pattern[(yy.rem_euclid(16)) as usize % fill.pattern.len()];
            for xx in x..x.saturating_add(w) {
                let set = row & (0x8000u16 >> (xx.rem_euclid(16) as u16)) != 0;
                match fill.mode {
                    WriteMode::Replace => self.put(xx, yy, if set { fg } else { bg }, clip),
                    WriteMode::Transparent => {
                        if set {
                            self.put(xx, yy, fg, clip)
                        }
                    }
                    WriteMode::Xor => {
                        if set {
                            self.xor(xx, yy, clip)
                        }
                    }
                    WriteMode::RevTransparent => {
                        if !set {
                            self.put(xx, yy, fg, clip)
                        }
                    }
                }
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
        let v = self.dev(fg);
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
        let f = self.dev(fg);
        let b = self.dev(bg);
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
/// 2 red, 3 green, 8 LWHITE (733), 9 LBLACK (533), 13 LCYAN (0,667,667).
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
const G_IBOX: u16 = 25;
const G_BUTTON: u16 = 26;
const G_STRING: u16 = 28;
const G_FTEXT: u16 = 29;
const G_TITLE: u16 = 32;
const LASTOB: u16 = 0x0020;
const FL3DIND: u16 = 0x0200;
const FL3DBAK: u16 = 0x0400;
const FL3DACT: u16 = 0x0600;
const SELECTED: u16 = 0x0001;
const NIL: i16 = -1;

// `te_color` words (`gr_crack`): border 1 (black), text 1 (black), inside
// pattern 0 (hollow), inside colour 0 (white); bit 7 = REPLACE.
const TE_HOLLOW_WHITE_REPLACE: i16 = 0x1180;
const TE_HOLLOW_WHITE_TRANS: i16 = 0x1100;

/// A device value no palette pen produces: "nothing drew here".
const SLATE: u32 = 0x0012_3456;

const OX: i16 = 220;
const OY: i16 = 100;
const OW: i16 = 24;
const OH: i16 = 16;

const WHITE: Pel = 0;
const LWHITE: Pel = 8;

fn object(ob_type: u16, flags: u16, state: u16, spec: u64) -> [Object; 1] {
    [Object {
        ob_next: NIL,
        ob_head: NIL,
        ob_tail: NIL,
        ob_type,
        ob_flags: flags | LASTOB,
        ob_state: state,
        ob_spec: spec,
        ob_x: OX,
        ob_y: OY,
        ob_width: OW,
        ob_height: OH,
    }]
}

/// A leaked TEDINFO: IBM font, `just`, `color`, `thickness`, `text`, `tmpl`.
fn ted(text: &'static [u8], tmpl: &'static [u8], just: i16, color: i16, thickness: i16) -> u64 {
    static EMPTY: &[u8] = b"\0";
    let t = Box::leak(Box::new(TedInfo {
        te_ptext: text.as_ptr() as u64,
        te_ptmplt: tmpl.as_ptr() as u64,
        te_pvalid: EMPTY.as_ptr() as u64,
        te_font: 3,
        te_junk1: 0,
        te_just: just,
        te_color: color,
        te_junk2: 0,
        te_thickness: thickness,
        te_txtlen: text.len() as i16,
        te_tmplen: tmpl.len() as i16,
    }));
    t as *const TedInfo as u64
}

/// `objc_draw` (42) of the whole tree, no clip.
fn objc_draw(pb: &mut Aespb, t: &[Object; 1]) {
    pb.intin.fill(0);
    pb.intin[1] = 8;
    pb.addrin.fill(0);
    pb.addrin[0] = t.as_ptr() as u64;
    pb.call(42, 6, 1, 1);
    assert_eq!(pb.intout[0], 1, "objc_draw refused the tree");
}

/// `objc_sysvar` (48) SET of a single-valued 3D colour variable.
fn sysvar_set(pb: &mut Aespb, which: i16, value: i16) {
    pb.intin.fill(0);
    pb.intin[0] = 1;
    pb.intin[1] = which;
    pb.intin[2] = value;
    pb.intin[3] = 0;
    pb.addrin.fill(0);
    pb.call(48, 4, 3, 0);
    assert_eq!(pb.intout[0], 1, "objc_sysvar SET {which} = {value} refused");
}

const INDBUTCOL: i16 = 3;
const RED: i16 = 2;
const LWHITE_I: i16 = 8;

/// Draw `t` on a slate screen (cursor out of the way); `ground` = paint
/// `(200, 90, 70, 40)` in that pen first, so the object lies on a defined
/// background (the parent's face), as it does inside a dialog.
fn draw_on(t: &[Object; 1], ground: Option<Pel>) -> &'static BufBackend {
    let h = rig();
    let b = &h.raster;
    let mut pb = Aespb::new();
    mouse::show_reset(b);
    mouse::remove(b);
    b.px.lock().unwrap().fill(SLATE);
    if let Some(pel) = ground {
        b.fill_rect(200, 90, 70, 40, pel, ClipRect::full(SCREEN_W, SCREEN_H));
    }
    objc_draw(&mut pb, t);
    b
}

fn draw(t: &[Object; 1]) -> &'static BufBackend {
    draw_on(t, None)
}

/// Pixel class at `(x, y)` — see the module doc for the letters.
fn class(b: &BufBackend, x: i16, y: i16) -> char {
    let v = b.get_device_pixel(x, y);
    if v == SLATE {
        return '_';
    }
    if v == b.dev(LWHITE) ^ 0x00FF_FFFF {
        return 'L';
    }
    for (pel, c) in [(0, '.'), (1, '#'), (2, 'r'), (3, 'g'), (8, 'l'), (9, 'd'), (13, 'c')] {
        if v == b.dev(pel) {
            return c;
        }
    }
    '?'
}

/// One screen row as a class string over `x0..=x1`.
fn row(b: &BufBackend, y: i16, x0: i16, x1: i16) -> String {
    (x0..=x1).map(|x| class(b, x, y)).collect()
}

/// The object's rect `(220..=243, 100..=115)` as one class string.
fn cell(b: &BufBackend) -> String {
    (OY..OY + OH).map(|y| row(b, y, OX, OX + OW - 1)).collect()
}

/// Compare `rows` (`y`, expected) over `x0..=x1`; `*` in the expectation
/// matches anything. Reports every differing row at once.
fn expect_rows(b: &BufBackend, x0: i16, x1: i16, rows: &[(i16, &str)], what: &str) {
    let mut bad = String::new();
    for &(y, want) in rows {
        let got = row(b, y, x0, x1);
        let same = got.chars().zip(want.chars()).all(|(g, w)| w == '*' || g == w);
        if !same || got.len() != want.len() {
            bad.push_str(&format!("  y={y:3}  ours |{got}|\n         want |{want}|\n"));
        }
    }
    assert!(bad.is_empty(), "{what}: rows differ from the expectation (x {x0}..{x1})\n{bad}");
}

/// The rows of an un-grown object at the slot with a 1-px *outside* black
/// border (`th = -1`: ring at `x = 219/244`, `y = 99/116`) and a uniform
/// interior of class `face`, over `x = 217..=246`; `edge` is what lies
/// beside the ring (slate `_`, or the ground's class).
fn outside_border_rows(face: char, edge: char) -> Vec<(i16, String)> {
    let e = edge.to_string();
    let ring = format!("{e}{e}{}{e}{e}", "#".repeat(26));
    let body = format!("{e}{e}#{}#{e}{e}", face.to_string().repeat(24));
    let mut rows = vec![(98, e.repeat(30)), (99, ring.clone())];
    rows.extend((100..=115).map(|y| (y, body.clone())));
    rows.push((116, ring));
    rows.push((117, e.repeat(30)));
    rows
}

fn expect_owned(b: &BufBackend, rows: &[(i16, String)], what: &str) {
    let borrowed: Vec<(i16, &str)> = rows.iter().map(|(y, s)| (*y, s.as_str())).collect();
    expect_rows(b, 217, 246, &borrowed, what);
}

// ── ticket cell 1: SELECTED FL3DBAK G_BOX → XOR-inverted ──────────────────

/// A hollow-white `G_BOX` under `FL3DBAK` (`th = -1`), SELECTED. `FL3DBAK`
/// is not `FL3DOBJ` (`obdefs.h:92,97`), so `changecol = !xor_is_ok(G_BOX,
/// flags, 0x1100)` = FALSE (interior 0 and text 1 are ≤ BLACK,
/// `gemoblib.c:140-147`): the face is filled `backgrcol` (`:631-634`) and
/// the trailing block XORs the whole rect (`:932-943`) — every interior
/// pixel becomes the complement of LWHITE (`vdi_line.c:494`), **not** the
/// LBLACK pen. No bevel (`:950`).
#[test]
fn a85_selected_fl3dbak_g_box_is_xor_inverted() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let t = object(G_BOX, FL3DBAK, SELECTED, 0x00ff_1100);
    let b = draw(&t);
    expect_owned(b, &outside_border_rows('L', '_'), "G_BOX (FL3DBAK, SELECTED)");
}

/// The same for a `G_BOXTEXT` under `FL3DBAK` (QED ships six SELECTABLE
/// ones): hollow-white → `backgrcol` (`:582-585`), `xor_is_ok` reads the
/// TEDINFO's `te_color` (`:129-131`, 0x1180 → OK), trailing XOR.
#[test]
fn a85_selected_fl3dbak_g_boxtext_is_xor_inverted() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    static TEXT: &[u8] = b"\0";
    static TMPL: &[u8] = b"\0";
    let t = object(G_BOXTEXT, FL3DBAK, SELECTED, ted(TEXT, TMPL, 2, TE_HOLLOW_WHITE_REPLACE, -1));
    let b = draw(&t);
    expect_owned(b, &outside_border_rows('L', '_'), "G_BOXTEXT (FL3DBAK, SELECTED)");
}

// ── ticket cell 2: SELECTED FL3DIND G_BOX → colour change, no XOR ─────────

/// A hollow-white `G_BOX` under `FL3DIND` (`th = -1`), SELECTED, with the
/// default `indcolchange = 1` (`init_3d`, `gemoblib.c:1330`): the face takes
/// `indbutcol` (`:623-626`) and, being `changecol` + SELECTED, is swapped to
/// `xor_color(LWHITE) = LBLACK` (`:681-690`, `vdi16comp[8] = LBLACK`,
/// `:83-86`); no trailing XOR (`FL3DOBJ`, `:936`); the bevel is sunk
/// (`add_3d_effect`, `:252` BLACK top-left, `:270` LWHITE bottom-right).
/// Grown by ADJ3DSTD: ring at `217/246 × 97/118`.
#[test]
fn a85_selected_fl3dind_g_box_changes_colour_without_xor() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let t = object(G_BOX, FL3DIND, SELECTED, 0x00ff_1100);
    let b = draw(&t);
    const FACE: &str = "##ddddddddddddddddddddddddddl#";
    let mut rows: Vec<(i16, &str)> = vec![
        (97, "##############################"),
        (98, "############################d#"),
    ];
    rows.extend((99..=116).map(|y| (y, FACE)));
    rows.extend([
        (117, "#dlllllllllllllllllllllllllll#"),
        (118, "##############################"),
    ]);
    expect_rows(b, 217, 246, &rows, "G_BOX (FL3DIND, SELECTED, indcolchange = 1)");
}

/// `xor_color` is EmuTOS's complement *table*, not `pen ^ 1`: with
/// `indbutcol = RED` (2) a SELECTED indicator's face is `vdi16comp[2] =
/// LCYAN` (13) (`gemoblib.c:83-86`, `:93-102`), not GREEN (3).
#[test]
fn a85_selected_fl3dind_xor_color_uses_the_complement_table() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let mut pb = Aespb::new();
    let _ = rig();
    sysvar_set(&mut pb, INDBUTCOL, RED);
    let t = object(G_BOX, FL3DIND, SELECTED, 0x00ff_1100);
    let b = draw(&t);
    sysvar_set(&mut pb, INDBUTCOL, LWHITE_I);
    assert_eq!(
        row(b, 108, 219, 244),
        "c".repeat(26),
        "SELECTED FL3DIND face with indbutcol = RED must be LCYAN (xor_color table), got {}",
        row(b, 108, 217, 246)
    );
}

// ── ticket cell 3: SELECTED FL3DACT G_BUTTON → no XOR, no colour, sunk ────

/// A `G_BUTTON` under `FL3DACT` (`th = -1`), label `X`, drawn raised and
/// pressed. `actcolchange = 0` (`init_3d`, `:1333`): the face keeps
/// `actbutcol` (`:647-650`, no `:681-690` swap) and the label stays BLACK
/// (`:807-808` not taken); `acttxtmove = 1`: the raised label sits at
/// `(tmpx-1, tmpy-1)` (`:833-836`), the pressed one at the base; no trailing
/// XOR (`:936`); the bevel is sunk: BLACK top-left (`:252`), and the
/// bottom-right line — nominally LWHITE (`:270`) — falls back to WHITE
/// because it would vanish into the LWHITE `actbutcol` face
/// (`draw_color == color`, `:271-272`).
#[test]
fn a85_selected_fl3dact_g_button_keeps_face_moves_text_sinks_bevel() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    static LABEL: &[u8] = b"X\0";
    let up = object(G_BUTTON, FL3DACT, 0, LABEL.as_ptr() as u64);
    let b = draw(&up);
    // Raised: glyph cell at (227, 99); grab it.
    let raised: Vec<String> = (99..99 + 16).map(|y| row(b, y, 227, 234)).collect();
    assert!(raised.iter().any(|r| r.contains('#')), "raised label should be black: {raised:?}");

    let down = object(G_BUTTON, FL3DACT, SELECTED, LABEL.as_ptr() as u64);
    let b = draw(&down);
    // Pressed: the same glyph one pixel right and down, still BLACK on LWHITE.
    let pressed: Vec<String> = (100..100 + 16).map(|y| row(b, y, 228, 235)).collect();
    assert_eq!(pressed, raised, "pressed label = raised label shifted by (+1, +1), same black ink");
    // The face is untouched actbutcol (LWHITE) beside the label.
    assert_eq!(row(b, 108, 220, 226), "l".repeat(7), "SELECTED FL3DACT face must stay actbutcol");
    // Sunk bevel: BLACK top-left, LWHITE bottom-right, ring intact.
    let rows: [(i16, &str); 4] = [
        (97, "##############################"),
        (98, "############################l#"),
        (117, "#l...........................#"),
        (118, "##############################"),
    ];
    expect_rows(b, 217, 246, &rows, "G_BUTTON (FL3DACT, SELECTED)");
    assert_eq!(class(b, 218, 108), '#', "sunk bevel: left line BLACK");
    assert_eq!(class(b, 245, 108), '.', "sunk bevel: right line WHITE (LWHITE would vanish into the face)");
}

// ── further cells where the matrix differs ────────────────────────────────

/// A `G_BOXTEXT` under `FL3DACT` is an activator too: `actcolchange = 0`, so
/// SELECTED keeps the `backgrcol` face and the BLACK text (`:681-690`,
/// `:699-706` both gated on `changecol`), only moves the text (`:750-756`,
/// `c.g_x++`) and sinks the bevel. Hollow-white REPLACE TEDINFO, `th = -1`,
/// text `X` left-justified.
#[test]
fn a85_selected_fl3dact_g_boxtext_keeps_face_and_black_text() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    static TEXT: &[u8] = b"X\0";
    static TMPL: &[u8] = b"\0";
    let t = object(G_BOXTEXT, FL3DACT, SELECTED, ted(TEXT, TMPL, 0, TE_HOLLOW_WHITE_REPLACE, -1));
    let b = draw(&t);
    // Interior beyond the glyph cell (grown rect x 218..245, glyph at 219..226).
    assert_eq!(row(b, 108, 230, 244), "l".repeat(15), "SELECTED FL3DACT G_BOXTEXT face must stay backgrcol");
    // The glyph cell: `gr_gtext` centres the 16-row cell in the grown
    // 20-row rect (`gr_just`, `gemgraf.c:567-589`) → y 100..115; x 218..228
    // covers the BLACK left bevel line (218), the nudged glyph (219..226)
    // and face. The bevel's bottom line at y = 117 is *outside* the scan on
    // purpose: it is WHITE — LWHITE would vanish into the LWHITE face
    // (`add_3d_effect`, `:270-272`), the same fallback the FL3DACT G_BUTTON
    // guard above pins — and is not text.
    let glyph: String = (100..116).map(|y| row(b, y, 218, 228)).collect();
    assert!(glyph.contains('#'), "the text must be drawn BLACK (changecol = 0): {glyph}");
    assert!(!glyph.contains('.'), "the text must not be xor'd to WHITE: {glyph}");
}

/// TeraDesk's SETMASK list rows (`FTYPE1..4`, `desktop.rsc` tree 9 objects
/// 6-9): non-3D `G_FTEXT`, `te_color = 0x1100` (TRANSPARENT), `th = 0`,
/// on a `FL3DBAK` parent box (LWHITE). EmuTOS: no promotion (no 3D flag),
/// no fill, text drawn BLACK, then — `changecol` FALSE (`0x1100` is OK) —
/// the whole rect is XOR-inverted (`:932-943`): a dark bar with the glyphs
/// in WHITE. The row must not be left as white glyphs on the untouched
/// LWHITE ground.
#[test]
fn a85_selected_non3d_g_ftext_is_xor_inverted_as_a_block() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    static TEXT: &[u8] = b"AB\0";
    static TMPL: &[u8] = b"__\0";
    let t = object(G_FTEXT, 0, SELECTED, ted(TEXT, TMPL, 0, TE_HOLLOW_WHITE_TRANS, 0));
    let b = draw_on(&t, Some(LWHITE));
    // Beyond the two glyph cells (220..235) the rect is the XOR'd ground.
    assert_eq!(row(b, 100, 236, 243), "L".repeat(8), "the row's ground must be XOR-inverted, got\n{}", cell(b));
    let glyphs: String = (100..116).map(|y| row(b, y, 220, 235)).collect();
    assert!(glyphs.contains('.'), "glyph pixels are XOR'd BLACK → WHITE: {glyphs}");
    assert!(!glyphs.contains('l'), "no untouched LWHITE ground inside the SELECTED row: {glyphs}");
    // Outside the object the ground is untouched.
    assert_eq!(row(b, 99, 220, 243), "l".repeat(24));
    assert_eq!(row(b, 116, 220, 243), "l".repeat(24));
}

/// The plain `G_TEXT` twin (same `case G_TEXT` arm, `:734-757`): SELECTED
/// non-3D → XOR block.
#[test]
fn a85_selected_non3d_g_text_is_xor_inverted_as_a_block() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    static TEXT: &[u8] = b"AB\0";
    static TMPL: &[u8] = b"\0";
    let t = object(G_TEXT, 0, SELECTED, ted(TEXT, TMPL, 0, TE_HOLLOW_WHITE_TRANS, 0));
    let b = draw_on(&t, Some(LWHITE));
    assert_eq!(row(b, 100, 236, 243), "L".repeat(8), "the row's ground must be XOR-inverted, got\n{}", cell(b));
}

/// A `G_STRING` under `FL3DBAK`, SELECTED, on the LWHITE dialog ground:
/// `xor_is_ok(G_STRING) = TRUE` (`:112-113`) → `changecol` FALSE; not
/// `FL3DOBJ` → the trailing XOR runs (`:936-940`): complemented ground,
/// WHITE glyphs. Never a black block with black text.
#[test]
fn a85_selected_fl3dbak_g_string_is_xor_inverted_not_black_on_black() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    static TEXT: &[u8] = b"AB\0";
    let t = object(G_STRING, FL3DBAK, SELECTED, TEXT.as_ptr() as u64);
    let b = draw_on(&t, Some(LWHITE));
    let c = cell(b);
    assert!(!c.contains('#'), "SELECTED FL3DBAK G_STRING must not contain BLACK (black-on-black):\n{c}");
    assert!(c.contains('.'), "the glyphs must be XOR'd to WHITE:\n{c}");
    assert_eq!(row(b, 100, 236, 243), "L".repeat(8), "the ground must be XOR-inverted:\n{c}");
}

/// A `G_STRING` under `FL3DIND`, SELECTED: `FL3DOBJ` → no trailing XOR
/// (`:936`); the string block draws it BLACK (`:807` is `G_BUTTON`-only) on
/// the untouched ground, grown by ADJ3DSTD and beveled (`:950`). The text
/// must stay legible: black glyphs on LWHITE, no black block.
#[test]
fn a85_selected_fl3dind_g_string_stays_black_on_the_ground() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    static TEXT: &[u8] = b"AB\0";
    let t = object(G_STRING, FL3DIND, SELECTED, TEXT.as_ptr() as u64);
    let b = draw_on(&t, Some(LWHITE));
    // Grown rect (218, 98, 28, 20); glyph cells 218..233 at y 100..115;
    // x = 236..243 is interior beyond the text: still LWHITE ground.
    assert_eq!(row(b, 108, 236, 243), "l".repeat(8), "no fill under a SELECTED FL3DIND G_STRING, got\n{}", cell(b));
    let glyphs: String = (100..116).map(|y| row(b, y, 218, 233)).collect();
    assert!(glyphs.contains('#'), "the glyphs are BLACK: {glyphs}");
    assert!(glyphs.contains('l'), "the glyph cells keep the ground between the strokes: {glyphs}");
}

/// `ob_sst` gives a `G_TITLE` `th = 1` (`gemobjop.c:44-46`), and the state
/// block insets `t` by a positive `th` before the XOR (`gemoblib.c:889-890`,
/// `:940`): the dropped menu title's inverted block is `(x+1, y+1, w-2,
/// h-2)` — a one-pixel margin of the title's own rect stays un-inverted
/// (the `ob_change` XOR toggle, `:1301-1302`, insets identically).
#[test]
fn a85_selected_g_title_xor_is_inset_by_th_one() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    static TEXT: &[u8] = b"AB\0";
    let t = object(G_TITLE, 0, SELECTED, TEXT.as_ptr() as u64);
    let b = draw_on(&t, Some(WHITE));
    let c = cell(b);
    assert_eq!(row(b, 100, 220, 243), ".".repeat(24), "top row of the title rect stays un-inverted:\n{c}");
    assert_eq!(row(b, 115, 220, 243), ".".repeat(24), "bottom row of the title rect stays un-inverted:\n{c}");
    assert_eq!(class(b, 220, 108), '.', "left column stays un-inverted:\n{c}");
    assert_eq!(class(b, 243, 108), '.', "right column stays un-inverted:\n{c}");
    // `t.g_x + 1 .. t.g_x + t.g_w - 2` = 221..242 is inverted; 243 is the
    // un-inverted right column again (`gr_inside(&t, 1)`, `:889-890`).
    assert_eq!(row(b, 101, 236, 243), "#######.", "inside the inset the ground is inverted:\n{c}");
}

/// A non-3D `G_IBOX` (`th = -1`), SELECTED, on the LWHITE ground:
/// `xor_is_ok(G_IBOX) = TRUE` (`:112-113`) → `changecol` FALSE; the border
/// is drawn (`:663-667`; `gr_box` with `th = -1` outlines the rect grown by
/// one *and* the rect itself, `gemgraf.c:813-831`), the interior is not
/// filled (`:670`), and the trailing block XORs the object rect
/// (`:932-943`): the ground inside becomes its complement and the inner
/// ring (on the rect's edge) is XOR'd BLACK → WHITE. Nothing is left alone.
#[test]
fn a85_selected_non3d_g_ibox_is_xor_inverted() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let t = object(G_IBOX, 0, SELECTED, 0x00ff_1100);
    let b = draw_on(&t, Some(LWHITE));
    let ring = format!("ll{}ll", "#".repeat(26));
    let edge = format!("ll#{}#ll", ".".repeat(24));
    let body = format!("ll#.{}.#ll", "L".repeat(22));
    let mut rows: Vec<(i16, String)> = vec![(98, "l".repeat(30)), (99, ring.clone()), (100, edge.clone())];
    rows.extend((101..=114).map(|y| (y, body.clone())));
    rows.extend([(115, edge), (116, ring), (117, "l".repeat(30))]);
    expect_owned(b, &rows, "G_IBOX (non-3D, SELECTED)");
}

/// A non-3D `G_BOX` with a black `IP_4PATT` dither (`0x1141`), `th = -1`,
/// SELECTED: `xor_is_ok` is TRUE (interior BLACK, text BLACK, `:140-147`),
/// so the dither is drawn (`gr_rect`, black on white) and then XOR'd
/// (`:940`): every set pixel WHITE, every clear pixel BLACK — the inverse
/// dither, not a solid white face.
#[test]
fn a85_selected_non3d_patterned_g_box_inverts_the_dither() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let plain = object(G_BOX, 0, 0, 0x00ff_1141);
    let b = draw(&plain);
    let before = row(b, 108, 220, 243);
    assert!(before.contains('#') && before.contains('.'), "a 50 % dither has both classes: {before}");

    let sel = object(G_BOX, 0, SELECTED, 0x00ff_1141);
    let b = draw(&sel);
    let after = row(b, 108, 220, 243);
    let inverse: String = before.chars().map(|c| if c == '#' { '.' } else { '#' }).collect();
    assert_eq!(after, inverse, "SELECTED dither is the XOR (inverse) of the unselected one");
}

// ── guard: a non-FL3DOBJ hollow-white button still reads inverted ─────────

/// A `G_BUTTON` under `FL3DBAK` (`th = -1`), SELECTED: hollow white
/// (`:652-657`), `xor_is_ok` `default:` → TRUE, trailing XOR → BLACK
/// interior, the label (drawn BLACK, `:806`) XOR'd to WHITE. This is the
/// look NewTOS's inline inversion produces too — the guard pins it.
#[test]
fn a85_selected_fl3dbak_g_button_reads_inverted() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    static LABEL: &[u8] = b"X\0";
    let t = object(G_BUTTON, FL3DBAK, SELECTED, LABEL.as_ptr() as u64);
    let b = draw(&t);
    assert_eq!(row(b, 100, 220, 227), "#".repeat(8), "interior BLACK beside the label");
    let glyph: String = (100..116).map(|y| row(b, y, 228, 235)).collect();
    assert!(glyph.contains('.'), "label WHITE on the inverted face: {glyph}");
    assert!(!glyph.contains('l') && !glyph.contains('d'), "no 3D face on a FL3DBAK button: {glyph}");
}
