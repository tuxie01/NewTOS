//! NEON raster kernels of the VDI screen backend (ADR-036, TASK/V49).
//!
//! The kernel builds for `aarch64-unknown-none-softfloat`, so the compiler
//! never emits Advanced SIMD and never vectorises a raster loop. The kernels
//! here are hand-written AArch64 Advanced SIMD (`asm!`), called by
//! `crate::vdi_backend` only — behind the `RasterBackend` trait, nowhere else
//! in the kernel (ADR-036 point 1).
//!
//! # Interface
//!
//! One call fills one rectangle. Everything a pixel loop used to fetch per
//! pixel — the framebuffer behind a `dyn`, its base, stride and bounds, the
//! colour — is resolved once by the caller and handed over in three plain
//! values: a [`Surface`] (base pointer and row pitch), an [`Area`] already
//! clipped to the visible screen, and a [`Fill`] (pattern rows, write mode,
//! the two device colours). The kernel does no clipping and no bounds check;
//! its `# Safety` contract is that the area lies in the store.
//!
//! # Rules the assembler keeps (ADR-036)
//!
//! - Integer and bitwise SIMD only: `dup`, `cmtst`, `bsl`, `bit`,
//!   `eor`, `mov`, `ext`, loads and stores. No floating-point arithmetic and
//!   no saturating instruction, so `FPCR` and `FPSR` are neither read nor
//!   changed.
//! - Every V register a kernel uses is stored to a save area on entry and
//!   loaded back before the `asm!` block ends. During a trap the V registers
//!   still hold the EL0 caller's values (`save_fp_context`,
//!   `kernel/src/arch/aarch64/exceptions.rs:999@save_fp_context`, saves them
//!   only when the kernel switches EL0 contexts); the kernels hand them back
//!   untouched.
//! - Nothing can switch the EL0 FP/SIMD context while a kernel runs
//!   (TASK/V49, "FP-Kontext"): the kernels are leaves — no call, no upcall,
//!   no wait inside the `asm!` block; the only exception that can arrive
//!   inside one is an IRQ, and an IRQ taken at EL1 never switches a context
//!   (`rust_irq_handler`, `kernel/src/drivers/gic.rs:201@rust_irq_handler`:
//!   timer, PL011 drain, the frame tick `crate::vbl::irq_tick` and
//!   `irq_input_pump`, both of which return at once unless the IRQ came from
//!   EL0, and the cursor blink, which draws through the scalar row methods). `save_fp_context` / `restore_fp_context` run only on the
//!   synchronous `svc` paths after a dispatch has returned, and only the boot
//!   core runs.
//! - The assembler's FP/SIMD feature is switched on for the block only and
//!   back off at its end on the kernel target (`.arch armv8-a+nofp+nosimd`,
//!   as `save_fp_context` does), so nothing else in the translation unit can
//!   emit vector code by accident.
//!
//! # Pixel semantics
//!
//! Exactly those of the scalar path the kernels replace
//! (`VdiScreen::combine`, `kernel/src/vdi_backend.rs`): pattern row
//! `pattern[y & (len - 1)]` for absolute row `y`, bit `0x8000 >> (x & 15)`
//! for absolute column `x`, and per pixel
//!
//! | mode | bit set | bit clear |
//! |---|---|---|
//! | `Replace` | `fg` | `bg` |
//! | `Transparent` | `fg` | untouched |
//! | `Xor` | `!dst` | untouched |
//! | `RevTransparent` | untouched | `fg` |
//!
//! [`fill_pattern_reference`] is that scalar path on a plain store; host
//! tests on an AArch64 host (`tos-api/tests/raster_kernel_fill.rs`) run the
//! kernel against it bit for bit (ADR-036 point 4).
//!
//! # Technique
//!
//! Expand each pattern row once into 16 device pixels (or 16 lane masks),
//! rotated to the area's start column, and repeat that 64-byte block across
//! the row: a store-only loop for `Replace`, a load-combine-store loop for
//! the other modes, one instruction per four pixels. A row whose pattern word
//! leaves every pixel untouched (`Transparent`/`Xor` with `0x0000`,
//! `RevTransparent` with `0xFFFF`) is skipped. Read for technique only, not
//! copied (ADR-035): fVDI's one routine per write mode with the all-set /
//! all-clear pattern word special-cased per row
//! (`references/fvdi/fvdi/drivers/16_bit/16b_fill.c:33-275`, dispatch
//! `:515-587`) and MagiC's high-colour driver, which expands a pattern row
//! into a line of pixels and copies it across the row
//! (`references/magicmac/kernel/vdi/drivers/off16m.s:1995-2204`, the solid
//! case `:1895-1927`).

