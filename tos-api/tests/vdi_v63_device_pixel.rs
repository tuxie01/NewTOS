//! V63 — the 32-plane device pixel: rules R1-R6 of
//! `docs/api/vdi/raster/_raster.md` § "32-plane setting: the device pixel
//! (ADR-038 point 4 and 11, V63)", pinned at the 32-plane setting, and the
//! unchanged behaviour at the default 16 planes beside it.
//!
//! - R1/R2: the device pixel is the store word `0xXXRRGGBB`; a pen writes
//!   X = 0 — fills, lines and text draw the pen's pixel.
//! - R3: `v_clrwk` writes a constant; tests assert the colour and what a
//!   masked read returns, never the X byte of a cleared pixel (TASK/V63,
//!   lead's reading, point 3).
//! - R4: logic operations are bitwise on all 32 bits, X included
//!   (`WM_XOR` complements the stored pixel and ignores the pen; the sixteen
//!   BitBlt operations of the kernel's `raster_op`/`copy_rect`).
//! - R5: every read ignores X: `v_get_pixel`, the flood fill's span probes
//!   and its seed read (TASK/V63, lead's reading, point 1); a cleared screen
//!   reads as the white pen.
//! - R6: at 16 planes none of that applies: the reads stay unmasked, a
//!   cleared screen reads `0xFFFF`, a seed fill on it draws nothing (EmuTOS
//!   `vdi/vdi_fill.c:671-681,972`).
//!
//! **The backend is the kernel's `VdiScreen` over a host store.** The
//! kernel crate is not host-linkable, so its draw kernels are compiled in
//! here by path (`kernel/src/raster_kernel.rs`, `raster_line.rs`,
//! `raster_expand.rs`, `raster_copy.rs`, as the `raster_kernel_*` tests do)
//! and the backend methods call them exactly as `kernel/src/vdi_backend.rs`
//! does; the device-pixel conversions (`word_to_device`, `device_to_word`,
//! `pel_to_rgb`, `xor_pixel`, `clear_screen_word`, `read_pixel`) are
//! transcribed expression for expression. **Keep the transcription in sync
//! with `kernel/src/vdi_backend.rs`** — the tests are only as faithful as
//! the copy. `planes()` answers what the test stores, as the kernel's
//! answers the boot argument `vdiplanes` (TASK/V62).
//!
//! Until V65 `vro_cpyfm`/`vrt_cpyfm` refuse a 32-plane screen
//! (`CHUNKY_PLANES`, `_raster.md` §4), so R4 for the sixteen operations is
//! pinned on the kernel's `raster_op` and `copy_rect` directly. Until V65
//! `v_get_pixel` keeps its 16-bit answer shape (`_color-pixel-input.md`,
//! "Interim answer until V65", not a contract): the tests compare it with
//! the answer for the same pixel with X = 0 rather than with a constant,
//! except the cleared screen, whose value the spec states (`0xFFDF`).
//!
//! Ticket: `TASK/V63.md`.

#![cfg(all(target_arch = "aarch64", any(target_os = "macos", target_os = "linux")))]
#![allow(clippy::too_many_arguments)]

#[allow(dead_code)]
#[path = "../../kernel/src/raster_kernel.rs"]
mod raster_kernel;
#[allow(dead_code)]
#[path = "../../kernel/src/raster_line.rs"]
mod raster_line;
#[allow(dead_code)]
#[path = "../../kernel/src/raster_expand.rs"]
mod raster_expand;
#[allow(dead_code)]
#[path = "../../kernel/src/raster_copy.rs"]
mod raster_copy;

use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

use tos_api::bdos::process::Abi;
use tos_api::host::Host;
use tos_api::vdi::backend::{ClipRect, MonoSrc, PatternFill, Pel, RasterBackend, Rect, WriteMode};
use tos_api::vdi::line::LineTarget;

const W: u16 = 320;
const H: u16 = 200;
/// The kernel's store pitch is the framebuffer's; a wider one than the
/// visible width is what the VideoCore may hand out.
const STRIDE: u32 = W as u32 + 16;

