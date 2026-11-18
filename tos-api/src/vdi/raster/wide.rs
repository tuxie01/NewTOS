//! The 32-plane device memory form (`vdiplanes=32`, ADR-038 points 5-7 and
//! 10, TASK/V65; spec `docs/api/vdi/raster/_raster.md`, "32-plane setting:
//! memory forms, copies, `vr_trnfm`, `v_get_pixel`, save buffers").
//!
//! At the 32-plane setting the screen counts as 32 planes and the device
//! memory form is `fd_stand = 0`, `fd_nplanes = 32`: one native-endian 32-bit
//! pixel `0xXXRRGGBB` per position (little-endian bytes B, G, R, X), a row of
//! `fd_wdwidth * 16` pixels ([`super::Mfdb::mem_stride_px`]; `fd_wdwidth *
//! fd_nplanes` would double it), addressed as the 16-bit form is (V26) with
//! 4-byte pixels. The copies move the **raw store word**, X included,
//! through the backend's raw rectangle seam
//! ([`RasterBackend::read_device_rect`] / [`RasterBackend::write_device_rect`],
//! the kernel's NEON row copy, ADR-036), and the sixteen logic ops act
//! bitwise on all 32 bits. The copy walks of `super` are written once over
//! [`MemPx`]; at 16 planes they are the `u16` instances, word for word the
//! code before V65.
//!
//! Accepted memory forms at that setting: 1..8 planes (planar, unchanged) and
//! 32; 9..31 — a 16-plane form included, owner choice O5 — are a silent no-op
//! before anything is bound ([`refused_at_depth`]). The plane cap, the
//! `bb_save` stamp and the save-buffer size follow one depth source,
//! [`device_mem_planes`] (the stop of TASK/V62). The colour-icon form stays
//! 16-bit until TASK/V67 and is expanded per pixel at the blit
//! ([`MemPx::fetch_row`]). At 16 planes nothing here applies (ADR-038 point 11).

use alloc::alloc::{alloc_zeroed, dealloc, Layout};

use super::super::backend::{RasterBackend, VDI_PLANES_WIDE};
use super::{Mfdb, CHUNKY_PLANES, PALETTE_PLANES_MAX, ROW_CHUNK};

/// The plane count of a 32-plane device memory form (ADR-038 point 5).
pub(super) const WIDE_PLANES: i16 = VDI_PLANES_WIDE as i16;

/// **The one depth source** of the device memory form (TASK/V65): the
/// `device_mem` stamp of `bb_save`/`bb_restore` ([`save_form`]), the size
/// `bb_save_words` gives the AES for that form, and the plane cap of
/// `vro_cpyfm`/`vrt_cpyfm` (EmuTOS `plane_ct <= 16`, `vdi_raster.c:962-966`,
/// raised to the setting's depth, ADR-038 point 5). 32 at the 32-plane
/// setting, [`CHUNKY_PLANES`] (16) otherwise — at 16 planes and on a palette
/// backend exactly the constant it was.
#[inline]
pub(super) fn device_mem_planes(backend: &dyn RasterBackend) -> i16 {
    if backend.planes() == VDI_PLANES_WIDE {
        WIDE_PLANES
    } else {
        CHUNKY_PLANES
    }
}

/// Whether a form of `planes` planes is refused at the setting's depth `cap`
/// ([`device_mem_planes`]) as a silent no-op before anything is bound: at the
/// 32-plane setting a form of 9..31 planes has no pixel size — a 16-plane
/// memory MFDB included (owner choice O5), also as both forms of a
/// memory-to-memory copy (`_raster.md`, 32-plane setting §2, reading 1 of
/// TASK/V65). 1..8 (planar) and 32 (chunky) are accepted; above the cap the
/// caller's own cap refuses. Never at 16 planes, where 9..16 is the chunky
/// 16-bit form (V28).
#[inline]
pub(super) fn refused_at_depth(planes: i16, cap: i16) -> bool {
    cap == WIDE_PLANES && planes > PALETTE_PLANES_MAX && planes != WIDE_PLANES
}

/// The device form of a `bb_save` buffer at `addr` for a `w × h` rectangle,
/// stamped at the setting's depth ([`device_mem_planes`]): what `bb_save`
/// and `bb_restore` copy through and what `bb_save_words` sizes, so the
/// buffer and the stride the copy walks cannot disagree.
#[inline]
pub(super) fn save_form(backend: &dyn RasterBackend, addr: u64, w: i16, h: i16) -> Mfdb {
    Mfdb::device_mem(addr, w, h, device_mem_planes(backend))
}

/// Run `$e` with the type alias `$T` set to the pixel width of a chunky form
/// of `$planes` planes: `u32` for a 32-plane form, `u16` otherwise.
macro_rules! at_px {
    ($planes:expr, $T:ident => $e:expr) => {
        if $planes == $crate::vdi::raster::wide::WIDE_PLANES {
            type $T = u32;
            $e
        } else {
            type $T = u16;
            $e
        }
    };
}

