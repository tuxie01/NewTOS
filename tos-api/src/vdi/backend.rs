//! VDI rasterizer backend — the swappable hardware boundary.
//!
//! The VDI layer above (dispatcher, `Vwk` attribute state, validation,
//! clipping, coordinate handling) is **hardware-independent**. Only the
//! actual pixel-touching primitives live behind this trait, so a later
//! NEON / DMA / VideoCore-V3D accelerated implementation can replace the
//! backend without the VDI layer changing at all.
//!
//! Bring-up provides a single software implementation (in the kernel)
//! that draws into the [`hal::framebuffer`] linear buffer. The kernel
//! hands the active backend to `tos-api` through
//! [`crate::host::Host::raster`].
//!
//! Colours cross this boundary as **PEL** (palette indices, see
//! [`Pel`]); the backend owns the palette → device-colour mapping, just
//! as the hardware CLUT would. This matches how the VDI stores colours
//! in the [`Vwk`](crate::vdi::vwk::Vwk).

/// A palette colour index (PEL), as stored in the `Vwk` and passed to
/// the backend. The backend resolves it to a concrete device colour
/// (RGB for the bring-up truecolor framebuffer).
pub type Pel = u8;

/// The VDI writing mode (`wrt_mode`), selecting how a primitive's
/// foreground pixels combine with what is already on screen. Mapped from
/// the **0-based** `wrt_mode` the `Vwk` stores (`vswr_mode` stores
/// `INTIN[0] - 1`; EmuTOS `WM_REPLACE`/`WM_TRANS`/`WM_XOR`/`WM_ERASE` =
/// 0/1/2/3). Spec: `docs/api/vdi/attributes/_mode-clip-color.md`.
///
/// A primitive presents each candidate pixel with a boolean "source bit":
/// for a filled span it is the interior-pattern bit, for a dashed line it
/// is the `LN_MASK` bit, for a glyph it is the font bit. The mode decides
/// what (if anything) is written:
///
/// | mode | source **set** | source **clear** |
/// |------|----------------|------------------|
/// | `Replace`        | foreground | background |
/// | `Transparent`    | foreground | untouched  |
/// | `Xor`            | complement | untouched  |
/// | `RevTransparent` | untouched  | foreground |
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WriteMode {
    /// `WM_REPLACE` (0, the default): foreground on set bits, background
    /// on clear bits — opaque, so it also clears the cell/gap.
    Replace,
    /// `WM_TRANS` (1): foreground on set bits; clear bits untouched.
    Transparent,
    /// `WM_XOR` (2): complement the destination on set bits; clear bits
    /// untouched. Self-inverse (drawing twice restores the original).
    Xor,
    /// `WM_ERASE` (3), reverse-transparent: foreground on **clear** bits;
    /// set bits untouched — the inverse of [`Transparent`](Self::Transparent).
    RevTransparent,
}

impl WriteMode {
    /// Map the **0-based** `wrt_mode` stored in the [`Vwk`](crate::vdi::vwk::Vwk)
    /// to a [`WriteMode`]. Out-of-range values fall back to
    /// [`Replace`](Self::Replace) (`DEF_WRT_MODE`), matching how
    /// `vswr_mode` clamps an out-of-range `INTIN[0]`.
    #[inline]
    pub fn from_wrt_mode(wrt_mode: i16) -> WriteMode {
        match wrt_mode {
            1 => WriteMode::Transparent,
            2 => WriteMode::Xor,
            3 => WriteMode::RevTransparent,
            _ => WriteMode::Replace,
        }
    }
}

/// The set of raster primitives the VDI layer needs from the device.
///
/// Intentionally minimal for the workstation-lifecycle milestone
/// (D1/D2): geometry queries plus a rectangle fill (used by `v_clrwk`).
/// Output primitives (`line`, `span_fill`, `blit`, `glyph`) are added as
/// the corresponding VDI opcodes are implemented (D3+), so the trait
/// never carries an unimplemented method.
pub trait RasterBackend: Sync {
    /// Visible width in device pixels.
    fn width(&self) -> u16;

