//! A68 / A69 / A70 — three neighbours in `just_draw`'s 3D handling
//! (`docs/api/aes/object-form.md`, "3D handling in `just_draw`" (a)/(b)/(c),
//! D-103).
//!
//! * **A68** — `th` for the text family comes from the one `ob_sst()` call
//!   (`aes/gemoblib.c:489`), whose single fold `if (th > 128) th -= 256`
//!   (`aes/gemobjop.c:68-69`) turns a `te_thickness` of `255` into `-1`: a
//!   1-px border *outside* the rect, interior untouched.
//! * **A69** — the hollow-white → `backgrcol` override is unconditional
//!   (`gemoblib.c:582-585`), but the `G_TEXT`→`G_BOXTEXT` promotion with
//!   `th = 0` runs only `if (tmode == MD_REPLACE)` (`:586-599`). A
//!   TRANSPARENT `G_TEXT` therefore stays an unboxed text (the second type
//!   switch has no `case G_TEXT`, `:609-696`) and the background beside its
//!   glyphs stays visible; a TRANSPARENT `G_BOXTEXT` is filled `backgrcol`
//!   and keeps its own border.
//! * **A70** — `case G_BUTTON:` (`gemoblib.c:639-658`): `bcol = BLACK`;
//!   `if (flags & FL3DOBJ)` → solid `actbutcol`/`indbutcol`, **else**
//!   (`FL3DBAK` or no 3D) → `IP_HOLLOW`, `WHITE`. `FL3DBAK` (`0x0400`) does
//!   not carry the `FL3DOBJ` bit (`0x0200`, `obdefs.h:92,97`).
//!
//! # The profile
//!
//! 640×480 Falcon truecolor, pseudo-palette seeded like `init_wk` does
//! (`aes_object_3d_box_border.rs`): pens 0/1/2/8/9 resolve to five distinct
//! device words (`.` white, `#` black, `r` red, `l` LWHITE, `d` LBLACK); `_`
//! is the slate the screen is cleared to — "nothing drew here". Every object
//! sits at `(220, 100, 24, 16)`, the FSELECTR drive-A slot of the sibling
//! test, so the row strings compare one-to-one with that file's reference
//! rows over `x = 217..=246`.

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
const G_BUTTON: u16 = 26;
const LASTOB: u16 = 0x0020;
const FL3DIND: u16 = 0x0200;
const FL3DBAK: u16 = 0x0400;
const FL3DACT: u16 = 0x0600;
const NIL: i16 = -1;

// `te_color` words (`gr_crack`): border 1 (black), text 1 (black), inside
// pattern 0 (hollow), inside colour 0 (white); bit 7 = REPLACE.
const TE_HOLLOW_WHITE_REPLACE: i16 = 0x1180;
const TE_HOLLOW_WHITE_TRANS: i16 = 0x1100;

/// A device value no palette pen produces: "nothing drew here".
const SLATE: u32 = 0x0012_3456;

// The object slot shared with `aes_object_3d_box_border.rs`.
const OX: i16 = 220;
const OY: i16 = 100;
const OW: i16 = 24;
const OH: i16 = 16;

fn object(ob_type: u16, flags: u16, spec: u64) -> [Object; 1] {
    [Object {
        ob_next: NIL,
        ob_head: NIL,
        ob_tail: NIL,
        ob_type,
        ob_flags: flags | LASTOB,
        ob_state: 0,
        ob_spec: spec,
        ob_x: OX,
        ob_y: OY,
        ob_width: OW,
        ob_height: OH,
    }]
}