/// The colour bits of the 32-plane device pixel (`_raster.md` R1, R5).
const COLOUR: u32 = 0x00FF_FFFF;
/// `OVERLAY_BIT` of the 16-bit word, `vdi/vdi_fill.c:26`.
const OVERLAY_BIT: u16 = 0x0020;
/// The white pen's pixel in V63: pen 0 of the default palette, 5-bit 31
/// replicated to `0xFF` per gun (`_raster.md` §3, first bullet).
const WHITE_PIXEL: u32 = 0x00FF_FFFF;
/// `v_get_pixel`'s interim 16-bit word of the white pen (`0xFFDF`, the
/// pen-0 word, `_color-pixel-input.md` "Interim answer until V65").
const WHITE_WORD: u16 = 0xFFDF;
/// The cleared word at 16 planes, `memset(0xff)` (`vdi/vdi_control.c:610-618`).
const CLEARED_WORD_16: u16 = 0xFFFF;

// ── the kernel's device-pixel model, transcribed ─────────────────────────
//
// Keep in sync with `kernel/src/vdi_backend.rs` and `kernel/src/hal/framebuffer.rs`.

/// `kernel/src/hal/framebuffer.rs:36@rgb`.
const fn rgb(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

/// `kernel/src/vdi_backend.rs:174@expand5to8`.
const fn expand5to8(c5: u16) -> u8 {
    ((c5 << 3) | (c5 >> 2)) as u8
}

/// `kernel/src/vdi_backend.rs:192@OVERLAY_X`.
const OVERLAY_X: u32 = 0xFF00_0000;

/// `kernel/src/vdi_backend.rs:200@word_to_device`.
fn word_to_device(word: u16) -> u32 {
    let x = if word & OVERLAY_BIT != 0 { OVERLAY_X } else { 0 };
    x | rgb(expand5to8((word >> 11) & 0x1f), expand5to8((word >> 6) & 0x1f), expand5to8(word & 0x1f))
}

/// `kernel/src/vdi_backend.rs:213@device_to_word`.
fn device_to_word(px: u32) -> u16 {
    let r5 = ((px >> 19) & 0x1f) as u16;
    let g5 = ((px >> 11) & 0x1f) as u16;
    let b5 = ((px >> 3) & 0x1f) as u16;
    let o = ((px >> 26) & OVERLAY_BIT as u32) as u16; // bit 31 → bit 5
    (r5 << 11) | (g5 << 6) | b5 | o
}

/// `kernel/src/vdi_backend.rs:223@rgb_dist`.
fn rgb_dist(a: u32, b: u32) -> u32 {
    let dr = ((a >> 16) & 0xFF) as i32 - ((b >> 16) & 0xFF) as i32;
    let dg = ((a >> 8) & 0xFF) as i32 - ((b >> 8) & 0xFF) as i32;
    let db = (a & 0xFF) as i32 - (b & 0xFF) as i32;
    (dr * dr + dg * dg + db * db) as u32
}

/// EmuTOS `videl_palette1` (`vdi/vdi_col.c:130-136`), the device default
/// `query_color` answers for pens 0..15 (`kernel/src/vdi_backend.rs`,
/// `VIDEL_PALETTE1`); a workstation seeds its pseudo-palette from it.
#[rustfmt::skip]
const VIDEL_PALETTE1: [(u16, u16, u16); 16] = [
    (1000, 1000, 1000), (0, 0, 0),       (1000, 0, 0),    (0, 1000, 0),
    (0, 0, 1000),       (0, 1000, 1000), (1000, 1000, 0), (1000, 0, 1000),
    (733, 733, 733),    (533, 533, 533), (667, 0, 0),     (0, 667, 0),
    (0, 0, 667),        (0, 667, 667),   (667, 667, 0),   (667, 0, 667),
];

/// The kernel's `VdiScreen` over a host store of `H` rows of [`STRIDE`]
/// pixels, without the cache maintenance (`clean`), which writes nothing.
struct Screen {
    planes: AtomicU16,
    px: Mutex<Vec<u32>>,
    palette: Mutex<[u16; 256]>,
    req: Mutex<[(u16, u16, u16); 256]>,
}

impl Screen {
    /// `VdiScreen::pel_to_rgb` (`kernel/src/vdi_backend.rs:282@pel_to_rgb`).
    fn pel_to_rgb(&self, pel: Pel) -> u32 {
        word_to_device(self.palette.lock().unwrap()[pel as usize])
    }
    /// The store as the kernel kernels see it (`kernel_surface`).
    fn with_surface<R>(&self, f: impl FnOnce(raster_kernel::Surface, *mut u32) -> R) -> R {
        let mut px = self.px.lock().unwrap();
        let base = px.as_mut_ptr();
        f(raster_kernel::Surface { base, stride_px: STRIDE as usize }, base)
    }
    fn target() -> LineTarget {
        LineTarget { width: W as u32, height: H as u32, stride: STRIDE }
    }
    /// `VdiScreen::combine_index` with `xor_pixel` (`kernel/src/vdi_backend.rs:408@xor_pixel`).
    fn combine_index(px: &mut [u32], idx: usize, set: bool, mode: WriteMode, fg: u32, bg: u32) {
        let Some(p) = px.get_mut(idx) else { return };
        match mode {
            WriteMode::Replace => *p = if set { fg } else { bg },
            WriteMode::Transparent if set => *p = fg,
            WriteMode::Xor if set => *p = !*p,
            WriteMode::RevTransparent if !set => *p = fg,
            _ => {}
        }
    }
    fn raw(&self, x: i16, y: i16) -> u32 {
        self.px.lock().unwrap()[y as usize * STRIDE as usize + x as usize]
    }
    /// The visible store, row by row.
    fn snapshot(&self) -> Vec<u32> {
        let px = self.px.lock().unwrap();
        (0..H as usize).flat_map(|y| px[y * STRIDE as usize..y * STRIDE as usize + W as usize].to_vec()).collect()
    }
}

/// `kernel_area` of an inclusive box.
fn area(x0: i32, y0: i32, x1: i32, y1: i32) -> raster_kernel::Area {
    raster_kernel::Area { x: x0 as usize, y: y0 as usize, w: (x1 - x0 + 1) as usize, h: (y1 - y0 + 1) as usize }
}

/// `rect ∩ clip ∩ screen`, the box every kernel draw method computes first.
fn clamp(x: i16, y: i16, w: i16, h: i16, clip: ClipRect) -> Option<(i32, i32, i32, i32)> {
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
    fn set_active_palette(&self, palette: &[u16; 256]) {
        *self.palette.lock().unwrap() = *palette;
    }
    fn pel_to_device16(&self, pel: Pel) -> u16 {
        self.palette.lock().unwrap()[pel as usize]
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
            let d = rgb_dist(packed, word_to_device(entry) & 0x00FF_FFFF);
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

struct TestHost {
    screen: Screen,
}

impl Host for TestHost {
    fn console_write_byte(&self, _byte: u8) {}
    fn raster(&self) -> Option<&dyn RasterBackend> {
        Some(&self.screen)
    }
}

static HOST: OnceLock<&'static TestHost> = OnceLock::new();
static HOST_REF: OnceLock<&'static dyn Host> = OnceLock::new();
/// The installed host, the VDI handle table and the store are process-wide.
static SERIAL: Mutex<()> = Mutex::new(());

fn screen() -> &'static Screen {
    &HOST.get().expect("setting() not called").screen
}

const POISON: i16 = 0x7EAD;

struct Pb {
    contrl: Box<[i16; 16]>,
    intin: Box<[i16; 128]>,
    ptsin: Box<[i16; 256]>,
    intout: Box<[i16; 512]>,
    ptsout: Box<[i16; 256]>,
    block: Box<[u64; 5]>,
}

impl Pb {
    fn new() -> Pb {
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
    fn call(&mut self, opcode: i16, sub: i16, handle: i16, intin: &[i16], ptsin: &[i16]) -> i16 {
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
struct Ws {
    h: i16,
    pb: Pb,
}

impl Ws {
    fn open() -> Ws {
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
    fn call(&mut self, opcode: i16, sub: i16, intin: &[i16], ptsin: &[i16]) {
        self.pb.call(opcode, sub, self.h, intin, ptsin);
    }
    fn v_clrwk(&mut self) {
        self.call(3, 0, &[], &[]);
    }
    fn vswr_mode(&mut self, mode: i16) {
        self.call(32, 0, &[mode], &[]);
    }
    /// Solid interior, pen `pen`, no perimeter.
    fn solid_fill(&mut self, pen: i16) {
        self.call(23, 0, &[1], &[]); // vsf_interior FIS_SOLID
        self.call(25, 0, &[pen], &[]); // vsf_color
        self.call(104, 0, &[0], &[]); // vsf_perimeter off
    }
    fn vr_recfl(&mut self, x0: i16, y0: i16, x1: i16, y1: i16) {
        self.call(114, 0, &[], &[x0, y0, x1, y1]);
    }
    fn vs_clip(&mut self, on: bool, x0: i16, y0: i16, x1: i16, y1: i16) {
        self.call(129, 0, &[on as i16], &[x0, y0, x1, y1]);
    }
    /// `v_get_pixel(x, y)` → `(INTOUT[0], INTOUT[1])`.
    fn v_get_pixel(&mut self, x: i16, y: i16) -> (i16, u16) {
        self.call(105, 0, &[], &[x, y]);
        (self.pb.intout[0], self.pb.intout[1] as u16)
    }
    fn v_contourfill(&mut self, x: i16, y: i16, intin0: i16) {
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
fn setting(planes: u16) -> MutexGuard<'static, ()> {
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
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn x_byte(&mut self) -> u32 {
        ((self.next() >> 56) as u32) << 24
    }
    fn u32(&mut self) -> u32 {
        (self.next() >> 32) as u32
    }
}

/// The X byte of a store word.
fn x_of(px: u32) -> u32 {
    px >> 24
}

// ── R1, R2: a pen writes X = 0 ───────────────────────────────────────────

/// The pens this test draws with: the default sixteen and three of the
/// extended table.
const PENS: [i16; 19] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 100, 255];

/// One scene of fills, lines and text in replace mode on a cleared screen;
/// returns the visible store.
fn pen_scene(planes: u16) -> Vec<u32> {
    let _g = setting(planes);
    let mut ws = Ws::open();
    ws.v_clrwk();
    let scr = screen();
    for (i, &pen) in PENS.iter().enumerate() {
        let (x0, y0) = (4 + 16 * i as i16, 4);
        ws.solid_fill(pen);
        ws.vr_recfl(x0, y0, x0 + 11, y0 + 11);
        // The pen's pixel: the pseudo-palette word through the kernel's pen
        // step, X = 0 (R2); the same 24 colour bits as at 16 planes (§3).
        let want = word_to_device(scr.pel_to_device16(pen as Pel));
        for (x, y) in [(x0, y0), (x0 + 11, y0 + 11), (x0 + 5, y0 + 6)] {
            let got = scr.raw(x, y);
            assert_eq!(got, want, "{planes} planes, pen {pen}: fill pixel at ({x},{y})");
            assert_eq!(x_of(got), 0, "{planes} planes, pen {pen}: a fill writes X = 0");
        }
        // A horizontal solid line one row below.
        ws.call(17, 0, &[pen], &[]); // vsl_color
        ws.call(6, 0, &[], &[x0, 20, x0 + 11, 20]); // v_pline
        for x in x0..=x0 + 11 {
            assert_eq!(scr.raw(x, 20), want, "{planes} planes, pen {pen}: line pixel at ({x},20)");
        }
    }
    // Text in replace mode: every cell pixel is the text pen's or pen 0's
    // pixel, both X = 0; some are the text pen's.
    let before = scr.snapshot();
    ws.call(22, 0, &[2], &[]); // vst_color red
    ws.call(8, 0, &[65, 66, 67, 87], &[40, 80]); // v_gtext "ABCW"
    let (red, white) = (word_to_device(scr.pel_to_device16(2)), word_to_device(scr.pel_to_device16(0)));
    let after = scr.snapshot();
    let changed: Vec<u32> = before.iter().zip(&after).filter(|(b, a)| b != a).map(|(_, &a)| a).collect();
    assert!(changed.iter().filter(|&&p| p == red).count() > 20, "{planes} planes: v_gtext drew no text pixels");
    for p in &changed {
        assert!(*p == red || *p == white, "{planes} planes: a text pixel 0x{p:08X} is neither pen");
        assert_eq!(x_of(*p), 0, "{planes} planes: text writes X = 0");
    }
    after
}

/// Fills, lines and text draw the pen's pixel `0x00RRGGBB` at 32 planes
/// (R1, R2), and the scene's store equals the 16-plane one (R6). The
/// comparison masks X, because a cleared pixel's X byte is not a contract
/// at 32 planes (R3); every drawn pixel's X is asserted above.
#[test]
fn fills_lines_and_text_draw_the_pen_pixel_with_x_zero() {
    let at16 = pen_scene(16);
    let at32 = pen_scene(32);
    let masked = |v: &[u32]| v.iter().map(|p| p & COLOUR).collect::<Vec<_>>();
    assert_eq!(masked(&at16), masked(&at32), "the scene's colours differ between 16 and 32 planes");
    for (a, b) in at16.iter().zip(&at32) {
        if x_of(*b) == 0 {
            assert_eq!(a, b, "a drawn pixel differs between 16 and 32 planes");
        }
    }
}

// ── R3, R5: a cleared screen reads as the white pen ──────────────────────

/// At 32 planes `v_clrwk` shows white and reads as the pixel of the white
/// pen through `v_get_pixel` (R3, R5; interim answer `0, 0xFFDF`), equal to
/// a pen-0 pixel. At 16 planes the answer stays EmuTOS's `0xFFFF`, one
/// `OVERLAY_BIT` apart from pen 0 (R6, `vdi/vdi_fill.c:671-681`).
#[test]
fn a_cleared_screen_reads_as_the_white_pen_at_32_planes() {
    for planes in [32u16, 16] {
        let _g = setting(planes);
        let mut ws = Ws::open();
        ws.v_clrwk();
        let scr = screen();
        for (x, y) in [(0, 0), (5, 5), (W as i16 - 1, H as i16 - 1)] {
            assert_eq!(scr.raw(x, y) & COLOUR, WHITE_PIXEL, "{planes} planes: the cleared colour at ({x},{y})");
        }
        ws.solid_fill(0);
        ws.vr_recfl(100, 100, 109, 109);
        assert_eq!(word_to_device(scr.pel_to_device16(0)), WHITE_PIXEL, "pen 0 is the white pen");
        let pen0 = ws.v_get_pixel(105, 105);
        assert_eq!(pen0, (0, WHITE_WORD), "{planes} planes: v_get_pixel of a pen-0 pixel");
        for (x, y) in [(0, 0), (5, 5), (W as i16 - 1, H as i16 - 1)] {
            let cleared = ws.v_get_pixel(x, y);
            if planes == 32 {
                assert_eq!(cleared, pen0, "32 planes: a cleared pixel at ({x},{y}) reads as the white pen");
            } else {
                assert_eq!(cleared, (0, CLEARED_WORD_16), "16 planes: v_get_pixel after v_clrwk (EmuTOS)");
                assert_eq!(cleared.1 ^ pen0.1, OVERLAY_BIT, "16 planes: cleared and pen 0 differ by OVERLAY_BIT");
            }
        }
    }
}

// ── R5: every read ignores X ─────────────────────────────────────────────

/// `v_get_pixel` at 32 planes answers for `X | colour` what it answers for
/// the colour with X = 0, for every X (R5). At 16 planes the top bit of X
/// still comes back as `OVERLAY_BIT` (R6, the unmasked read).
#[test]
fn v_get_pixel_ignores_the_x_byte_at_32_planes() {
    let colours = [0x00FF_FFFFu32, 0x0000_0000, 0x00FF_0000, 0x0000_FF00, 0x0000_00FF, 0x00BD_BDBD, 0x0012_3456];
    let xs = [0x00u32, 0x01, 0x20, 0x5A, 0x7F, 0x80, 0xA5, 0xFF];
    for planes in [32u16, 16] {
        let _g = setting(planes);
        let mut ws = Ws::open();
        let scr = screen();
        for &c in &colours {
            scr.put_device_pixel(10, 10, c);
            let plain = ws.v_get_pixel(10, 10);
            assert_eq!(plain, (0, device_to_word(c)), "{planes} planes: v_get_pixel of 0x{c:08X}");
            for &x in &xs {
                scr.put_device_pixel(10, 10, x << 24 | c);
                let got = ws.v_get_pixel(10, 10);
                if planes == 32 {
                    assert_eq!(got, plain, "32 planes: X 0x{x:02X} over 0x{c:06X} changed the read");
                } else {
                    let overlay = if x & 0x80 != 0 { OVERLAY_BIT } else { 0 };
                    assert_eq!(got, (0, plain.1 | overlay), "16 planes: X 0x{x:02X} over 0x{c:06X}");
                }
            }
        }
    }
}

/// A white region whose pixels carry random X noise, inside a black frame,
/// on a cleared screen; the seed pixel's X has its top bit set. Returns the
/// region's inclusive box.
fn noisy_region(ws: &mut Ws, rng: &mut Rng) -> (i16, i16, i16, i16) {
    ws.v_clrwk();
    let scr = screen();
    let (x0, y0, x1, y1) = (40i16, 30i16, 99i16, 69i16);
    let black = word_to_device(scr.pel_to_device16(1));
    for y in y0 - 1..=y1 + 1 {
        for x in x0 - 1..=x1 + 1 {
            let inside = (x0..=x1).contains(&x) && (y0..=y1).contains(&y);
            scr.put_device_pixel(x, y, if inside { rng.x_byte() | WHITE_PIXEL } else { black });
        }
    }
    scr.put_device_pixel(60, 50, 0xA5 << 24 | WHITE_PIXEL);
    (x0, y0, x1, y1)
}

/// The flood fill at 32 planes ignores X noise in the span probes and in
/// the seed read (R5, lead's reading point 1): a fill-while-same from a
/// noisy seed and a fill up to the black boundary both fill the whole
/// region with the fill pen's pixel and nothing beyond the frame. At 16
/// planes the seed read stays unmasked as in EmuTOS (`vdi_fill.c:972`): a
/// seed whose X top bit is set never equals a masked span colour, so the
/// fill-while-same draws nothing, while the boundary fill (pen colour vs.
/// masked spans) fills (R6).
#[test]
fn the_flood_fill_ignores_x_noise_at_32_planes() {
    for planes in [32u16, 16] {
        for (intin0, name) in [(-1i16, "fill while same"), (1, "fill to the black boundary")] {
            let _g = setting(planes);
            let mut ws = Ws::open();
            let mut rng = Rng(0x9E37_79B9_7F4A_7C15 ^ planes as u64);
            let (x0, y0, x1, y1) = noisy_region(&mut ws, &mut rng);
            let scr = screen();
            let before = scr.snapshot();
            ws.vswr_mode(1);
            ws.solid_fill(2);
            ws.v_contourfill(60, 50, intin0);
            let red = word_to_device(scr.pel_to_device16(2));
            let after = scr.snapshot();
            let inside = |i: usize| {
                let (x, y) = ((i % W as usize) as i16, (i / W as usize) as i16);
                (x0..=x1).contains(&x) && (y0..=y1).contains(&y)
            };
            let fills = planes == 32 || intin0 >= 0;
            for (i, (b, a)) in before.iter().zip(&after).enumerate() {
                if !inside(i) {
                    assert_eq!(a, b, "{planes} planes, {name}: pixel {i} outside the region changed");
                } else if fills {
                    assert_eq!(*a, red, "{planes} planes, {name}: pixel {i} of the region not filled");
                } else {
                    assert_eq!(a, b, "{planes} planes, {name}: the unmasked seed must fill nothing");
                }
            }
        }
    }
}

/// A fill-while-same seeded on a freshly cleared screen fills the clip
/// rectangle at 32 planes (the masked seed reads as the white pen) and
/// draws nothing at 16 planes (EmuTOS: the unmasked seed `0xFFFF` never
/// equals a masked span `0xFFDF`, `vdi_fill.c:755,763,972`).
#[test]
fn a_seed_fill_on_a_cleared_screen_fills_at_32_and_draws_nothing_at_16() {
    for planes in [32u16, 16] {
        let _g = setting(planes);
        let mut ws = Ws::open();
        ws.v_clrwk();
        let scr = screen();
        ws.vs_clip(true, 10, 10, 49, 29);
        ws.vswr_mode(1);
        ws.solid_fill(2);
        let before = scr.snapshot();
        ws.v_contourfill(20, 20, -1);
        ws.vs_clip(false, 0, 0, 0, 0);
        let red = word_to_device(scr.pel_to_device16(2));
        let after = scr.snapshot();
        for (i, (b, a)) in before.iter().zip(&after).enumerate() {
            let (x, y) = ((i % W as usize) as i16, (i / W as usize) as i16);
            let in_clip = (10..=49).contains(&x) && (10..=29).contains(&y);
            if planes == 32 && in_clip {
                assert_eq!(*a, red, "32 planes: ({x},{y}) in the clip not filled");
            } else {
                assert_eq!(a, b, "{planes} planes: ({x},{y}) changed");
            }
        }
    }
}

// ── R4: logic operations are bitwise on all 32 bits ──────────────────────

/// `WM_XOR` complements the whole stored pixel, X included, and ignores the
/// pen (R4; EmuTOS `vdi_line.c:494,501`): a solid fill and a solid line over
/// pixels with X noise, at both settings; a second pass restores the store.
#[test]
fn wm_xor_complements_all_32_bits_and_ignores_the_pen() {
    for planes in [32u16, 16] {
        let _g = setting(planes);
        let mut ws = Ws::open();
        let scr = screen();
        let mut rng = Rng(0x0123_4567_89AB_CDEF);
        for y in 0..H as i16 {
            for x in 0..W as i16 {
                scr.put_device_pixel(x, y, rng.u32());
            }
        }
        let start = scr.snapshot();
        ws.vswr_mode(3);
        ws.solid_fill(2);
        ws.vr_recfl(20, 20, 59, 39);
        ws.call(17, 0, &[5], &[]); // vsl_color, ignored by XOR
        ws.call(6, 0, &[], &[100, 50, 179, 50]); // v_pline, solid
        let once = scr.snapshot();
        for (i, (s, o)) in start.iter().zip(&once).enumerate() {
            let (x, y) = ((i % W as usize) as i16, (i / W as usize) as i16);
            let in_fill = (20..=59).contains(&x) && (20..=39).contains(&y);
            let on_line = (100..=179).contains(&x) && y == 50;
            if in_fill || on_line {
                assert_eq!(*o, !s, "{planes} planes: ({x},{y}) 0x{s:08X} not complemented on 32 bits");
            } else {
                assert_eq!(o, s, "{planes} planes: ({x},{y}) outside the XOR changed");
            }
        }
        ws.vr_recfl(20, 20, 59, 39);
        ws.call(6, 0, &[], &[100, 50, 179, 50]);
        assert_eq!(scr.snapshot(), start, "{planes} planes: XOR twice does not restore the store");
        ws.vswr_mode(1);
    }
}

/// The independent definition of the sixteen BitBlt operations on a 32-bit
/// pixel (`_raster.md` R4: the bitwise table on 32 bits; EmuTOS's on 16,
/// `vdi/vdi_raster.c:1007-1215`, `ALL_BLACK` writes all ones `:1213`).
fn bitblt(op: u8, s: u32, d: u32) -> u32 {
    let mut r = 0u32;
    for bit in 0..32 {
        let (sb, db) = ((s >> bit) & 1, (d >> bit) & 1);
        // op bit 3 = f(0,0), bit 2 = f(0,1), bit 1 = f(1,0), bit 0 = f(1,1)
        let idx = match (sb, db) {
            (0, 0) => 3,
            (0, 1) => 2,
            (1, 0) => 1,
            _ => 0,
        };
        r |= ((op as u32 >> idx) & 1) << bit;
    }
    r
}

/// The kernel's `raster_op` and its screen-to-screen `copy_rect` (the
/// `vro_cpyfm` and window-move path) act bitwise on all 32 bits, X included
/// (R4): every one of the sixteen operations on sources and destinations
/// with X noise; `ALL_BLACK` writes `0xFFFFFFFF`, `ALL_WHITE` 0.
#[test]
fn the_sixteen_logic_operations_are_bitwise_on_32_bits() {
    let mut rng = Rng(0xDEAD_BEEF_F00D_1234);
    for op in 0u8..16 {
        for _ in 0..64 {
            let (s, d) = (rng.u32(), rng.u32());
            assert_eq!(raster_copy::raster_op(op, s, d), bitblt(op, s, d), "raster_op {op} on 0x{s:08X}, 0x{d:08X}");
        }
    }
    assert_eq!(raster_copy::raster_op(15, 0x1234_5678, 0x00AB_CDEF), 0xFFFF_FFFF);
    assert_eq!(raster_copy::raster_op(0, 0x1234_5678, 0xFFAB_CDEF), 0);

    let _g = setting(32);
    let scr = screen();
    for op in 0u8..16 {
        for y in 0..H as i16 {
            for x in 0..W as i16 {
                scr.put_device_pixel(x, y, rng.u32());
            }
        }
        let start = scr.snapshot();
        let at = |x: i16, y: i16| start[y as usize * W as usize + x as usize];
        scr.copy_rect(Rect { x: 10, y: 10, w: 37, h: 9 }, 120, 60, op, ClipRect::full(W, H));
        let end = scr.snapshot();
        for y in 0..H as i16 {
            for x in 0..W as i16 {
                let in_dst = (120..157).contains(&x) && (60..69).contains(&y);
                let want = if in_dst { bitblt(op, at(x - 110, y - 50), at(x, y)) } else { at(x, y) };
                assert_eq!(end[y as usize * W as usize + x as usize], want, "copy_rect op {op} at ({x},{y})");
            }
        }
    }
}