    /// Visible height in device pixels.
    fn height(&self) -> u16;

    /// Number of simultaneously displayable colours (the VDI pen count as
    /// advertised in `DEV_TAB[13]`; EmuTOS `numcolors`). The Falcon
    /// truecolor profile reports 256 (`vdi_control.c:206`).
    fn num_colors(&self) -> u16;

    /// Number of bit-planes the VDI advertises as `INQ_TAB[4] = v_planes`
    /// (`vdi_control.c:210`). **Decoupled from** [`num_colors`](Self::num_colors):
    /// under Falcon truecolor `v_planes == 16` while the pen count
    /// (`DEV_TAB[13]`) is 256 — the two must not be conflated
    /// (`vdi/vdi_defs.h:22`, `TRUECOLOR_MODE (v_planes > 8)`; they only
    /// coincide in the palette modes, 16 colours = 4 planes). This feeds
    /// `DEV_TAB[35]` (`v_planes == 1`), `INQ_TAB[4]`, and — since the AES
    /// reads `ap_planes`/`gl_nplanes` straight from `INQ_TAB[4]`
    /// (`gemgsxif.c:497-501`) — the AES plane count too.
    fn planes(&self) -> u16;

    /// Set palette entry `index` to the requested RGB, each component in
    /// the VDI's 0..=1000 range (`vs_color`). The backend remembers the
    /// request (for [`query_color`](Self::query_color)) and programs the
    /// device CLUT. Out-of-range indices are ignored.
    fn set_color(&self, index: u16, r: u16, g: u16, b: u16);

    /// Read back the last requested RGB (0..=1000) of palette entry
    /// `index` (`vq_color` "requested" mode). Out-of-range → `(0,0,0)`.
    fn query_color(&self, index: u16) -> (u16, u16, u16);

    /// Point the backend's PEL → device-colour resolution at the given
    /// **active** pseudo-palette for the following draw ops. Under Falcon
    /// truecolor every primitive keeps a plain PEL and resolves it at the
    /// write point through `CUR_WORK->ext->palette` (EmuTOS
    /// `swblit_rect_common16`, `vdi_line.c:459-460`) — not through a global
    /// device CLUT. The VDI dispatcher sets this to the drawing
    /// workstation's own per-`Vwk` pseudo-palette before each colour-
    /// sensitive op (`vdi_col.c:601-614`, `set_color16`).
    ///
    /// Each entry is one 16-bit pixel value `RRRRRGGGGG0BBBBB` (RGB555
    /// packed into a 565 slot, the green LSB / bit 5 always 0 — EmuTOS
    /// `vdi_defs.h:156`, `vdi_col.c:599`); the backend expands each gun
    /// 5→8 bits at the write point (`c8 = (c5<<3)|(c5>>2)`, the VIDEL
    /// truecolor DAC expansion).
    fn set_active_palette(&self, palette: &[u16; 256]);

    /// Resolve a PEL to the **16-bit RGB565 memory-MFDB pixel word** through
    /// the currently [active pseudo-palette](Self::set_active_palette) —
    /// EmuTOS `vrt_cpyfm16`'s `fgcol = palette[info->fg_col]`
    /// (`vdi_raster.c:1234-1236`). This is the value written into a
    /// memory-destination bitmap (a `vrt_cpyfm` memory `d_form`), where the
    /// stored pixels are raw `RRRRRGGGGG0BBBBB` words rather than expanded
    /// framebuffer pixels. The screen-destination path resolves the same PEL
    /// to a device pixel internally (`blit_mono`); this exposes the packed
    /// 16-bit form the off-screen memory buffer needs. `pel` is a `u8`, so it
    /// always indexes the 256-entry active palette in bounds.
    fn pel_to_device16(&self, pel: Pel) -> u16;

