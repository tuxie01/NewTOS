//! VDI raster operations — `vrt_cpyfm` (transparent mono→colour blit)
//! and `vro_cpyfm` (opaque copy). Spec: `docs/api/vdi/raster/_raster.md`.
//!
//! **MFDB pointer convention (caller-ABI aware).** gemlib packs VDI
//! pointer `n` at `CONTRL[7 + n * N_PTRINTS]`, with `N_PTRINTS =
//! sizeof(void*) / sizeof(short)` (`gem_vdiP.h:177,204`). For an **LP64**
//! caller `N_PTRINTS = 4`, so the source MFDB pointer occupies
//! `CONTRL[7..10]` and the destination `CONTRL[11..14]` (8 bytes each).
//! For an **ILP32** caller `N_PTRINTS = 2`: source at `CONTRL[7..8]`,
//! destination at `CONTRL[9..10]` (4 bytes each). Both the pointer read
//! and the MFDB struct-field offsets shift with the ABI — see
//! [`Vdipb::contrl_ptr`] and [`Mfdb::read`]. `fd_addr == NULL` means
//! the screen device under either width.
//!
//! Scope: `vrt_cpyfm` with a 1-plane source (icons/sprites), all four
//! `MD_*` write modes (replace / transparent / XOR / erase); `vro_cpyfm`
//! for all four source/destination combinations with the full 16-entry
//! logic-op table — Screen↔Screen, **Screen→Memory** (AES `bb_save`),
//! **Memory→Screen** (`bb_restore`), and **Memory↔Memory**.
//!
//! **NewTOS memory-MFDB format (device form, `fd_stand = 0`).** A memory
//! MFDB (`fd_addr != 0`) is a **16-bit RGB565-chunky truecolor** buffer:
//! one `UWORD` (`RRRRRGGGGG0BBBBB`, the green LSB / bit 5 always 0 — the
//! Falcon truecolor pixel, `vdi_defs.h:156`, `vdi_col.c:599`) per pixel.
//! This is the app-visible **wireformat** EmuTOS' `vro_cpyfm16` /
//! `vrt_cpyfm16` operate on raw (`vdi_raster.c:975-1221,1226-1326`): a
//! Falcon program that pokes RGB565 pixels into its own MFDB and blits
//! expects exactly this layout — it is observable, not an implementation
//! detail (`docs/COMPATIBILITY.md`). The row stride is derived from
//! `fd_wdwidth` (16-pixel words, so the row is rounded up to a multiple of
//! 16 px, mirroring EmuTOS `bb_set`'s word alignment) — a memory copy uses
//! the MFDB's *own* geometry, never the screen stride. `fd_nplanes = 16`
//! marks the depth ([`CHUNKY_PLANES`]); with it the classic interleaved
//! identity `bytes/line = fd_wdwidth * fd_nplanes / 8 * 16 = fd_wdwidth *
//! 32` holds.
//!
//! **Below nine planes a memory form is planar (V19).** A memory MFDB with
//! `fd_nplanes` 1..8 is EmuTOS' interleaved device form — per row
//! `fd_wdwidth` groups of `fd_nplanes` WORDs, one WORD per plane, pixel `x`
//! at bit `15 - x % 16` (`setup_info`, `vdi_raster.c:919-940,959-960`). A
//! `vro_cpyfm` between two such forms is EmuTOS' planar BitBlt
//! ([`planar_mem_to_mem`]), not the chunky loop, and so is a `vrt_cpyfm`
//! into one (V28, [`planar_blit`]); the screen is never such a form (16
//! planes), so only a memory destination reaches it. A chunky memory form
//! of 9..15 planes has a row of `fd_wdwidth * fd_nplanes` WORDs, as EmuTOS'
//! `vro_cpyfm16`/`vrt_cpyfm16` walk it (V28, [`Mfdb::mem_stride_px`]).
//!
//! The **physical framebuffer stays 32-bpp XRGB8888** (an internal detail
//! per `docs/COMPATIBILITY.md`); the 16↔32 conversion happens only at the
//! screen boundary inside [`screen_to_mem`] / [`mem_to_screen`], via the
//! backend's [`RasterBackend::device_to_rgb565`] /
//! [`RasterBackend::rgb565_to_device`]. Because the screen is
//! 5-bit-quantised (every VDI draw resolves through the RGB555-in-565
//! pseudo-palette) and the backend keeps the word's `OVERLAY_BIT` in the
//! device pixel's X byte (V11/V12, 2026-09-21), that round-trip is
//! **lossless** for every 16-bit word, so `bb_save`/`bb_restore` reproduce
//! the screen bit-for-bit — a saved `v_clrwk`-cleared area holds `0xFFFF`
//! words, as on a Falcon.
//!
//! The buffer is caller-owned (AES `bb_save` allocates it); the VDI only
//! reads/writes through `fd_addr`. A memory form is addressed as EmuTOS does
//! (V26): pixel `(x, y)` is WORD `y * fd_wdwidth * fd_nplanes + x` for any `x`, so
//! `fd_w` is no limit (a block wider than the row runs on into the next row,
//! the padding columns are pixels too); the one bound is the form itself,
//! its `fd_h` rows, and of those only what the call bound is touched
//! ([`Mfdb::bind_block`]).

use alloc::alloc::{alloc_zeroed, dealloc, Layout};

use super::backend::{ClipRect, MonoBits, MonoSrc, RasterBackend, Rect, WriteMode};
use super::vwk::{validate_color_index, Vwk};
use super::Vdipb;
use crate::bdos::process::Abi;

/// The `S_ONLY` logic op (`INTIN[0] = 3`, `D' = S`, replace) — the op AES
/// `bb_save`/`bb_restore` use (EmuTOS `gemgsxif.c` `bb_set`).
const OP_REPLACE: u8 = 3;

/// The plane count of a NewTOS device-format memory MFDB (`fd_nplanes`) —
/// **16**, the Falcon truecolor depth (`v_planes`, `bios/videl.c:320-341`).
/// The screen resolves to the same count via
/// [`RasterBackend::planes`](super::backend::RasterBackend::planes). Used
/// both to stamp `device_mem` MFDBs and as the `setup_info`/`cpy_raster`
/// plane-equality gate for `vro_cpyfm` (`vdi_raster.c:963,1360`, AUD-V5-004).
const CHUNKY_PLANES: i16 = 16;

/// The largest plane count EmuTOS still handles on its **palette** BitBlt
/// path. `cpy_raster` dispatches to the truecolor `vro_cpyfm16` /
/// `vrt_cpyfm16` routines for `plane_ct > 8` (`vdi_raster.c:1363-1368,
/// 1432-1437`), and those two re-read `INTIN[0]` unstripped — which is why
/// the threshold is observable and not just an internal detail.
const PALETTE_PLANES_MAX: i16 = 8;

/// Memory Form Definition Block (fixed ABI; native pointer in `addr`).
/// `stand`/`nplanes` are parsed for completeness (they describe the form
/// but the pixel walk only needs `addr`/`w`/`h`/`wdwidth`).
#[allow(dead_code)]
struct Mfdb {
    addr: u64, // 0 = screen
    w: i16,
    h: i16,
    wdwidth: i16,
    stand: i16,
    nplanes: i16,
}

impl Mfdb {
    /// Read an MFDB from its pointer at the caller's `abi` width (0 ⇒ a
    /// null/screen MFDB). The struct is `void *fd_addr; short fd_w, fd_h,
    /// fd_wdwidth, fd_stand, fd_nplanes, …` (`mt_gem.h:2757-2768`), so the
    /// pointer width **and** the following WORD-field offsets shift with
    /// the ABI: LP64 → `fd_addr` 8B @0, fields @8/10/12/14/16; ILP32 →
    /// `fd_addr` 4B @0, fields @4/6/8/10/12. A NULL `fd_addr` (screen
    /// MFDB) reads as `addr == 0` under either width.
    ///
    /// The struct is read in the caller's space ([`Vdipb::space`], ARCH03
    /// stage 3a); `None` — fault recorded on `pb` — when the program may not
    /// read it. `fd_addr` stays the caller's address: a memory MFDB's bitmap
    /// is reached only after [`Self::bind`].
    fn read(pb: &Vdipb, ptr: u64) -> Option<Self> {
        if ptr == 0 {
            return Some(Mfdb { addr: 0, w: 0, h: 0, wdwidth: 0, stand: 0, nplanes: 0 });
        }
        if pb.faulted() {
            return None;
        }
        let (aw, len) = match pb.abi() {
            Abi::Lp64 => (8usize, 18usize),
            Abi::Ilp32 => (4, 14),
        };
        let mut b = [0u8; 18];
        if pb.space().copy_from(&mut b[..len], ptr).is_err() {
            pb.note_fault();
            return None;
        }
        let word = |off: usize| i16::from_ne_bytes([b[off], b[off + 1]]);
        let addr = match pb.abi() {
            Abi::Lp64 => u64::from_ne_bytes(b[..8].try_into().unwrap_or([0; 8])),
            Abi::Ilp32 => u64::from(u32::from_ne_bytes(b[..4].try_into().unwrap_or([0; 4]))),
        };
        Some(Mfdb {
            addr,
            w: word(aw),
            h: word(aw + 2),
            wdwidth: word(aw + 4),
            stand: word(aw + 6),
            nplanes: word(aw + 8),
        })
    }

    /// Make a **memory** MFDB's bitmap usable by the pixel walks: check that
    /// the caller may access the `bytes` from byte `off` of its bitmap
    /// (`write`: read and write) with its own rights, and replace `addr` by
    /// the bitmap's kernel address (`Space::kptr`, plan §3.6 — the blit then
    /// never dereferences a user address). A screen MFDB and an empty range
    /// need nothing. `false` — fault recorded on `pb` — when the program may
    /// not use the bitmap: the op must not touch it.
    ///
    /// Only the part the op will touch is checked (`off`/`bytes`), as EmuTOS
    /// only touches those rows: an MFDB whose `fd_h` overstates its buffer
    /// still works for a copy that stays inside the real buffer.
    fn bind(&mut self, pb: &Vdipb, off: u64, bytes: u64, write: bool) -> bool {
        if self.addr == 0 {
            return true;
        }
        if bytes == 0 {
            // Nothing will be touched; keep the address inert but kernel-side
            // (no access follows).
            return true;
        }
        match pb.space().kptr(self.addr + off, bytes, write) {
            Ok(k) => {
                self.addr = k - off;
                true
            }
            Err(_) => {
                pb.note_fault();
                false
            }
        }
    }

    /// [`Self::bind`] for the bytes a `w × h` block at `(x, y)` of a
    /// 16-bit chunky memory form can touch: the pixels of
    /// [`Self::chunky_hull`] (V26). A screen MFDB and an empty block need
    /// nothing.
    fn bind_block(&mut self, pb: &Vdipb, x: i32, y: i32, w: i32, h: i32, write: bool) -> bool {
        if self.is_screen() {
            return true;
        }
        match self.chunky_hull(x, y, w, h) {
            Some((lo, hi)) => self.bind(pb, 2 * lo as u64, 2 * (hi - lo) as u64, write),
            None => true,
        }
    }

    /// [`Self::bind`] for the WORDs a `w × h` block at `(x, y)` of an
    /// interleaved (planar) memory form can touch: the groups of
    /// [`Self::planar_hull`], `fd_nplanes` WORDs each (V19, V26).
    fn bind_planar_block(&mut self, pb: &Vdipb, x: i32, y: i32, w: i32, h: i32, write: bool) -> bool {
        let group_bytes = 2 * self.nplanes.max(0) as u64;
        match self.planar_hull(x, y, w, h) {
            Some((lo, hi)) => self.bind(pb, lo as u64 * group_bytes, (hi - lo) as u64 * group_bytes, write),
            None => true,
        }
    }

    /// Build a NewTOS 16-bit RGB565 device-format memory MFDB over a
    /// caller-owned pixel buffer of `w × h` `u16` pixels (AES `bb_save`):
    /// `stand = 0` (device), `nplanes = 16` ([`CHUNKY_PLANES`]),
    /// `wdwidth = (w + 15) / 16`.
    fn device_mem(addr: u64, w: i16, h: i16) -> Self {
        Mfdb { addr, w, h, wdwidth: mem_wdwidth(w), stand: 0, nplanes: CHUNKY_PLANES }
    }

    fn is_screen(&self) -> bool {
        self.addr == 0
    }

    /// Row stride in **pixels** of a chunky memory MFDB (one `u16` per
    /// pixel): `fd_wdwidth * fd_nplanes` WORDs (V28). EmuTOS' `setup_info`
    /// sets the row length `s_nxln`/`d_nxln = fd_wdwidth * fd_nplanes * 2`
    /// bytes (`vdi_raster.c:919-923,933-938`) and `vro_cpyfm16` /
    /// `vrt_cpyfm16` step a row by `nxln / sizeof(WORD)` WORDs (`:986-987`,
    /// `:1249`), so a form of 9..15 planes has a row of `fd_wdwidth * 9..15`
    /// WORDs — shorter than its `fd_wdwidth * 16` pixels — and only at 16
    /// planes (a `device_mem` form, the screen depth) is it `fd_wdwidth * 16`.
    /// The copy uses the MFDB's own geometry — never the screen stride. Below
    /// nine planes a memory form is planar ([`PlanarForm`]) and never takes
    /// this stride.
    fn mem_stride_px(&self) -> usize {
        (self.wdwidth.max(0) as usize) * (self.nplanes.max(0) as usize)
    }

    /// The pixels of the form, `fd_h` rows of [`Self::mem_stride_px`] (V26,
    /// V28).
    ///
    /// EmuTOS lays a memory form out by `fd_wdwidth` and `fd_nplanes` alone:
    /// `setup_info` sets the row length `d_nxln = fd_wdwidth * fd_nplanes * 2`
    /// bytes (`vdi_raster.c:919-923,933-938`), `fd_wdwidth * fd_nplanes`
    /// WORDs, and `vro_cpyfm16`/`vrt_cpyfm16` address pixel `(x, y)` as
    /// `form + y * width + x` (`:986-990`, `:1241-1250`) without reading
    /// `fd_w` or `fd_h`. NewTOS uses the same address — so column `x` may lie
    /// past the row (it is a pixel of a following row) or before it — and
    /// only refuses what lies outside these `fd_h` rows: the kernel never
    /// touches memory past the form (`docs/COMPATIBILITY.md`, V26).
    #[inline]
    fn mem_span_px(&self) -> i64 {
        self.mem_stride_px() as i64 * self.h.max(0) as i64
    }

    /// The linear pixel range `[lo, hi)` a `w × h` block at `(x, y)` can
    /// touch, clipped to the form ([`Self::mem_span_px`]); `None` when it
    /// misses the form. Pixel `(x + c, y + r)` is `(y + r) * stride + x + c`,
    /// so every pixel of the block lies between its first and its last.
    fn chunky_hull(&self, x: i32, y: i32, w: i32, h: i32) -> Option<(i64, i64)> {
        if w <= 0 || h <= 0 {
            return None;
        }
        let st = self.mem_stride_px() as i64;
        let lo = (y as i64 * st + x as i64).max(0);
        let hi = ((y as i64 + h as i64 - 1) * st + x as i64 + w as i64).min(self.mem_span_px());
        (lo < hi).then_some((lo, hi))
    }

    /// The linear 16-pixel-group range `[lo, hi)` a `w × h` block at `(x, y)`
    /// of an interleaved form can touch, clipped to the form's
    /// `fd_h * fd_wdwidth` groups; `None` when it misses the form. Group `g`
    /// of row `y` is linear group `y * fd_wdwidth + g` (`s_nxln = fd_wdwidth
    /// * s_nxwd`, `vdi_raster.c:922-923,937-938`).
    fn planar_hull(&self, x: i32, y: i32, w: i32, h: i32) -> Option<(i64, i64)> {
        if w <= 0 || h <= 0 {
            return None;
        }
        let wd = self.wdwidth.max(0) as i64;
        let (x, y) = (x as i64, y as i64);
        let lo = (y * wd + x.div_euclid(16)).max(0);
        let hi = ((y + h as i64 - 1) * wd + (x + w as i64 - 1).div_euclid(16) + 1).min(wd * self.h.max(0) as i64);
        (lo < hi).then_some((lo, hi))
    }

    /// The word index of pixel `(col, row)`, `row * fd_wdwidth * fd_nplanes + col`,
    /// or `None` outside the form ([`Self::mem_span_px`]).
    #[inline]
    fn mem_index(&self, col: i32, row: i32) -> Option<usize> {
        let idx = row as i64 * self.mem_stride_px() as i64 + col as i64;
        (0..self.mem_span_px()).contains(&idx).then_some(idx as usize)
    }

    /// Read the 16-bit RGB565 pixel at `(col, row)` of a memory MFDB, or `0`
    /// outside the form ([`Self::mem_index`]).
    ///
    /// # Safety
    /// `addr` must reach the word, i.e. the call bound it
    /// ([`Self::bind_block`] over a block containing the pixel).
    #[inline]
    unsafe fn get_mem(&self, col: i32, row: i32) -> u16 {
        let Some(idx) = self.mem_index(col, row) else {
            return 0;
        };
        // SAFETY: `idx` is inside the form, and bound by the caller.
        unsafe { (self.addr as *const u16).add(idx).read_unaligned() }
    }

    /// Write the 16-bit RGB565 pixel at `(col, row)` of a memory MFDB; a
    /// pixel outside the form is ignored.
    ///
    /// # Safety
    /// As [`get_mem`](Self::get_mem).
    #[inline]
    unsafe fn put_mem(&self, col: i32, row: i32, val: u16) {
        let Some(idx) = self.mem_index(col, row) else {
            return;
        };
        // SAFETY: `idx` is inside the form, and bound by the caller.
        unsafe { (self.addr as *mut u16).add(idx).write_unaligned(val) }
    }

    /// The columns of row `row` that [`get_mem`](Self::get_mem) /
    /// [`put_mem`](Self::put_mem) accept, as `(start, cmin, cmax)`: column
    /// `c` is inside iff `cmin <= c < cmax`, at word index `start + c`
    /// (`start = row * fd_wdwidth * fd_nplanes`, `cmin = -start`, `cmax =
    /// mem_span_px - start`). One interval per row for the per-pixel checks
    /// (GUI07); a column past the row is a pixel of a later row (V26).
    #[inline]
    fn row_cols(&self, row: i32) -> (i64, i64, i64) {
        let start = row as i64 * self.mem_stride_px() as i64;
        (start, -start, self.mem_span_px() - start)
    }

