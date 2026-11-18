//! Shared fixture of the tests at the 32-plane setting (TASK/V63, TASK/V64):
//! the kernel's `VdiScreen` over a host store, a test host, a VDI parameter
//! block and a virtual workstation, with `planes()` settable so each test runs
//! both settings.
//!
//! **The backend is the kernel's `VdiScreen` over a host store.** The
//! kernel crate is not host-linkable, so its draw kernels are compiled in
//! here by path (`kernel/src/raster_kernel.rs`, `raster_line.rs`,
//! `raster_expand.rs`, `raster_copy.rs`, as the `raster_kernel_*` tests do)
//! and the backend methods call them exactly as `kernel/src/vdi_backend.rs`
//! does; the device-pixel conversions (`word_to_device`, `device_to_word`,
//! `pen_pixel`, `pel_to_rgb`, `pel_to_device16`, `xor_pixel`,
//! `clear_screen_word`, `read_pixel`) are transcribed expression for
//! expression. **Keep the transcription in sync with
//! `kernel/src/vdi_backend.rs`** — the tests are only as faithful as the copy.
//! `planes()` answers what the test stores, as the kernel's answers the boot
//! argument `vdiplanes` (TASK/V62).

#![allow(dead_code)]

#[allow(dead_code)]
#[path = "../../../kernel/src/raster_kernel.rs"]
pub mod raster_kernel;
#[allow(dead_code)]
#[path = "../../../kernel/src/raster_line.rs"]
pub mod raster_line;
#[allow(dead_code)]
#[path = "../../../kernel/src/raster_expand.rs"]
pub mod raster_expand;
#[allow(dead_code)]
#[path = "../../../kernel/src/raster_copy.rs"]
pub mod raster_copy;

use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

use tos_api::bdos::process::Abi;
use tos_api::host::Host;
use tos_api::vdi::backend::{ClipRect, MonoSrc, PatternFill, Pel, RasterBackend, Rect, WriteMode, VDI_PLANES_WIDE};
use tos_api::vdi::line::LineTarget;

pub const W: u16 = 320;
pub const H: u16 = 200;
/// The kernel's store pitch is the framebuffer's; a wider one than the
/// visible width is what the VideoCore may hand out.
pub const STRIDE: u32 = W as u32 + 16;

/// The colour bits of the 32-plane device pixel (`_raster.md` R1, R5).
pub const COLOUR: u32 = 0x00FF_FFFF;
/// `OVERLAY_BIT` of the 16-bit word, `vdi/vdi_fill.c:26`.
pub const OVERLAY_BIT: u16 = 0x0020;
/// The white pen's pixel in V63: pen 0 of the default palette, 5-bit 31
/// replicated to `0xFF` per gun (`_raster.md` §3, first bullet).
pub const WHITE_PIXEL: u32 = 0x00FF_FFFF;
/// `v_get_pixel`'s interim 16-bit word of the white pen (`0xFFDF`, the
/// pen-0 word, `_color-pixel-input.md` "Interim answer until V65").
pub const WHITE_WORD: u16 = 0xFFDF;
/// The cleared word at 16 planes, `memset(0xff)` (`vdi/vdi_control.c:610-618`).
pub const CLEARED_WORD_16: u16 = 0xFFFF;

// ── the kernel's device-pixel model, transcribed ─────────────────────────
//
// Keep in sync with `kernel/src/vdi_backend.rs` and `kernel/src/hal/framebuffer.rs`.