    /// Convert a **raw device pixel word** (as returned by
    /// [`get_device_pixel`](Self::get_device_pixel)) to the 16-bit screen
    /// word, the memory-MFDB wireformat (`RRRRRGGGGGOBBBBB`: 5-bit guns,
    /// O = `OVERLAY_BIT`). This is the screen→memory half of the 16↔32
    /// boundary a memory-MFDB copy crosses (`vro_cpyfm` Screen→Memory / AES
    /// `bb_save`): the physical framebuffer is 32-bpp XRGB8888 (an internal
    /// detail per `docs/COMPATIBILITY.md`) while the app-visible memory MFDB
    /// is the 16-bit word. Because the screen is 5-bit-quantised (every VDI
    /// draw resolves through the RGB555-in-565 pseudo-palette), the top 5
    /// bits of each gun recover the exact stored colour, and the backend
    /// keeps the one non-colour bit of the word — `OVERLAY_BIT` (`0x0020`,
    /// `vdi_fill.c:26`), which `set_color16` never sets
    /// (`(r<<11)|(g<<6)|b`, `vdi_col.c:601-614`) but `v_clrwk`'s
    /// `memset(v_bas_ad, 0xff, …)` does (`vdi_control.c:610-618`) — in the
    /// pixel's X byte, so [`rgb565_to_device`](Self::rgb565_to_device)
    /// inverts this losslessly for **every** 16-bit word: the cleared screen
    /// reads back `0xFFFF` and a pen-0 fill `0xFFDF`, the two words a Falcon
    /// reports (VDI-OVL-001, closed by V11/V12, 2026-09-21;
    /// `kernel/src/vdi_backend.rs` `device_to_word`).
    fn device_to_rgb565(&self, argb: u32) -> u16;

    /// Convert a 16-bit screen word (`RRRRRGGGGGOBBBBB`) back to a raw
    /// device pixel word — the memory→screen half of the 16↔32 boundary
    /// (`vro_cpyfm` Memory→Screen / AES `bb_restore`). Each 5-bit gun is
    /// bit-replicated to 8 bits (`c8 = (c5<<3)|(c5>>2)`, the VIDEL truecolor
    /// DAC expansion) and `OVERLAY_BIT` is carried into the X byte. Exact
    /// inverse of [`device_to_rgb565`](Self::device_to_rgb565) on every
    /// word, so a `bb_save`/`bb_restore` round-trip is bit-exact, overlay
    /// bit included (`kernel/src/vdi_backend.rs` `word_to_device`).
    fn rgb565_to_device(&self, px: u16) -> u32;

    /// Read the palette index (PEL) at device pixel `(x, y)`, reverse-
    /// mapping the stored colour to the nearest CLUT entry. Out-of-bounds
    /// → `0`. Used by the **palette-mode** paths only — under truecolor
    /// both `v_get_pixel` and `v_contourfill` work on the raw 16-bit screen
    /// word instead (`vdi_fill.c:670-690,744-767`), because a truecolor
    /// screen holds no palette indices to reverse-map.
    fn read_pixel(&self, x: i16, y: i16) -> Pel;

