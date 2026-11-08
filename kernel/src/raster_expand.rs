//! NEON expansion of 1-bit sources into 32-bit pixels: text glyphs
//! (`RasterBackend::draw_glyph_sized`) and the mono blit of `vrt_cpyfm`,
//! AES icons and images (`RasterBackend::blit_mono`) — raster kernel 2
//! (ADR-036, TASK/V50), on the frame of kernel 1 ([`crate::raster_kernel`],
//! TASK/V49).
//!
//! # Interface
//!
//! One call expands one rectangle. [`expand`] takes the destination as kernel
//! 1 does — a [`Surface`] and an [`Area`] already clipped to the store — and
//! the source as [`Bits`]: a pointer to the first source unit of row 0, the
//! row pitch in bytes, the bit of row 0 that lands on the area's left column
//! (any value, counted MSB-first through the units), and the unit: bytes with
//! bit 7 leftmost (glyph rows of `ceil(cell_w / 8)` bytes) or 16-bit
//! host-order words with bit 15 leftmost, at any alignment (mono rasters,
//! `MonoBits`). [`Expand`] carries the write mode and the two device pixels.
//!
//! [`glyph`] and [`mono`] are the backend's two callers turned into plain
//! functions, so the host tests cover them whole: from the clipped box the
//! backend computes, they find the source row, unit and bit the box starts
//! at, check that every unit the box reads lies in the source, and call
//! [`expand`]. They return `false` and draw nothing when they cannot take a
//! call exactly as the scalar path would (a source shorter than the box, a
//! negative source origin, a source inside the destination rows); the
//! backend then runs its scalar loop, which stays the reference.
//!
//! # Rules the assembler keeps (ADR-036)
//!
//! Those of [`crate::raster_kernel`] (module docs), unchanged: integer and
//! bitwise SIMD only (`dup`, `cmtst`, `bsl`, `bit`, `eor`, `mov`, `ext`,
//! loads and stores; `FPCR`/`FPSR` untouched), every V register used saved on
//! entry and restored before the `asm!` block ends, FP/SIMD assembly switched
//! back off at the block's end on the kernel target. The kernels are leaves
//! called from the same backend methods on the same trap path as kernel 1,
//! so the finding of TASK/V49 ("FP-Kontext") holds as it stands: nothing can
//! switch the EL0 FP/SIMD context while one runs.
//!
//! # Pixel semantics
//!
//! Exactly the scalar path the kernel replaces (`VdiScreen::combine`,
//! `kernel/src/vdi_backend.rs`): source bit set at a pixel →
//!
//! | mode | bit set | bit clear |
//! |---|---|---|
//! | `Replace` | `fg` | `bg` |
//! | `Transparent` | `fg` | untouched |
//! | `Xor` | `!dst` | untouched |
//! | `RevTransparent` | untouched | `fg` |
//!
//! `vrt_cpyfm`'s `MD_ERASE` paints the **background** colour on the clear
//! bits; [`mono_colors`] hands it over in the `fg` slot, as the backend did
//! before. [`expand_reference`], [`glyph_reference`] and [`mono_reference`]
//! are the scalar paths on a plain store (host builds only); the host tests
//! (`tos-api/tests/raster_kernel_expand.rs`) run the kernel against them bit
//! for bit (ADR-036 point 4).
//!
//! # Technique
//!
//! A row is a bit stream read unit by unit into a 64-bit accumulator, left
//! aligned: the first unit shifted past the start bit, then one more unit
//! whenever fewer bits are held than the pixels still due, so no unit beyond
//! the row's last pixel is ever read. Each 16 pixels take the top 16 bits,
//! spread them into 16 lane masks (`dup` + `cmtst` against `0x8000 >> i`, as
//! kernel 1 does with a pattern word) and combine one 64-byte block: `Replace`
//! stores `bsl`(fg, bg), the others load, combine (`bit` / `eor`) and store; a
//! block whose 16 bits change nothing (`Transparent`/`Xor` all clear,
//! `RevTransparent` all set) is skipped, which is most of a glyph cell. The
//! last 1..15 pixels go out 8/4/2/1 at a time with the masks slid down, as in
//! kernel 1. Read for technique only, not copied (ADR-035, ADR-034): fVDI
//! walks a one-bit mask through each source word per pixel, one routine per
//! write mode, the first mask from `x & 15`
//! (`references/fvdi/fvdi/drivers/16_bit/16b_exp.c:33-347`, source address and
//! row skip `:365-366`); MagiC's 16M driver rotates the first source word past
//! the leading pixels, counts the pixels of the first word separately and
//! shifts each word out bit by bit, one loop per mode, and swaps the colours
//! for `MD_ERASE` text (`references/magicmac/kernel/vdi/drivers/off16m.s:2565-2583`,
//! `:2621-2700`, row output `:4080-4170`); TOS 4.04's VDI addresses the source
//! word of `SOURCEX >> 4` with the bit offset `SOURCEX & 15`
//! (`references/tos4x/pixvdi/sources/spxtxtbl.s:1967-1974`, tag `build/tos404`).