    /// Read `out.len()` pixels of memory row `row` from column `col` on:
    /// `out[i] = get_mem(col + i, row)` for every `i`, a pixel outside the
    /// extent reading `0` — the row form of [`get_mem`](Self::get_mem).
    ///
    /// # Safety
    /// As [`get_mem`](Self::get_mem); `out` must not overlap the buffer.
    #[inline]
    unsafe fn read_row(&self, col: i32, row: i32, out: &mut [u16]) {
        let (start, cmin, cmax) = self.row_cols(row);
        let (lo, hi) = span(col, out.len(), cmin, cmax);
        out[..lo].fill(0);
        out[hi..].fill(0);
        if lo < hi {
            // SAFETY: columns `col + lo .. col + hi` lie in `cmin..cmax`, so the
            // words `start + col + lo ..` are inside the form (and bound). The
            // buffer may be unaligned (`fd_addr` is the caller's), hence a
            // byte copy; `out` is a separate (stack) buffer.
            unsafe {
                let first = (self.addr as *const u16).add((start + col as i64 + lo as i64) as usize);
                core::ptr::copy_nonoverlapping(
                    first as *const u8,
                    out[lo..hi].as_mut_ptr() as *mut u8,
                    (hi - lo) * 2,
                );
            }
        }
    }

    /// Combine `s` into memory row `row` from column `col` on with logic op
    /// `op`: for every `i` whose pixel lies inside the extent,
    /// `put_mem(col + i, row, raster_op(op, s[i], get_mem(col + i, row)))`;
    /// pixels outside are left alone — the row form of the per-pixel
    /// read-op-write. Pixels of one row are distinct words, so reading the
    /// whole run before writing it is the same as alternating per pixel. The
    /// destination is read only when the op depends on it
    /// ([`op_ignores_dst`]); `BM_D_ONLY` writes back what it read and is
    /// skipped (EmuTOS `vro_cpyfm16` "nothing to do", `vdi_raster.c:1083-1085`).
    ///
    /// # Safety
    /// As [`put_mem`](Self::put_mem); `s.len() <= ROW_CHUNK`, and `s` must
    /// not overlap the buffer.
    #[inline]
    unsafe fn combine_row(&self, col: i32, row: i32, s: &[u16], op: u8) {
        if op & 0x0f == 5 {
            return;
        }
        let (start, cmin, cmax) = self.row_cols(row);
        let (lo, hi) = span(col, s.len(), cmin, cmax);
        if lo >= hi {
            return;
        }
        let n = hi - lo;
        let mut d = [0u16; ROW_CHUNK];
        let d = &mut d[..n];
        // SAFETY: as in `read_row` — columns `col + lo .. col + hi` are inside
        // the form, so every word is (and is bound); byte copies because the
        // caller's buffer may be unaligned; `d` is on the stack.
        unsafe {
            let first = (self.addr as *mut u16).add((start + col as i64 + lo as i64) as usize) as *mut u8;
            if !op_ignores_dst(op) {
                core::ptr::copy_nonoverlapping(first as *const u8, d.as_mut_ptr() as *mut u8, n * 2);
            }
            apply_row(op, &s[lo..hi], d);
            core::ptr::copy_nonoverlapping(d.as_ptr() as *const u8, first, n * 2);
        }
    }
}

/// The index window `[lo, hi)` of a run of `n` pixels starting at column
/// `col` whose columns satisfy `cmin <= col + i < cmax` (`lo <= hi <= n`).
#[inline]
fn span(col: i32, n: usize, cmin: i64, cmax: i64) -> (usize, usize) {
    let col = col as i64;
    let n = n as i64;
    let lo = (cmin - col).clamp(0, n);
    let hi = (cmax - col).clamp(lo, n);
    (lo as usize, hi as usize)
}

/// Pixels per row chunk of the row-wise copy paths (GUI07): the stack
/// buffers the screen and memory runs are staged in. A row longer than this
/// is processed in consecutive chunks.
const ROW_CHUNK: usize = 256;

/// Whether `raster_op(op, s, d)` is independent of `d` — the ops that need
/// no destination read. Derived from [`raster_op`]'s table, not chosen:
/// `0` (`D' = 0`), `3` (`D' = S`), `12` (`D' = ~S`), `15` (`D' = ~0`) are
/// the only four arms that do not mention `d`. Pinned exhaustively against
/// the table by a host test. EmuTOS' `vro_cpyfm16` likewise never reads the
/// destination in these four arms (`vdi_raster.c:1009-1019,1048-1068,
/// 1167-1180,1209-1219`).
#[inline]
fn op_ignores_dst(op: u8) -> bool {
    matches!(op & 0x0f, 0 | 3 | 12 | 15)
}

/// Whether `raster_op(op, s, d)` is independent of `s` — `0`, `5` (`D' =
/// D`), `10` (`D' = ~D`), `15`, the four arms that do not mention `s`
/// (pinned by the same host test). Lets a copy skip the source fetch.
#[inline]
fn op_ignores_src(op: u8) -> bool {
    matches!(op & 0x0f, 0 | 5 | 10 | 15)
}

/// `d[i] = raster_op(OP, s[i], d[i])` over a run, with the op fixed at
/// compile time so the loop body is the bare word expression — the per-op
/// specialisation fVDI's `16b_blit.c` writes out by hand (`blit_copy`,
/// `blit_or`, `blit_16b`'s `DO_OP`) and EmuTOS' `vro_cpyfm16` spells as one
/// loop per `case` (`vdi_raster.c:1008-1220`). Same [`raster_op`], so the
/// same words.
#[inline(always)]
fn apply_row_op<const OP: u8>(s: &[u16], d: &mut [u16]) {
    for (dw, &sw) in d.iter_mut().zip(s) {
        *dw = raster_op(OP, sw, *dw);
    }
}

/// Apply logic op `op` over a run: `d[i] = raster_op(op, s[i], d[i])` for
/// `i < d.len()` (`s.len() >= d.len()`), dispatching once per run instead of
/// once per pixel.
fn apply_row(op: u8, s: &[u16], d: &mut [u16]) {
    match op & 0x0f {
        0 => apply_row_op::<0>(s, d),
        1 => apply_row_op::<1>(s, d),
        2 => apply_row_op::<2>(s, d),
        3 => apply_row_op::<3>(s, d),
        4 => apply_row_op::<4>(s, d),
        5 => apply_row_op::<5>(s, d),
        6 => apply_row_op::<6>(s, d),
        7 => apply_row_op::<7>(s, d),
        8 => apply_row_op::<8>(s, d),
        9 => apply_row_op::<9>(s, d),
        10 => apply_row_op::<10>(s, d),
        11 => apply_row_op::<11>(s, d),
        12 => apply_row_op::<12>(s, d),
        13 => apply_row_op::<13>(s, d),
        14 => apply_row_op::<14>(s, d),
        _ => apply_row_op::<15>(s, d),
    }
}

/// Words-per-row (16-pixel units) of a device-format memory MFDB of width
/// `w` — the `fd_wdwidth = (fd_w + 15) / 16` ABI formula.
#[inline]
fn mem_wdwidth(w: i16) -> i16 {
    ((w.max(0) as i32 + 15) / 16) as i16
}

/// The 16-entry BitBlt logic-op table, `D' = op(S, D)`, acting **literally
/// on the 16-bit RGB565 pixel word** — EmuTOS `vro_cpyfm16` operates the
/// exact same table on `UWORD` pixels (`vdi_raster.c:1008-1220`). The
/// memory side of every copy is now 16-bit RGB565 (see module docs), and
/// the screen is converted to/from RGB565 at the boundary, so a
/// memory-MFDB copy always computes the op in the 16-bit domain, matching
/// the Falcon truecolor VDI. Kept here so the off-screen paths need no
/// `kernel` dependency. Only the low nibble selects the op (`PAT_FLAG` is
/// already stripped by the caller).
#[inline]
fn raster_op(op: u8, s: u16, d: u16) -> u16 {
    match op & 0x0f {
        0 => 0,          // BM_ALL_WHITE   D' = 0
        1 => s & d,      // BM_S_AND_D
        2 => s & !d,     // BM_S_AND_NOTD
        3 => s,          // BM_S_ONLY      (replace)
        4 => !s & d,     // BM_NOTS_AND_D
        5 => d,          // BM_D_ONLY      (no-op)
        6 => s ^ d,      // BM_S_XOR_D     (XOR)
        7 => s | d,      // BM_S_OR_D      (OR)
        8 => !(s | d),   // BM_NOT_SORD
        9 => !(s ^ d),   // BM_NOT_SXORD
        10 => !d,        // BM_NOT_D
        11 => s | !d,    // BM_S_OR_NOTD
        12 => !s,        // BM_NOT_S
        13 => !s | d,    // BM_NOTS_OR_D
        14 => !(s & d),  // BM_NOT_SANDD
        _ => !0,         // BM_ALL_BLACK   D' = ~0
    }
}

/// Effective plane count of an MFDB for the `s_nxwd == d_nxwd` /
/// `plane_ct <= 16` gates EmuTOS applies in `setup_info` / `cpy_raster`
/// (`vdi_raster.c:922,937,963,1360`). The screen resolves to `v_planes`
/// (the backend's [`planes`](RasterBackend::planes), 16 under Falcon
/// truecolor); a memory MFDB carries its own `fd_nplanes`.
#[inline]
fn effective_planes(mfdb: &Mfdb, backend: &dyn RasterBackend) -> i16 {
    if mfdb.is_screen() {
        backend.planes() as i16
    } else {
        mfdb.nplanes
    }
}

/// Normalise a rectangle's two corners read from `PTSIN[base..base+3]`.
fn rect_from_ptsin(pb: &Vdipb, base: usize) -> Rect {
    let mut x1 = pb.ptsin(base);
    let mut y1 = pb.ptsin(base + 1);
    let mut x2 = pb.ptsin(base + 2);
    let mut y2 = pb.ptsin(base + 3);
    if x1 > x2 {
        core::mem::swap(&mut x1, &mut x2);
    }
    if y1 > y2 {
        core::mem::swap(&mut y1, &mut y2);
    }
    Rect { x: x1, y: y1, w: x2 - x1 + 1, h: y2 - y1 + 1 }
}

/// Opcode 121 — `vrt_cpyfm`: copy a 1-plane monochrome source into a colour
/// destination, expanding to `INTIN[1]` (fg) / `INTIN[2]` (bg). The
/// destination may be the **screen** (the common case: icons/sprites onto
/// the display) or a **memory MFDB** — 16-bit chunky for 9..16 planes
/// (AUD-V5-005), the interleaved planar form for 1..8 planes (V28).
pub fn vrt_cpyfm(pb: &Vdipb, vwk: &Vwk, backend: &dyn RasterBackend, ncol: u16) {
    // Caller-ABI MFDB-pointer convention (see module docs); the src pointer
    // is at CONTRL[7], the dst at 7 + N_PTRINTS — both MFDBs read in the
    // caller's space (an MFDB the program cannot read ends the call).
    let (Some(mut src), Some(mut dst)) = (Mfdb::read(pb, pb.contrl_ptr(0)), Mfdb::read(pb, pb.contrl_ptr(1)))
    else {
        return;
    };
    // Source must be a 1-plane memory raster (`cpy_raster:1384`,
    // `s_nxwd != 2` ⇒ reject); a screen source is not a mono form.
    if src.is_screen() || src.nplanes != 1 {
        return;
    }
    // `setup_info`: the destination plane count must be ≤ 16
    // (`vdi_raster.c:963`), else there is nothing to do.
    let dp = effective_planes(&dst, backend);
    if dp > CHUNKY_PLANES {
        return;
    }

    // `INTIN[0]` is the `MD_*` write mode (EmuTOS `vdi_defs.h:77-80`,
    // `cpy_raster:1397-1430`): MD_REPLACE=1, MD_TRANS=2, MD_XOR=3,
    // MD_ERASE=4. Reject anything above the low 5 bits before the switch,
    // like `vro_cpyfm` and EmuTOS `cpy_raster:1341-1342`. Any other value
    // (0 or ≥5) is a silent no-op — EmuTOS' `default: return`.
    let raw = pb.intin(0);
    if raw & !0x1F != 0 {
        return; // bits above PAT_FLAG set → invalid, no-op
    }
    // `PAT_FLAG` (bit 4) on the **truecolor** path kills the call, exactly
    // as in `vro_cpyfm` (D-VDI-23): `cpy_raster` strips it into its own
    // local `mode` (`vdi_raster.c:1347`) and switches on that to validate
    // the `MD_*` code (`:1397-1430`), but then calls `vrt_cpyfm16(info)`
    // without it (`:1432-1437`); `vrt_cpyfm16` re-reads the *unstripped*
    // `INTIN[0]` (`:1232`), `0x11..0x14` matches none of its four arms and
    // `default: return` (`:1323-1324`) leaves the destination untouched.
    // Only the palette path (`plane_ct <= 8`) sees the stripped mode
    // (D-082's strip-only contract). Measured (R05, 2026-09-21) on EmuTOS
    // 1.4 / Hatari Falcon030 VIDEL truecolor: `tosdump` case 215
    // (`vdi-raster-screen.tsv`, `INTIN = 0x11,1,0`, 32×16 mono → screen)
    // reads back 512 × `0xFFFF` (nothing drawn), while case 201
    // (`INTIN[0] = 1`, same geometry) expands the form.
    if dp > PALETTE_PLANES_MAX && raw & 0x10 != 0 {
        return; // PAT_FLAG on the truecolor path → nothing is drawn
    }
    let mode = match raw & !0x10 {
        1 => WriteMode::Replace,        // fg on 1-bits, bg on 0-bits
        2 => WriteMode::Transparent,    // fg on 1-bits, leave 0-bits
        3 => WriteMode::Xor,            // complement on 1-bits, leave 0-bits
        4 => WriteMode::RevTransparent, // leave 1-bits, bg on 0-bits (MD_ERASE)
        _ => return,                    // invalid → no-op
    };
    let fg = validate_color_index(pb.intin(1), ncol);
    let bg = validate_color_index(pb.intin(2), ncol);

    let s = rect_from_ptsin(pb, 0); // source rect
    let d = rect_from_ptsin(pb, 4); // dest rect (same extent)

    // The bitmaps the walk touches, checked with the caller's rights and
    // reached through their kernel addresses (ARCH03 3a, `Mfdb::bind`): the
    // whole mono source (the `MonoBits` view spans it) and, of a memory
    // destination, the pixels the block can reach inside the form (V26). One
    // the program may not use ends the call before anything is drawn.
    let words = (src.h.max(0) as usize) * (src.wdwidth.max(0) as usize);
    if !src.bind(pb, 0, 2 * words as u64, false) {
        return;
    }

    // A memory destination below nine planes (V28): `cpy_raster` takes
    // `vrt_cpyfm16` only for `plane_ct > 8` (`vdi_raster.c:1432-1437`); below
    // that the planar BitBlt (`bit_blt`/`fast_bit_blt`, `:1446-1458`) expands
    // the one source plane (`s_nxpl = 0`, `:1387`) into every plane of the
    // interleaved device form `setup_info` describes (`:919-940`), each plane
    // with the op its colour bits pick ([`vrt_planar_op`]). `PAT_FLAG` is
    // stripped above (D-082). The screen is 16 planes, so only a memory form
    // gets here.
    if !dst.is_screen() && dp <= PALETTE_PLANES_MAX {
        if dp < 1 {
            // No planes: nothing to draw (EmuTOS' plane loop would count down
            // past zero, `vdi_blit.S:1102` — as for `vro_cpyfm`, V19).
            return;
        }
        if !dst.bind_planar_block(pb, d.x as i32, d.y as i32, s.w as i32, s.h as i32, true) {
            return;
        }
        // The plane bits are the hardware colour register, so here — unlike
        // the 16-bit paths, which resolve a pen through the pen-ordered
        // pseudo-palette (V5-008 fold, below) — the pen goes through EmuTOS'
        // own `MAP_COL` (`:1390-1395`).
        let (fgc, bgc) = (map_col(fg, ncol), map_col(bg, ncol));
        planar_blit(&src, &dst, s, d.x, d.y, &|p| vrt_planar_op(mode, fgc, bgc, p), true);
        return;
    }
    if !dst.bind_block(pb, d.x as i32, d.y as i32, s.w as i32, s.h as i32, true) {
        return;
    }
    // SAFETY: the source MFDB describes `h * wdwidth` words at `addr` (now
    // its checked kernel address). The MFDB is the caller's, so `fd_addr` may
    // sit at an odd address; `MonoBits` reads the words without requiring
    // alignment.
    let bits = unsafe { MonoBits::from_raw(src.addr as *const u8, words) };
    let mono = MonoSrc { bits, stride_words: src.wdwidth as u16, x: s.x, y: s.y };

    if dst.is_screen() {
        // Screen destination: the backend expands the mono source into the
        // display, clipped to the workstation clip (the only clipped case,
        // `setup_info` `use_clip` when dst is screen).
        backend.blit_mono(Rect { x: d.x, y: d.y, w: s.w, h: s.h }, mono, fg, bg, mode, vwk.clip_rect());
    } else {
        // Memory destination (AUD-V5-005): expand into the 16-bit RGB565
        // buffer with the same `MD_*` semantics, resolving fg/bg through the
        // active pseudo-palette (`vrt_cpyfm16`, `fgcol = palette[fg_col]`,
        // `vdi_raster.c:1234-1236`). No clipping (dst is not the screen).
        //
        // AUD-V5-008 (`MAP_COL` fold): EmuTOS resolves `palette[MAP_COL[fg]]`
        // — `cpy_raster` maps `fg_col = MAP_COL[validate_color_index(INTIN[1])]`
        // (`vdi_raster.c:1390-1395`) before `vrt_cpyfm16` indexes the
        // pseudo-palette. NewTOS seeds the active pseudo-palette **in VDI-pen
        // order** (`vdi_backend.rs` `videl_palette1`/`videl_palette2` at index
        // = pen), i.e. the `MAP_COL` permutation is folded into the palette
        // seed, so `palette[fg]` here equals EmuTOS' `palette[MAP_COL[fg]]`
        // pixel-for-pixel — and identically to how every other primitive
        // (line/fill/text/`blit_mono`) resolves the same pen. The fold holds
        // for **all** pens (seed is a full 0..255 identity-ordered table), so
        // V5-008 stays a folded-acceptable, unobservable deviation; adding a
        // non-identity `MAP_COL` here would desync from the seeded palette and
        // break the desktop's colours.
        let fg16 = backend.pel_to_device16(fg);
        let bg16 = backend.pel_to_device16(bg);
        mono_to_mem(&dst, mono, d.x, d.y, s.w, s.h, mode, fg16, bg16);
    }
}

