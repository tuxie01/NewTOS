//! B2b — a raster pointer with odd alignment must still draw.
//!
//! `G_ICON`/`G_IMAGE` rasters reach the Object Manager as bare pointers out of
//! a resource: `ib_pmask`/`ib_pdata` (`aes/object.rs` `draw_icon`) and
//! `bi_pdata` (`blit_image`). Nothing on that path can promise the pointer is
//! even:
//!
//! * a classic Atari `.rsc` keeps its rasters in the separate, word-aligned
//!   `rsh_imdata` section, so EmuTOS never meets an odd one (`gr_gblt` takes a
//!   plain `WORD *`, `aes/gemgraf.c:631`, and checks nothing — on a 68000 an
//!   odd word access is a fatal address error, on the Falcon's 68030 it simply
//!   works). There is no EmuTOS behaviour to reproduce here;
//! * a native `.rsc` produced by `tools/mkrsc.py` interns strings and rasters
//!   into ONE data section, so an odd-length string pushes the next raster onto
//!   an odd offset (B2a — fixed in the tool, but only for files WE build);
//! * TeraDesk relocates `icons.rsc` with its **own** loader
//!   (`sdk/apps/teradesk/rsc_load.c` `xrsrc_load`), so those pointers never pass
//!   through our `rsrc_load`; the same holds for any `MFDB` an app hands to
//!   `vrt_cpyfm`. The renderer is the only place that sees every raster.
//!
//! So the Object Manager must read an odd raster instead of trusting an
//! alignment it cannot enforce. Before the fix `blit_plane` built a `&[u16]`
//! with `slice::from_raw_parts`, whose alignment precondition made a debug
//! build abort ("unsafe precondition(s) violated") and a release build perform
//! the access anyway (`ub_checks` compiled out) — the same input ending two
//! different ways, with the release build committing UB.
//!
//! Each test draws the same icon/image twice, once from an even and once from
//! an odd address, and compares the two framebuffers pixel for pixel.

use std::sync::{Mutex, MutexGuard};

use tos_api::aes::object::{render_tree_lp64, BitBlk, IconBlk, Object};
use tos_api::vdi::backend::{ClipRect, MonoSrc, PatternFill, Pel, RasterBackend, Rect, WriteMode};

const SCREEN_W: u16 = 128;
const SCREEN_H: u16 = 96;
/// Pen no drawing operation writes, so "untouched" is distinguishable.
const UNTOUCHED: Pel = 15;

struct PenBackend {
    px: Mutex<Vec<Pel>>,
}

impl PenBackend {
    fn new() -> PenBackend {
        PenBackend {
            px: Mutex::new(vec![UNTOUCHED; SCREEN_W as usize * SCREEN_H as usize]),
        }
    }

    fn inside(x: i16, y: i16) -> bool {
        x >= 0 && y >= 0 && (x as i32) < SCREEN_W as i32 && (y as i32) < SCREEN_H as i32
    }

    fn put(&self, x: i16, y: i16, pel: Pel, clip: ClipRect) {
        if !Self::inside(x, y) {
            return;
        }
        if x < clip.xmn || x > clip.xmx || y < clip.ymn || y > clip.ymx {
            return;
        }
        self.px.lock().unwrap()[y as usize * SCREEN_W as usize + x as usize] = pel;
    }

    fn get(&self, x: i16, y: i16) -> Pel {
        self.px.lock().unwrap()[y as usize * SCREEN_W as usize + x as usize]
    }

    fn snapshot(&self) -> Vec<Pel> {
        self.px.lock().unwrap().clone()
    }

    fn count(&self, pen: Pel) -> usize {
        self.px
            .lock()
            .unwrap()
            .iter()
            .filter(|p| **p == pen)
            .count()
    }
}