use tos_api::vdi::backend::{MonoSrc, WriteMode};

use crate::raster_kernel::{Area, Surface, VSave, LANE_BITS};

/// The unit a 1-bit source is stored in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    /// Bytes, bit 7 (`0x80`) = leftmost pixel: glyph rows.
    Byte,
    /// 16-bit host-order words at any alignment, bit 15 (`0x8000`) =
    /// leftmost pixel: mono rasters (`MonoBits`).
    Word,
}

/// A 1-bit source of [`expand`].
#[derive(Clone, Copy, Debug)]
pub struct Bits {
    /// Row 0 of the source.
    pub base: *const u8,
    /// Bytes from one source row to the next (`0`: every row reads row 0).
    pub stride_bytes: usize,
    /// The bit of row 0 that lands on the area's left column, counted from
    /// `base` MSB-first through the units (any value).
    pub bit: usize,
    /// What the rows are made of.
    pub unit: Unit,
}

/// How a source bit combines with the destination.
#[derive(Clone, Copy, Debug)]
pub struct Expand {
    /// The write mode (module docs table).
    pub mode: WriteMode,
    /// Device pixel on set bits (`RevTransparent`: on clear bits).
    pub fg: u32,
    /// Device pixel on clear bits (`Replace` only).
    pub bg: u32,
}

/// The assembler switch back after a kernel, as in [`crate::raster_kernel`].
#[cfg(target_os = "none")]
macro_rules! simd_off {
    () => {
        ".arch armv8-a+nofp+nosimd"
    };
}
#[cfg(not(target_os = "none"))]
macro_rules! simd_off {
    () => {
        ""
    };
}

/// Row prologue: the destination and source cursors, the pixel count, and
/// the first source unit into the accumulator, shifted past the start bit
/// (`{acc}` left aligned, `{nb}` = valid bits). `$ld` loads one unit into
/// `{u}` and steps `{q}`, `$top` = 64 - unit bits, `$ub` = unit bits.
macro_rules! row_start {
    ($ld:literal, $top:literal, $ub:literal) => {
        concat!(
            "mov {p}, {dst}\n",
            "mov {q}, {src}\n",
            "mov {n}, {w}\n",
            $ld, "\n",
            "lsl {acc}, {u}, #", $top, "\n",
            "lsl {acc}, {acc}, {off}\n",
            "mov {nb}, #", $ub, "\n",
            "sub {nb}, {nb}, {off}",
        )
    };
}

/// The next 16 source bits into `{t}` (bit 15 = the next pixel): refill the
/// accumulator unit by unit while it holds fewer bits than the pixels still
/// due (`min({n}, 16)`), then take its top 16 bits.
macro_rules! next_word {
    ($ld:literal, $top:literal, $ub:literal) => {
        concat!(
            "cmp {n}, #16\n",
            "mov {t}, #16\n",
            "csel {t}, {n}, {t}, lo\n",
            "4:\n",
            "cmp {nb}, {t}\n",
            "b.hs 5f\n",
            $ld, "\n",
            "lsl {u}, {u}, #", $top, "\n",
            "lsr {u}, {u}, {nb}\n",
            "orr {acc}, {acc}, {u}\n",
            "add {nb}, {nb}, #", $ub, "\n",
            "b 4b\n",
            "5:\n",
            "lsr {t}, {acc}, #48\n",
            "lsl {acc}, {acc}, #16\n",
            "sub {nb}, {nb}, #16",
        )
    };
}

/// Lane masks of `{t}`: `v0..v3` lane = all ones where the pixel's source bit
/// is set (`v4..v7` = [`LANE_BITS`], `v10` = scratch).
macro_rules! word_masks {
    () => {
        concat!(
            "dup v10.4s, {t:w}\n",
            "cmtst v0.4s, v10.4s, v4.4s\n",
            "cmtst v1.4s, v10.4s, v5.4s\n",
            "cmtst v2.4s, v10.4s, v6.4s\n",
            "cmtst v3.4s, v10.4s, v7.4s",
        )
    };
}

/// One combine of destination register `$d` with mask register `$m`
/// (`v8` = fg), per write mode.
macro_rules! combine {
    // Transparent / RevTransparent: fg where the mask is set (`BIT`).
    (bit, $d:literal, $m:literal) => {
        concat!("bit ", $d, ".16b, v8.16b, ", $m, ".16b")
    };
    // XOR: complement the destination where the mask is set.
    (eor, $d:literal, $m:literal) => {
        concat!("eor ", $d, ".16b, ", $d, ".16b, ", $m, ".16b")
    };
}