/// `kernel/src/hal/framebuffer.rs:36@rgb`.
pub const fn rgb(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

/// `kernel/src/vdi_backend.rs:174@expand5to8`.
pub const fn expand5to8(c5: u16) -> u8 {
    ((c5 << 3) | (c5 >> 2)) as u8
}

/// `kernel/src/vdi_backend.rs:192@OVERLAY_X`.
pub const OVERLAY_X: u32 = 0xFF00_0000;

/// `kernel/src/vdi_backend.rs:200@word_to_device`.
pub fn word_to_device(word: u16) -> u32 {
    let x = if word & OVERLAY_BIT != 0 { OVERLAY_X } else { 0 };
    x | rgb(expand5to8((word >> 11) & 0x1f), expand5to8((word >> 6) & 0x1f), expand5to8(word & 0x1f))
}

/// `kernel/src/vdi_backend.rs:213@device_to_word`.
pub fn device_to_word(px: u32) -> u16 {
    let r5 = ((px >> 19) & 0x1f) as u16;
    let g5 = ((px >> 11) & 0x1f) as u16;
    let b5 = ((px >> 3) & 0x1f) as u16;
    let o = ((px >> 26) & OVERLAY_BIT as u32) as u16; // bit 31 → bit 5
    (r5 << 11) | (g5 << 6) | b5 | o
}

/// `kernel/src/vdi_backend.rs:223@rgb_dist`.
pub fn rgb_dist(a: u32, b: u32) -> u32 {
    let dr = ((a >> 16) & 0xFF) as i32 - ((b >> 16) & 0xFF) as i32;
    let dg = ((a >> 8) & 0xFF) as i32 - ((b >> 8) & 0xFF) as i32;
    let db = (a & 0xFF) as i32 - (b & 0xFF) as i32;
    (dr * dr + dg * dg + db * db) as u32
}

/// The 32-plane quantisation of the spec, written from the formula and not
/// from the code under test: `q = (c * 255 + 500) / 1000`, `c` clamped to
/// 0..1000 (`docs/api/vdi/raster/_raster.md` P2, ADR-038 point 4).
pub fn q8(c: i32) -> u32 {
    (c.clamp(0, 1000) as u32 * 255 + 500) / 1000
}

/// The 16-plane quantisation, EmuTOS `vdi2fivebits` (`vdi/vdi_col.c:588-591`).
pub fn q5(c: i32) -> u32 {
    (c.clamp(0, 1000) as u32 * 31 + 500) / 1000
}

/// The realised value of an 8-bit gun, `(q * 1000 + 127) / 255` (P5).
pub fn a8(q: u32) -> i16 {
    ((q * 1000 + 127) / 255) as i16
}

/// The realised value of a 5-bit gun, EmuTOS `fivebits2vdi` (`vdi/vdi_col.c:804-807`).
pub fn a5(q: u32) -> i16 {
    ((q * 1000 + 16) / 31) as i16
}

/// The 32-plane pen pixel of a request, `0x00RRGGBB` (P3).
pub fn pixel32(rgb: [i32; 3]) -> u32 {
    (q8(rgb[0]) << 16) | (q8(rgb[1]) << 8) | q8(rgb[2])
}

/// The 16-plane pen word of a request, EmuTOS `set_color16` (`vdi/vdi_col.c:601-614`).
pub fn word16(rgb: [i32; 3]) -> u16 {
    ((q5(rgb[0]) << 11) | (q5(rgb[1]) << 6) | q5(rgb[2])) as u16
}

/// The pixel the draw path must write for pen `pen` of a freshly opened
/// workstation at `planes`: its default request (the screen's `query_color`
/// table, [`VIDEL_PALETTE1`] for pens 0..15, 0 above) by the setting's rule;
/// pen 255 is left at 0 by `init_wk` (P6, `vdi/vdi_control.c:360`).
pub fn default_pen_pixel(planes: u16, pen: usize) -> u32 {
    let (r, g, b) = if pen < 16 { VIDEL_PALETTE1[pen] } else { (0, 0, 0) };
    let rgb = [r as i32, g as i32, b as i32];
    if planes == VDI_PLANES_WIDE {
        pixel32(rgb)
    } else {
        word_to_device(word16(rgb))
    }
}

/// The top 5 bits of each gun of a 32-bit pixel, the projection of
/// acceptance (ii) (`_raster.md` P9): `(r5, g5, b5)`.
pub fn guns5(px: u32) -> (i32, i32, i32) {
    (((px >> 19) & 0x1f) as i32, ((px >> 11) & 0x1f) as i32, ((px >> 3) & 0x1f) as i32)
}

/// EmuTOS `videl_palette1` (`vdi/vdi_col.c:130-136`), the device default
/// `query_color` answers for pens 0..15 (`kernel/src/vdi_backend.rs`,
/// `VIDEL_PALETTE1`); a workstation seeds its pseudo-palette from it.
#[rustfmt::skip]
pub const VIDEL_PALETTE1: [(u16, u16, u16); 16] = [
    (1000, 1000, 1000), (0, 0, 0),       (1000, 0, 0),    (0, 1000, 0),
    (0, 0, 1000),       (0, 1000, 1000), (1000, 1000, 0), (1000, 0, 1000),
    (733, 733, 733),    (533, 533, 533), (667, 0, 0),     (0, 667, 0),
    (0, 0, 667),        (0, 667, 667),   (667, 667, 0),   (667, 0, 667),
];

/// The kernel's `VdiScreen` over a host store of `H` rows of [`STRIDE`]
/// pixels, without the cache maintenance (`clean`), which writes nothing.
pub struct Screen {
    pub planes: AtomicU16,
    pub px: Mutex<Vec<u32>>,
    /// The active pen table, in the form of the setting (TASK/V64): the
    /// 16-bit word at 16 planes, the pixel `0x00RRGGBB` at 32.
    pub palette: Mutex<[u32; 256]>,
    pub req: Mutex<[(u16, u16, u16); 256]>,
}

impl Screen {
    /// `VdiScreen::pel_to_rgb` (`kernel/src/vdi_backend.rs:282@pel_to_rgb`).
    pub fn pel_to_rgb(&self, pel: Pel) -> u32 {
        self.pen_pixel(self.palette.lock().unwrap()[pel as usize])
    }
    /// `pen_pixel` (`kernel/src/vdi_backend.rs:1171@pen_pixel`, V64): the entry itself at 32
    /// planes, the word expanded by `word_to_device` at 16.
    pub fn pen_pixel(&self, entry: u32) -> u32 {
        if self.planes() == VDI_PLANES_WIDE {
            entry
        } else {
            word_to_device(entry as u16)
        }
    }
    /// The store as the kernel kernels see it (`kernel_surface`).
    pub fn with_surface<R>(&self, f: impl FnOnce(raster_kernel::Surface, *mut u32) -> R) -> R {
        let mut px = self.px.lock().unwrap();
        let base = px.as_mut_ptr();
        f(raster_kernel::Surface { base, stride_px: STRIDE as usize }, base)
    }
    pub fn target() -> LineTarget {
        LineTarget { width: W as u32, height: H as u32, stride: STRIDE }
    }
    /// `VdiScreen::combine_index` with `xor_pixel` (`kernel/src/vdi_backend.rs:408@xor_pixel`).
    pub fn combine_index(px: &mut [u32], idx: usize, set: bool, mode: WriteMode, fg: u32, bg: u32) {
        let Some(p) = px.get_mut(idx) else { return };
        match mode {
            WriteMode::Replace => *p = if set { fg } else { bg },
            WriteMode::Transparent if set => *p = fg,
            WriteMode::Xor if set => *p = !*p,
            WriteMode::RevTransparent if !set => *p = fg,
            _ => {}
        }
    }
    pub fn raw(&self, x: i16, y: i16) -> u32 {
        self.px.lock().unwrap()[y as usize * STRIDE as usize + x as usize]
    }
    /// The visible store, row by row.
    pub fn snapshot(&self) -> Vec<u32> {
        let px = self.px.lock().unwrap();
        (0..H as usize).flat_map(|y| px[y * STRIDE as usize..y * STRIDE as usize + W as usize].to_vec()).collect()
    }
}

/// `kernel_area` of an inclusive box.
pub fn area(x0: i32, y0: i32, x1: i32, y1: i32) -> raster_kernel::Area {
    raster_kernel::Area { x: x0 as usize, y: y0 as usize, w: (x1 - x0 + 1) as usize, h: (y1 - y0 + 1) as usize }
}

/// `rect ∩ clip ∩ screen`, the box every kernel draw method computes first.
pub fn clamp(x: i16, y: i16, w: i16, h: i16, clip: ClipRect) -> Option<(i32, i32, i32, i32)> {
    ClipRect {
        xmn: x.max(clip.xmn),
        ymn: y.max(clip.ymn),
        xmx: ((x as i32 + w as i32 - 1) as i16).min(clip.xmx),
        ymx: ((y as i32 + h as i32 - 1) as i16).min(clip.ymx),
    }
    .clamp_to(W, H)
}

impl RasterBackend for Screen {
    fn width(&self) -> u16 {
        W
    }
    fn height(&self) -> u16 {
        H
    }
    fn num_colors(&self) -> u16 {
        256
    }
    fn planes(&self) -> u16 {
        self.planes.load(Ordering::Relaxed)
    }
    fn set_color(&self, index: u16, r: u16, g: u16, b: u16) {
        if (index as usize) < 256 {
            self.req.lock().unwrap()[index as usize] = (r, g, b);
        }
    }
    fn query_color(&self, index: u16) -> (u16, u16, u16) {
        self.req.lock().unwrap().get(index as usize).copied().unwrap_or((0, 0, 0))
    }
    fn set_active_palette(&self, palette: &[u32; 256]) {
        *self.palette.lock().unwrap() = *palette;
    }
    /// `VdiScreen::pel_to_device16` with `pen_word16` (`kernel/src/vdi_backend.rs:1160@pen_word16`): the word at
    /// 16 planes, the pen pixel's top 5 bits per gun at 32 (until TASK/V65).
    fn pel_to_device16(&self, pel: Pel) -> u16 {
        let entry = self.palette.lock().unwrap()[pel as usize];
        if self.planes() == VDI_PLANES_WIDE {
            device_to_word(entry)
        } else {
            entry as u16
        }
    }
    fn device_to_rgb565(&self, argb: u32) -> u16 {
        device_to_word(argb)
    }
    fn rgb565_to_device(&self, px: u16) -> u32 {
        word_to_device(px)
    }
    /// `VdiScreen::read_pixel` (`kernel/src/vdi_backend.rs:531@read_pixel`).
    fn read_pixel(&self, x: i16, y: i16) -> Pel {
        if x < 0 || y < 0 || x as u16 >= W || y as u16 >= H {
            return 0;
        }
        let packed = self.raw(x, y) & 0x00FF_FFFF;
        let pal = *self.palette.lock().unwrap();
        let mut best = 0u8;
        let mut best_d = u32::MAX;
        for (i, &entry) in pal.iter().enumerate() {
            let d = rgb_dist(packed, self.pen_pixel(entry) & 0x00FF_FFFF);
            if d < best_d {
                best_d = d;
                best = i as u8;
            }
        }
        best
    }
    /// `VdiScreen::clear_screen_word` (`kernel/src/vdi_backend.rs:558@clear_screen_word`):
    /// `fb.clear(rgb565_to_device(word))` over the visible area.
    fn clear_screen_word(&self, word: u16) {
        let v = self.rgb565_to_device(word);
        let mut px = self.px.lock().unwrap();
        for y in 0..H as usize {
            px[y * STRIDE as usize..y * STRIDE as usize + W as usize].fill(v);
        }
    }
    fn fill_rect(&self, x: i16, y: i16, w: i16, h: i16, pel: Pel, clip: ClipRect) {
        let Some((x0, y0, x1, y1)) = clamp(x, y, w, h, clip) else { return };
        let color = self.pel_to_rgb(pel);
        // SAFETY: the clamped box lies in the store.
        self.with_surface(|s, _| unsafe { raster_kernel::fill_solid(s, area(x0, y0, x1, y1), color) });
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
        let (color, bg) = (self.pel_to_rgb(pel), self.pel_to_rgb(0));
        let target = Screen::target();
        // SAFETY: the store holds `H * STRIDE` pixels, `raster_line::abline`'s contract.
        self.with_surface(|_, base| unsafe {
            raster_line::abline(base, &target, (x0, y0, x1, y1), mode, clip, mask, last, color, bg).0
        })
    }
    fn fill_pattern(&self, x: i16, y: i16, w: i16, h: i16, fill: PatternFill, clip: ClipRect) {
        if fill.pattern.is_empty() {
            return;
        }
        let Some((x0, y0, x1, y1)) = clamp(x, y, w, h, clip) else { return };
        let (fg, bg) = (self.pel_to_rgb(fill.fg), self.pel_to_rgb(fill.bg));
        let kfill = raster_kernel::Fill { pattern: fill.pattern, mode: fill.mode, fg, bg };
        // SAFETY: the clamped box lies in the store.
        self.with_surface(|s, _| unsafe { raster_kernel::fill_pattern(s, area(x0, y0, x1, y1), &kfill) });
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
        let (fg_rgb, bg_rgb) = (self.pel_to_rgb(fg), self.pel_to_rgb(0));
        let on_screen =
            x >= 0 && y >= 0 && x as i32 + cell_w as i32 <= W as i32 && y as i32 + cell_h as i32 <= H as i32;
        if clip == ClipRect::OFF && !on_screen {
            let target = Screen::target();
            let mut px = self.px.lock().unwrap();
            tos_api::vdi::line::glyph_unclipped(&target, x, y, glyph, cell_w, cell_h, |idx, set| {
                Screen::combine_index(&mut px, idx, set, mode, fg_rgb, bg_rgb);
            });
            return;
        }
        let Some(bx) = clamp(x, y, cell_w, cell_h, clip) else { return };
        // SAFETY: the clamped box lies in the store.
        self.with_surface(|s, _| unsafe {
            if !raster_expand::glyph(s, x as i32, y as i32, glyph, cell_w as usize, bx, mode, fg_rgb, bg_rgb) {
                raster_expand::glyph_reference(s, x as i32, y as i32, glyph, cell_w as usize, bx, mode, fg_rgb, bg_rgb);
            }
        });
    }
    fn blit_mono(&self, dst: Rect, src: MonoSrc, fg: Pel, bg: Pel, mode: WriteMode, clip: ClipRect) {
        let Some(bx) = clamp(dst.x, dst.y, dst.w, dst.h, clip) else { return };
        let (fg_rgb, bg_rgb) = (self.pel_to_rgb(fg), self.pel_to_rgb(bg));
        // SAFETY: the clamped box lies in the store.
        self.with_surface(|s, _| unsafe {
            if !raster_expand::mono(s, dst.x as i32, dst.y as i32, &src, bx, mode, fg_rgb, bg_rgb) {
                raster_expand::mono_reference(s, dst.x as i32, dst.y as i32, &src, bx, mode, fg_rgb, bg_rgb);
            }
        });
    }
    fn copy_rect(&self, src: Rect, dx: i16, dy: i16, op: u8, clip: ClipRect) {
        if op & 0x0f == 5 {
            return;
        }
        let Some(bx) = clamp(dx, dy, src.w, src.h, clip) else { return };
        // SAFETY: the clamped box lies on the screen in the store, `W <= STRIDE`.
        self.with_surface(|s, _| unsafe {
            raster_copy::copy_rect(s, W as i32, H as i32, src.x as i32, src.y as i32, dx as i32, dy as i32, bx, op)
        });
    }
    fn get_device_pixel(&self, x: i16, y: i16) -> u32 {
        if x < 0 || y < 0 || x as u16 >= W || y as u16 >= H {
            return 0;
        }
        self.raw(x, y)
    }
    /// `VdiScreen::get_device_pixel_wrapped`: the store with its own pitch.
    fn get_device_pixel_wrapped(&self, x: i16, y: i16) -> u32 {
        match Screen::target().index(x as i32, y as i32) {
            Some(idx) => self.px.lock().unwrap()[idx],
            None => 0,
        }
    }
    fn put_device_pixel(&self, x: i16, y: i16, argb: u32) {
        if x >= 0 && y >= 0 && (x as u16) < W && (y as u16) < H {
            self.px.lock().unwrap()[y as usize * STRIDE as usize + x as usize] = argb;
        }
    }
    fn flush_rect(&self, _x0: i16, _y0: i16, _x1: i16, _y1: i16) {}
}

// ── host, workstation, parameter block ───────────────────────────────────

pub struct TestHost {
    pub screen: Screen,
}

impl Host for TestHost {
    fn console_write_byte(&self, _byte: u8) {}
    fn raster(&self) -> Option<&dyn RasterBackend> {
        Some(&self.screen)
    }
}

pub static HOST: OnceLock<&'static TestHost> = OnceLock::new();
pub static HOST_REF: OnceLock<&'static dyn Host> = OnceLock::new();
/// The installed host, the VDI handle table and the store are process-wide.
pub static SERIAL: Mutex<()> = Mutex::new(());

pub fn screen() -> &'static Screen {
    &HOST.get().expect("setting() not called").screen
}

