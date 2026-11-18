//! A41 — the `G_CICON` draw arm: EmuTOS `just_draw` → `gr_gicon(state, ib,
//! cicon)` with a **colour** variant (`aes/gemgraf.c:710-807`,
//! `CONF_WITH_COLOUR_ICONS`), pixel for pixel.
//!
//! The Object Manager reads the CICONBLK behind a `G_CICON`'s `ob_spec` as
//! the loader left it after `fix_cicons` (`tos-api/src/aes/resource.rs`):
//! `mainlist` names the chosen `CICON`, whose `col_data`/`sel_data` are
//! **device-form** images — NewTOS's 16-bit RGB565 memory-MFDB form — and
//! whose `col_mask`/`sel_mask` are the file's one-plane masks. `gr_gicon`
//! then (numbers are `gemgraf.c` lines):
//!
//! 1. picks `sel_data`/`sel_mask` for a SELECTED icon that has them, else
//!    `col_data`/`col_mask` (`:729-741`);
//! 2. blits the mask in the icon's background pen (`gr_gblt(pmask, pi,
//!    bgcol, fgcol)`, `:757`) and, for a non-empty label, fills the label
//!    box in `fgcol` when SELECTED else `bgcol` (`:760-766`) — unless
//!    `WHITEBAK` is set and the background is white (`:755`);
//! 3. blits the image with `gr_colourblit` — a Memory→Screen `vro_cpyfm`
//!    with `S_AND_D` on the truecolor path (`:642-671`, `:772`);
//! 4. when SELECTED, swaps fg/bg for the label and, for a variant
//!    **without** a SELECTED image, darkens the icon by blitting a copy of
//!    the mask ANDed with `0x5555`/`0xAAAA` on alternate rows in BLACK
//!    (`dither_mask`, `:680-700`, `:778-786`);
//! 5. draws the label (`:806`).
//!
//! The backend below stores raw RGB565 words as its device pixels (the
//! 16↔32 conversions are the identity), so what lands on screen is exactly
//! the word the logic op produced. Spec: `docs/api/aes/object-form.md`
//! (`G_CICON` callout after the `just_draw` walkthrough);
//! `docs/api/aes/resource-fileselector.md` §5.

use std::sync::Mutex;

use tos_api::aes::object::{render_tree_lp64, Object};
use tos_api::vdi::backend::{ClipRect, MonoSrc, PatternFill, Pel, RasterBackend, Rect, WriteMode};

const SCREEN_W: u16 = 64;
const SCREEN_H: u16 = 48;

/// A device word no draw produces: bit 5 (the green LSB the palette never
/// sets) is clear, so `screen & WHITE == screen` and an untouched pixel is
/// distinguishable from every pen below.
const UNTOUCHED: u32 = 0x1084;

/// The pseudo-palette, RGB555-in-565 (`set_color16`: bit 5 always 0).
const PAL_WHITE: u16 = 0xFFDF; // pen 0
const PAL_BLACK: u16 = 0x0000; // pen 1
const PAL_RED: u16 = 0xF800; // pen 2
const PAL_GREEN: u16 = 0x07C0; // pen 3
const PAL_BLUE: u16 = 0x001F; // pen 4
const PALETTE: [u16; 5] = [PAL_WHITE, PAL_BLACK, PAL_RED, PAL_GREEN, PAL_BLUE];

fn pal(pel: Pel) -> u32 {
    PALETTE[pel as usize] as u32
}

struct Rgb565Backend {
    px: Mutex<Vec<u32>>,
}

impl Rgb565Backend {
    fn new() -> Self {
        Self { px: Mutex::new(vec![UNTOUCHED; SCREEN_W as usize * SCREEN_H as usize]) }
    }
    fn inside(x: i16, y: i16) -> bool {
        x >= 0 && y >= 0 && (x as i32) < SCREEN_W as i32 && (y as i32) < SCREEN_H as i32
    }
    fn put(&self, x: i16, y: i16, v: u32, clip: ClipRect) {
        if !Self::inside(x, y) || x < clip.xmn || x > clip.xmx || y < clip.ymn || y > clip.ymx {
            return;
        }
        self.px.lock().unwrap()[y as usize * SCREEN_W as usize + x as usize] = v;
    }
    fn get(&self, x: i16, y: i16) -> u32 {
        self.px.lock().unwrap()[y as usize * SCREEN_W as usize + x as usize]
    }
}