    /// Fill the **whole visible screen** with the raw 16-bit VDI pixel word
    /// `word`, ignoring any clip rectangle — the truecolor half of
    /// `v_clrwk`, which EmuTOS performs as `memset(v_bas_ad, 0xff, size)`
    /// over the entire framebuffer (`vdi_control.c:600-618`), i.e. the word
    /// `0xFFFF`.
    ///
    /// This deliberately does **not** go through a PEL. `0xFFFF` is not a
    /// pen: `set_color16` never sets the green LSB (`vdi_col.c:601-614`), so
    /// the pseudo-palette's white is `(31<<11)|(31<<6)|31` = `0xFFDF` and the
    /// cleared screen differs from it by `OVERLAY_BIT` (`0x0020`,
    /// `vdi_fill.c:26`) — exactly the bit the truecolor seed fill masks out
    /// of both sides so the two compare equal (`vdi_fill.c:702,709,726,733`).
    /// Nor does the constant follow the palette: EmuTOS clears to `0xFFFF`
    /// however pen 0 has been redefined.
    ///
    /// The default implementation writes the word's device representation
    /// ([`rgb565_to_device`](Self::rgb565_to_device)) with
    /// [`put_device_pixel`](Self::put_device_pixel) and flushes once. A
    /// backend able to fill in bulk should override it.
    fn clear_screen_word(&self, word: u16) {
        let px = self.rgb565_to_device(word);
        let (w, h) = (self.width(), self.height());
        for y in 0..h {
            for x in 0..w {
                self.put_device_pixel(x as i16, y as i16, px);
            }
        }
        self.flush_rect(0, 0, w.saturating_sub(1) as i16, h.saturating_sub(1) as i16);
    }

    /// Fill the rectangle `[x, x+w) × [y, y+h)` (device coordinates,
    /// origin top-left) with palette colour `pel`. The draw is clipped to
    /// `clip` ∩ the visible area; negative / oversized rectangles are
    /// clamped.
    fn fill_rect(&self, x: i16, y: i16, w: i16, h: i16, pel: Pel, clip: ClipRect);