impl Mfdb {
    /// Bytes per pixel of a chunky memory form: 4 for the 32-plane form
    /// (ADR-038 point 5), 2 (one `u16`) otherwise.
    #[inline]
    pub(super) fn px_bytes(&self) -> usize {
        if self.nplanes == WIDE_PLANES {
            4
        } else {
            2
        }
    }

    /// The `u32` words of the form (`fd_h` rows of [`Mfdb::mem_stride_px`]
    /// pixels of [`Self::px_bytes`]): `bb_save_words` of a [`save_form`].
    #[inline]
    pub(super) fn save_words(&self) -> usize {
        self.mem_span_px() as usize * self.px_bytes() / 4
    }
}

/// A pixel of a **chunky** memory form: the 16-bit word of a 9..16-plane form
/// (`u16`), the 32-bit pixel `0xXXRRGGBB` of a 32-plane form (`u32`, ADR-038
/// point 5). The screen side of a copy is the same width: the 16-bit screen
/// words of the GUI07 row seam, or the raw store words.
pub(super) trait MemPx:
    Copy
    + core::ops::Not<Output = Self>
    + core::ops::BitAnd<Output = Self>
    + core::ops::BitOr<Output = Self>
    + core::ops::BitXor<Output = Self>
{
    /// All bits clear (`BM_ALL_WHITE`); `!ZERO` is `BM_ALL_BLACK`.
    const ZERO: Self;
    /// Bytes per pixel.
    const BYTES: usize;
    /// The screen run `(x .. x + out.len(), y)` as pixels of this width; a
    /// position off the screen reads [`Self::off_screen`]. `out.len() <=
    /// ROW_CHUNK`.
    fn read_screen_row(backend: &dyn RasterBackend, x: i16, y: i16, out: &mut [Self]);
    /// Store a run on the screen from `(x, y)`; positions off the screen are
    /// dropped. No flush.
    fn write_screen_row(backend: &dyn RasterBackend, x: i16, y: i16, src: &[Self]);
    /// What a screen position off the screen reads as.
    fn off_screen(backend: &dyn RasterBackend) -> Self;
    /// The run of memory row `row` from column `col` of `src` as pixels of
    /// this width ([`Mfdb::read_row`]).
    ///
    /// # Safety
    /// As [`Mfdb::read_row`]; `out.len() <= ROW_CHUNK`.
    unsafe fn fetch_row(backend: &dyn RasterBackend, src: &Mfdb, col: i32, row: i32, out: &mut [Self]);
}

impl MemPx for u16 {
    const ZERO: u16 = 0;
    const BYTES: usize = 2;
    /// The 16-bit screen words (`device_to_rgb565`), the seam of GUI07.
    #[inline]
    fn read_screen_row(backend: &dyn RasterBackend, x: i16, y: i16, out: &mut [u16]) {
        backend.read_row_rgb565(x, y, out);
    }
    #[inline]
    fn write_screen_row(backend: &dyn RasterBackend, x: i16, y: i16, src: &[u16]) {
        backend.write_row_rgb565(x, y, src);
    }
    #[inline]
    fn off_screen(backend: &dyn RasterBackend) -> u16 {
        backend.device_to_rgb565(0)
    }
    #[inline]
    unsafe fn fetch_row(_backend: &dyn RasterBackend, src: &Mfdb, col: i32, row: i32, out: &mut [u16]) {
        // SAFETY: the caller's contract.
        unsafe { src.read_row(col, row, out) }
    }
}

impl MemPx for u32 {
    const ZERO: u32 = 0;
    const BYTES: usize = 4;
    /// The **raw store words**, X included (`_raster.md`, 32-plane setting
    /// §3, reading 3 of TASK/V65), through the backend's raw rectangle read
    /// one row high — the kernel's NEON row copy (ADR-036); off the screen 0,
    /// as `get_device_pixel`.
    #[inline]
    fn read_screen_row(backend: &dyn RasterBackend, x: i16, y: i16, out: &mut [u32]) {
        backend.read_device_rect(x, y, out.len() as u16, 1, out);
    }
    /// The words as they are: X is unused on the screen and every read masks it
    /// (V63 R5).
    #[inline]
    fn write_screen_row(backend: &dyn RasterBackend, x: i16, y: i16, src: &[u32]) {
        backend.write_device_rect(x, y, src.len() as u16, 1, src);
    }
    #[inline]
    fn off_screen(_backend: &dyn RasterBackend) -> u32 {
        0
    }
    /// A 32-plane form's pixels as they are. A **16-bit** form read for a
    /// 32-bit screen walk is the colour-icon form at the 32-plane setting
    /// (`blit_device_mem`, reading 4 of TASK/V65): each 16-bit pixel is
    /// expanded to its device pixel ([`RasterBackend::rgb565_to_device`]), so
    /// the op runs against the raw 32-bit screen word.
    #[inline]
    unsafe fn fetch_row(backend: &dyn RasterBackend, src: &Mfdb, col: i32, row: i32, out: &mut [u32]) {
        if src.nplanes == WIDE_PLANES {
            // SAFETY: the caller's contract.
            unsafe { src.read_row(col, row, out) };
            return;
        }
        let mut words = [0u16; ROW_CHUNK];
        let words = &mut words[..out.len()];
        // SAFETY: the caller's contract; `words` is a stack buffer.
        unsafe { src.read_row(col, row, words) };
        for (o, &w) in out.iter_mut().zip(words.iter()) {
            *o = backend.rgb565_to_device(w);
        }
    }
}