impl RasterBackend for Rgb565Backend {
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
    fn set_color(&self, _i: u16, _r: u16, _g: u16, _b: u16) {}
    fn query_color(&self, _i: u16) -> (u16, u16, u16) {
        (0, 0, 0)
    }
    fn set_active_palette(&self, _p: &[u32; 256]) {}
    fn pel_to_device16(&self, pel: Pel) -> u16 {
        pal(pel) as u16
    }
    fn device_to_rgb565(&self, argb: u32) -> u16 {
        argb as u16
    }
    fn rgb565_to_device(&self, px: u16) -> u32 {
        px as u32
    }
    fn read_pixel(&self, x: i16, y: i16) -> Pel {
        let v = self.get(x, y);
        PALETTE.iter().position(|&p| p as u32 == v).unwrap_or(0) as Pel
    }
    fn fill_rect(&self, x: i16, y: i16, w: i16, h: i16, pel: Pel, clip: ClipRect) {
        for yy in y..y.saturating_add(h) {
            for xx in x..x.saturating_add(w) {
                self.put(xx, yy, pal(pel), clip);
            }
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn abline(
        &self,
        x0: i16,
        y0: i16,
        x1: i16,
        y1: i16,
        pel: Pel,
        _mode: WriteMode,
        clip: ClipRect,
        mask: u16,
        _last: bool,
    ) -> u16 {
        for yy in y0.min(y1)..=y0.max(y1) {
            for xx in x0.min(x1)..=x0.max(x1) {
                self.put(xx, yy, pal(pel), clip);
            }
        }
        mask
    }
    fn fill_pattern(&self, x: i16, y: i16, w: i16, h: i16, fill: PatternFill, clip: ClipRect) {
        for yy in y..y.saturating_add(h) {
            let row = fill.pattern[(yy.rem_euclid(fill.pattern.len() as i16)) as usize];
            for xx in x..x.saturating_add(w) {
                let set = row & (0x8000u16 >> (xx.rem_euclid(16) as u16)) != 0;
                if set {
                    self.put(xx, yy, pal(fill.fg), clip);
                } else if fill.mode == WriteMode::Replace {
                    self.put(xx, yy, pal(fill.bg), clip);
                }
            }
        }
    }
    #[allow(clippy::too_many_arguments)]
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
        let bpr = (cell_w as usize).div_ceil(8);
        for row in 0..cell_h {
            for col in 0..cell_w {
                let byte = glyph[row as usize * bpr + col as usize / 8];
                if byte & (0x80u8 >> (col % 8)) != 0 {
                    self.put(x + col, y + row, pal(fg), clip);
                }
            }
        }
    }
    fn blit_mono(&self, dst: Rect, src: MonoSrc, fg: Pel, bg: Pel, mode: WriteMode, clip: ClipRect) {
        for row in 0..dst.h {
            for col in 0..dst.w {
                let (sx, sy) = (src.x + col, src.y + row);
                let wi = (sy as usize) * src.stride_words as usize + (sx as usize) / 16;
                let w = src.bits.word(wi).unwrap_or(0);
                let set = w & (0x8000u16 >> (sx.rem_euclid(16)) as u16) != 0;
                if set {
                    self.put(dst.x + col, dst.y + row, pal(fg), clip);
                } else if mode != WriteMode::Transparent {
                    self.put(dst.x + col, dst.y + row, pal(bg), clip);
                }
            }
        }
    }
    fn copy_rect(&self, _src: Rect, _dx: i16, _dy: i16, _op: u8, _clip: ClipRect) {}
    fn get_device_pixel(&self, x: i16, y: i16) -> u32 {
        if Self::inside(x, y) {
            self.get(x, y)
        } else {
            0
        }
    }
    fn put_device_pixel(&self, x: i16, y: i16, argb: u32) {
        self.put(x, y, argb, ClipRect::full(SCREEN_W, SCREEN_H));
    }
    fn flush_rect(&self, _x0: i16, _y0: i16, _x1: i16, _y1: i16) {}
}

// SAFETY: the only mutable state is behind a Mutex.
unsafe impl Sync for Rgb565Backend {}
unsafe impl Send for Rgb565Backend {}

// ── the fixture: one 16×16 colour icon ────────────────────────────────────

const NIL: i16 = -1;
const ROOT: i16 = 0;
const G_IBOX: u16 = 25;
const G_CICON: u16 = 33;
const SELECTED: u16 = 0x0001;
const WHITEBAK: u16 = 0x0040;

const W: usize = 16;
const H: usize = 16;
/// Where the object (and, with `ib_xicon = ib_yicon = 0`, the icon) sits.
const OX: i16 = 8;
const OY: i16 = 8;

/// The label box: below the icon, 3 × 8 px cells wide, 8 tall.
const TX: i16 = 0;
const TY: i16 = H as i16;
const TW: i16 = 24;
const TH: i16 = 8;

/// Mask: rows 0..8 set, rows 8..16 clear.
fn mask_words() -> [u16; H] {
    let mut m = [0u16; H];
    for (r, w) in m.iter_mut().enumerate() {
        *w = if r < 8 { 0xFFFF } else { 0 };
    }
    m
}

/// The device-form image: a red/blue checkerboard where the mask is set,
/// **white** where it is clear (what `expand_cicondata`'s masking +
/// `palette[0]` leave there — the AND-transparent value).
fn image_words(mask: &[u16; H]) -> [u16; W * H] {
    let mut img = [PAL_WHITE; W * H];
    for r in 0..H {
        for c in 0..W {
            if mask[r] & (0x8000 >> c) != 0 {
                img[r * W + c] = if (r + c) % 2 == 0 { PAL_RED } else { PAL_BLUE };
            }
        }
    }
    img
}

/// Native LP64 `CICONBLK` (56 bytes) followed by one `CICON` (48 bytes).
#[repr(C, align(8))]
struct CiconFixture {
    // ICONBLK head
    ib_pmask: u64,
    ib_pdata: u64,
    ib_ptext: u64,
    words: [i16; 11],
    _pad: u16,
    mainlist: u64,
    // CICON
    num_planes: i16,
    _pad2: [u16; 3],
    col_data: u64,
    col_mask: u64,
    sel_data: u64,
    sel_mask: u64,
    next_res: u64,
}

struct Scene {
    _mask: Box<[u16; H]>,
    _img: Box<[u16; W * H]>,
    _sel: Option<Box<[u16; W * H]>>,
    _label: Box<[u8; 8]>,
    fixture: Box<CiconFixture>,
}

impl Scene {
    /// `ib_char` = fg pen 1 (black), bg pen 0 (white), no overlay character.
    fn new(with_sel: bool, label: &[u8]) -> Scene {
        let mask = Box::new(mask_words());
        let img = Box::new(image_words(&mask));
        // The SELECTED image: green inside the mask, white outside (as the
        // loader's expansion leaves every device-form image).
        let sel = with_sel.then(|| {
            let mut g = Box::new([PAL_WHITE; W * H]);
            for (i, px) in g.iter_mut().enumerate() {
                if mask[i / W] & (0x8000 >> (i % W)) != 0 {
                    *px = PAL_GREEN;
                }
            }
            g
        });
        let mut lbl = Box::new([0u8; 8]);
        lbl[..label.len()].copy_from_slice(label);
        let fixture = Box::new(CiconFixture {
            // The mono head is not drawn when a variant exists; still, give
            // it the same mask so an accidental mono draw is visible as a
            // different colour (black data), not as nothing.
            ib_pmask: mask.as_ptr() as u64,
            ib_pdata: mask.as_ptr() as u64,
            ib_ptext: lbl.as_ptr() as u64,
            words: [0x1000, 0, 0, 0, 0, W as i16, H as i16, TX, TY, TW, TH],
            _pad: 0,
            mainlist: 0, // patched below (self-referential)
            num_planes: 16,
            _pad2: [0; 3],
            col_data: img.as_ptr() as u64,
            col_mask: mask.as_ptr() as u64,
            sel_data: sel.as_ref().map_or(0, |s| s.as_ptr() as u64),
            sel_mask: if with_sel { mask.as_ptr() as u64 } else { 0 },
            next_res: 0,
        });
        let mut s = Scene { _mask: mask, _img: img, _sel: sel, _label: lbl, fixture };
        let cicon_addr = &s.fixture.num_planes as *const i16 as u64;
        s.fixture.mainlist = cicon_addr;
        s
    }