/// A leaked TEDINFO: IBM font, `just`, `color`, `thickness`, `text`.
fn ted(text: &'static [u8], just: i16, color: i16, thickness: i16) -> u64 {
    static EMPTY: &[u8] = b"\0";
    let t = Box::leak(Box::new(TedInfo {
        te_ptext: text.as_ptr() as u64,
        te_ptmplt: EMPTY.as_ptr() as u64,
        te_pvalid: EMPTY.as_ptr() as u64,
        te_font: 3,
        te_junk1: 0,
        te_just: just,
        te_color: color,
        te_junk2: 0,
        te_thickness: thickness,
        te_txtlen: text.len() as i16,
        te_tmplen: 1,
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

const ACTBUTCOL: i16 = 4;
const RED: i16 = 2;
const LWHITE: i16 = 8;

/// Draw `t` on a slate screen with the cursor out of the way, hand back the
/// backend.
fn draw(t: &[Object; 1]) -> &'static BufBackend {
    let h = rig();
    let b = &h.raster;
    let mut pb = Aespb::new();
    mouse::show_reset(b);
    mouse::remove(b);
    b.px.lock().unwrap().fill(SLATE);
    objc_draw(&mut pb, t);
    b
}

/// One screen row as a class string over `x0..=x1`.
fn row(b: &BufBackend, y: i16, x0: i16, x1: i16) -> String {
    (x0..=x1)
        .map(|x| {
            if b.get_device_pixel(x, y) == SLATE {
                return '_';
            }
            match b.read_pixel(x, y) {
                0 => '.',
                1 => '#',
                2 => 'r',
                8 => 'l',
                9 => 'd',
                _ => '?',
            }
        })
        .collect()
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
/// interior of class `face`, over `x = 217..=246`.
fn outside_border_rows(face: char) -> Vec<(i16, String)> {
    let ring = format!("__{}__", "#".repeat(26));
    let body = format!("__#{}#__", face.to_string().repeat(24));
    let mut rows = vec![(98, "_".repeat(30)), (99, ring.clone())];
    rows.extend((100..=115).map(|y| (y, body.clone())));
    rows.push((116, ring));
    rows.push((117, "_".repeat(30)));
    rows
}

fn expect_owned(b: &BufBackend, rows: &[(i16, String)], what: &str) {
    let borrowed: Vec<(i16, &str)> = rows.iter().map(|(y, s)| (*y, s.as_str())).collect();
    expect_rows(b, 217, 246, &borrowed, what);
}

// ── A68: te_thickness = 255 folds to -1 ───────────────────────────────────

/// A plain (non-3D) `G_BOXTEXT` whose TEDINFO carries `te_thickness = 255`
/// — the raw word a classic `.rsc` stores for `-1`, passed through unchanged
/// by `tools/mkrsc.py`. `ob_sst`'s fold (`gemobjop.c:68-69`) makes it `-1`:
/// one black pixel *outside* the rect, the rect itself hollow white. Without
/// the fold the text branch draws a 255-px inside border that swallows the
/// interior and spills far beyond the object.
#[test]
fn a68_boxtext_te_thickness_255_is_a_one_pixel_outside_border() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    static TEXT: &[u8] = b"\0";
    let t = object(G_BOXTEXT, 0, ted(TEXT, 2, TE_HOLLOW_WHITE_REPLACE, 255));
    let b = draw(&t);
    expect_owned(b, &outside_border_rows('.'), "G_BOXTEXT (te_thickness = 255)");
    // Nothing spills past the ring — a 255-px inside border would paint
    // black down to y = 354 and out to x = 474.
    assert_eq!(row(b, 130, 217, 246), "_".repeat(30), "30 rows below the object");
    assert_eq!(row(b, 108, 250, 260), "_".repeat(11), "right of the object");
}

/// The same fold under `FL3DIND`: `te_thickness = 255` must render
/// byte-identically to `te_thickness = -1` — the reference rows are those of
/// `aes_object_3d_box_border.rs::fl3dind_boxtext_with_outside_border_keeps_it_under_the_bevel`.
/// `effect_th` (the bevel's inset, `gemoblib.c:498`) is the folded value too:
/// an unfolded `255` would shrink the bevel to nothing.
#[test]
fn a68_fl3dind_boxtext_te_thickness_255_draws_like_minus_one() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    static TEXT: &[u8] = b"\0";
    let t = object(G_BOXTEXT, FL3DIND, ted(TEXT, 2, TE_HOLLOW_WHITE_REPLACE, 255));
    let b = draw(&t);
    #[rustfmt::skip]
    let rows: [(i16, &str); 6] = [
        (97,  "##############################"),
        (98,  "#...........................l#"),
        (99,  "#.lllllllllllllllllllllllllld#"),
        (116, "#.lllllllllllllllllllllllllld#"),
        (117, "#lddddddddddddddddddddddddddd#"),
        (118, "##############################"),
    ];
    expect_rows(b, 217, 246, &rows, "G_BOXTEXT (FL3DIND, te_thickness = 255)");
}

// ── A69: the promotion is MD_REPLACE-gated, the colour override is not ────

/// A TRANSPARENT `G_TEXT` (`te_color` bit 7 clear) under `FL3DBAK`, one
/// glyph `X` left-justified: EmuTOS leaves `obtype == G_TEXT`
/// (`gemoblib.c:586` is false), so no box is drawn — only the glyph, and the
/// slate beside it (columns 228..243 of the object) stays untouched. The
/// promotion-and-fill would paint those columns `backgrcol`.
#[test]
fn a69_transparent_fl3dbak_g_text_stays_unboxed_over_the_background() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    static TEXT: &[u8] = b"X\0";
    let t = object(G_TEXT, FL3DBAK, ted(TEXT, 0, TE_HOLLOW_WHITE_TRANS, 0));
    let b = draw(&t);
    // The glyph cell is (220..227, 100..115); everything else in the rect
    // and its 3-px margin is still slate.
    let mut rows: Vec<(i16, String)> = vec![(99, "_".repeat(30))];
    rows.extend((100..=115).map(|y| (y, format!("___{}{}___", "*".repeat(8), "_".repeat(16)))));
    rows.push((116, "_".repeat(30)));
    expect_owned(b, &rows, "G_TEXT (FL3DBAK, TRANSPARENT)");
    // The glyph itself did draw, in the text colour.
    let inked = (100..=115).map(|y| row(b, y, 220, 227)).collect::<String>();
    assert!(inked.contains('#'), "the glyph should have been drawn in black: {inked}");
    assert!(!inked.contains('l'), "the glyph cell must not be filled backgrcol: {inked}");
}

/// The `MD_REPLACE` counterpart: the same `G_TEXT` with bit 7 set is promoted
/// to `G_BOXTEXT` with `th = 0` (`gemoblib.c:589-593`) and its rect filled
/// solid `backgrcol` — `te_thickness = -1` proves the zeroing: no black ring.
#[test]
fn a69_replace_fl3dbak_g_text_is_promoted_and_filled_backgrcol_without_border() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    static TEXT: &[u8] = b"\0";
    let t = object(G_TEXT, FL3DBAK, ted(TEXT, 2, TE_HOLLOW_WHITE_REPLACE, -1));
    let b = draw(&t);
    let body = format!("___{}___", "l".repeat(24));
    let mut rows: Vec<(i16, String)> = vec![(99, "_".repeat(30))];
    rows.extend((100..=115).map(|y| (y, body.clone())));
    rows.push((116, "_".repeat(30)));
    expect_owned(b, &rows, "G_TEXT (FL3DBAK, REPLACE, te_thickness = -1)");
}