/// The `Replace` kernel for source unit `$ld`/`$top`/`$ub` (see
/// [`row_start!`]): fg on set bits, bg on clear bits, stores only.
///
/// V registers: `v0..v3` masks, then pixels; `v4..v7` lane bits, `v8` fg,
/// `v9` bg, `v10` source word — saved and restored.
macro_rules! replace_kernel {
    ($ld:literal, $top:literal, $ub:literal, $dst:ident, $stride:ident, $src:ident, $sstride:ident,
     $off:ident, $w:ident, $h:ident, $fg:ident, $bg:ident, $save:ident, $bits:ident) => {
        core::arch::asm!(
            ".arch armv8-a+simd",
            "stp q0, q1, [{save}, #0]",
            "stp q2, q3, [{save}, #32]",
            "stp q4, q5, [{save}, #64]",
            "stp q6, q7, [{save}, #96]",
            "stp q8, q9, [{save}, #128]",
            "str q10, [{save}, #160]",
            "ld1 {{v4.4s, v5.4s, v6.4s, v7.4s}}, [{bits}]",
            "dup v8.4s, {fg:w}",
            "dup v9.4s, {bg:w}",
            // ── one row ──
            "2:",
            row_start!($ld, $top, $ub),
            // ── 16 pixels (or the 1..15 of the tail) per pass ──
            "3:",
            next_word!($ld, $top, $ub),
            word_masks!(),
            "bsl v0.16b, v8.16b, v9.16b",
            "bsl v1.16b, v8.16b, v9.16b",
            "bsl v2.16b, v8.16b, v9.16b",
            "bsl v3.16b, v8.16b, v9.16b",
            "cmp {n}, #16",
            "b.lo 6f",
            "st1 {{v0.4s, v1.4s, v2.4s, v3.4s}}, [{p}], #64",
            "subs {n}, {n}, #16",
            "b.ne 3b",
            "b 9f",
            // Tail: `n` = 1..15 pixels; 8, 4, 2, 1, sliding the pixels down.
            "6:",
            "tbz {n}, #3, 7f",
            "st1 {{v0.4s, v1.4s}}, [{p}], #32",
            "mov v0.16b, v2.16b",
            "mov v1.16b, v3.16b",
            "7:",
            "tbz {n}, #2, 8f",
            "st1 {{v0.4s}}, [{p}], #16",
            "mov v0.16b, v1.16b",
            "8:",
            "tbz {n}, #1, 22f",
            "st1 {{v0.2s}}, [{p}], #8",
            "ext v0.16b, v0.16b, v0.16b, #8",
            "22:",
            "tbz {n}, #0, 9f",
            "st1 {{v0.s}}[0], [{p}]",
            // ── next row ──
            "9:",
            "add {dst}, {dst}, {stride}",
            "add {src}, {src}, {sstride}",
            "subs {h}, {h}, #1",
            "b.ne 2b",
            "ldp q0, q1, [{save}, #0]",
            "ldp q2, q3, [{save}, #32]",
            "ldp q4, q5, [{save}, #64]",
            "ldp q6, q7, [{save}, #96]",
            "ldp q8, q9, [{save}, #128]",
            "ldr q10, [{save}, #160]",
            simd_off!(),
            dst = inout(reg) $dst => _,
            stride = in(reg) $stride,
            src = inout(reg) $src => _,
            sstride = in(reg) $sstride,
            off = in(reg) $off,
            w = in(reg) $w,
            h = inout(reg) $h => _,
            fg = in(reg) $fg,
            bg = in(reg) $bg,
            save = in(reg) $save,
            bits = in(reg) $bits,
            p = out(reg) _,
            q = out(reg) _,
            n = out(reg) _,
            acc = out(reg) _,
            nb = out(reg) _,
            t = out(reg) _,
            u = out(reg) _,
            options(nostack),
        )
    };
}

