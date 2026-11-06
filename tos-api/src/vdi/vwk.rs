//! Virtual workstation (`Vwk`) attribute state + the handle table.
//!
//! Spec: [`docs/api/structures/vwk.md`] and
//! [`docs/api/vdi/workstation/100-v_opnvwk.md`]. A `Vwk` holds the
//! per-handle attribute state a workstation draws with. `v_opnvwk`
//! allocates one, `init_wk` fills it from the caller's `work_in`, and
//! the dispatcher resolves `contrl[6]` to it.
//!
//! NewTOS matches EmuTOS' `NUM_VDI_HANDLES` cap but, for the bring-up,
//! backs the handles with a **static slot table** rather than `Mxalloc`
//! (the `Vwk` layout is internal — not an ABI promise — so this is free
//! to choose; a heap-allocated `Vwk` can come later if needed).

use core::cell::UnsafeCell;

use super::backend::{ClipRect, Pel};
use super::font::FontRef;

/// Total VDI handles (EmuTOS `NUM_VDI_HANDLES`, make-512 profile = 64).
/// Handle 1 is the physical workstation; 2..=64 are virtual. Index 0 is
/// unused (handle 0 = "no/failed handle").
pub const HANDLE_COUNT: usize = 64;

/// The physical-workstation handle (`v_opnwk`).
pub const PHYS_HANDLE: i16 = 1;

// Attribute defaults (EmuTOS `vdi_defs.h`), expressed in our 0-based
// internal form where noted.
const DEF_LINE_INDEX: i16 = 0; // solid (style 1, stored 0-based)
const DEF_LINE_WIDTH: i16 = 1;
const SQUARED: i16 = 0; // line end style
const DEF_MARK_INDEX: i16 = 2; // marker type 3 (asterisk), 0-based
const DEF_MARK_HEIGHT: i16 = 11;
const DEF_FILL_STYLE: i16 = 0; // FIS_HOLLOW
const WM_REPLACE: i16 = 0; // writing mode (0-based: replace)
const DEF_PEN: Pel = 1; // default drawing colour (black) when out of range

/// Coordinate transform mode (`work_in[10]`): NDC vs raster.
pub const XFM_RC: i16 = 2;

/// Per-handle attribute area. The field layout is internal (see module
/// docs); the attribute groups mirror `docs/api/structures/vwk.md`.
///
/// `init_wk` populates the whole area; the workstation-lifecycle
/// milestone (D1/D2) reads `handle`/`in_use` and the geometry, while the
/// drawing/inquiry opcodes (D3+) consume the attribute groups. Hence the
/// allow: the fields are genuinely written now and read as those opcodes
/// land.
#[allow(dead_code)]
#[derive(Clone, Copy)]
pub struct Vwk {
    /// The handle this attribute area belongs to (`0` = free slot).
    pub handle: i16,
    /// Slot occupancy.
    pub in_use: bool,

    // Line attributes.
    pub line_index: i16,
    pub line_color: Pel,
    pub line_width: i16,
    pub line_beg: i16,
    pub line_end: i16,
    pub ud_ls: u16,

    // Marker attributes.
    pub mark_index: i16,
    pub mark_color: Pel,
    pub mark_height: i16,
    pub mark_scale: i16,

    // Fill attributes.
    pub fill_style: i16,
    pub fill_index: i16,
    pub fill_color: Pel,
    pub fill_per: bool,
    pub multifill: bool,
    /// User-defined fill pattern (16 rows), used when `fill_style` is
    /// `FIS_USER`; set by `vsf_udpat`.
    pub ud_patrn: [u16; 16],

