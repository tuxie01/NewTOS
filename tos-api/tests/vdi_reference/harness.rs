//! Harness for the VDI reference-conformance tests.
//!
//! These tests replay measured TOS behaviour against `tos_api::vdi`. The
//! reference is a dump taken by `tosdump/` on real hardware — EmuTOS on an
//! Atari TT — and the specification in `docs/api/` was extracted from that
//! same EmuTOS source, so a difference here has exactly one reading: NewTOS
//! is wrong. See `tosdump/reference/FINDINGS-r1.md`.
//!
//! **What these tests cover.** The dump records the five VDI parameter-block
//! arrays and the trap's return value — not pixels. So they exercise
//! everything *above* the [`RasterBackend`] seam: dispatch, `Vwk` attribute
//! state, validation and clamping, clipping, and the inquiries. Whether the
//! rasterizer puts the right pixels down is a separate question that needs a
//! pixel comparison the record format does not carry.
//!
//! **Why a memory backend.** The kernel's rasterizer lives in the `kernel`
//! crate, is pinned to AArch64 and reaches the board framebuffer through the
//! HAL, so it cannot run in a host test. The backend below is a plain linear
//! buffer at the same geometry the reference was measured at (640×480). It
//! is not the kernel's rasterizer, and these tests are not a statement about
//! the kernel's rasterizer.
//!
//! **What matching the geometry buys, named rather than left implicit.**
//! Some VDI answers are *derived from the screen*, so two workstations of
//! different size must give different numbers and a byte comparison across
//! machines would be meaningless. Running the harness at the reference's own
//! 640×480 turns those cases back into real comparisons — of the *formula*,
//! which is the part that is contract. The cases this applies to:
//!
//! * **`vq_chcells` (escape sub-op 1), case 155.** EmuTOS returns
//!   `v_cel_my + 1` rows and `v_cel_mx + 1` columns (`vdi/vdi_esc.c:62-67`),
//!   i.e. screen height/cell height and width/cell width. At 640×480 with
//!   the 8×16 cell that is 30 and 80, which is what the TT reported and what
//!   this harness checks. On the Pi 4's 1920×1080 the same formula gives 67
//!   and 240, and the full-machine dump comparison
//!   (`tosdump/reference/NEWTOS-VDI-all.BIN`, case 173) differs there **by
//!   arithmetic, not by defect** — see `tos-api/src/vdi/escape.rs`, `cols`
//!   and `rows`.
//! * **`v_opnvwk`/`vq_extnd` colour and plane counts.** `DEV_TAB[13]` and
//!   `INQ_TAB[4]`: the TT is a 16-colour 4-plane workstation, NewTOS a
//!   Falcon truecolor one. Depth is *not* neutralised here — see
//!   [`MemBackend::num_colors`] — so the handful of answers EmuTOS itself
//!   derives from `v_planes` / `numcolors` cannot be byte-compared. They are
//!   named in [`MACHINE_EXCLUDED`] and each is checked instead by EmuTOS'
//!   own formula evaluated on this workstation ([`substitute`]). Nothing is
//!   skipped silently and no case is `#[ignore]`d.
//!
//! **Serialisation.** The VDI keeps device-global state (handle table, `Vwk`
//! slots, palette). Cargo runs tests on parallel threads, so every case takes
//! [`VDI_LOCK`] and the workstation is opened once behind it.

#![allow(dead_code)]

use std::sync::{Mutex, OnceLock};

use tos_api::bdos::process::Abi;
use tos_api::host::Host;
use tos_api::vdi::backend::{ClipRect, MonoSrc, PatternFill, Pel, RasterBackend, Rect, WriteMode};
use tos_api::vdi::line::{self, LineTarget};

/// The geometry the reference dump was measured at: the TT in TT-medium.
/// `DEV_TAB[0..1]` in `tosdump/reference/EMUTOS14-VDI-r2.LOG` reads
/// `639,479`, i.e. 640×480.
pub const SCREEN_W: u16 = 640;
pub const SCREEN_H: u16 = 480;

// ── the memory raster backend ────────────────────────────────────────────

pub struct MemBackend {
    px: Mutex<Vec<u32>>,
    palette: Mutex<[u16; 256]>,
    req: Mutex<[(u16, u16, u16); 256]>,
}

/// EmuTOS' `videl_palette1` (`vdi/vdi_col.c:130-136`) as VDI `(r, g, b)`
/// triples in the 0..=1000 range, the same sixteen the kernel's backend
/// seeds pens 0..15 from (`kernel/src/vdi_backend.rs`). Index 0 is white
/// (background), 1 is black (foreground).
///
/// Seeding this matters: without it `vq_color` reports pen 0 as (0,0,0)
/// where every real machine reports white, and the difference is the test
/// harness's, not NewTOS's.
#[rustfmt::skip]
const VIDEL_PALETTE1: [(u16, u16, u16); 16] = [
    (1000, 1000, 1000), (0, 0, 0),       (1000, 0, 0),    (0, 1000, 0),
    (0, 0, 1000),       (0, 1000, 1000), (1000, 1000, 0), (1000, 0, 1000),
    (733, 733, 733),    (533, 533, 533), (667, 0, 0),     (0, 667, 0),
    (0, 0, 667),        (0, 667, 667),   (667, 667, 0),   (667, 0, 667),
];