/// EmuTOS' `MAP_COL[pen]`, the VDI pen → hardware colour register map
/// (`vdi/vdi_col.c:28-29,736-742`): `MAP_COL_ROM` for pens 0..15 with pen 1
/// set to `numcolors - 1`, and — `EXTENDED_PALETTE`, on for the
/// VIDEL/TT-shifter build (`vdi_defs.h:20`) — pens 16..254 mapped to
/// themselves and pen 255 to register 15. `ncol` is `numcolors`
/// (`DEV_TAB[13]`, `vdi_control.c:206`; 256 at 16 planes). Only the planar
/// `vrt_cpyfm` path reads it: there the register's bits are the plane bits
/// written, observable in the destination form.
fn map_col(pen: super::backend::Pel, ncol: u16) -> u16 {
    const MAP_COL_ROM: [u16; 16] = [0, 15, 1, 2, 4, 6, 3, 5, 7, 8, 9, 10, 12, 14, 11, 13];
    match pen as usize {
        1 => ncol.wrapping_sub(1),
        p @ 0..=15 => MAP_COL_ROM[p],
        255 => 15,
        p => p as u16,
    }
}

/// The logic op `bit_blt` runs on plane `plane` of a planar `vrt_cpyfm`
/// (`vdi_raster.c:744-746`): `op_tab[(fg_bit << 1) | bg_bit]` with the
/// plane's bits of the two colour registers `fg`/`bg`. `cpy_raster` fills
/// `op_tab` and the colours per write mode (`:1397-1430`):
///
/// | mode | `op_tab` (fg:bg = 0:0, 0:1, 1:0, 1:1) | colours |
/// |---|---|---|
/// | `MD_REPLACE` | 0, 12, 3, 15 | fg, bg |
/// | `MD_TRANS` | 4, —, 7, — | fg, 0 |
/// | `MD_XOR` | 6, —, —, — | 0, 0 |
/// | `MD_ERASE` | 1, 13, —, — | 0, bg |
///
/// A slot EmuTOS leaves unset (—) is never indexed: the colour that would
/// select it is 0.
fn vrt_planar_op(mode: WriteMode, fg: u16, bg: u16, plane: usize) -> u8 {
    let bit = |c: u16| (c >> plane) & 1 != 0;
    match mode {
        WriteMode::Replace => match (bit(fg), bit(bg)) {
            (false, false) => 0,
            (false, true) => 12,
            (true, false) => 3,
            (true, true) => 15,
        },
        WriteMode::Transparent => {
            if bit(fg) {
                7
            } else {
                4
            }
        }
        WriteMode::Xor => 6,
        WriteMode::RevTransparent => {
            if bit(bg) {
                13
            } else {
                1
            }
        }
    }
}

/// `vrt_cpyfm` memory-destination expansion (EmuTOS `vrt_cpyfm16`,
/// `vdi_raster.c:1226-1326`): blit the 1-plane `mono` source into the
/// 16-bit chunky memory MFDB `dst` (9..16 planes) at origin `(dx, dy)` over `w × h`
/// pixels, per the `MD_*` write mode. `fg16`/`bg16` are the already-
/// resolved RGB565 pixel words. No clipping.
///
/// Both forms are addressed as EmuTOS does (V26): source pixel `(x, y)` is
/// bit `15 - x % 16` of WORD `y * fd_wdwidth + x / 16` (floor; `src_off =
/// s_xmin >> 4`, `p++` on each 16th pixel, `src += src_width` per row,
/// `vdi_raster.c:1241-1244,1255-1262`), destination pixel `(x, y)` is WORD
/// `y * fd_wdwidth * fd_nplanes + x` ([`Mfdb::get_mem`], V28). A pixel outside its form —
/// before WORD 0 or past `fd_h` rows — reads clear / is not written.
#[allow(clippy::too_many_arguments)]
fn mono_to_mem(dst: &Mfdb, mono: MonoSrc, dx: i16, dy: i16, w: i16, h: i16, mode: WriteMode, fg16: u16, bg16: u16) {
    let stride_px = mono.stride_words as i64 * 16;
    for r in 0..h as i32 {
        let sy = mono.y as i64 + r as i64;
        for c in 0..w as i32 {
            let lin = sy * stride_px + mono.x as i64 + c as i64;
            let set = lin >= 0
                && mono.bits.word((lin >> 4) as usize).is_some_and(|word| word & (0x8000u16 >> (lin & 15)) != 0);
            let (col, row) = (dx as i32 + c, dy as i32 + r);
            match mode {
                // MD_REPLACE: fg on 1-bits, bg on 0-bits (opaque).
                WriteMode::Replace => {
                    // SAFETY: `dst` is a valid device-format memory MFDB.
                    unsafe { dst.put_mem(col, row, if set { fg16 } else { bg16 }) };
                }
                // MD_TRANS: fg on 1-bits; 0-bits untouched.
                WriteMode::Transparent => {
                    if set {
                        // SAFETY: as above.
                        unsafe { dst.put_mem(col, row, fg16) };
                    }
                }
                // MD_XOR: complement the destination on 1-bits.
                WriteMode::Xor => {
                    if set {
                        // SAFETY: as above; `get_mem` bounds-checks.
                        unsafe { dst.put_mem(col, row, !dst.get_mem(col, row)) };
                    }
                }
                // MD_ERASE (reverse-transparent): bg on 0-bits; 1-bits alone.
                WriteMode::RevTransparent => {
                    if !set {
                        // SAFETY: as above.
                        unsafe { dst.put_mem(col, row, bg16) };
                    }
                }
            }
        }
    }
}

/// Opcode 109 — `vro_cpyfm`: opaque copy. Handles all four source/
/// destination combinations of screen and memory MFDB with the BitBlt
/// logic op in `INTIN[0]`: Screen↔Screen, Screen→Memory (`bb_save`),
/// Memory→Screen (`bb_restore`), Memory↔Memory. The destination extent
/// is the source extent; clipping applies **only** when the destination
/// is the screen (`_raster.md` edge case 8).
pub fn vro_cpyfm(pb: &Vdipb, vwk: &Vwk, backend: &dyn RasterBackend) {
    // Caller-ABI MFDB-pointer convention (see module docs); the src pointer
    // is at CONTRL[7], the dst at 7 + N_PTRINTS — both MFDBs read in the
    // caller's space (an MFDB the program cannot read ends the call).
    let (Some(mut src), Some(mut dst)) = (Mfdb::read(pb, pb.contrl_ptr(0)), Mfdb::read(pb, pb.contrl_ptr(1)))
    else {
        return;
    };
    // `INTIN[0]` carries the raster op in the low 4 bits and `PAT_FLAG`
    // (bit 4, 0x10); anything above bit 4 makes the mode invalid
    // (`cpy_raster:1341-1342`). The `PAT_FLAG` decision itself depends on
    // the plane count and is therefore taken below, after the plane gates.
    let raw_mode = pb.intin(0);
    if raw_mode & !0x1F != 0 {
        return; // bits above PAT_FLAG set → invalid, no-op
    }

    // Opaque-copy plane gate (AUD-V5-004), EmuTOS `setup_info` +
    // `cpy_raster:1360`: the destination plane count must be ≤ 16
    // (`vdi_raster.c:963`) and the source/destination plane counts must
    // match (`s_nxwd != d_nxwd` ⇒ return). The screen resolves to
    // `v_planes` (16), a memory MFDB to its own `fd_nplanes`; a
    // `device_mem` buffer is stamped [`CHUNKY_PLANES`] (16), so the
    // Screen↔Memory `bb_save`/`bb_restore` case (16 vs 16) still passes.
    let (sp, dp) = (effective_planes(&src, backend), effective_planes(&dst, backend));
    if dp > CHUNKY_PLANES || sp != dp {
        return;
    }

    // `PAT_FLAG` (bit 4) on the **truecolor** path is not merely stripped —
    // it kills the copy. `cpy_raster` strips it into its own local `mode`
    // (`vdi_raster.c:1347`) but then hands the truecolor branch nothing:
    // `vro_cpyfm16` re-reads the *unstripped* `INTIN[0]`
    // (`vdi_raster.c:981`), so a mode with bit 4 set (0x10..0x1F) matches
    // no arm of its 16-entry switch — which has no `default:`
    // (`vdi_raster.c:1008-1220`) — and the destination is left untouched.
    // Only the palette path (`plane_ct <= 8`, `bit_blt`) sees the stripped
    // mode, which is D-082's strip-only contract. Measured on a real Atari
    // TT running EmuTOS 1.4 with `CONF_WITH_VDI_16BIT`: `tosdump` case 144
    // (`INTIN[0] = 0x13`, 16-plane forms) leaves the destination buffer
    // bit-identical to its fill, while case 127 (`INTIN[0] = 3`, same
    // geometry) copies 64 words.
    if dp > PALETTE_PLANES_MAX && raw_mode & 0x10 != 0 {
        return; // PAT_FLAG on the truecolor path → nothing is copied
    }
    let op = (raw_mode & 0x0F) as u8;

    let s = rect_from_ptsin(pb, 0); // source rect (extent)
    let d = rect_from_ptsin(pb, 4); // destination upper-left
    let rows = s.h.max(0) as i32;

    // Memory→Memory below nine planes (V19): `cpy_raster` takes the 16-bit
    // branch only for `plane_ct > 8` (`vdi_raster.c:1368`); every other
    // plane count goes to the planar BitBlt (`fast_bit_blt`, `:1457`) on
    // the interleaved device form `setup_info` describes (`:919-940`).
    // `PAT_FLAG` is already stripped above (`op` is the low nibble, D-082).
    // The screen is 16 planes, so with `sp == dp` both forms are memory.
    if dp <= PALETTE_PLANES_MAX && !src.is_screen() && !dst.is_screen() {
        if dp < 1 {
            // No planes: nothing to copy (EmuTOS' plane loop would count down
            // past zero, `vdi_blit.S:1102` — see `_raster.md`, V19).
            return;
        }
        if !src.bind_planar_block(pb, s.x as i32, s.y as i32, s.w as i32, rows, false)
            || !dst.bind_planar_block(pb, d.x as i32, d.y as i32, s.w as i32, rows, true)
        {
            return;
        }
        planar_mem_to_mem(&src, &dst, s, d.x, d.y, op);
        return;
    }

    // The memory bitmaps the copy touches — the source rows read, the
    // destination rows written — checked with the caller's rights and reached
    // through their kernel addresses (ARCH03 3a, `Mfdb::bind_block`: the
    // pixels the block can reach inside each form, V26); one the program may
    // not use ends the call before anything is copied.
    if !src.bind_block(pb, s.x as i32, s.y as i32, s.w as i32, rows, false)
        || !dst.bind_block(pb, d.x as i32, d.y as i32, s.w as i32, rows, true)
    {
        return;
    }
    match (src.is_screen(), dst.is_screen()) {
        // Screen→Screen: the backend block-copy (overlap-safe, cache-clean).
        (true, true) => backend.copy_rect(s, d.x, d.y, op, vwk.clip_rect()),
        // Screen→Memory (bb_save): read the screen, write the buffer.
        (true, false) => screen_to_mem(backend, &dst, s, d.x, d.y, op),
        // Memory→Screen (bb_restore): read the buffer, write the screen.
        (false, true) => mem_to_screen(backend, &src, s, d.x, d.y, op, vwk.clip_rect()),
        // Memory→Memory: pure buffer copy (overlap-safe when co-located).
        (false, false) => mem_to_mem(&src, &dst, s, d.x, d.y, op),
    }
}

/// Screen→Memory copy (`vro_cpyfm` `bb_save` direction). The destination
/// is a device-format memory MFDB; **no clipping** (dest is not the
/// screen). Source rect `(s.x, s.y, s.w, s.h)` on screen → memory origin
/// `(dx, dy)`, applying `op` against the buffer's current pixel.
///
/// Row-wise (GUI07): per destination row, only the columns the buffer
/// accepts are visited ([`Mfdb::row_cols`]); the screen run under them is
/// fetched with one [`RasterBackend::read_row_rgb565`] call per
/// [`ROW_CHUNK`] pixels and combined into the buffer with
/// [`Mfdb::combine_row`]. Same words as the per-pixel definition — a screen
/// position off the screen reads `device_to_rgb565(0)`, a buffer pixel
/// outside the extent is left alone — the op runs on the 16-bit words
/// exactly as in EmuTOS `vro_cpyfm16`. `BM_D_ONLY` changes nothing in the
/// buffer and returns at once (`vdi_raster.c:1083-1085`).
fn screen_to_mem(backend: &dyn RasterBackend, dst: &Mfdb, s: Rect, dx: i16, dy: i16, op: u8) {
    if op & 0x0f == 5 {
        return;
    }
    let need_s = !op_ignores_src(op);
    let off_screen = backend.device_to_rgb565(0);
    let mut sbuf = [0u16; ROW_CHUNK];
    for r in 0..s.h as i32 {
        let (_, cmin, cmax) = dst.row_cols(dy as i32 + r);
        // The columns `c` of the copy whose destination pixel exists.
        let (lo, hi) = span(dx as i32, s.w.max(0) as usize, cmin, cmax);
        let sy = s.y as i32 + r;
        let mut c = lo;
        while c < hi {
            let n = (hi - c).min(ROW_CHUNK);
            let run = &mut sbuf[..n];
            let sx = s.x as i32 + c as i32;
            if !need_s {
                run.fill(0); // the op ignores S: any value gives the same word
            } else if sx > i16::MAX as i32 || sy > i16::MAX as i32 {
                // Past the `i16` coordinate range is past the screen (the old
                // per-pixel cast wrapped such a coordinate to a negative one,
                // which is off the screen too).
                run.fill(off_screen);
            } else {
                backend.read_row_rgb565(sx as i16, sy as i16, run);
            }
            // SAFETY: `dst` is a valid device-format memory MFDB; `run` is a
            // stack buffer of at most `ROW_CHUNK` words.
            unsafe { dst.combine_row(dx as i32 + c as i32, dy as i32 + r, run, op) };
            c += n;
        }
    }
}

/// Memory→Screen copy (`vro_cpyfm` `bb_restore` direction). The
/// destination is the screen, so the write is clipped to `clip` ∩ the
/// screen (`_raster.md` edge case 8). Source memory rect at `(s.x, s.y)`
/// → screen origin `(dx, dy)`, applying `op` against the live screen
/// pixel, then flushing the touched region to the point of coherency.
///
/// Row-wise (GUI07): per clipped screen row and [`ROW_CHUNK`] run, the
/// source words are fetched with [`Mfdb::read_row`], the live screen words
/// with one [`RasterBackend::read_row_rgb565`] call — **only if the op
/// depends on the destination** ([`op_ignores_dst`]: `S_ONLY`, `ALL_WHITE`,
/// `NOT_S`, `ALL_BLACK` do not, so they never read the screen) — the op is
/// applied over the run ([`apply_row`]) and the result is stored with one
/// [`RasterBackend::write_row_rgb565`] call. The op still runs on the
/// 16-bit words (screen word = `device_to_rgb565` of the stored pixel), so
/// every pixel is the word the per-pixel loop produced. `BM_D_ONLY` keeps
/// its read-and-rewrite (the rewrite stores `rgb565_to_device` of the read
/// word, as before).
fn mem_to_screen(
    backend: &dyn RasterBackend,
    src: &Mfdb,
    s: Rect,
    dx: i16,
    dy: i16,
    op: u8,
    clip: ClipRect,
) {
    let (sw, sh) = (backend.width() as i32, backend.height() as i32);
    // Destination box (dx, dy, s.w, s.h) ∩ clip ∩ screen (inclusive).
    let x0 = (dx as i32).max(clip.xmn as i32).max(0);
    let y0 = (dy as i32).max(clip.ymn as i32).max(0);
    let x1 = (dx as i32 + s.w as i32 - 1).min(clip.xmx as i32).min(sw - 1);
    let y1 = (dy as i32 + s.h as i32 - 1).min(clip.ymx as i32).min(sh - 1);
    if x0 > x1 || y0 > y1 {
        return;
    }
    let need_s = !op_ignores_src(op);
    let need_d = !op_ignores_dst(op);
    let mut sbuf = [0u16; ROW_CHUNK];
    let mut dbuf = [0u16; ROW_CHUNK];
    for ty in y0..=y1 {
        let sr = s.y as i32 + (ty - dy as i32);
        let mut tx = x0;
        while tx <= x1 {
            let n = ((x1 - tx + 1) as usize).min(ROW_CHUNK);
            let sc = s.x as i32 + (tx - dx as i32);
            let sv = &mut sbuf[..n];
            if need_s {
                // SAFETY: `src` is a valid device-format memory MFDB; `sv` is
                // a stack buffer.
                unsafe { src.read_row(sc, sr, sv) }; // 16-bit RGB565
            } else {
                sv.fill(0); // the op ignores S: any value gives the same word
            }
            let dv = &mut dbuf[..n];
            if need_d {
                // `tx`, `ty` lie on the screen, so they fit an `i16`.
                backend.read_row_rgb565(tx as i16, ty as i16, dv);
            } else {
                dv.fill(0); // the op ignores D: the screen is not read
            }
            apply_row(op, sv, dv);
            backend.write_row_rgb565(tx as i16, ty as i16, dv);
            tx += n as i32;
        }
    }
    backend.flush_rect(x0 as i16, y0 as i16, x1 as i16, y1 as i16);
}

