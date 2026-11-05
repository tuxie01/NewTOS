//! VDI (Virtual Device Interface) — Trap #2.
//!
//! Spec: [`docs/api/vdi/_dispatcher.md`] and the per-opcode files under
//! `docs/api/vdi/`. Unlike GEMDOS/BIOS/XBIOS, the VDI is not
//! register-based: the kernel routes selector 115 (`x0`) here with a
//! single **parameter-block** pointer (`x1`), and functions are selected
//! by an opcode in `contrl[0]`, passing all inputs/outputs through five
//! WORD arrays.
//!
//! Layering (see [`backend`]): this module is the hardware-independent
//! VDI layer — dispatch, the [`vwk`] attribute state, parameter
//! marshalling. The actual pixel work goes through the swappable
//! [`backend::RasterBackend`] the kernel installs via
//! [`crate::host::Host::raster`].
//!
//! **Milestone D1/D2:** dispatcher + workstation lifecycle
//! (`v_opnvwk`/`v_opnwk`/`v_clsvwk`/`v_clswk`/`v_clrwk`). Output,
//! attribute and inquiry opcodes are added in later milestones; until
//! then a non-routed opcode is a VDI no-op (the VDI has no error
//! return — see the dispatcher spec), **not** a silent stub of a
//! drawing call.

pub mod backend;
mod attributes;
mod contour;
/// EdDI `vq_scrninfo` (opcode 102, sub-opcode 1) — a NewTOS extension
/// beyond EmuTOS, `docs/api/vdi/eddi-vq_scrninfo.md`.
pub mod eddi;
mod escape;
/// The machine's one **alpha (text) screen**: the VT52 console the VDI
/// escape sub-functions *and* the BIOS `Bconout(2, …)`/`Bconout(5, …)`
/// devices share (`docs/api/bios/_vt52.md`). EmuTOS keeps that state in the
/// BIOS VT52 console and lets the escapes write to it through
/// `Bconout`/`Crawio` (`vdi/vdi_esc.c:29-42`); NewTOS keeps it here and lets
/// the kernel's console adapter (`kernel::fbcon`) write to it, which is the
/// same single state with the call direction inverted.
pub use escape::{
    blink as alpha_blink, clear_home as alpha_clear_home, conout as alpha_conout,
    cursconf as alpha_cursconf, cursor_cell as alpha_cursor, init as alpha_init,
    rawout as alpha_rawout,
};
mod fill_patterns;
pub mod font;
mod font_data;
mod gdp;
mod input;
/// The one-pixel line walk (EmuTOS `abline` / `clip_line`), shared by every
/// raster backend so the pixel path is host-testable (V20).
pub mod line;
/// The AES's `SET_INPUT_MODE` entry point. EmuTOS' AES sets a logical input
/// device's mode through the VDI trap — `gsx_ncode(SET_INPUT_MODE, 0, 2)` in
/// `gsx_char()` (`aes/gemgsxif.c:459-463`) — and the four mode variables are
/// VDI globals (`vdi/vdi_defs.h:258`), so the AES's write lands in the same
/// state `vsin_mode`/`vqin_mode` read.
pub(crate) use input::{set_input_mode, DEV_STRING, MODE_SAMPLE};
mod inquiry;
pub mod mouse;
/// Input-subsystem entry point + queries: feed a relative-mouse event
/// (the kernel's IKBD mouse decoder calls this) and read back the cursor
/// state (used by the IKBD self-test).
pub use mouse::{
    buttons as mouse_buttons, feed_relative, hide as cursor_hide, position as mouse_position,
    show as cursor_show, show_reset as cursor_show_reset,
};
mod primitives;
mod raster;
/// AES form-manager screen save/restore (`bb_save`/`bb_restore`): a
/// `vro_cpyfm` Screen↔Memory copy over a caller-owned 32-bpp device-format
/// buffer. Used by `form_alert`/`form_dial` to preserve the screen behind
/// a dialog. See [`raster`] for the memory-MFDB format.
pub use raster::{bb_restore, bb_save, bb_save_words, blit_device_mem};
mod req_col;
mod text;
mod trig;
mod vwk;
/// The pseudo-palette packing of `set_color16`, for the AES colour-icon
/// loader's own palette table (`aes/resource.rs`).
pub use vwk::pack_color16;
mod workstation;