pub const POISON: i16 = 0x7EAD;

pub struct Pb {
    pub contrl: Box<[i16; 16]>,
    pub intin: Box<[i16; 128]>,
    pub ptsin: Box<[i16; 256]>,
    pub intout: Box<[i16; 512]>,
    pub ptsout: Box<[i16; 256]>,
    pub block: Box<[u64; 5]>,
}

impl Pb {
    pub fn new() -> Pb {
        let mut pb = Pb {
            contrl: Box::new([0; 16]),
            intin: Box::new([0; 128]),
            ptsin: Box::new([0; 256]),
            intout: Box::new([0; 512]),
            ptsout: Box::new([0; 256]),
            block: Box::new([0; 5]),
        };
        pb.block[0] = pb.contrl.as_ptr() as u64;
        pb.block[1] = pb.intin.as_ptr() as u64;
        pb.block[2] = pb.ptsin.as_ptr() as u64;
        pb.block[3] = pb.intout.as_ptr() as u64;
        pb.block[4] = pb.ptsout.as_ptr() as u64;
        pb
    }

    /// One VDI call through the real dispatcher: `contrl[0]` opcode, `[1]`
    /// PTSIN pairs, `[3]` INTIN count, `[5]` sub-opcode, `[6]` handle.
    pub fn call(&mut self, opcode: i16, sub: i16, handle: i16, intin: &[i16], ptsin: &[i16]) -> i16 {
        self.contrl.fill(0);
        self.intout.fill(POISON);
        self.ptsout.fill(POISON);
        self.intin[..intin.len()].copy_from_slice(intin);
        self.ptsin[..ptsin.len()].copy_from_slice(ptsin);
        self.contrl[0] = opcode;
        self.contrl[1] = (ptsin.len() / 2) as i16;
        self.contrl[3] = intin.len() as i16;
        self.contrl[5] = sub;
        self.contrl[6] = handle;
        // SAFETY: `block` holds five live pointers to arrays owned by `self`,
        // each larger than any opcode issued here writes.
        unsafe { tos_api::vdi::dispatch(self.block.as_ptr() as u64, Abi::Lp64) }
    }
}