/// Whether EmuTOS' `vro_cpyfm16` walks the `s` block from `src` to `(dx,
/// dy)` of `dst` backwards: its source start pointer `s_form + s_ymin *
/// src_width + s_xmin` lies below the destination's (`vdi_raster.c:986-995`,
/// widths in WORDs, [`Mfdb::mem_stride_px`]). Compared as addresses, as
/// EmuTOS does; for two forms that do not overlap the direction changes
/// nothing.
fn chunky_reverse(src: &Mfdb, dst: &Mfdb, s: Rect, dx: i16, dy: i16) -> bool {
    let start = |m: &Mfdb, x: i16, y: i16| {
        m.addr as i128 + 2 * (y as i128 * m.mem_stride_px() as i128 + x as i128)
    };
    start(src, s.x, s.y) < start(dst, dx, dy)
}

/// Memory→Memory copy (EmuTOS `vro_cpyfm16`, `vdi_raster.c:975-1221`); no
/// clipping.
///
/// **Direction as EmuTOS (V28).** `vro_cpyfm16` compares the two start
/// pointers — the block's upper-left pixel in each form,
/// `form + y * width + x` — and, iff the source's lies below the
/// destination's, walks the block backwards: rows from the last to the
/// first, each from its last pixel to its first (`:989-1002`). Otherwise it
/// walks forwards. Where a block overlaps itself the walk decides which
/// value lands — measurable once a row is shorter than the block (a form of
/// 9..15 planes, `vdi-v18-t2-raster.tsv` cases 44–45). The start is the
/// destination's upper-left, NewTOS' anchor (D-VDI-25).
///
/// Row-wise (GUI07), with the per-pixel order kept wherever it can matter.
/// Rows run in that order, and only the columns the destination accepts
/// are visited. Within a row, when the bytes the row reads from the source
/// and the bytes it writes to the destination are disjoint, the order of
/// the pixels cannot change a result, and the row is copied in
/// [`ROW_CHUNK`] runs ([`Mfdb::read_row`] + [`Mfdb::combine_row`]). When
/// they intersect — a shared buffer scrolled sideways, or two MFDBs whose
/// buffers overlap at different addresses — the row falls back to the
/// per-pixel read-op-write in the walk's column order, so even a smearing
/// overlap reproduces EmuTOS' words. `BM_D_ONLY` writes back every word it
/// reads and returns at once (`vdi_raster.c:1083-1085`).
fn mem_to_mem(src: &Mfdb, dst: &Mfdb, s: Rect, dx: i16, dy: i16, op: u8) {
    if op & 0x0f == 5 {
        return;
    }
    let need_s = !op_ignores_src(op);
    let need_d = !op_ignores_dst(op);
    let reverse = chunky_reverse(src, dst, s, dx, dy);
    let (down, right) = (reverse, reverse);
    let mut sbuf = [0u16; ROW_CHUNK];
    for ri in 0..s.h as i32 {
        let r = if down { s.h as i32 - 1 - ri } else { ri };
        let (drow, srow) = (dy as i32 + r, s.y as i32 + r);
        let (dstart, dmin, dmax) = dst.row_cols(drow);
        // The columns `c` of the copy whose destination pixel exists.
        let (lo, hi) = span(dx as i32, s.w.max(0) as usize, dmin, dmax);
        if lo >= hi {
            continue;
        }
        // Byte ranges this row touches: the destination words it writes and
        // the source words it reads (none if the op ignores S or the source
        // columns lie outside the source form). Both word indices are >= 0:
        // the columns lie in `cmin..cmax`.
        let dbytes = {
            let a = dst.addr + 2 * (dstart + dx as i64 + lo as i64) as u64;
            (a, a + 2 * (hi - lo) as u64)
        };
        let sbytes = if need_s {
            let (sstart, smin, smax) = src.row_cols(srow);
            let (a, b) = span(s.x as i32 + lo as i32, hi - lo, smin, smax);
            (a < b).then(|| {
                let first = src.addr + 2 * (sstart + s.x as i64 + lo as i64 + a as i64) as u64;
                (first, first + 2 * (b - a) as u64)
            })
        } else {
            None
        };
        let disjoint = sbytes.is_none_or(|(a, b)| b <= dbytes.0 || dbytes.1 <= a);
        if disjoint {
            let mut c = lo;
            while c < hi {
                let n = (hi - c).min(ROW_CHUNK);
                let run = &mut sbuf[..n];
                if need_s {
                    // SAFETY: valid device-format memory MFDB; stack buffer.
                    unsafe { src.read_row(s.x as i32 + c as i32, srow, run) };
                } else {
                    run.fill(0); // the op ignores S
                }
                // SAFETY: valid device-format memory MFDB; `run` is a stack
                // buffer of at most `ROW_CHUNK` words.
                unsafe { dst.combine_row(dx as i32 + c as i32, drow, run, op) };
                c += n;
            }
        } else {
            for ci in 0..(hi - lo) {
                let c = (if right { hi - 1 - ci } else { lo + ci }) as i32;
                // SAFETY: both MFDBs are valid device-format memory buffers;
                // `get_mem`/`put_mem` bounds-check every index.
                unsafe {
                    let sp = src.get_mem(s.x as i32 + c, srow);
                    let dp = if need_d { dst.get_mem(dx as i32 + c, drow) } else { 0 };
                    dst.put_mem(dx as i32 + c, drow, raster_op(op, sp, dp));
                }
            }
        }
    }
}

/// An interleaved (planar) device-form memory MFDB below nine planes, as
/// EmuTOS' `setup_info` describes it (`vdi_raster.c:919-940,959-960`): row
/// `y` starts at WORD `y * fd_wdwidth * P`, the 16-pixel group `g` of a row
/// is the `P` WORDs from `g * P` on (`s_nxwd = fd_nplanes * 2`), plane `p` is
/// WORD `p` of the group (`s_nxpl = 2`), pixel `x` is bit `15 - x % 16`.
///
/// Addressed linearly as in EmuTOS (V26): group `g` of row `y` is linear
/// group `y * fd_wdwidth + g` for any `g` — a group past the row is one of a
/// following row, one before it of a preceding row; `fd_w` plays no part.
/// Only the groups in `win` — the block's hull clipped to the form's
/// `fd_h * fd_wdwidth` groups ([`Mfdb::planar_hull`], exactly what
/// [`Mfdb::bind_planar_block`] bound) — are read or written; a group outside
/// reads as 0 and is not written.
#[derive(Clone, Copy)]
struct PlanarForm {
    addr: u64,
    /// Groups per row (`fd_wdwidth`).
    wd: i64,
    planes: usize,
    /// The linear groups `[win.0, win.1)` that may be touched.
    win: (i64, i64),
}

impl PlanarForm {
    fn of(m: &Mfdb, win: Option<(i64, i64)>) -> Self {
        PlanarForm {
            addr: m.addr,
            wd: m.wdwidth.max(0) as i64,
            planes: m.nplanes.max(0) as usize,
            win: win.unwrap_or((0, 0)),
        }
    }

    /// The address of plane `p`'s WORD of linear group `lg`; `lg` must lie in
    /// `win` and `p < planes`.
    #[inline]
    fn word_addr(&self, lg: i64, p: usize) -> u64 {
        self.addr + 2 * (lg as u64 * self.planes as u64 + p as u64)
    }

    #[inline]
    fn in_win(&self, lg: i64) -> bool {
        self.win.0 <= lg && lg < self.win.1
    }

    /// Plane `p`'s WORD of linear group `lg`; 0 for a group outside `win`.
    ///
    /// # Safety
    /// `addr` must reach the groups of `win` (bound).
    #[inline]
    unsafe fn fetch(&self, lg: i64, p: usize) -> u16 {
        if !self.in_win(lg) {
            return 0;
        }
        // SAFETY: `lg` lies in `win`, which the call bound.
        unsafe { (self.word_addr(lg, p) as *const u16).read_unaligned() }
    }

    /// The 16 source bits of plane `p` starting at the pixel with linear
    /// index `lin` (`y * fd_wdwidth * 16 + x`; bit 15 = that pixel),
    /// assembled from the two groups the run spans — the blitter's skew
    /// (`vdi_raster.c:654-656`). A bit from a group outside `win` is 0; such a
    /// bit never belongs to a pixel of the block (the hull covers them all),
    /// so the end mask drops it.
    ///
    /// # Safety
    /// As [`Self::fetch`].
    #[inline]
    unsafe fn bits_at(&self, lin: i64, p: usize) -> u16 {
        let (g, sh) = (lin.div_euclid(16), lin.rem_euclid(16) as u32);
        // SAFETY: forwarded to `fetch`, which bounds every group.
        unsafe {
            let hi = self.fetch(g, p);
            if sh == 0 {
                return hi;
            }
            let lo = self.fetch(g + 1, p);
            ((((hi as u32) << 16) | lo as u32) >> (16 - sh)) as u16
        }
    }
}

/// Memory→Memory `vro_cpyfm` between two interleaved device forms of the
/// same plane count 1..8 (V19) — EmuTOS' planar BitBlt (`fast_bit_blt`,
/// `vdi/vdi_blit.S:794`; the C `bit_blt`, `vdi_raster.c:588-773`), reached
/// from `cpy_raster` for `plane_ct <= 8` (`:1368,1457`).
///
/// The `s.w × s.h` block at `(s.x, s.y)` lands at `(dx, dy)`. Per plane and
/// destination WORD, the source run under it is shifted into place
/// ([`PlanarForm::bits_at`]), the logic op of the 16-entry table runs on the
/// WORD ([`raster_op`]; the same op for every plane, `op_tab[0]`,
/// `:1363-1365`), and only the bits of the block's pixels are stored — the
/// end masks keep the neighbours (`mask_out`, `:548-552`). `BM_D_ONLY`
/// stores what it read and returns at once.
///
/// Direction, for an overlap (`bit_blt :660-669`, `vdi_blit.S:942-946`):
/// bottom-up and right-to-left iff the source's start WORD lies below the
/// destination's in memory, or it is the same WORD and the source's
/// `x % 16` is the smaller; otherwise top-down, left-to-right. Each WORD is
/// read before it is written, so an overlapping copy within one form reads
/// only source bits not yet overwritten.
///
/// Geometry as EmuTOS (V26, [`PlanarForm`]): `fd_w` is not a limit, a block
/// wider than the row runs on into the next row, the padding bits past
/// `fd_w` are copied like any other; only groups outside the form's `fd_h`
/// rows are left alone. The block is anchored at `(dx, dy)` (D-VDI-25).
fn planar_mem_to_mem(src: &Mfdb, dst: &Mfdb, s: Rect, dx: i16, dy: i16, op: u8) {
    planar_blit(src, dst, s, dx, dy, &|_| op, false);
}

/// EmuTOS' planar BitBlt between two interleaved memory forms
/// (`bit_blt`, `vdi_raster.c:588-773`; `fast_bit_blt`, `vdi/vdi_blit.S:794`):
/// the walk of [`planar_mem_to_mem`], with the op of each destination plane
/// `p` given by `plane_op(p)` (`op_tab[...]`, `:744-746`) and, for
/// `mono_src`, the source's one plane under every destination plane
/// (`s_nxpl = 0`, the `vrt_cpyfm` set-up, `:1387`) instead of plane `p`. A
/// plane whose op is `BM_D_ONLY` stores what it read and is skipped.
fn planar_blit(src: &Mfdb, dst: &Mfdb, s: Rect, dx: i16, dy: i16, plane_op: &dyn Fn(usize) -> u8, mono_src: bool) {
    let (bw, bh) = (s.w as i32, s.h as i32);
    let Some(dwin) = dst.planar_hull(dx as i32, dy as i32, bw, bh) else {
        return;
    };
    let swin = src.planar_hull(s.x as i32, s.y as i32, bw, bh);
    let (sf, df) = (PlanarForm::of(src, swin), PlanarForm::of(dst, Some(dwin)));
    let (sx, sy, dx, dy) = (s.x as i64, s.y as i64, dx as i64, dy as i64);
    let (bw, bh) = (bw as i64, bh as i64);
    // EmuTOS' start addresses (`s_xy2addr`/`d_xy2addr`, `vdi_blit.S:1240-1264`)
    // decide the direction; only an overlap can tell the two apart.
    let start = |f: &PlanarForm, x: i64, y: i64| {
        f.addr as i128 + 2 * ((y * f.wd + x.div_euclid(16)) as i128 * f.planes as i128)
    };
    let (s_start, d_start) = (start(&sf, sx, sy), start(&df, dx, dy));
    let reverse = s_start < d_start || (s_start == d_start && (sx & 15) < (dx & 15));
    // The block's groups in a row and the pixels it covers.
    let (g0, g1) = (dx.div_euclid(16), (dx + bw - 1).div_euclid(16));
    let (xa, xb) = (dx, dx + bw - 1);
    for p in 0..df.planes {
        let op = plane_op(p) & 0x0f;
        if op == 5 {
            continue;
        }
        let need_s = !op_ignores_src(op);
        let sp = if mono_src { 0 } else { p };
        for yi in 0..bh {
            let y = if reverse { dy + bh - 1 - yi } else { dy + yi };
            let srow = y - dy + sy;
            // The groups of this block row that lie in the window.
            let base = y * df.wd;
            let (ga, gb) = ((dwin.0 - base).max(g0), (dwin.1 - 1 - base).min(g1));
            if ga > gb {
                continue;
            }
            for gi in 0..=(gb - ga) {
                let g = if reverse { gb - gi } else { ga + gi };
                let x0 = 16 * g;
                let (lo, hi) = (xa.max(x0) - x0, xb.min(x0 + 15) - x0);
                let mask = (0xFFFFu16 >> lo) & (0xFFFFu16 << (15 - hi));
                // SAFETY: `base + g` lies in `dwin` (bound for writing) and `p`
                // is below the destination's plane count; `sp` is below the
                // source's (plane `p` of a form with as many planes, or the one
                // plane of a mono source); `bits_at` reads only groups of `swin`
                // (bound for reading).
                unsafe {
                    let sw = if need_s { sf.bits_at(srow * sf.wd * 16 + x0 - dx + sx, sp) } else { 0 };
                    let a = df.word_addr(base + g, p) as *mut u16;
                    let d = a.read_unaligned();
                    a.write_unaligned((raster_op(op, sw, d) & mask) | (d & !mask));
                }
            }
        }
    }
}

/// Number of `u32` words a `bb_save` buffer for a `w × h` screen region
/// needs. The save bitmap is **16-bit RGB565 chunky** (one `u16` per pixel,
/// rows padded to 16 px), unified with the app-visible memory-MFDB
/// wireformat and EmuTOS' own truecolor `bb_save` buffer (`gemgsxif.c:334`,
/// `gl_nplanes = v_planes = 16`): `memsize(sw/16, sh, 16)` bytes, i.e.
/// `wdwidth * h * 16 * 2`. The `u32` element is a counting convenience while
/// the *pixels* are 16-bit (`docs/COMPATIBILITY.md`): two 16-bit pixels per
/// `u32`, so the count is `stride_px * h / 2 = fd_wdwidth * 8 * h`.
/// `stride_px` is a multiple of 16, so the division is exact. The AES sizes
/// its one screen buffer (`aes::gsxif`, `gsx_mcalc`) and `popup_blit`'s
/// transient block from exactly this.
pub fn bb_save_words(w: i16, h: i16) -> usize {
    (mem_wdwidth(w) as usize) * 8 * (h.max(0) as usize)
}

/// AES `bb_save` (EmuTOS `gemgsxif.c` `bb_set`, save direction): copy the
/// screen rectangle `(x, y, w, h)` into the caller-owned buffer `mem` as a
/// 16-bit RGB565 device-format bitmap (a Screen→Memory `vro_cpyfm`, `S_ONLY`
/// / replace). Since the screen is 5-bit-quantised, the RGB565 store is
/// lossless and the matching [`bb_restore`] reproduces the pixels bit-for-
/// bit. This is the blit; the buffer it goes into is the AES's business —
/// `aes::gsxif` hands the one persistent screen buffer (`gl_tmp`) to the
/// alert and the drop-down and a transient pool block to a pop-up / submenu,
/// and holds the length guard. Called directly by the AES rather than
/// through a re-entrant trap. `mem` is a `u32` pointer for the caller's
/// convenience; its bytes hold two 16-bit pixels per word (see
/// [`bb_save_words`]).
///
/// **The mouse cursor is hidden for the copy** (`bb_set`'s `gsx_moff()` …
/// `gsx_mon()` bracket, `gemgsxif.c:367,378`), so the save-under sprite is
/// never read into the image — otherwise the restore would paint it back as
/// background, a ghost arrow at the position the pointer had when the
/// alert opened. The bracket sits here, under `bb_save`/`bb_restore`, not in
/// the caller: `fm_alert` (`gemfmalt.c:363,390`) has none of its own. The
/// hide depth counts (EmuTOS `gl_moff`), so a caller's own bracket nests.
///
/// # Safety
/// `mem` must point at a buffer of at least [`bb_save_words`]`(w, h)`
/// `u32` words that stays alive until the matching [`bb_restore`].
pub unsafe fn bb_save(backend: &dyn RasterBackend, mem: *mut u32, x: i16, y: i16, w: i16, h: i16) {
    let dst = Mfdb::device_mem(mem as u64, w, h);
    super::mouse::hide(backend);
    screen_to_mem(backend, &dst, Rect { x, y, w, h }, 0, 0, OP_REPLACE);
    super::mouse::show(backend);
}

/// AES `bb_restore`: copy the 16-bit RGB565 device-format buffer `mem`
/// (filled by [`bb_save`]) back to the screen rectangle `(x, y, w, h)` — a
/// Memory→Screen `vro_cpyfm` replace, clipped to `clip`. The cursor is
/// hidden for the copy, as in [`bb_save`] (`bb_set`, `gemgsxif.c:367,378`):
/// the sprite's save-under is released before the screen changes under it
/// and re-taken from the restored pixels afterwards.
///
/// # Safety
/// As [`bb_save`]; `mem` must hold [`bb_save_words`]`(w, h)` `u32` words.
pub unsafe fn bb_restore(
    backend: &dyn RasterBackend,
    mem: *const u32,
    x: i16,
    y: i16,
    w: i16,
    h: i16,
    clip: ClipRect,
) {
    let src = Mfdb::device_mem(mem as u64, w, h);
    super::mouse::hide(backend);
    mem_to_screen(backend, &src, Rect { x: 0, y: 0, w, h }, x, y, OP_REPLACE, clip);
    super::mouse::show(backend);
}