use crate::bdos::process::Abi;
use crate::host;
use crate::uaccess::{Space, UserFault};

/// The VDI parameter block: five pointers to WORD (`i16`) arrays
/// (`contrl`/`intin`/`ptsin`/`intout`/`ptsout`). The five block pointers
/// are the caller's pointer width — 8-byte for an LP64 caller, 4-byte for
/// an ILP32 one (ADR-020) — so [`Vdipb::from_raw`] is told the caller's
/// [`Abi`] and reads them at the right stride. The arrays they address
/// are WORD arrays (16-bit either way).
///
/// **Caller space (ARCH03 stage 3a).** The block and its arrays live in the
/// caller's address space ([`Space`]). For an EL0 program every element is
/// read and written with its rights through [`crate::uaccess`] — at the
/// moment the handler touches it, as EmuTOS's handlers touch the arrays in
/// place — so a bad array pointer is a recorded user-access fault and the
/// syscall tail ends the program, instead of a kernel abort. The block and
/// `CONTRL[0..7]` (opcode, counts, handle: read and written by every call,
/// `vdi_main.c`) are checked up front in [`Vdipb::from_raw`]; a bad one
/// stops the call before any handler runs. After a fault inside a handler
/// every further read gives 0 and every further write is dropped, so the
/// handler runs out on inert values and the program is ended on return. An
/// in-kernel caller's block holds kernel addresses and is used as it is.
pub(crate) struct Vdipb {
    /// `CONTRL[0..7]` at its **kernel** address (checked readable and
    /// writable in [`Self::from_raw`]); `CONTRL[7..]` — the MFDB / `vex_*`
    /// pointer slots — through [`Self::contrl_va`].
    contrl: *mut i16,
    /// The caller's `CONTRL` base address (for the slots past index 6).
    contrl_va: u64,
    intin: u64,
    ptsin: u64,
    intout: u64,
    ptsout: u64,
    /// The calling process's pointer-width ABI (ADR-020), recorded at
    /// construction. The raster ops need it to read the CONTRL MFDB
    /// pointers (`N_PTRINTS` = 4 LP64 / 2 ILP32) and the MFDB struct
    /// fields at the caller's width — see [`Vdipb::contrl_ptr`] and
    /// [`crate::vdi::raster`].
    abi: Abi,
    /// Whose address space the block and its arrays are in.
    space: Space,
    /// A user access of this call faulted (see the type comment).
    faulted: core::cell::Cell<bool>,
}

/// `CONTRL` words every VDI call reads or writes (`CONTRL[0..7]`: opcode,
/// `n_ptsin`, `n_ptsout`, `n_intin`, `n_intout`, sub-opcode, handle).
const CONTRL_FIXED: usize = 7;

impl Vdipb {
    /// Build from the raw `x1` pointer to the 5-pointer block, reading
    /// the block pointers at the caller's pointer width (`abi`): 8-byte
    /// LP64 pointers, or 4-byte (zero-extended) ILP32 pointers — in the
    /// caller's `space`, with `CONTRL[0..7]` checked readable and writable.
    /// `Err` (fault recorded) when the program may not use them.
    ///
    /// # Safety
    /// For [`Space::Kernel`]: `ptr` must point at five consecutive valid
    /// array pointers of the caller's width, each to a WORD array large
    /// enough for the opcode's counts.
    pub(crate) unsafe fn from_raw(ptr: u64, abi: Abi, space: Space) -> Result<Self, UserFault> {
        let mut blk = [0u64; 5];
        for (i, slot) in blk.iter_mut().enumerate() {
            *slot = abi.read_ptr_in(space, ptr, i)?;
        }
        let [contrl_va, intin, ptsin, intout, ptsout] = blk;
        let contrl = space.kptr(contrl_va, 2 * CONTRL_FIXED as u64, true)? as *mut i16;
        Ok(Self {
            contrl,
            contrl_va,
            intin,
            ptsin,
            intout,
            ptsout,
            abi,
            space,
            faulted: core::cell::Cell::new(false),
        })
    }