/// A virtual workstation on the physical one, opened at the current
/// setting with every attribute index 1 and raster coordinates; closed on
/// drop.
pub struct Ws {
    pub h: i16,
    pub pb: Pb,
}

impl Ws {
    pub fn open() -> Ws {
        let mut pb = Pb::new();
        let mut work_in = [1i16; 11];
        work_in[10] = 2;
        pb.call(1, 0, 0, &work_in, &[]); // v_opnwk
        let phys = pb.contrl[6];
        pb.call(100, 0, phys, &work_in, &[]); // v_opnvwk
        let h = pb.contrl[6];
        assert!(h > 0, "v_opnvwk handle {h}");
        Ws { h, pb }
    }
    pub fn call(&mut self, opcode: i16, sub: i16, intin: &[i16], ptsin: &[i16]) {
        self.pb.call(opcode, sub, self.h, intin, ptsin);
    }
    pub fn v_clrwk(&mut self) {
        self.call(3, 0, &[], &[]);
    }
    pub fn vswr_mode(&mut self, mode: i16) {
        self.call(32, 0, &[mode], &[]);
    }
    /// Solid interior, pen `pen`, no perimeter.
    pub fn solid_fill(&mut self, pen: i16) {
        self.call(23, 0, &[1], &[]); // vsf_interior FIS_SOLID
        self.call(25, 0, &[pen], &[]); // vsf_color
        self.call(104, 0, &[0], &[]); // vsf_perimeter off
    }
    pub fn vr_recfl(&mut self, x0: i16, y0: i16, x1: i16, y1: i16) {
        self.call(114, 0, &[], &[x0, y0, x1, y1]);
    }
    pub fn vs_clip(&mut self, on: bool, x0: i16, y0: i16, x1: i16, y1: i16) {
        self.call(129, 0, &[on as i16], &[x0, y0, x1, y1]);
    }
    /// `v_get_pixel(x, y)` → `(INTOUT[0], INTOUT[1])`.
    pub fn v_get_pixel(&mut self, x: i16, y: i16) -> (i16, u16) {
        self.call(105, 0, &[], &[x, y]);
        (self.pb.intout[0], self.pb.intout[1] as u16)
    }
    pub fn v_contourfill(&mut self, x: i16, y: i16, intin0: i16) {
        self.call(103, 0, &[intin0], &[x, y]);
    }
}