impl RasterBackend for PenBackend {
    fn width(&self) -> u16 {
        SCREEN_W
    }
    fn height(&self) -> u16 {
        SCREEN_H
    }
    fn num_colors(&self) -> u16 {
        16
    }
    fn planes(&self) -> u16 {
        4
    }
    fn set_color(&self, _index: u16, _r: u16, _g: u16, _b: u16) {}
    fn query_color(&self, _index: u16) -> (u16, u16, u16) {
        (0, 0, 0)
    }
    fn set_active_palette(&self, _palette: &[u32; 256]) {}
    fn pel_to_device16(&self, pel: Pel) -> u16 {
        pel as u16
    }
    fn device_to_rgb565(&self, argb: u32) -> u16 {
        argb as u16
    }
    fn rgb565_to_device(&self, px: u16) -> u32 {
        px as u32
    }
    fn read_pixel(&self, x: i16, y: i16) -> Pel {
        self.get(x, y)
    }
    fn fill_rect(&self, x: i16, y: i16, w: i16, h: i16, pel: Pel, clip: ClipRect) {
        for yy in y..y.saturating_add(h) {
            for xx in x..x.saturating_add(w) {
                self.put(xx, yy, pel, clip);
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
        _mode: WriteMode,
        clip: ClipRect,
        mask: u16,
        _last: bool,
    ) -> u16 {
        // Straight-line only; the object renderer draws nothing else here.
        for yy in y0.min(y1)..=y0.max(y1) {
            for xx in x0.min(x1)..=x0.max(x1) {
                self.put(xx, yy, pel, clip);
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
                    self.put(xx, yy, fill.fg, clip);
                } else if fill.mode == WriteMode::Replace {
                    self.put(xx, yy, fill.bg, clip);
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
        assert!(
            cell_w <= 8,
            "these tests only use single-byte-per-row cells"
        );
        for row in 0..cell_h {
            let byte = glyph[row as usize];
            for col in 0..cell_w {
                if byte & (0x80u8 >> col) != 0 {
                    self.put(x + col, y + row, fg, clip);
                }
            }
        }
    }
    fn blit_mono(
        &self,
        dst: Rect,
        src: MonoSrc,
        fg: Pel,
        bg: Pel,
        mode: WriteMode,
        clip: ClipRect,
    ) {
        for row in 0..dst.h {
            for col in 0..dst.w {
                let (sx, sy) = (src.x + col, src.y + row);
                let wi = (sy as usize) * src.stride_words as usize + (sx as usize) / 16;
                let w = src.bits.word(wi).unwrap_or(0);
                let set = w & (0x8000u16 >> (sx.rem_euclid(16)) as u16) != 0;
                if set {
                    self.put(dst.x + col, dst.y + row, fg, clip);
                } else if mode != WriteMode::Transparent {
                    self.put(dst.x + col, dst.y + row, bg, clip);
                }
            }
        }
    }
    fn copy_rect(&self, _src: Rect, _dx: i16, _dy: i16, _op: u8, _clip: ClipRect) {}
    fn get_device_pixel(&self, x: i16, y: i16) -> u32 {
        self.get(x, y) as u32
    }
    fn put_device_pixel(&self, x: i16, y: i16, argb: u32) {
        self.put(x, y, argb as Pel, ClipRect::full(SCREEN_W, SCREEN_H));
    }
    fn flush_rect(&self, _x0: i16, _y0: i16, _x1: i16, _y1: i16) {}
}

// SAFETY: the only mutable state is behind a Mutex.
unsafe impl Sync for PenBackend {}
unsafe impl Send for PenBackend {}

// ── raster fixtures ───────────────────────────────────────────────────────

const NIL: i16 = -1;
const ROOT: i16 = 0;
const MAX_DEPTH: i16 = 8;

const G_BOX: u16 = 20;
const G_IMAGE: u16 = 23;
const G_ICON: u16 = 31;

/// A 16×16 raster (one word per row) with a recognisable, asymmetric bit
/// pattern: the two bytes of every word differ, so reading the raster one byte
/// off — or with the word halves swapped — changes the drawn pixels.
const ROWS: usize = 16;
fn raster_bytes() -> Vec<u8> {
    let mut v = Vec::with_capacity(ROWS * 2);
    for r in 0..ROWS {
        let word: u16 = 0x8001u16.rotate_left(r as u32) ^ (r as u16) << 3;
        v.extend_from_slice(&word.to_ne_bytes());
    }
    v
}

/// A byte buffer whose base address is guaranteed **even**, so `+1` into it is
/// guaranteed **odd** — `[u8; N]` alone has no alignment guarantee at all.
#[repr(align(2))]
struct Aligned<const N: usize>([u8; N]);

fn root_box(child: i16) -> Object {
    Object {
        ob_next: NIL,
        ob_head: child,
        ob_tail: child,
        ob_type: G_BOX,
        ob_flags: 0,
        ob_state: 0,
        // IP_HOLLOW / white interior / no border: a plain white ground.
        ob_spec: 0x0000_0000,
        ob_x: 0,
        ob_y: 0,
        ob_width: SCREEN_W as i16,
        ob_height: SCREEN_H as i16,
    }
}

fn child(ob_type: u16, spec: u64, w: i16, h: i16) -> Object {
    Object {
        ob_next: ROOT,
        ob_head: NIL,
        ob_tail: NIL,
        ob_type,
        ob_flags: 0,
        ob_state: 0,
        ob_spec: spec,
        ob_x: 8,
        ob_y: 8,
        ob_width: w,
        ob_height: h,
    }
}

fn render(tree: &[Object; 2]) -> PenBackend {
    let fb = PenBackend::new();
    // SAFETY: `tree` is a live, well-formed native (LP64) OBJECT array whose
    // ROOT index is in range for the whole call.
    unsafe { render_tree_lp64(tree.as_ptr(), ROOT, MAX_DEPTH, 0, 0, 0, 0, &fb) };
    fb
}

/// `G_ICON` with the 16×16 raster at `mask`/`data`, no overlay character and
/// no label — the ink in the frame is exactly the two raster blits.
fn iconblk(mask: u64, data: u64) -> IconBlk {
    IconBlk {
        ib_pmask: mask,
        ib_pdata: data,
        ib_ptext: 0,
        // fg = pen 1, bg = pen 2, no overlay character.
        ib_char: 0x1200,
        ib_xchar: 0,
        ib_ychar: 0,
        ib_xicon: 0,
        ib_yicon: 0,
        ib_wicon: 16,
        ib_hicon: ROWS as i16,
        ib_xtext: 0,
        ib_ytext: 0,
        ib_wtext: 0,
        ib_htext: 0,
    }
}

fn bitblk(data: u64) -> BitBlk {
    BitBlk {
        bi_pdata: data,
        bi_wb: 2,
        bi_hl: ROWS as i16,
        bi_x: 0,
        bi_y: 0,
        bi_color: 1,
    }
}

// ── the tests ─────────────────────────────────────────────────────────────

/// Every object draw is bracketed by the VDI mouse cursor's hide/show
/// (`render_tree_t` → `crate::vdi::cursor_hide`/`cursor_show`), and that
/// cursor — its hide depth `HIDE_COUNT`, its save-under and its position
/// (`vdi/mouse.rs`) — is one process-wide static with no locking (single-CPU
/// trap path). Two draws overlapping on two test threads can lose a hide
/// level, and the second draw then paints the arrow into its own frame (T14);
/// the tests in this binary must not overlap.
static LOCK: Mutex<()> = Mutex::new(());

fn lock() -> MutexGuard<'static, ()> {
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// `draw_icon` → `blit_plane(ib_pmask)` / `blit_plane(ib_pdata)`.
#[test]
fn a_g_icon_with_odd_raster_pointers_draws_like_an_even_one() {
    let _g = lock();
    let src = raster_bytes();

    let mut even = Aligned([0u8; ROWS * 2 + 2]);
    even.0[..src.len()].copy_from_slice(&src);
    let even_addr = even.0.as_ptr() as u64;
    assert_eq!(even_addr % 2, 0, "fixture base must be even");

    let mut odd = Aligned([0u8; ROWS * 2 + 2]);
    odd.0[1..1 + src.len()].copy_from_slice(&src);
    let odd_addr = odd.0.as_ptr() as u64 + 1;
    assert_eq!(
        odd_addr % 2,
        1,
        "fixture must hand blit_plane an ODD pointer"
    );

    let ib_even = iconblk(even_addr, even_addr);
    let ib_odd = iconblk(odd_addr, odd_addr);

    let t_even = [
        root_box(1),
        child(G_ICON, &ib_even as *const IconBlk as u64, 16, ROWS as i16),
    ];
    let t_odd = [
        root_box(1),
        child(G_ICON, &ib_odd as *const IconBlk as u64, 16, ROWS as i16),
    ];

    let fb_even = render(&t_even);
    let fb_odd = render(&t_odd);

    assert!(
        fb_even.count(1) > 0,
        "the fixture raster must set some pixels"
    );
    assert_eq!(
        fb_odd.snapshot(),
        fb_even.snapshot(),
        "an odd ib_pmask/ib_pdata must draw the same icon as an even one"
    );
}

/// `blit_image` → `blit_plane(bi_pdata)`.
#[test]
fn a_g_image_with_an_odd_bi_pdata_draws_like_an_even_one() {
    let _g = lock();
    let src = raster_bytes();

    let mut even = Aligned([0u8; ROWS * 2 + 2]);
    even.0[..src.len()].copy_from_slice(&src);
    let even_addr = even.0.as_ptr() as u64;

    let mut odd = Aligned([0u8; ROWS * 2 + 2]);
    odd.0[1..1 + src.len()].copy_from_slice(&src);
    let odd_addr = odd.0.as_ptr() as u64 + 1;
    assert_eq!(
        odd_addr % 2,
        1,
        "fixture must hand blit_plane an ODD pointer"
    );

    let bi_even = bitblk(even_addr);
    let bi_odd = bitblk(odd_addr);

    let t_even = [
        root_box(1),
        child(G_IMAGE, &bi_even as *const BitBlk as u64, 16, ROWS as i16),
    ];
    let t_odd = [
        root_box(1),
        child(G_IMAGE, &bi_odd as *const BitBlk as u64, 16, ROWS as i16),
    ];

    let fb_even = render(&t_even);
    let fb_odd = render(&t_odd);

    assert!(
        fb_even.count(1) > 0,
        "the fixture raster must set some pixels"
    );
    assert_eq!(
        fb_odd.snapshot(),
        fb_even.snapshot(),
        "an odd bi_pdata must draw the same image as an even one"
    );
}
