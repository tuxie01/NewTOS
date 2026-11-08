//! Lines of the VDI screen backend without a per-pixel detour (ADR-036,
//! TASK/V58): `VdiScreen::abline` (`crate::vdi_backend`) on the frame of
//! kernel 1 ([`crate::raster_kernel`], TASK/V49).
//!
//! # Which pixels
//!
//! Exactly the shared walk's, `tos_api::vdi::line::abline` (EmuTOS
//! `abline`, `vdi/vdi_line.c:2266-2362`, with `clip_line`, the XOR rule and
//! `LSTLIN` as `last`): its run form `tos_api::vdi::line::abline_runs` hands
//! over the same store indices, dash bits and order as one span (horizontal),
//! one column (vertical) or single pixels (any other), and the store
//! rectangle they touch. Host tests compare the run form with the walk
//! (`tos-api/src/vdi/line.rs`) and this file's [`abline`] with
//! [`abline_reference`], the backend's per-pixel path before V58
//! (`tos-api/tests/raster_kernel_line.rs`).
//!
//! # How
//!
//! The pixel store, the two device colours and the write mode are resolved
//! once per segment; the mode picks one of four monomorphic stores
//! ([`Combine`]), so no pixel goes through a `dyn`, a mode match or a bounds
//! check — the run form keeps every index inside the store.
//!
//! - **Span** (horizontal): kernel 1's row path, `raster_kernel::fill_span`,
//!   with the dash word as a one-row pattern started at the span's phase —
//!   the dash bit of pixel `x` is `mask & (0x8000 >> (x & 15))`, exactly the
//!   pattern rule of kernel 1. The NEON rules (integer/bitwise SIMD only,
//!   every V register saved and restored, leaf `asm!`) are kernel 1's,
//!   unchanged; no new assembler here.
//! - **Column** (vertical): one stride loop, the dash bit one rotating word.
//! - **Pixels** (any other): the run form's Bresenham steps the index by `1`
//!   and `±stride`; each pixel is one combine.
//!
//! Each combine is `VdiScreen::combine_index`'s (`kernel/src/vdi_backend.rs`):
//! volatile, `Xor` complements the stored pixel.

use core::marker::PhantomData;

use tos_api::vdi::backend::{ClipRect, WriteMode};
use tos_api::vdi::line::{abline_runs, LineRuns, LineTarget, Touched};

use crate::raster_kernel;

/// One write mode's combine of one pixel — `VdiScreen::combine_index`'s
/// arms, one type each, so the walk is compiled once per mode.
trait Combine {
    /// The mode, for kernel 1's spans.
    const MODE: WriteMode;
    /// Combine dash bit `set` into the pixel at `p`.
    ///
    /// # Safety
    ///
    /// `p` must be valid for a `u32` read and write.
    unsafe fn put(p: *mut u32, set: bool, fg: u32, bg: u32);
}

/// `fg` on set bits, `bg` on clear ones.
struct Replace;
/// `fg` on set bits.
struct Transparent;
/// The stored pixel complemented on set bits.
struct Xor;
/// `fg` on clear bits.
struct RevTransparent;

impl Combine for Replace {
    const MODE: WriteMode = WriteMode::Replace;
    #[inline(always)]
    unsafe fn put(p: *mut u32, set: bool, fg: u32, bg: u32) {
        // SAFETY: the caller's contract.
        unsafe { p.write_volatile(if set { fg } else { bg }) };
    }
}

impl Combine for Transparent {
    const MODE: WriteMode = WriteMode::Transparent;
    #[inline(always)]
    unsafe fn put(p: *mut u32, set: bool, fg: u32, _bg: u32) {
        if set {
            // SAFETY: the caller's contract.
            unsafe { p.write_volatile(fg) };
        }
    }
}

impl Combine for Xor {
    const MODE: WriteMode = WriteMode::Xor;
    #[inline(always)]
    unsafe fn put(p: *mut u32, set: bool, _fg: u32, _bg: u32) {
        if set {
            // SAFETY: the caller's contract.
            unsafe { p.write_volatile(!p.read_volatile()) };
        }
    }
}

impl Combine for RevTransparent {
    const MODE: WriteMode = WriteMode::RevTransparent;
    #[inline(always)]
    unsafe fn put(p: *mut u32, set: bool, fg: u32, _bg: u32) {
        if !set {
            // SAFETY: the caller's contract.
            unsafe { p.write_volatile(fg) };
        }
    }
}

/// The pixel store a segment is drawn into, in mode `M`.
struct Store<M> {
    /// Store index 0.
    base: *mut u32,
    fg: u32,
    bg: u32,
    mode: PhantomData<M>,
}

impl<M: Combine> LineRuns for Store<M> {
    #[inline]
    fn span(&mut self, idx: usize, n: usize, phase: u32, mask: u16) {
        // SAFETY: `abline_runs` hands out store indices only, and the store
        // is valid (the contract of `abline`, which built this `Store`).
        unsafe { raster_kernel::fill_span(self.base.add(idx), n, phase, mask, M::MODE, self.fg, self.bg) };
    }