use tos_api::vdi::backend::WriteMode;

/// The pixel store a kernel writes into.
#[derive(Clone, Copy, Debug)]
pub struct Surface {
    /// Pixel `(0, 0)`: one `u32` per pixel.
    pub base: *mut u32,
    /// Row pitch in pixels (`>=` the visible width).
    pub stride_px: usize,
}

/// A rectangle of the store, already clipped: columns `x..x + w`, rows
/// `y..y + h`. `x` and `y` are absolute, because the pattern is aligned to
/// the screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Area {
    /// Leftmost column.
    pub x: usize,
    /// Top row.
    pub y: usize,
    /// Columns (`0` draws nothing).
    pub w: usize,
    /// Rows (`0` draws nothing).
    pub h: usize,
}

/// What to fill with.
#[derive(Clone, Copy, Debug)]
pub struct Fill<'a> {
    /// Pattern rows, MSB = leftmost pixel; row `y & (len - 1)` serves
    /// absolute row `y` (the VDI hands power-of-two lengths). Empty: nothing
    /// is drawn.
    pub pattern: &'a [u16],
    /// How a pattern bit combines with the destination (module docs table).
    pub mode: WriteMode,
    /// Device pixel on the pattern's foreground bits.
    pub fg: u32,
    /// Device pixel on the clear bits (`Replace` only).
    pub bg: u32,
}

/// The sixteen lane masks `0x8000 >> i` of one 16-pixel block, lane `i` =
/// the block's `i`-th pixel, in `ld1 {v4.4s-v7.4s}` order.
pub(crate) static LANE_BITS: [u32; 16] = [
    0x8000, 0x4000, 0x2000, 0x1000, 0x0800, 0x0400, 0x0200, 0x0100, 0x0080, 0x0040, 0x0020, 0x0010,
    0x0008, 0x0004, 0x0002, 0x0001,
];

/// The save area of one kernel call: room for `v0..v14`, 16 bytes each.
#[repr(C, align(16))]
pub(crate) struct VSave([u64; 30]);

/// The assembler switch back after a kernel: on the kernel target FP/SIMD
/// assembly goes off again (module docs); a host test build keeps its own
/// target's setting.
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

/// The per-row prologue shared by every kernel: pick the pattern word of the
/// row (`{t}` = `pattern[prow & pmask]`), rotate it left by the area's start
/// phase (`{shift}` = `16 - (x & 15)`: `(w | w << 16) >> shift`), mask it to
/// 16 bits.
macro_rules! row_word {
    () => {
        concat!(
            "and {t}, {prow}, {pmask}\n",
            "ldrh {t:w}, [{pat}, {t}, lsl #1]\n",
            "orr {t:w}, {t:w}, {t:w}, lsl #16\n",
            "lsr {t:w}, {t:w}, {shift:w}\n",
            "and {t:w}, {t:w}, #0xffff",
        )
    };
}

/// Lane masks of the row word: `v0..v3` lane = all ones where the pixel's
/// pattern bit is set (`v4..v7` = [`LANE_BITS`], `v10` = scratch).
macro_rules! row_masks {
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
    // Transparent: fg where the mask is set (`BIT`, insert if true).
    (bit, $d:literal, $m:literal) => {
        concat!("bit ", $d, ".16b, v8.16b, ", $m, ".16b")
    };
    // XOR: complement the destination where the mask is set.
    (eor, $d:literal, $m:literal) => {
        concat!("eor ", $d, ".16b, ", $d, ".16b, ", $m, ".16b")
    };
}