    // Text attributes.
    pub text_color: Pel,
    pub style: i16,
    pub h_align: i16,
    pub v_align: i16,
    pub chup: i16,
    /// The selected font (`cur_font`, EmuTOS `Vwk.cur_font`): a built-in one or
    /// a header of a loaded chain. Set by `vst_font`/`vst_height`/`vst_point`,
    /// rendered by `v_gtext`. `init_wk` sets it to `def_font` on workstation open
    /// (`text_init2`, `vdi_text.c:574`): the 8×8 face under 400 lines, the 8×16
    /// face from 400 up
    /// ([`font::text_init_def_font`](super::font::text_init_def_font)).
    pub cur_font: FontRef,
    /// The size was last set by `vst_point` (EmuTOS `Vwk.pts_mode`): `vst_font`
    /// re-applies it through `vst_point`, else through `vst_height`.
    pub pts_mode: bool,
    /// The chain this workstation loaded (EmuTOS `Vwk.loaded_fonts`), 0 = none:
    /// the guard of `vst_load_fonts`, one chance per workstation.
    pub loaded_fonts: u64,
    /// DDA fraction for a non-exact `vst_height`/`vst_point` size (EmuTOS
    /// `Vwk.dda_inc`). `0xFFFF` means "exactly double"; only meaningful when
    /// [`scaled`](Self::scaled) is set.
    pub dda_inc: u16,
    /// `true` when the selected size is rendered by DDA-scaling the nearest
    /// ring font rather than a native raster (EmuTOS `Vwk.scaled`).
    pub scaled: bool,
    /// Text-scaling direction (EmuTOS `Vwk.t_sclsts`): `true` enlarges,
    /// `false` shrinks. Only meaningful when [`scaled`](Self::scaled) is set.
    pub t_sclsts: bool,

    // Mode + clipping.
    pub wrt_mode: i16,
    pub xfm_mode: i16,
    pub clip: bool,
    pub xmn_clip: i16,
    pub ymn_clip: i16,
    pub xmx_clip: i16,
    pub ymx_clip: i16,
}

impl Vwk {
    /// The workstation's clip rectangle as an inclusive [`ClipRect`]
    /// (always valid: `init_wk` sets it to the full screen, `vs_clip`
    /// updates it). Every drawing primitive passes this to the backend.
    pub fn clip_rect(&self) -> ClipRect {
        ClipRect {
            xmn: self.xmn_clip,
            ymn: self.ymn_clip,
            xmx: self.xmx_clip,
            ymx: self.ymx_clip,
        }
    }

    /// An empty (free) slot.
    pub const EMPTY: Vwk = Vwk {
        handle: 0,
        in_use: false,
        line_index: DEF_LINE_INDEX,
        line_color: DEF_PEN,
        line_width: DEF_LINE_WIDTH,
        line_beg: SQUARED,
        line_end: SQUARED,
        ud_ls: 0xFFFF,
        mark_index: DEF_MARK_INDEX,
        mark_color: DEF_PEN,
        mark_height: DEF_MARK_HEIGHT,
        mark_scale: 1,
        fill_style: DEF_FILL_STYLE,
        fill_index: 0,
        fill_color: DEF_PEN,
        fill_per: true,
        multifill: false,
        ud_patrn: [0; 16],
        text_color: DEF_PEN,
        style: 0,
        h_align: 0,
        v_align: 0,
        chup: 0,
        // A free slot's face is never read: `init_wk` (`text_init2`) sets
        // `def_font` for the screen before the handle is returned.
        cur_font: FontRef::System(super::font::RING[0]),
        pts_mode: false,
        loaded_fonts: 0,
        dda_inc: 0,
        scaled: false,
        t_sclsts: false,
        wrt_mode: WM_REPLACE,
        xfm_mode: XFM_RC,
        clip: false,
        xmn_clip: 0,
        ymn_clip: 0,
        xmx_clip: 0,
        ymx_clip: 0,
    };
}

/// Clamp a requested colour index to the device's palette, falling back
/// to the default pen when out of range (EmuTOS `validate_color_index`).
pub fn validate_color_index(c: i16, num_colors: u16) -> Pel {
    if c >= 0 && (c as u16) < num_colors {
        c as Pel
    } else {
        DEF_PEN
    }
}