    /// Draw one polyline segment from `(x0, y0)` to `(x1, y1)` with dash-mask
    /// threading — the EmuTOS `polyline` step `clip_line` + `abline`
    /// (`vdi_line.c:1115-1119`, `:2266`). Which pixels, in which order and
    /// with which dash bit is [`crate::vdi::line::abline`]'s; a backend runs
    /// that walk and combines each pixel it hands out into its store, so
    /// every backend draws the same line. In short:
    ///
    /// - `clip` is applied to the **endpoints** (`clip_line`, bounded by the
    ///   screen as `vs_clip` bounds it) and the segment is drawn from the
    ///   clipped points; [`ClipRect::OFF`] (`vs_clip(0)`) draws it unclipped,
    ///   and a pixel past a screen edge goes to the store index EmuTOS
    ///   addresses — the neighbouring row — while one outside the logical
    ///   screen is dropped (`docs/COMPATIBILITY.md`, TASK/V20);
    /// - a **vertical** segment walks from `(x0, y0)` and is never shortened;
    /// - otherwise it is drawn left to right, and with `mode == Xor` and
    ///   `!last` (`LSTLIN`) the far `x` is shortened by one pixel so a
    ///   polyline's shared vertex is not XOR'd twice;
    /// - a **horizontal** segment is screen-aligned: the dash bit is
    ///   `mask & (0x8000 >> (x & 0xf))` at absolute `x`, and `LN_MASK` is
    ///   *not* advanced;
    /// - anything else is `draw_line16`'s Bresenham, the mask rotated one bit
    ///   per pixel.
    ///
    /// `mask` is the 16-bit `LN_MASK` dash pattern (`0xFFFF` = solid).
    /// Returns the mask after the segment (unchanged when the clip rejects
    /// it), so the caller threads it into the next segment — reproducing
    /// EmuTOS' persistent `LN_MASK` across a whole `v_pline` without a
    /// global. `last` marks the final segment of the polyline (a standalone
    /// line is its own last).
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
    ) -> u16;

    /// Draw a standalone one-pixel-wide line from `(x0, y0)` to `(x1, y1)` in
    /// palette colour `pel`. Convenience wrapper over [`abline`](Self::abline)
    /// for callers that draw a single, independent line (AES chrome, text
    /// decoration): a standalone line is its own last segment (`last = true`,
    /// so the XOR `LSTLIN` shortening never applies) and the rotated mask is
    /// discarded. `mask`, `mode` and `clip` behave as for [`abline`](Self::abline).
    #[allow(clippy::too_many_arguments)]
    fn draw_line(
        &self,
        x0: i16,
        y0: i16,
        x1: i16,
        y1: i16,
        pel: Pel,
        mode: WriteMode,
        clip: ClipRect,
        mask: u16,
    ) {
        self.abline(x0, y0, x1, y1, pel, mode, clip, mask, true);
    }

    /// Fill the rectangle `[x, x+w) × [y, y+h)` with the monochrome
    /// pattern in [`PatternFill`], expanded into colour per its
    /// [`WriteMode`](PatternFill::mode). Clipped to `clip` ∩ the visible
    /// area.
    fn fill_pattern(&self, x: i16, y: i16, w: i16, h: i16, fill: PatternFill, clip: ClipRect);

    /// Blit a `cell_w`-wide × `cell_h`-tall monochrome glyph with its
    /// top-left at `(x, y)` in colour `fg`. `glyph` holds `cell_h` rows of
    /// `bytes_per_row = ceil(cell_w/8)` bytes each (leftmost byte first,
    /// bit 7 = `0x80` = leftmost pixel); only the leftmost `cell_w` columns
    /// are used (the 6×6 font packs 6 px into bits 7..2). Cells wider than
    /// 8 px occur when a text effect (outline / skew) widens the base cell;
    /// for `cell_w <= 8` this is one byte per row.
    /// The font bit is the primitive's "source set". `mode` selects the
    /// raster combination — `Transparent` draws only set bits (the common
    /// text case), `Replace` also clears the `cell_w`×`cell_h` cell
    /// background (VDI background pen 0), `Xor` complements, and
    /// `RevTransparent` draws the clear bits. Clipped to `clip` ∩ the
    /// visible area. `glyph.len()` must be `>= cell_h * bytes_per_row`.
    #[allow(clippy::too_many_arguments)]
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
    );

    /// Blit a fixed 8-wide × 16-tall monochrome glyph — the system-font
    /// cell size used by the AES/desktop/console text paths. Forwards to
    /// [`draw_glyph_sized`](Self::draw_glyph_sized) with `(cell_w, cell_h)
    /// = (8, 16)`.
    #[inline]
    fn draw_glyph(&self, x: i16, y: i16, glyph: &[u8; 16], fg: Pel, mode: WriteMode, clip: ClipRect) {
        self.draw_glyph_sized(x, y, glyph, 8, 16, fg, mode, clip);
    }

    /// Blit a monochrome source ([`MonoSrc`]) into the screen rectangle
    /// `dst`, expanding it to colour per the VDI [`WriteMode`]
    /// (`vrt_cpyfm`'s `MD_*` codes). A source bit is the primitive's
    /// "source set"; the mode decides what lands on set/clear bits:
    ///
    /// | mode | source **set** (1) | source **clear** (0) |
    /// |------|--------------------|----------------------|
    /// | `Replace`        | `fg` | `bg` |
    /// | `Transparent`    | `fg` | untouched |
    /// | `Xor`            | complement | untouched |
    /// | `RevTransparent` | untouched | `bg` |
    ///
    /// `RevTransparent` maps EmuTOS' `MD_ERASE`: the 0-bits take the
    /// **background** colour (`INTIN[2]`), the 1-bits are left alone.
    /// Clipped to `clip` ∩ the visible area.
    fn blit_mono(&self, dst: Rect, src: MonoSrc, fg: Pel, bg: Pel, mode: WriteMode, clip: ClipRect);

    /// Copy a screen rectangle `src` to the screen position `(dx, dy)`
    /// applying one of the 16 BitBlt logic ops `op` (`D' = op(S, D)`, the
    /// `vro_cpyfm` `INTIN[0]` table with `PAT_FLAG` already stripped): e.g.
    /// `0`=`0`, `3`=`S` (replace), `5`=`D` (no-op), `6`=`S ^ D` (XOR),
    /// `7`=`S | D` (OR), `15`=`~0`. The op acts literally on the packed
    /// pixel word — the screen-to-screen case of `vro_cpyfm`. The
    /// destination write is clipped to `clip` ∩ the visible area.
    fn copy_rect(&self, src: Rect, dx: i16, dy: i16, op: u8, clip: ClipRect);

    /// Read the **raw device pixel word** (the packed value actually stored
    /// in the framebuffer — XRGB8888 for the bring-up truecolor screen) at
    /// device coordinate `(x, y)`. Out-of-bounds → `0`.
    ///
    /// Unlike [`read_pixel`](Self::read_pixel), which reverse-maps to the
    /// nearest CLUT index (lossy), this returns the exact stored colour.
    /// It is the screen side of a memory-MFDB copy (`vro_cpyfm`
    /// Screen→Memory, i.e. AES `bb_save`): the saved bitmap must reproduce
    /// the screen bit-for-bit on restore, which a PEL round-trip cannot
    /// guarantee for non-palette content.
    fn get_device_pixel(&self, x: i16, y: i16) -> u32;

    /// Write the **raw device pixel word** at `(x, y)`; out-of-bounds is
    /// ignored. Writes made this way are not flushed to the point of
    /// coherency — the caller batches a region and calls
    /// [`flush_rect`](Self::flush_rect) once, so the display DMA (a
    /// separate master beyond the CPU caches on real hardware) sees them.
    /// This is the screen side of `vro_cpyfm` Memory→Screen (AES
    /// `bb_restore`).
    fn put_device_pixel(&self, x: i16, y: i16, argb: u32);

    /// Read a horizontal run of screen pixels as **16-bit screen words**:
    /// `out[i] = device_to_rgb565(get_device_pixel(x + i, y))` for every
    /// `i < out.len()`, including the out-of-bounds rule — a position off the
    /// screen (or beyond the `i16` coordinate range) reads as
    /// `device_to_rgb565(0)`. The default implementation is exactly that
    /// per-pixel definition.
    ///
    /// This is the row form of the screen side of `vro_cpyfm` Screen→Memory
    /// and of the destination read of Memory→Screen (GUI07): one call per
    /// row instead of two dynamically dispatched calls per pixel, so a
    /// backend with direct framebuffer access can fetch base, stride and
    /// bounds once per run. An override must return the identical words.
    fn read_row_rgb565(&self, x: i16, y: i16, out: &mut [u16]) {
        for (i, o) in out.iter_mut().enumerate() {
            let xi = x as i32 + i as i32;
            let px = if xi > i16::MAX as i32 { 0 } else { self.get_device_pixel(xi as i16, y) };
            *o = self.device_to_rgb565(px);
        }
    }

    /// Write a horizontal run of **16-bit screen words** to the screen:
    /// `put_device_pixel(x + i, y, rgb565_to_device(src[i]))` for every
    /// `i < src.len()`; positions off the screen are ignored. Like
    /// [`put_device_pixel`](Self::put_device_pixel) it does not flush — the
    /// caller batches a region and calls [`flush_rect`](Self::flush_rect)
    /// once. The default implementation is exactly that per-pixel definition;
    /// an override must store the identical device words (GUI07, the row
    /// form of the `vro_cpyfm` Memory→Screen write).
    fn write_row_rgb565(&self, x: i16, y: i16, src: &[u16]) {
        for (i, &word) in src.iter().enumerate() {
            let xi = x as i32 + i as i32;
            if xi > i16::MAX as i32 {
                break; // off the screen, and so is every later position
            }
            self.put_device_pixel(xi as i16, y, self.rgb565_to_device(word));
        }
    }

    /// Flush (clean to the point of coherency) the inclusive device
    /// rectangle `[x0..=x1] × [y0..=y1]`, so writes made via
    /// [`put_device_pixel`](Self::put_device_pixel) become visible to the
    /// display controller. Coordinates are clamped to the screen; a no-op
    /// on a Non-Cacheable framebuffer.
    fn flush_rect(&self, x0: i16, y0: i16, x1: i16, y1: i16);
}