/// AES `gr_colourblit` (`aes/gemgraf.c:642-671`): the Memory→Screen
/// `vro_cpyfm` of a colour icon's **device-form** image — a `dst.w × dst.h`
/// buffer of 16-bit RGB565 pixels at `mem` (the memory-MFDB wireformat,
/// rows `(dst.w + 15) / 16` words wide) — onto the screen at `(dst.x,
/// dst.y)` with logic op `op` (`S_AND_D` = 1 on the truecolor path,
/// `gemgraf.c:664-667`), clipped to `clip`. EmuTOS builds the source MFDB
/// with `gsx_fix(&gl_src, pdata, w/8, h)` (`fd_wdwidth = w/16`) and the
/// screen as destination; this is [`vro_cpyfm`]'s memory→screen arm for
/// exactly that pair. No cursor bracket here: the object renderer's
/// `ob_draw` already holds one.
///
/// # Safety
/// `mem` must address `((dst.w + 15) / 16) * 16 * dst.h` readable `u16`s
/// (no alignment required) that stay valid for the call.
pub unsafe fn blit_device_mem(backend: &dyn RasterBackend, mem: *const u16, dst: Rect, op: u8, clip: ClipRect) {
    let src = Mfdb::device_mem(mem as u64, dst.w, dst.h);
    mem_to_screen(backend, &src, Rect { x: 0, y: 0, w: dst.w, h: dst.h }, dst.x, dst.y, op, clip);
}

/// Truecolor `vr_trnfm` branch — a faithful port of EmuTOS `vr_trnfm16`
/// (`vdi_raster.c:190-256`). Converts a bitmap between **16-bit standard**
/// form (`fd_stand = 1`: 16 plane-sequential planes, machine-independent)
/// and **Truecolor device-dependent** form (`fd_stand = 0`: 16-bit
/// RGB565-chunky, one `UWORD` per pixel — NewTOS' memory-MFDB wireformat,
/// [`CHUNKY_PLANES`]). Direction is chosen by `src.stand`; both forms hold
/// the same `planesize * 16` words (`planesize = fd_h * fd_wdwidth`, in
/// words) laid out flat, so the transpose is pure index arithmetic — a
/// standard plane `p` contributes pixel-word bit `0x8000 >> p`, and pixel
/// word `k` maps to plane word `k / 16` bit `0x8000 >> (k & 15)`. With the
/// unified 16-bit memory format (TC3b) there is no format jump: the chunky
/// output is exactly what [`vro_cpyfm`] / [`bb_save`] read, resolving
/// AUD-V5-002/003.
///
/// The destination `fd_stand` is **not** written here, matching EmuTOS'
/// truecolor branch (which, unlike the ≤8-plane planar path below, leaves
/// `dst_mfdb->fd_stand` untouched — the caller stamps the target form).
/// Works in place via a zeroed scratch buffer (EmuTOS `dos_alloc_anyram`);
/// a tight heap fails silently, exactly as the reference does.
fn vr_trnfm16(src: &Mfdb, dst: &Mfdb) {
    // `planesize = fd_h * fd_wdwidth` words per plane; the whole form (either
    // direction) is `planesize * 16` words (`formsize = planesize * 2 * 16`
    // bytes, `vdi_raster.c:198-199`).
    let planesize = (src.h.max(0) as usize) * (src.wdwidth.max(0) as usize);
    if planesize == 0 {
        return;
    }
    let total = planesize * 16;

    let s = src.addr as *const u16;
    let inplace = src.addr == dst.addr;

    // Destination buffer: the caller's `dst` (cleared), or a zeroed scratch
    // when transforming in place (EmuTOS `dos_alloc_anyram` + `bzero`). A
    // failed allocation is a silent no-op, matching the reference.
    let scratch = if inplace {
        let Ok(layout) = Layout::array::<u16>(total) else {
            return;
        };
        // SAFETY: `total >= 16` (planesize >= 1), so the layout is non-zero.
        let p = unsafe { alloc_zeroed(layout) } as *mut u16;
        if p.is_null() {
            return;
        }
        Some((p, layout))
    } else {
        None
    };
    let d = match &scratch {
        Some((p, _)) => *p,
        None => {
            // EmuTOS `bzero(dst, formsize)` — clear the output before OR-ing.
            // SAFETY: the dst MFDB describes `total` words at `dst.addr`.
            unsafe { core::ptr::write_bytes(dst.addr as *mut u16, 0, total) };
            dst.addr as *mut u16
        }
    };

    // SAFETY: `s` holds `total` words (the source form) and `d` holds `total`
    // words (the caller's dst or the `total`-word scratch); every index below
    // stays within `[0, total)`, mirroring the reference loops 1:1. In the
    // in-place case `s` and `d` are disjoint (scratch), so there is no alias.
    unsafe {
        if src.stand != 0 {
            // standard (16 plane-sequential planes) -> device (chunky):
            // plane `p` sets bit `0x8000 >> p` of each pixel word.
            let mut src_idx = 0usize;
            for p in 0..16u32 {
                let dst_mask = 0x8000u16 >> p;
                let mut work = 0usize;
                for _ in 0..planesize {
                    let sword = *s.add(src_idx);
                    src_idx += 1;
                    let mut src_mask = 0x8000u16;
                    while src_mask != 0 {
                        if sword & src_mask != 0 {
                            *d.add(work) |= dst_mask;
                        }
                        work += 1;
                        src_mask >>= 1;
                    }
                }
            }
        } else {
            // device (chunky) -> standard (16 plane-sequential planes):
            // pixel-word bit `0x8000 >> p` becomes plane `p`.
            let mut dst_idx = 0usize;
            for p in 0..16u32 {
                let src_mask = 0x8000u16 >> p;
                let mut work = 0usize;
                for _ in 0..planesize {
                    let mut dst_mask = 0x8000u16;
                    while dst_mask != 0 {
                        if *s.add(work) & src_mask != 0 {
                            *d.add(dst_idx) |= dst_mask;
                        }
                        work += 1;
                        dst_mask >>= 1;
                    }
                    dst_idx += 1;
                }
            }
        }
    }

    // In place: copy the finished scratch back over the shared buffer, free.
    if let Some((p, layout)) = scratch {
        // SAFETY: `p` holds `total` words (`alloc_zeroed(layout)`); `dst.addr`
        // holds `total` words (MFDB geometry). Distinct allocations.
        unsafe {
            core::ptr::copy_nonoverlapping(p as *const u16, dst.addr as *mut u16, total);
            dealloc(p as *mut u8, layout);
        }
    }
}

/// Opcode 110 — `vr_trnfm`: transform a bitmap between **standard**
/// (plane-sequential, machine-independent) and **device-dependent**
/// (word-interleaved for 1-8 planes, RGB565-chunky for Truecolor) form.
/// The destination's `fd_stand` is forced to the opposite of the source's
/// (planar path only); works in place when the addresses coincide. Faithful
/// port of EmuTOS `vdi_vr_trnfm` (`vdi_raster.c:269-356`), including the
/// Truecolor gate (`:281-294`).
pub fn vr_trnfm(pb: &Vdipb, backend: &dyn RasterBackend) {
    let abi = pb.abi();
    // Caller-ABI MFDB-pointer convention (see module docs); both MFDBs read
    // in the caller's space (ARCH03 3a).
    let Some(mut src) = Mfdb::read(pb, pb.contrl_ptr(0)) else {
        return;
    };
    let dst_ptr = pb.contrl_ptr(1);
    if src.addr == 0 || dst_ptr == 0 {
        return;
    }
    // Dst MFDB has `abi`'s layout (addr@0, …, fd_stand at `stand_off`).
    let Some(mut dst) = Mfdb::read(pb, dst_ptr) else {
        return;
    };
    if dst.addr == 0 {
        return;
    }

    // TRUECOLOR_MODE branch (EmuTOS `vdi_vr_trnfm`, `vdi_raster.c:281-294`):
    // in a Truecolor mode (`TRUECOLOR_MODE = v_planes > 8`) a source with more
    // than 8 planes is transformed between 16-bit standard and Truecolor
    // device form, *not* by the planar interleave below. The backend reports
    // `v_planes` (16 under Falcon truecolor); a non-truecolor backend
    // (`planes() <= 8`) falls through to the planar path, exactly as EmuTOS'
    // gate does. (AUD-V5-002/003.)
    if backend.planes() > 8 && src.nplanes > 8 {
        // Both forms hold `planesize * 16` words of the *source* geometry
        // (`vr_trnfm16`), checked with the caller's rights and reached
        // through their kernel addresses (ARCH03 3a).
        let total = 2 * 16 * (src.h.max(0) as u64) * (src.wdwidth.max(0) as u64);
        if !src.bind(pb, 0, total, false) || !dst.bind(pb, 0, total, true) {
            return;
        }
        vr_trnfm16(&src, &dst);
        return;
    }

    // EmuTOS: `size = (LONG)fd_h * fd_wdwidth` words per plane and the loop
    // counts are `LONG`s (`vdi_raster.c:299-316`): a zero or negative plane
    // size or plane count copies nothing — after the destination's `fd_stand`
    // has been forced, which EmuTOS does first. Computed signed and wide, so
    // a negative `fd_h` is no overflow here either.
    let stand_off = match abi {
        Abi::Lp64 => 14,
        Abi::Ilp32 => 10,
    };
    let size_words = i64::from(src.h) * i64::from(src.wdwidth);
    if size_words <= 0 || src.nplanes <= 0 {
        let new_stand: u16 = if src.stand != 0 { 0 } else { 1 };
        if pb.space().put_u16(dst_ptr + stand_off, new_stand).is_err() {
            pb.note_fault();
        }
        return;
    }
    let planes = src.nplanes as usize;
    let size = size_words as usize; // words per plane, > 0
    let (outer, inner) = if src.stand != 0 {
        (planes, size) // standard → device
    } else {
        (size, planes) // device → standard
    };

    // Both forms hold `size * planes` words, checked with the caller's rights
    // and reached through their kernel addresses (ARCH03 3a); the same buffer
    // when transforming in place.
    let inplace = src.addr == dst.addr;
    let form = 2u64.saturating_mul(size as u64).saturating_mul(planes as u64);
    if !src.bind(pb, 0, form, inplace) || !dst.bind(pb, 0, form, true) {
        return;
    }
    let s = src.addr as *const u16;
    let d = dst.addr as *mut u16;

    // Force the destination form opposite to the source. `fd_stand` sits
    // after `fd_addr` + three WORDs, so its byte offset shifts with the
    // ABI: 14 for LP64 (8B ptr), 10 for ILP32 (4B ptr). Written in the
    // caller's space (the dst MFDB is the caller's struct).
    let new_stand: i16 = if src.stand != 0 { 0 } else { 1 };
    if pb.space().put_u16(dst_ptr + stand_off, new_stand as u16).is_err() {
        pb.note_fault();
        return;
    }

    if !inplace {
        // Simple transpose: read the source linearly, scatter into the
        // destination with stride `outer`.
        // SAFETY: both buffers hold `size*planes` words (MFDB geometry).
        unsafe {
            let mut si = 0usize;
            for i in 0..outer {
                let mut wi = i;
                for _ in 0..inner {
                    *d.add(wi) = *s.add(si);
                    si += 1;
                    wi += outer;
                }
            }
        }
        return;
    }

    // In-place: mono has no standard/device difference.
    if planes == 1 {
        return;
    }
    if outer <= 1 {
        return;
    }
    // Faithful port of the EmuTOS in-place rotation (no scratch buffer).
    // SAFETY: the buffer holds `size*planes` words; all index motion stays
    // within it, mirroring the reference algorithm 1:1.
    unsafe {
        let outer = (outer - 1) as isize;
        let mut inner = inner as isize - 1;
        let mut src = d; // src == dst buffer
        let mut work = src;
        while inner >= 0 {
            let mut count: isize = 0;
            for _ in 0..outer {
                src = src.offset(inner + 1);
                let temp = *src;
                let mut dst = src;
                work = src;
                count += inner;
                for _ in 0..count {
                    work = dst;
                    dst = dst.offset(-1);
                    *work = *dst;
                }
                *dst = temp;
            }
            src = work;
            inner -= 1;
        }
    }
}

/// GUI07 — the row-wise copy paths against the per-pixel ones they replace.
///
/// The three `oracle_*` functions below are the per-pixel `screen_to_mem`,
/// `mem_to_screen` and `mem_to_mem` exactly as they stood before GUI07
/// (verbatim apart from the name). Every test runs the new and the old path
/// on identical copies of the same screen and buffers and demands identical
/// words everywhere — screen, both buffers including their padding, and the
/// flushed rectangle — for all sixteen ops (and the high-nibble aliases),
/// clipped and unclipped, odd widths, runs longer than [`ROW_CHUNK`], source
/// and destination partly outside, a `fd_wdwidth` too small for `fd_w`, odd
/// buffer addresses, and every kind of memory overlap.
#[cfg(test)]
mod gui07_tests {
    use super::*;
    use crate::vdi::backend::{PatternFill, Pel};
    use alloc::vec;
    use alloc::vec::Vec;
    use std::sync::Mutex;

    // ── the old per-pixel paths, the oracle ─────────────────────────────

    /// Screen→Memory copy (`vro_cpyfm` `bb_save` direction). The destination
    /// is a device-format memory MFDB; **no clipping** (dest is not the
    /// screen). Source rect `(s.x, s.y, s.w, s.h)` on screen → memory origin
    /// `(dx, dy)`, applying `op` against the buffer's current pixel.
    fn oracle_screen_to_mem(backend: &dyn RasterBackend, dst: &Mfdb, s: Rect, dx: i16, dy: i16, op: u8) {
        for r in 0..s.h as i32 {
            for c in 0..s.w as i32 {
                // Convert the raw 32-bpp screen pixel to the 16-bit RGB565
                // memory wireformat, then apply the logic op in the 16-bit
                // domain (EmuTOS `vro_cpyfm16`, all ops on `UWORD` pixels).
                let sp = backend
                    .device_to_rgb565(backend.get_device_pixel((s.x as i32 + c) as i16, (s.y as i32 + r) as i16));
                // SAFETY: `dst` is a valid device-format memory MFDB.
                let dp = unsafe { dst.get_mem(dx as i32 + c, dy as i32 + r) };
                // SAFETY: as above; `put_mem` bounds-checks the index.
                unsafe { dst.put_mem(dx as i32 + c, dy as i32 + r, raster_op(op, sp, dp)) };
            }
        }
    }

    /// Memory→Screen copy (`vro_cpyfm` `bb_restore` direction). The
    /// destination is the screen, so the write is clipped to `clip` ∩ the
    /// screen (`_raster.md` edge case 8). Source memory rect at `(s.x, s.y)`
    /// → screen origin `(dx, dy)`, applying `op` against the live screen
    /// pixel, then flushing the touched region to the point of coherency.
    fn oracle_mem_to_screen(
        backend: &dyn RasterBackend,
        src: &Mfdb,
        s: Rect,
        dx: i16,
        dy: i16,
        op: u8,
        clip: ClipRect,
    ) {
        let (sw, sh) = (backend.width() as i32, backend.height() as i32);
        // Destination box (dx, dy, s.w, s.h) ∩ clip ∩ screen (inclusive).
        let x0 = (dx as i32).max(clip.xmn as i32).max(0);
        let y0 = (dy as i32).max(clip.ymn as i32).max(0);
        let x1 = (dx as i32 + s.w as i32 - 1).min(clip.xmx as i32).min(sw - 1);
        let y1 = (dy as i32 + s.h as i32 - 1).min(clip.ymx as i32).min(sh - 1);
        if x0 > x1 || y0 > y1 {
            return;
        }
        for ty in y0..=y1 {
            for tx in x0..=x1 {
                let sc = s.x as i32 + (tx - dx as i32);
                let sr = s.y as i32 + (ty - dy as i32);
                // SAFETY: `src` is a valid device-format memory MFDB.
                let sp = unsafe { src.get_mem(sc, sr) }; // 16-bit RGB565
                // Bring the live screen pixel into the 16-bit domain, apply the
                // op there, then expand the result back to a device pixel — the
                // op runs on `UWORD` pixels exactly as EmuTOS `vro_cpyfm16`.
                let dp = backend.device_to_rgb565(backend.get_device_pixel(tx as i16, ty as i16));
                let result = raster_op(op, sp, dp);
                backend.put_device_pixel(tx as i16, ty as i16, backend.rgb565_to_device(result));
            }
        }
        backend.flush_rect(x0 as i16, y0 as i16, x1 as i16, y1 as i16);
    }

    /// Memory→Memory copy, pixel by pixel in EmuTOS `vro_cpyfm16`'s walk
    /// (`vdi_raster.c:986-1002`): backwards — last row first, each row from
    /// its last pixel — iff the source block's first pixel lies at a lower
    /// address than the destination's (V28); no clipping.
    fn oracle_mem_to_mem(src: &Mfdb, dst: &Mfdb, s: Rect, dx: i16, dy: i16, op: u8) {
        let first = |m: &Mfdb, x: i16, y: i16| {
            m.addr as i128 + 2 * (y as i128 * (m.wdwidth as i128 * m.nplanes as i128) + x as i128)
        };
        let back = first(src, s.x, s.y) < first(dst, dx, dy);
        let (down, right) = (back, back);
        for ri in 0..s.h as i32 {
            let r = if down { s.h as i32 - 1 - ri } else { ri };
            for ci in 0..s.w as i32 {
                let c = if right { s.w as i32 - 1 - ci } else { ci };
                // SAFETY: both MFDBs are valid device-format memory buffers.
                let sp = unsafe { src.get_mem(s.x as i32 + c, s.y as i32 + r) };
                let dp = unsafe { dst.get_mem(dx as i32 + c, dy as i32 + r) };
                // SAFETY: as above; `put_mem` bounds-checks the index.
                unsafe { dst.put_mem(dx as i32 + c, dy as i32 + r, raster_op(op, sp, dp)) };
            }
        }
    }