/// Per-workstation truecolor colour management — NewTOS' equivalent of
/// EmuTOS' `VwkExt` (`vdi/vdi_defs.h:153-158`).
///
/// Under the Falcon truecolor profile (`v_planes = 16 > 8`,
/// `TRUECOLOR_MODE`) each workstation carries its **own** pseudo-palette
/// and requested-colour table, so two open workstations can hold different
/// colours on the same pen index — an EmuTOS-documented, testable behaviour
/// (`vdi_control.c:354-363`, `vdi_col.c:601-648,813-838`). `vs_color16`
/// writes only the calling workstation's tables; `vq_color16` reads them.
///
/// **Storage.** The plan (`truecolor-conversion-plan.md` §3.5) sanctions
/// either embedding the arrays in [`Vwk`] or a separate cell. NewTOS keeps
/// a static table parallel to the [`Vwk`] handle table (indexed by the same
/// handle, see [`ext`]): this keeps the 2 KiB/handle store in
/// zero-initialised `.bss` instead of bloating the image `.data` with a
/// non-zero embedded default across all 64 handles.
///
/// **MAP_COL is folded to the identity** (the pseudo-palette is indexed
/// directly by the VDI pen, not by an EmuTOS "hardware register" via
/// `MAP_COL[pen]`). This stays observably equivalent to EmuTOS' real
/// `MAP_COL` permutation (V5-008): EmuTOS stores at `palette[MAP_COL[pen]]`
/// and reads back through the same `MAP_COL[pen]`, so the indirection
/// cancels for a consistent write+read of one pen; the default seed
/// ([`crate::vdi::workstation`] `init_wk`) is laid out in matching VDI-pen
/// order (pens 16..254 map to themselves in EmuTOS too; pen 255 maps to
/// register 15, which no seeded pen writes, leaving that slot 0 — folded
/// leaves `palette[255] = 0` the same way, so `vq_color(255, flag=1)` is
/// `(0,0,0)` either way).
pub struct VwkExt {
    /// Pseudo-palette: one 16-bit pixel value per VDI pen, laid out
    /// `RRRRRGGGGG0BBBBB` (5:5:5 packed into a 565 slot with the green LSB
    /// / bit 5 always 0 — EmuTOS `vdi_defs.h:156`, `vdi_col.c:599`).
    pub palette: [u16; 256],
    /// Raw requested colour per pen (VDI 0..1000, stored **unclamped** so
    /// `vq_color(flag=0)` round-trips the exact request — EmuTOS
    /// `vdi_defs.h:157` `req_col[256][3]`, `vdi_col.c:640`).
    pub req_col: [[i16; 3]; 256],
}

impl VwkExt {
    /// A zeroed extension (pseudo-palette black, requests zero). `init_wk`
    /// seeds it from the device-global default before the workstation is
    /// used.
    pub const EMPTY: VwkExt = VwkExt {
        palette: [0; 256],
        req_col: [[0; 3]; 256],
    };

    /// Scale a VDI colour component (0..1000, pre-clamped by the caller) to
    /// 5 bits (0..31). EmuTOS `vdi2fivebits` (`vdi_col.c:588-591`), scale
    /// 1000 → 31.
    #[inline]
    fn vdi2fivebits(col: i16) -> u16 {
        ((col as u32 * 31 + 500) / 1000) as u16
    }

    /// Scale a 5-bit component (0..31) back to a VDI colour (0..1000).
    /// EmuTOS `fivebits2vdi` (`vdi_col.c:804-807`), scale 31 → 1000.
    #[inline]
    fn fivebits2vdi(col: u16) -> i16 {
        (((col & 0x1f) as u32 * 1000 + 16) / 31) as i16
    }