/// A rectangle in device coordinates (top-left + size).
#[derive(Clone, Copy)]
pub struct Rect {
    /// Left edge.
    pub x: i16,
    /// Top edge.
    pub y: i16,
    /// Width in pixels.
    pub w: i16,
    /// Height in pixels.
    pub h: i16,
}

/// The raster words of a [`MonoSrc`], held as what they are on the wire: a
/// run of **bytes** carrying host-order 16-bit words, read through
/// [`word`](Self::word), which imposes no alignment.
///
/// A raster reaches the blit as a bare pointer out of a resource or an
/// application `MFDB` — `ICONBLK.ib_pmask`/`ib_pdata`, `BITBLK.bi_pdata`,
/// `MFDB.fd_addr` — and nothing on that path can promise the pointer is even:
///
/// * A classic Atari `.rsc` keeps its rasters in the separate, word-aligned
///   `rsh_imdata` section, each an even number of bytes, so EmuTOS never meets
///   an odd one and checks nothing for it: `gr_gblt(WORD *pimage, …)`
///   (`aes/gemgraf.c:631`) just dereferences. The guarantee is the *file
///   format's*, not the renderer's.
/// * An application under no such constraint hands us one anyway: it may build
///   an `ICONBLK` in its own memory, relocate its own resource (TeraDesk's
///   `xrsrc_load`, which never passes through our `rsrc_load`), or point
///   `fd_addr` at an odd address.
///
/// Typing the source as `&[u16]` would impose an alignment we cannot enforce:
/// `slice::from_raw_parts` aborts a debug build on an odd pointer ("unsafe
/// precondition(s) violated") while a release build performs the access anyway
/// with the check compiled out — the same input ending two different ways, the
/// release one committing UB. Reading it as bytes removes the precondition
/// instead of gambling on it.
///
/// It is free on AArch64: an unaligned 16-bit load of Normal memory is the same
/// single `LDRH` as an aligned one, because `SCTLR_EL1.A` is 0
/// (`kernel/src/arch/aarch64/boot.rs` `SCTLR_EL1_INIT = 0x30D0_0800`; ARM ARM
/// D19.2 "SCTLR_EL1", bit `A`) — which matters, because this is the blit the
/// graphics benchmark measures.
#[derive(Clone, Copy)]
pub struct MonoBits<'a> {
    bytes: &'a [u8],
}