/// The read-modify-write kernel (`Transparent`, `Xor`, `RevTransparent`):
/// `$op` is the [`combine!`] arm, `$skip` the row-skip test on `{t}` (branch
/// to `8f` when the row leaves every pixel untouched; `RevTransparent`
/// complements `{t}` there first, so its masks select the clear bits).
///
/// V registers: `v0..v3` masks, `v4..v7` lane bits, `v8` fg, `v10` row
/// word, `v11..v14` destination pixels — saved and restored, `v9` unused.
macro_rules! rmw_kernel {
    ($op:ident, $skip:literal, $dst:ident, $stride:ident, $w:ident, $h:ident, $pat:ident,
     $pmask:ident, $prow:ident, $shift:ident, $fg:ident, $save:ident, $bits:ident) => {
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
            row_word!(),
            $skip,
            row_masks!(),
            "mov {p}, {dst}",
            "subs {n}, {w}, #16",
            "b.lo 4f",
            // 16 pixels (one 64-byte block) per pass.
            "3:",
            "ld1 {{v11.4s, v12.4s, v13.4s, v14.4s}}, [{p}]",
            combine!($op, "v11", "v0"),
            combine!($op, "v12", "v1"),
            combine!($op, "v13", "v2"),
            combine!($op, "v14", "v3"),
            "st1 {{v11.4s, v12.4s, v13.4s, v14.4s}}, [{p}], #64",
            "subs {n}, {n}, #16",
            "b.hs 3b",
            // Tail: `n` = remaining - 16, whose low four bits are the
            // remaining 0..15 pixels; consume 8, 4, 2, 1 and slide the masks.
            "4:",
            "tbz {n}, #3, 5f",
            "ld1 {{v11.4s, v12.4s}}, [{p}]",
            combine!($op, "v11", "v0"),
            combine!($op, "v12", "v1"),
            "st1 {{v11.4s, v12.4s}}, [{p}], #32",
            "mov v0.16b, v2.16b",
            "mov v1.16b, v3.16b",
            "5:",
            "tbz {n}, #2, 6f",
            "ld1 {{v11.4s}}, [{p}]",
            combine!($op, "v11", "v0"),
            "st1 {{v11.4s}}, [{p}], #16",
            "mov v0.16b, v1.16b",
            "6:",
            "tbz {n}, #1, 7f",
            "ld1 {{v11.2s}}, [{p}]",
            combine!($op, "v11", "v0"),
            "st1 {{v11.2s}}, [{p}], #8",
            "ext v0.16b, v0.16b, v0.16b, #8",
            "7:",
            "tbz {n}, #0, 8f",
            "ld1 {{v11.s}}[0], [{p}]",
            combine!($op, "v11", "v0"),
            "st1 {{v11.s}}[0], [{p}]",
            // ── next row ──
            "8:",
            "add {dst}, {dst}, {stride}",
            "add {prow}, {prow}, #1",
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
            w = in(reg) $w,
            h = inout(reg) $h => _,
            pat = in(reg) $pat,
            pmask = in(reg) $pmask,
            prow = inout(reg) $prow => _,
            shift = in(reg) $shift,
            fg = in(reg) $fg,
            save = in(reg) $save,
            bits = in(reg) $bits,
            p = out(reg) _,
            n = out(reg) _,
            t = out(reg) _,
            options(nostack),
        )
    };
}

/// Fill `area` of `surface` with `fill`'s pattern in its write mode — the
/// pattern-fill kernel (TASK/V49 part 2).
///
/// An empty area or an empty pattern draws nothing.
///
/// # Safety
///
/// Every pixel `surface.base + row * surface.stride_px + col` for
/// `row in area.y..area.y + area.h`, `col in area.x..area.x + area.w` must
/// be valid for reads and writes of a `u32` and not be accessed by anything
/// else during the call.
pub unsafe fn fill_pattern(surface: Surface, area: Area, fill: &Fill<'_>) {
    if area.w == 0 || area.h == 0 || fill.pattern.is_empty() {
        return;
    }
    // SAFETY: the caller guarantees the area lies in the store (function
    // contract), so `base + y * stride + x` is in bounds.
    let dst = unsafe { surface.base.add(area.y * surface.stride_px + area.x) };
    let rows = Rows { stride: surface.stride_px * 4, w: area.w, h: area.h, pat: fill.pattern.as_ptr(),
        pmask: fill.pattern.len() - 1, prow: area.y, phase: area.x & 15 };
    // SAFETY: the rows are the area (function contract), `& pmask` < `len`.
    unsafe { fill_rows(dst, &rows, fill.mode, fill.fg, fill.bg) };
}