    /// Set pen `colnum`'s pseudo-palette entry from a clamped `rgb`
    /// (each component 0..1000). EmuTOS `set_color16` (`vdi_col.c:601-614`):
    /// quantise each gun to 5 bits and pack `(r<<11)|(g<<6)|b`. The green
    /// LSB (bit 5) stays 0 by construction ("as in TOS4, the VDI never sets
    /// the least-significant bit of green"). Folded MAP_COL (see the struct
    /// docs): the slot index is the VDI pen itself.
    pub fn set_color16(&mut self, colnum: usize, rgb: [i16; 3]) {
        self.palette[colnum] = Self::pack_color16(rgb);
    }

    /// The pseudo-palette word `set_color16` stores for an `rgb` (each
    /// component 0..1000, clamped here) — the same quantisation and packing.
    pub fn pack_color16(rgb: [i16; 3]) -> u16 {
        let r = Self::vdi2fivebits(rgb[0].clamp(0, 1000));
        let g = Self::vdi2fivebits(rgb[1].clamp(0, 1000));
        let b = Self::vdi2fivebits(rgb[2].clamp(0, 1000));
        (r << 11) | (g << 6) | b
    }

    /// The current (5-bit-quantised) colour of pen `colnum`, read back from
    /// the pseudo-palette as VDI 0..1000 triples. EmuTOS `vq_color16`
    /// "actual" branch (`vdi_col.c:832-837`): `palette[colnum]` (folded
    /// MAP_COL), then `fivebits2vdi` per gun.
    pub fn actual16(&self, colnum: usize) -> [i16; 3] {
        let rgb = self.palette[colnum];
        [
            Self::fivebits2vdi(rgb >> 11),
            Self::fivebits2vdi(rgb >> 6),
            Self::fivebits2vdi(rgb),
        ]
    }
}

/// [`VwkExt::pack_color16`] for a caller outside the VDI that builds a
/// palette table of its own from the device's requested colours the way
/// `init_wk` seeds a workstation's (the AES resource loader's colour-icon
/// transform, `aes/resource.rs` `CiconDevice::live`).
pub fn pack_color16(rgb: [i16; 3]) -> u16 {
    VwkExt::pack_color16(rgb)
}

/// Static handle table. Mirrors the `MdArenaCell` idiom: an
/// `UnsafeCell` array accessed only from the single-CPU, IRQ-off trap
/// context (serialised like the rest of the TOS-API surface).
struct VwkTableCell(UnsafeCell<[Vwk; HANDLE_COUNT]>);

// SAFETY: accessed only from the synchronous trap handler on the boot
// CPU with interrupts masked; never concurrently. Same discipline as
// the GEMDOS/BDOS mutable statics.
unsafe impl Sync for VwkTableCell {}

static TABLE: VwkTableCell = VwkTableCell(UnsafeCell::new([Vwk::EMPTY; HANDLE_COUNT]));

/// Static per-workstation truecolor extension table, parallel to [`TABLE`]
/// and indexed by the same handle (see [`VwkExt`]). Zero-initialised
/// ([`VwkExt::EMPTY`] is all-zero), so it lives in `.bss`, not `.data`.
struct VwkExtTableCell(UnsafeCell<[VwkExt; HANDLE_COUNT]>);

// SAFETY: touched only from the single-CPU, IRQ-off VDI trap path — the
// same discipline as the `Vwk` handle table (`VwkTableCell`).
unsafe impl Sync for VwkExtTableCell {}

static EXT_TABLE: VwkExtTableCell = VwkExtTableCell(UnsafeCell::new([VwkExt::EMPTY; HANDLE_COUNT]));

/// Borrow the handle table mutably.
///
/// # Safety
/// Caller must be on the single trap path (no re-entrancy).
#[allow(clippy::mut_from_ref)]
unsafe fn table() -> &'static mut [Vwk; HANDLE_COUNT] {
    // SAFETY: see `VwkTableCell`'s Sync impl.
    unsafe { &mut *TABLE.0.get() }
}