/// EmuTOS' `videl_palette2` (`vdi/vdi_col.c:137-199`), the Falcon default
/// for pens 16..255, transcribed from the pinned source so that the double
/// seeds every pen the kernel's backend seeds (`kernel/src/vdi_backend.rs`,
/// `VIDEL_PALETTE2`). Until 2026-09-21 pens 16..255 read (0,0,0) here, and
/// the screen raster table's case H10 (pens 255/16) showed it as a harness
/// difference the QEMU run did not have.
#[rustfmt::skip]
const VIDEL_PALETTE2: [(u16, u16, u16); 240] = [
    (1000, 1000, 1000), (933, 933, 933), (867, 867, 867), (800, 800, 800),
    (733, 733, 733), (667, 667, 667), (600, 600, 600), (533, 533, 533),
    (467, 467, 467), (400, 400, 400), (333, 333, 333), (267, 267, 267),
    (200, 200, 200), (133, 133, 133), (67, 67, 67), (0, 0, 0),
    (1000, 0, 0), (1000, 0, 67), (1000, 0, 133), (1000, 0, 200),
    (1000, 0, 267), (1000, 0, 333), (1000, 0, 400), (1000, 0, 467),
    (1000, 0, 533), (1000, 0, 600), (1000, 0, 667), (1000, 0, 733),
    (1000, 0, 800), (1000, 0, 867), (1000, 0, 933), (1000, 0, 1000),
    (933, 0, 1000), (867, 0, 1000), (800, 0, 1000), (733, 0, 1000),
    (667, 0, 1000), (600, 0, 1000), (533, 0, 1000), (467, 0, 1000),
    (400, 0, 1000), (333, 0, 1000), (267, 0, 1000), (200, 0, 1000),
    (133, 0, 1000), (67, 0, 1000), (0, 0, 1000), (0, 67, 1000),
    (0, 133, 1000), (0, 200, 1000), (0, 267, 1000), (0, 333, 1000),
    (0, 400, 1000), (0, 467, 1000), (0, 533, 1000), (0, 600, 1000),
    (0, 667, 1000), (0, 733, 1000), (0, 800, 1000), (0, 867, 1000),
    (0, 933, 1000), (0, 1000, 1000), (0, 1000, 933), (0, 1000, 867),
    (0, 1000, 800), (0, 1000, 733), (0, 1000, 667), (0, 1000, 600),
    (0, 1000, 533), (0, 1000, 467), (0, 1000, 400), (0, 1000, 333),
    (0, 1000, 267), (0, 1000, 200), (0, 1000, 133), (0, 1000, 67),
    (0, 1000, 0), (67, 1000, 0), (133, 1000, 0), (200, 1000, 0),
    (267, 1000, 0), (333, 1000, 0), (400, 1000, 0), (467, 1000, 0),
    (533, 1000, 0), (600, 1000, 0), (667, 1000, 0), (733, 1000, 0),
    (800, 1000, 0), (867, 1000, 0), (933, 1000, 0), (1000, 1000, 0),
    (1000, 933, 0), (1000, 867, 0), (1000, 800, 0), (1000, 733, 0),
    (1000, 667, 0), (1000, 600, 0), (1000, 533, 0), (1000, 467, 0),
    (1000, 400, 0), (1000, 333, 0), (1000, 267, 0), (1000, 200, 0),
    (1000, 133, 0), (1000, 67, 0), (733, 0, 0), (733, 0, 67),
    (733, 0, 133), (733, 0, 200), (733, 0, 267), (733, 0, 333),
    (733, 0, 400), (733, 0, 467), (733, 0, 533), (733, 0, 600),
    (733, 0, 667), (733, 0, 733), (667, 0, 733), (600, 0, 733),
    (533, 0, 733), (467, 0, 733), (400, 0, 733), (333, 0, 733),
    (267, 0, 733), (200, 0, 733), (133, 0, 733), (67, 0, 733),
    (0, 0, 733), (0, 67, 733), (0, 133, 733), (0, 200, 733),
    (0, 267, 733), (0, 333, 733), (0, 400, 733), (0, 467, 733),
    (0, 533, 733), (0, 600, 733), (0, 667, 733), (0, 733, 733),
    (0, 733, 667), (0, 733, 600), (0, 733, 533), (0, 733, 467),
    (0, 733, 400), (0, 733, 333), (0, 733, 267), (0, 733, 200),
    (0, 733, 133), (0, 733, 67), (0, 733, 0), (67, 733, 0),
    (133, 733, 0), (200, 733, 0), (267, 733, 0), (333, 733, 0),
    (400, 733, 0), (467, 733, 0), (533, 733, 0), (600, 733, 0),
    (667, 733, 0), (733, 733, 0), (733, 667, 0), (733, 600, 0),
    (733, 533, 0), (733, 467, 0), (733, 400, 0), (733, 333, 0),
    (733, 267, 0), (733, 200, 0), (733, 133, 0), (733, 67, 0),
    (467, 0, 0), (467, 0, 67), (467, 0, 133), (467, 0, 200),
    (467, 0, 267), (467, 0, 333), (467, 0, 400), (467, 0, 467),
    (400, 0, 467), (333, 0, 467), (267, 0, 467), (200, 0, 467),
    (133, 0, 467), (67, 0, 467), (0, 0, 467), (0, 67, 467),
    (0, 133, 467), (0, 200, 467), (0, 267, 467), (0, 333, 467),
    (0, 400, 467), (0, 467, 467), (0, 467, 400), (0, 467, 333),
    (0, 467, 267), (0, 467, 200), (0, 467, 133), (0, 467, 67),
    (0, 467, 0), (67, 467, 0), (133, 467, 0), (200, 467, 0),
    (267, 467, 0), (333, 467, 0), (400, 467, 0), (467, 467, 0),
    (467, 400, 0), (467, 333, 0), (467, 267, 0), (467, 200, 0),
    (467, 133, 0), (467, 67, 0), (267, 0, 0), (267, 0, 67),
    (267, 0, 133), (267, 0, 200), (267, 0, 267), (200, 0, 267),
    (133, 0, 267), (67, 0, 267), (0, 0, 267), (0, 67, 267),
    (0, 133, 267), (0, 200, 267), (0, 267, 267), (0, 267, 200),
    (0, 267, 133), (0, 267, 67), (0, 267, 0), (67, 267, 0),
    (133, 267, 0), (200, 267, 0), (267, 267, 0), (267, 200, 0),
    (267, 133, 0), (267, 67, 0), (1000, 1000, 1000), (0, 0, 0),
];