/// Kernel 1 proper over `rows` from `dst` ([`fill_pattern`], [`fill_span`]).
/// `# Safety`: as [`fill_pattern`], for the pixels and pattern words of `rows`.
#[inline(always)]
unsafe fn fill_rows(dst: *mut u32, rows: &Rows, mode: WriteMode, fg: u32, bg: u32) {
    let (stride, w, h, pat, pmask, prow) = (rows.stride, rows.w, rows.h, rows.pat, rows.pmask, rows.prow);
    let shift = 16 - rows.phase as u32;
    // Written by the asm before it is read; never read by Rust.
    let mut save = core::mem::MaybeUninit::<VSave>::uninit();
    let save = save.as_mut_ptr().cast::<u64>();
    let bits = LANE_BITS.as_ptr();
    // SAFETY (all four arms): the asm reads `h` rows of `w` pixels from
    // `dst` on at `stride` bytes per row and the pattern words
    // `pat[(prow + i) & pmask]` (`& pmask` < `len`), writes only those pixels
    // and the 240-byte save area, does not touch the stack, saves every V
    // register it uses before its first use and restores it before it ends
    // (module docs), and keeps FPCR/FPSR (integer/bitwise SIMD only).
    unsafe {
        match mode {
            WriteMode::Replace => core::arch::asm!(
                // V registers: `v0..v3` pixels, `v4..v7` lane bits, `v8` fg,
                // `v9` bg, `v10` row word — saved and restored.
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
                // ── one row: its 16 pixels, fg on set bits, bg on clear ──
                "2:",
                row_word!(),
                row_masks!(),
                "bsl v0.16b, v8.16b, v9.16b",
                "bsl v1.16b, v8.16b, v9.16b",
                "bsl v2.16b, v8.16b, v9.16b",
                "bsl v3.16b, v8.16b, v9.16b",
                "mov {p}, {dst}",
                "subs {n}, {w}, #16",
                "b.lo 4f",
                "3:",
                "st1 {{v0.4s, v1.4s, v2.4s, v3.4s}}, [{p}], #64",
                "subs {n}, {n}, #16",
                "b.hs 3b",
                // Tail as in `rmw_kernel!`.
                "4:",
                "tbz {n}, #3, 5f",
                "st1 {{v0.4s, v1.4s}}, [{p}], #32",
                "mov v0.16b, v2.16b",
                "mov v1.16b, v3.16b",
                "5:",
                "tbz {n}, #2, 6f",
                "st1 {{v0.4s}}, [{p}], #16",
                "mov v0.16b, v1.16b",
                "6:",
                "tbz {n}, #1, 7f",
                "st1 {{v0.2s}}, [{p}], #8",
                "ext v0.16b, v0.16b, v0.16b, #8",
                "7:",
                "tbz {n}, #0, 8f",
                "st1 {{v0.s}}[0], [{p}]",
                "8:",
                "add {dst}, {dst}, {stride}",
                "add {prow}, {prow}, #1",
                "subs {h}, {h}, #1",
                "b.ne 2b",
                "ldp q0, q1, [{save}, #0]",
                "ldp q2, q3, [{save}, #32]",
                "ldp q4, q5, [{save}, #64]",
                "ldp q6, q7, [{save}, #96]",
                "ldp q8, q9, [{save}, #128]",
                "ldr q10, [{save}, #160]",
                simd_off!(),
                dst = inout(reg) dst => _,
                stride = in(reg) stride,
                w = in(reg) w,
                h = inout(reg) h => _,
                pat = in(reg) pat,
                pmask = in(reg) pmask,
                prow = inout(reg) prow => _,
                shift = in(reg) shift,
                fg = in(reg) fg,
                bg = in(reg) bg,
                save = in(reg) save,
                bits = in(reg) bits,
                p = out(reg) _,
                n = out(reg) _,
                t = out(reg) _,
                options(nostack),
            ),
            // Nothing to do on a row whose word is 0x0000.
            WriteMode::Transparent => rmw_kernel!(
                bit, "cbz {t:w}, 8f", dst, stride, w, h, pat, pmask, prow, shift, fg, save, bits
            ),
            WriteMode::Xor => rmw_kernel!(
                eor, "cbz {t:w}, 8f", dst, stride, w, h, pat, pmask, prow, shift, fg, save, bits
            ),
            // Fg on the clear bits = `Transparent` on the complemented word;
            // nothing to do on a row whose word is 0xFFFF.
            WriteMode::RevTransparent => rmw_kernel!(
                bit,
                "eor {t:w}, {t:w}, #0xffff\ncbz {t:w}, 8f",
                dst, stride, w, h, pat, pmask, prow, shift, fg, save, bits
            ),
        }
    }
}