    /// Read WORD `i` of the caller array at `base` (0 after a fault).
    #[inline]
    fn rd(&self, base: u64, i: usize) -> i16 {
        if self.faulted.get() {
            return 0;
        }
        match self.space.get_u16(base + 2 * i as u64) {
            Ok(v) => v as i16,
            Err(_) => {
                self.faulted.set(true);
                0
            }
        }
    }

    /// Write WORD `i` of the caller array at `base` (dropped after a fault).
    #[inline]
    fn wr(&self, base: u64, i: usize, v: i16) {
        if self.faulted.get() {
            return;
        }
        if self.space.put_u16(base + 2 * i as u64, v as u16).is_err() {
            self.faulted.set(true);
        }
    }

    /// The caller's space (see [`Vdipb`]).
    #[inline]
    pub fn space(&self) -> Space {
        self.space
    }

    /// Record that a user access made outside the accessors (an MFDB, its
    /// bitmap) faulted: further reads give 0, further writes are dropped.
    #[inline]
    pub fn note_fault(&self) {
        self.faulted.set(true);
    }

    /// `true` once a user access of this call has faulted.
    #[inline]
    pub fn faulted(&self) -> bool {
        self.faulted.get()
    }

    #[inline]
    pub fn contrl(&self, i: usize) -> i16 {
        if i < CONTRL_FIXED {
            // SAFETY: `contrl` is the checked kernel address of CONTRL[0..7].
            unsafe { self.contrl.add(i).read() }
        } else {
            self.rd(self.contrl_va, i)
        }
    }

    /// The calling process's pointer-width ABI (ADR-020). Used by the
    /// raster ops to read the MFDB struct fields at the caller's width.
    #[inline]
    pub fn abi(&self) -> Abi {
        self.abi
    }