/// The read-modify-write kernel (`Transparent`, `Xor`, `RevTransparent`) for
/// source unit `$ld`/`$top`/`$ub`: `$op` is the [`combine!`] arm, `$pre`
/// prepares the source word (`RevTransparent` complements it, so its masks
/// select the clear bits). A word that is `0` after `$pre` leaves its pixels
/// untouched and is skipped.
///
/// V registers: `v0..v3` masks, `v4..v7` lane bits, `v8` fg, `v10` source
/// word, `v11..v14` destination pixels — saved and restored, `v9` unused.
macro_rules! rmw_kernel {
    ($op:ident, $pre:literal, $ld:literal, $top:literal, $ub:literal, $dst:ident, $stride:ident,
     $src:ident, $sstride:ident, $off:ident, $w:ident, $h:ident, $fg:ident, $save:ident, $bits:ident) => {
        core::arch::asm!(
            ".arch armv8-a+simd",
            "stp q0, q1, [{save}, #0]",
            "stp q2, q3, [{save}, #32]",
            "stp q4, q5, [{save}, #64]",
            "stp q6, q7, [{save}, #96]",
            "stp q8, q10, [{save}, #128]",
            "stp q11, q12, [{save}, #160]",
            "stp q13, q14, [{save}, #192]",
            "ld1 {{v4.4s, v5.4s, v6.4s, v7.4s}}, [{bits}]",
            "dup v8.4s, {fg:w}",
            // ── one row ──
            "2:",
            row_start!($ld, $top, $ub),
            // ── 16 pixels per pass ──
            "3:",
            next_word!($ld, $top, $ub),
            $pre,
            "cmp {n}, #16",
            "b.lo 6f",
            "cbz {t:w}, 23f",
            word_masks!(),
            "ld1 {{v11.4s, v12.4s, v13.4s, v14.4s}}, [{p}]",
            combine!($op, "v11", "v0"),
            combine!($op, "v12", "v1"),
            combine!($op, "v13", "v2"),
            combine!($op, "v14", "v3"),
            "st1 {{v11.4s, v12.4s, v13.4s, v14.4s}}, [{p}], #64",
            "subs {n}, {n}, #16",
            "b.ne 3b",
            "b 9f",
            // A word that changes nothing: step over its 16 pixels.
            "23:",
            "add {p}, {p}, #64",
            "subs {n}, {n}, #16",
            "b.ne 3b",
            "b 9f",
            // Tail: `n` = 1..15 pixels; 8, 4, 2, 1, sliding the masks down.
            "6:",
            "cbz {t:w}, 9f",
            word_masks!(),
            "tbz {n}, #3, 7f",
            "ld1 {{v11.4s, v12.4s}}, [{p}]",
            combine!($op, "v11", "v0"),
            combine!($op, "v12", "v1"),
            "st1 {{v11.4s, v12.4s}}, [{p}], #32",
            "mov v0.16b, v2.16b",
            "mov v1.16b, v3.16b",
            "7:",
            "tbz {n}, #2, 8f",
            "ld1 {{v11.4s}}, [{p}]",
            combine!($op, "v11", "v0"),
            "st1 {{v11.4s}}, [{p}], #16",
            "mov v0.16b, v1.16b",
            "8:",
            "tbz {n}, #1, 22f",
            "ld1 {{v11.2s}}, [{p}]",
            combine!($op, "v11", "v0"),
            "st1 {{v11.2s}}, [{p}], #8",
            "ext v0.16b, v0.16b, v0.16b, #8",
            "22:",
            "tbz {n}, #0, 9f",
            "ld1 {{v11.s}}[0], [{p}]",
            combine!($op, "v11", "v0"),
            "st1 {{v11.s}}[0], [{p}]",
            // ── next row ──
            "9:",
            "add {dst}, {dst}, {stride}",
            "add {src}, {src}, {sstride}",
            "subs {h}, {h}, #1",
            "b.ne 2b",
            "ldp q0, q1, [{save}, #0]",
            "ldp q2, q3, [{save}, #32]",
            "ldp q4, q5, [{save}, #64]",
            "ldp q6, q7, [{save}, #96]",
            "ldp q8, q10, [{save}, #128]",
            "ldp q11, q12, [{save}, #160]",
            "ldp q13, q14, [{save}, #192]",
            simd_off!(),
            dst = inout(reg) $dst => _,
            stride = in(reg) $stride,
            src = inout(reg) $src => _,
            sstride = in(reg) $sstride,
            off = in(reg) $off,
            w = in(reg) $w,
            h = inout(reg) $h => _,
            fg = in(reg) $fg,
            save = in(reg) $save,
            bits = in(reg) $bits,
            p = out(reg) _,
            q = out(reg) _,
            n = out(reg) _,
            acc = out(reg) _,
            nb = out(reg) _,
            t = out(reg) _,
            u = out(reg) _,
            options(nostack),
        )
    };
}