/// The 32-plane `vr_trnfm` branch (ADR-038 point 6, TASK/V65): EmuTOS'
/// `vr_trnfm16` (`vdi_raster.c:190-256`) with 32 for 16 (`_raster.md`,
/// 32-plane setting §5). Converts between the **32-plane standard form** — 32
/// plane-sequential planes of `planesize = fd_h * fd_wdwidth` native 16-bit
/// words each (read from the source MFDB), plane `p` holding bit `31 - p` of
/// the pixel (planes 0..7 X, 8..15 R, 16..23 G, 24..31 B), pixel `k` at bit
/// `0x8000 >> (k & 15)` of word `k / 16` — and the **32-plane device form**,
/// `planesize * 16` native-endian 32-bit pixels. Both are `planesize * 64`
/// bytes. Direction by `src.stand` (`!= 0` standard to device). Every plane is
/// carried, X included, so device to standard and back (or the reverse) is
/// the identity bit for bit.
///
/// As `vr_trnfm16`: the output is zeroed before the bits are set; in place
/// (`src.addr == dst.addr`) through a zeroed scratch, a failed allocation a
/// silent no-op; the destination's `fd_stand` is **not** written. Every
/// access is unaligned-safe (the caller's `fd_addr`). Both forms are bound by
/// the caller (`vr_trnfm`).
pub(super) fn vr_trnfm32(src: &Mfdb, dst: &Mfdb) {
    let planesize = (src.h.max(0) as usize) * (src.wdwidth.max(0) as usize);
    if planesize == 0 {
        return;
    }
    let bytes = planesize * 64;
    let s = src.addr as *const u8;
    let inplace = src.addr == dst.addr;

    let scratch = if inplace {
        let Ok(layout) = Layout::from_size_align(bytes, 4) else {
            return;
        };
        // SAFETY: `bytes >= 64` (planesize >= 1), so the layout is non-zero.
        let p = unsafe { alloc_zeroed(layout) };
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
            // SAFETY: the dst MFDB describes `bytes` bytes at `dst.addr` (bound).
            unsafe { core::ptr::write_bytes(dst.addr as *mut u8, 0, bytes) };
            dst.addr as *mut u8
        }
    };

    // SAFETY: `s` and `d` each hold `bytes` bytes (the bound source form, the
    // bound destination or the scratch); plane word `p * planesize + w` (`p <
    // 32`, `w < planesize`) lies below `planesize * 32` 16-bit words and pixel
    // `w * 16 + j` (`j < 16`) below `planesize * 16` 32-bit pixels, both
    // `bytes`. In place `d` is the scratch, disjoint from `s`.
    unsafe {
        if src.stand != 0 {
            // standard (32 plane-sequential planes) -> device (chunky): plane
            // `p` sets bit `31 - p` of each pixel.
            for p in 0..32usize {
                let dst_mask = 0x8000_0000u32 >> p;
                for w in 0..planesize {
                    let sword = (s as *const u16).add(p * planesize + w).read_unaligned();
                    for j in 0..16usize {
                        if sword & (0x8000u16 >> j) != 0 {
                            let a = (d as *mut u32).add(w * 16 + j);
                            a.write_unaligned(a.read_unaligned() | dst_mask);
                        }
                    }
                }
            }
        } else {
            // device (chunky) -> standard (32 plane-sequential planes): bit
            // `31 - p` of each pixel becomes plane `p`.
            for w in 0..planesize {
                for j in 0..16usize {
                    let px = (s as *const u32).add(w * 16 + j).read_unaligned();
                    let bit = 0x8000u16 >> j;
                    for p in 0..32usize {
                        if px & (0x8000_0000u32 >> p) != 0 {
                            let a = (d as *mut u16).add(p * planesize + w);
                            a.write_unaligned(a.read_unaligned() | bit);
                        }
                    }
                }
            }
        }
    }

    // In place: copy the finished scratch back over the shared buffer, free.
    if let Some((p, layout)) = scratch {
        // SAFETY: `p` holds `bytes` bytes (`alloc_zeroed(layout)`); `dst.addr`
        // holds `bytes` bytes (MFDB geometry, bound). Distinct allocations.
        unsafe {
            core::ptr::copy_nonoverlapping(p as *const u8, dst.addr as *mut u8, bytes);
            dealloc(p, layout);
        }
    }
}