    // ── the screen double ───────────────────────────────────────────────

    /// The kernel's device-pixel model (`kernel/src/vdi_backend.rs`
    /// `word_to_device` / `device_to_word`): 5→8-bit gun replication plus the
    /// overlay bit in the X byte. The screen is seeded with **arbitrary** 32-bit
    /// words, most of which are not in the image of `word_to_device`, so the
    /// lossy screen→word conversion is exercised, not just its exact inverse.
    fn word_to_device(word: u16) -> u32 {
        let e = |c5: u16| ((c5 << 3) | (c5 >> 2)) as u32;
        let x = if word & 0x0020 != 0 { 0xFF00_0000 } else { 0 };
        x | (e((word >> 11) & 0x1f) << 16) | (e((word >> 6) & 0x1f) << 8) | e(word & 0x1f)
    }
    pub(super) fn device_to_word(px: u32) -> u16 {
        let r5 = ((px >> 19) & 0x1f) as u16;
        let g5 = ((px >> 11) & 0x1f) as u16;
        let b5 = ((px >> 3) & 0x1f) as u16;
        let o = ((px >> 26) & 0x20) as u16;
        (r5 << 11) | (g5 << 6) | b5 | o
    }

    pub(super) struct Screen {
        w: u16,
        h: u16,
        px: Mutex<Vec<u32>>,
        flushes: Mutex<Vec<(i16, i16, i16, i16)>>,
        /// Override the row methods with a clamp-once implementation (the
        /// kernel's shape) instead of the trait's per-pixel defaults.
        fast_rows: bool,
    }

    impl Screen {
        pub(super) fn new(w: u16, h: u16, seed: u64, fast_rows: bool) -> Screen {
            let mut rng = Rng(seed);
            let px = (0..w as usize * h as usize).map(|_| rng.next() as u32).collect();
            Screen { w, h, px: Mutex::new(px), flushes: Mutex::new(Vec::new()), fast_rows }
        }
        pub(super) fn snapshot(&self) -> (Vec<u32>, Vec<(i16, i16, i16, i16)>) {
            (self.px.lock().unwrap().clone(), self.flushes.lock().unwrap().clone())
        }
        fn in_bounds(&self, x: i32, y: i32) -> bool {
            x >= 0 && y >= 0 && x < self.w as i32 && y < self.h as i32
        }
    }

    impl RasterBackend for Screen {
        fn width(&self) -> u16 {
            self.w
        }
        fn height(&self) -> u16 {
            self.h
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
        fn set_active_palette(&self, _p: &[u16; 256]) {}
        fn pel_to_device16(&self, _pel: Pel) -> u16 {
            0
        }
        fn device_to_rgb565(&self, argb: u32) -> u16 {
            device_to_word(argb)
        }
        fn rgb565_to_device(&self, px: u16) -> u32 {
            word_to_device(px)
        }
        fn read_pixel(&self, _x: i16, _y: i16) -> Pel {
            0
        }
        fn fill_rect(&self, _x: i16, _y: i16, _w: i16, _h: i16, _pel: Pel, _clip: ClipRect) {}
        fn abline(
            &self,
            _x0: i16,
            _y0: i16,
            _x1: i16,
            _y1: i16,
            _pel: Pel,
            _mode: WriteMode,
            _clip: ClipRect,
            mask: u16,
            _last: bool,
        ) -> u16 {
            mask
        }
        fn fill_pattern(&self, _x: i16, _y: i16, _w: i16, _h: i16, _f: PatternFill, _clip: ClipRect) {}
        fn draw_glyph_sized(
            &self,
            _x: i16,
            _y: i16,
            _g: &[u8],
            _cw: i16,
            _ch: i16,
            _fg: Pel,
            _m: WriteMode,
            _clip: ClipRect,
        ) {
        }
        fn blit_mono(&self, _d: Rect, _s: MonoSrc, _fg: Pel, _bg: Pel, _m: WriteMode, _clip: ClipRect) {}
        fn copy_rect(&self, _s: Rect, _dx: i16, _dy: i16, _op: u8, _clip: ClipRect) {}
        fn get_device_pixel(&self, x: i16, y: i16) -> u32 {
            if !self.in_bounds(x as i32, y as i32) {
                return 0;
            }
            self.px.lock().unwrap()[y as usize * self.w as usize + x as usize]
        }
        fn put_device_pixel(&self, x: i16, y: i16, argb: u32) {
            if !self.in_bounds(x as i32, y as i32) {
                return;
            }
            self.px.lock().unwrap()[y as usize * self.w as usize + x as usize] = argb;
        }
        fn read_row_rgb565(&self, x: i16, y: i16, out: &mut [u16]) {
            if !self.fast_rows {
                // The trait default, spelled out (an override cannot call it).
                for (i, o) in out.iter_mut().enumerate() {
                    let xi = x as i32 + i as i32;
                    let px = if xi > i16::MAX as i32 { 0 } else { self.get_device_pixel(xi as i16, y) };
                    *o = self.device_to_rgb565(px);
                }
                return;
            }
            let off = device_to_word(0);
            if y < 0 || y as u16 >= self.h {
                out.fill(off);
                return;
            }
            let lo = (-(x as i64)).clamp(0, out.len() as i64) as usize;
            let hi = (self.w as i64 - x as i64).clamp(lo as i64, out.len() as i64) as usize;
            out[..lo].fill(off);
            out[hi..].fill(off);
            let px = self.px.lock().unwrap();
            let base = y as usize * self.w as usize + (x as i64 + lo as i64) as usize;
            for (i, o) in out[lo..hi].iter_mut().enumerate() {
                *o = device_to_word(px[base + i]);
            }
        }
        fn write_row_rgb565(&self, x: i16, y: i16, src: &[u16]) {
            if !self.fast_rows {
                for (i, &word) in src.iter().enumerate() {
                    let xi = x as i32 + i as i32;
                    if xi > i16::MAX as i32 {
                        break;
                    }
                    self.put_device_pixel(xi as i16, y, self.rgb565_to_device(word));
                }
                return;
            }
            if y < 0 || y as u16 >= self.h {
                return;
            }
            let lo = (-(x as i64)).clamp(0, src.len() as i64) as usize;
            let hi = (self.w as i64 - x as i64).clamp(lo as i64, src.len() as i64) as usize;
            let mut px = self.px.lock().unwrap();
            let base = y as usize * self.w as usize + (x as i64 + lo as i64) as usize;
            for (i, &word) in src[lo..hi].iter().enumerate() {
                px[base + i] = word_to_device(word);
            }
        }
        fn flush_rect(&self, x0: i16, y0: i16, x1: i16, y1: i16) {
            self.flushes.lock().unwrap().push((x0, y0, x1, y1));
        }
    }

    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            // xorshift64*
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }
    }

    /// A byte arena holding one or more MFDB buffers at arbitrary (also odd)
    /// byte offsets, seeded with random bytes.
    fn arena(len: usize, seed: u64) -> Vec<u8> {
        let mut rng = Rng(seed);
        (0..len).map(|_| rng.next() as u8).collect()
    }

    fn mfdb(base: *mut u8, byte_off: usize, w: i16, h: i16, wdwidth: i16) -> Mfdb {
        Mfdb { addr: base as u64 + byte_off as u64, w, h, wdwidth, stand: 0, nplanes: CHUNKY_PLANES }
    }

    const SW: u16 = 700; // wider than two ROW_CHUNKs
    const SH: u16 = 12;
    const FULL: ClipRect = ClipRect { xmn: 0, ymn: 0, xmx: SW as i16 - 1, ymx: SH as i16 - 1 };

    /// Every op the copy paths can see: the sixteen, and the same sixteen
    /// with a high nibble set (both `raster_op` and the new paths mask).
    fn ops() -> impl Iterator<Item = u8> {
        0u8..32
    }

    // ── the derived op sets ─────────────────────────────────────────────

    /// `op_ignores_dst` / `op_ignores_src` are the truth table of
    /// `raster_op`, not a choice: each op is a bitwise function, so it is
    /// independent of an operand iff flipping that operand's bit between 0
    /// and 1 never changes the result for either value of the other.
    #[test]
    fn op_sets_match_the_raster_op_table() {
        for op in 0u8..32 {
            let d_free = [0u16, 0xFFFF].iter().all(|&s| raster_op(op, s, 0) == raster_op(op, s, 0xFFFF));
            let s_free = [0u16, 0xFFFF].iter().all(|&d| raster_op(op, 0, d) == raster_op(op, 0xFFFF, d));
            assert_eq!(op_ignores_dst(op), d_free, "op {op}: D-independence");
            assert_eq!(op_ignores_src(op), s_free, "op {op}: S-independence");
        }
        let d_free: Vec<u8> = (0u8..16).filter(|&op| op_ignores_dst(op)).collect();
        assert_eq!(d_free, vec![0, 3, 12, 15]);
        let s_free: Vec<u8> = (0u8..16).filter(|&op| op_ignores_src(op)).collect();
        assert_eq!(s_free, vec![0, 5, 10, 15]);
    }

    /// `apply_row` is `raster_op` word for word, for every op.
    #[test]
    fn apply_row_is_raster_op() {
        let mut rng = Rng(7);
        let s: Vec<u16> = (0..300).map(|_| rng.next() as u16).collect();
        let d0: Vec<u16> = (0..300).map(|_| rng.next() as u16).collect();
        for op in ops() {
            let mut d = d0.clone();
            apply_row(op, &s, &mut d);
            for i in 0..d.len() {
                assert_eq!(d[i], raster_op(op, s[i], d0[i]), "op {op} word {i}");
            }
        }
    }

    // ── Memory → Screen ─────────────────────────────────────────────────

    /// (source rect, dx, dy, clip, fd_w, fd_h, fd_wdwidth, byte offset)
    type M2s = (Rect, i16, i16, ClipRect, i16, i16, i16, usize);

    fn m2s_cases() -> Vec<M2s> {
        let r = |x, y, w, h| Rect { x, y, w, h };
        let c = |xmn, ymn, xmx, ymx| ClipRect { xmn, ymn, xmx, ymx };
        let mut v: Vec<M2s> = vec![
            (r(0, 0, 17, 5), 3, 4, FULL, 17, 5, 2, 0),             // plain, odd width
            (r(0, 0, 1, 1), 0, 0, FULL, 1, 1, 1, 0),               // one pixel
            (r(0, 0, 3, 3), 698, 10, FULL, 3, 3, 1, 1),            // off the right/bottom edge, odd address
            (r(0, 0, 40, 6), -5, -2, FULL, 40, 6, 3, 0),           // off the left/top edge
            (r(2, 1, 30, 6), 10, 2, c(15, 3, 30, 5), 40, 8, 3, 0), // clipped on all sides
            (r(0, 0, 30, 6), 10, 2, c(50, 3, 60, 5), 40, 8, 3, 0), // clipped away
            (r(-4, -2, 30, 8), 0, 0, FULL, 20, 4, 2, 0),           // source partly outside the MFDB
            (r(0, 0, 40, 4), 0, 0, FULL, 40, 4, 1, 0),             // wdwidth too small: stride 16 < fd_w
            (r(0, 0, 40, 4), 0, 0, FULL, 40, 4, 0, 0),             // wdwidth 0
            (r(0, 0, 0, 4), 0, 0, FULL, 40, 4, 3, 0),              // empty
            (r(0, 0, -3, 4), 0, 0, FULL, 40, 4, 3, 0),             // negative width
        ];
        for (w, off) in [(255i16, 0usize), (256, 1), (257, 0), (513, 3), (700, 0), (701, 1)] {
            let wd = (w + 15) / 16;
            v.push((r(0, 0, w, 3), 0, 1, FULL, w, 3, wd, off));
            v.push((r(1, 0, w, 3), -2, 9, c(5, 0, 690, 10), w, 3, wd, off));
        }
        v
    }

    type Snap = (Vec<u32>, Vec<(i16, i16, i16, i16)>);

    fn run_m2s(fast: bool, f: fn(&dyn RasterBackend, &Mfdb, Rect, i16, i16, u8, ClipRect)) -> Vec<Snap> {
        let mut out = Vec::new();
        for (k, &(s, dx, dy, clip, fw, fh, wd, off)) in m2s_cases().iter().enumerate() {
            for op in ops() {
                let screen = Screen::new(SW, SH, 0x1234 + k as u64, fast);
                let mut buf = arena(off + (wd.max(0) as usize * 16) * fh.max(0) as usize * 2 + 8, 99 + k as u64);
                let src = mfdb(buf.as_mut_ptr(), off, fw, fh, wd);
                f(&screen, &src, s, dx, dy, op, clip);
                out.push(screen.snapshot());
            }
        }
        out
    }

    #[test]
    fn mem_to_screen_matches_the_per_pixel_path() {
        for fast in [false, true] {
            let new = run_m2s(fast, mem_to_screen);
            let old = run_m2s(fast, oracle_mem_to_screen);
            assert_eq!(new.len(), old.len());
            for (i, (n, o)) in new.iter().zip(old.iter()).enumerate() {
                assert!(n == o, "mem_to_screen differs: case {} op {} (fast_rows {fast})", i / 32, i % 32);
            }
        }
    }

    // ── Screen → Memory ─────────────────────────────────────────────────

    /// (screen source rect, dx, dy, fd_w, fd_h, fd_wdwidth, byte offset)
    type S2m = (Rect, i16, i16, i16, i16, i16, usize);

    fn s2m_cases() -> Vec<S2m> {
        let r = |x, y, w, h| Rect { x, y, w, h };
        let mut v: Vec<S2m> = vec![
            (r(3, 4, 17, 5), 0, 0, 17, 5, 2, 0),
            (r(0, 0, 1, 1), 0, 0, 1, 1, 1, 1),
            (r(690, 8, 20, 6), 0, 0, 20, 6, 2, 0),       // screen source off the right/bottom
            (r(-5, -3, 20, 6), 0, 0, 20, 6, 2, 1),       // screen source off the left/top
            (r(32760, 0, 20, 2), 0, 0, 20, 2, 2, 0),     // coordinates past i16::MAX
            (r(0, 32760, 20, 12), 0, 0, 20, 12, 2, 0),   // rows past i16::MAX
            (r(0, 0, 30, 6), -4, -2, 20, 4, 2, 0),       // destination partly outside
            (r(0, 0, 40, 4), 0, 0, 40, 4, 1, 0),         // wdwidth too small
            (r(0, 0, 40, 4), 2, 1, 40, 4, 0, 1),         // wdwidth 0
            (r(0, 0, 0, 4), 0, 0, 40, 4, 3, 0),          // empty
        ];
        for (w, off) in [(255i16, 0usize), (256, 1), (257, 0), (513, 3), (700, 0), (701, 1)] {
            let wd = (w + 15) / 16;
            v.push((r(0, 1, w, 3), 0, 0, w, 3, wd, off));
            v.push((r(-3, 9, w, 5), 1, 0, w, 3, wd, off));
        }
        v
    }

    fn run_s2m(fast: bool, f: fn(&dyn RasterBackend, &Mfdb, Rect, i16, i16, u8)) -> Vec<(Vec<u8>, Vec<u32>)> {
        let mut out = Vec::new();
        for (k, &(s, dx, dy, fw, fh, wd, off)) in s2m_cases().iter().enumerate() {
            for op in ops() {
                let screen = Screen::new(SW, SH, 0x777 + k as u64, fast);
                let mut buf = arena(off + (wd.max(0) as usize * 16) * fh.max(0) as usize * 2 + 8, 5 + k as u64);
                let dst = mfdb(buf.as_mut_ptr(), off, fw, fh, wd);
                f(&screen, &dst, s, dx, dy, op);
                out.push((buf.clone(), screen.snapshot().0));
            }
        }
        out
    }

    #[test]
    fn screen_to_mem_matches_the_per_pixel_path() {
        for fast in [false, true] {
            let new = run_s2m(fast, screen_to_mem);
            let old = run_s2m(fast, oracle_screen_to_mem);
            for (i, (n, o)) in new.iter().zip(old.iter()).enumerate() {
                assert!(n == o, "screen_to_mem differs: case {} op {} (fast_rows {fast})", i / 32, i % 32);
            }
        }
    }

    // ── Memory → Memory ─────────────────────────────────────────────────

    /// (src byte off, src w/h/wd, dst byte off, dst w/h/wd, source rect, dx, dy)
    type M2m = (usize, (i16, i16, i16), usize, (i16, i16, i16), Rect, i16, i16);

    fn m2m_cases() -> Vec<M2m> {
        let r = |x, y, w, h| Rect { x, y, w, h };
        let g = (40i16, 10i16, 3i16); // 40x10, stride 48
        let mut v: Vec<M2m> = vec![
            // separate buffers
            (0, g, 2000, g, r(0, 0, 17, 5), 3, 4),
            (1, g, 2001, g, r(-3, -2, 30, 8), -2, 1),
            (0, g, 2000, (300, 3, 19), r(0, 0, 300, 3), 0, 0),
            (0, (300, 3, 19), 2000, (300, 3, 19), r(0, 0, 300, 3), 0, 0),
            (0, (40, 4, 1), 2000, (40, 4, 1), r(0, 0, 40, 4), 0, 0), // stride < fd_w on both
            (0, g, 2000, g, r(0, 0, 0, 3), 0, 0),
        ];
        // The same buffer: every scroll direction, including same-row moves.
        for ddx in [-3i16, 0, 3] {
            for ddy in [-2i16, 0, 2] {
                v.push((0, g, 0, g, r(5, 3, 20, 4), 5 + ddx, 3 + ddy));
            }
        }
        // The same address, different geometry (stride 48 vs 32).
        v.push((0, g, 0, (40, 10, 2), r(0, 0, 30, 6), 2, 1));
        v.push((0, (40, 10, 2), 0, g, r(2, 1, 30, 6), 0, 0));
        // Overlapping buffers at different addresses, both ways, odd offsets.
        for doff in [1usize, 2, 6, 33, 96, 97] {
            v.push((0, g, doff, g, r(0, 0, 30, 6), 0, 0));
            v.push((doff, g, 0, g, r(0, 0, 30, 6), 0, 0));
            v.push((0, g, doff, g, r(2, 1, 30, 6), 3, 0));
        }
        // Same buffer, stride < fd_w: rows run on into each other (V26).
        v.push((0, (40, 6, 1), 0, (40, 6, 1), r(0, 0, 40, 5), 0, 1));
        v.push((0, (40, 6, 1), 0, (40, 6, 1), r(0, 1, 40, 5), 2, 0));
        v
    }

    fn run_m2m(f: fn(&Mfdb, &Mfdb, Rect, i16, i16, u8)) -> Vec<Vec<u8>> {
        let mut out = Vec::new();
        for (k, &(so, (sw, sh, swd), dof, (dw, dh, dwd), s, dx, dy)) in m2m_cases().iter().enumerate() {
            for op in ops() {
                let mut buf = arena(4096, 31 + k as u64);
                let base = buf.as_mut_ptr();
                let src = mfdb(base, so, sw, sh, swd);
                let dst = mfdb(base, dof, dw, dh, dwd);
                f(&src, &dst, s, dx, dy, op);
                out.push(buf.clone());
            }
        }
        out
    }

    #[test]
    fn mem_to_mem_matches_the_per_pixel_path() {
        let new = run_m2m(mem_to_mem);
        let old = run_m2m(oracle_mem_to_mem);
        for (i, (n, o)) in new.iter().zip(old.iter()).enumerate() {
            assert!(n == o, "mem_to_mem differs: case {} op {}", i / 32, i % 32);
        }
    }
}