    /// Read the source (`n = 0`) / destination (`n = 1`) MFDB pointer from
    /// CONTRL at the **caller's** pointer width. gemlib packs VDI pointer
    /// `n` at CONTRL word `7 + n * N_PTRINTS`, where `N_PTRINTS =
    /// sizeof(void*) / sizeof(short)` = 4 for LP64 / 2 for ILP32
    /// (`sdk/gemlib/gemlib-0.44.0/gem_vdiP.h:177,204`); the pointer itself
    /// is an 8-byte load (LP64) or a 4-byte zero-extended load (ILP32).
    /// Honouring `N_PTRINTS` rather than assuming the LP64 layout is what
    /// keeps an ILP32 app (QED, cflib) from mis-reading its MFDB pointers.
    /// Read in the caller's space (0 after a fault).
    #[inline]
    pub fn contrl_ptr(&self, n: usize) -> u64 {
        let nptrints = match self.abi {
            Abi::Lp64 => 4,
            Abi::Ilp32 => 2,
        };
        if self.faulted.get() {
            return 0;
        }
        // The packed pointer is WORD- but not pointer-aligned; the accessors
        // read it unaligned at the caller's width.
        let va = self.contrl_va + 2 * (7 + n * nptrints) as u64;
        let r = match self.abi {
            Abi::Lp64 => self.space.get_u64(va),
            Abi::Ilp32 => self.space.get_u32(va).map(u64::from),
        };
        r.unwrap_or_else(|_| {
            self.faulted.set(true);
            0
        })
    }
    /// Write VDI pointer slot `n` back into CONTRL at the **caller's**
    /// pointer width — the inverse of [`Vdipb::contrl_ptr`], used to return
    /// the previous routine pointer from the `vex_*` calls. gemlib reads slot
    /// `n` at CONTRL word `7 + n * N_PTRINTS`
    /// (`sdk/gemlib/gemlib-0.44.0/gem_vdiP.h:177,248`), so an LP64 caller's
    /// `old` lives at word 11 and an ILP32 caller's at word 9. Written in the
    /// caller's space (dropped after a fault).
    #[inline]
    pub fn set_contrl_ptr(&self, n: usize, v: u64) {
        let nptrints = match self.abi {
            Abi::Lp64 => 4,
            Abi::Ilp32 => 2,
        };
        if self.faulted.get() {
            return;
        }
        let va = self.contrl_va + 2 * (7 + n * nptrints) as u64;
        let r = match self.abi {
            Abi::Lp64 => self.space.put_u64(va, v),
            Abi::Ilp32 => self.space.put_u32(va, v as u32),
        };
        if r.is_err() {
            self.faulted.set(true);
        }
    }
    #[inline]
    pub fn set_contrl(&self, i: usize, v: i16) {
        if i < CONTRL_FIXED {
            // SAFETY: `contrl` is the checked kernel address of CONTRL[0..7].
            unsafe { self.contrl.add(i).write(v) }
        } else {
            self.wr(self.contrl_va, i, v);
        }
    }
    #[inline]
    pub fn intin(&self, i: usize) -> i16 {
        self.rd(self.intin, i)
    }
    #[inline]
    pub fn ptsin(&self, i: usize) -> i16 {
        self.rd(self.ptsin, i)
    }
    /// Write back into the caller's INTIN array.
    ///
    /// INTIN is an *input* array and a VDI routine normally must not touch
    /// it. Exactly one EmuTOS handler does: `escfn18` (`v_dspcur`) stores
    /// `INTIN[0] = 0` before delegating to `vdi_v_show_c`, because that is
    /// how it says "show regardless of the hide count"
    /// (`vdi/vdi_esc.c:274-278`). The write lands in the caller's block and
    /// the TT reference records it, so it is reproduced rather than
    /// rounded off — see [`escape::v_escape`](super::escape::v_escape).
    #[inline]
    pub fn set_intin(&self, i: usize, v: i16) {
        self.wr(self.intin, i, v);
    }
    #[inline]
    pub fn set_intout(&self, i: usize, v: i16) {
        self.wr(self.intout, i, v);
    }
    #[inline]
    pub fn set_ptsout(&self, i: usize, v: i16) {
        self.wr(self.ptsout, i, v);
    }
}

/// The VDI trap's `d0` return value: **coordinates**.
///
/// EmuTOS keeps this as the global `flip_y` (`vdi/vdi_main.c:20`,
/// *"True if magnitudes being returned"*), clears it on entry to `screen()`
/// (`:135`) and returns it, zero-extended, as the trap's `d0`
/// (`vdi/vdi_asm.S:124-125`) — the documented dispatcher return
/// (`docs/api/vdi/_dispatcher.md`, "Return `d0` = `flip_y` (0/1)").
///
/// Its meaning is the NDC→RC contract: a handler that reports *sizes*
/// (a width, a height, a text extent) rather than screen *positions* raises
/// the flag so a coordinate-converting caller knows not to y-flip the
/// PTSOUT values it just got.
pub const FLIP_Y_COORDINATES: i16 = 0;
/// The VDI trap's `d0` return value: **magnitudes** — see
/// [`FLIP_Y_COORDINATES`]. EmuTOS raises it in exactly nine places:
/// `init_wk` (`vdi_control.c:379`), `vq_extnd` (`:631`), `vsm_height`
/// (`vdi_marker.c:49`), `vqm_attributes` (`:178`), `setup_width_height`
/// (`vdi_text.c:678`, reached from `vst_height`/`vst_point` and hence from
/// `vst_font`), `vqt_attributes` (`:1020`), `vqt_extent` (`:1058`),
/// `vqt_width` (`:1095`) and `vqt_fontinfo` (`:1173`).
pub const FLIP_Y_MAGNITUDES: i16 = 1;