    #[inline]
    fn column(&mut self, idx: usize, n: usize, step: isize, phase: u32, mask: u16) {
        let mut p = self.base.wrapping_add(idx);
        let mut bit = 0x8000u16 >> (phase & 15);
        for _ in 0..n {
            // SAFETY: each `idx + i * step`, `i < n`, is a store index
            // (`abline_runs`), so `p` points into the valid store.
            unsafe { M::put(p, mask & bit != 0, self.fg, self.bg) };
            bit = bit.rotate_right(1);
            p = p.wrapping_offset(step);
        }
    }

    #[inline]
    fn pixel(&mut self, idx: usize, set: bool) {
        // SAFETY: as in `span`.
        unsafe { M::put(self.base.add(idx), set, self.fg, self.bg) };
    }
}

/// Draw segment `seg` = `(x0, y0, x1, y1)` into the store at `base`, laid out
/// as `target`, in `mode` with the device pixels `fg` (pen) and `bg` (pen 0,
/// `Replace`'s dash gaps) — `RasterBackend::abline`'s pixels
/// (`tos_api::vdi::line::abline`). Returns `LN_MASK` after the segment and
/// the store rectangle the segment touched (`None`: no pixel), which the
/// caller cleans.
///
/// # Safety
///
/// `base` must point to `target.height * target.stride` `u32`s, valid for
/// reads and writes and not accessed by anything else during the call.
#[allow(clippy::too_many_arguments)]
pub unsafe fn abline(
    base: *mut u32,
    target: &LineTarget,
    seg: (i16, i16, i16, i16),
    mode: WriteMode,
    clip: ClipRect,
    mask: u16,
    last: bool,
    fg: u32,
    bg: u32,
) -> (u16, Option<Touched>) {
    let (x0, y0, x1, y1) = seg;
    match mode {
        WriteMode::Replace => {
            let mut store = Store::<Replace> { base, fg, bg, mode: PhantomData };
            abline_runs(target, x0, y0, x1, y1, mode, clip, mask, last, &mut store)
        }
        WriteMode::Transparent => {
            let mut store = Store::<Transparent> { base, fg, bg, mode: PhantomData };
            abline_runs(target, x0, y0, x1, y1, mode, clip, mask, last, &mut store)
        }
        WriteMode::Xor => {
            let mut store = Store::<Xor> { base, fg, bg, mode: PhantomData };
            abline_runs(target, x0, y0, x1, y1, mode, clip, mask, last, &mut store)
        }
        WriteMode::RevTransparent => {
            let mut store = Store::<RevTransparent> { base, fg, bg, mode: PhantomData };
            abline_runs(target, x0, y0, x1, y1, mode, clip, mask, last, &mut store)
        }
    }
}

/// The scalar reference of [`abline`]: `VdiScreen::abline` before TASK/V58
/// on a plain store — the shared walk `tos_api::vdi::line::abline`, each
/// pixel through `combine_index`'s bounds check and arms, the touched
/// rectangle from `index % stride` and `index / stride` per pixel. Host
/// builds only — the host tests compare [`abline`] against it (ADR-036
/// point 4); the kernel never runs it.
///
/// # Safety
///
/// As [`abline`].
#[cfg(not(target_os = "none"))]
#[allow(clippy::too_many_arguments)]
pub unsafe fn abline_reference(
    base: *mut u32,
    target: &LineTarget,
    seg: (i16, i16, i16, i16),
    mode: WriteMode,
    clip: ClipRect,
    mask: u16,
    last: bool,
    fg: u32,
    bg: u32,
) -> (u16, Option<Touched>) {
    let (x0, y0, x1, y1) = seg;
    let stride = target.stride;
    let mut touched: Option<Touched> = None;
    let mask = tos_api::vdi::line::abline(target, x0, y0, x1, y1, mode, clip, mask, last, |idx, set| {
        if idx < target.height as usize * stride as usize {
            // SAFETY: inside the store (checked), which the caller guarantees.
            unsafe {
                let p = base.add(idx);
                match mode {
                    WriteMode::Replace => p.write_volatile(if set { fg } else { bg }),
                    WriteMode::Transparent if set => p.write_volatile(fg),
                    WriteMode::Xor if set => p.write_volatile(!p.read_volatile()),
                    WriteMode::RevTransparent if !set => p.write_volatile(fg),
                    _ => {}
                }
            }
        }
        let (col, row) = ((idx % stride as usize) as u32, (idx / stride as usize) as u32);
        touched = Some(match touched {
            None => (col, row, col, row),
            Some((c0, r0, c1, r1)) => (c0.min(col), r0.min(row), c1.max(col), r1.max(row)),
        });
    });
    (mask, touched)
}