/// V19: `vro_cpyfm` memory→memory below nine planes is EmuTOS' planar BitBlt
/// on the interleaved device form (`_raster.md` § "Memory → memory below nine
/// planes"). [`planar_mem_to_mem`] is checked against a per-pixel, per-bit
/// statement of the same contract, written from the spec and not from the
/// WORD loop: every destination pixel of the block inside the form becomes
/// `op(S, D)` bit by bit on every plane, `S` read from the source as it stood
/// before the call (0 for a pixel outside the source form), pixels addressed
/// linearly as in EmuTOS (V26: `fd_w` no limit, a block wider than its row
/// runs on into the next), and **no other byte of the arena changes** — not
/// the bits of a partial group outside the block, not the WORDs of the other
/// planes, not the bytes before and after the form. The
/// oracle side of the same contract is `tos-api/tests/vdi_raster_reference_v18.rs`
/// (17 measured cases, profile falcontc).
#[cfg(test)]
mod v19_planar_tests {
    use super::*;
    use alloc::vec::Vec;

    /// One form in the arena: byte offset, `fd_w`, `fd_h`, `fd_wdwidth`.
    type Geo = (usize, i16, i16, i16);

    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }
    }

    fn arena(len: usize, seed: u64) -> Vec<u8> {
        let mut rng = Rng(seed);
        (0..len).map(|_| rng.next() as u8).collect()
    }

    fn mfdb(base: *mut u8, g: Geo, planes: i16) -> Mfdb {
        Mfdb { addr: base as u64 + g.0 as u64, w: g.1, h: g.2, wdwidth: g.3, stand: 0, nplanes: planes }
    }

    /// Byte index and bit mask of pixel `(x, y)` plane `p` of form `g` with
    /// `planes` planes, or `None` if the pixel lies outside the form: the
    /// layout of `setup_info` (`vdi_raster.c:919-940,959-960`) addressed as
    /// EmuTOS does — linear group `y * fd_wdwidth + floor(x / 16)`, `fd_w`
    /// not a limit — and the one NewTOS bound, the form's `fd_h * fd_wdwidth`
    /// groups (V26).
    fn pixel(g: Geo, planes: usize, x: i32, y: i32, p: usize) -> Option<(usize, u16)> {
        let (off, _w, h, wd) = g;
        let lg = y as i64 * wd as i64 + (x as i64).div_euclid(16);
        if lg < 0 || lg >= wd as i64 * h as i64 {
            return None;
        }
        let word = lg as usize * planes + p;
        Some((off + 2 * word, 0x8000u16 >> x.rem_euclid(16)))
    }

    fn get(buf: &[u8], at: usize) -> u16 {
        u16::from_ne_bytes([buf[at], buf[at + 1]])
    }

    /// The contract, pixel by pixel: in EmuTOS' direction (`bit_blt
    /// :660-669` — it decides which write lands last when the block wider
    /// than its row covers a pixel twice), the source as it stood before
    /// the call, the destination as it stands when the pixel is written.
    fn model(buf: &mut [u8], (sg, dg): (Geo, Geo), planes: usize, s: Rect, (dx, dy): (i16, i16), op: u8) {
        let before = buf.to_vec();
        let start = |g: Geo, x: i32, y: i32| {
            g.0 as i64 + 2 * (y as i64 * g.3 as i64 + (x as i64).div_euclid(16)) * planes as i64
        };
        let (ss, ds) = (start(sg, s.x as i32, s.y as i32), start(dg, dx as i32, dy as i32));
        let reverse = ss < ds || (ss == ds && (s.x & 15) < (dx & 15));
        let (bw, bh) = (s.w.max(0) as i32, s.h.max(0) as i32);
        for ri in 0..bh {
            let r = if reverse { bh - 1 - ri } else { ri };
            for ci in 0..bw {
                let c = if reverse { bw - 1 - ci } else { ci };
                for p in 0..planes {
                    let Some((da, dm)) = pixel(dg, planes, dx as i32 + c, dy as i32 + r, p) else {
                        continue;
                    };
                    let sbit = pixel(sg, planes, s.x as i32 + c, s.y as i32 + r, p)
                        .is_some_and(|(sa, sm)| get(&before, sa) & sm != 0);
                    let dbit = get(buf, da) & dm != 0;
                    let s16 = if sbit { 0xFFFF } else { 0 };
                    let d16 = if dbit { 0xFFFF } else { 0 };
                    let bit = raster_op(op, s16, d16) & 1 != 0;
                    let w = get(buf, da);
                    let w = if bit { w | dm } else { w & !dm };
                    buf[da..da + 2].copy_from_slice(&w.to_ne_bytes());
                }
            }
        }
    }

    /// (source form, destination form, source rect, dx, dy) — `None` for the
    /// destination means the source form itself (an in-place copy).
    type Case = (Geo, Option<Geo>, Rect, i16, i16);

    fn cases(planes: usize) -> Vec<Case> {
        let r = |x, y, w, h| Rect { x, y, w, h };
        let a: Geo = (16, 40, 12, 3); // 40 px wide, 3 groups, padding 41..47
        let b: Geo = (3000, 40, 12, 3);
        let b_odd: Geo = (3001, 33, 9, 3); // odd address, fd_w not a multiple of 16
        let mut v: Vec<Case> = vec![
            (a, Some(b), r(0, 0, 40, 12), 0, 0),         // whole form, aligned
            (a, Some(b), r(0, 0, 16, 16), 0, 0),         // one group, rows past fd_h
            (a, Some(b), r(3, 2, 8, 8), 9, 3),           // shift right inside one group
            (a, Some(b), r(9, 3, 8, 8), 2, 1),           // shift left inside one group
            (a, Some(b), r(2, 1, 11, 9), 17, 4),         // across a group boundary
            (a, Some(b), r(5, 0, 30, 12), 0, 0),         // left shift by 5 over three groups
            (a, Some(b), r(0, 0, 30, 12), 7, 0),         // right shift by 7 over three groups
            (a, Some(b), r(1, 1, 1, 1), 38, 10),         // one pixel
            (a, Some(b), r(0, 0, 40, 12), 20, 6),        // block runs past the form
            (a, Some(b), r(-5, -3, 20, 10), -2, -4),     // both partly before the forms
            (a, Some(b), r(30, 8, 20, 10), 0, 0),        // source partly outside
            (a, Some(b_odd), r(0, 0, 40, 12), 0, 0),     // dest odd address, padding bits
            (b_odd, Some(a), r(4, 2, 30, 7), 6, 3),      // source odd address
            (a, Some((3000, 40, 12, 0)), r(0, 0, 40, 12), 0, 0), // dest wdwidth 0
            (a, Some((3000, 60, 12, 2)), r(0, 0, 40, 12), 0, 0), // dest fd_w > wdwidth * 16
            (a, Some(b), r(0, 0, 0, 12), 0, 0),          // empty
        ];
        // In place, every scroll direction, sub-WORD and whole-group shifts.
        for ddx in [-17i16, -3, 0, 3, 16] {
            for ddy in [-2i16, 0, 2] {
                v.push((a, None, r(10, 3, 20, 6), 10 + ddx, 3 + ddy));
            }
        }
        // Two MFDBs of the same geometry one row (3 groups of `planes` WORDs)
        // apart in the same buffer.
        let row = 2 * 3 * planes;
        v.push((a, Some((16 + row, 40, 12, 3)), r(2, 0, 30, 8), 5, 1));
        v.push(((16 + row, 40, 12, 3), Some(a), r(5, 1, 30, 8), 2, 0));
        v
    }

    #[test]
    fn planar_copy_matches_the_per_pixel_contract() {
        let mut n = 0;
        for planes in [1usize, 2, 4, 8] {
            for (k, &(sg, dg, s, dx, dy)) in cases(planes).iter().enumerate() {
                let dg_eff = dg.unwrap_or(sg);
                for op in 0u8..16 {
                    let seed = 0x5EED + (planes * 1000 + k * 16 + op as usize) as u64;
                    let mut got = arena(6000, seed);
                    let mut want = got.clone();
                    let base = got.as_mut_ptr();
                    let (src, dst) = (mfdb(base, sg, planes as i16), mfdb(base, dg_eff, planes as i16));
                    planar_mem_to_mem(&src, &dst, s, dx, dy, op);
                    model(&mut want, (sg, dg_eff), planes, s, (dx, dy), op);
                    let first = got.iter().zip(&want).position(|(g, w)| g != w);
                    assert!(
                        first.is_none(),
                        "planes {planes} case {k} op {op}: first differing byte {first:?} \
                         (dst form at byte {})",
                        dg_eff.0
                    );
                    n += 1;
                }
            }
        }
        assert_eq!(n, [1, 2, 4, 8].iter().map(|&p| cases(p).len() * 16).sum::<usize>());
    }

    /// The memory around and inside a destination form outside the block is
    /// not touched — the NewTOS bound, stated once more without the model:
    /// a 1-plane 16×16 form (`fd_wdwidth` 1, 16 WORDs) receiving a block that
    /// claims 32×32 changes no byte past its 32 bytes (the block wraps row
    /// into row inside the form and is cut at its end, V26), and the destination of
    /// a 4-plane copy of an 8×8 block at (4,4) changes only bits 4..11 of
    /// WORDs of rows 4..11 (VDI-DRAW-007: NewTOS used to write 16 WORDs per
    /// row of 16 pixels, up to 16-fold past the form).
    #[test]
    fn nothing_is_written_past_the_destination_form() {
        let mut buf = arena(4096, 77);
        let before = buf.clone();
        let base = buf.as_mut_ptr();
        let src = mfdb(base, (0, 32, 32, 2), 1);
        let dst = mfdb(base, (1024, 16, 16, 1), 1);
        planar_mem_to_mem(&src, &dst, Rect { x: 0, y: 0, w: 32, h: 32 }, 0, 0, 3);
        assert_eq!(&buf[..1024], &before[..1024], "source changed");
        assert_eq!(&buf[1024 + 32..], &before[1024 + 32..], "bytes past the 16x16 form changed");

        let mut buf = arena(4096, 78);
        let before = buf.clone();
        let base = buf.as_mut_ptr();
        let src = mfdb(base, (0, 16, 16, 1), 4);
        let dst = mfdb(base, (1024, 16, 16, 1), 4);
        planar_mem_to_mem(&src, &dst, Rect { x: 0, y: 0, w: 8, h: 8 }, 4, 4, 6);
        for (i, (&n, &o)) in buf.iter().zip(&before).enumerate() {
            let inside = (1024..1024 + 128).contains(&i);
            if !inside {
                assert_eq!(n, o, "byte {i} outside the destination form changed");
                continue;
            }
            let word = (i - 1024) / 2;
            let row = word / 4;
            let changed = u16::from_ne_bytes([buf[1024 + 2 * word], buf[1025 + 2 * word]])
                ^ u16::from_ne_bytes([before[1024 + 2 * word], before[1025 + 2 * word]]);
            if !(4..12).contains(&row) {
                assert_eq!(changed, 0, "row {row} changed");
            } else {
                assert_eq!(changed & !0x0FF0, 0, "row {row}: bits outside pixels 4..11 changed");
            }
        }
    }

    /// `BM_D_ONLY` (5) and the high-nibble aliases: op 5 changes nothing,
    /// and `op | 0x10` (a stripped `PAT_FLAG`) is the plain op — the
    /// function masks to the low nibble as `cpy_raster` leaves it (D-082).
    #[test]
    fn d_only_is_a_no_op_and_the_op_is_the_low_nibble() {
        let s = Rect { x: 3, y: 1, w: 20, h: 6 };
        for op in 0u8..16 {
            let mut a = arena(2048, 9 + op as u64);
            let mut b = a.clone();
            let (pa, pb) = (a.as_mut_ptr(), b.as_mut_ptr());
            planar_mem_to_mem(&mfdb(pa, (0, 32, 8, 2), 2), &mfdb(pa, (512, 32, 8, 2), 2), s, 5, 2, op);
            planar_mem_to_mem(&mfdb(pb, (0, 32, 8, 2), 2), &mfdb(pb, (512, 32, 8, 2), 2), s, 5, 2, op | 0x10);
            assert_eq!(a, b, "op {op} vs op {:#x}", op | 0x10);
            if op == 5 {
                assert_eq!(a, arena(2048, 9 + 5), "D_ONLY changed the arena");
            }
        }
    }
}

/// V26: memory forms are addressed as EmuTOS does — pixel `(x, y)` of a
/// 16-plane (chunky) form is WORD `y * fd_wdwidth * 16 + x` (`setup_info`
/// `d_nxln = fd_wdwidth * fd_nplanes * 2`, `vdi_raster.c:919-923,933-938`;
/// `vro_cpyfm16`/`vrt_cpyfm16` `form + y * width + x`, `:986-990,1241-1250`),
/// `fd_w` is not a limit: a block wider than the row runs on into the next
/// row, the padding columns up to `fd_wdwidth * 16` are read and written.
/// The one NewTOS limit is the form itself, `fd_h` rows: nothing outside
/// `[0, fd_h * fd_wdwidth * 32)` bytes from `fd_addr` is read or written,
/// and only what `Mfdb::bind_block` checked is touched. Every test calls the
/// opcode through a [`Vdipb`] (so the bind runs).
#[cfg(test)]
mod v26_row_bound_tests {
    use super::gui07_tests::{device_to_word, Screen};
    use super::*;
    use crate::uaccess::Space;
    use alloc::boxed::Box;
    use alloc::vec::Vec;

    /// An LP64 MFDB as a caller lays it out (`mt_gem.h:2757-2768`).
    #[repr(C)]
    struct Form {
        addr: u64,
        w: i16,
        h: i16,
        wdwidth: i16,
        stand: i16,
        nplanes: i16,
        r: [i16; 3],
    }

    fn form(addr: u64, w: i16, h: i16, wdwidth: i16, nplanes: i16) -> Box<Form> {
        Box::new(Form { addr, w, h, wdwidth, stand: 0, nplanes, r: [0; 3] })
    }

    /// The handlers read `PTSIN` from the VDI's one local copy (V35), which
    /// [`call`] fills as the dispatcher does; the tests take turns at it.
    static PTSIN_COPY: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Run opcode `f` with the two MFDBs, `INTIN[0..3]` and the eight
    /// `PTSIN` corners (`CONTRL[1]` = 4, copied as `dispatch_in` copies
    /// them), through an LP64 kernel-space [`Vdipb`].
    fn call(src: &Form, dst: &Form, intin: [i16; 3], pts: [i16; 8], f: impl FnOnce(&Vdipb)) {
        let _turn = PTSIN_COPY.lock().unwrap_or_else(|e| e.into_inner());
        let mut contrl = [0i16; 15];
        contrl[1] = 4;
        let p = contrl.as_mut_ptr();
        // SAFETY: pointer slots 7..10 and 11..14 lie inside the 15 WORDs.
        unsafe {
            (p.add(7) as *mut u64).write_unaligned(src as *const Form as u64);
            (p.add(11) as *mut u64).write_unaligned(dst as *const Form as u64);
        }
        let mut intin = intin;
        let mut ptsin = pts;
        let (mut intout, mut ptsout) = ([0i16; 8], [0i16; 8]);
        let block = [
            contrl.as_mut_ptr() as u64,
            intin.as_mut_ptr() as u64,
            ptsin.as_mut_ptr() as u64,
            intout.as_mut_ptr() as u64,
            ptsout.as_mut_ptr() as u64,
        ];
        // SAFETY: five valid kernel array pointers, each large enough.
        let pb = unsafe { Vdipb::from_raw(block.as_ptr() as u64, Abi::Lp64, Space::Kernel) }.unwrap();
        crate::vdi::lcl_ptsin_copy(&pb, 4).unwrap();
        f(&pb);
        assert!(!pb.faulted());
    }

    fn vwk() -> Vwk {
        let mut v = Vwk::EMPTY;
        (v.xmn_clip, v.ymn_clip, v.xmx_clip, v.ymx_clip) = (0, 0, 63, 7);
        v
    }

    const POISON: u16 = 0xA5A5;

    /// An arena of `len` bytes, every WORD `POISON`.
    fn poisoned(len: usize) -> Vec<u8> {
        POISON.to_ne_bytes().iter().copied().cycle().take(len).collect()
    }

    fn word(buf: &[u8], at: usize) -> u16 {
        u16::from_ne_bytes([buf[at], buf[at + 1]])
    }

    /// The bytes outside `[lo, hi)` are unchanged.
    fn only_inside(after: &[u8], before: &[u8], lo: usize, hi: usize, what: &str) {
        for (i, (&a, &b)) in after.iter().zip(before).enumerate() {
            if !(lo..hi).contains(&i) {
                assert_eq!(a, b, "{what}: byte {i} outside [{lo}, {hi}) changed");
            }
        }
    }