/// Resolve a handle to its truecolor extension ([`VwkExt`]), or `None` for
/// an out-of-range handle. Unlike [`resolve`], this does **not** check
/// `in_use`: `init_wk` seeds a workstation's extension while opening it, and
/// `vs_color16`/`vq_color16` only reach here for a handle the dispatcher
/// already resolved to an open workstation. The extension lives in a static
/// parallel to the `Vwk` table (see [`EXT_TABLE`]).
///
/// # Safety
/// Single trap path (no re-entrancy), as for the `Vwk` table.
pub unsafe fn ext(handle: i16) -> Option<&'static mut VwkExt> {
    if handle < 1 || handle as usize > HANDLE_COUNT {
        return None;
    }
    // SAFETY: see `VwkExtTableCell`'s Sync impl.
    let t = unsafe { &mut *EXT_TABLE.0.get() };
    Some(&mut t[(handle - 1) as usize])
}

/// Allocate a free virtual-workstation handle (2..=`HANDLE_COUNT`) and
/// return a mutable reference to its freshly-reset slot, or `None` if all
/// are in use. The slot's `handle`/`in_use` are set; the caller runs
/// `init_wk`.
///
/// # Safety
/// Single trap path (see [`table`]).
pub unsafe fn alloc_handle() -> Option<&'static mut Vwk> {
    // SAFETY: caller upholds the single-trap-path invariant.
    let t = unsafe { table() };
    for (i, slot) in t.iter_mut().enumerate() {
        // Handle 1 (index 0) is reserved for the physical workstation.
        if i == 0 {
            continue;
        }
        if !slot.in_use {
            *slot = Vwk::EMPTY;
            slot.in_use = true;
            slot.handle = (i + 1) as i16;
            return Some(slot);
        }
    }
    None
}

/// Open (or re-open) the physical workstation slot (handle 1) and
/// return a mutable reference to its freshly-reset slot. The caller runs
/// `init_wk`.
///
/// # Safety
/// Single trap path (see [`table`]).
pub unsafe fn open_physical() -> &'static mut Vwk {
    // SAFETY: caller upholds the single-trap-path invariant.
    let t = unsafe { table() };
    t[0] = Vwk::EMPTY;
    t[0].in_use = true;
    t[0].handle = PHYS_HANDLE;
    &mut t[0]
}

/// Resolve a handle to its slot, or `None` if the handle is invalid or
/// not open.
///
/// # Safety
/// Single trap path (see [`table`]).
pub unsafe fn resolve(handle: i16) -> Option<&'static mut Vwk> {
    if handle < 1 || handle as usize > HANDLE_COUNT {
        return None;
    }
    // SAFETY: caller upholds the single-trap-path invariant.
    let t = unsafe { table() };
    let slot = &mut t[(handle - 1) as usize];
    if slot.in_use {
        Some(slot)
    } else {
        None
    }
}

/// Release a handle's slot.
///
/// # Safety
/// Single trap path (see [`table`]).
pub unsafe fn free_handle(handle: i16) {
    if handle < 1 || handle as usize > HANDLE_COUNT {
        return;
    }
    // SAFETY: caller upholds the single-trap-path invariant.
    let t = unsafe { table() };
    t[(handle - 1) as usize] = Vwk::EMPTY;
}

/// Close every open virtual workstation (handles 2..=`HANDLE_COUNT`),
/// leaving the physical workstation (handle 1 / index 0) untouched.
/// Mirrors the loop in EmuTOS `vdi_v_clswk` (`vdi_control.c:579-584`),
/// which frees `vwk_ptr[2..]` and keeps `vwk_ptr[VDI_PHYS_HANDLE]`.
///
/// # Safety
/// Single trap path (see [`table`]).
pub unsafe fn close_all_virtual() {
    // SAFETY: caller upholds the single-trap-path invariant.
    let t = unsafe { table() };
    // Index 0 is the physical workstation (handle 1); skip it.
    for slot in t.iter_mut().skip(1) {
        *slot = Vwk::EMPTY;
    }
}