/// Expand `src` into `area` of `surface` per `ex` — the expansion kernel
/// (TASK/V50 part 1). Pixel `(area.x + c, area.y + r)` takes source bit
/// `src.bit + c` of row `r` (module docs table). An empty area draws nothing.
///
/// # Safety
///
/// Every pixel `surface.base + row * surface.stride_px + col` for
/// `row in area.y..area.y + area.h`, `col in area.x..area.x + area.w` must
/// be valid for reads and writes of a `u32`; for every `r < area.h` the units
/// holding bits `src.bit .. src.bit + area.w` of row `r` (at
/// `src.base + r * src.stride_bytes`) must be readable; the source must not
/// lie in the destination rows; nothing else may access either during the
/// call.
pub unsafe fn expand(surface: Surface, area: Area, src: Bits, ex: &Expand) {
    if area.w == 0 || area.h == 0 {
        return;
    }
    // SAFETY: the caller guarantees the area lies in the store (contract).
    let dst = unsafe { surface.base.add(area.y * surface.stride_px + area.x) };
    let stride = surface.stride_px * 4;
    let (ubits, ubytes) = match src.unit {
        Unit::Byte => (8, 1),
        Unit::Word => (16, 2),
    };
    // The unit holding the start bit, and the start bit inside it.
    // SAFETY: that unit holds a bit the caller guarantees readable.
    let src_p = unsafe { src.base.add(src.bit / ubits * ubytes) };
    let off = src.bit % ubits;
    let sstride = src.stride_bytes;
    let w = area.w;
    let h = area.h;
    let fg = ex.fg;
    let bg = ex.bg;
    // Written by the asm before it is read; never read by Rust.
    let mut save = core::mem::MaybeUninit::<VSave>::uninit();
    let save = save.as_mut_ptr().cast::<u64>();
    let bits = LANE_BITS.as_ptr();
    // SAFETY (all eight arms): the asm reads, per row, exactly the source
    // units holding that row's bits `off .. off + w` from `src_p` on at
    // `sstride` bytes per row (module docs, "Technique"), reads and writes
    // `h` rows of `w` pixels from `dst` on at `stride` bytes per row and the
    // 240-byte save area, does not touch the stack, saves every V register it
    // uses before its first use and restores it before it ends, and keeps
    // FPCR/FPSR (integer/bitwise SIMD only).
    unsafe {
        match (src.unit, ex.mode) {
            (Unit::Byte, WriteMode::Replace) => replace_kernel!(
                "ldrb {u:w}, [{q}], #1", "56", "8", dst, stride, src_p, sstride, off, w, h, fg, bg, save, bits
            ),
            (Unit::Word, WriteMode::Replace) => replace_kernel!(
                "ldrh {u:w}, [{q}], #2", "48", "16", dst, stride, src_p, sstride, off, w, h, fg, bg, save, bits
            ),
            (Unit::Byte, WriteMode::Transparent) => rmw_kernel!(
                bit, "", "ldrb {u:w}, [{q}], #1", "56", "8", dst, stride, src_p, sstride, off, w, h, fg, save, bits
            ),
            (Unit::Word, WriteMode::Transparent) => rmw_kernel!(
                bit, "", "ldrh {u:w}, [{q}], #2", "48", "16", dst, stride, src_p, sstride, off, w, h, fg, save, bits
            ),
            (Unit::Byte, WriteMode::Xor) => rmw_kernel!(
                eor, "", "ldrb {u:w}, [{q}], #1", "56", "8", dst, stride, src_p, sstride, off, w, h, fg, save, bits
            ),
            (Unit::Word, WriteMode::Xor) => rmw_kernel!(
                eor, "", "ldrh {u:w}, [{q}], #2", "48", "16", dst, stride, src_p, sstride, off, w, h, fg, save, bits
            ),
            // Fg on the clear bits = `Transparent` on the complemented word.
            (Unit::Byte, WriteMode::RevTransparent) => rmw_kernel!(
                bit, "eor {t:w}, {t:w}, #0xffff", "ldrb {u:w}, [{q}], #1", "56", "8",
                dst, stride, src_p, sstride, off, w, h, fg, save, bits
            ),
            (Unit::Word, WriteMode::RevTransparent) => rmw_kernel!(
                bit, "eor {t:w}, {t:w}, #0xffff", "ldrh {u:w}, [{q}], #2", "48", "16",
                dst, stride, src_p, sstride, off, w, h, fg, save, bits
            ),
        }
    }
}

/// Whether the source bytes `lo..hi` reach into the destination rows of
/// `area` (the bytes from its first pixel to its last, pitch included).
fn overlaps(surface: Surface, area: Area, lo: *const u8, hi: *const u8) -> bool {
    let first = surface.base as usize + (area.y * surface.stride_px + area.x) * 4;
    let end = surface.base as usize + ((area.y + area.h - 1) * surface.stride_px + area.x + area.w) * 4;
    (lo as usize) < end && first < hi as usize
}

/// The clipped box `(x0, y0, x1, y1)` (inclusive, from `ClipRect::clamp_to`)
/// as a kernel area; `None` if it is empty.
fn box_area(bx: (i32, i32, i32, i32)) -> Option<Area> {
    let (x0, y0, x1, y1) = bx;
    if x0 < 0 || y0 < 0 || x1 < x0 || y1 < y0 {
        return None;
    }
    Some(Area { x: x0 as usize, y: y0 as usize, w: (x1 - x0 + 1) as usize, h: (y1 - y0 + 1) as usize })
}