/// The solid pattern: every bit set, for [`fill_solid`].
static SOLID: [u16; 1] = [0xFFFF];

/// Fill `area` of `surface` with one device pixel `color` — `vr_recfl`'s and
/// the AES' solid rectangles (`RasterBackend::fill_rect`). The pattern kernel
/// in `Replace` mode with every bit set: the same stores as the former
/// per-pixel loop (`Framebuffer::fill_rect`, `kernel/src/hal/framebuffer.rs`).
///
/// # Safety
///
/// As [`fill_pattern`].
pub unsafe fn fill_solid(surface: Surface, area: Area, color: u32) {
    let fill = Fill { pattern: &SOLID, mode: WriteMode::Replace, fg: color, bg: color };
    // SAFETY: the caller's contract is `fill_pattern`'s.
    unsafe { fill_pattern(surface, area, &fill) };
}

/// The scalar reference of [`fill_pattern`]: the backend's per-pixel path
/// before TASK/V49 (`fill_pattern` → `combine` → `put_pixel` / `xor_pixel`),
/// on a plain store. Host builds only — the host tests compare the kernel
/// against it (ADR-036 point 4); the kernel never runs it.
///
/// # Safety
///
/// As [`fill_pattern`].
#[cfg(not(target_os = "none"))]
pub unsafe fn fill_pattern_reference(surface: Surface, area: Area, fill: &Fill<'_>) {
    if fill.pattern.is_empty() {
        return;
    }
    let pmask = fill.pattern.len() - 1;
    for yy in area.y..area.y + area.h {
        let row = fill.pattern[yy & pmask];
        for xx in area.x..area.x + area.w {
            let set = row & (0x8000u16 >> (xx & 0xF)) != 0;
            // SAFETY: inside the area, which the caller guarantees.
            let p = unsafe { surface.base.add(yy * surface.stride_px + xx) };
            // SAFETY: as above.
            unsafe {
                match fill.mode {
                    WriteMode::Replace => p.write(if set { fill.fg } else { fill.bg }),
                    WriteMode::Transparent if set => p.write(fill.fg),
                    WriteMode::Xor if set => p.write(!p.read()),
                    WriteMode::RevTransparent if !set => p.write(fill.fg),
                    _ => {}
                }
            }
        }
    }
}

/// Fill the `n` pixels `dst[0..n]` with the 16-bit dash `word` in `mode` —
/// one span of a horizontal line (TASK/V58, `crate::raster_line`): pixel `i`
/// takes bit `0x8000 >> ((phase + i) & 15)` of `word` and combines as in
/// [`fill_pattern`] (module docs table). Kernel 1 on one row whose pattern
/// is `word`, started at `phase` instead of `x & 15`. `n == 0` draws
/// nothing.
///
/// # Safety
///
/// `dst[0..n]` must be valid for reads and writes of `u32`s and not be
/// accessed by anything else during the call.
pub unsafe fn fill_span(dst: *mut u32, n: usize, phase: u32, word: u16, mode: WriteMode, fg: u32, bg: u32) {
    if n == 0 {
        return;
    }
    let pat = [word];
    let rows = Rows { stride: 0, w: n, h: 1, pat: pat.as_ptr(), pmask: 0, prow: 0, phase: phase as usize & 15 };
    // SAFETY: one row of `n` pixels from `dst` (function contract); with
    // `pmask` 0 the kernel reads only `pat[0]`, alive for the call.
    unsafe { fill_rows(dst, &rows, mode, fg, bg) };
}

/// The geometry of one kernel-1 call: `h >= 1` rows of `w >= 1` pixels,
/// `stride` bytes apart; row `i` takes pattern word `pat[(prow + i) & pmask]`,
/// its first pixel bit `0x8000 >> phase` (`phase < 16`).
struct Rows {
    stride: usize,
    w: usize,
    h: usize,
    pat: *const u16,
    pmask: usize,
    prow: usize,
    phase: usize,
}
