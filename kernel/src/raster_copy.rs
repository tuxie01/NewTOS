//! NEON rectangle copy within the screen with the 16 BitBlt logic ops:
//! `vro_cpyfm` screen to screen, AES window moves and scrolls, the VT52
//! console scroll (`RasterBackend::copy_rect`) — raster kernel 3 (ADR-036,
//! TASK/V51), on the frame of kernel 1 ([`crate::raster_kernel`], TASK/V49).
//!
//! # Interface
//!
//! [`copy_area`] copies one rectangle of a [`Surface`] onto another of the
//! same surface: the destination an [`Area`], the source its top-left pixel,
//! the op, and the two directions — columns right to left or left to right,
//! rows bottom-up or top-down. [`copy_rect`] is the backend's caller turned
//! into a plain function, so the host tests cover it whole: from the clipped
//! destination box the backend computes, it picks the directions and drops
//! the rows and columns whose source lies off the screen, exactly as the
//! per-pixel loop it replaces, and calls [`copy_area`].
//!
//! # Rules the assembler keeps (ADR-036)
//!
//! Those of [`crate::raster_kernel`] (module docs), unchanged: integer and
//! bitwise SIMD only (`dup`, `and`, `eor`, `bsl`, loads and stores;
//! `FPCR`/`FPSR` untouched), every V register used saved on entry and
//! restored before the `asm!` block ends (copy kernel `v0..v3`, op kernel
//! `v0..v13`, [`copy_rows`] `v0..v3`), FP/SIMD assembly off again at the end on
//! the kernel target. The kernels are leaves; the TASK/V49 finding
//! ("FP-Kontext") holds: nothing can switch the EL0 FP/SIMD context while one
//! runs ([`copy_area`] on the trap path; [`copy_rows`] also in the timer
//! interrupt from EL0, see there).
//!
//! # Pixel semantics
//!
//! Exactly the scalar loop the kernel replaces (`VdiScreen::copy_rect` before
//! TASK/V51, [`copy_rect_reference`]): for every pixel of the clipped
//! destination box whose source lies on the screen, `D' = raster_op(op, S, D)`
//! ([`raster_op`], the packed-word table of EmuTOS' `vro_cpyfm16`); a pixel
//! whose source is off the screen stays untouched. The loop walks the rows
//! bottom-up when the destination lies below the source (`dy > src_y`) and the
//! columns right to left when it lies right of it (`dx > src_x`), so it never
//! reads a pixel it has already written: every result is the op of the
//! **original** source and destination pixels. The kernel walks the same
//! directions in blocks of up to 16 pixels, loading a block's source (and
//! destination) before it stores it; in the direction of the loop no block
//! reads what an earlier block stored, so the result is that of the loop.
//! Rows of different index never share a byte (both lie in `[0, width)` of
//! their row and `width <= stride`), so overlap within a row arises only when
//! source and destination share it — and there the column direction rules,
//! as in the loop.
//!
//! # Technique
//!
//! `S_ONLY` (3), the window move and scroll, is a pure copy: 64-byte blocks of
//! loads and stores, then 8/4/2/1 pixels. Every other op runs one kernel that
//! loads source and destination and computes any of the 16 functions from
//! four constant masks per call: with `op` bit 0 = `f(S=1, D=1)`, bit 1 =
//! `f(1, 0)`, bit 2 = `f(0, 1)`, bit 3 = `f(0, 0)` (the BitBlt numbering:
//! `S & D` = 1, `S` = 3, `S ^ D` = 6, `!D` = 10), `X = c1 ^ (D & (c0 ^ c1))`
//! is the result where S is set and `Y = c3 ^ (D & (c2 ^ c3))` where it is
//! clear, and `bsl` picks per bit (`S ? X : Y`). The right-to-left kernels
//! walk each row from its end with the pointers stepping down before each
//! block. Read for technique only, not copied (ADR-035, ADR-034): fVDI's
//! 16-bit driver walks the rows bottom-up when `src_y < dst_y`
//! (`references/fvdi/fvdi/drivers/16_bit/16b_blit.c:1150-1155`), each row
//! backwards only when source and destination share a row and `src_x < dst_x`
//! (`:1173-1186`), with routines of its own for copy and OR and one switch on
//! the op for the rest (`:617`, `:704`, `:795-856`); MagiC's 16M driver walks
//! the whole block by descending address unless the source starts above the
//! destination, the op ignores the source (0, 10, 15) or the two do not
//! overlap (`references/magicmac/kernel/vdi/drivers/off16m.s:2879-2895`,
//! descending loop `:2916`, one routine per op `:2954`), and copies with
//! `move16` where both start on the same 16-byte boundary (`:3136-3230`);
//! TOS 4.04 compares the two start addresses and runs its BLiTTER descending
//! when the source lies lower (`references/tos4x/pixvdi/sources/hpxoblit.s:178-179`,
//! `:234-279`, tag `build/tos404`).