/// `draw_glyph_sized`'s box path: the `cell_w`-wide glyph `glyph` with its
/// cell at `(x, y)`, drawn into the clipped box `bx` (inclusive, inside the
/// cell) per `mode` with device pixels `fg`/`bg` — what the backend's scalar
/// loop does (`VdiScreen::glyph_scalar`, [`glyph_reference`]). `false`, with
/// nothing drawn, if the box is not inside the cell or reads past
/// `glyph.len()` (the scalar loop then runs, and panics on the short glyph as
/// before).
///
/// # Safety
///
/// `bx` lies in the store of `surface` (`[0, width) x [0, height)` of a
/// `height * stride_px` pixel store), nothing else accesses it during the
/// call.
#[allow(clippy::too_many_arguments)]
pub unsafe fn glyph(
    surface: Surface,
    x: i32,
    y: i32,
    glyph: &[u8],
    cell_w: usize,
    bx: (i32, i32, i32, i32),
    mode: WriteMode,
    fg: u32,
    bg: u32,
) -> bool {
    let Some(area) = box_area(bx) else { return false };
    let (x0, y0, x1, y1) = bx;
    if x0 < x || y0 < y || (x1 - x) as usize >= cell_w {
        return false;
    }
    // Rows of `ceil(cell_w / 8)` bytes; the box starts at cell column `c0`
    // of cell row `r0` and its last pixel sits in byte `last`, the highest
    // the box reads.
    let bpr = cell_w.div_ceil(8);
    let (c0, r0) = ((x0 - x) as usize, (y0 - y) as usize);
    let (c1, r1) = ((x1 - x) as usize, (y1 - y) as usize);
    let Some(last) = r1.checked_mul(bpr).and_then(|b| b.checked_add(c1 / 8)) else { return false };
    if last >= glyph.len() {
        return false;
    }
    let lo = glyph[r0 * bpr + c0 / 8..].as_ptr();
    let hi = glyph[last..].as_ptr().wrapping_add(1);
    if overlaps(surface, area, lo, hi) {
        return false;
    }
    let src = Bits { base: glyph[r0 * bpr..].as_ptr(), stride_bytes: bpr, bit: c0, unit: Unit::Byte };
    // SAFETY: the area lies in the store (contract); every byte the box reads
    // is at most `last < glyph.len()` and not in the destination rows
    // (checked above).
    unsafe { expand(surface, area, src, &Expand { mode, fg, bg }) };
    true
}

/// The colours `blit_mono` hands the write mode: `vrt_cpyfm`'s `MD_ERASE`
/// (`RevTransparent`) paints the **background** colour (`INTIN[2]`) on the
/// clear bits, and the four-mode combine paints its `fg` slot there, so that
/// one mode gets `(bg, fg)`; the others `(fg, bg)` — Replace: set → fg, clear
/// → bg; Transparent: set → fg; Xor: set → `!dst`.
#[inline]
pub fn mono_colors(mode: WriteMode, fg: u32, bg: u32) -> (u32, u32) {
    match mode {
        WriteMode::RevTransparent => (bg, fg),
        _ => (fg, bg),
    }
}

/// `blit_mono`: the mono raster `src` with its sub-rectangle placed at
/// `(dst_x, dst_y)`, drawn into the clipped box `bx` (inclusive) per `mode`
/// with the device pixels `fg`/`bg` of `vrt_cpyfm` (`INTIN[1]`/`INTIN[2]`;
/// [`mono_colors`]) — what the backend's scalar loop does
/// (`VdiScreen::mono_scalar`, [`mono_reference`]). `false`, with nothing
/// drawn, if a source origin is negative, the box starts left of or above
/// `(dst_x, dst_y)`, a word the box reads lies past the raster (the scalar
/// loop reads those as clear bits), or the raster lies in the destination
/// rows.
///
/// # Safety
///
/// `bx` lies in the store of `surface` (as [`glyph`]), nothing else accesses
/// it or the raster during the call.
#[allow(clippy::too_many_arguments)]
pub unsafe fn mono(
    surface: Surface,
    dst_x: i32,
    dst_y: i32,
    src: &MonoSrc<'_>,
    bx: (i32, i32, i32, i32),
    mode: WriteMode,
    fg: u32,
    bg: u32,
) -> bool {
    let Some(area) = box_area(bx) else { return false };
    let (x0, y0, _, _) = bx;
    if src.x < 0 || src.y < 0 || x0 < dst_x || y0 < dst_y {
        return false;
    }
    // The source pixel of the box's top left; word `sy * stride + sx / 16`
    // holds pixel `sx` of row `sy`, as in the scalar loop. Rows ascend and
    // `stride >= 0`, so the box's last pixel sits in the highest word read.
    let sx0 = src.x as usize + (x0 - dst_x) as usize;
    let sy0 = src.y as usize + (y0 - dst_y) as usize;
    let stride = src.stride_words as usize;
    let first = sy0 * stride + sx0 / 16;
    let last = (sy0 + area.h - 1) * stride + (sx0 + area.w - 1) / 16;
    let bytes = src.bits.as_bytes();
    if 2 * last + 2 > bytes.len() {
        return false;
    }
    let lo = bytes[2 * first..].as_ptr();
    let hi = bytes[2 * last + 1..].as_ptr().wrapping_add(1);
    if overlaps(surface, area, lo, hi) {
        return false;
    }
    let (fg, bg) = mono_colors(mode, fg, bg);
    let bits = Bits { base: bytes[2 * sy0 * stride..].as_ptr(), stride_bytes: 2 * stride, bit: sx0, unit: Unit::Word };
    // SAFETY: the area lies in the store (contract); every word the box reads
    // is at most `last`, inside the raster and not in the destination rows
    // (checked above).
    unsafe { expand(surface, area, bits, &Expand { mode, fg, bg }) };
    true
}