impl<'a> MonoBits<'a> {
    /// View the `words` 16-bit words at `addr` as a raster source, whatever
    /// the alignment of `addr`.
    ///
    /// # Safety
    /// `addr` must be non-null and point at `words * 2` readable bytes that
    /// stay valid and unaliased-for-writing for `'a`. Alignment is **not**
    /// required — that is the point of this type.
    #[inline]
    pub unsafe fn from_raw(addr: *const u8, words: usize) -> MonoBits<'a> {
        // `saturating_mul`: a caller-supplied word count is derived from
        // resource fields, so cap rather than wrap the byte length. Every
        // read is bounds-checked against it either way.
        // SAFETY: the caller guarantees `words * 2` readable bytes at `addr`;
        // `u8` has no alignment requirement, so nothing else is imposed.
        MonoBits { bytes: unsafe { core::slice::from_raw_parts(addr, words.saturating_mul(2)) } }
    }

    /// Raster word `idx`, or `None` past the end of the source (the blit
    /// treats a short raster as clear bits, as it did when this was a slice
    /// index).
    #[inline]
    pub fn word(&self, idx: usize) -> Option<u16> {
        let off = idx.checked_mul(2)?;
        let pair = self.bytes.get(off..)?.get(..2)?;
        // SAFETY: `pair` is exactly two readable bytes of the source, and
        // `read_unaligned` requires no alignment of its pointer.
        Some(unsafe { pair.as_ptr().cast::<u16>().read_unaligned() })
    }
}