impl MemBackend {
    /// The 16-bit screen word at `(x, y)`, read straight out of the pixel
    /// vector and converted here rather than through
    /// [`RasterBackend::get_device_pixel`] / `device_to_rgb565`. Used by the
    /// `v_get_pixel` substitute check so that the expected value does not come
    /// back out of the same two trait methods the implementation calls.
    ///
    /// Addressed as EmuTOS' `pixelread` addresses the screen
    /// (`vdi/vdi_fill.c:671-681`, `get_start_addr16`,
    /// `vdi/vdi_inline.h:14-17`): `(x, y)` is the word at index
    /// `y * SCREEN_W + x` of this 640-pixel-stride store, so an `x` off
    /// either edge reads the neighbouring row; an index before the first or
    /// past the last pixel reads 0 (user decision B, TASK/V24).
    ///
    /// **Colour model (V11/V12, 2026-09-21):** the kernel backend's — five
    /// bits per gun, replicated to eight in the 32-bpp store, and the
    /// 16-bit word's `OVERLAY_BIT` (bit 5, `vdi_fill.c:26`) replicated into
    /// the X byte (`kernel/src/vdi_backend.rs` `word_to_device` /
    /// `device_to_word`, `kernel/src/hal/framebuffer.rs` module doc). That is
    /// EmuTOS' truecolor model too: 5-bit guns from `set_color16` /
    /// `vq_color16` (`vdi_col.c:601-614,802-838`) plus the overlay bit.
    /// Until 2026-09-21 this double kept six green bits instead and could
    /// not see the kernel's dropped overlay bit (VDI-OVL-001); the two now
    /// agree, and `tos-api/tests/vdi_reference_overlay_bit.rs` pins the
    /// kernel's own expressions.
    pub fn screen_word(&self, x: i16, y: i16) -> u16 {
        let idx = y as i64 * SCREEN_W as i64 + x as i64;
        if !(0..SCREEN_W as i64 * SCREEN_H as i64).contains(&idx) {
            return 0;
        }
        let v = self.px.lock().unwrap()[idx as usize];
        let (r, g, b) = ((v >> 16) & 0xFF, (v >> 8) & 0xFF, v & 0xFF);
        let overlay = if v & 0x8000_0000 != 0 { 0x0020 } else { 0 };
        (((r >> 3) << 11) | ((g >> 3) << 6) | (b >> 3)) as u16 | overlay
    }