/// `draw_sprite`: the two-colour 16 x 16 pointer sprite with its cell at
/// `(x, y)`, drawn into the clipped box `bx` (inclusive, from
/// `ClipRect::clamp_to`: the cell on the screen) — per pixel, a set `data`
/// bit paints `fg`, else a set `mask` bit paints `bg`, else the pixel stays
/// (TASK/V59; [`sprite_reference`], the VDI pointer's per-pixel paint
/// before). Two [`expand`] passes in `Transparent`: the mask rows with `bg`,
/// then the data rows with `fg` over them, so a data pixel ends `fg`
/// wherever its mask bit stands. Row `r` of the cell is word `r` of each
/// array (bit 15 leftmost, host order); every row reads that one word.
///
/// Unlike the text and blit paths, this one also runs inside the timer
/// interrupt when that interrupted EL0 (the frame tick's pointer redraw,
/// `crate::vbl`, TASK/V55; the FP context note before
/// `crate::raster_copy::copy_rows`): the kernel's save
/// and restore of every V register it uses covers that case unchanged.
///
/// # Safety
///
/// `bx` lies in the store of `surface` (as [`glyph`]), nothing else
/// accesses it during the call. Panics if `bx` is not inside the cell.
#[allow(clippy::too_many_arguments)]
pub unsafe fn sprite(
    surface: Surface,
    x: i32,
    y: i32,
    mask: &[u16; 16],
    data: &[u16; 16],
    bx: (i32, i32, i32, i32),
    fg: u32,
    bg: u32,
) {
    let Some(area) = box_area(bx) else { return };
    let (x0, y0, x1, y1) = bx;
    assert!(x0 >= x && y0 >= y && x1 - x < 16 && y1 - y < 16, "sprite box outside its cell");
    let (c0, r0) = ((x0 - x) as usize, (y0 - y) as usize);
    let rows = |words: &[u16; 16]| Bits {
        base: words[r0..].as_ptr().cast::<u8>(),
        stride_bytes: 2,
        bit: c0,
        unit: Unit::Word,
    };
    // SAFETY: the area lies in the store (contract); row `r < area.h` reads
    // word `r0 + r <= 15` of each array, the bits `c0 .. c0 + area.w <= 16`
    // of it, and the arrays are borrowed, so not the store.
    unsafe {
        expand(surface, area, rows(mask), &Expand { mode: WriteMode::Transparent, fg: bg, bg: 0 });
        expand(surface, area, rows(data), &Expand { mode: WriteMode::Transparent, fg, bg: 0 });
    }
}

/// The scalar reference of [`sprite`]: the VDI pointer's paint before
/// TASK/V59 (`tos-api/src/vdi/mouse.rs` `draw`: per cell pixel a 1 x 1
/// `fill_rect` with the whole screen as clip — `fg` on a data bit, else `bg`
/// on a mask bit), on a plain `scr_w` x `scr_h` store. Host builds only.
///
/// # Safety
///
/// The store holds the `scr_h` rows of the screen, `scr_w <= stride_px`.
#[cfg(not(target_os = "none"))]
#[allow(clippy::too_many_arguments)]
pub unsafe fn sprite_reference(
    surface: Surface,
    scr_w: i32,
    scr_h: i32,
    x: i32,
    y: i32,
    mask: &[u16; 16],
    data: &[u16; 16],
    fg: u32,
    bg: u32,
) {
    for dy in 0..16 {
        for dx in 0..16 {
            let (px, py) = (x + dx, y + dy);
            if px < 0 || py < 0 || px >= scr_w || py >= scr_h {
                continue;
            }
            let bit = 0x8000u16 >> dx;
            // SAFETY: on the screen, so in the store (contract).
            let p = unsafe { surface.base.add(py as usize * surface.stride_px + px as usize) };
            if data[dy as usize] & bit != 0 {
                // SAFETY: as above.
                unsafe { p.write(fg) };
            } else if mask[dy as usize] & bit != 0 {
                // SAFETY: as above.
                unsafe { p.write(bg) };
            }
        }
    }
}