    fn render(&self, state: u16) -> Rgb565Backend {
        let tree = [
            Object {
                ob_next: NIL,
                ob_head: 1,
                ob_tail: 1,
                ob_type: G_IBOX,
                ob_flags: 0,
                ob_state: 0,
                // An invisible box without a border: the root paints
                // nothing, so the screen stays `UNTOUCHED` around the icon.
                ob_spec: 0,
                ob_x: 0,
                ob_y: 0,
                ob_width: SCREEN_W as i16,
                ob_height: SCREEN_H as i16,
            },
            Object {
                ob_next: ROOT,
                ob_head: NIL,
                ob_tail: NIL,
                ob_type: G_CICON,
                ob_flags: 0,
                ob_state: state,
                ob_spec: &*self.fixture as *const CiconFixture as u64,
                ob_x: OX,
                ob_y: OY,
                ob_width: W as i16,
                ob_height: (H as i16) + TH,
            },
        ];
        let fb = Rgb565Backend::new();
        // SAFETY: `tree` is a live native (LP64) OBJECT array; the fixture
        // and its rasters outlive the call.
        unsafe { render_tree_lp64(tree.as_ptr(), ROOT, 8, 0, 0, 0, 0, &fb) };
        fb
    }
}

fn icon_px(fb: &Rgb565Backend, c: usize, r: usize) -> u32 {
    fb.get(OX + c as i16, OY + r as i16)
}

// ── the tests ─────────────────────────────────────────────────────────────

/// Steps 2–3: mask in `bgcol` (white), then the image AND-ed over it — the
/// checkerboard where the mask is set, the screen untouched where it is not.
#[test]
fn a_colour_icon_draws_its_device_form_image_through_the_mask() {
    let scene = Scene::new(false, b"");
    let fb = scene.render(0);
    let mask = mask_words();
    let img = image_words(&mask);
    for r in 0..H {
        for c in 0..W {
            let got = icon_px(&fb, c, r);
            if r < 8 {
                assert_eq!(got, img[r * W + c] as u32, "pixel ({c},{r}) inside the mask");
            } else {
                assert_eq!(got, UNTOUCHED, "pixel ({c},{r}) outside the mask must be untouched");
            }
        }
    }
}

/// Step 1: a SELECTED icon with a SELECTED image draws that image (green),
/// and — having one — is **not** dither-darkened.
#[test]
fn a_selected_icon_with_sel_data_draws_the_selected_image_undarkened() {
    let scene = Scene::new(true, b"");
    let fb = scene.render(SELECTED);
    for r in 0..8 {
        for c in 0..W {
            assert_eq!(icon_px(&fb, c, r), PAL_GREEN as u32, "pixel ({c},{r})");
        }
    }
    for r in 8..H {
        for c in 0..W {
            assert_eq!(icon_px(&fb, c, r), UNTOUCHED, "pixel ({c},{r})");
        }
    }
}

/// Step 4: a SELECTED icon **without** a SELECTED image is darkened with the
/// dithered mask — `0x5555` on even rows, `0xAAAA` on odd rows (`dither_mask`,
/// `gemgraf.c:694`), blitted in BLACK, transparent: every second pixel inside
/// the mask turns black, the others keep the image, and nothing outside the
/// mask is touched.
#[test]
fn a_selected_icon_without_sel_data_is_dither_darkened() {
    let scene = Scene::new(false, b"");
    let fb = scene.render(SELECTED);
    let mask = mask_words();
    let img = image_words(&mask);
    for r in 0..H {
        let dither: u16 = if r & 1 != 0 { 0xAAAA } else { 0x5555 };
        for c in 0..W {
            let got = icon_px(&fb, c, r);
            let want = if r >= 8 {
                UNTOUCHED
            } else if dither & (0x8000 >> c) != 0 {
                PAL_BLACK as u32
            } else {
                img[r * W + c] as u32
            };
            assert_eq!(got, want, "pixel ({c},{r})");
        }
    }
}

/// Step 2: the label box is filled in `bgcol` (white) for a plain icon and
/// in `fgcol` (black) for a SELECTED one — `gr_rect(cicon && SELECTED ?
/// fgcol : bgcol, IP_SOLID, pt)` (`:762-765`) — before the label is drawn.
/// The box corner is a pixel no glyph of a centred one-letter label reaches.
#[test]
fn the_label_box_follows_the_selected_state() {
    let plain = Scene::new(true, b"A").render(0);
    assert_eq!(plain.get(OX + TX, OY + TY), PAL_WHITE as u32, "plain: label box in bgcol");
    let selected = Scene::new(true, b"A").render(SELECTED);
    assert_eq!(selected.get(OX + TX, OY + TY), PAL_BLACK as u32, "selected: label box in fgcol");
}

/// `WHITEBAK` with a white background skips the mask blit and the label box
/// (`:755`): the image is still AND-ed onto the untouched screen — where the
/// mask is set the screen word is `UNTOUCHED & image`, where it is clear
/// `UNTOUCHED & white` = `UNTOUCHED` — and the label box stays untouched.
#[test]
fn whitebak_on_a_white_background_skips_the_mask_and_the_label_box() {
    let scene = Scene::new(false, b"A");
    let fb = scene.render(WHITEBAK);
    let mask = mask_words();
    let img = image_words(&mask);
    for r in 0..H {
        for c in 0..W {
            let want = if r < 8 { UNTOUCHED & img[r * W + c] as u32 } else { UNTOUCHED };
            assert_eq!(icon_px(&fb, c, r), want, "pixel ({c},{r})");
        }
    }
    assert_eq!(fb.get(OX + TX, OY + TY), UNTOUCHED, "label box corner untouched");
}