/// A TRANSPARENT `G_BOXTEXT` under `FL3DBAK`: the colour override still fires
/// (`gemoblib.c:582-585`), the gate does not — `th` keeps `ob_sst`'s `-1`, so
/// the box is filled `backgrcol` *and* keeps its black outside ring.
#[test]
fn a69_transparent_fl3dbak_g_boxtext_is_filled_backgrcol_and_keeps_its_border() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    static TEXT: &[u8] = b"\0";
    let t = object(G_BOXTEXT, FL3DBAK, ted(TEXT, 2, TE_HOLLOW_WHITE_TRANS, -1));
    let b = draw(&t);
    expect_owned(b, &outside_border_rows('l'), "G_BOXTEXT (FL3DBAK, TRANSPARENT, th = -1)");
}

// ── A70: a FL3DBAK G_BUTTON stays hollow white ────────────────────────────

/// An un-selected `G_BUTTON` (no EXIT/DEFAULT: `th = -1`) with `FL3DBAK`:
/// `flags & FL3DOBJ` is false, so `gr_rect(WHITE, IP_HOLLOW)` — hollow white
/// interior inside a 1-px black ring, no grow, no bevel (`gemoblib.c:652-657`).
#[test]
fn a70_fl3dbak_g_button_stays_hollow_white_with_a_black_border() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    static LABEL: &[u8] = b"\0";
    let t = object(G_BUTTON, FL3DBAK, LABEL.as_ptr() as u64);
    let b = draw(&t);
    expect_owned(b, &outside_border_rows('.'), "G_BUTTON (FL3DBAK)");
}

/// Guard: a `FL3DACT` `G_BUTTON` takes `actbutcol` (`gemoblib.c:647-650`).
/// `actbutcol` is set to RED through `objc_sysvar` so it is tellable from
/// `backgrcol` (both default to LWHITE), and restored afterwards. Grown by
/// `ADJ3DSTD`, ring at `217/246 × 97/118`, WHITE top-left bevel, LBLACK
/// bottom-right bevel, the two short-stopped corners show the face.
#[test]
fn a70_fl3dact_g_button_takes_actbutcol() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    static LABEL: &[u8] = b"\0";
    let mut pb = Aespb::new();
    let _ = rig();
    sysvar_set(&mut pb, ACTBUTCOL, RED);
    let t = object(G_BUTTON, FL3DACT, LABEL.as_ptr() as u64);
    let b = draw(&t);
    sysvar_set(&mut pb, ACTBUTCOL, LWHITE);
    const FACE: &str = "#.rrrrrrrrrrrrrrrrrrrrrrrrrrd#";
    let mut rows: Vec<(i16, &str)> = vec![
        (97, "##############################"),
        (98, "#...........................r#"),
    ];
    rows.extend((99..=116).map(|y| (y, FACE)));
    rows.extend([
        (117, "#rddddddddddddddddddddddddddd#"),
        (118, "##############################"),
    ]);
    expect_rows(b, 217, 246, &rows, "G_BUTTON (FL3DACT, actbutcol = RED)");
}

/// Guard: a hollow-white `G_BOX` under `FL3DBAK` keeps taking `backgrcol`
/// (`gemoblib.c:631-634`) — A70 narrows the override for `G_BUTTON` only.
#[test]
fn a70_fl3dbak_g_box_still_takes_backgrcol() {
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    // thickness byte 0xff (-1) << 16 | colour word: black border, hollow white.
    let t = object(G_BOX, FL3DBAK, 0x00ff_1100);
    let b = draw(&t);
    expect_owned(b, &outside_border_rows('l'), "G_BOX (FL3DBAK, th = -1)");
}