// CONTRL indices with fixed meaning (see dispatcher spec).
const C_OPCODE: usize = 0;
const C_NPTSOUT: usize = 2;
const C_NINTOUT: usize = 4;
const C_HANDLE: usize = 6;

/// VDI dispatch entry. `vdipb_ptr` is the `x1` value at the `svc #2`
/// trap (selector 115 in `x0`); `abi` is the calling process's pointer
/// width (the kernel passes [`crate::bdos::process::current_abi`]). The
/// results go into the parameter-block arrays; the **return value is the
/// trap's `d0`** — EmuTOS' `flip_y`, [`FLIP_Y_COORDINATES`] or
/// [`FLIP_Y_MAGNITUDES`] (`vdi/vdi_asm.S:124-125`).
///
/// `flip_y` is not state here: EmuTOS clears it on entry to `screen()`
/// and each handler that reports magnitudes raises it unconditionally, so
/// it is exactly a function of *which handler ran to completion*. Every
/// path that reaches no handler — no raster backend, an unknown handle
/// (EmuTOS `vdi_main.c:139-148`), an unrouted opcode, a `v_opnvwk` that
/// found no free slot (`vdi_control.c:425-427,444-446`) — yields
/// [`FLIP_Y_COORDINATES`], the value `screen()` leaves in place.
///
/// # Safety
/// `vdipb_ptr` must be a valid VDI parameter block of `abi`'s pointer
/// width (see [`Vdipb`]).
pub unsafe fn dispatch(vdipb_ptr: u64, abi: Abi) -> i16 {
    // SAFETY: forwarded — a kernel-address parameter block.
    unsafe { dispatch_in(vdipb_ptr, abi, Space::Kernel) }
}