/// One pixel of the scalar paths on a plain store: `VdiScreen::combine`.
///
/// # Safety
///
/// `p` is valid for a `u32` read and write.
#[cfg(not(target_os = "none"))]
unsafe fn combine_reference(p: *mut u32, set: bool, mode: WriteMode, fg: u32, bg: u32) {
    // SAFETY: the caller's contract.
    unsafe {
        match mode {
            WriteMode::Replace => p.write(if set { fg } else { bg }),
            WriteMode::Transparent if set => p.write(fg),
            WriteMode::Xor if set => p.write(!p.read()),
            WriteMode::RevTransparent if !set => p.write(fg),
            _ => {}
        }
    }
}

/// The scalar reference of [`expand`]: per pixel, the source bit read on its
/// own, combined as `VdiScreen::combine` does. Host builds only.
///
/// # Safety
///
/// As [`expand`].
#[cfg(not(target_os = "none"))]
pub unsafe fn expand_reference(surface: Surface, area: Area, src: Bits, ex: &Expand) {
    for r in 0..area.h {
        // SAFETY: row `r` of the source (contract).
        let row = unsafe { src.base.add(r * src.stride_bytes) };
        for c in 0..area.w {
            let b = src.bit + c;
            // SAFETY: the unit holding bit `b` of row `r` (contract).
            let set = unsafe {
                match src.unit {
                    Unit::Byte => row.add(b / 8).read() & (0x80u8 >> (b & 7)) != 0,
                    Unit::Word => {
                        row.add(b / 16 * 2).cast::<u16>().read_unaligned() & (0x8000u16 >> (b & 15)) != 0
                    }
                }
            };
            // SAFETY: inside the area (contract).
            let p = unsafe { surface.base.add((area.y + r) * surface.stride_px + area.x + c) };
            // SAFETY: as above.
            unsafe { combine_reference(p, set, ex.mode, ex.fg, ex.bg) };
        }
    }
}

/// The scalar reference of [`glyph`]: `draw_glyph_sized`'s box loop before
/// TASK/V50 (`glyph[(yy - y) * bpr + col / 8] & (0x80 >> (col & 7))` →
/// `combine`), on a plain store. Host builds only.
///
/// # Safety
///
/// As [`glyph`].
#[cfg(not(target_os = "none"))]
#[allow(clippy::too_many_arguments)]
pub unsafe fn glyph_reference(
    surface: Surface,
    x: i32,
    y: i32,
    glyph: &[u8],
    cell_w: usize,
    bx: (i32, i32, i32, i32),
    mode: WriteMode,
    fg: u32,
    bg: u32,
) {
    let (x0, y0, x1, y1) = bx;
    let bpr = cell_w.div_ceil(8);
    for yy in y0..=y1 {
        let base = (yy - y) as usize * bpr;
        for xx in x0..=x1 {
            let col = (xx - x) as usize;
            let byte = glyph[base + col / 8];
            let set = byte & (0x80u8 >> (col & 7)) != 0;
            // SAFETY: inside the box (contract).
            let p = unsafe { surface.base.add(yy as usize * surface.stride_px + xx as usize) };
            // SAFETY: as above.
            unsafe { combine_reference(p, set, mode, fg, bg) };
        }
    }
}

/// The scalar reference of [`mono`]: `blit_mono`'s loop before TASK/V50
/// (`MonoBits::word(sy * stride + sx / 16)`, a word past the raster clear;
/// `MD_ERASE` with fg and bg swapped → `combine`), on a plain store. Host
/// builds only.
///
/// # Safety
///
/// As [`mono`].
#[cfg(not(target_os = "none"))]
#[allow(clippy::too_many_arguments)]
pub unsafe fn mono_reference(
    surface: Surface,
    dst_x: i32,
    dst_y: i32,
    src: &MonoSrc<'_>,
    bx: (i32, i32, i32, i32),
    mode: WriteMode,
    fg: u32,
    bg: u32,
) {
    let (x0, y0, x1, y1) = bx;
    let (c_fg, c_bg) = match mode {
        WriteMode::RevTransparent => (bg, fg),
        _ => (fg, bg),
    };
    let stride = src.stride_words as usize;
    for dyc in y0..=y1 {
        let sy = src.y as i32 + (dyc - dst_y);
        for dxc in x0..=x1 {
            let sx = src.x as i32 + (dxc - dst_x);
            let widx = sy as usize * stride + (sx as usize) / 16;
            let set = src.bits.word(widx).is_some_and(|w| w & (0x8000u16 >> (sx & 15)) != 0);
            // SAFETY: inside the box (contract).
            let p = unsafe { surface.base.add(dyc as usize * surface.stride_px + dxc as usize) };
            // SAFETY: as above.
            unsafe { combine_reference(p, set, mode, c_fg, c_bg) };
        }
    }
}