impl Drop for Ws {
    fn drop(&mut self) {
        let h = self.h;
        self.pb.call(101, 0, h, &[], &[]); // v_clsvwk
    }
}

/// Take the lock, install the host once and set the depth the boot
/// argument would have chosen.
pub fn setting(planes: u16) -> MutexGuard<'static, ()> {
    let g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let host = *HOST.get_or_init(|| {
        let mut req = [(0u16, 0u16, 0u16); 256];
        req[..16].copy_from_slice(&VIDEL_PALETTE1);
        Box::leak(Box::new(TestHost {
            screen: Screen {
                planes: AtomicU16::new(16),
                px: Mutex::new(vec![0; H as usize * STRIDE as usize]),
                palette: Mutex::new([0; 256]),
                req: Mutex::new(req),
            },
        }))
    });
    HOST_REF.get_or_init(|| {
        let href: &'static &'static dyn Host = Box::leak(Box::new(host as &'static dyn Host));
        tos_api::host::install_host(href);
        host as &'static dyn Host
    });
    host.screen.planes.store(planes, Ordering::Relaxed);
    g
}

/// xorshift64*, deterministic: the X noise.
pub struct Rng(pub u64);
impl Rng {
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn x_byte(&mut self) -> u32 {
        ((self.next() >> 56) as u32) << 24
    }
    pub fn u32(&mut self) -> u32 {
        (self.next() >> 32) as u32
    }
}

/// The X byte of a store word.
pub fn x_of(px: u32) -> u32 {
    px >> 24
}