    fn new() -> Self {
        let mut req = [(0u16, 0u16, 0u16); 256];
        req[..16].copy_from_slice(&VIDEL_PALETTE1);
        req[16..].copy_from_slice(&VIDEL_PALETTE2);
        MemBackend {
            px: Mutex::new(vec![0; SCREEN_W as usize * SCREEN_H as usize]),
            palette: Mutex::new([0; 256]),
            req: Mutex::new(req),
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

impl RasterBackend for MemBackend {
    fn width(&self) -> u16 {
        SCREEN_W
    }
    fn height(&self) -> u16 {
        SCREEN_H
    }

    /// Reported exactly as the kernel's backend reports it
    /// (`kernel/src/vdi_backend.rs`): a Falcon truecolor workstation says
    /// 256 colours over 16 planes. The TT the reference came from says 16
    /// and 4, so `DEV_TAB[13]` and `INQ_TAB[4]` differ *by design* between
    /// the reference and NewTOS. That is a deliberate profile difference,
    /// not a defect, and it is recorded as such rather than hidden here.
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
    /// The kernel's `device_to_word` (`kernel/src/vdi_backend.rs`): top five
    /// bits of each gun, top bit of the X byte → `OVERLAY_BIT`.
    fn device_to_rgb565(&self, argb: u32) -> u16 {
        let (r, g, b) = ((argb >> 16) & 0xFF, (argb >> 8) & 0xFF, argb & 0xFF);
        let overlay = if argb & 0x8000_0000 != 0 { 0x0020 } else { 0 };
        (((r >> 3) << 11) | ((g >> 3) << 6) | (b >> 3)) as u16 | overlay
    }
    /// The kernel's `word_to_device`: 5→8-bit gun replication, and the
    /// overlay bit replicated into all eight X bits.
    fn rgb565_to_device(&self, px: u16) -> u32 {
        let (r5, g5, b5) = ((px >> 11) & 0x1F, (px >> 6) & 0x1F, px & 0x1F);
        let r = ((r5 << 3) | (r5 >> 2)) as u32;
        let g = ((g5 << 3) | (g5 >> 2)) as u32;
        let b = ((b5 << 3) | (b5 >> 2)) as u32;
        let x = if px & 0x0020 != 0 { 0xFF00_0000 } else { 0 };
        x | (r << 16) | (g << 8) | b
    }

    /// The VDI reads pixels back as PELs. This backend stores device words,
    /// so it resolves back through the active palette; an unmatched colour
    /// reads as pen 0, which is what a background pixel is.
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

    /// The kernel's line: the shared walk `tos_api::vdi::line::abline`
    /// (EmuTOS `abline`/`clip_line`, which pixels and which dash bit) with
    /// the store combine of `kernel/src/vdi_backend.rs` `combine_index`
    /// (TASK/V20). Until V20 this double drew a plain Bresenham that
    /// ignored the write mode, `last` and the clip semantics, so no line
    /// pixel here said anything about the kernel.
    #[allow(clippy::too_many_arguments)]
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
        let target = LineTarget { width: SCREEN_W as u32, height: SCREEN_H as u32, stride: SCREEN_W as u32 };
        let mut px = self.px.lock().unwrap();
        line::abline(&target, x0, y0, x1, y1, mode, clip, mask, last, |idx, set| {
            let p = &mut px[idx];
            match mode {
                WriteMode::Replace => *p = if set { fg } else { bg },
                WriteMode::Transparent if set => *p = fg,
                WriteMode::Xor if set => *p = !*p,
                WriteMode::RevTransparent if !set => *p = fg,
                _ => {}
            }
        })
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

    /// The four `MD_*` write modes as the kernel's `combine` applies them
    /// (`kernel/src/vdi_backend.rs`): `Replace` fg on set bits and bg on
    /// clear bits, `Transparent` fg on set bits only, `Xor` complements the
    /// whole stored word on set bits (`xor_pixel`, `!v`), `RevTransparent`
    /// (`MD_ERASE`) bg on clear bits only. Until 2026-09-21 this double
    /// painted every mode as `Replace` and could not see a mode defect;
    /// the screen raster table (`vdi-raster-screen.tsv`, section H) is what
    /// made the modes measurable.
    fn blit_mono(&self, dst: Rect, src: MonoSrc, fg: Pel, bg: Pel, mode: WriteMode, clip: ClipRect) {
        let f = self.rgb565_to_device(self.pel_to_device16(fg));
        let b = self.rgb565_to_device(self.pel_to_device16(bg));
        for row in 0..dst.h {
            for col in 0..dst.w {
                let sx = src.x + col;
                let sy = src.y + row;
                let wi = (sy as usize) * src.stride_words as usize + (sx as usize) / 16;
                let w = src.bits.word(wi).unwrap_or(0);
                let set = w & (0x8000u16 >> (sx.rem_euclid(16)) as u16) != 0;
                let (x, y) = (dst.x + col, dst.y + row);
                match mode {
                    WriteMode::Replace => self.put(x, y, if set { f } else { b }, clip),
                    WriteMode::Transparent => {
                        if set {
                            self.put(x, y, f, clip);
                        }
                    }
                    WriteMode::Xor => {
                        if set {
                            let v = self.get_device_pixel(x, y);
                            self.put(x, y, !v, clip);
                        }
                    }
                    WriteMode::RevTransparent => {
                        if !set {
                            self.put(x, y, b, clip);
                        }
                    }
                }
            }
        }
    }

    /// Screen→screen copy with the logic op applied on the stored word
    /// (EmuTOS `vro_cpyfm16` acts on the `UWORD` pixel; the kernel's
    /// `copy_rect` applies the same table to the 32-bit expansion, which
    /// is the same operation because the expansion is bitwise). The source
    /// is snapshotted first: for the direction EmuTOS chooses, every source
    /// pixel is read before the copy overwrites it, so a snapshot and the
    /// in-place walk produce the same picture — the screen raster table's
    /// section G measured that on the Falcon. `D_ONLY` touches nothing.
    fn copy_rect(&self, src: Rect, dx: i16, dy: i16, op: u8, clip: ClipRect) {
        if op & 0x0f == 5 {
            return;
        }
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
                let (x, y) = (dx + col, dy + row);
                let s = self.device_to_rgb565(tmp[(row as usize) * (src.w as usize) + col as usize]);
                let d = self.device_to_rgb565(self.get_device_pixel(x, y));
                let v = self.rgb565_to_device(raster_op16(op, s, d));
                self.put(x, y, v, clip);
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

/// The 16-entry BitBlt table on 16-bit words, `D' = op(S, D)`, as EmuTOS
/// `vro_cpyfm16` applies it (`vdi_raster.c:1008-1220`). Stated here in the
/// double rather than imported from the code under test.
pub fn raster_op16(op: u8, s: u16, d: u16) -> u16 {
    match op & 0x0f {
        0 => 0,
        1 => s & d,
        2 => s & !d,
        3 => s,
        4 => !s & d,
        5 => d,
        6 => s ^ d,
        7 => s | d,
        8 => !(s | d),
        9 => !(s ^ d),
        10 => !d,
        11 => s | !d,
        12 => !s,
        13 => !s | d,
        14 => !(s & d),
        _ => !0,
    }
}

// ── the host ─────────────────────────────────────────────────────────────

pub struct TestHost {
    raster: MemBackend,
}

impl Host for TestHost {
    /// The kernel's `Bconout(2)` device (`kernel::fbcon`): the VT52 console
    /// on this screen. `v_curtext` reaches it through `Crawio`
    /// (`vdi/vdi_esc.c:201-206`, CON04), so its cursor moves here as on the
    /// reference machine.
    fn console_write_byte(&self, byte: u8) {
        tos_api::vdi::alpha_conout(&self.raster, byte);
    }
    fn raster(&self) -> Option<&dyn RasterBackend> {
        Some(&self.raster)
    }
}

// SAFETY: every field is behind a Mutex.
unsafe impl Sync for TestHost {}
unsafe impl Send for TestHost {}

pub static VDI_LOCK: Mutex<()> = Mutex::new(());
static HOST_CELL: OnceLock<&'static TestHost> = OnceLock::new();
static HOST_REF: OnceLock<&'static dyn Host> = OnceLock::new();
static WK: OnceLock<Workstation> = OnceLock::new();

#[derive(Copy, Clone)]
pub struct Workstation {
    pub phys: i16,
    pub handle: i16,
    /// `SIZ_TAB` as this workstation reported it, i.e. `v_opnvwk`'s `ptsout`.
    /// The reset reads three device-defined defaults out of it, exactly as
    /// `tosdump/core/runner.c` does.
    pub siz_tab: [i16; 12],
}

pub const ABI: Abi = Abi::Lp64;

// ── the parameter block ──────────────────────────────────────────────────

pub const CONTRL_N: usize = 12;
pub const INTIN_N: usize = 128;
pub const PTSIN_N: usize = 256;
pub const INTOUT_N: usize = 128;
pub const PTSOUT_N: usize = 256;

/// The 0xDEAD poison (`tosdump` spec section 4). Everything the call is not
/// supposed to write stays at this value, which is what makes "wrote a field
/// it must not touch" visible at all.
pub const POISON: i16 = 0xDEADu16 as i16;

pub struct Pb {
    pub contrl: Box<[i16; CONTRL_N]>,
    pub intin: Box<[i16; INTIN_N]>,
    pub ptsin: Box<[i16; PTSIN_N]>,
    pub intout: Box<[i16; INTOUT_N]>,
    pub ptsout: Box<[i16; PTSOUT_N]>,
    block: Box<[u64; 5]>,
}

impl Pb {
    pub fn new() -> Pb {
        let mut pb = Pb {
            contrl: Box::new([0; CONTRL_N]),
            intin: Box::new([0; INTIN_N]),
            ptsin: Box::new([0; PTSIN_N]),
            intout: Box::new([0; INTOUT_N]),
            ptsout: Box::new([0; PTSOUT_N]),
            block: Box::new([0; 5]),
        };
        pb.block[0] = pb.contrl.as_ptr() as u64;
        pb.block[1] = pb.intin.as_ptr() as u64;
        pb.block[2] = pb.ptsin.as_ptr() as u64;
        pb.block[3] = pb.intout.as_ptr() as u64;
        pb.block[4] = pb.ptsout.as_ptr() as u64;
        pb
    }

    pub fn poison(&mut self) {
        self.contrl.fill(POISON);
        self.intin.fill(POISON);
        self.ptsin.fill(POISON);
        self.intout.fill(POISON);
        self.ptsout.fill(POISON);
    }

    /// Issue the call. Mirrors what the trap does: the caller has already
    /// filled `contrl`, everything comes back through the arrays, and the
    /// return value is the trap's `d0` (`flip_y`, `vdi/vdi_asm.S:124-125`),
    /// which the record carries at [`OFF_RET`] and [`check`] compares.
    pub fn call(&mut self) -> i16 {
        // SAFETY: `block` holds five live pointers to arrays owned by `self`
        // and large enough for every opcode under test.
        unsafe { tos_api::vdi::dispatch(self.block.as_ptr() as u64, ABI) }
    }

    fn op(&mut self, opcode: i16, sub: i16, n_pts: i16, n_int: i16, handle: i16) {
        self.contrl.fill(0);
        self.contrl[0] = opcode;
        self.contrl[1] = n_pts;
        self.contrl[3] = n_int;
        self.contrl[5] = sub;
        self.contrl[6] = handle;
        self.call();
    }

    fn int1(&mut self, opcode: i16, v: i16, handle: i16) {
        self.intin[0] = v;
        self.op(opcode, 0, 0, 1, handle);
    }
    fn int2(&mut self, opcode: i16, a: i16, b: i16, handle: i16) {
        self.intin[0] = a;
        self.intin[1] = b;
        self.op(opcode, 0, 0, 2, handle);
    }
    fn pts(&mut self, opcode: i16, x: i16, y: i16, handle: i16) {
        self.ptsin[0] = x;
        self.ptsin[1] = y;
        self.op(opcode, 0, 1, 0, handle);
    }
}

// ── workstation ──────────────────────────────────────────────────────────

/// Opens the physical workstation and one virtual workstation on it, with
/// the same `work_in` the dumper used (`tosdump/core/gem.c`): every
/// attribute index 1, raster coordinates. EmuTOS' `init_wk` derives the
/// open-time attribute state from exactly these
/// (`vdi/vdi_control.c:269-311`), so the reset below restores the same ones.
///
/// The dumper reached its handle through `graf_handle` because a GEM
/// application does; there is no AES here, so the physical workstation is
/// opened directly, which is what the AES itself does at boot.
pub fn workstation() -> &'static Workstation {
    WK.get_or_init(|| {
        let host: &'static TestHost = HOST_CELL.get_or_init(|| {
            Box::leak(Box::new(TestHost { raster: MemBackend::new() }))
        });
        let href: &'static &'static dyn Host =
            Box::leak(Box::new(*HOST_REF.get_or_init(|| host as &'static dyn Host)));
        tos_api::host::install_host(href);

        let mut pb = Pb::new();

        for i in 0..10 {
            pb.intin[i] = 1;
        }
        pb.intin[10] = 2; // raster coordinates
        pb.op(1, 0, 0, 11, 0); // v_opnwk
        let phys = pb.contrl[6];

        for i in 0..10 {
            pb.intin[i] = 1;
        }
        pb.intin[10] = 2;
        pb.op(100, 0, 0, 11, phys); // v_opnvwk
        let handle = pb.contrl[6];

        let mut siz_tab = [0i16; 12];
        siz_tab.copy_from_slice(&pb.ptsout[..12]);

        Workstation { phys, handle, siz_tab }
    })
}

/// The 16-bit screen word at `(x, y)` on the harness workstation's screen
/// ([`MemBackend::screen_word`]), for tests that compare a drawn rectangle
/// pixel by pixel with a probe map (`tosdump/probes/vdprobe.c` reads the
/// same words back through `vro_cpyfm`). Call [`workstation`] first.
pub fn screen_word(x: i16, y: i16) -> u16 {
    HOST_CELL.get().expect("workstation() not opened").raster.screen_word(x, y)
}

/// The reset the dumper runs before every case, call for call.
///
/// This is a port of `tosdump/core/runner.c`'s `td_reset_vdi`, and it has to
/// stay a port: the reference dump was measured with exactly this sequence,
/// so any divergence here compares two different things. The three
/// device-defined values come from the `SIZ_TAB` this workstation reported
/// (EmuTOS `vdi_control.c:349-352`, slot mapping `vdi_defs.h:100-104`):
/// line width `SIZ_TAB[4]`, marker height `SIZ_TAB[9]`, character height
/// `SIZ_TAB[1]`.
pub fn reset_vdi(pb: &mut Pb, wk: &Workstation) {
    let h = wk.handle;
    pb.op(3, 0, 0, 0, h); // v_clrwk

    pb.int1(32, 1, h); // vswr_mode  replace

    pb.int1(15, 1, h); // vsl_type   solid
    pb.pts(16, wk.siz_tab[4], 0, h); // vsl_width  DEF_LWID
    pb.int1(17, 1, h); // vsl_color  pen 1
    pb.int2(108, 0, 0, h); // vsl_ends   squared
    pb.int1(113, 0xFFFFu16 as i16, h); // vsl_udsty  all ones

    pb.int1(18, 1, h); // vsm_type   1
    pb.pts(19, 0, wk.siz_tab[9], h); // vsm_height DEF_MKHT
    pb.int1(20, 1, h); // vsm_color  pen 1

    pb.int1(23, 1, h); // vsf_interior
    pb.int1(24, 1, h); // vsf_style
    pb.int1(25, 1, h); // vsf_color  pen 1
    pb.int1(104, 1, h); // vsf_perimeter on

    pb.int1(21, 1, h); // vst_font   1
    pb.pts(12, 0, wk.siz_tab[1], h); // vst_height DEF_CHHT
    pb.int1(13, 0, h); // vst_rotation 0
    pb.int1(22, 1, h); // vst_color  pen 1
    pb.int1(106, 0, h); // vst_effects none
    pb.int2(39, 0, 0, h); // vst_alignment

    pb.int1(129, 0, h); // vs_clip    off
}

// ── the reference record ─────────────────────────────────────────────────

pub const REC_SIZE: usize = 1592;
pub const HDR_SIZE: usize = 32;
const OFF_CASEID: usize = 0;
const OFF_OPCODE: usize = 2;
const OFF_RET: usize = 4;
const OFF_CONTRL: usize = 8;
const OFF_INTIN: usize = 32;
const OFF_PTSIN: usize = 288;
const OFF_INTOUT: usize = 800;
const OFF_PTSOUT: usize = 1056;
const OFF_CONTRL_BEFORE: usize = 1568;

fn be16(b: &[u8], off: usize) -> i16 {
    (((b[off] as u16) << 8) | b[off + 1] as u16) as i16
}

/// A call issued before the measured one: it establishes the state the case
/// measures, runs after the reset so it survives it, and is not recorded.
/// Without it a setter case can only prove its own echo, since the reset
/// wipes anything the previous case set.
pub struct Call {
    pub opcode: i16,
    pub sub: i16,
    pub intin: &'static [i16],
    pub ptsin: &'static [i16],
}

/// One case, as the generator emits it. `intin`/`ptsin` are the inputs from
/// the case table; the expected outputs come from the dump.
pub struct Case {
    pub prelude: &'static [Call],
    pub id: u16,
    pub rec: usize,
    pub opcode: i16,
    pub sub: i16,
    pub intin: &'static [i16],
    pub ptsin: &'static [i16],
    pub comment: &'static str,
}

struct Diff {
    field: &'static str,
    index: usize,
    got: i16,
    want: i16,
}

// ── fields the reference cannot decide, and what is checked instead ──────

/// The plane count this harness's workstation reports, and the one the
/// reference machine reported. Everything in [`MACHINE_EXCLUDED`] follows
/// from the pair: the TT was a four-plane palette screen, NewTOS is a
/// sixteen-plane Falcon truecolor one, and several VDI answers are — in
/// EmuTOS's own source — a function of exactly that number.
pub const PLANES: i16 = 16; // `MemBackend::planes`
/// The reference machine's plane count, for the record; the TT ran TT-medium.
pub const REF_PLANES: i16 = 4; // `INQ_TAB[4]`, `EMUTOS14-VDI-r3` cases 117/119
/// `numcolors` = `DEV_TAB[13]` (`vdi_defs.h:98`) here and on the TT.
pub const NUM_COLORS: i16 = 256; // `MemBackend::num_colors`
pub const REF_NUM_COLORS: i16 = 16; // `DEV_TAB` in `EMUTOS14-VDI-r2.LOG`

/// One `INTOUT` slot that must **not** be byte-compared against the dump,
/// because EmuTOS itself computes it from a property of the machine that the
/// two workstations do not share. Every entry is checked instead by
/// [`substitute`], which asserts the EmuTOS *formula* evaluated on this
/// workstation — so the rule is tested even though the number cannot be.
///
/// Nothing is skipped silently: an entry here is a claim that running EmuTOS
/// unchanged on NewTOS's workstation would produce NewTOS's number, and the
/// list below carries, for each one, the `vdi/…:line` that says so.
///
/// * **`v_opnwk`/`v_opnvwk` `INTOUT[13]` — `DEV_TAB[13]`, the colour count.**
///   `DEV_TAB[13] = (v_planes<8) ? (1 << v_planes) : 256`
///   (`vdi_control.c:207`). Sixteen planes → 256; the TT's four → 16.
/// * **`v_opnwk`/`v_opnvwk` `INTOUT[39]` — `DEV_TAB[39]`, the DAC's RGB
///   choice count.** `DEV_TAB[39] = get_palette()` (`vdi_control.c:208`).
///   On VIDEL, a mode that is neither ST-compatible nor 4bpp returns **0**
///   (`bios/screen.c:840-846`); the TT's shifter path returns 4096
///   (`:850-866`). NewTOS derives it from the plane count
///   (`tos-api/src/vdi/workstation.rs` `palette_precision`, V03).
/// * **`vq_extnd(1)` `INTOUT[4]` — `INQ_TAB[4]`.** Literally `v_planes`
///   (`vdi_control.c:210`).
/// * **`vq_extnd(1)` `INTOUT[5]` — `INQ_TAB[5]`, "video lookup table".**
///   `(v_planes == 16 || monitor == MON_MONO) ? 0 : 1`
///   (`vdi_control.c:211-213`).
/// * **`vq_color` `INTOUT[0]` and `vsm_color` `INTOUT[0]`, for a pen the two
///   machines disagree about.** Both range-check against `numcolors`
///   (`vdi_col.c:854-859`; `validate_color_index`, `vdi_control.c:224-230`).
///   A pen in `16..256` is *valid here and invalid on the TT*, so the TT
///   answers with its out-of-range reply (`-1`, resp. the default pen 1) and
///   this workstation answers with the pen. Pens below 16 are valid on both
///   and stay compared (cases 43/44/45, 88); a pen ≥ 256 is invalid on both
///   and stays compared (case 47).
/// * **`v_get_pixel` `INTOUT[0]` and `INTOUT[1]`.** EmuTOS branches the whole
///   reply on `TRUECOLOR_MODE` (`v_planes > 8`, `vdi_defs.h:22`): truecolor
///   answers `INTOUT[0] = 0` with the raw 16-bit screen word in `INTOUT[1]`,
///   palette answers the pixel value and `REV_MAP_COL[pel]`
///   (`vdi_fill.c:1104-1114`). The TT took the palette arm, this workstation
///   takes the truecolor arm, and the two arms share no value.
/// * **`vqin_mode(4)` `INTOUT[0]` with no prelude — the string device's
///   mode.** This one is not about planes; it is about the AES that was
///   running on the reference machine and is not running here. `str_mode`
///   defaults to 0 (REQUEST) in `vdimouse_init` (`vdi_mouse.c:575-578`) and
///   is written only by `vsin_mode` (`vdi_input.c:139`) — but the EmuTOS
///   **AES** calls `vsin_mode(4, 2)` on every keyboard poll, in `gsx_char()`
///   (`aes/gemgsxif.c:459-463`), which `chkkbd()` runs from the dispatcher's
///   scheduler on every pass (`aes/gemdisp.c:157-177`). On the TT that
///   scheduler was ticking behind the dumper, so `str_mode` was 1 and the
///   dump says 1.
///
///   Two things make this harness's 0 the right answer for *this*
///   workstation rather than a NewTOS defect. It runs no AES at all (see
///   [`workstation`]), so nothing performs that write; and it opens the
///   **physical** workstation itself, which is exactly where EmuTOS runs
///   `vdimouse_init` (`vdi_control.c:563`) and resets all four devices to
///   REQUEST — `v_opnvwk`, the call the dumper made, does not. So the
///   sequence replayed here ends at 0 on an EmuTOS VDI too; what cannot be
///   replayed is the AES scheduler ticking concurrently afterwards.
///
///   NewTOS's AES does perform the write: `dsptch()` calls `chkkbd()`
///   (`tos-api/src/aes/mod.rs`), and `tests/aes_reference.rs`'s
///   `aes_dispatch_leaves_the_string_device_in_sample_mode` checks the
///   result against these very dump records (r2 record 16, all record 40)
///   in a binary that *does* run the AES. The VDI half — the default, and
///   the set/read round trip — is checked below and by cases 123 and 125.
pub const MACHINE_EXCLUDED: &[Excluded] = &[
    Excluded { opcode: 1, index: 13, when: None },
    Excluded { opcode: 100, index: 13, when: None },
    Excluded { opcode: 1, index: 39, when: None },
    Excluded { opcode: 100, index: 39, when: None },
    Excluded { opcode: 102, index: 4, when: Some(extnd_mode1) },
    Excluded { opcode: 102, index: 5, when: Some(extnd_mode1) },
    Excluded { opcode: 26, index: 0, when: Some(pen_only_we_accept) },
    Excluded { opcode: 20, index: 0, when: Some(pen_only_we_accept) },
    Excluded { opcode: 105, index: 0, when: None },
    Excluded { opcode: 105, index: 1, when: None },
    Excluded { opcode: 115, index: 0, when: Some(untouched_string_device) },
];

/// An `INTOUT` slot excluded from the byte comparison for [`MACHINE_EXCLUDED`]'s
/// reasons, optionally only for the cases matching `when`.
pub struct Excluded {
    pub opcode: i16,
    pub index: usize,
    pub when: Option<fn(&Case) -> bool>,
}

/// `vq_extnd` mode ≠ 0, the arm that returns `INQ_TAB` (`vdi_control.c:645-658`).
fn extnd_mode1(case: &Case) -> bool {
    case.intin.first().copied().unwrap_or(0) != 0
}

/// A pen this workstation accepts and the reference machine did not:
/// `REF_NUM_COLORS <= pen < NUM_COLORS`.
fn pen_only_we_accept(case: &Case) -> bool {
    let pen = case.intin.first().copied().unwrap_or(0);
    (REF_NUM_COLORS..NUM_COLORS).contains(&pen)
}

/// `vqin_mode(4)` on a VDI that nobody has called `vsin_mode(4, …)` on.
fn untouched_string_device(case: &Case) -> bool {
    case.prelude.is_empty() && case.intin.first().copied() == Some(4)
}

/// Field name marking a [`substitute`] result, so the failure message says
/// what the right-hand column is: the EmuTOS formula, not the dump.
const SUBST: &str = "intout (substitute)";

fn is_excluded(case: &Case, field: &str, index: usize) -> bool {
    field == "intout"
        && MACHINE_EXCLUDED.iter().any(|e| {
            e.opcode == case.opcode && e.index == index && e.when.is_none_or(|f| f(case))
        })
}

/// What is checked *in place of* each excluded field: EmuTOS's own formula,
/// evaluated on this workstation. Failures land in the same diff list as the
/// byte comparisons, so an excluded field is still a way for a case to fail —
/// it is just no longer the reference's number that decides.
fn substitute(out: &mut Vec<Diff>, case: &Case, intout: &[i16], ptsin: &[i16]) {
    for e in MACHINE_EXCLUDED {
        if e.opcode != case.opcode || !e.when.is_none_or(|f| f(case)) {
            continue;
        }
        let got = intout[e.index];
        let expect = match (case.opcode, e.index) {
            // DEV_TAB[13] = (v_planes<8) ? (1<<v_planes) : 256
            (1 | 100, 13) => {
                if PLANES < 8 {
                    1 << PLANES
                } else {
                    256
                }
            }
            // DEV_TAB[39] = get_palette() on a VIDEL (`bios/screen.c:840-846`):
            // 4096 for the 4bpp mode (and the ST-compatible modes, which a
            // plane count cannot express), 0 for every other depth — 1, 2,
            // 8 bpp and truecolor (V03).
            (1 | 100, 39) => {
                if PLANES == 4 {
                    4096
                } else {
                    0
                }
            }
            (102, 4) => PLANES,                                       // INQ_TAB[4] = v_planes
            (102, 5) => i16::from(PLANES != 16),                      // INQ_TAB[5]
            // vq_color: in range → the pen itself, else -1 (`vdi_col.c:854-859`).
            (26, 0) => {
                let pen = case.intin[0];
                if (0..NUM_COLORS).contains(&pen) {
                    pen
                } else {
                    -1
                }
            }
            // vsm_color: `validate_color_index` (`vdi_control.c:224-230`).
            (20, 0) => {
                let pen = case.intin[0];
                if (0..NUM_COLORS).contains(&pen) {
                    pen
                } else {
                    1
                }
            }
            // v_get_pixel under TRUECOLOR_MODE (`vdi_fill.c:1104-1108`):
            // INTOUT[0] = 0, INTOUT[1] = the raw 16-bit screen word. The
            // word is read out of the backend's pixel vector here, not
            // through the two trait methods the implementation uses.
            (105, 0) => 0,
            (105, 1) => {
                let backend = &HOST_CELL.get().expect("workstation() not opened").raster;
                backend.screen_word(ptsin[0], ptsin[1]) as i16
            }
            // vqin_mode: the `vdimouse_init` default (`vdi_mouse.c:575-578`),
            // which this harness's own `v_opnwk` restored (`vdi_control.c:563`).
            (115, 0) => 0,
            _ => unreachable!("no substitute for opcode {} INTOUT[{}]", case.opcode, e.index),
        };
        if got != expect {
            out.push(Diff { field: SUBST, index: e.index, got, want: expect });
        }
    }
}

fn cmp_field(
    out: &mut Vec<Diff>,
    case: &Case,
    field: &'static str,
    got: &[i16],
    rec: &[u8],
    off: usize,
    count: usize,
) {
    for i in 0..count {
        if is_excluded(case, field, i) {
            continue; // checked by `substitute` instead
        }
        let want = be16(rec, off + i * 2);
        if got[i] != want {
            out.push(Diff { field, index: i, got: got[i], want });
        }
    }
}

/// Replay one case and compare every array against the reference, field by
/// field. The input arrays are compared too: a VDI routine may not modify
/// them, and the dump captured them *after* the call precisely so that a
/// violation shows up.
pub fn check(dump: &[u8], case: &Case) {
    let _guard = VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let wk = workstation();
    let mut pb = Pb::new();

    reset_vdi(&mut pb, wk);

    // The case's preludes, in order, exactly as tosdump/core/runner.c issues
    // them: after the reset, before the measured call, never recorded.
    for p in case.prelude {
        pb.contrl.fill(0);
        pb.intin[..p.intin.len()].copy_from_slice(p.intin);
        pb.ptsin[..p.ptsin.len()].copy_from_slice(p.ptsin);
        pb.contrl[0] = p.opcode;
        pb.contrl[1] = (p.ptsin.len() / 2) as i16;
        pb.contrl[3] = p.intin.len() as i16;
        pb.contrl[5] = p.sub;
        pb.contrl[6] = wk.handle;
        pb.call();
    }

    let rec = &dump[HDR_SIZE + case.rec * REC_SIZE..][..REC_SIZE];
    assert_eq!(
        be16(rec, OFF_CASEID) as u16,
        case.id,
        "record {} is case {}, expected case {} -- dump and generated tests disagree",
        case.rec,
        be16(rec, OFF_CASEID),
        case.id
    );

    pb.poison();
    pb.intin[..case.intin.len()].copy_from_slice(case.intin);
    pb.ptsin[..case.ptsin.len()].copy_from_slice(case.ptsin);
    pb.contrl[0] = case.opcode;
    pb.contrl[1] = (case.ptsin.len() / 2) as i16;
    pb.contrl[3] = case.intin.len() as i16;
    pb.contrl[5] = case.sub;
    pb.contrl[6] = wk.handle;

    let before: Vec<i16> = pb.contrl.to_vec();
    let flip_y = pb.call();

    // The reference must have gone in with the same contrl we did; if not,
    // the case table and the dump are out of step and nothing below means
    // anything.
    for i in 0..CONTRL_N {
        assert_eq!(
            before[i],
            be16(rec, OFF_CONTRL_BEFORE + i * 2),
            "case {}: contrl_before[{}] differs -- the dump was taken with a \
             different case table",
            case.id,
            i
        );
    }

    let mut diffs = Vec::new();
    // The trap's `d0` = `flip_y` (`vdi/vdi_asm.S:124-125`). Compared as the
    // WORD the contract defines, not the whole LONG: EmuTOS clears the high
    // half (`moveq #0,d0`), Atari TOS 3.06 does not, so the LONG carries a
    // register leftover that is not part of the contract. Same rule as
    // `tosdump/host` uses (`core/record.c`, field `d0` at `OFF_RET + 2`).
    cmp_field(&mut diffs, case, "d0", &[flip_y], rec, OFF_RET + 2, 1);
    cmp_field(&mut diffs, case, "contrl", &pb.contrl[..], rec, OFF_CONTRL, CONTRL_N);
    cmp_field(&mut diffs, case, "intin", &pb.intin[..], rec, OFF_INTIN, INTIN_N);
    cmp_field(&mut diffs, case, "ptsin", &pb.ptsin[..], rec, OFF_PTSIN, PTSIN_N);
    cmp_field(&mut diffs, case, "intout", &pb.intout[..], rec, OFF_INTOUT, INTOUT_N);
    cmp_field(&mut diffs, case, "ptsout", &pb.ptsout[..], rec, OFF_PTSOUT, PTSOUT_N);
    // The named exclusions, checked by the EmuTOS formula instead of by the
    // reference's number (see `MACHINE_EXCLUDED`).
    substitute(&mut diffs, case, &pb.intout[..], case.ptsin);

    if !diffs.is_empty() {
        let mut msg = format!(
            "case {} (opcode {}, sub {}) -- {}\n  {} field(s) differ from the \
             EmuTOS reference:\n",
            case.id,
            case.opcode,
            case.sub,
            case.comment,
            diffs.len()
        );
        for d in diffs.iter().take(24) {
            // For a substitute the right-hand column is EmuTOS' formula
            // evaluated on this workstation, not the dump -- say so, or the
            // message claims the reference decided something it cannot.
            let source = if d.field == SUBST { "EmuTOS formula" } else { "reference" };
            msg += &format!(
                "    {}[{}]  NewTOS = 0x{:04X} ({})   {} = 0x{:04X} ({})\n",
                d.field, d.index, d.got as u16, d.got, source, d.want as u16, d.want
            );
        }
        if diffs.len() > 24 {
            msg += &format!("    ... and {} more\n", diffs.len() - 24);
        }
        let _ = OFF_OPCODE;
        panic!("{}", msg);
    }
}