/// [`dispatch`] for a caller whose parameter block lives in `space` (ARCH03
/// stage 3a): the GEM trap passes the caller's [`Space`], so an EL0
/// program's block and arrays are used with its rights ([`Vdipb`]).
/// [`dispatch`] is this with [`Space::Kernel`], for the kernel's own callers
/// and the host tests.
///
/// # Safety
/// As [`dispatch`] for [`Space::Kernel`]; for [`Space::User`] nothing — a
/// bad user pointer is a recorded fault, not undefined behaviour.
pub unsafe fn dispatch_in(vdipb_ptr: u64, abi: Abi, space: Space) -> i16 {
    // SAFETY: caller guarantees a valid parameter block of `abi`'s width
    // (for a kernel caller); a user block is checked in its space.
    let Ok(pb) = (unsafe { Vdipb::from_raw(vdipb_ptr, abi, space) }) else {
        // A block or CONTRL the program may not use: EmuTOS takes the bus
        // error before any handler runs. Fault recorded; the syscall tail
        // ends the program. `screen()`'s `flip_y` stays 0.
        return FLIP_Y_COORDINATES;
    };

    let opcode = pb.contrl(C_OPCODE);
    // Default output counts; handlers refine them.
    pb.set_contrl(C_NPTSOUT, 0);
    pb.set_contrl(C_NINTOUT, 0);

    // Every primitive needs the device backend. With no backend
    // installed (hostless test, no framebuffer) the VDI is inert.
    let backend = match host::host().and_then(|h| h.raster()) {
        Some(b) => b,
        None => return FLIP_Y_COORDINATES,
    };

    // ── workstation open (no handle on input; must NOT reset multifill) ──
    // Both run `init_wk`, which raises `flip_y` (`vdi_control.c:379`); for
    // `v_opnvwk` only when a slot was actually allocated (`:457`).
    if opcode == 1 {
        workstation::v_opnwk(&pb, backend);
        return FLIP_Y_MAGNITUDES;
    }
    if opcode == 100 {
        return if workstation::v_opnvwk(&pb, backend) {
            FLIP_Y_MAGNITUDES
        } else {
            FLIP_Y_COORDINATES
        };
    }

    // Every other op needs a valid, open workstation (EmuTOS
    // `vdi_main.c:139-148`). Resolve it and, unless the user fill style is
    // active, reset the per-op `multifill` flag (AUD-V1-009). This borrow
    // is scoped so the mutable handle-table borrow is released before a
    // close op (`v_clswk`/`v_clsvwk`) re-borrows the table.
    let handle = pb.contrl(C_HANDLE);
    // SAFETY: single trap path.
    match unsafe { vwk::resolve(handle) } {
        Some(vwk) => {
            if vwk.fill_style != 4 {
                vwk.multifill = false;
            }
        }
        None => return FLIP_Y_COORDINATES, // no such open workstation → VDI no-op
    }

    // Point the backend's PEL resolution at THIS workstation's private
    // pseudo-palette for the op about to run: under Falcon truecolor every
    // primitive keeps a plain PEL and resolves it at the write point through
    // `CUR_WORK->ext->palette` (EmuTOS `swblit_rect_common16`,
    // `vdi_line.c:459-460`), so the active palette must track the drawing
    // workstation. Unless `vs_color` changed it, this equals the device
    // default, so `read_pixel`/inquiry stay consistent too.
    // SAFETY: single trap path; `handle` resolved to an open ws above.
    if let Some(ext) = unsafe { vwk::ext(handle) } {
        backend.set_active_palette(&ext.palette);
    }

    // EdDI `vq_scrninfo` rides on opcode 102 (E7: only with `CONTRL[5] == 1`
    // and `INTIN[0] == 2`); every other opcode-102 call is `vq_extnd` exactly
    // as in EmuTOS (`vdi_main.c:77`). Decided once, before any handler can
    // write an output array that a caller may have aliased onto INTIN.
    let scrninfo = opcode == 102 && eddi::is_scrninfo(&pb, backend);

    match opcode {
        // ── workstation close (dispatched by handle, not the vwk ref) ──
        // v_clswk (2) closes ALL virtual workstations and leaves the
        // physical one open (AUD-V1-001); v_clsvwk (101) closes a single
        // virtual workstation, with a physical-handle guard (AUD-V1-002).
        2 => workstation::v_clswk(backend),
        101 => workstation::v_clsvwk(handle),
        3 => workstation::v_clrwk(backend),

        // ── attribute setters + output primitives + inquiry (need the vwk) ──
        _ => {
            // SAFETY: single trap path; `handle` was validated above.
            if let Some(vwk) = unsafe { vwk::resolve(handle) } {
                let ncol = backend.num_colors();
                match opcode {
                    102 if scrninfo => eddi::vq_scrninfo(&pb),
                    102 => workstation::vq_extnd(&pb, vwk, backend),
                    14 => attributes::vs_color(&pb, vwk, ncol, backend),
                    15 => attributes::vsl_type(&pb, vwk),
                    16 => attributes::vsl_width(&pb, vwk),
                    17 => attributes::vsl_color(&pb, vwk, ncol),
                    108 => attributes::vsl_ends(&pb, vwk),
                    113 => attributes::vsl_udsty(&pb, vwk),
                    18 => attributes::vsm_type(&pb, vwk),
                    19 => attributes::vsm_height(&pb, vwk),
                    20 => attributes::vsm_color(&pb, vwk, ncol),
                    25 => attributes::vsf_color(&pb, vwk, ncol),
                    22 => attributes::vst_color(&pb, vwk, ncol),
                    23 => attributes::vsf_interior(&pb, vwk),
                    24 => attributes::vsf_style(&pb, vwk),
                    104 => attributes::vsf_perimeter(&pb, vwk),
                    32 => attributes::vswr_mode(&pb, vwk),
                    39 => attributes::vst_alignment(&pb, vwk),
                    106 => attributes::vst_effects(&pb, vwk),
                    13 => attributes::vst_rotation(&pb, vwk),
                    21 => attributes::vst_font(&pb, vwk),
                    12 => attributes::vst_height(&pb, vwk),
                    107 => attributes::vst_point(&pb, vwk),
                    112 => attributes::vsf_udpat(&pb, vwk, backend.planes()),
                    129 => attributes::vs_clip(&pb, vwk, backend),
                    5 => escape::v_escape(&pb, backend),
                    119 => attributes::vst_load_fonts(&pb),
                    120 => attributes::vst_unload_fonts(&pb),
                    6 => primitives::v_pline(&pb, vwk, backend),
                    7 => primitives::v_pmarker(&pb, vwk, backend),
                    8 => text::v_gtext(&pb, vwk, backend),
                    9 => primitives::v_fillarea(&pb, vwk, backend),
                    11 => gdp::v_gdp(&pb, vwk, backend),
                    103 => contour::v_contourfill(&pb, vwk, backend),
                    // ── inquiry (read-only) ──
                    26 => inquiry::vq_color(&pb, vwk, ncol, backend),
                    35 => inquiry::vql_attributes(&pb, vwk),
                    36 => inquiry::vqm_attributes(&pb, vwk),
                    37 => inquiry::vqf_attributes(&pb, vwk),
                    38 => inquiry::vqt_attributes(&pb, vwk),
                    105 => inquiry::v_get_pixel(&pb, backend),
                    33 => input::vsin_mode(&pb),
                    115 => input::vqin_mode(&pb),
                    28 => input::v_locator(&pb, backend),
                    30 => input::v_choice(&pb),
                    31 => input::v_string(&pb),
                    // ── mouse cursor + exchange vectors ──
                    111 => mouse::vsc_form(&pb, backend),
                    122 => mouse::v_show_c(&pb, backend),
                    123 => mouse::v_hide_c(backend),
                    124 => mouse::vq_mouse(&pb),
                    118 => mouse::vex_timv(&pb),
                    125 => mouse::vex_butv(&pb),
                    126 => mouse::vex_motv(&pb),
                    127 => mouse::vex_curv(&pb),
                    134 => mouse::vex_wheelv(&pb),
                    116 => inquiry::vqt_extent(&pb, vwk),
                    117 => inquiry::vqt_width(&pb, vwk),
                    128 => inquiry::vq_key_s(&pb),
                    130 => inquiry::vqt_name(&pb),
                    131 => inquiry::vqt_fontinfo(&pb, vwk),
                    114 => primitives::vr_recfl(&pb, vwk, backend),
                    121 => raster::vrt_cpyfm(&pb, vwk, backend, ncol),
                    109 => raster::vro_cpyfm(&pb, vwk, backend),
                    110 => raster::vr_trnfm(&pb, backend),
                    // ── genuine VDI no-ops + not-yet-implemented opcodes ──
                    // (v_updwk #4, v_cellarray #10, vq_cellarray #27,
                    // v_valuator #29, #34, and any out-of-range opcode). By
                    // VDI semantics an unrouted opcode is a no-op — EmuTOS
                    // maps these to `vdi_v_nop` / returns without drawing.
                    _ => {}
                }
            }
        }
    }

    // The trap's `d0`. `flip_y` is a pure function of *which* handler ran,
    // and these are EmuTOS' nine assignment sites listed by opcode — see
    // [`FLIP_Y_MAGNITUDES`] for the file:line of each. Control only reaches
    // here after a handler ran: every earlier exit already returned
    // [`FLIP_Y_COORDINATES`], the value `screen()` leaves in place.
    //
    // `vq_scrninfo` is not one of them: EmuTOS has no EdDI, and the call
    // reports no coordinates or magnitudes in PTSOUT, so it leaves the value
    // `screen()` cleared (`docs/api/vdi/eddi-vq_scrninfo.md` §3.1/§3.6 define
    // no return value).
    match opcode {
        102 if scrninfo => FLIP_Y_COORDINATES,
        12 | 19 | 21 | 36 | 38 | 102 | 107 | 116 | 117 | 131 => FLIP_Y_MAGNITUDES,
        _ => FLIP_Y_COORDINATES,
    }
}