use crate::raster_kernel::{Area, Surface, VSave};

/// The 16-entry BitBlt logic-op table, `D' = op(S, D)`, acting **literally
/// on the packed pixel word** (`vro_cpyfm`, EmuTOS `vdi_raster.c:495-544` /
/// `vro_cpyfm16:1008-1220`). Only the low nibble of `op` selects the
/// operation (`PAT_FLAG` is stripped by the caller); the same chunky word
/// semantics EmuTOS' truecolor `vro_cpyfm16` uses. Note the Atari planar
/// names invert against a truecolor screen: `ALL_WHITE` (0) writes `0`
/// (black), `ALL_BLACK` (15) writes `!0` (white) — the op acts on bits,
/// not palette indices. Applied to the 32-bit store it is the same op on
/// the 16-bit screen word, overlay bit included, because the store is a
/// bitwise expansion of that word (`word_to_device` in
/// `kernel/src/vdi_backend.rs`): `ALL_BLACK` yields `0xFF_FFFFFF` =
/// `0xFFFF`, the cleared-screen word, as on a Falcon.
///
/// Moved here from `crate::vdi_backend` (which re-exports it) by TASK/V51, so
/// the kernel's host tests check the kernel against this very table.
#[inline]
pub fn raster_op(op: u8, s: u32, d: u32) -> u32 {
    match op & 0x0f {
        0 => 0,             // BM_ALL_WHITE   D' = 0
        1 => s & d,         // BM_S_AND_D     D' = S & D
        2 => s & !d,        // BM_S_AND_NOTD  D' = S & ~D
        3 => s,             // BM_S_ONLY      D' = S            (replace)
        4 => !s & d,        // BM_NOTS_AND_D  D' = ~S & D
        5 => d,             // BM_D_ONLY      D' = D            (no-op)
        6 => s ^ d,         // BM_S_XOR_D     D' = S ^ D        (XOR)
        7 => s | d,         // BM_S_OR_D      D' = S | D        (OR)
        8 => !(s | d),      // BM_NOT_SORD    D' = ~(S | D)
        9 => !(s ^ d),      // BM_NOT_SXORD   D' = ~(S ^ D)
        10 => !d,           // BM_NOT_D       D' = ~D
        11 => s | !d,       // BM_S_OR_NOTD   D' = S | ~D
        12 => !s,           // BM_NOT_S       D' = ~S
        13 => !s | d,       // BM_NOTS_OR_D   D' = ~S | D
        14 => !(s & d),     // BM_NOT_SANDD   D' = ~(S & D)
        _ => !0,            // BM_ALL_BLACK   D' = ~0
    }
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

/// One op on one register: source `$s`, destination `$d`, scratch `$x`;
/// `v10` = c1, `v11` = c0 ^ c1, `v12` = c3, `v13` = c2 ^ c3 (module docs,
/// "Technique"). Leaves `S ? X : Y` in `$s`; `$d` is consumed.
macro_rules! rop_reg {
    ($s:literal, $d:literal, $x:literal) => {
        concat!(
            "and ", $x, ".16b, ", $d, ".16b, v11.16b\n",
            "eor ", $x, ".16b, ", $x, ".16b, v10.16b\n",
            "and ", $d, ".16b, ", $d, ".16b, v13.16b\n",
            "eor ", $d, ".16b, ", $d, ".16b, v12.16b\n",
            "bsl ", $s, ".16b, ", $x, ".16b, ", $d, ".16b\n",
        )
    };
}

/// The op on the registers of a block: `r4` = `v0..v3` with `v4..v7`, `r2`
/// = `v0, v1` with `v4, v5`, `r1` = `v0` with `v4` (scratch `v8`, `v9`).
macro_rules! rop {
    (r4) => {
        concat!(
            rop_reg!("v0", "v4", "v8"),
            rop_reg!("v1", "v5", "v9"),
            rop_reg!("v2", "v6", "v8"),
            rop_reg!("v3", "v7", "v9"),
        )
    };
    (r2) => {
        concat!(rop_reg!("v0", "v4", "v8"), rop_reg!("v1", "v5", "v9"))
    };
    (r1) => {
        rop_reg!("v0", "v4", "v8")
    };
}

/// One block of `$b` bytes: `{p}` source, `{q}` destination cursor, `$s` /
/// `$d` the source / destination register lists. `fwd` steps the cursors up
/// after the block, `bwd` down before it; every load of a block comes before
/// its store (module docs, "Pixel semantics").
macro_rules! chunk {
    (fwd, copy, $s:literal, $d:literal, $r:ident, $b:literal) => {
        concat!("ld1 ", $s, ", [{p}], #", $b, "\n", "st1 ", $s, ", [{q}], #", $b, "\n")
    };
    (fwd, rop, $s:literal, $d:literal, $r:ident, $b:literal) => {
        concat!(
            "ld1 ", $s, ", [{p}], #", $b, "\n",
            "ld1 ", $d, ", [{q}]\n",
            rop!($r),
            "st1 ", $s, ", [{q}], #", $b, "\n",
        )
    };
    (bwd, copy, $s:literal, $d:literal, $r:ident, $b:literal) => {
        concat!(
            "sub {p}, {p}, #", $b, "\n",
            "ld1 ", $s, ", [{p}]\n",
            "sub {q}, {q}, #", $b, "\n",
            "st1 ", $s, ", [{q}]\n",
        )
    };
    (bwd, rop, $s:literal, $d:literal, $r:ident, $b:literal) => {
        concat!(
            "sub {p}, {p}, #", $b, "\n",
            "ld1 ", $s, ", [{p}]\n",
            "sub {q}, {q}, #", $b, "\n",
            "ld1 ", $d, ", [{q}]\n",
            rop!($r),
            "st1 ", $s, ", [{q}]\n",
        )
    };
}

/// Where a row's cursors start: its left end (`fwd`) or one past its right
/// end (`bwd`).
macro_rules! row_start {
    (fwd) => {
        concat!("mov {p}, {src}\n", "mov {q}, {dst}\n")
    };
    (bwd) => {
        concat!("add {p}, {src}, {w}, lsl #2\n", "add {q}, {dst}, {w}, lsl #2\n")
    };
}

/// The row loop of every kernel: `{h}` rows of `{w}` pixels, row pointers
/// `{src}`/`{dst}` stepped by `{step}` bytes (negative: bottom-up). 16
/// pixels per pass, then the remaining 0..15 as 8, 4, 2, 1 — `{n}` =
/// remaining - 16, whose low four bits are that remainder (as kernel 1).
macro_rules! row_loop {
    ($dir:ident, $kind:ident) => {
        concat!(
            "2:\n",
            row_start!($dir),
            "subs {n}, {w}, #16\n",
            "b.lo 4f\n",
            "3:\n",
            chunk!($dir, $kind, "{{v0.4s, v1.4s, v2.4s, v3.4s}}", "{{v4.4s, v5.4s, v6.4s, v7.4s}}", r4, "64"),
            "subs {n}, {n}, #16\n",
            "b.hs 3b\n",
            "4:\n",
            "tbz {n}, #3, 5f\n",
            chunk!($dir, $kind, "{{v0.4s, v1.4s}}", "{{v4.4s, v5.4s}}", r2, "32"),
            "5:\n",
            "tbz {n}, #2, 6f\n",
            chunk!($dir, $kind, "{{v0.4s}}", "{{v4.4s}}", r1, "16"),
            "6:\n",
            "tbz {n}, #1, 7f\n",
            chunk!($dir, $kind, "{{v0.2s}}", "{{v4.2s}}", r1, "8"),
            "7:\n",
            "tbz {n}, #0, 8f\n",
            chunk!($dir, $kind, "{{v0.s}}[0]", "{{v4.s}}[0]", r1, "4"),
            "8:\n",
            "add {src}, {src}, {step}\n",
            "add {dst}, {dst}, {step}\n",
            "subs {h}, {h}, #1\n",
            "b.ne 2b",
        )
    };
}

/// The copy kernel (`S_ONLY`). V registers `v0..v3` — saved and restored.
macro_rules! copy_kernel {
    ($dir:ident, $src:ident, $dst:ident, $step:ident, $w:ident, $h:ident, $save:ident) => {
        core::arch::asm!(
            ".arch armv8-a+simd",
            "stp q0, q1, [{save}, #0]",
            "stp q2, q3, [{save}, #32]",
            row_loop!($dir, copy),
            "ldp q0, q1, [{save}, #0]",
            "ldp q2, q3, [{save}, #32]",
            simd_off!(),
            src = inout(reg) $src => _,
            dst = inout(reg) $dst => _,
            step = in(reg) $step,
            w = in(reg) $w,
            h = inout(reg) $h => _,
            save = in(reg) $save,
            p = out(reg) _,
            q = out(reg) _,
            n = out(reg) _,
            options(nostack),
        )
    };
}

/// The op kernel (every op but `S_ONLY`). V registers: `v0..v3` source and
/// result, `v4..v7` destination, `v8`, `v9` scratch, `v10..v13` the op's
/// constants — saved and restored.
macro_rules! rop_kernel {
    ($dir:ident, $src:ident, $dst:ident, $step:ident, $w:ident, $h:ident, $save:ident,
     $c1:ident, $k01:ident, $c3:ident, $k23:ident) => {
        core::arch::asm!(
            ".arch armv8-a+simd",
            "stp q0, q1, [{save}, #0]",
            "stp q2, q3, [{save}, #32]",
            "stp q4, q5, [{save}, #64]",
            "stp q6, q7, [{save}, #96]",
            "stp q8, q9, [{save}, #128]",
            "stp q10, q11, [{save}, #160]",
            "stp q12, q13, [{save}, #192]",
            "dup v10.4s, {c1:w}",
            "dup v11.4s, {k01:w}",
            "dup v12.4s, {c3:w}",
            "dup v13.4s, {k23:w}",
            row_loop!($dir, rop),
            "ldp q0, q1, [{save}, #0]",
            "ldp q2, q3, [{save}, #32]",
            "ldp q4, q5, [{save}, #64]",
            "ldp q6, q7, [{save}, #96]",
            "ldp q8, q9, [{save}, #128]",
            "ldp q10, q11, [{save}, #160]",
            "ldp q12, q13, [{save}, #192]",
            simd_off!(),
            src = inout(reg) $src => _,
            dst = inout(reg) $dst => _,
            step = in(reg) $step,
            w = in(reg) $w,
            h = inout(reg) $h => _,
            save = in(reg) $save,
            c1 = in(reg) $c1,
            k01 = in(reg) $k01,
            c3 = in(reg) $c3,
            k23 = in(reg) $k23,
            p = out(reg) _,
            q = out(reg) _,
            n = out(reg) _,
            options(nostack),
        )
    };
}

/// Copy the `area.w` x `area.h` pixels whose top left is `(sx, sy)` onto
/// `area` of the same `surface`, `D' = raster_op(op, S, D)` — the copy
/// kernel (TASK/V51 part 1). `backward`: each row right to left, else left to
/// right; `bottom_up`: the rows from the last to the first, else from the
/// first. An empty area, or op 5 (`D' = D`), writes nothing.
///
/// # Safety
///
/// Every pixel `surface.base + row * surface.stride_px + col` of the source
/// rectangle (`row in sy..sy + area.h`, `col in sx..sx + area.w`) must be
/// valid for reads of a `u32`, every pixel of `area` valid for reads and
/// writes; a source row and a destination row of different index must not
/// share a byte; where a source and a destination row share bytes, the
/// directions must be those of the per-pixel loop that never reads a pixel
/// it has written (`backward` iff the destination lies right of the source,
/// `bottom_up` iff below — module docs); nothing else may access either
/// rectangle during the call.
pub unsafe fn copy_area(surface: Surface, sx: usize, sy: usize, area: Area, op: u8, backward: bool, bottom_up: bool) {
    let op = op & 0x0f;
    if area.w == 0 || area.h == 0 || op == 5 {
        return;
    }
    let pitch = surface.stride_px * 4;
    let r0 = if bottom_up { area.h - 1 } else { 0 };
    // SAFETY: row `r0` of either rectangle lies in the store (contract).
    let src = unsafe { surface.base.add((sy + r0) * surface.stride_px + sx) };
    // SAFETY: as above.
    let dst = unsafe { surface.base.add((area.y + r0) * surface.stride_px + area.x) };
    // Bytes from one row to the next in the walk; two's complement upwards.
    let step = if bottom_up { pitch.wrapping_neg() } else { pitch };
    let w = area.w;
    let h = area.h;
    // Written by the asm before it is read; never read by Rust.
    let mut save = core::mem::MaybeUninit::<VSave>::uninit();
    let save = save.as_mut_ptr().cast::<u64>();
    // SAFETY (all four arms): the asm reads `h` rows of `w` pixels of the
    // source and the destination and writes those of the destination, rows
    // `step` bytes apart from `src`/`dst` on, each row in blocks that load
    // before they store in the direction the contract fixes; it writes the
    // save area (at most 224 of its 240 bytes), does not touch the stack,
    // saves every V register it uses before its first use and restores it
    // before it ends, and keeps FPCR/FPSR (integer/bitwise SIMD only).
    unsafe {
        if op == 3 {
            if backward {
                copy_kernel!(bwd, src, dst, step, w, h, save);
            } else {
                copy_kernel!(fwd, src, dst, step, w, h, save);
            }
        } else {
            let bit = |i: u32| if op & (1 << i) != 0 { u32::MAX } else { 0 };
            let (c1, c3) = (bit(1), bit(3));
            let (k01, k23) = (bit(0) ^ c1, bit(2) ^ c3);
            if backward {
                rop_kernel!(bwd, src, dst, step, w, h, save, c1, k01, c3, k23);
            } else {
                rop_kernel!(fwd, src, dst, step, w, h, save, c1, k01, c3, k23);
            }
        }
    }
}

/// `copy_rect`: the screen rectangle with top left `(src_x, src_y)` copied
/// to `(dx, dy)` with `op`, written into the clipped destination box `bx`
/// (inclusive, from `ClipRect::clamp_to`) of a `scr_w` x `scr_h` screen —
/// what the backend's per-pixel loop did (module docs, [`copy_rect_reference`]):
/// directions from the start points, rows and columns whose source lies off
/// the screen left alone.
///
/// # Safety
///
/// `bx` lies in `[0, scr_w) x [0, scr_h)`, `scr_w <= surface.stride_px`, the
/// store holds the `scr_h` rows of the screen (`(scr_h - 1) * stride_px +
/// scr_w` pixels at least), nothing else accesses it during the call.
#[allow(clippy::too_many_arguments)]
pub unsafe fn copy_rect(
    surface: Surface,
    scr_w: i32,
    scr_h: i32,
    src_x: i32,
    src_y: i32,
    dx: i32,
    dy: i32,
    bx: (i32, i32, i32, i32),
    op: u8,
) {
    let (tx0, ty0, tx1, ty1) = bx;
    // Destination rows / columns whose source row / column is on the screen:
    // `0 <= src + (t - d) < scr` ⇔ `d - src <= t < d - src + scr`.
    let (ox, oy) = (dx - src_x, dy - src_y);
    let (x0, x1) = (tx0.max(ox), tx1.min(ox + scr_w - 1));
    let (y0, y1) = (ty0.max(oy), ty1.min(oy + scr_h - 1));
    if x0 > x1 || y0 > y1 {
        return;
    }
    let area = Area { x: x0 as usize, y: y0 as usize, w: (x1 - x0 + 1) as usize, h: (y1 - y0 + 1) as usize };
    // The loop's directions: bottom-up if the destination lies below, right
    // to left if it lies right of the source.
    let (backward, bottom_up) = (dx > src_x, dy > src_y);
    // SAFETY: both rectangles lie on the screen in the store (contract and
    // the clipping above); rows of different index share no byte (each lies
    // in `[0, scr_w)` of its row, `scr_w <= stride_px`); the directions are
    // the loop's.
    unsafe { copy_area(surface, (x0 - ox) as usize, (y0 - oy) as usize, area, op, backward, bottom_up) };
}

// ── Rectangle transfer between the screen and a buffer (TASK/V59) ──
//
// `copy_rows` moves a rectangle between the screen and a buffer with a pitch
// of its own: `read_rect` and `write_rect` are the backend's
// `read_device_rect` / `write_device_rect` as plain functions — the save-under
// of the mouse pointer —, each the per-pixel `get_device_pixel` /
// `put_device_pixel` definition with its off-the-screen rule
// (`read_rect_reference`, `write_rect_reference`).
//
// FP context (ADR-036 point 3): `copy_rows` also runs inside the timer
// interrupt when that interrupted EL0 — INPUT01 moves the pointer from there
// (`kernel/src/host_env.rs:567@irq_input_pump`), as the pointer's 1x1 fills
// through kernel 1 already did before TASK/V59. The V registers then hold
// the interrupted program's live values; the save and restore hand them back
// the same way, and no other kernel is running underneath (the pump returns
// unless the interrupt came from EL0, and no trap is in progress there).

/// The row loop of [`copy_rows`]: `row_loop!(fwd, copy)` with a pitch of
/// its own for each side — `{h}` rows of `{w}` pixels, the source rows
/// `{sstep}` bytes apart, the destination rows `{dstep}` bytes apart.
macro_rules! xfer_loop {
    () => {
        concat!(
            "2:\n",
            row_start!(fwd),
            "subs {n}, {w}, #16\n",
            "b.lo 4f\n",
            "3:\n",
            chunk!(fwd, copy, "{{v0.4s, v1.4s, v2.4s, v3.4s}}", "", r4, "64"),
            "subs {n}, {n}, #16\n",
            "b.hs 3b\n",
            "4:\n",
            "tbz {n}, #3, 5f\n",
            chunk!(fwd, copy, "{{v0.4s, v1.4s}}", "", r2, "32"),
            "5:\n",
            "tbz {n}, #2, 6f\n",
            chunk!(fwd, copy, "{{v0.4s}}", "", r1, "16"),
            "6:\n",
            "tbz {n}, #1, 7f\n",
            chunk!(fwd, copy, "{{v0.2s}}", "", r1, "8"),
            "7:\n",
            "tbz {n}, #0, 8f\n",
            chunk!(fwd, copy, "{{v0.s}}[0]", "", r1, "4"),
            "8:\n",
            "add {src}, {src}, {sstep}\n",
            "add {dst}, {dst}, {dstep}\n",
            "subs {h}, {h}, #1\n",
            "b.ne 2b",
        )
    };
}

/// Copy `h` rows of `w` pixels from `src` (rows `src_stride_px` pixels
/// apart) to `dst` (rows `dst_stride_px` pixels apart), each row left to
/// right in blocks of 16/8/4/2/1 pixels — the transfer kernel between the
/// screen and a buffer (TASK/V59). V registers `v0..v3`, saved and restored.
/// An empty rectangle copies nothing.
///
/// # Safety
///
/// Every pixel `src + r * src_stride_px + c` (`r < h`, `c < w`) must be valid
/// for reads of a `u32`, every pixel `dst + r * dst_stride_px + c` valid for
/// writes; the two sets must not share a byte; nothing else may access
/// either during the call.
pub unsafe fn copy_rows(src: *const u32, src_stride_px: usize, dst: *mut u32, dst_stride_px: usize, w: usize, h: usize) {
    if w == 0 || h == 0 {
        return;
    }
    let sstep = src_stride_px * 4;
    let dstep = dst_stride_px * 4;
    // Written by the asm before it is read; never read by Rust.
    let mut save = core::mem::MaybeUninit::<VSave>::uninit();
    let save = save.as_mut_ptr().cast::<u64>();
    // SAFETY: the asm reads `h` rows of `w` pixels from `src` on at `sstep`
    // bytes per row and writes as many to `dst` on at `dstep` bytes per row
    // (contract), writes 64 of the save area's 240 bytes, does not touch the
    // stack, saves `v0..v3` before their first use and restores them before
    // it ends, and keeps FPCR/FPSR (loads and stores only).
    unsafe {
        core::arch::asm!(
            ".arch armv8-a+simd",
            "stp q0, q1, [{save}, #0]",
            "stp q2, q3, [{save}, #32]",
            xfer_loop!(),
            "ldp q0, q1, [{save}, #0]",
            "ldp q2, q3, [{save}, #32]",
            simd_off!(),
            src = inout(reg) src => _,
            dst = inout(reg) dst => _,
            sstep = in(reg) sstep,
            dstep = in(reg) dstep,
            w = in(reg) w,
            h = inout(reg) h => _,
            save = in(reg) save,
            p = out(reg) _,
            q = out(reg) _,
            n = out(reg) _,
            options(nostack),
        )
    };
}

/// The part of the `w` x `h` rectangle with top left `(x, y)` that lies on
/// a `scr_w` x `scr_h` screen, in the rectangle's own coordinates: columns
/// `c0..c1`, rows `r0..r1` (half-open); `None` if nothing of it does.
fn on_screen(scr_w: i32, scr_h: i32, x: i32, y: i32, w: usize, h: usize) -> Option<(usize, usize, usize, usize)> {
    let span = |p: i32, n: usize, scr: i32| {
        let (p, n) = (p as i64, n as i64);
        let lo = (-p).clamp(0, n);
        let hi = (scr as i64 - p).clamp(lo, n);
        (lo as usize, hi as usize)
    };
    let (c0, c1) = span(x, w, scr_w);
    let (r0, r1) = span(y, h, scr_h);
    (c0 < c1 && r0 < r1).then_some((c0, c1, r0, r1))
}

/// `read_device_rect`: the device pixels of the `w` x `h` rectangle with top
/// left `(x, y)` of a `scr_w` x `scr_h` screen into `out`, row `r` at
/// `out[r * w..]`; a position off the screen reads as `0` — exactly
/// `get_device_pixel` per pixel ([`read_rect_reference`]). The on-screen
/// part goes through [`copy_rows`], the rest is zero-filled.
///
/// # Safety
///
/// `scr_w <= surface.stride_px`, the store holds the `scr_h` rows of the
/// screen, nothing else accesses it during the call. Panics if `out` holds
/// fewer than `w * h` pixels.
#[allow(clippy::too_many_arguments)]
pub unsafe fn read_rect(surface: Surface, scr_w: i32, scr_h: i32, x: i32, y: i32, w: usize, h: usize, out: &mut [u32]) {
    let out = &mut out[..w * h];
    let Some((c0, c1, r0, r1)) = on_screen(scr_w, scr_h, x, y, w, h) else {
        out.fill(0);
        return;
    };
    if (c0, c1, r0, r1) != (0, w, 0, h) {
        for (r, row) in out.chunks_exact_mut(w).enumerate() {
            if r < r0 || r >= r1 {
                row.fill(0);
            } else {
                row[..c0].fill(0);
                row[c1..].fill(0);
            }
        }
    }
    let (sx, sy) = ((x + c0 as i32) as usize, (y + r0 as i32) as usize);
    // SAFETY: rows `sy..sy + (r1 - r0)` and columns `sx..sx + (c1 - c0)` lie
    // on the screen (`on_screen`), so in the store (contract); the target
    // rows lie in `out`, which is not the store (a `&mut` borrow of its own).
    unsafe {
        copy_rows(
            surface.base.add(sy * surface.stride_px + sx),
            surface.stride_px,
            out.as_mut_ptr().add(r0 * w + c0),
            w,
            c1 - c0,
            r1 - r0,
        )
    };
}

/// `write_device_rect`: the `w` x `h` device pixels of `src` (row `r` at
/// `src[r * w..]`) onto the rectangle with top left `(x, y)` of a `scr_w` x
/// `scr_h` screen; positions off the screen are dropped — exactly
/// `put_device_pixel` per pixel ([`write_rect_reference`]). No cache
/// maintenance (the caller flushes, as after `put_device_pixel`).
///
/// # Safety
///
/// As [`read_rect`]; `src` is not the store. Panics if `src` holds fewer
/// than `w * h` pixels.
#[allow(clippy::too_many_arguments)]
pub unsafe fn write_rect(surface: Surface, scr_w: i32, scr_h: i32, x: i32, y: i32, w: usize, h: usize, src: &[u32]) {
    let src = &src[..w * h];
    let Some((c0, c1, r0, r1)) = on_screen(scr_w, scr_h, x, y, w, h) else {
        return;
    };
    let (dx, dy) = ((x + c0 as i32) as usize, (y + r0 as i32) as usize);
    // SAFETY: as in `read_rect`, the other way round.
    unsafe {
        copy_rows(
            src.as_ptr().add(r0 * w + c0),
            w,
            surface.base.add(dy * surface.stride_px + dx),
            surface.stride_px,
            c1 - c0,
            r1 - r0,
        )
    };
}

/// The scalar reference of [`read_rect`]: `get_device_pixel` per pixel
/// (bounds check, `0` off the screen) on a plain store. Host builds only.
///
/// # Safety
///
/// As [`read_rect`].
#[cfg(not(target_os = "none"))]
#[allow(clippy::too_many_arguments)]
pub unsafe fn read_rect_reference(
    surface: Surface,
    scr_w: i32,
    scr_h: i32,
    x: i32,
    y: i32,
    w: usize,
    h: usize,
    out: &mut [u32],
) {
    for r in 0..h {
        for c in 0..w {
            let (px, py) = (x + c as i32, y + r as i32);
            out[r * w + c] = if px < 0 || py < 0 || px >= scr_w || py >= scr_h {
                0
            } else {
                // SAFETY: on the screen, so in the store (contract).
                unsafe { surface.base.add(py as usize * surface.stride_px + px as usize).read() }
            };
        }
    }
}

/// The scalar reference of [`write_rect`]: `put_device_pixel` per pixel
/// (bounds check, dropped off the screen) on a plain store. Host builds
/// only.
///
/// # Safety
///
/// As [`write_rect`].
#[cfg(not(target_os = "none"))]
#[allow(clippy::too_many_arguments)]
pub unsafe fn write_rect_reference(
    surface: Surface,
    scr_w: i32,
    scr_h: i32,
    x: i32,
    y: i32,
    w: usize,
    h: usize,
    src: &[u32],
) {
    for r in 0..h {
        for c in 0..w {
            let (px, py) = (x + c as i32, y + r as i32);
            if px < 0 || py < 0 || px >= scr_w || py >= scr_h {
                continue;
            }
            // SAFETY: on the screen, so in the store (contract).
            unsafe { surface.base.add(py as usize * surface.stride_px + px as usize).write(src[r * w + c]) };
        }
    }
}

/// The scalar reference of [`copy_area`]: per pixel in the walk's order,
/// `raster_op` on a plain store. Host builds only.
///
/// # Safety
///
/// As [`copy_area`].
#[cfg(not(target_os = "none"))]
pub unsafe fn copy_area_reference(
    surface: Surface,
    sx: usize,
    sy: usize,
    area: Area,
    op: u8,
    backward: bool,
    bottom_up: bool,
) {
    for ri in 0..area.h {
        let r = if bottom_up { area.h - 1 - ri } else { ri };
        for ci in 0..area.w {
            let c = if backward { area.w - 1 - ci } else { ci };
            // SAFETY: inside the two rectangles (contract).
            unsafe {
                let s = surface.base.add((sy + r) * surface.stride_px + sx + c).read();
                let d = surface.base.add((area.y + r) * surface.stride_px + area.x + c);
                d.write(raster_op(op, s, d.read()));
            }
        }
    }
}

/// The scalar reference of [`copy_rect`]: `VdiScreen::copy_rect`'s loop
/// before TASK/V51, on a plain store. Host builds only.
///
/// # Safety
///
/// As [`copy_rect`].
#[cfg(not(target_os = "none"))]
#[allow(clippy::too_many_arguments)]
pub unsafe fn copy_rect_reference(
    surface: Surface,
    scr_w: i32,
    scr_h: i32,
    src_x: i32,
    src_y: i32,
    dx: i32,
    dy: i32,
    bx: (i32, i32, i32, i32),
    op: u8,
) {
    let (tx0, ty0, tx1, ty1) = bx;
    let stride = surface.stride_px;
    let base = surface.base;
    let down = dy > src_y;
    let right = dx > src_x;
    for ri in 0..=(ty1 - ty0) {
        let ty = if down { ty1 - ri } else { ty0 + ri };
        let sy = src_y + (ty - dy);
        if sy < 0 || sy >= scr_h {
            continue;
        }
        for ci in 0..=(tx1 - tx0) {
            let tx = if right { tx1 - ci } else { tx0 + ci };
            let sx = src_x + (tx - dx);
            if sx < 0 || sx >= scr_w {
                continue;
            }
            // SAFETY: indices are on the screen (contract).
            unsafe {
                let sp = base.add(sy as usize * stride + sx as usize).read();
                let dp = base.add(ty as usize * stride + tx as usize);
                let val = raster_op(op, sp, dp.read());
                dp.write(val);
            }
        }
    }
}