/// A monochrome blit source (one plane). `bits` is the raster as 16-bit
/// host-order words, `stride_words` words per row; `(x, y)` is
/// the top-left of the sub-rectangle to read. Bit `0x8000 >> (col & 15)`
/// of word `col/16` is the leftmost pixel.
#[derive(Clone, Copy)]
pub struct MonoSrc<'a> {
    /// The source raster words.
    pub bits: MonoBits<'a>,
    /// Words per raster row.
    pub stride_words: u16,
    /// Source sub-rectangle left.
    pub x: i16,
    /// Source sub-rectangle top.
    pub y: i16,
}

/// A monochrome fill source: a pattern expanded into colour per a
/// [`WriteMode`]. The `pattern` is a run of 16-bit rows whose length is a
/// power of two; scanline `yy` uses row `pattern[yy & (len-1)]`, pixel
/// `xx` uses bit `0x8000 >> (xx & 0xF)` (patterns are screen-aligned to
/// absolute device coordinates). The pattern bit is the primitive's
/// "source set"; [`mode`](Self::mode) decides what is written for set and
/// clear bits (see [`WriteMode`]).
#[derive(Clone, Copy)]
pub struct PatternFill<'a> {
    /// Colour for the foreground pixels (see [`WriteMode`]).
    pub fg: Pel,
    /// Colour for the background pixels (used only by
    /// [`WriteMode::Replace`]).
    pub bg: Pel,
    /// The pattern rows (length a power of two).
    pub pattern: &'a [u16],
    /// The raster combination applied to the pattern.
    pub mode: WriteMode,
}

/// Inclusive clipping rectangle in device coordinates, as carried by the
/// workstation. Passed to line drawing so the backend clips without
/// reaching back into the `Vwk`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ClipRect {
    /// Left edge (inclusive).
    pub xmn: i16,
    /// Top edge (inclusive).
    pub ymn: i16,
    /// Right edge (inclusive).
    pub xmx: i16,
    /// Bottom edge (inclusive).
    pub ymx: i16,
}

impl ClipRect {
    /// Clipping **off** for a line — `vs_clip(0)`, EmuTOS `vwk->clip == 0`.
    /// [`RasterBackend::abline`] given this rectangle draws the segment
    /// unclipped (`polyline` skips `clip_line`, `vdi/vdi_line.c:1115-1116`);
    /// see [`crate::vdi::line::abline`] for where its pixels go. Every other
    /// primitive treats it as the rectangle it spells, which its own clamp
    /// to the screen reduces to the whole device.
    pub const OFF: ClipRect = ClipRect { xmn: i16::MIN, ymn: i16::MIN, xmx: i16::MAX, ymx: i16::MAX };

    /// The clip rectangle covering the whole `w × h` device.
    pub fn full(w: u16, h: u16) -> ClipRect {
        ClipRect {
            xmn: 0,
            ymn: 0,
            xmx: w.saturating_sub(1) as i16,
            ymx: h.saturating_sub(1) as i16,
        }
    }

    /// Intersect with the device bounds `w × h`, returning the inclusive
    /// pixel rectangle `(x0, y0, x1, y1)` to draw into, or `None` if the
    /// result is empty. The backend calls this to honour both the
    /// workstation clip and the screen edges in one step.
    pub fn clamp_to(&self, w: u16, h: u16) -> Option<(i32, i32, i32, i32)> {
        let x0 = (self.xmn as i32).max(0);
        let y0 = (self.ymn as i32).max(0);
        let x1 = (self.xmx as i32).min(w as i32 - 1);
        let y1 = (self.ymx as i32).min(h as i32 - 1);
        if x0 <= x1 && y0 <= y1 {
            Some((x0, y0, x1, y1))
        } else {
            None
        }
    }
}