    /// The destination form: `fd_w` 40, `fd_wdwidth` 1 (16-pixel rows, 32
    /// bytes), `fd_h` 4, at byte 512 of a 1024-byte arena: bytes `[512, 640)`.
    const DST_AT: usize = 512;
    const DST_FORM: (usize, usize) = (DST_AT, DST_AT + 128);

    /// A 1024-byte arena of distinct-ish bytes.
    fn ramp() -> Vec<u8> {
        (0..1024).map(|i| (i as u8) ^ 0x5A).collect()
    }

    // ── the form is the limit ──────────────────────────────────────────

    /// A 40×1 block on the last row (3) of the destination: pixels 48..63 are
    /// in the form, 64..87 past it — cut, the bytes after the form unchanged.
    #[test]
    fn memory_to_memory_writes_nothing_past_the_form() {
        let backend = Screen::new(64, 8, 1, true);
        let mut buf = ramp();
        let before = buf.clone();
        let base = buf.as_mut_ptr() as u64;
        let src = form(base, 48, 4, 3, 16);
        let dst = form(base + DST_AT as u64, 40, 4, 1, 16);
        call(&src, &dst, [3, 0, 0], [0, 0, 39, 0, 0, 3, 39, 3], |pb| vro_cpyfm(pb, &vwk(), &backend));
        only_inside(&buf, &before, DST_AT + 96, DST_FORM.1, "vro_cpyfm memory->memory");
        for x in 0..16 {
            assert_eq!(word(&buf, DST_AT + 96 + 2 * x), word(&before, 2 * x), "pixel 48 + {x}");
        }
    }

    #[test]
    fn screen_to_memory_writes_nothing_past_the_form() {
        let backend = Screen::new(64, 8, 2, true);
        let mut buf = ramp();
        let before = buf.clone();
        let base = buf.as_mut_ptr() as u64;
        let src = form(0, 0, 0, 0, 0);
        let dst = form(base + DST_AT as u64, 40, 4, 1, 16);
        call(&src, &dst, [3, 0, 0], [0, 0, 39, 0, 0, 3, 39, 3], |pb| vro_cpyfm(pb, &vwk(), &backend));
        only_inside(&buf, &before, DST_AT + 96, DST_FORM.1, "vro_cpyfm screen->memory");
        let (px, _) = backend.snapshot();
        for (x, &p) in px.iter().take(16).enumerate() {
            assert_eq!(word(&buf, DST_AT + 96 + 2 * x), device_to_word(p), "pixel 48 + {x}");
        }
    }

    #[test]
    fn mono_to_memory_writes_nothing_past_the_form() {
        let backend = Screen::new(64, 8, 3, true);
        let mut buf: Vec<u8> = (0..1024).map(|i| (i as u8) | 1).collect();
        let before = buf.clone();
        let base = buf.as_mut_ptr() as u64;
        let src = form(base, 48, 4, 3, 1); // 1-plane, 6 bytes a row
        let dst = form(base + DST_AT as u64, 40, 4, 1, 16);
        for mode in 1..=4 {
            call(&src, &dst, [mode, 1, 0], [0, 0, 39, 0, 0, 3, 39, 3], |pb| vrt_cpyfm(pb, &vwk(), &backend, 256));
            only_inside(&buf, &before, DST_AT + 96, DST_FORM.1, "vrt_cpyfm mono->memory");
        }
    }

    /// The source form `fd_w` 40, `fd_wdwidth` 1, `fd_h` 4 at byte 256
    /// (bytes `[256, 384)`), WORD `i` holding `0x1000 + i`; every other byte
    /// of the arena is `POISON`.
    const SRC_AT: usize = 256;
    fn poisoned_source() -> Vec<u8> {
        let mut buf = poisoned(1024);
        for i in 0..64 {
            let at = SRC_AT + 2 * i;
            buf[at..at + 2].copy_from_slice(&(0x1000 + i as u16).to_ne_bytes());
        }
        buf
    }

    /// What a 40-pixel run from source pixel 48 (row 3) reads: pixels
    /// 48..63 of the form, then nothing (0) — not the poison behind it.
    fn last_row_run() -> Vec<u16> {
        (0..40).map(|x| if x < 16 { 0x1030 + x as u16 } else { 0 }).collect()
    }

    #[test]
    fn memory_to_memory_reads_nothing_past_the_form() {
        let backend = Screen::new(64, 8, 4, true);
        let mut buf = poisoned_source();
        let base = buf.as_mut_ptr() as u64;
        buf[DST_AT..DST_AT + 96].fill(0);
        let src = form(base + SRC_AT as u64, 40, 4, 1, 16);
        let dst = form(base + DST_AT as u64, 48, 1, 3, 16);
        call(&src, &dst, [3, 0, 0], [0, 3, 39, 3, 0, 0, 39, 0], |pb| vro_cpyfm(pb, &vwk(), &backend));
        let got: Vec<u16> = (0..40).map(|x| word(&buf, DST_AT + 2 * x)).collect();
        assert_eq!(got, last_row_run());
    }

    #[test]
    fn memory_to_screen_reads_nothing_past_the_form() {
        let backend = Screen::new(64, 8, 5, true);
        let mut buf = poisoned_source();
        let base = buf.as_mut_ptr() as u64;
        let src = form(base + SRC_AT as u64, 40, 4, 1, 16);
        let dst = form(0, 0, 0, 0, 0);
        call(&src, &dst, [3, 0, 0], [0, 3, 39, 3, 0, 2, 39, 2], |pb| vro_cpyfm(pb, &vwk(), &backend));
        let (px, _) = backend.snapshot();
        let got: Vec<u16> = (0..40).map(|x| device_to_word(px[2 * 64 + x])).collect();
        assert_eq!(got, last_row_run());
    }

    // ── EmuTOS' geometry inside the form ───────────────────────────────

    /// A 40×1 block at (0, 1) of the 16-pixel-row destination lands on
    /// pixels 16..55 — rows 1, 2 and half of 3, as in EmuTOS; an 8×1 block at
    /// (-4, 2) lands on pixels 28..35, the tail of row 1 and the head of 2.
    #[test]
    fn a_block_wider_than_the_row_runs_on_into_the_next_row() {
        let backend = Screen::new(64, 8, 6, true);
        for (x, y, w, first) in [(0i16, 1i16, 40usize, 16usize), (-4, 2, 8, 28)] {
            let mut buf = ramp();
            let before = buf.clone();
            let base = buf.as_mut_ptr() as u64;
            let src = form(base, 48, 4, 3, 16);
            let dst = form(base + DST_AT as u64, 40, 4, 1, 16);
            let pts = [0, 0, w as i16 - 1, 0, x, y, x + w as i16 - 1, y];
            call(&src, &dst, [3, 0, 0], pts, |pb| vro_cpyfm(pb, &vwk(), &backend));
            let at = DST_AT + 2 * first;
            only_inside(&buf, &before, at, at + 2 * w, "wrap");
            for i in 0..w {
                assert_eq!(word(&buf, at + 2 * i), word(&before, 2 * i), "block ({x},{y}) pixel {i}");
            }
        }
    }

    /// `vrt_cpyfm` wraps the same way: `MD_XOR` with an all-ones source
    /// complements pixels 16..55 and nothing else.
    #[test]
    fn mono_to_memory_runs_on_into_the_next_row() {
        let backend = Screen::new(64, 8, 7, true);
        let mut buf = ramp();
        buf[..24].fill(0xFF); // the 48×4 1-plane source, all ones
        let before = buf.clone();
        let base = buf.as_mut_ptr() as u64;
        let src = form(base, 48, 4, 3, 1);
        let dst = form(base + DST_AT as u64, 40, 4, 1, 16);
        call(&src, &dst, [3, 1, 0], [0, 0, 39, 0, 0, 1, 39, 1], |pb| vrt_cpyfm(pb, &vwk(), &backend, 256));
        let at = DST_AT + 32;
        only_inside(&buf, &before, at, at + 80, "vrt_cpyfm wrap");
        for i in 0..40 {
            assert_eq!(word(&buf, at + 2 * i), !word(&before, at + 2 * i), "pixel 16 + {i}");
        }
    }

    /// `fd_w` 10 in a 16-pixel row: the padding columns 10..15 are pixels
    /// like the others — written by a 16-wide copy into the form, read by a
    /// 16-wide copy out of it.
    #[test]
    fn the_padding_columns_are_read_and_written() {
        let backend = Screen::new(64, 8, 8, true);
        let mut buf = ramp();
        let before = buf.clone();
        let base = buf.as_mut_ptr() as u64;
        let src = form(base, 48, 4, 3, 16);
        let dst = form(base + DST_AT as u64, 10, 2, 1, 16);
        call(&src, &dst, [3, 0, 0], [0, 0, 15, 0, 0, 0, 15, 0], |pb| vro_cpyfm(pb, &vwk(), &backend));
        only_inside(&buf, &before, DST_AT, DST_AT + 32, "padding write");
        for x in 0..16 {
            assert_eq!(word(&buf, DST_AT + 2 * x), word(&before, 2 * x), "column {x}");
        }

        let mut buf = poisoned_source();
        let base = buf.as_mut_ptr() as u64;
        let src = form(base + SRC_AT as u64, 10, 4, 1, 16);
        let scr = form(0, 0, 0, 0, 0);
        call(&src, &scr, [3, 0, 0], [0, 0, 15, 0, 0, 0, 15, 0], |pb| vro_cpyfm(pb, &vwk(), &backend));
        let (px, _) = backend.snapshot();
        let got: Vec<u16> = (0..16).map(|x| device_to_word(px[x])).collect();
        assert_eq!(got, (0..16).map(|i| 0x1000 + i).collect::<Vec<u16>>(), "padding read");
    }

    /// What is bound covers what is touched: every pixel (chunky) or group
    /// (planar) of a block that lies inside the form lies in the hull the
    /// opcode binds — over blocks before, across and past the form.
    #[test]
    fn the_bound_hull_holds_every_touched_pixel() {
        let geo = [(40i16, 4i16, 1i16), (48, 3, 3), (10, 5, 2), (16, 1, 1)];
        let blocks = [(0i32, 0i32, 40i32, 1i32), (-20, 1, 50, 3), (5, -2, 30, 4), (0, 3, 100, 2), (-40, -1, 8, 9)];
        for &(w, h, wd) in &geo {
            for planes in [16i16, 1, 4] {
                let m = Mfdb { addr: 0x1000, w, h, wdwidth: wd, stand: 0, nplanes: planes };
                for &(x, y, bw, bh) in &blocks {
                    let ch = m.chunky_hull(x, y, bw, bh);
                    let ph = m.planar_hull(x, y, bw, bh);
                    for r in 0..bh {
                        for c in 0..bw {
                            if let Some(i) = m.mem_index(x + c, y + r) {
                                let (lo, hi) = ch.expect("a touched pixel but no hull");
                                assert!((lo..hi).contains(&(i as i64)), "{w}x{h}/{wd} block {x},{y}: pixel {i}");
                            }
                            let lg = (y + r) as i64 * wd as i64 + ((x + c) as i64).div_euclid(16);
                            if (0..wd as i64 * h as i64).contains(&lg) {
                                let (lo, hi) = ph.expect("a touched group but no hull");
                                assert!((lo..hi).contains(&lg), "{w}x{h}/{wd} block {x},{y}: group {lg}");
                            }
                        }
                    }
                }
            }
        }
    }
}

/// V28: `vrt_cpyfm` into a memory form below nine planes is EmuTOS' planar
/// BitBlt (`cpy_raster`, `vdi_raster.c:1387-1458`), and a chunky memory form
/// of 9..15 planes has a row of `fd_wdwidth * fd_nplanes` WORDs
/// (`setup_info`, `:919-940`). [`planar_blit`] with [`vrt_planar_op`] is
/// checked against the expansion `_raster.md` states for `vrt_cpyfm`,
/// written per pixel and per plane bit, not from the op table: a source
/// 1-bit sets plane `p` to bit `p` of the foreground register (`MD_REPLACE`,
/// `MD_TRANS`) or complements it (`MD_XOR`), a 0-bit sets it to the
/// background register's bit (`MD_REPLACE`, `MD_ERASE`), every other case
/// leaves it; the registers are `MAP_COL` of the pens.
#[cfg(test)]
mod v28_tests {
    use super::*;

    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }
    }

    fn arena(len: usize, seed: u64) -> Vec<u8> {
        let mut rng = Rng(seed);
        (0..len).map(|_| rng.next() as u8).collect()
    }

    fn get(buf: &[u8], at: usize) -> u16 {
        u16::from_ne_bytes([buf[at], buf[at + 1]])
    }

    /// Byte index and mask of pixel `(x, y)` plane `p` of a form at byte
    /// `off` with `wd` groups per row, `h` rows and `planes` planes; `None`
    /// outside the form's `h * wd` groups (V26).
    fn pixel(off: usize, wd: i16, h: i16, planes: usize, x: i32, y: i32, p: usize) -> Option<(usize, u16)> {
        let lg = y as i64 * wd as i64 + (x as i64).div_euclid(16);
        (0..wd as i64 * h as i64)
            .contains(&lg)
            .then(|| (off + 2 * (lg as usize * planes + p), 0x8000u16 >> x.rem_euclid(16)))
    }

    const MODES: [WriteMode; 4] =
        [WriteMode::Replace, WriteMode::Transparent, WriteMode::Xor, WriteMode::RevTransparent];

    #[test]
    fn map_col_is_emutos_256_colour_table() {
        // `vdi_col.c:28-29,736-742` with numcolors 256.
        let want = [0u16, 255, 1, 2, 4, 6, 3, 5, 7, 8, 9, 10, 12, 14, 11, 13];
        for (pen, &w) in want.iter().enumerate() {
            assert_eq!(map_col(pen as u8, 256), w, "pen {pen}");
        }
        assert_eq!(map_col(16, 256), 16);
        assert_eq!(map_col(254, 256), 254);
        assert_eq!(map_col(255, 256), 15);
        assert_eq!(map_col(1, 16), 15, "pen 1 = numcolors - 1");
    }

    #[test]
    fn planar_expand_matches_the_per_pixel_contract() {
        let r = |x, y, w, h| Rect { x, y, w, h };
        // Source: 1 plane, 48 x 10 (3 groups) at byte 8; destination: 40 x 12,
        // 3 groups per row, at byte 2001 (odd).
        let (s_off, s_wd, s_h) = (8usize, 3i16, 10i16);
        let (d_off, d_wd, d_h) = (2001usize, 3i16, 12i16);
        let blocks = [
            (r(0, 0, 40, 10), 0i16, 0i16),
            (r(3, 2, 8, 8), 9, 3),
            (r(9, 1, 25, 7), 2, 4),
            (r(0, 0, 30, 10), 20, 6), // runs past the destination form
            (r(-5, -3, 20, 10), -2, -4),
        ];
        let pens: [(u8, u8); 5] = [(1, 0), (5, 10), (0, 1), (3, 2), (200, 255)];
        let mut n = 0;
        for planes in 1usize..=8 {
            for (k, &(s, dx, dy)) in blocks.iter().enumerate() {
                for &mode in &MODES {
                    for &(fg, bg) in &pens {
                        let seed = 0x28 + (planes * 1000 + k * 100 + fg as usize * 3 + bg as usize) as u64;
                        let mut got = arena(6000, seed);
                        let mut want = got.clone();
                        let base = got.as_mut_ptr() as u64;
                        let src = Mfdb { addr: base + s_off as u64, w: 48, h: s_h, wdwidth: s_wd, stand: 0, nplanes: 1 };
                        let dst =
                            Mfdb { addr: base + d_off as u64, w: 40, h: d_h, wdwidth: d_wd, stand: 0, nplanes: planes as i16 };
                        let (fgc, bgc) = (map_col(fg, 256), map_col(bg, 256));
                        planar_blit(&src, &dst, s, dx, dy, &|p| vrt_planar_op(mode, fgc, bgc, p), true);

                        // The model: the two forms do not overlap, so the order
                        // is free; the source is read from the untouched copy.
                        let before = want.clone();
                        for row in 0..s.h as i32 {
                            for col in 0..s.w as i32 {
                                let sbit = pixel(s_off, s_wd, s_h, 1, s.x as i32 + col, s.y as i32 + row, 0)
                                    .is_some_and(|(a, m)| get(&before, a) & m != 0);
                                for p in 0..planes {
                                    let Some((a, m)) =
                                        pixel(d_off, d_wd, d_h, planes, dx as i32 + col, dy as i32 + row, p)
                                    else {
                                        continue;
                                    };
                                    let d = get(&want, a) & m != 0;
                                    let (f, b) = ((fgc >> p) & 1 != 0, (bgc >> p) & 1 != 0);
                                    let bit = match mode {
                                        WriteMode::Replace => if sbit { f } else { b },
                                        WriteMode::Transparent => if sbit { f } else { d },
                                        WriteMode::Xor => sbit != d,
                                        WriteMode::RevTransparent => if sbit { d } else { b },
                                    };
                                    let w = get(&want, a);
                                    let w = if bit { w | m } else { w & !m };
                                    want[a..a + 2].copy_from_slice(&w.to_ne_bytes());
                                }
                            }
                        }
                        let first = got.iter().zip(&want).position(|(g, w)| g != w);
                        assert!(
                            first.is_none(),
                            "planes {planes} block {k} mode {mode:?} pens {fg}/{bg}: first differing byte {first:?}"
                        );
                        n += 1;
                    }
                }
            }
        }
        assert_eq!(n, 8 * 5 * 4 * 5);
    }

    #[test]
    fn a_chunky_form_of_nine_to_fifteen_planes_has_a_row_of_wdwidth_times_planes_words() {
        for planes in 9i16..=16 {
            let m = Mfdb { addr: 0x1000, w: 32, h: 4, wdwidth: 2, stand: 0, nplanes: planes };
            let row = 2 * planes as usize;
            assert_eq!(m.mem_stride_px(), row);
            assert_eq!(m.mem_index(5, 3), Some(3 * row + 5));
            // The form is `fd_h * fd_wdwidth * fd_nplanes` WORDs; the next is out.
            assert_eq!(m.mem_index(0, 4), None);
            assert_eq!(m.mem_index(row as i32 * 4 - 1, 0), Some(row * 4 - 1));
        }
    }
}
