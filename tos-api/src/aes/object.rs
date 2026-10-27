//! AES Object Manager (`objc_*`) — Batch 1: the OBJECT tree model plus
//! `objc_draw` (42), `objc_find` (43), `objc_offset` (44).
//!
//! Spec: [`docs/api/aes/object-form.md`]; rendering extracted from EmuTOS
//! `aes/gemoblib.c` (`ob_draw`/`just_draw`).
//!
//! **Native OBJECT layout:** classic TOS packs `OBJECT` into 24 bytes
//! with a 32-bit `ob_spec`. NewTOS is native AArch64, so `ob_spec` is a
//! **64-bit** value (a pointer for text/string types, a packed box word
//! for box types) and the struct is 32 bytes. Programs build trees with
//! this layout; `addr_in[0]` points at the array.
//!
//! **Batch-1 scope (documented limits, not stubs):** types `G_BOX`,
//! `G_IBOX`, `G_STRING`, `G_BUTTON`, `G_BOXCHAR`; black-on-white look
//! (the full `gr_crack` colour word, 3D effects, icons/images, and the
//! `TEDINFO` text types `G_TEXT`/`G_BOXTEXT` land in later batches). The
//! per-object clip rectangle is honoured at screen granularity for now.

use core::sync::atomic::{AtomicI16, Ordering};

use super::Aespb;
use crate::bdos::process::{current_abi, usermem, Abi};
use crate::host::host;
use crate::vdi::backend::{ClipRect, MonoBits, MonoSrc, PatternFill, Pel, RasterBackend, Rect, WriteMode};
use tos_types::aes::{
    ACTBUTCOL, AD3DVALUE, ADJ3DOUT, ADJ3DSTD, BACKGRCOL, CHECKED, CROSSED, DEFAULT, DISABLED,
    ED_CHAR, ED_INIT, ED_START, EXIT, FL3DACT, FL3DBAK, FL3DIND, FL3DMASK, FL3DOBJ, G_BOX,
    G_BOXCHAR, G_BOXTEXT, G_BUTTON, G_CICON, G_FBOXTEXT, G_FTEXT, G_IBOX, G_ICON, G_IMAGE,
    G_STRING, G_TEXT, G_TITLE, G_USERDEF, HIDETREE, INDBUTCOL, INDIRECT, IP_HOLLOW, IP_SOLID,
    LASTOB, LK3DACT, LK3DIND, NIL, OUTLINED, SELECTED, SHADOWED, TE_CNTR, TE_LEFT, TE_RIGHT,
    WHITEBAK,
};

// ── OBJECT model ────────────────────────────────────────────────────

// The native `OBJECT`/`TEDINFO`/`ICONBLK` records live in `tos-types`
// (`tos_types::aes`, DESK43 U5b) so the desktop and native Rust programs share
// them without depending on the AES; re-exported here for the AES' own users.
pub use tos_types::aes::{IconBlk, Object, TedInfo};

// ── ObjTree: ABI-parametrized in-place OBJECT accessor (ADR-020 stage 4) ──
//
// An OBJECT tree is read/written at the **caller's** ABI width: an LP64 caller
// (a native program such as the desktop, the boot selftests, any kernel-built
// tree) has 32-byte records
// with a u64 `ob_spec` @16 and coordinates @24..30; an ILP32 caller (a
// source-ported GEM app such as QED) has 24-byte records with a u32 `ob_spec`
// @12 and coordinates @16..22 (confirmed against `Object` above + the ILP32
// relocation in `resource.rs`: `OBJECT_SIZE_ILP32=24`, `OB_TYPE_OFF=6`,
// `OB_SPEC_OFF_ILP32=12`). The link/type WORDs (next/head/tail/type/flags/state)
// share offsets 0..10 in both ABIs.
//
// The ABI is not threaded through call sites; it is read from the current
// execution context (`current_abi`) once when an accessor is built. During an
// ILP32 app's `svc #2` that is `Ilp32`; for an in-kernel (EL1) caller — the
// desktop, whose `current()` is null — it is `Lp64`, so the offsets equal the
// native `Object` struct and every read/write is byte-identical to before.

/// Field offset table for one ABI: the stride and the offsets that differ
/// between LP64 (32-byte) and ILP32 (24-byte) OBJECT records.
#[derive(Clone, Copy)]
pub(crate) struct ObjTree {
    /// VA of the OBJECT array's first record.
    base: u64,
    /// Caller ABI (pointer / record width).
    abi: Abi,
    /// Tree origin: `true` for a **kernel-built** tree ([`Self::lp64`] — the
    /// desktop's `g_screen`, kernel-built alerts/fileselector), `false` for an
    /// **app-provided** tree ([`Self::new`], reached through the AES trap). This
    /// is the privilege discriminator for a `G_USERDEF`'s `ub_code` callback:
    /// a kernel-built tree's callback is an EL1 function at this code's privilege
    /// (same-privilege call); an app tree's callback lives in the EL0 app and
    /// must not be up-called from the EL1 AES handler. It is **not** derivable
    /// from `abi`: a native LP64 app runs at EL0 yet has `abi == Lp64`, so the
    /// origin must be tracked explicitly (Memory `aes-object-abi-is-tree-abi`).
    kernel_built: bool,
    /// The process slot whose address space the tree lives in, when the tree
    /// is read **outside** that process's trap ([`Self::app_owned`], A97):
    /// every address the accessor follows is then resolved through that
    /// slot's mapping ([`Self::ua`]). `None` reads raw — the caller's own
    /// view, right inside the owner's trap and for a kernel-built tree.
    owner: Option<usize>,
}

impl ObjTree {
    /// Build an accessor over the tree at `base`, reading fields at the
    /// current execution context's ABI width.
    ///
    /// When an EL0 process is the caller, the tree is its memory and every
    /// address the accessor follows is resolved through that process's view
    /// ([`Self::ua`] → [`owner_kva`]): the kernel alias of live user memory
    /// (ARCH03 stage 1 / plan §3.6, "AES reads app trees via `kalias`"). An
    /// address that names no live memory — an index past the end of a
    /// malformed tree, a dangling pointer — then reads as nothing (a NIL
    /// record, a zero field) instead of a kernel data abort on a privileged
    /// load through `TTBR0`, which halted the machine. EmuTOS reads whatever
    /// the flat memory holds there; with no memory behind the address that
    /// is the closest answer the kernel can give without dying. A kernel
    /// caller (no process running) reads raw, as before.
    ///
    /// # Invariant
    /// `base` addresses a valid OBJECT array; every index passed to a getter/
    /// setter is in range (the GEM caller's contract, exactly as the previous
    /// `*const Object` + `tree.add(i)` code assumed).
    #[inline]
    pub(crate) fn new(base: *const Object) -> Self {
        let owner = crate::bdos::process::slot_for(crate::bdos::process::current());
        Self { base: base as u64, abi: current_abi(), kernel_built: false, owner }
    }

    /// Build an accessor over a **kernel-built** tree at `base`, forcing the
    /// native LP64 (32-byte) record layout regardless of the current execution
    /// context's ABI.
    ///
    /// Kernel-built trees — the `form_alert`/`form_error` alert array assembled
    /// on the kernel stack, and the fileselector's GEM.RSC tree — are always
    /// native `Object`s (32-byte, `u64 ob_spec`) even while an ILP32 app such as
    /// QED is the current process. Reading `current_abi()` there would pick the
    /// app's 24-byte width and misread the kernel array, so these callers build
    /// the accessor with an explicit LP64 width instead of [`Self::new`].
    ///
    /// # Invariant
    /// As [`Self::new`]: `base` addresses a valid OBJECT array and every index
    /// passed to a getter/setter is in range.
    #[inline]
    pub(crate) fn lp64(base: *const Object) -> Self {
        Self { base: base as u64, abi: Abi::Lp64, kernel_built: true, owner: None }
    }

    /// Build an accessor over an **app-provided** tree at `base`, reading fields
    /// at an explicitly supplied `abi` width (captured earlier, **not** from the
    /// current execution context) and with `kernel_built = false`.
    ///
    /// This is for a tree an EL0 GEM app hands the AES to draw **later**, when
    /// the registering app may no longer be the current process — specifically
    /// the desktop-background tree an app registers via
    /// `wind_set(DESKWH, WF_NEWDESK, tree)` ([`crate::aes::window`]). Its ABI is
    /// captured at registration (the app was current then) and replayed here;
    /// and because the tree lives in EL0 app memory, a `G_USERDEF`'s `ub_code`
    /// must be treated as an EL0 callback (`kernel_built = false` → an EL0
    /// callout from [`ob_user`], never called at EL1). [`Self::new`] would instead read
    /// `current_abi()` at build time (wrong when the app is not current), and
    /// [`Self::lp64`] would wrongly mark the app tree kernel-built (the
    /// WF_NEWDESK-background crash: TeraDesk's `ub_bckbox` called at EL1 → PXN
    /// instruction abort).
    ///
    /// # Invariant
    /// As [`Self::new`]: `base` addresses a valid OBJECT array (at `abi` width)
    /// and every index passed to a getter/setter is in range.
    #[inline]
    pub(crate) fn app_at(base: *const Object, abi: Abi) -> Self {
        Self { base: base as u64, abi, kernel_built: false, owner: None }
    }

    /// [`Self::app_at`] for a tree read **outside its owner's trap**: every
    /// address is resolved through process slot `owner`'s mapping (A97) —
    /// the `WF_NEWDESK` background tree, composited from other processes'
    /// AES calls and from kernel code. `owner == None` is
    /// [`Self::app_at`].
    ///
    /// # Invariant
    /// As [`Self::app_at`], in the owner's address space.
    #[inline]
    pub(crate) fn app_owned(base: *const Object, abi: Abi, owner: Option<usize>) -> Self {
        Self { base: base as u64, abi, kernel_built: false, owner }
    }

    /// The tree's ABI — the width at which its `ob_spec`-pointed sub-structs
    /// (TEDINFO/ICONBLK/BITBLK) are also packed. For an app tree ([`Self::new`])
    /// this is `current_abi()`; for a kernel-built tree ([`Self::lp64`]) it is
    /// forced `Lp64` even under an ILP32 app, so its LP64 sub-structs are read at
    /// their native offsets. Used to drive the ABI-aware sub-struct readers.
    #[inline]
    pub(crate) fn abi(self) -> Abi {
        self.abi
    }

    /// `true` iff this is a **kernel-built** tree ([`Self::lp64`]) — the
    /// privilege discriminator for a `G_USERDEF` callback (see the field doc).
    #[inline]
    pub(crate) fn kernel_built(self) -> bool {
        self.kernel_built
    }

    /// The tree's OBJECT-array base VA (EmuTOS `OBJECT *tree`), used to fill a
    /// `PARMBLK`'s `pb_tree` for a `G_USERDEF` callback.
    #[inline]
    pub(crate) fn tree_ptr(self) -> u64 {
        self.base
    }

    /// An accessor over another tree `base` of the **same** origin — same
    /// ABI width, same kernel-built flag, same owner mapping. The Menu
    /// Manager reads a submenu attached to the installed bar this way: it
    /// belongs to the client that installed the bar (`MNPPD`), not to
    /// whoever is calling (ARCH03 3a, F3).
    #[inline]
    pub(crate) fn with_base(self, base: *const Object) -> Self {
        Self { base: base as u64, ..self }
    }

    #[inline]
    fn stride(self) -> u64 {
        record_stride(self.abi)
    }
    #[inline]
    fn spec_off(self) -> u64 {
        match self.abi {
            Abi::Lp64 => 16,
            Abi::Ilp32 => 12,
        }
    }
    #[inline]
    fn coord_off(self) -> u64 {
        match self.abi {
            Abi::Lp64 => 24,
            Abi::Ilp32 => 16,
        }
    }
    /// The tree VA of field `off` of record `i`.
    #[inline]
    fn field_va(self, i: i16, off: u64) -> u64 {
        self.base + i as u64 * self.stride() + off
    }
    /// Where to **read** field `off` of record `i`: the field itself, or —
    /// for an owned tree ([`Self::owner`]) — its kernel alias in the owner's
    /// mapping, or the same field of [`NIL_RECORD`] when the owner has no
    /// mapping there. A field never straddles a page: every offset is even
    /// and the pointer-wide `ob_spec` sits at a pointer-aligned offset of a
    /// record whose base the caller aligned for its own accesses.
    #[inline]
    fn rd_ptr(self, i: i16, off: u64) -> *const u8 {
        match self.ua(self.field_va(i, off)) {
            Some(a) => a as *const u8,
            None => NIL_RECORD[off as usize..].as_ptr(),
        }
    }
    /// Where to **write** field `off` of record `i`, or `None` when the
    /// owner's mapping has nothing there (the write is dropped: the
    /// composite never writes, so this only guards a misuse).
    #[inline]
    fn wr_ptr(self, i: i16, off: u64) -> Option<*mut u8> {
        self.ua(self.field_va(i, off)).map(|a| a as *mut u8)
    }
    #[inline]
    fn rd_i16(self, i: i16, off: u64) -> i16 {
        // SAFETY: ObjTree invariant — `base` is a valid OBJECT array and `i`
        // is in range; a WORD field is 2 readable bytes (unaligned-safe).
        // For an owned tree the address is the owner's page alias or the
        // NIL record (`rd_ptr`).
        unsafe { (self.rd_ptr(i, off) as *const i16).read_unaligned() }
    }
    #[inline]
    fn wr_i16(self, i: i16, off: u64, v: i16) {
        if let Some(p) = self.wr_ptr(i, off) {
            // SAFETY: as `rd_i16`; the record is writable (GEM contract).
            unsafe { (p as *mut i16).write_unaligned(v) }
        }
    }
    #[inline]
    fn rd_u16(self, i: i16, off: u64) -> u16 {
        // SAFETY: as `rd_i16`.
        unsafe { (self.rd_ptr(i, off) as *const u16).read_unaligned() }
    }
    #[inline]
    fn wr_u16(self, i: i16, off: u64, v: u16) {
        if let Some(p) = self.wr_ptr(i, off) {
            // SAFETY: as `wr_i16`.
            unsafe { (p as *mut u16).write_unaligned(v) }
        }
    }

    #[inline]
    pub(crate) fn next(self, i: i16) -> i16 {
        self.rd_i16(i, 0)
    }
    #[inline]
    pub(crate) fn head(self, i: i16) -> i16 {
        self.rd_i16(i, 2)
    }
    #[inline]
    pub(crate) fn tail(self, i: i16) -> i16 {
        self.rd_i16(i, 4)
    }
    #[inline]
    pub(crate) fn ob_type(self, i: i16) -> u16 {
        self.rd_u16(i, 6)
    }
    #[inline]
    pub(crate) fn flags(self, i: i16) -> u16 {
        self.rd_u16(i, 8)
    }
    #[inline]
    pub(crate) fn state(self, i: i16) -> u16 {
        self.rd_u16(i, 10)
    }
    #[inline]
    pub(crate) fn x(self, i: i16) -> i16 {
        self.rd_i16(i, self.coord_off())
    }
    #[inline]
    pub(crate) fn y(self, i: i16) -> i16 {
        self.rd_i16(i, self.coord_off() + 2)
    }
    #[inline]
    pub(crate) fn width(self, i: i16) -> i16 {
        self.rd_i16(i, self.coord_off() + 4)
    }
    #[inline]
    pub(crate) fn height(self, i: i16) -> i16 {
        self.rd_i16(i, self.coord_off() + 6)
    }

    #[inline]
    pub(crate) fn set_next(self, i: i16, v: i16) {
        self.wr_i16(i, 0, v)
    }
    #[inline]
    pub(crate) fn set_head(self, i: i16, v: i16) {
        self.wr_i16(i, 2, v)
    }
    #[inline]
    pub(crate) fn set_tail(self, i: i16, v: i16) {
        self.wr_i16(i, 4, v)
    }
    #[inline]
    pub(crate) fn set_type(self, i: i16, v: u16) {
        self.wr_u16(i, 6, v)
    }
    #[inline]
    pub(crate) fn set_flags(self, i: i16, v: u16) {
        self.wr_u16(i, 8, v)
    }
    #[inline]
    pub(crate) fn set_state(self, i: i16, v: u16) {
        self.wr_u16(i, 10, v)
    }
    #[inline]
    pub(crate) fn set_x(self, i: i16, v: i16) {
        self.wr_i16(i, self.coord_off(), v)
    }
    #[inline]
    pub(crate) fn set_y(self, i: i16, v: i16) {
        self.wr_i16(i, self.coord_off() + 2, v)
    }
    #[inline]
    pub(crate) fn set_width(self, i: i16, v: i16) {
        self.wr_i16(i, self.coord_off() + 4, v)
    }
    #[inline]
    pub(crate) fn set_height(self, i: i16, v: i16) {
        self.wr_i16(i, self.coord_off() + 6, v)
    }

    /// Write the raw `ob_spec` field at the tree's width — the inverse of
    /// [`Self::raw_spec`]. `menu_fixup` needs it (`tree[ob].ob_spec =
    /// (LONG)D.g_acctitle[st++]`, `aes/gemmnlib.c:117`): a registered desk
    /// accessory's title pointer is stored straight into the placeholder
    /// object of whichever menu tree is installed, at that tree's pointer
    /// width. No INDIRECT resolution — EmuTOS's assignment has none either.
    #[inline]
    pub(crate) fn set_raw_spec(self, i: i16, v: u64) {
        let Some(p) = self.wr_ptr(i, self.spec_off()) else {
            return;
        };
        // SAFETY: as `raw_spec`; the record is writable (GEM contract).
        unsafe {
            match self.abi {
                Abi::Lp64 => (p as *mut u64).write_unaligned(v),
                Abi::Ilp32 => (p as *mut u32).write_unaligned(v as u32),
            }
        }
    }

    /// The raw `ob_spec` field read at the caller's width: a u64 for LP64, a u32
    /// zero-extended for ILP32. This is the value *before* INDIRECT resolution —
    /// for an `INDIRECT` object it is a pointer to the cell holding the real
    /// spec, not the spec itself. All consumers use [`Self::spec`], which
    /// resolves INDIRECT; this raw reader exists only for that resolution.
    #[inline]
    fn raw_spec(self, i: i16) -> u64 {
        let p = self.rd_ptr(i, self.spec_off());
        // SAFETY: ObjTree invariant; ob_spec is 8 (LP64) / 4 (ILP32) bytes
        // (`rd_ptr`: the field, its owner alias, or the NIL record's).
        unsafe {
            match self.abi {
                Abi::Lp64 => (p as *const u64).read_unaligned(),
                Abi::Ilp32 => (p as *const u32).read_unaligned() as u64,
            }
        }
    }

    /// `ob_spec` resolved through the `INDIRECT` flag (EmuTOS `ob_sst`,
    /// gemobjop.c:36-37; `ob_getsp`, gemobed.c:60-62). Normally the raw field:
    /// a user VA or a packed colour word; for a sub-struct type the base VA of
    /// the TEDINFO/ICONBLK/BITBLK/USERBLK read at the tree's ABI width by the
    /// sub-struct readers ([`tedinfo_read_abi`] etc.).
    ///
    /// When `ob_flags & INDIRECT` is set the raw `ob_spec` is instead a pointer
    /// to a cell holding the real spec (`*pspec = *(LONG *)ob_spec` in EmuTOS);
    /// that cell is read here at the **tree's ABI width** — u64 for an LP64 tree,
    /// u32 zero-extended for an ILP32 app tree, exactly like a direct `ob_spec`
    /// read (Memory `aes-object-abi-is-tree-abi`: the indirect read follows the
    /// tree's ABI, not blanket LP64). Resolving centrally here means every
    /// consumer (`just_draw`, `change_t`, `edit_field_t`) draws/edits the true
    /// spec instead of dereferencing the pointer cell as a colour word / wild
    /// TEDINFO/ICONBLK.
    #[inline]
    pub(crate) fn spec(self, i: i16) -> u64 {
        let raw = self.raw_spec(i);
        if self.flags(i) & INDIRECT == 0 {
            return raw;
        }
        // INDIRECT: `raw` is the VA of a cell holding the real spec at tree
        // width — read through the owner's mapping for an owned tree; a cell
        // the owner has no mapping for is the "no spec" sentinel, so the
        // object is neither dereferenced nor drawn.
        let Some(cell) = self.ua(raw) else {
            return self.spec_none();
        };
        // SAFETY: GEM contract — an INDIRECT object's ob_spec points at a
        // readable spec cell, 8 (LP64) / 4 (ILP32) bytes.
        unsafe {
            match self.abi {
                Abi::Lp64 => (cell as *const u64).read_unaligned(),
                Abi::Ilp32 => (cell as *const u32).read_unaligned() as u64,
            }
        }
    }

    /// The all-ones "no `ob_spec`" sentinel at the caller's width (EmuTOS
    /// `spec == -1L`, a 32-bit LONG): `u64::MAX` for LP64, `0xFFFF_FFFF` for
    /// ILP32 (matching the zero-extended read in [`Self::spec`]). Objects with
    /// this value are neither dereferenced nor drawn.
    #[inline]
    pub(crate) fn spec_none(self) -> u64 {
        match self.abi {
            Abi::Lp64 => u64::MAX,
            Abi::Ilp32 => 0xFFFF_FFFF,
        }
    }

    /// EmuTOS `ob_sst` border-thickness word `th` (gemobjop.c:41-70), used by
    /// `ob_center` (`form_center`) to size a SHADOWED root's drop-shadow extent
    /// and by `just_draw`'s text arm (A68: the one home of the sign fold).
    /// Reproduces the per-type `switch` for **every** object type, not just the
    /// box family:
    /// - `G_TITLE` → `1`
    /// - `G_TEXT`/`G_BOXTEXT`/`G_FTEXT`/`G_FBOXTEXT` → the TEDINFO `te_thickness`
    /// - `G_BOX`/`G_BOXCHAR`/`G_IBOX` → the signed thickness byte (bits 16..23 of
    ///   the INDIRECT-resolved colour LONG, EmuTOS `*(((char *)pspec)+1)`)
    /// - `G_BUTTON` → `-1`, one thinner for `EXIT`, one thinner for `DEFAULT`
    /// - any other type → `0`
    ///
    /// The trailing `if (th > 128) th -= 256` byte fixup is preserved.
    pub(crate) fn sst_th(self, obj: i16) -> i16 {
        let mut th: i16 = match self.ob_type(obj) & 0x00ff {
            G_TITLE => 1,
            G_TEXT | G_BOXTEXT | G_FTEXT | G_FBOXTEXT => {
                let spec = self.spec(obj);
                if spec == 0 {
                    0
                } else {
                    // SAFETY: the ob_spec of a *TEXT object is a TEDINFO packed
                    // at the tree's ABI width (INDIRECT already resolved).
                    unsafe { tedinfo_read_abi(spec, self).te_thickness }
                }
            }
            G_BOX | G_BOXCHAR | G_IBOX => ((self.spec(obj) >> 16) as u8 as i8) as i16,
            G_BUTTON => {
                let flags = self.flags(obj);
                let mut bt: i16 = -1;
                if flags & EXIT != 0 {
                    bt -= 1;
                }
                if flags & DEFAULT != 0 {
                    bt -= 1;
                }
                bt
            }
            _ => 0,
        };
        if th > 128 {
            th -= 256;
        }
        th
    }
}

// ── Reading an app tree through its owner's mapping (A97) ────────────────
//
// Every pointer the object drawer follows — the OBJECT records, an INDIRECT
// spec cell, the TEDINFO/BITBLK/ICONBLK/CICON/USERBLK sub-structs, the strings
// and the mask/data rasters they point at — is a **user VA of the process
// that built the tree**. Inside that process's own trap the raw dereference
// is right: its `TTBR0` is installed. The desktop-background tree an EL0 app
// registers with `wind_set(DESKWH, WF_NEWDESK, …)` is the one tree the AES
// draws **outside** its owner's trap: `w_drawdesk` runs from every other
// process's `wind_open`/`wind_close`/move (a `Pexec` child compositing over
// its parent's background) and from kernel code (the shell loop's
// `sh_draw`). Since
// ARCH03 stage 1 a user address means the same in every process (VA = PA,
// `bdos/process/mmu.rs`), but a raw read is still a privileged load through
// whichever `TTBR0` happens to be installed — and through none at all from
// kernel code — and it would follow a dangling pointer into freed or
// unmapped memory with a kernel data abort. Since stage 1 the same holds for
// a tree read inside its owner's trap (a malformed tree, an index past its
// end): [`ObjTree::new`] gives an EL0 caller's tree its owner too.
//
// [`ObjTree::owner`] therefore names the slot whose mapping the tree lives
// in, and every user address the drawer follows goes through [`ObjTree::ua`]
// / [`ObjTree::ua_span`]: with an owner, the kernel high-half alias of the
// page the process was handed — `usermem::resolve_kva`, the same authority
// XBIOS `Setscreen` uses for an address a program passes back
// (`xbios/screen.rs::caller_base_to_kva`) and `WF_NAME`/`WF_INFO` use for the
// title pointer (A83, `window.rs::LiveStr`); without one, the address itself
// (the raw, trap-time view every other accessor has always had). A VA the
// owner has no mapping for reads as nothing: a NIL record ([`NIL_RECORD`]),
// a zero field, an empty string, a skipped raster. Nothing is dereferenced
// through `TTBR0`. A `G_USERDEF` routine of the owner runs in the current
// process (EmuTOS, `gemoblib.c:411-426`), with the owner's rights for the
// callout ([`ob_user`], ARCH03 3c E10). [`crate::aes::window::draw_desk_tree`] builds such an accessor
// through [`render_tree_app`] with the owner recorded at registration.

impl ObjTree {
    /// The address to read `va` at: `va` itself for a tree without an
    /// owner, else the kernel high-half alias of `va` in the owner's mapping
    /// ([`owner_kva`]); `None` when the owner has no mapping there.
    #[inline]
    fn ua(self, va: u64) -> Option<u64> {
        match self.owner {
            None => Some(va),
            Some(slot) => owner_kva(slot, va),
        }
    }

    /// The address to read the `len`-byte span at `va` from, or `None` if
    /// any page of it is unmapped in the owner's table **or** the alias is
    /// not one contiguous run. Blocks are page-aligned and page-rounded,
    /// with a physically contiguous alias (`usermem::Block`, `map_pa +
    /// offset`), so checking the first page and every page boundary the
    /// span crosses establishes that the whole span is readable at
    /// `alias(va) + offset`. Without an owner the span is `va` itself.
    fn ua_span(self, va: u64, len: u64) -> Option<u64> {
        let Some(slot) = self.owner else {
            return Some(va);
        };
        let base = owner_kva(slot, va)?;
        let page = usermem::PAGE_SIZE;
        let end = va.checked_add(len)?;
        let mut p = (va | (page - 1)).wrapping_add(1); // first page boundary above `va`
        while p < end {
            if owner_kva(slot, p)? != base.wrapping_add(p - va) {
                return None;
            }
            p = p.wrapping_add(page);
        }
        Some(base)
    }

    /// The kernel address of a `len`-byte buffer at `va` that the AES is
    /// about to **write** for this tree — an editable field's `te_ptext`
    /// (`objc_edit`, the form manager's edit loop) — through the owner-aware
    /// path (ARCH03 stage 3a, F2):
    ///
    /// * no owner (a kernel-built tree, or the raw view of a kernel caller):
    ///   `va` itself, as before;
    /// * the owner is the **calling** process (its own tree, in its own
    ///   trap): checked writable with the caller's rights and its kernel
    ///   alias ([`Space::kptr`](crate::uaccess::Space::kptr)) — a buffer the
    ///   program may not write is a recorded fault, and the syscall tail ends
    ///   the program (EmuTOS: bus error in `ob_edit`'s `strcpy`);
    /// * another process's tree (read outside its owner's trap): its alias in
    ///   the owner's mapping ([`Self::ua_span`]), `None` (the write dropped)
    ///   when the owner has nothing there — no fault can be charged to a
    ///   process that is not running.
    ///
    /// A null `va` stays null. `None` means: do not touch the buffer.
    pub(crate) fn wbuf(self, va: u64, len: u64) -> Option<u64> {
        let Some(slot) = self.owner else {
            return Some(va);
        };
        if va == 0 {
            return Some(0);
        }
        if crate::bdos::process::slot_for(crate::bdos::process::current()) == Some(slot) {
            crate::uaccess::Space::User.kptr(va, len.max(1), true).ok()
        } else {
            self.ua_span(va, len.max(1))
        }
    }
}

/// The kernel high-half alias of user VA `va` in process `slot`'s mapping —
/// [`crate::aes::window::owner_kva`], the one seam both the frame builder
/// (A83) and the object drawer resolve through; the host tests substitute a
/// resolver over host buffers there.
#[inline]
fn owner_kva(slot: usize, va: u64) -> Option<u64> {
    super::window::owner_kva(slot, va)
}

/// A record every reader of an unresolvable OBJECT address is pointed at: a
/// leaf (`ob_next`/`ob_head`/`ob_tail` = `NIL`) of type 0, no flags, no
/// state, `ob_spec` 0, no extent — drawn as nothing, walked as the end of
/// a sibling chain. 32 bytes cover both record widths.
static NIL_RECORD: [u8; 32] = {
    let mut r = [0u8; 32];
    r[0] = 0xFF; // ob_next = -1
    r[1] = 0xFF;
    r[2] = 0xFF; // ob_head = -1
    r[3] = 0xFF;
    r[4] = 0xFF; // ob_tail = -1
    r[5] = 0xFF;
    r
};

/// The stride of an OBJECT record at `abi` width.
#[inline]
pub(crate) fn record_stride(abi: Abi) -> u64 {
    match abi {
        Abi::Lp64 => 32,
        Abi::Ilp32 => 24,
    }
}

/// `BITBLK` — a raw monochrome raster for a `G_IMAGE` object
/// (`obdefs.h`). `bi_pdata` is a native pointer to big-endian 16-bit
/// raster words.
#[repr(C)]
pub struct BitBlk {
    /// pointer to the mono raster data (16-bit words).
    pub bi_pdata: u64,
    /// width of one row in bytes.
    pub bi_wb: i16,
    /// height in rows.
    pub bi_hl: i16,
    /// source x within the raster.
    pub bi_x: i16,
    /// source y within the raster.
    pub bi_y: i16,
    /// foreground colour (PEL) of set bits.
    pub bi_color: i16,
}

// ── ABI-aware sub-struct readers (ADR-020 stage B) ──────────────────
//
// The `ob_spec` of a G_TEXT/G_IMAGE/G_ICON object points at a TEDINFO /
// BITBLK / ICONBLK sub-struct laid out at the **tree's** ABI width: an LP64
// tree (a native program such as the desktop, kernel-built alerts, boot
// selftests) packs 8-byte pointer
// fields, shifting the trailing WORDs; an ILP32 tree (a source-ported GEM app
// such as QED) packs 4-byte pointer fields, so every WORD sits 4 bytes lower
// per pointer. The native `TedInfo`/`BitBlk`/`IconBlk` structs above are the
// LP64 layout, so a plain `&*(spec as *const TedInfo)` misreads an ILP32 blob.
//
// `*::read_abi(base, t)` materialises the native struct by reading each field
// at its ABI-correct offset with `read_unaligned`; pointer fields are 8 bytes
// LP64 / 4 bytes zero-extended ILP32 (a user VA — reachable via the active
// TTBR0 inside the owner's trap, through the owner's mapping for an owned
// tree, A97 — exactly like the OBJECT tree and the string/bitmap data they
// point at). For an LP64 base the offsets equal the native struct, so the
// materialised value is byte-identical to the old direct dereference — LP64
// rendering is unchanged.
//
// The width is the **tree's** [`ObjTree::abi`], not `current_abi()`: a
// kernel-built (`ObjTree::lp64`) tree points at LP64 sub-structs even while an
// ILP32 app is current, so its readers must use LP64 offsets.

/// Pointer-field width for `abi`: 8 bytes LP64, 4 bytes ILP32. The leading
/// pointer fields of the three sub-structs stride by this, shifting every WORD.
#[inline]
fn abi_ptr_size(abi: Abi) -> u64 {
    match abi {
        Abi::Lp64 => 8,
        Abi::Ilp32 => 4,
    }
}

/// Read a sub-struct pointer field at `off` from `base` at the tree's ABI
/// width, zero-extending an ILP32 (4-byte) pointer to a native VA. For an
/// owned tree the field is read through the owner's mapping; a field the
/// owner has no mapping for reads as a NULL pointer (A97).
///
/// # Safety
/// `base + off` addresses a readable pointer field of the sub-struct (the GEM
/// caller's contract for a valid `ob_spec` sub-struct), sized 8 (LP64) / 4
/// (ILP32) bytes.
#[inline]
unsafe fn read_abi_ptr(t: ObjTree, base: u64, off: u64) -> u64 {
    let Some(p) = t.ua(base + off) else {
        return 0;
    };
    let p = p as *const u8;
    // SAFETY: caller's contract — `base + off` is a readable pointer field.
    unsafe {
        match t.abi() {
            Abi::Lp64 => (p as *const u64).read_unaligned(),
            Abi::Ilp32 => (p as *const u32).read_unaligned() as u64,
        }
    }
}

/// Read a sub-struct WORD field at `off` from `base` — through the owner's
/// mapping for an owned tree, 0 for a field it has no mapping for (A97).
///
/// # Safety
/// `base + off` addresses a readable 2-byte WORD field of the sub-struct.
#[inline]
unsafe fn read_abi_word(t: ObjTree, base: u64, off: u64) -> i16 {
    let Some(p) = t.ua(base + off) else {
        return 0;
    };
    // SAFETY: caller's contract — `base + off` is a readable 2-byte WORD field.
    unsafe { (p as *const i16).read_unaligned() }
}

/// Write a pointer field at `off` in `base` at the given ABI width (the inverse
/// of [`read_abi_ptr`]): an 8-byte store for LP64, the low 4 bytes for ILP32.
///
/// # Safety
/// `base + off` addresses a writable pointer field of the sub-struct, sized
/// 8 (LP64) / 4 (ILP32) bytes.
#[inline]
unsafe fn write_abi_ptr(base: u64, off: u64, abi: Abi, v: u64) {
    let p = (base + off) as *mut u8;
    // SAFETY: caller's contract — `base + off` is a writable pointer field.
    unsafe {
        match abi {
            Abi::Lp64 => (p as *mut u64).write_unaligned(v),
            Abi::Ilp32 => (p as *mut u32).write_unaligned(v as u32),
        }
    }
}

/// Write a WORD field at `off` in `base` (the inverse of [`read_abi_word`]).
///
/// # Safety
/// `base + off` addresses a writable 2-byte WORD field of the sub-struct.
#[inline]
unsafe fn write_abi_word(base: u64, off: u64, v: i16) {
    // SAFETY: caller's contract — `base + off` is a writable 2-byte WORD field.
    unsafe { ((base + off) as *mut i16).write_unaligned(v) }
}

impl BitBlk {
    /// Materialise the BITBLK at `base` at the tree's ABI width. Layout: one
    /// pointer (`bi_pdata`) @0, then five WORDs at `abi_ptr_size` (@8 LP64 /
    /// @4 ILP32). `mt_gem.h:2023-2031`.
    ///
    /// # Safety
    /// `base` is the valid `ob_spec` of a G_IMAGE object — a BITBLK packed at
    /// `abi` width.
    #[inline]
    unsafe fn read_abi(base: u64, t: ObjTree) -> BitBlk {
        let w = abi_ptr_size(t.abi()); // first WORD offset
        // SAFETY: `base` is a valid BITBLK packed at the tree's width (caller's contract).
        unsafe {
            BitBlk {
                bi_pdata: read_abi_ptr(t, base, 0),
                bi_wb: read_abi_word(t, base, w),
                bi_hl: read_abi_word(t, base, w + 2),
                bi_x: read_abi_word(t, base, w + 4),
                bi_y: read_abi_word(t, base, w + 6),
                bi_color: read_abi_word(t, base, w + 8),
            }
        }
    }
}

/// Materialise the TEDINFO at `base` at the tree's ABI width. Layout: three
/// pointers (`te_ptext`/`te_ptmplt`/`te_pvalid`), then eight WORDs at
/// `3 * abi_ptr_size` (@24 LP64 / @12 ILP32). `mt_gem.h:1988-2001`.
///
/// # Safety
/// `base` is the valid `ob_spec` of a *TEXT object — a TEDINFO packed at
/// `abi` width.
#[inline]
unsafe fn tedinfo_read_abi(base: u64, t: ObjTree) -> TedInfo {
    let ps = abi_ptr_size(t.abi());
    let w = 3 * ps; // first WORD offset
    // SAFETY: `base` is a valid TEDINFO packed at the tree's width (caller's contract).
    unsafe {
        TedInfo {
            te_ptext: read_abi_ptr(t, base, 0),
            te_ptmplt: read_abi_ptr(t, base, ps),
            te_pvalid: read_abi_ptr(t, base, 2 * ps),
            te_font: read_abi_word(t, base, w),
            te_junk1: read_abi_word(t, base, w + 2),
            te_just: read_abi_word(t, base, w + 4),
            te_color: read_abi_word(t, base, w + 6),
            te_junk2: read_abi_word(t, base, w + 8),
            te_thickness: read_abi_word(t, base, w + 10),
            te_txtlen: read_abi_word(t, base, w + 12),
            te_tmplen: read_abi_word(t, base, w + 14),
        }
    }
}

/// Materialise the ICONBLK at `base` at the tree's ABI width. Layout: three
/// pointers (`ib_pmask`/`ib_pdata`/`ib_ptext`), then eleven WORDs at
/// `3 * abi_ptr_size` (@24 LP64 / @12 ILP32). `mt_gem.h:2004-2020`.
///
/// # Safety
/// `base` is the valid `ob_spec` of a G_ICON object — an ICONBLK packed at
/// `abi` width.
#[inline]
unsafe fn iconblk_read_abi(base: u64, t: ObjTree) -> IconBlk {
    let ps = abi_ptr_size(t.abi());
    let w = 3 * ps; // first WORD offset
    // SAFETY: `base` is a valid ICONBLK packed at the tree's width (caller's contract).
    unsafe {
        IconBlk {
            ib_pmask: read_abi_ptr(t, base, 0),
            ib_pdata: read_abi_ptr(t, base, ps),
            ib_ptext: read_abi_ptr(t, base, 2 * ps),
            ib_char: read_abi_word(t, base, w),
            ib_xchar: read_abi_word(t, base, w + 2),
            ib_ychar: read_abi_word(t, base, w + 4),
            ib_xicon: read_abi_word(t, base, w + 6),
            ib_yicon: read_abi_word(t, base, w + 8),
            ib_wicon: read_abi_word(t, base, w + 10),
            ib_hicon: read_abi_word(t, base, w + 12),
            ib_xtext: read_abi_word(t, base, w + 14),
            ib_ytext: read_abi_word(t, base, w + 16),
            ib_wtext: read_abi_word(t, base, w + 18),
            ib_htext: read_abi_word(t, base, w + 20),
        }
    }
}

/// `CICON` (`obdefs.h:203-211`) — one colour variant of a `CICONBLK`, as the
/// drawer sees it **after** the loader's `fix_cicons` (`aes/resource.rs`):
/// `col_data` / `sel_data` point at the device-form image(s) (NewTOS's 16-bit
/// RGB565 memory-MFDB form, `ib_wicon × ib_hicon` pixels), `col_mask` /
/// `sel_mask` at the file's one-plane masks, `num_planes` is the device's.
/// `sel_data == 0` means the variant has no SELECTED-state image.
#[derive(Clone, Copy)]
pub struct Cicon {
    /// number of planes of the (transformed) image.
    pub num_planes: i16,
    /// pointer to the device-form image.
    pub col_data: u64,
    /// pointer to the mono mask (16-bit words).
    pub col_mask: u64,
    /// pointer to the device-form SELECTED image, 0 = none.
    pub sel_data: u64,
    /// pointer to the SELECTED mask, 0 = none.
    pub sel_mask: u64,
    /// next variant (0 after `fix_cicons`).
    pub next_res: u64,
}

impl Cicon {
    /// Materialise the CICON at `base` at the tree's ABI width. Layout: a WORD
    /// `num_planes` @0, padding to the pointer width, then five pointers at
    /// `k * abi_ptr_size` for `k` = 1..=5 (`mt_gem.h` `cicon_data`; LP64 48
    /// bytes, ILP32 24).
    ///
    /// # Safety
    /// `base` is a valid `CICONBLK.mainlist` — a CICON packed at `abi` width.
    #[inline]
    unsafe fn read_abi(base: u64, t: ObjTree) -> Cicon {
        let ps = abi_ptr_size(t.abi());
        // SAFETY: `base` is a valid CICON packed at the tree's width (caller's contract).
        unsafe {
            Cicon {
                num_planes: read_abi_word(t, base, 0),
                col_data: read_abi_ptr(t, base, ps),
                col_mask: read_abi_ptr(t, base, 2 * ps),
                sel_data: read_abi_ptr(t, base, 3 * ps),
                sel_mask: read_abi_ptr(t, base, 4 * ps),
                next_res: read_abi_ptr(t, base, 5 * ps),
            }
        }
    }
}

/// `CICONBLK.mainlist` (`obdefs.h:213-217`) of the CICONBLK at `base`: the
/// pointer right after the ICONBLK head (@48 LP64 / @36 ILP32), 0 when the
/// loader found no variant for the device (the mono fallback).
///
/// # Safety
/// `base` is the valid `ob_spec` of a G_CICON object — a CICONBLK packed at
/// `abi` width.
#[inline]
unsafe fn ciconblk_mainlist(base: u64, t: ObjTree) -> u64 {
    // ICONBLK = three pointers + eleven WORDs, padded to the pointer width.
    let head = match t.abi() {
        Abi::Lp64 => 48,
        Abi::Ilp32 => 36,
    };
    // SAFETY: caller's contract — `base + head` is the `mainlist` field.
    unsafe { read_abi_ptr(t, base, head) }
}

/// `USERBLK` (`obdefs.h`) — the descriptor a `G_USERDEF` object's `ob_spec`
/// points at: a draw/change callback plus one opaque parameter LONG. Both
/// fields are read at the tree's ABI width (pointer-sized), so a kernel-built
/// LP64 USERBLK holds a native 64-bit `ub_code`.
#[derive(Clone, Copy)]
pub struct UserBlk {
    /// `WORD (*ub_code)(PARMBLK *)` — the user draw/change routine (native VA).
    pub ub_code: u64,
    /// `ub_parm` — an opaque parameter passed straight through in `pb_parm`.
    pub ub_parm: u64,
}

impl UserBlk {
    /// Materialise the USERBLK at `base` at the tree's ABI width: two
    /// pointer-sized fields (`ub_code` @0, `ub_parm` @`abi_ptr_size`).
    ///
    /// # Safety
    /// `base` is the valid `ob_spec` of a `G_USERDEF` object — a USERBLK packed
    /// at `abi` width.
    #[inline]
    unsafe fn read_abi(base: u64, t: ObjTree) -> UserBlk {
        let ps = abi_ptr_size(t.abi());
        // SAFETY: `base` is a valid USERBLK packed at the tree's width (caller's contract).
        unsafe {
            UserBlk {
                ub_code: read_abi_ptr(t, base, 0),
                ub_parm: read_abi_ptr(t, base, ps),
            }
        }
    }
}

/// `PARMBLK` (`obdefs.h`) — the block passed by pointer to a `G_USERDEF`'s
/// `ub_code` callback. Native NewTOS layout (`#[repr(C)]`), built on the kernel
/// stack in [`ob_user`] and handed to a **kernel-built** (same-privilege, EL1)
/// callback. Field order matches EmuTOS: tree, object, prev/curr state, the
/// object rect (`pb_x..pb_h`), the current clip rect (`pb_xc..pb_hc`), and the
/// USERBLK's `ub_parm`.
#[repr(C)]
pub struct ParmBlk {
    /// `pb_tree` — the OBJECT-array base VA.
    pub pb_tree: u64,
    /// `pb_obj` — index of the userdef object.
    pub pb_obj: i16,
    /// `pb_prevstate` — the object's state before the change.
    pub pb_prevstate: i16,
    /// `pb_currstate` — the object's state after the change.
    pub pb_currstate: i16,
    /// object rect x (absolute).
    pub pb_x: i16,
    /// object rect y (absolute).
    pub pb_y: i16,
    /// object rect width.
    pub pb_w: i16,
    /// object rect height.
    pub pb_h: i16,
    /// clip rect x.
    pub pb_xc: i16,
    /// clip rect y.
    pub pb_yc: i16,
    /// clip rect width.
    pub pb_wc: i16,
    /// clip rect height.
    pub pb_hc: i16,
    /// `pb_parm` — the USERBLK's opaque parameter.
    pub pb_parm: u64,
}

impl ParmBlk {
    /// Write this PARMBLK into `dst` (a kernel VA aliasing the EL0 scratch
    /// region) at the tree's `abi` width, matching the app's C `PARMBLK`
    /// (`mt_gem.h`): `OBJECT *pb_tree; short pb_obj..pb_hc; long pb_parm;`.
    /// The pointer fields (`pb_tree`, `pb_parm`) are ABI-width (8 LP64 / 4
    /// ILP32); the eleven WORDs are ABI-neutral. `pb_parm` follows the WORD
    /// block at the next pointer-aligned offset — the natural C-struct
    /// padding (LP64: `pb_parm` @32 in a 40-byte struct; ILP32: @28 in a
    /// 32-byte struct). Mirror of [`UserBlk::read_abi`] / the `ObjTree`
    /// field-offset machinery (AES-as-thread design §4).
    ///
    /// # Safety
    /// `dst .. dst + 40` is a writable region (the reserved PARMBLK scratch
    /// aliased through the kernel high-half); `abi` is the tree's ABI.
    unsafe fn write_abi(&self, dst: u64, abi: Abi) {
        let ps = abi_ptr_size(abi); // 8 (LP64) / 4 (ILP32)
        let w = ps; // first WORD offset (pb_obj), just past the pointer
        // pb_parm follows the 11 WORDs (22 bytes), pointer-aligned.
        let parm_off = (w + 22).next_multiple_of(ps);
        // SAFETY: forwarded to the field writers' contract — `dst` backs the
        // widest (LP64, 40-byte) layout, so every field offset is in range.
        unsafe {
            write_abi_ptr(dst, 0, abi, self.pb_tree);
            write_abi_word(dst, w, self.pb_obj);
            write_abi_word(dst, w + 2, self.pb_prevstate);
            write_abi_word(dst, w + 4, self.pb_currstate);
            write_abi_word(dst, w + 6, self.pb_x);
            write_abi_word(dst, w + 8, self.pb_y);
            write_abi_word(dst, w + 10, self.pb_w);
            write_abi_word(dst, w + 12, self.pb_h);
            write_abi_word(dst, w + 14, self.pb_xc);
            write_abi_word(dst, w + 16, self.pb_yc);
            write_abi_word(dst, w + 18, self.pb_wc);
            write_abi_word(dst, w + 20, self.pb_hc);
            write_abi_ptr(dst, parm_off, abi, self.pb_parm);
        }
    }
}

/// `vro_cpyfm` logic op `S_AND_D` (`obdefs.h:53`) — the op `gr_colourblit`
/// blits a colour icon's device-form image with on the truecolor path
/// (`gemgraf.c:664-667`): the mask has been painted in the icon's background
/// pen first, and AND-ing the image over it keeps the image where the mask
/// is (white = all ones) and the screen where it is not (the image holds
/// white there, `expand_cicondata`'s masking + `palette[0]`).
const S_AND_D: u8 = 1;

/// 50 % dither used to grey-out DISABLED objects (EmuTOS `IP_4PATT`).
const STIPPLE_50: [u16; 2] = [0xAAAA, 0x5555];

// VDI default palette pens.
const WHITE: Pel = 0;
const BLACK: Pel = 1;
const GRAY_LT: Pel = 8; // EmuTOS LWHITE — the 3D object face
const GRAY_DK: Pel = 9; // EmuTOS LBLACK — the 3D shadow edge
// The remaining GEM pens (`obdefs.h:109-124`), named for the `xor_color`
// complement tables (`VDI4COMP`/`VDI16COMP`).
const RED: Pel = 2;
const GREEN: Pel = 3;
const BLUE: Pel = 4;
const CYAN: Pel = 5;
const YELLOW: Pel = 6;
const MAGENTA: Pel = 7;
const LWHITE: Pel = GRAY_LT;
const LBLACK: Pel = GRAY_DK;
const LRED: Pel = 10;
const LGREEN: Pel = 11;
const LBLUE: Pel = 12;
const LCYAN: Pel = 13;
const LYELLOW: Pel = 14;
const LMAGENTA: Pel = 15;

// AES font ids (`TEDINFO.te_font`, EmuTOS `obdefs.h:62-63`).
const FONT_IBM: i16 = 3;
const FONT_SMALL: i16 = 5;

/// The AES's two desktop font sizes. EmuTOS keeps both open at once
/// (`gsx_start`, `gemgraf.c:405-410`) and every text-drawing site names one:
/// `gsx_tblt(IBM, …)` / `gr_gtext(…, SMALL, …)`. The cell metrics are
/// EmuTOS's `gl_wchar`/`gl_hchar` and `gl_wschar`/`gl_hschar`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum AesFont {
    /// Normal desktop font (`IBM` = 3): `gl_wchar` × `gl_hchar` = 8×16.
    Ibm,
    /// Small desktop font (`SMALL` = 5): `gl_wschar` × `gl_hschar` = 6×6.
    /// Icon labels and icon overlay characters are drawn in this size, as
    /// is any `TEDINFO` whose `te_font` says so.
    Small,
}

impl AesFont {
    /// Map a `TEDINFO.te_font` code.
    ///
    /// EmuTOS's `ch_width`/`ch_height` (`gemgraf.c:456-477`) return 0 for any
    /// code that is neither `IBM` nor `SMALL`, and `gsx_tcalc` immediately
    /// divides by that height (`gemgraf.c:494`) — so EmuTOS has no defined
    /// third font here, it has a division by zero. Everything that is not
    /// `SMALL` is therefore read as `IBM` ([`FONT_IBM`]), the code every GEM
    /// resource editor writes for the normal font. Deliberately **not**
    /// asserted: an application resource may legally carry a GDOS font id we
    /// have no face for, and answering that with a kernel panic would be worse
    /// than EmuTOS's own undefined behaviour.
    #[inline]
    fn from_te_font(te_font: i16) -> AesFont {
        match te_font {
            FONT_SMALL => AesFont::Small,
            FONT_IBM => AesFont::Ibm,
            _ => AesFont::Ibm,
        }
    }

    /// `ch_width` (`gemgraf.c:456-465`) — the cell advance in pixels.
    #[inline]
    fn cell_w(self) -> i16 {
        match self {
            AesFont::Ibm => crate::vdi::font::CELL_W,
            AesFont::Small => crate::vdi::font::SMALL_CELL_W,
        }
    }

    /// `ch_height` (`gemgraf.c:468-477`) — the cell height in pixels.
    #[inline]
    fn cell_h(self) -> i16 {
        match self {
            AesFont::Ibm => crate::vdi::font::CELL_H,
            AesFont::Small => crate::vdi::font::SMALL_CELL_H,
        }
    }
}

// ── dispatch entry points ───────────────────────────────────────────

/// Map an AES `int_in` clip quadruple (`OB_XCLIP..OB_HCLIP`, `int_in[2..5]`:
/// `cx`/`cy`/`cw`/`ch`) to a VDI clip rectangle, honouring the EmuTOS
/// **zero-clip convention** (`gsx_sclip`, gemgraf.c:88-104): a zero-sized
/// rectangle (`cw <= 0 || ch <= 0`) means *no clip* — EmuTOS calls
/// `vs_clip(FALSE)`, i.e. clipping is turned OFF and everything is drawn
/// unclipped — so this returns `None` and the caller substitutes the full
/// screen. A non-empty rectangle becomes `(cx, cy)..(cx+cw-1, cy+ch-1)`,
/// exactly EmuTOS's `ptsin` computation.
///
/// This is the fix for AUD-P3-013: the previous inline `ClipRect { xmx: cx+cw-1
/// }` turned `cw == 0` into an empty `xmx < xmn` rectangle that drew *nothing*,
/// inverting the documented "w/h == 0 = draw everything" convention. Shared by
/// `objc_draw` (through `render_tree_t`) and `objc_change` so both build the AES
/// clip identically (D-052 documents `int_in[2..5]` as the clip).
fn clip_from_intin(cx: i16, cy: i16, cw: i16, ch: i16) -> Option<ClipRect> {
    if cw <= 0 || ch <= 0 {
        None
    } else {
        Some(ClipRect { xmn: cx, ymn: cy, xmx: cx + cw - 1, ymx: cy + ch - 1 })
    }
}

/// `objc_draw` (42) — draw the subtree rooted at `OB_DRAWOB`, `OB_DEPTH`
/// levels deep, clipped to `OB_XCLIP..OB_HCLIP`. `addr_in[0]` = tree.
pub fn objc_draw(pb: &Aespb) {
    let tree = pb.addrin(0) as *const Object;
    let drawob = pb.intin(0);
    let depth = pb.intin(1);

    let Some(h) = host() else {
        pb.set_ret(0);
        return;
    };
    let Some(backend) = h.raster() else {
        pb.set_ret(0);
        return;
    };
    if tree.is_null() || drawob < 0 {
        pb.set_ret(0);
        return;
    }

    // Clip to OB_XCLIP/YCLIP/WCLIP/HCLIP (int_in[2..5]); the backend
    // intersects with the screen edges. EmuTOS `objc_draw` sets exactly
    // this VDI clip before drawing, so a partial (damaged-rectangle)
    // redraw does not spill outside the caller's region.
    let (cx, cy, cw, ch) = (pb.intin(2), pb.intin(3), pb.intin(4), pb.intin(5));
    // SAFETY: the GEM caller guarantees a valid tree array.
    unsafe { render_tree(tree, drawob, depth, cx, cy, cw, ch, backend) };
    pb.set_ret(1);
}

/// Render object tree `tree` starting at object `start` for `depth` levels,
/// clipped to `(cx, cy, cw, ch)`, straight through `backend` — the body of
/// [`objc_draw`] **without** the AES trap, reading the tree at the **current**
/// ABI width ([`ObjTree::new`]).
///
/// In-kernel callers that must draw an object tree while already inside the
/// AES dispatch use this instead of a re-entrant `svc #2`. For a **kernel-built**
/// tree (the desktop's `g_screen`, the AES background fallback root) that is
/// always native `Object`s even while an ILP32 app is current, use
/// [`render_tree_lp64`] instead, which forces the 32-byte record layout.
///
/// # Safety
/// `tree` must be a valid `OBJECT` array with `start` in range.
#[allow(clippy::too_many_arguments)]
pub unsafe fn render_tree(
    tree: *const Object,
    start: i16,
    depth: i16,
    cx: i16,
    cy: i16,
    cw: i16,
    ch: i16,
    backend: &dyn RasterBackend,
) {
    if tree.is_null() || start < 0 {
        return;
    }
    render_tree_t(ObjTree::new(tree), start, depth, cx, cy, cw, ch, backend);
}

/// As [`render_tree`], but reads `tree` at the native LP64 (32-byte) record
/// layout regardless of the current execution context's ABI ([`ObjTree::lp64`]).
///
/// This is the non-trapping `w_drawdesk`/`objc_draw` primitive for the AES
/// screen-background composite ([`crate::aes::window`]): both the desktop's
/// registered `g_screen` tree (`WF_NEWDESK`) and the AES-internal fallback root
/// are kernel-built native `Object`s, so they must be read at LP64 width even
/// when an ILP32 app is the current process (exactly like the kernel-built
/// alert/fileselector trees use [`ObjTree::lp64`]).
///
/// # Safety
/// `tree` must be a valid native (LP64) `OBJECT` array with `start` in range.
#[allow(clippy::too_many_arguments)]
pub unsafe fn render_tree_lp64(
    tree: *const Object,
    start: i16,
    depth: i16,
    cx: i16,
    cy: i16,
    cw: i16,
    ch: i16,
    backend: &dyn RasterBackend,
) {
    if tree.is_null() || start < 0 {
        return;
    }
    render_tree_t(ObjTree::lp64(tree), start, depth, cx, cy, cw, ch, backend);
}

/// As [`render_tree_lp64`], but for an **app-provided** background tree an EL0
/// GEM app registered via `wind_set(DESKWH, WF_NEWDESK, tree)`: read the tree at
/// the `abi` captured at registration and treat it as **not** kernel-built
/// ([`ObjTree::app_at`]), so a `G_USERDEF`'s `ub_code` is an EL0 callback that
/// [`ob_user`] calls out to at EL0 instead of calling at EL1. This is the WF_NEWDESK-
/// background counterpart to the ILP32-app `objc_draw`-via-SVC path
/// ([`ObjTree::new`]): the same privilege guard, but with the ABI recorded at
/// registration because the composite runs when the app may not be current.
///
/// `owner` is the process slot the tree belongs to (`Some` for an EL0
/// registrant, `None` for a kernel one): every address the drawer follows is
/// then resolved through that slot's mapping ([`ObjTree::app_owned`], A97)
/// instead of being dereferenced through whichever `TTBR0` is installed; a
/// `G_USERDEF` routine runs in the current process with the owner's rights
/// for the callout (ARCH03 3c, E10).
///
/// # Safety
/// `tree` must be a valid `OBJECT` array (at `abi` width) with `start` in range
/// — in the owner's address space when `owner` is `Some`.
#[allow(clippy::too_many_arguments)]
pub unsafe fn render_tree_app(
    tree: *const Object,
    start: i16,
    depth: i16,
    cx: i16,
    cy: i16,
    cw: i16,
    ch: i16,
    abi: Abi,
    owner: Option<usize>,
    backend: &dyn RasterBackend,
) {
    if tree.is_null() || start < 0 {
        return;
    }
    render_tree_t(ObjTree::app_owned(tree, abi, owner), start, depth, cx, cy, cw, ch, backend);
}

/// Shared body of [`render_tree`]/[`render_tree_lp64`]: draw the subtree rooted
/// at `start`, `depth` levels deep, clipped to `(cx, cy, cw, ch)`, through the
/// caller-chosen [`ObjTree`] accessor (which fixes the ABI width).
#[allow(clippy::too_many_arguments)]
fn render_tree_t(
    t: ObjTree,
    start: i16,
    depth: i16,
    cx: i16,
    cy: i16,
    cw: i16,
    ch: i16,
    backend: &dyn RasterBackend,
) {
    // AUD-P3-013 zero-clip convention: `cw`/`ch == 0` means "no clip" = the full
    // screen, not an empty rectangle (which drew nothing). For a real damaged
    // rectangle this is byte-identical to the previous inline build.
    let clip = clip_from_intin(cx, cy, cw, ch)
        .unwrap_or_else(|| ClipRect::full(backend.width(), backend.height()));
    // Absolute origin of `start` = sum of its ancestors' x/y (like objc_offset).
    let (ox, oy) = abs_offset(t, start);
    hide_cursor_for_draw(backend);
    draw_tree(t, start, depth, ox - t.x(start), oy - t.y(start), clip, backend);
    show_cursor_after_draw(backend);
}

// ── the mouse bracket around an object draw (A65) ────────────────────────

/// Take the mouse cursor off the screen for the duration of an object draw —
/// EmuTOS `gsx_moff()` (`aes/gemgsxif.c:440-448`), which every object-drawing
/// entry point calls: `ob_draw` (`aes/gemoblib.c:974`) and `ob_change`
/// (`:1275`).
///
/// **Why it is not optional.** The cursor is a save-under sprite: `mouse::draw`
/// copies the 16×16 device pixels it covers into a backing store before painting
/// the arrow, and `mouse::erase` writes that copy straight back (EmuTOS
/// `cur_display`/`cur_replace`, `vdi/vdi_mouse.c:975-985,1099`). Drawing into
/// that box while the sprite is up does not invalidate the copy, so the **next**
/// cursor step restores the *pre-draw* pixels — a stale rectangle exactly 16
/// columns wide and up to 16 rows tall that stays on screen until something
/// repaints it. That is A65, photographed in the file selector on a real Pi 4.
///
/// **Why the bracket sits here and not deeper.** EmuTOS brackets in `ob_draw`
/// and `ob_change`, i.e. once per *entry point*, not inside the tree walk
/// (`everyobj`) or the per-object painter (`just_draw`), which are called with
/// the cursor already down. The functions carrying that role here are
/// [`render_tree_t`] (the body behind `objc_draw`/`render_tree*`),
/// [`draw_obj_t`], [`draw_obj_clip`], [`draw_root`] and [`change_clip_t`] (the
/// body behind `objc_change`) — and nowhere else. In particular
/// [`set_state_bit_t`] is **not** bracketed: it is a caller of [`change_t`], the
/// same relation EmuTOS' `menu_icheck` has to `ob_change` (`aes/gemmnlib.c:170`),
/// and would only add a redundant nesting level.
///
/// **Nesting is expected and carried.** EmuTOS nests these brackets itself —
/// `w_update` hides the cursor (`aes/gemwmlib.c:1011`) and then reaches `ob_draw`
/// through `w_cpwalk`/`do_walk` (`:1029,378,309`), which hides it again — because
/// `gl_moff` is a counting semaphore: only the 0→1 transition erases and only the
/// 1→0 transition redraws. [`crate::vdi::cursor_hide`]/[`crate::vdi::cursor_show`]
/// are that same counter (`HIDE_COUNT`, `vdi/mouse.rs`), so the brackets the Menu
/// Manager, the Form Manager, the Window Manager and the desktop already place
/// around these calls stay correct and cost nothing.
#[inline]
fn hide_cursor_for_draw(backend: &dyn RasterBackend) {
    crate::vdi::cursor_hide(backend);
}

/// Put the cursor back after an object draw — EmuTOS `gsx_mon()`
/// (`aes/gemgsxif.c:450-455`); see [`hide_cursor_for_draw`]. The redraw takes a
/// **fresh** save-under from the pixels just drawn, which is what makes the
/// following cursor step restore the new content instead of the old.
#[inline]
fn show_cursor_after_draw(backend: &dyn RasterBackend) {
    crate::vdi::cursor_show(backend);
}

/// `objc_offset` (44) — the object's absolute screen x/y (sum of parent
/// offsets). `int_in[0]` = object; `OB_XOFF`/`OB_YOFF` = `int_out[1..2]`.
pub fn objc_offset(pb: &Aespb) {
    let tree = pb.addrin(0) as *const Object;
    let obj = pb.intin(0);
    if tree.is_null() || obj < 0 {
        pb.set_ret(0);
        return;
    }
    let (x, y) = abs_offset(ObjTree::new(tree), obj);
    pb.set_intout(1, x);
    pb.set_intout(2, y);
    pb.set_ret(1);
}

/// `objc_add` (40) — link object `child` (`int_in[1]`) as the last child of
/// `parent` (`int_in[0]`). `addr_in[0]` = tree. EmuTOS `ob_add`.
///
/// **Answers `1` always** (A55): `crysbind` starts with `ret = TRUE`
/// (`aes/gemsuper.c:85`) and the `OBJC_ADD` arm never touches it
/// (`:204-206`; `ob_add` is `void`). `ob_add`'s own guard, `parent != NIL
/// && child != NIL` (`gemoblib.c:1127`), only decides whether the tree is
/// written — measured on the TT as `1` for `(NIL, NIL)`, `(NIL, ROOT)` and
/// `(ROOT, NIL)` (`aes-r4-subfunctions.tsv` cases 550–552). The guards
/// below are that guard widened to every negative index plus a null tree:
/// they refuse the write and change nothing else.
pub fn objc_add(pb: &Aespb) {
    let tree = pb.addrin(0) as *mut Object;
    let parent = pb.intin(0);
    let child = pb.intin(1);
    if !tree.is_null() && parent >= 0 && child >= 0 {
        ob_add(ObjTree::new(tree), parent, child);
    }
    pb.set_ret(1);
}

/// `objc_delete` (41) — unlink object `int_in[0]` from its parent's child
/// list (the ROOT cannot be deleted). `addr_in[0]` = tree. EmuTOS `ob_delete`.
pub fn objc_delete(pb: &Aespb) {
    let tree = pb.addrin(0) as *mut Object;
    let obj = pb.intin(0);
    if tree.is_null() || obj < 0 {
        pb.set_ret(0);
        return;
    }
    let ok = ob_delete(ObjTree::new(tree), obj);
    pb.set_ret(ok);
}

/// `objc_order` (45) — move object `int_in[0]` to sibling position
/// `int_in[1]` (0 = front/head, -1 = back/tail). `addr_in[0]` = tree.
/// EmuTOS `ob_order`.
pub fn objc_order(pb: &Aespb) {
    let tree = pb.addrin(0) as *mut Object;
    let obj = pb.intin(0);
    let newpos = pb.intin(1);
    if tree.is_null() || obj < 0 {
        pb.set_ret(0);
        return;
    }
    let ok = ob_order(ObjTree::new(tree), obj, newpos);
    pb.set_ret(if ok { 1 } else { 0 });
}

/// Link `child` as the last child of `parent` (EmuTOS `ob_add`).
pub(crate) fn ob_add(t: ObjTree, parent: i16, child: i16) {
    if parent == NIL || child == NIL {
        return;
    }
    t.set_next(child, parent); // last sibling → parent
    let lastkid = t.tail(parent);
    if lastkid == NIL {
        t.set_head(parent, child); // first child
    } else {
        t.set_next(lastkid, child); // append to the end
    }
    t.set_tail(parent, child);
}

/// Unlink `obj` from its parent's child list (EmuTOS `ob_delete`). Returns 1
/// on success, 0 for the ROOT or a not-found object.
fn ob_delete(t: ObjTree, obj: i16) -> i16 {
    if obj == 0 {
        return 0; // can't delete the ROOT
    }
    let mut nextsib = t.next(obj);
    let parent = parent_of(t, obj);
    if parent == NIL {
        return 0;
    }
    if t.head(parent) == obj {
        if t.tail(parent) == obj {
            nextsib = NIL; // the only child — clear head + tail
            t.set_tail(parent, NIL);
        }
        t.set_head(parent, nextsib);
    } else {
        let prev = get_prev(t, parent, obj);
        if prev == NIL {
            return 0; // not in this chain
        }
        t.set_next(prev, nextsib);
        if t.tail(parent) == obj {
            t.set_tail(parent, prev);
        }
    }
    1
}

/// Move `mov_obj` to sibling position `new_pos` (0 = head, NIL = tail) among
/// its siblings (EmuTOS `ob_order`, `aes/gemoblib.c:1201-1246`): unlink it
/// with [`ob_delete`], then splice it back in after the sibling `new_pos`
/// selects, and close the chain (`ob_tail`) only if its `ob_next` now points
/// at the parent (`:1242-1243`).
///
/// **The only child at `new_pos == 0` is left as EmuTOS leaves it** (A47):
/// `ob_delete` has set the parent's `ob_head` *and* `ob_tail` to `NIL`
/// (`:1164-1175`), the head branch copies that `NIL` `ob_head` into the
/// object's `ob_next` and re-hangs the object as head (`:1216-1220`), and
/// the tail test compares `NIL` with the parent and misses — so the result
/// is `parent.ob_head = mov_obj`, `parent.ob_tail = NIL`, `mov_obj.ob_next =
/// NIL`, `TRUE`. A chain that no longer closes on the parent, but it is the
/// reference's, measured over the dispatcher and derived line by line
/// (`docs/api/aes/object-form.md` § `objc_order`).
///
/// Two places where the reference has **no** result, because it indexes
/// `tree[-1]`, get a defined one instead: an only child at `new_pos != 0`
/// (`chg_obj == NIL` at `:1226`/`:1237`) is re-linked as the parent's sole
/// child, and a `new_pos` past the end of the sibling list (`:1229-1233`
/// walks through the parent and on) is clamped to the last sibling.
fn ob_order(t: ObjTree, mov_obj: i16, new_pos: i16) -> bool {
    if mov_obj == 0 {
        return false; // the ROOT has no siblings (`:1208-1209`)
    }
    let parent = parent_of(t, mov_obj);
    if parent == NIL {
        return false;
    }
    ob_delete(t, mov_obj);
    let mut chg_obj = t.head(parent); // NIL if `mov_obj` was the only child
    if new_pos == 0 {
        // `:1217-1221` verbatim, NIL included — see the doc comment.
        t.set_next(mov_obj, chg_obj);
        t.set_head(parent, mov_obj);
    } else {
        if new_pos == NIL {
            chg_obj = t.tail(parent);
        } else if chg_obj != NIL {
            let mut ii = 1;
            while ii < new_pos {
                let n = t.next(chg_obj);
                if n == parent || n == NIL {
                    break; // clamp to the end of the sibling list
                }
                chg_obj = n;
                ii += 1;
            }
        }
        if chg_obj == NIL {
            // The reference would splice after `tree[-1]` here; the parent is
            // empty, so the object becomes its sole child again.
            t.set_next(mov_obj, parent);
            t.set_head(parent, mov_obj);
            t.set_tail(parent, mov_obj);
            return true;
        }
        let cn = t.next(chg_obj);
        t.set_next(mov_obj, cn);
        t.set_next(chg_obj, mov_obj);
    }
    if t.next(mov_obj) == parent {
        t.set_tail(parent, mov_obj);
    }
    true
}

/// Previous sibling of `obj` under `parent`, or `NIL` (EmuTOS `get_prev`).
fn get_prev(t: ObjTree, parent: i16, obj: i16) -> i16 {
    let mut pobj = t.head(parent);
    if pobj == obj {
        return NIL;
    }
    loop {
        let nobj = t.next(pobj);
        if nobj == obj {
            return pobj;
        }
        if nobj == parent || nobj == NIL {
            return NIL;
        }
        pobj = nobj;
    }
}

/// `objc_change` (47) — set `obj`'s state to `OB_NEWSTATE` (`int_in[6]`)
/// and, if `OB_REDRAW` (`int_in[7]`) is set, redraw that object.
/// `addr_in[0]` = tree.
pub fn objc_change(pb: &Aespb) {
    let tree = pb.addrin(0) as *mut Object;
    let obj = pb.intin(0);
    let newstate = pb.intin(6) as u16;
    let redraw = pb.intin(7) != 0;
    if tree.is_null() || obj < 0 {
        pb.set_ret(0);
        return;
    }
    // OB_XCLIP..OB_HCLIP (int_in[2..5]): EmuTOS sets exactly this VDI clip
    // before `ob_change` (gemsuper.c:228-230, `gsx_sclip((GRECT*)&OB_XCLIP)`),
    // so a partial (damaged-rectangle) redraw stays inside the caller's region
    // instead of spilling full-screen. Zero w/h = no clip = full screen, per the
    // `gsx_sclip` convention (AUD-P3-003; D-052 documents int_in[2..5] as clip).
    let clip = clip_from_intin(pb.intin(2), pb.intin(3), pb.intin(4), pb.intin(5));
    change_clip_t(ObjTree::new(tree), obj, newstate, redraw, clip);
    pb.set_ret(1);
}

// ── objc_sysvar (3-D look) ──────────────────────────────────────────────
//
// The 3-D look system variables (EmuTOS `ob_sysvar`), with EmuTOS' defaults
// (`gemoblib.c`): activators move their text on press (`acttxtmove = 1`) but
// keep their colour; indicators change colour when selected
// (`indcolchange = 1`) but do not move text. `just_draw` honours these
// (`movetext`/`changecol`); a SET restyles the next draw.
static IND_TXTMOVE: AtomicI16 = AtomicI16::new(0);
static IND_COLCHANGE: AtomicI16 = AtomicI16::new(1);
pub(crate) static ACT_TXTMOVE: AtomicI16 = AtomicI16::new(1);
pub(crate) static ACT_COLCHANGE: AtomicI16 = AtomicI16::new(0);
static IND_BUTCOL: AtomicI16 = AtomicI16::new(GRAY_LT as i16);
pub(crate) static ACT_BUTCOL: AtomicI16 = AtomicI16::new(GRAY_LT as i16);
pub(crate) static BACKGR_COL: AtomicI16 = AtomicI16::new(GRAY_LT as i16);

/// `init_3d()` (`aes/gemoblib.c:1325-1343`, `#if CONF_WITH_3D_OBJECTS`) — the
/// once-per-AES-session reset of the seven 3-D look variables, called from
/// `run_accs_and_desktop` (`geminit.c:674-675`).
///
/// ```c
/// indtxtmove = 0; indcolchange = 1;
/// acttxtmove = 1; actcolchange = 0;
/// if (gl_ws.ws_ncolors <= LWHITE)
///     actbutcol = indbutcol = backgrcol = WHITE;
/// else
///     actbutcol = indbutcol = backgrcol = LWHITE;
/// ```
///
/// The statics above carry the `else` arm's values, which is what every
/// NewTOS profile computes (`ws_ncolors` is the workstation's colour count,
/// 256 on the RPi4 raster backend — `> LWHITE` = 8), so a cold boot and a
/// restarted session agree. The palette branch is reproduced anyway: it is
/// the function's contract, and `objc_sysvar(SET)` can have moved all three
/// colours in between.
pub(super) fn init_3d() {
    IND_TXTMOVE.store(0, Ordering::Relaxed);
    IND_COLCHANGE.store(1, Ordering::Relaxed);
    ACT_TXTMOVE.store(1, Ordering::Relaxed);
    ACT_COLCHANGE.store(0, Ordering::Relaxed);
    let ncolors = host()
        .and_then(|h| h.raster())
        .map(|r| r.num_colors() as i16)
        .unwrap_or(16);
    let col = if ncolors <= LWHITE as i16 { WHITE } else { LWHITE } as i16;
    IND_BUTCOL.store(col, Ordering::Relaxed);
    ACT_BUTCOL.store(col, Ordering::Relaxed);
    BACKGR_COL.store(col, Ordering::Relaxed);
}

/// `objc_sysvar` (48) — get (`int_in[0]` = 0) or set (1) a 3-D look variable
/// `int_in[1]` from values `int_in[2..3]`. GET returns the pair in
/// `int_out[1..2]`; `int_out[0]` = 1 (ok) / 0 (bad which or out-of-range
/// colour). EmuTOS `ob_sysvar`.
pub fn objc_sysvar(pb: &Aespb) {
    let (mode, which, in1, in2) = (pb.intin(0), pb.intin(1), pb.intin(2), pb.intin(3));
    let ncol = host()
        .and_then(|h| h.raster())
        .map(|r| r.num_colors() as i16)
        .unwrap_or(16);
    let ok = if mode != 0 {
        // SET (-1 in either value leaves that field unchanged).
        match which {
            LK3DIND => {
                if in1 != -1 {
                    IND_TXTMOVE.store(in1, Ordering::Relaxed);
                }
                if in2 != -1 {
                    IND_COLCHANGE.store(in2, Ordering::Relaxed);
                }
                true
            }
            LK3DACT => {
                if in1 != -1 {
                    ACT_TXTMOVE.store(in1, Ordering::Relaxed);
                }
                if in2 != -1 {
                    ACT_COLCHANGE.store(in2, Ordering::Relaxed);
                }
                true
            }
            INDBUTCOL if in1 < ncol => {
                IND_BUTCOL.store(in1, Ordering::Relaxed);
                true
            }
            ACTBUTCOL if in1 < ncol => {
                ACT_BUTCOL.store(in1, Ordering::Relaxed);
                true
            }
            BACKGRCOL if in1 < ncol => {
                BACKGR_COL.store(in1, Ordering::Relaxed);
                true
            }
            _ => false, // unknown which, or colour >= ws_ncolors
        }
    } else {
        // GET
        let (o1, o2, valid) = match which {
            LK3DIND => (IND_TXTMOVE.load(Ordering::Relaxed), IND_COLCHANGE.load(Ordering::Relaxed), true),
            LK3DACT => (ACT_TXTMOVE.load(Ordering::Relaxed), ACT_COLCHANGE.load(Ordering::Relaxed), true),
            INDBUTCOL => (IND_BUTCOL.load(Ordering::Relaxed), 0, true),
            ACTBUTCOL => (ACT_BUTCOL.load(Ordering::Relaxed), 0, true),
            BACKGRCOL => (BACKGR_COL.load(Ordering::Relaxed), 0, true),
            AD3DVALUE => (ADJ3DSTD, ADJ3DSTD, true),
            _ => (0, 0, false),
        };
        pb.set_intout(1, o1);
        pb.set_intout(2, o2);
        valid
    };
    pb.set_ret(if ok { 1 } else { 0 });
}

/// Set `obj`'s state and optionally redraw just that object. Shared by
/// `objc_change` and the Form Manager (to show a pressed button). Reads the
/// tree at the **current** ABI width (an app-provided tree); a kernel-built
/// tree uses [`change_t`] with an [`ObjTree::lp64`] accessor.
pub(crate) fn change(tree: *mut Object, obj: i16, newstate: u16, redraw: bool) {
    change_t(ObjTree::new(tree), obj, newstate, redraw);
}

/// Set `obj`'s state and optionally redraw it, driving the tree through the
/// caller-supplied [`ObjTree`] accessor (whose ABI width the caller chose:
/// `new` for an app tree, `lp64` for a kernel-built one). The redraw covers the
/// full screen — the behaviour every internal caller (menu/form highlight,
/// `graf_watchbox`) wants; `objc_change` uses [`change_clip_t`] directly to
/// honour its `int_in[2..5]` clip rectangle instead.
pub(crate) fn change_t(t: ObjTree, obj: i16, newstate: u16, redraw: bool) {
    change_clip_t(t, obj, newstate, redraw, None);
}

/// Body of [`change_t`]: set `obj`'s state and, if `redraw`, show the change
/// on screen within `clip_override` (`None` = the full screen, the internal
/// callers' behaviour; `objc_change` passes the AES `int_in[2..5]` clip).
///
/// EmuTOS `ob_change` (`gemoblib.c:1253-1320`, A99). After the state write the
/// draw is **not** always a redraw: when only the `SELECTED` bit toggles and
/// `xor_is_ok(obtype, flags, spec)` holds, the object rectangle — inset by a
/// positive `ob_sst` thickness — is complemented with one
/// `bb_fill(MD_XOR, FIS_SOLID, IP_SOLID, …)` and `just_draw` is skipped
/// (`redraw = !ok`, `:1295-1304`). That is what makes selecting *and*
/// deselecting an object without an opaque fill (an unboxed `G_TEXT`/`G_FTEXT`,
/// a `G_IBOX`, a `G_STRING` menu item, a `G_TITLE`) symmetric: the second XOR
/// takes the first one back. Redrawing instead would paint the ink over the
/// still-inverted ground and leave the bar standing (TeraDesk's SETMASK list,
/// the visible consequence of A85's trailing XOR in `just_draw`).
fn change_clip_t(t: ObjTree, obj: i16, newstate: u16, redraw: bool, clip_override: Option<ClipRect>) {
    let spec = t.spec(obj);
    // EmuTOS `ob_change` early-out (gemoblib.c:1263): `if ((curr_state ==
    // new_state) || (spec == -1L)) return;`. An unchanged state or the `-1`
    // sentinel leaves the object entirely untouched — no state write, no redraw:
    // the redraw path never dereferences `-1`, and re-asserting a state the
    // object already has never flickers. AUD-P3-003 adds the previously-missing
    // `curr_state == new_state` half (the `spec == -1` half was already present).
    let curr_state = t.state(obj);
    if curr_state == newstate || spec == t.spec_none() {
        return;
    }
    t.set_state(obj, newstate);
    if !redraw {
        return;
    }
    if let Some(backend) = host().and_then(|h| h.raster()) {
        // Zero-clip (`None`) = the full screen; otherwise the caller's rectangle.
        let clip = clip_override.unwrap_or_else(|| ClipRect::full(backend.width(), backend.height()));
        let (ox, oy) = abs_offset(t, obj);
        let (w, h) = (t.width(obj), t.height(obj));
        // `ob_sst` (gemobjop.c:25-70) gives the type (0x00ff-masked, `:40` —
        // AUD-P3-007), the flags and the signed border thickness; `ob_change`
        // clamps a negative (outside) thickness to 0 (`if (th < 0) th = 0;`,
        // `:1277-1278`) so the XOR covers the full rect for an outside border
        // and is inset by `th` for an inside one.
        let ob_type = t.ob_type(obj) & 0x00ff;
        let ob_flags = t.flags(obj);
        let th = t.sst_th(obj).max(0);
        // A65: the cursor comes down here and goes back up after the draw —
        // exactly where EmuTOS `ob_change` places `gsx_moff()` / `gsx_mon()`
        // (gemoblib.c:1275 / :1319): after `ob_offset`, around the whole type
        // switch, so the `G_USERDEF` arm is inside the bracket too. See
        // `hide_cursor_for_draw` for what the bracket buys and why it is at this
        // level. The `ob_user` arm therefore cannot `return` early any more —
        // the show has to run on both paths.
        hide_cursor_for_draw(backend);
        // The type switch (gemoblib.c:1280-1316); `redraw` is the C variable it
        // reassigns.
        let mut redraw = true;
        match ob_type {
            // AUD-P3-002: EmuTOS `ob_change` redraws a G_USERDEF by calling its
            // user routine with (prev=curr_state, curr=new_state) and suppresses
            // the generic object redraw (`redraw = FALSE`, gemoblib.c:1282-1284).
            // A kernel-built tree's callback is called directly, an app
            // tree's runs at EL0 in the current process (see `ob_user`). A null
            // `ob_spec` (no USERBLK) is left to the generic redraw below so
            // `ob_user` never dereferences null.
            G_USERDEF if spec != 0 => {
                ob_user(t, obj, ox, oy, w, h, spec, clip, curr_state, newstate);
                redraw = false;
            }
            // `case G_ICON: /* never use XOR for icons */ break;` (`:1286-1287`)
            // — a plain redraw; `gr_gicon` renders the selection itself. Only
            // G_ICON: a G_CICON takes the `default:` arm as in EmuTOS.
            G_ICON => {}
            // `default:` (`:1288-1304`): a change of the SELECTED bit is shown
            // by one XOR fill when `xor_is_ok` allows it (then no redraw), and
            // additionally for a G_IMAGE being *deselected* (D-093a: XOR back
            // the ground inversion `just_draw` applied, then redraw the image
            // normal — `redraw = !ok` stays TRUE there). An FL3DOBJ object is
            // never XOR'd (`xor_is_ok` `:114`): its press is the bevel, drawn
            // by the redraw. A toggle of any other state bit is a redraw.
            _ => {
                if (newstate ^ curr_state) & SELECTED != 0 {
                    let ok = xor_is_ok(t, ob_type, ob_flags, spec, backend);
                    if ok || (ob_type == G_IMAGE && newstate & SELECTED == 0) {
                        // `bb_fill(MD_XOR, FIS_SOLID, IP_SOLID, t.g_x+th,
                        // t.g_y+th, t.g_w-(2*th), t.g_h-(2*th))` (`:1301-1302`):
                        // the solid pattern in XOR mode complements every
                        // device pixel of the rect (`vdi/vdi_line.c:484-495`),
                        // the same primitive `just_draw`'s trailing SELECTED
                        // block uses (A85), so the two cancel exactly.
                        let (tw, tht) = (w - 2 * th, h - 2 * th);
                        if tw > 0 && tht > 0 {
                            let pat = PatternFill {
                                fg: BLACK,
                                bg: WHITE,
                                pattern: &IP_PATTERNS[IP_SOLID as usize],
                                mode: WriteMode::Xor,
                            };
                            backend.fill_pattern(ox + th, oy + th, tw, tht, pat, clip);
                        }
                        redraw = !ok;
                    }
                }
            }
        }
        if redraw {
            // `just_draw(tree, obj, t.g_x, t.g_y)` (`:1315-1316`): depth 0
            // redraws only this object (`draw_tree` carries `just_draw`'s own
            // HIDETREE / `spec == -1` reject, gemoblib.c:491).
            draw_tree(t, obj, 0, ox - t.x(obj), oy - t.y(obj), clip, backend);
        }
        show_cursor_after_draw(backend);
    }
}

/// `objc_edit` (46) — edit the EDITABLE `TEDINFO` text field `obj`.
/// `int_in[1]` = character, `int_in[2]` = cursor index, `int_in[3]` =
/// kind (EDSTART/EDINIT/EDCHAR/EDEND). The new cursor index is returned
/// in `OB_ODX` (`int_out[1]`).
///
/// Handles `EDINIT` (cursor to end), `EDCHAR` (validated insert / backspace /
/// delete / arrows, with over-type at the length limit and template
/// skip-ahead for formatted fields) and `EDEND`; `EDSTART` is a no-op.
pub fn objc_edit(pb: &Aespb) {
    let tree = pb.addrin(0) as *const Object;
    let obj = pb.intin(0);
    let kind = pb.intin(3);
    if kind == ED_START || tree.is_null() || obj <= 0 {
        // EmuTOS `ob_edit` (gemobed.c:342-343): `if ((kind == EDSTART) ||
        // (obj <= 0)) return TRUE;` — a pure no-op with no draw and no
        // TEDINFO access, returning TRUE with the passed-in index unchanged
        // (AUD-P3-008). EDSTART begins an edit sequence without touching the
        // field; `obj <= 0` (incl. the ROOT, 0) has no TEDINFO (its `ob_spec`
        // is a colour/thickness word), so editing it would deref a wild
        // pointer. `int_out[1]` = the caller's `int_in[2]`.
        pb.set_intout(1, pb.intin(2));
        pb.set_ret(1);
        return;
    }
    let idx = edit_field(tree, obj, pb.intin(1), pb.intin(2), kind);
    pb.set_intout(1, idx);
    pb.set_ret(1);
}

/// Edit the EDITABLE `TEDINFO` field `obj` in place and return the new
/// cursor index. Shared by `objc_edit` (the trap) and the Form Manager's
/// modal loop. `kind` = EDINIT (cursor to end) / EDCHAR (insert/delete at
/// the cursor) / EDEND (finish, no caret). Draws as EmuTOS `ob_edit` does:
/// the caret is an XOR line switched on by EDINIT, moved by EDCHAR (with a
/// redraw of the changed columns only) and switched off by EDEND.
///
/// Faithful to EmuTOS `ob_edit` (`gemobed.c`): the cursor index `idx` is a
/// **raw-string** index into `te_ptext` (data only, no template literals);
/// the on-screen caret column is `find_pos(te_ptmplt, idx)`, and each typed
/// character is validated against the `te_pvalid` code for its position.
/// For a plain field (no template / no validation — e.g. the desktop's
/// rename box) `find_pos` is the identity and validation accepts anything,
/// so this reduces to plain in-place editing.
pub(crate) fn edit_field(tree: *const Object, obj: i16, in_char: i16, idx: i16, kind: i16) -> i16 {
    edit_field_t(ObjTree::new(tree), obj, in_char, idx, kind)
}

/// Edit the EDITABLE `TEDINFO` field `obj` through the caller-supplied
/// [`ObjTree`] accessor (`new` for an app tree, `lp64` for a kernel-built
/// tree such as the fileselector's). Body of [`edit_field`].
pub(crate) fn edit_field_t(t: ObjTree, obj: i16, in_char: i16, mut idx: i16, kind: i16) -> i16 {
    let spec = t.spec(obj);
    if spec == 0 {
        return idx;
    }
    // The editable field's ob_spec points at a TEDINFO packed at the tree's ABI
    // width; read/write `te_ptext`/`te_ptmplt`/`te_pvalid` at that width so an
    // ILP32 app's edit fields (cursor/insert/backspace) work like LP64's.
    // SAFETY: `spec` is the valid TEDINFO base of an EDITABLE field (GEM contract).
    let ted = unsafe { tedinfo_read_abi(spec, t) };
    let ted = &ted;
    let txtlen = ted.te_txtlen.max(1);
    // The field's text buffer (`te_txtlen` bytes, the GEM contract), written
    // in place below: its kernel address through the owner-aware path
    // (ARCH03 3a, F2, [`ObjTree::wbuf`]) — never the raw user address. A
    // buffer the caller may not write ends the call (and the program); one
    // of another process that is not there is left alone.
    let Some(text) = t.wbuf(ted.te_ptext, txtlen as u64) else {
        return idx;
    };
    let text = text as *mut u8;
    let tmpl = read_cstr(t, ted.te_ptmplt);

    // The caret is EmuTOS `curfld`'s XOR line (`aes/gemobed.c:194-218`), so
    // `ob_edit` switches it off and on with the same call
    // (`gemobed.c:361-461`): `EDINIT` only puts it on (no field draw),
    // `EDCHAR` takes it off at the old place, redraws the changed columns if
    // the text changed and puts it on at the new place, `EDEND` only takes it
    // off. The caller keeps the calls paired, as `fm_do` does.
    //
    // A65: all draws of one call go under one cursor bracket. EmuTOS reaches
    // them through two primitives that each bracket for themselves — the
    // field via `ob_draw` (`gemobed.c:214`, hence `gemoblib.c:974/:976`) and
    // the caret via `gsx_cline` (`:216`), "draw a clipped polyline, hiding the
    // mouse as you do it" (`aes/gemgraf.c:186-192`). One bracket over the
    // call is the same guarantee without flickering the sprite between them.
    // See `hide_cursor_for_draw`.
    let backend = host().and_then(|h| h.raster());
    if let Some(b) = backend {
        hide_cursor_for_draw(b);
    }
    match kind {
        // `*idx = strlen(D.g_rawstr)` (`gemobed.c:363-365`).
        ED_INIT => idx = cstr_len_p(text) as i16,
        ED_CHAR => {
            // "find cursor & turn it off" (`gemobed.c:374-377`): `ob_stfn`
            // gives the template columns of the index and of the text end.
            let mut start = find_pos(tmpl, idx);
            let finish = find_pos(tmpl, cstr_len_p(text) as i16);
            if let Some(b) = backend {
                curfld(t, obj, ted, tmpl, start, 0, b);
            }
            let no_redraw = edit_char(t, ted, tmpl, text, txtlen, &mut idx, in_char, &mut start);
            // Redraw the changed span only (`gemobed.c:445-455`).
            if !no_redraw {
                let nstart = find_pos(tmpl, idx);
                let nfinish = find_pos(tmpl, cstr_len_p(text) as i16);
                let start = start.min(nstart);
                let dist = finish.max(nfinish) - start;
                if dist != 0 {
                    if let Some(b) = backend {
                        curfld(t, obj, ted, tmpl, start, dist, b);
                    }
                }
            }
        }
        // EDEND: no edit (`gemobed.c:455-456`), only the closing `curfld` below.
        _ => {}
    }
    // "draw/erase the cursor" (`gemobed.c:459-461`).
    if let Some(b) = backend {
        curfld(t, obj, ted, tmpl, find_pos(tmpl, idx), 0, b);
        show_cursor_after_draw(b);
    }
    idx
}

/// EmuTOS `curfld` (`aes/gemobed.c:194-218`) — with `dist != 0` redraw the
/// field `obj` clipped to the `dist` template columns from `new_pos`, else
/// complement the caret at template column `new_pos`:
///
/// ```c
/// pxl_rect(tree, obj, new_pos, &t);
/// if (dist)
///     t.g_w += (dist - 1) * gl_wchar;
/// else {
///     gsx_attr(FALSE, MD_XOR, BLACK);
///     t.g_y -= 3;
///     t.g_h += 6;
/// }
/// gsx_gclip(&oc); gsx_sclip(&t);
/// if (dist) ob_draw(tree, obj, 0);
/// else      gsx_cline(t.g_x, t.g_y, t.g_x, t.g_y+t.g_h-1);
/// gsx_sclip(&oc);
/// ```
///
/// The caret is one vertical line `gl_hchar + 6` pixels long from three rows
/// above the text cell, drawn as the AES draws every line — solid, one pixel
/// wide (`gemgraf.c:420-422`) — in XOR mode, so a second call at the same
/// place restores the pixels under it, the three rows above and below the
/// field's text included. The clip is the widened cell; the caller's clip is
/// untouched (each draw here takes its own).
fn curfld(t: ObjTree, obj: i16, ted: &TedInfo, tmpl: &[u8], new_pos: i16, dist: i16, backend: &dyn RasterBackend) {
    use crate::vdi::font::{CELL_H, CELL_W};
    let (ox, oy) = abs_offset(t, obj);
    let (x, y) = pxl_rect(ted, tmpl, ox, oy, t.width(obj), t.height(obj), new_pos);
    if dist != 0 {
        // `t.g_w = gl_wchar + (dist - 1) * gl_wchar`, `t.g_h = gl_hchar`.
        let w = CELL_W + (dist - 1) * CELL_W;
        let clip = ClipRect { xmn: x, ymn: y, xmx: x + w - 1, ymx: y + CELL_H - 1 };
        // depth 0 redraws only this field.
        draw_tree(t, obj, 0, ox - t.x(obj), oy - t.y(obj), clip, backend);
    } else {
        let (y0, h) = (y - 3, CELL_H + 6);
        let clip = ClipRect { xmn: x, ymn: y0, xmx: x + CELL_W - 1, ymx: y0 + h - 1 };
        backend.draw_line(x, y0, x, y0 + h - 1, BLACK, WriteMode::Xor, clip, 0xFFFF);
    }
}

/// EmuTOS `pxl_rect` (`aes/gemobed.c:180-190`) — the screen origin of the
/// caret cell at template column `ch_pos` of the field whose absolute box is
/// `(ox, oy, w, h)`:
///
/// ```c
/// ob_actxywh(tree, obj, &o);
/// gr_just(edblk.te_just, edblk.te_font, edblk.te_ptmplt, o.g_w, o.g_h, &o);
/// pt->g_x = o.g_x + (ch_pos * gl_wchar);
/// pt->g_y = o.g_y;
/// ```
///
/// `gr_just` (`aes/gemgraf.c:567-589`) measures the **template** in the
/// TEDINFO's font with `gsx_tcalc` (`:480-497`: width `min(w, n * ch_width)`,
/// height `min(h, ch_height)`), centres it vertically and justifies it
/// horizontally per `te_just` — each only when there is slack, the half-margin
/// rounded up — exactly as [`gr_gtext`] places the field's text. The column
/// step is always `gl_wchar`, the normal font's cell, as in EmuTOS.
fn pxl_rect(ted: &TedInfo, tmpl: &[u8], ox: i16, oy: i16, w: i16, h: i16, ch_pos: i16) -> (i16, i16) {
    let fnt = AesFont::from_te_font(ted.te_font);
    // gsx_tcalc on the template.
    let textw = w.min((tmpl.len() as i16).saturating_mul(fnt.cell_w()));
    let texth = h.min(fnt.cell_h());
    // gr_just.
    let mut x = ox;
    let mut y = oy;
    let slack_h = h - texth;
    if slack_h > 0 {
        y += (slack_h + 1) / 2;
    }
    let slack_w = w - textw;
    if slack_w > 0 {
        match ted.te_just {
            TE_CNTR => x += (slack_w + 1) / 2,
            TE_RIGHT => x += slack_w,
            // EmuTOS's `if (TE_CNTR) … else if (TE_RIGHT) …` leaves TE_LEFT
            // and any other value at the box origin.
            _ => {}
        }
    }
    (x + ch_pos * crate::vdi::font::CELL_W, y)
}

// Arrow scancodes in the key word's high byte (EmuTOS `scancode.h`).
const SC_LEFT: u8 = 0x4B;
const SC_RIGHT: u8 = 0x4D;

/// Apply one keystroke to the raw text buffer at the cursor (EmuTOS
/// `ob_edit` EDCHAR). Handles backspace / delete / escape / left-right
/// arrows and validated printable insertion.
///
/// Returns EmuTOS's `no_redraw` (`gemobed.c:373-443`): `true` when the text
/// did not change (arrows, a rejected character, nothing to delete), so no
/// span is redrawn. `start` is `ob_edit`'s `start` — the caret's template
/// column on entry, moved back one with the index when a full field is
/// over-typed (`gemobed.c:407-413`), the left end of the redrawn span.
#[allow(clippy::too_many_arguments)]
fn edit_char(
    t: ObjTree,
    ted: &TedInfo,
    tmpl: &[u8],
    text: *mut u8,
    txtlen: i16,
    idx: &mut i16,
    ch: i16,
    start: &mut i16,
) -> bool {
    let pvalid = read_cstr(t, ted.te_pvalid);
    let c = (ch & 0xff) as u8;
    let scan = ((ch >> 8) & 0xff) as u8;
    let len = cstr_len_p(text) as i16;

    // Cursor movement (scancode in the high byte).
    match scan {
        SC_LEFT => {
            if *idx > 0 {
                *idx -= 1;
            }
            return true;
        }
        SC_RIGHT => {
            if *idx < len {
                *idx += 1;
            }
            return true;
        }
        _ => {}
    }

    match c {
        0x08 => {
            // BACKSPACE: delete the char before the cursor.
            if *idx > 0 {
                *idx -= 1;
                // `ob_delit` (`gemobed.c:324-332`): FALSE (= redraw) when a
                // character was there to delete.
                if *idx < len {
                    delete_at(text, *idx, len);
                    return false;
                }
            }
            true
        }
        0x7F => {
            // DELETE: delete the char at the cursor.
            if *idx < len {
                delete_at(text, *idx, len);
                return false;
            }
            true
        }
        0x1B => {
            // ESCAPE: clear the whole field.
            *idx = 0;
            // SAFETY: `text` is a valid buffer of at least one byte.
            unsafe { *text = 0 };
            false
        }
        0x20..=0xFF => {
            // EmuTOS `ob_edit` EDCHAR `default` branch (gemobed.c:405-441):
            // validated printable insertion, over-typing the last position at
            // the length limit and, on a validation failure in a formatted
            // field, skipping the cursor ahead to the next data slot.
            //
            // `cur_pos` is the template column of the cursor (EmuTOS `start` =
            // `find_pos`), needed only by the skip-ahead scan.
            let mut cur_pos = find_pos(tmpl, *idx);
            let mut tmp_back = false;
            // Full field (`idx > te_txtlen-2`): back the cursor up one so the
            // insert over-types the last character rather than being dropped
            // (gemobed.c:407-413, AUD-P3-009).
            if *idx > txtlen - 2 {
                cur_pos -= 1;
                *start = cur_pos;
                tmp_back = true;
                *idx -= 1;
            }
            let mut ic = c;
            if check(&mut ic, valid_code(pvalid, *idx)) {
                // `ins_char` truncates at `te_txtlen` (gemobed.c:420), so a
                // full field drops its tail instead of overflowing.
                ins_char(text, *idx, ic, txtlen);
                *idx += 1;
                false
            } else {
                // Validation failed: try to skip ahead to the next template
                // data slot whose separator is the typed char, blanking the
                // skipped positions (gemobed.c:424-438, AUD-P3-010). For a
                // plain field (no validation) `check` always succeeds, so this
                // branch runs for formatted (`te_ptmplt`) fields only.
                if tmp_back {
                    *idx += 1;
                    cur_pos += 1;
                }
                let pos = scan_to_end(tmpl, cur_pos, *idx, c);
                if pos < txtlen - 2 {
                    fill_blanks(text, *idx, pos);
                    *idx = pos;
                    return false;
                }
                true
            }
        }
        _ => true,
    }
}

/// Delete the char at `pos` from the NUL-terminated buffer of length `len`
/// (shift the tail down, EmuTOS `ob_delit`/`strcpy`).
fn delete_at(text: *mut u8, pos: i16, len: i16) {
    // SAFETY: shifting within the NUL-terminated buffer (pos < len).
    unsafe {
        let mut i = pos;
        while i < len {
            *text.add(i as usize) = *text.add((i + 1) as usize);
            i += 1;
        }
    }
}

/// Insert `chr` at `pos` in the NUL-terminated buffer, truncating at
/// `tot_len` (the TEDINFO `te_txtlen`, buffer size including the NUL).
/// Faithful to EmuTOS `ins_char` (gemobed.c:117): shifts the tail up and
/// drops the overflow when the field is full, so the caller need not guard
/// the length.
fn ins_char(text: *mut u8, pos: i16, chr: u8, tot_len: i16) {
    // `ob_edit`'s "field full" branch backs the cursor up one before inserting
    // (gemobed.c:407-413), which reaches `pos == -1` for `te_txtlen == 1` — a
    // field with no room at all, exactly what `fix_tedinfo_std` yields for a
    // resource whose editable text is empty. EmuTOS edits a *scratch* copy
    // (`D.g_rawstr`, gemobed.c:349) and writes the field only afterwards with
    // `strcpy(edblk.te_ptext, D.g_rawstr)` (gemobed.c:438), so its `str[-1]`
    // store lands in the scratch and the field keeps its content. NewTOS edits
    // `te_ptext` in place, where the same index would write *before* the
    // resource buffer; clamping reproduces EmuTOS's observable result — the
    // character is dropped, the terminator rewritten — without leaving it.
    let pos = pos.max(0);
    let len = cstr_len_p(text) as i16;
    // SAFETY: `0 <= pos <= len <= tot_len-1`; every write index is
    // `<= tot_len-1`, so all accesses stay inside the caller's
    // `te_txtlen`-byte buffer.
    unsafe {
        let mut ii = len;
        while ii > pos {
            *text.add(ii as usize) = *text.add((ii - 1) as usize);
            ii -= 1;
        }
        *text.add(pos as usize) = chr;
        if len + 1 < tot_len {
            *text.add((len + 1) as usize) = 0;
        } else {
            *text.add((tot_len - 1) as usize) = 0;
        }
    }
}

/// Scan the template `tmpl` forward from column `start`, counting `_` data
/// slots into `idx`, until the typed separator `chr` is found (or the
/// template ends), returning the raw-string index of that data slot. Faithful
/// to EmuTOS `scan_to_end` (gemobed.c:141): lets a typed template separator
/// jump the cursor to the following data position.
fn scan_to_end(tmpl: &[u8], start: i16, mut idx: i16, chr: u8) -> i16 {
    let mut i = start.max(0) as usize;
    while i < tmpl.len() && tmpl[i] != chr {
        if tmpl[i] == b'_' {
            idx += 1;
        }
        i += 1;
    }
    idx
}

/// Blank out raw positions `[from, to)` and terminate at `to` (EmuTOS
/// `memset(&rawstr[idx], ' ', pos-idx); rawstr[pos] = '\0'`, gemobed.c:434-435).
fn fill_blanks(text: *mut u8, from: i16, to: i16) {
    // SAFETY: caller guarantees `from <= to < te_txtlen-2`, so writes stay in
    // the buffer.
    unsafe {
        let mut i = from;
        while i < to {
            *text.add(i as usize) = b' ';
            i += 1;
        }
        *text.add(to as usize) = 0;
    }
}

/// The `te_pvalid` validation code for raw position `idx` (EmuTOS expands
/// the string to `te_tmplen` by repeating the last code). An empty
/// validation string yields code 0 → "accept anything" (plain field).
fn valid_code(pvalid: &[u8], idx: i16) -> u8 {
    if pvalid.is_empty() {
        return 0;
    }
    let i = (idx as usize).min(pvalid.len() - 1);
    pvalid[i]
}

/// Map a raw-string index to its template column (EmuTOS `find_pos`): the
/// index of the `idx`-th `_` in `tmpl`, advanced to the next `_`. Degrades
/// to the identity when the template has no `_` left (plain field).
fn find_pos(tmpl: &[u8], mut pos: i16) -> i16 {
    let mut i = 0i16;
    while pos > 0 {
        if i as usize >= tmpl.len() {
            // Template exhausted: remaining positions map 1:1 past its end.
            return i + pos;
        }
        if tmpl[i as usize] == b'_' {
            pos -= 1;
        }
        i += 1;
    }
    // Skip to the first `_` at or after `i`.
    while (i as usize) < tmpl.len() && tmpl[i as usize] != b'_' {
        i += 1;
    }
    i
}

// TEDINFO validation character sets (EmuTOS `gemobed.c`; `..` denotes an
// inclusive range, parsed by `instr`). The derived sets drop the leading
// members of their parent (`A` = `N` without `0..9`, etc.).
const VALIDATE_N: &[u8] = b"0..9A..Z \x80\x8e\x8f\x90\x92\x99\x9a\x9e\xa5\xb5\xb6\xb7\xb8\xc2..\xdc";
const VALIDATE_LN: &[u8] = b"0..9a..zA..Z \x80..\xff"; // EmuTOS VALIDATE_n
const VALIDATE_F: &[u8] = b":?*a..zA..Z0..9_\x80..\xff";
const VALIDATE_P: &[u8] = b".?*a..zA..Z0..9_\\:\x80..\xff";

/// ASCII upcase (EmuTOS `toupper`, Roman range only).
fn upcase(c: u8) -> u8 {
    if c.is_ascii_lowercase() {
        c - 32
    } else {
        c
    }
}

/// Test whether `chr` is in the range/enum set `set` (EmuTOS `instr`):
/// each `x..z` is an inclusive range, every other byte a literal member.
fn instr(chr: u8, set: &[u8]) -> bool {
    let mut i = 0;
    while i < set.len() {
        let lo = set[i];
        i += 1;
        // A trailing `..z` after `lo` makes it the range `lo..=z`.
        let hi = if i + 1 < set.len() && set[i] == b'.' && set[i + 1] == b'.' {
            i += 2;
            let h = set.get(i).copied().unwrap_or(lo);
            i += 1;
            h
        } else {
            lo
        };
        if chr >= lo && chr <= hi {
            return true;
        }
    }
    false
}

/// Validate/upshift a character against a `te_pvalid` code (EmuTOS
/// `check`). Code 0 (no validation) and `X`/`x` accept anything; `x`
/// upshifts. Returns whether the character is allowed, upshifting
/// `*in_char` in place when the code requests it.
fn check(in_char: &mut u8, valchar: u8) -> bool {
    let (set, up): (&[u8], bool) = match valchar {
        0 => return true, // plain field: accept anything
        b'9' => (b"0..9", false),
        b'A' => (&VALIDATE_N[4..], true),
        b'N' => (VALIDATE_N, true),
        b'a' => (&VALIDATE_LN[4..], false),
        b'n' => (VALIDATE_LN, false),
        b'F' => (VALIDATE_F, true),
        b'f' => (&VALIDATE_F[3..], true),
        b'P' => (VALIDATE_P, true),
        b'p' => (&VALIDATE_P[3..], true),
        b'X' => return true,
        b'x' => {
            *in_char = upcase(*in_char);
            return true;
        }
        _ => return false,
    };
    if instr(*in_char, set) {
        if up {
            *in_char = upcase(*in_char);
        }
        true
    } else {
        false
    }
}

/// Length of a NUL-terminated string at `p` (cap 256).
fn cstr_len_p(p: *const u8) -> usize {
    if p.is_null() {
        return 0;
    }
    // SAFETY: GEM contract — valid NUL-terminated buffer.
    unsafe {
        let mut n = 0;
        while n < 256 && *p.add(n) != 0 {
            n += 1;
        }
        n
    }
}

/// `objc_find` (43) — hit-test: index of the deepest **visible** object
/// (`!HIDETREE`, inside its rect) under `(OB_MX, OB_MY)` within `OB_DEPTH`
/// levels of `OB_DRAWOB`, or `-1`. DISABLED objects **are** returned (the
/// disabled filter is in `form_do`, not here). `addr_in[0]` = tree.
pub fn objc_find(pb: &Aespb) {
    let tree = pb.addrin(0) as *const Object;
    let startob = pb.intin(0);
    let depth = pb.intin(1);
    let mx = pb.intin(2);
    let my = pb.intin(3);
    if tree.is_null() || startob < 0 {
        pb.set_ret(NIL);
        return;
    }
    let t = ObjTree::new(tree);
    let (ox, oy) = abs_offset(t, startob);
    let found = find_hit(t, startob, depth, ox - t.x(startob), oy - t.y(startob), mx, my);
    pb.set_ret(found);
}

/// Hit-test entry for internal callers (the Form Manager's modal loop):
/// deepest visible object under `(mx, my)` within `depth` levels of
/// `root`, or `NIL`. Mirrors `objc_find` — DISABLED objects are returned;
/// `form_do` applies the disabled filter on the result.
///
/// # Safety
/// `tree` is a valid OBJECT array.
pub(crate) unsafe fn hit_test(
    tree: *const Object,
    root: i16,
    depth: i16,
    mx: i16,
    my: i16,
) -> i16 {
    hit_test_t(ObjTree::new(tree), root, depth, mx, my)
}

/// Hit-test through the caller-supplied [`ObjTree`] accessor (`new` for an app
/// tree, `lp64` for a kernel-built one such as an alert or the fileselector).
/// Body of [`hit_test`].
pub(crate) fn hit_test_t(t: ObjTree, root: i16, depth: i16, mx: i16, my: i16) -> i16 {
    let (ox, oy) = abs_offset(t, root);
    find_hit(t, root, depth, ox - t.x(root), oy - t.y(root), mx, my)
}

/// Absolute screen rectangle `(x, y, w, h)` of `obj`, reading an app tree at
/// the current ABI width. A kernel-built tree uses [`abs_rect_t`].
pub(crate) fn abs_rect(tree: *const Object, obj: i16) -> (i16, i16, i16, i16) {
    abs_rect_t(ObjTree::new(tree), obj)
}

/// Absolute screen rectangle `(x, y, w, h)` of `obj` through the caller-supplied
/// [`ObjTree`] accessor. Body of [`abs_rect`].
pub(crate) fn abs_rect_t(t: ObjTree, obj: i16) -> (i16, i16, i16, i16) {
    let (x, y) = abs_offset(t, obj);
    (x, y, t.width(obj), t.height(obj))
}

/// First child of `parent` whose absolute rectangle contains `(mx, my)`,
/// scanning only direct children; `NIL` if none. Used for menu hit-tests.
pub(crate) fn child_at(tree: *const Object, parent: i16, mx: i16, my: i16) -> i16 {
    child_at_t(ObjTree::new(tree), parent, mx, my)
}

/// [`child_at`] through the caller-supplied [`ObjTree`] accessor — the Menu
/// Manager's tracker reads the installed bar through its owner's accessor
/// (`menu::mntree_accessor`, ARCH03 3a F3), not the current process's.
pub(crate) fn child_at_t(t: ObjTree, parent: i16, mx: i16, my: i16) -> i16 {
    let mut c = t.head(parent);
    while c != NIL && c != parent {
        let (x, y, w, h) = abs_rect_t(t, c);
        if mx >= x && mx < x + w && my >= y && my < y + h {
            return c;
        }
        c = t.next(c);
    }
    NIL
}

/// Set or clear one `ob_state` bit on `obj` and redraw it. Thin wrapper
/// the Menu Manager shares for title/item highlighting.
pub(crate) fn set_state_bit(tree: *mut Object, obj: i16, bit: u16, set: bool) {
    set_state_bit_t(ObjTree::new(tree), obj, bit, set);
}

/// Set or clear one `ob_state` bit on `obj` and redraw it, through the
/// caller-supplied [`ObjTree`] accessor (`lp64` for a kernel-built tree such as
/// the fileselector). Body of [`set_state_bit`].
///
/// A65: deliberately **not** cursor-bracketed here. This is a caller of
/// [`change_t`], not an `ob_*` primitive — the same relation EmuTOS'
/// `menu_icheck` has to `ob_change` (`aes/gemmnlib.c:170`), which likewise adds
/// no `gsx_moff` of its own. The bracket lives one level down in
/// [`change_clip_t`]; see [`hide_cursor_for_draw`].
pub(crate) fn set_state_bit_t(t: ObjTree, obj: i16, bit: u16, set: bool) {
    let st = t.state(obj);
    let new = if set { st | bit } else { st & !bit };
    change_t(t, obj, new, true);
}

/// Draw the subtree rooted at `obj` (any object), `depth` levels deep, at
/// its absolute screen position. Used by the Menu Manager to draw the bar.
pub(crate) fn draw_obj(tree: *const Object, obj: i16, depth: i16, backend: &dyn RasterBackend) {
    draw_obj_t(ObjTree::new(tree), obj, depth, backend);
}

/// Draw the subtree rooted at `obj` through the caller-supplied [`ObjTree`]
/// accessor (`lp64` for a kernel-built tree such as the fileselector). Body of
/// [`draw_obj`].
pub(crate) fn draw_obj_t(t: ObjTree, obj: i16, depth: i16, backend: &dyn RasterBackend) {
    let clip = ClipRect::full(backend.width(), backend.height());
    let (ox, oy) = abs_offset(t, obj);
    // A65: an `ob_draw(tree, obj, depth)` by another name, so it carries
    // `ob_draw`'s `gsx_moff`/`gsx_mon` bracket (EmuTOS gemoblib.c:974/:976).
    // See `hide_cursor_for_draw`.
    hide_cursor_for_draw(backend);
    draw_tree(t, obj, depth, ox - t.x(obj), oy - t.y(obj), clip, backend);
    show_cursor_after_draw(backend);
}

/// Draw the subtree rooted at `obj`, `depth` levels deep, confined to `clip`.
/// Like [`draw_obj`] but with a caller-supplied clip rectangle: the Menu Manager
/// passes a clip equal to the drop-down box's grown save/restore area so nothing
/// is painted outside the saved region — the invariant *saved-area ⊇ drawn-area*
/// that makes the on-close restore exact even for a box with a `th<0` outward
/// border or a child positioned past the box edge. EmuTOS's decoration-aware
/// menu path does the same: `popup_blit` sets `gsx_sclip(&t)` over exactly the
/// grown save rectangle before drawing (`gemmnext.c:154`).
pub(crate) fn draw_obj_clip(
    tree: *const Object,
    obj: i16,
    depth: i16,
    clip: ClipRect,
    backend: &dyn RasterBackend,
) {
    draw_obj_clip_t(ObjTree::new(tree), obj, depth, clip, backend);
}

/// [`draw_obj_clip`] through the caller-supplied [`ObjTree`] accessor (the
/// Menu Manager's drop-down and submenu boxes, read through the bar owner's
/// accessor — ARCH03 3a F3).
pub(crate) fn draw_obj_clip_t(t: ObjTree, obj: i16, depth: i16, clip: ClipRect, backend: &dyn RasterBackend) {
    let (ox, oy) = abs_offset(t, obj);
    // A65: same `ob_draw` role as `draw_obj_t`, same bracket. The Menu Manager
    // already hides the cursor around its save-blit + draw pair, so this nests —
    // which EmuTOS does too (`w_update` → `w_cpwalk` → `do_walk` → `ob_draw`,
    // gemwmlib.c:1011,1029,378,309); the counter carries it.
    hide_cursor_for_draw(backend);
    draw_tree(t, obj, depth, ox - t.x(obj), oy - t.y(obj), clip, backend);
    show_cursor_after_draw(backend);
}

/// Draw a whole tree (used by the Form Manager after building an alert and by
/// the fileselector). The caller supplies the [`ObjTree`] accessor and thereby
/// chooses the ABI width: kernel-built trees pass an [`ObjTree::lp64`] accessor
/// so a 32-byte record is read even while an ILP32 app is current.
pub(crate) fn draw_root(t: ObjTree, depth: i16, backend: &dyn RasterBackend) {
    let clip = ClipRect::full(backend.width(), backend.height());
    // Root is index 0; its ob_x/ob_y are absolute (no parent), so the parent
    // offset is 0.
    // A65: `ob_draw(tree, ROOT, depth)` — EmuTOS reaches exactly this from
    // `fm_alert` (gemfmalt.c:367), `sh_draw` (gemshlib.c:280,287) and the
    // fileselector (gemfslib.c:709), each relying on `ob_draw`'s own bracket.
    // See `hide_cursor_for_draw`.
    hide_cursor_for_draw(backend);
    draw_tree(t, 0, depth, 0, 0, clip, backend);
    show_cursor_after_draw(backend);
}

// ── tree helpers ────────────────────────────────────────────────────

/// Absolute screen origin of `obj` = sum of x/y from the root down. Walks
/// parents by scanning sibling chains (EmuTOS `ob_offset`).
fn abs_offset(t: ObjTree, obj: i16) -> (i16, i16) {
    let mut x = 0i16;
    let mut y = 0i16;
    let mut cur = obj;
    while cur != NIL {
        x += t.x(cur);
        y += t.y(cur);
        cur = parent_of(t, cur);
    }
    (x, y)
}

/// `obj`'s parent index, or `NIL` for the root (EmuTOS `get_par`). Exposed
/// for the Form Manager's radio-button sibling sweep.
///
/// # Safety
/// `tree` is a valid OBJECT array (GEM contract).
pub(crate) unsafe fn parent(tree: *const Object, obj: i16) -> i16 {
    parent_of(ObjTree::new(tree), obj)
}

/// [`parent`] through a tree accessor (EmuTOS `get_par`, `aes/gemobjop.c:
/// 145-161`) — for trees read at their owner's ABI.
pub(crate) fn parent_t(t: ObjTree, obj: i16) -> i16 {
    parent_of(t, obj)
}

/// Index of `obj`'s parent, or `NIL` for the root. The parent is the node
/// whose child chain reaches `obj` (TOS finds it via `ob_next` wrapping
/// back to the parent at the end of the sibling list).
fn parent_of(t: ObjTree, obj: i16) -> i16 {
    // Follow ob_next from obj until it points to a node that lists obj as
    // a child range end — TOS sets the last sibling's ob_next to the
    // parent index. Walk ob_next until we find a node whose ob_tail is `last`.
    let mut last = obj;
    loop {
        let n = t.next(last);
        if n == NIL {
            return NIL; // root
        }
        // If n's tail is `last`, then n is the parent.
        if t.tail(n) == last {
            return n;
        }
        last = n;
    }
}

/// Recursively draw `obj` and up to `depth` levels of descendants. `(px,
/// py)` is the absolute origin of `obj`'s parent.
fn draw_tree(
    t: ObjTree,
    obj: i16,
    depth: i16,
    px: i16,
    py: i16,
    clip: ClipRect,
    backend: &dyn RasterBackend,
) {
    if t.flags(obj) & HIDETREE != 0 {
        return;
    }
    let x = px + t.x(obj);
    let y = py + t.y(obj);
    // Skip drawing an object whose `ob_spec` is the `-1` "none" sentinel
    // (EmuTOS `just_draw`, gemoblib.c:491) so it is never dereferenced as a
    // pointer — but still recurse into its children: EmuTOS `everyobj` stops
    // the tree walk only on HIDETREE, not on `spec == -1`.
    if t.spec(obj) != t.spec_none() {
        just_draw(t, obj, x, y, clip, backend);
    }

    if depth > 0 {
        let mut child = t.head(obj);
        while child != NIL {
            draw_tree(t, child, depth - 1, x, y, clip, backend);
            if child == t.tail(obj) {
                break;
            }
            child = t.next(child);
        }
    }
}

/// Render a single object by type at absolute `(x, y)` (Batch-1 types).
// The 8 AES "inside patterns" IP_HOLLOW(0)..IP_SOLID(7) as 4-word dithers
// (EmuTOS `vdi/vdi_fill.c` DITHER; IP_4PATT = 50 %).
pub(crate) const IP_PATTERNS: [[u16; 4]; 8] = [
    [0x0000, 0x0000, 0x0000, 0x0000], // IP_HOLLOW
    [0x0000, 0x4444, 0x0000, 0x1111],
    [0x0000, 0x5555, 0x0000, 0x5555],
    [0x8888, 0x5555, 0x2222, 0x5555],
    [0xAAAA, 0x5555, 0xAAAA, 0x5555], // IP_4PATT (50 %)
    [0xAAAA, 0xDDDD, 0xAAAA, 0x7777],
    [0xEEEE, 0xFFFF, 0xBBBB, 0xFFFF],
    [0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF], // IP_SOLID
];

/// Decode a GEM object colour word (EmuTOS `gr_crack`): returns
/// `(border, text, pattern, inside, replace_mode)`.
pub(crate) fn gr_crack(color: u16) -> (Pel, Pel, u16, Pel, bool) {
    (
        ((color >> 12) & 0x0f) as Pel, // border colour
        ((color >> 8) & 0x0f) as Pel,  // text colour
        (color >> 4) & 0x07,           // inside pattern
        (color & 0x0f) as Pel,         // inside colour
        color & 0x80 != 0,             // 1 = REPLACE, 0 = TRANSPARENT
    )
}

/// EmuTOS `vdi4comp` (`gemoblib.c:60`): the complementary pen of each of the
/// four pens of a 2-plane screen — what an XOR of the pen's *pixel value*
/// yields, expressed as a pen.
const VDI4COMP: [Pel; 4] = [BLACK, WHITE, GREEN, RED];

/// EmuTOS `vdi16comp` (`gemoblib.c:83-86`): the complementary pen of each of
/// the sixteen pens of a 4-plane screen (pixel value XOR `1111`, mapped back
/// to a pen through the VDI's pen ↔ pixel table). LWHITE ↔ LBLACK, RED ↔
/// LCYAN, GREEN ↔ LMAGENTA, …
const VDI16COMP: [Pel; 16] = [
    BLACK, WHITE, LCYAN, LMAGENTA, LYELLOW, LRED, LBLUE, LGREEN, LBLACK, LWHITE, CYAN, MAGENTA, YELLOW, RED, BLUE,
    GREEN,
];

/// Complementary colour for a SELECTED object — EmuTOS `xor_color`
/// (`gemoblib.c:93-102`): WHITE for a pen beyond 15 or beyond the
/// workstation's colour count (`gl_ws.ws_ncolors`, here
/// [`RasterBackend::num_colors`]), the [`VDI4COMP`] entry on a ≤ 4-colour
/// screen, else the [`VDI16COMP`] entry. This is a *table*, not `pen ^ 1`:
/// `xor_color(RED) = LCYAN`, not GREEN (A85).
pub(crate) fn xor_color(backend: &dyn RasterBackend, color: Pel) -> Pel {
    let ncolors = backend.num_colors();
    if color >= 16 || color as u16 >= ncolors {
        return WHITE;
    }
    if ncolors <= 4 {
        return VDI4COMP[color as usize];
    }
    VDI16COMP[color as usize]
}

/// EmuTOS `xor_is_ok` (`gemoblib.c:107-148`): TRUE iff toggling `obj`'s
/// SELECTED state by an XOR of its rectangle gives the right picture, i.e.
/// the object is drawn in black and white only.
///
/// * `G_IBOX`/`G_STRING`/`G_TITLE` → TRUE (`:112-113`, they have no interior
///   colour of their own);
/// * `G_IMAGE` or any `FL3DOBJ` object → FALSE (`:115-116`);
/// * a workstation of ≤ 16 colours → TRUE (`:118-119`, the pen ↔ pixel table
///   makes an XOR a pen complement there);
/// * otherwise, for the TEDINFO family the `te_color` word and for
///   `G_BOX`/`G_BOXCHAR` the low word of `ob_spec` are checked: TRUE only
///   if both the interior colour (bits 0-3) and the text colour (bits 8-11)
///   are ≤ BLACK (`:125-147`); every other type (`G_BUTTON`, `G_USERDEF`,
///   `G_ICON`, `G_CICON`) takes the `default:` → TRUE (`:136-137`).
///
/// `just_draw` derives `changecol = !xor_is_ok(…)` from it for every object
/// that is not `FL3DOBJ` (`:521-524`); `ob_change` uses it to decide between
/// its XOR toggle and a redraw (`:1295-1304`).
fn xor_is_ok(t: ObjTree, ob_type: u16, ob_flags: u16, ob_spec: u64, backend: &dyn RasterBackend) -> bool {
    if ob_type == G_IBOX || ob_type == G_STRING || ob_type == G_TITLE {
        return true;
    }
    if ob_type == G_IMAGE || ob_flags & FL3DOBJ != 0 {
        return false;
    }
    if backend.num_colors() <= 16 {
        return true;
    }
    let colour_word: u16 = match ob_type {
        G_BOXTEXT | G_FBOXTEXT | G_TEXT | G_FTEXT => {
            if ob_spec == 0 {
                // No TEDINFO to read: the text arm draws nothing for this
                // object either (EmuTOS would dereference the null pointer).
                return true;
            }
            // SAFETY: the ob_spec of a *TEXT object is a TEDINFO packed at the
            // tree's ABI width (INDIRECT already resolved by `spec`).
            unsafe { tedinfo_read_abi(ob_spec, t).te_color as u16 }
        }
        G_BOX | G_BOXCHAR => ob_spec as u16,
        _ => return true, // G_BUTTON, G_USERDEF, G_ICON, G_CICON
    };
    // Interior colour, then text colour: both must be WHITE or BLACK.
    if (colour_word & 0x000f) as Pel > BLACK {
        return false;
    }
    if ((colour_word >> 8) & 0x000f) as Pel > BLACK {
        return false;
    }
    true
}

/// Fill `(x,y,w,h)` with the AES inside pattern `ipat` in colour `icol`
/// (EmuTOS `gr_rect`, `gemgraf.c:547-559`): SOLID a flat `icol` fill, the rest
/// an `icol`-on-white dither, and — crucially — HOLLOW an opaque **white** fill,
/// NOT a no-op.
///
/// All patterns (HOLLOW included) are drawn in **`WriteMode::Replace`** — EmuTOS
/// `gr_rect` always fills with `MD_REPLACE` (`bb_fill(MD_REPLACE, FIS_HOLLOW,
/// pattern=0x0000, …)`, `gemgraf.c:547-559`). For a hollow box that reaches
/// `vr_recfl`/the REPLACE fill (`vdi_line.c:528-548`) with the all-zero HOLLOW
/// pattern, `pattern & mask` is 0 for *every* pixel, so every pixel is written
/// `bgcol = palette[0] = WHITE` (`vdi_line.c:461`). A hollow G_BOX interior is
/// therefore filled OPAQUE WHITE, not left transparent — the fill colour is the
/// background (WHITE), INDEPENDENT of `icol`. A dither's *clear* bits are painted
/// the same white background. This is what makes a patterned box (the desktop
/// `DROOT` background, `IP_4PATT` green-on-white) or a hollow-white box (a
/// TeraDesk dropdown, `ob_spec=0x00ff1100` → IP_HOLLOW/WHITE) an opaque fill that
/// erases whatever it covers, rather than letting the underlying pixels show
/// through.
pub(crate) fn gr_rect(backend: &dyn RasterBackend, x: i16, y: i16, w: i16, h: i16, icol: Pel, ipat: u16, clip: ClipRect) {
    if w <= 0 || h <= 0 {
        return;
    }
    match ipat {
        // HOLLOW = REPLACE-mode fill with the all-zero pattern: every pixel is
        // painted the background colour WHITE (bgcol = palette[0]), NOT skipped.
        // The fill colour is the background WHITE, independent of `icol`.
        IP_HOLLOW => backend.fill_rect(x, y, w, h, WHITE, clip),
        IP_SOLID => backend.fill_rect(x, y, w, h, icol, clip),
        p => {
            let pat = PatternFill {
                fg: icol,
                bg: WHITE,
                pattern: &IP_PATTERNS[(p & 7) as usize],
                mode: WriteMode::Replace,
            };
            backend.fill_pattern(x, y, w, h, pat, clip);
        }
    }
}

/// Maximum formatted-string length (EmuTOS `obdefs.h` `MAX_LEN`).
const MAX_LEN: usize = 81;

/// Merge a raw text string into a template (EmuTOS `ob_format`): each `_`
/// in the template consumes the next character of `raw` (blanked back to
/// `_` once `raw` is exhausted); every other template character is copied
/// verbatim. A leading `@` in `raw` means "empty". `just == TE_RIGHT`
/// fills from the right end. The formatted result (always template length,
/// capped at `out.len()`) is written into `out`; the length is returned.
fn ob_format(just: i16, raw: &[u8], tmpl: &[u8], out: &mut [u8]) -> usize {
    // A leading '@' marks an empty raw string (EmuTOS clears it to "").
    let raw: &[u8] = if raw.first() == Some(&b'@') { &[] } else { raw };
    let ptlen = tmpl.len().min(out.len());
    let prlen = raw.len();

    if just == TE_RIGHT {
        // Walk template and raw from their right ends.
        let mut r = prlen; // one past the next raw char to consume
        for k in 0..ptlen {
            let ti = ptlen - 1 - k;
            out[ti] = if tmpl[ti] != b'_' {
                tmpl[ti]
            } else if r > 0 {
                r -= 1;
                raw[r]
            } else {
                b'_'
            };
        }
    } else {
        let mut r = 0usize; // next raw char to consume
        for i in 0..ptlen {
            out[i] = if tmpl[i] != b'_' {
                tmpl[i]
            } else if r < prlen {
                let c = raw[r];
                r += 1;
                c
            } else {
                b'_'
            };
        }
    }
    ptlen
}

/// EmuTOS `ob_user` (`gemoblib.c:433-448`): load the `USERBLK` from `spec`,
/// fill a [`ParmBlk`] (tree, object, prev/curr state, the object rect, the clip
/// rect, `ub_parm`) and invoke the user draw/change callback `ub_code(PARMBLK*)`,
/// returning its WORD result as the object's new state.
///
/// **Privilege split (AUD-P3-002).** For a **kernel-built** tree
/// ([`ObjTree::kernel_built`]) `ub_code` is an EL1 function at this AES code's
/// privilege and is called directly. For an **app-provided** tree the callback
/// is EL0 code: the PARMBLK goes into the current process's private-page scratch and
/// the AES fiber stalls while the kernel runs `ub_code` at EL0 in the
/// **current** process ([`crate::aes::thread::block_for_userdef`]) — also when
/// the tree belongs to another process (ARCH03 3c, E10 (a): EmuTOS runs it
/// wherever the draw happens, `gemoblib.c:411-426`; with memory protection on
/// the current space holds the owner's rights for the callout,
/// [`crate::bdos::process::memprot::grant_temp`]). Only with no EL0 process to
/// run it in (a kernel caller) is the state returned unchanged.
#[allow(clippy::too_many_arguments)]
fn ob_user(
    t: ObjTree,
    obj: i16,
    x: i16,
    y: i16,
    w: i16,
    h: i16,
    spec: u64,
    clip: ClipRect,
    prev_state: u16,
    new_state: u16,
) -> u16 {
    // SAFETY: `spec` is the ob_spec of a G_USERDEF object — a USERBLK packed at
    // the tree's ABI width (GEM contract).
    let ub = unsafe { UserBlk::read_abi(spec, t) };
    let pb = ParmBlk {
        pb_tree: t.tree_ptr(),
        pb_obj: obj,
        pb_prevstate: prev_state as i16,
        pb_currstate: new_state as i16,
        pb_x: x,
        pb_y: y,
        pb_w: w,
        pb_h: h,
        pb_xc: clip.xmn,
        pb_yc: clip.ymn,
        pb_wc: clip.xmx - clip.xmn + 1,
        pb_hc: clip.ymx - clip.ymn + 1,
        pb_parm: ub.ub_parm,
    };
    if ub.ub_code == 0 {
        // No USERBLK draw routine — nothing to call; keep the state unchanged.
        new_state
    } else if t.kernel_built() {
        // Same-privilege EL1 callback: `ub_code` is a native
        // `extern "C" fn(*const ParmBlk) -> WORD`. Go through a data pointer
        // (not an integer) so no int-to-fn transmute lint fires.
        let code = ub.ub_code as *const ();
        // SAFETY: kernel-built USERBLK — `ub_code` is a valid EL1 function
        // pointer of this exact signature (the kernel/desktop that built the
        // tree is responsible for that contract).
        let f: extern "C" fn(*const ParmBlk) -> i16 = unsafe { core::mem::transmute(code) };
        f(&pb) as u16
    } else {
        // App-provided tree: `ub_code` lives in EL0 app memory. Synchronous
        // `G_USERDEF` callout via the AES fiber (AES-as-thread design §3.4/§5
        // step 2). Write the PARMBLK into the process's reserved EL0 scratch —
        // through the kernel alias of its private read-write page (MEM12: the
        // routine may write its PARMBLK, as on EmuTOS's AES stack),
        // at the tree's ABI width — then stall the fiber: the kernel runs
        // `ub_code(parmblk)` at EL0 (no PXN) and resumes us with the WORD result
        // (EmuTOS `state = ob_user(...)`, gemoblib.c:793). No live process slot
        // (e.g. an EL1 test caller reaching an app tree) falls back to the old
        // "keep the state" behaviour rather than dereferencing a null basepage.
        use crate::bdos::process::loader::{PARMBLK_SCRATCH_VA, USER_VA_BASE};
        use crate::bdos::process::{current, memprot, mmu, slot_for};
        // ARCH03 3c, E10 (a): the routine runs in the **current** process,
        // whoever owns the tree — EmuTOS `call_usercode` is a plain `jsr` on
        // the current stack (`aes/gemoblib.c:411-426`, called from `ob_user`
        // `:433-448` and `just_draw` `:792-793`), so a `Pexec` child
        // compositing its parent's `WF_NEWDESK` background runs the parent's
        // `ub_code`. Since stage 1 a user address means the same in every
        // process (VA = PA), so the owner's `ub_code`, its tree and its
        // `ub_parm` are reachable from here. With memory protection on, the
        // current space is granted the owner's rights for the callout and
        // loses them afterwards (FreeMiNT `prot_temp`,
        // `sys/arch/mprot030.c:775-805`; `memprot::grant_temp`). This lifts the
        // A97 deferral, which existed only because before stage 1 the foreign
        // VA named nothing in the current process.
        let Some(slot) = slot_for(current()) else {
            // No EL0 process to run it in (kernel code such as the boot path
            // or the shell loop is the caller): there is no context for an EL0
            // callout.
            return new_state;
        };
        let Some(private) = mmu::priv_page_kva(slot) else {
            return new_state;
        };
        // The tree owner when it is another process; a dead owner's slot has
        // no image any more (and its tree would not have resolved).
        let foreign = match t.owner {
            Some(o) if o != slot => {
                if mmu::page0_kva(o).is_none() {
                    return new_state;
                }
                Some(o)
            }
            _ => None,
        };
        // SAFETY: `private + (PARMBLK_SCRATCH_VA - PRIV_PAGE_VA)` is the kernel
        // alias of the reserved scratch in the process's private page,
        // ≥ 40 bytes (loader asserts); single-CPU under the trap.
        unsafe { pb.write_abi(private as u64 + (PARMBLK_SCRATCH_VA - mmu::PRIV_PAGE_VA), t.abi()); }
        let parmblk_ptr = USER_VA_BASE + PARMBLK_SCRATCH_VA;
        if let Some(owner) = foreign {
            // SAFETY: AES trap context under the BDOS giant lock (single CPU).
            unsafe { memprot::grant_temp(slot, owner) };
        }
        let word = crate::aes::thread::block_for_userdef(slot, ub.ub_code, parmblk_ptr);
        if foreign.is_some() {
            // SAFETY: as above — the callout has returned (`Psigreturn`).
            unsafe { memprot::revoke_temp(slot) };
        }
        word as u16
    }
}

fn just_draw(t: ObjTree, obj: i16, x: i16, y: i16, clip: ClipRect, backend: &dyn RasterBackend) {
    // Snapshot the object's fields once at the caller's ABI width; a sub-struct
    // (`ob_spec`-pointed TEDINFO/ICONBLK/BITBLK) is read through its ABI-aware
    // reader (`*::read_abi`) at the same width in the type arms below.
    // `ob_state` is `mut`: a G_USERDEF's callback returns the object's new state
    // (EmuTOS `state = ob_user(...)`, gemoblib.c:793), which then drives the
    // trailing OUTLINED/SHADOWED/CROSSED/DISABLED decorations. Every other type
    // leaves it unchanged, so their rendering is byte-identical to before.
    let mut ob_state = t.state(obj);
    let ob_flags = t.flags(obj);
    // AUD-P3-007: dispatch on the **base** type only. EmuTOS `ob_sst`
    // (gemobjop.c:40) masks `*ptype = objptr->ob_type & 0x00ff` before every
    // type switch — the high byte carries extended-type/flags (e.g. the
    // submenu-slot the Menu Manager packs into a G_STRING item's `ob_type`), so
    // an unmasked compare would drop any such object into the `_ => {}` arm and
    // leave it undrawn. The masked value also drives the G_USERDEF check below.
    let ob_type = t.ob_type(obj) & 0x00ff;
    let ob_spec = t.spec(obj);
    let ob_width = t.width(obj);
    let ob_height = t.height(obj);
    let selected = ob_state & SELECTED != 0;
    let d3 = ob_flags & FL3DMASK;
    // D7: the grow and the add_3d_effect bevel key on FL3DOBJ (0x0200), the
    // "indicator OR activator" bit — set on FL3DIND/FL3DACT but CLEAR on
    // FL3DBAK (0x0400). EmuTOS `just_draw` grows only `if (flags & FL3DOBJ)`
    // (gemoblib.c:504) and bevels only `if (flags & FL3DOBJ)` (gemoblib.c:950).
    // An FL3DBAK background panel is therefore neither grown nor beveled; its
    // 3D optic comes from `draw_3d_outline` when OUTLINED (gemoblib.c:875-877).
    let d3obj = ob_flags & FL3DOBJ != 0;

    // EmuTOS `just_draw` grows an FL3DOBJ object by ADJ3DSTD (2px) on every side,
    // so its bevel sits in the margin *outside* the nominal rectangle instead of
    // eating the object's own edge (which shrank every 3D button by 4px and let
    // the bevel overlap the border/text). Grow here, then everything below —
    // fill, border, text, bevel, DISABLED dither — draws against the grown
    // rectangle. FL3DBAK is not an FL3DOBJ, so it keeps its nominal rect.
    let (x, y, w, h) = if d3obj {
        (x - ADJ3DSTD, y - ADJ3DSTD, ob_width + 2 * ADJ3DSTD, ob_height + 2 * ADJ3DSTD)
    } else {
        (x, y, ob_width, ob_height)
    };

    // SELECTED dynamics (EmuTOS `just_draw`, gemoblib.c:504-525, A85 / D-104):
    // for an FL3DOBJ object `movetext`/`changecol` are the `objc_sysvar`
    // variables — an activator nudges its text on press (`acttxtmove`) and
    // keeps its colour (`actcolchange` = 0), an indicator changes colour
    // (`indcolchange` = 1) and keeps its text still. For every other object
    // (no 3D flag **or FL3DBAK**) `movetext` is FALSE and `changecol =
    // !xor_is_ok(obtype, flags, spec)` (`:521-524`): an object drawn in black
    // and white is not colour-swapped in the type body at all — it is drawn
    // normally and the trailing block XOR-inverts its rectangle (`:932-943`);
    // only a coloured box/text spec on a > 16-colour workstation (or a
    // G_IMAGE) takes the in-body swaps (`:681-690`, `:699-706`, `:807-808`).
    let (movetext, changecol) = if d3obj {
        if d3 == FL3DACT {
            (ACT_TXTMOVE.load(Ordering::Relaxed) != 0, ACT_COLCHANGE.load(Ordering::Relaxed) != 0)
        } else {
            (IND_TXTMOVE.load(Ordering::Relaxed) != 0, IND_COLCHANGE.load(Ordering::Relaxed) != 0)
        }
    } else {
        (false, !xor_is_ok(t, ob_type, ob_flags, ob_spec, backend))
    };

    // Border colour + signed thickness captured from the type body for the
    // SHADOWED decoration below (EmuTOS `just_draw` reuses `bcol`/`th` from the
    // box/text body in the trailing `if (state)` block, gemoblib.c:894-901).
    let mut deco_bcol: Pel = BLACK;
    let mut deco_th: i16 = 0;
    // The bevel's inputs (EmuTOS `just_draw`): `effect_th` is `ob_sst`'s `th`
    // saved *before* the type body may zero it (`effect_th = th`,
    // gemoblib.c:498 — a promoted G_TEXT still shrinks its bevel by the
    // TEDINFO thickness), `effect_icol` the interior colour as the body left
    // it, after any SELECTED xor (`add_3d_effect(…, effect_th, icol)`, :951).
    // For a type without an interior (string, image, icon, userdef) EmuTOS
    // passes whatever `icol` holds; WHITE here means "no fill to collide with".
    let mut effect_th: i16 = 0;
    let mut effect_icol: Pel = WHITE;

    match ob_type {
        G_BOX | G_IBOX | G_BUTTON | G_BOXCHAR => {
            // Crack the colour word: box types pack it in `ob_spec`'s low 16
            // bits (EmuTOS `gr_crack`); a G_BUTTON's `ob_spec` is a text
            // pointer, so it takes the standard black-border / white-interior
            // button colours. Border thickness is in `ob_spec` bits 16..23 for
            // the box types; a button uses 1px.
            let (bcol, tcol, mut ipat, mut icol, _mode) = if ob_type == G_BUTTON {
                (BLACK, BLACK, IP_HOLLOW, WHITE, false)
            } else {
                gr_crack(ob_spec as u16)
            };
            // D4: a G_BUTTON's border is derived from its role and drawn
            // OUTSIDE its rect (negative th), EmuTOS `ob_sst`
            // (gemobjop.c:59-65): base -1, one thinner for EXIT, one thinner
            // for DEFAULT — so a default exit button is -3 and reads as the
            // heavy "this is Enter" outline. A box type takes its spec-byte
            // thickness (`th = *(((char *)pspec)+1)`, gemobjop.c:54-58).
            //
            // A67: the thickness is the same for every 3D kind. EmuTOS draws
            // the border whenever `th != 0` *regardless of 3D*
            // (gemoblib.c:663-667) and lays the bevel over it afterwards
            // (`add_3d_effect`, gemoblib.c:950-951), so an FL3DIND/FL3DACT box
            // — the file selector's drive letters, arrows and elevator, all
            // `th = -1` (gem_rsc.c:214,376,394,400,412) — keeps its black
            // ring AND gets the bevel, exactly like a 3D G_BUTTON. The former
            // `th = 0` for FL3DOBJ boxes was an invention of the first 3D look
            // (fb82794, "the flat black border is suppressed for 3D objects"),
            // measured against the rscview reference as the loss of the
            // drive boxes' 1-pixel frame (`TASK/A67.md`).
            let th = if ob_type == G_BUTTON {
                let mut bt: i16 = -1;
                if ob_flags & EXIT != 0 {
                    bt -= 1;
                }
                if ob_flags & DEFAULT != 0 {
                    bt -= 1;
                }
                bt
            } else {
                ((ob_spec >> 16) as u8 as i8) as i16
            };
            effect_th = th;
            // 3D colour override (EmuTOS): a hollow-white box/button takes a
            // solid 3D face colour so the bevel reads against it. Two rules,
            // one per type family (A70, D-103):
            // - G_BOX/G_BOXCHAR (not G_IBOX): `switch (flags & FL3DMASK)` —
            //   FL3DIND → indbutcol, FL3DACT → actbutcol, FL3DBAK → backgrcol
            //   (`gemoblib.c:619-636`).
            // - G_BUTTON: `if (flags & FL3DOBJ)` → actbutcol/indbutcol, **else**
            //   hollow white — FL3DBAK (0x0400) does not carry the FL3DOBJ bit
            //   (0x0200), so a FL3DBAK button stays `IP_HOLLOW`/`WHITE` like a
            //   non-3D one (`gemoblib.c:639-658`).
            let takes_3d_face = if ob_type == G_BUTTON { d3obj } else { ob_type != G_IBOX && d3 != 0 };
            if takes_3d_face && ipat == IP_HOLLOW && icol == WHITE {
                ipat = IP_SOLID;
                icol = match d3 {
                    FL3DACT => ACT_BUTCOL.load(Ordering::Relaxed) as Pel,
                    FL3DIND => IND_BUTCOL.load(Ordering::Relaxed) as Pel,
                    _ => BACKGR_COL.load(Ordering::Relaxed) as Pel, // FL3DBAK, box types only
                };
            }
            // SELECTED, but only when the object changes colour (EmuTOS
            // `changecol`, gemoblib.c:681-690): a hollow box goes solid black,
            // else the interior takes its complement. A 3D activator
            // (actcolchange = 0) keeps its face — the bevel alone shows the
            // press; a black-and-white non-FL3DOBJ box (changecol FALSE) is
            // drawn as normal and XOR-inverted by the trailing block instead.
            if selected && changecol {
                if ipat == IP_HOLLOW {
                    ipat = IP_SOLID;
                    icol = BLACK;
                } else {
                    icol = xor_color(backend, icol);
                }
            }
            // D-089 signed thickness: `th > 0` draws the border INSIDE the rect
            // (the interior fill is inset by `th`); `th < 0` draws it OUTSIDE
            // (the fill keeps the full rect, the border is an add-on around it);
            // `th == 0` = no border. `inset` is EmuTOS `tmpth = (th<0)?0:th`.
            let inset = th.max(0);
            if th != 0 {
                draw_signed_border(backend, x, y, w, h, th, bcol, clip);
            }
            if ob_type != G_IBOX {
                gr_rect(backend, x + inset, y + inset, w - 2 * inset, h - 2 * inset, icol, ipat, clip);
            }
            deco_bcol = bcol;
            deco_th = th;
            effect_icol = icol;
            // Text colour, complemented when selected *and* the object changes
            // colour (EmuTOS gemoblib.c:699-706; the `tmode = MD_TRANS` half
            // has no counterpart — `gsx_tblt` here always writes transparently).
            let ink = if selected && changecol { xor_color(backend, tcol) } else { tcol };
            if ob_type == G_BOXCHAR {
                // A G_BOXCHAR is a TEDINFO-family draw: EmuTOS copies `t`,
                // insets it by `tmpth`, applies the 3D `movetext` nudge and
                // calls `gr_gtext(TE_CNTR, IBM, …)` with a one-character string
                // (gemoblib.c:719-757 — `edblk.te_font = IBM` is forced at :729).
                let ch = ((ob_spec >> 24) & 0xFF) as u8;
                if ch != 0 {
                    let (cx, cy) = ted_movetext(movetext, selected, x + inset, y + inset);
                    gr_gtext(
                        backend,
                        TE_CNTR,
                        AesFont::Ibm,
                        &[ch],
                        cx,
                        cy,
                        w - 2 * inset,
                        h - 2 * inset,
                        ink,
                        clip,
                    );
                }
            } else if ob_type == G_BUTTON {
                // A G_BUTTON's label is drawn by the shared string block
                // (gemoblib.c:801-846), **not** by `gr_gtext`: no clamping to
                // the box, no whole-character truncation, plain `n/2` centring
                // on the *uninset* rect, and its own `movetext` rule
                // (`tmpx--; tmpy--` while raised, gemoblib.c:833-836).
                //
                // The label is BLACK (`tcol = BLACK`, :805) — WHITE only for a
                // SELECTED button that changes colour (`:807-808`, an FL3DIND
                // one by default); a plain or FL3DBAK button (`xor_is_ok`
                // `default:` → TRUE, changecol FALSE) keeps the black label on
                // its hollow-white face and the trailing XOR inverts both.
                let s = read_cstr(t, ob_spec);
                let len = s.len() as i16;
                let label = if selected && changecol { WHITE } else { BLACK };
                let mut tmpx = x + (w - len * crate::vdi::font::CELL_W) / 2;
                let mut tmpy = y + (h - crate::vdi::font::CELL_H) / 2;
                if movetext && !selected {
                    tmpx -= 1;
                    tmpy -= 1;
                }
                gsx_tblt(backend, AesFont::Ibm, tmpx, tmpy, s, label, clip);
            }
        }
        G_STRING => {
            // Menu items / plain strings: drawn through the shared string
            // block in BLACK (`tcol = BLACK`, gemoblib.c:805), never filled.
            // The SELECTED (menu highlight) look is the trailing XOR of the
            // whole rect (`:932-943`; `xor_is_ok(G_STRING)` is TRUE, `:112`,
            // so `changecol` is FALSE and, a G_STRING having `th = 0`, no
            // inset) over whatever the string was drawn on — a black block
            // with white glyphs on the white drop-down. CHECKED draws a check
            // glyph over the leading blank, before the XOR as in EmuTOS's
            // state block (`:903-908` precede `:932`).
            let ink = BLACK;
            if ob_state & CHECKED != 0 {
                // EmuTOS draws the CHECKED mark at `t.g_x+2, t.g_y` — 2
                // *pixels* to the right, no vertical centring — over the
                // string's leading blank, in the IBM font
                // (`gsx_tblt(IBM, t.g_x+2, t.g_y, 1)`, gemoblib.c:903-908).
                gsx_tblt(backend, AesFont::Ibm, x + 2, y, &[0x08], ink, clip);
            }
            let s = read_cstr(t, ob_spec);
            if ob_state & DISABLED != 0 && s.len() > 3 && is_dashes(s) {
                // A46 — "nicelines" (EmuTOS `CONF_WITH_NICELINES`, default 1,
                // config.h:1312-1314): a DISABLED string of more than three
                // dashes is a menu separator, and EmuTOS draws it as a
                // one-pixel line across the whole *object* width on the row
                // `t.g_y + t.g_h/2` **instead of** the text
                // (`gsx_cline(t.g_x, t.g_y+t.g_h/2, t.g_x+t.g_w-1, t.g_y+t.g_h/2)`,
                // gemoblib.c:838-841). `gsx_cline` (gemgraf.c:188-195) is a
                // plain `v_pline` of two points: solid (`vsl_type 7` /
                // `vsl_udsty 0xffff`, gemgraf.c:420-422), one pixel wide, in
                // the MD_TRANS mode the string block set just before
                // (`gsx_attr(TRUE, MD_TRANS, tcol)`, :816). The line colour is
                // the AES's sticky VDI line colour (`gl_lcolor`, gemgraf.c:75),
                // which the string block does not set — in the menu-drawing
                // sequence it is BLACK, left by the drop-down G_BOX's border
                // (`gsx_attr(FALSE, MD_REPLACE, bcol)`, :663) — so the line
                // takes the string's ink here. The trailing DISABLED dither
                // below lays its 50 % white over the line as it would over
                // the dashes (gemoblib.c:917-929).
                //
                // `len > 3` is the pinned source's gate (`4b0d9518`,
                // 2025-08-20); EmuTOS 1.4 draws one to three dashes as a line
                // too — a version delta, see `docs/api/aes/object-form.md`.
                let ly = y + h / 2;
                backend.draw_line(x, ly, x + w - 1, ly, ink, WriteMode::Transparent, clip, 0xFFFF);
            } else {
                // EmuTOS draws a G_STRING through the shared string block at
                // `tmpx = t.g_x`, `tmpy = t.g_y + ((t.g_h-gl_hchar)/2)` in the IBM
                // font, with no clamping and no truncation (gemoblib.c:817-845) —
                // `gr_gtext`'s clamped/rounded path is *not* used for strings. A
                // menu item's check column comes from the leading blanks built into
                // its resource string (GEM uses two leading spaces), not a
                // code-added indent.
                let tmpy = y + (h - crate::vdi::font::CELL_H) / 2;
                gsx_tblt(backend, AesFont::Ibm, x, tmpy, s, ink, clip);
            }
        }
        G_TITLE => {
            // EmuTOS draws a G_TITLE through the shared string block at `t.g_x`
            // with no indent (gemoblib.c:801-846), in BLACK (`:805`) and with
            // no box or fill; the menu-dropped highlight is the trailing XOR
            // (gemoblib.c:932-943, `xor_is_ok(G_TITLE) == TRUE`, `:112`) of
            // the rect **inset by one pixel**: `ob_sst` gives a G_TITLE
            // `th = 1` (gemobjop.c:44-46) and the state block insets `t` by a
            // positive `th` before the XOR (`:889-890`), so the inverted block
            // is `(x+1, y+1, w-2, h-2)` — exactly `ob_change`'s XOR toggle
            // (`:1301-1302`). `deco_th` carries that `th` to the trailing
            // block (and to SHADOWED, which EmuTOS sizes by the same `th`).
            deco_th = 1;
            // No face fill in either state — the bar's G_BOX parent is the
            // ground. A title's highlight comes and goes by `ob_change`'s XOR
            // toggle (`:1295-1304`, `change_clip_t`, A99), so the unselected
            // redraw never has to clear an inverted block itself (the pre-A99
            // face fill that did so is gone with it, together with the
            // separator repaint it forced on `aes/menu.rs`).
            let s = read_cstr(t, ob_spec);
            // Same string block as G_STRING: IBM font, `t.g_x`, vertically
            // centred on `gl_hchar` (gemoblib.c:817-845).
            let tmpy = y + (h - crate::vdi::font::CELL_H) / 2;
            gsx_tblt(backend, AesFont::Ibm, x, tmpy, s, BLACK, clip);
        }
        G_TEXT | G_BOXTEXT | G_FTEXT | G_FBOXTEXT => {
            // The TEDINFO interior is read at the tree's ABI width, so an ILP32
            // app's text/box draws too; for an LP64 tree the reader reproduces
            // the native offsets byte-for-byte (rendering unchanged).
            if ob_spec != 0 {
                // SAFETY: ob_spec of a *TEXT object points at a TEDINFO packed
                // at the tree's ABI width.
                let ted = unsafe { tedinfo_read_abi(ob_spec, t) };
                let ted = &ted;
                // Crack the TEDINFO colour word (EmuTOS `gr_crack`): border,
                // text, inside pattern, inside colour.
                let (bcol, tcol, mut ipat, mut icol, mode_replace) = gr_crack(ted.te_color as u16);
                let mut is_box = ob_type == G_BOXTEXT || ob_type == G_FBOXTEXT;
                // The TEDINFO thickness, for every 3D kind alike (A67, text
                // half): EmuTOS `just_draw` never zeroes a thickness for 3D —
                // `gemoblib.c:663-667` draws the border whenever `th != 0`,
                // and the bevel goes over it (`:950-951`). The only place a
                // text thickness is zeroed is the G_TEXT→G_BOXTEXT promotion
                // below (`tmpth = th = 0`, `:589-598`). Measured against the
                // rscview reference (`docs/analysis/fsel-reference-2026-09-17/`):
                // the file box's FTITLE caption is a G_BOXTEXT with
                // te_thickness -1 that EmuTOS frames in black under FL3DBAK
                // (`gemfslib.c:78`).
                //
                // A68: `th` comes from `ob_sst` (`gemoblib.c:489`), whose one
                // shared fold `if (th > 128) th -= 256` (`gemobjop.c:68-69`)
                // makes a `te_thickness` of 255 — the raw `.rsc` word for -1,
                // passed through unchanged by `tools/mkrsc.py` — a 1-px
                // *outside* border. `sst_th` is that fold's single home; the
                // raw `ted.te_thickness` is never read here.
                let mut th = t.sst_th(obj);
                effect_th = th;
                // 3D background (EmuTOS `:582-599`): a hollow-white text
                // background takes the solid 3D background colour *for any
                // 3D flag and any writing mode* (`:582-585`); the promotion of
                // a plain TEXT/FTEXT to BOXTEXT/FBOXTEXT with a zero border —
                // so the fill is drawn — happens only when the text mode is
                // MD_REPLACE (`if (tmode == MD_REPLACE)`, `:586-599`, A69). A
                // TRANSPARENT G_TEXT therefore stays an unboxed text (no
                // border, no fill; the second type switch has no `case
                // G_TEXT`, `:609-696`) and the background beside its glyphs
                // stays visible. A G_BOXTEXT/G_FBOXTEXT is never promoted and
                // keeps its thickness (`:589-598` zero `th` only inside the
                // `obtype == G_TEXT` / `G_FTEXT` arms) — a TRANSPARENT one is
                // filled `backgrcol` *and* keeps its border. (EmuTOS's
                // `tmode = MD_TRANS` switch in the same block has no
                // counterpart: `gsx_tblt` here always writes transparently.)
                if d3 != 0 && ipat == IP_HOLLOW && icol == WHITE {
                    ipat = IP_SOLID;
                    icol = BACKGR_COL.load(Ordering::Relaxed) as Pel;
                    if mode_replace && !is_box {
                        is_box = true;
                        th = 0;
                    }
                }
                // SELECTED interior swap, only when the object changes colour
                // (EmuTOS `changecol && (state & SELECTED)`, gemoblib.c:681-690)
                // and only for a boxed text — the swap sits inside the
                // `case G_BOXTEXT: case G_FBOXTEXT:` fill (`:669-693`), which
                // an unpromoted G_TEXT/G_FTEXT never reaches. A black-and-white
                // non-FL3DOBJ text (changecol FALSE) is drawn as normal and
                // XOR-inverted by the trailing block; an FL3DACT one
                // (actcolchange = 0) keeps its `backgrcol` face.
                if selected && changecol && is_box {
                    if ipat == IP_HOLLOW {
                        ipat = IP_SOLID;
                        icol = BLACK;
                    } else {
                        icol = xor_color(backend, icol);
                    }
                }
                // D-089 signed thickness (see the box arm above): positive =
                // inside border (fill/text inset by `inset`), negative = outside
                // border (fill/text keep the full rect).
                let inset = th.max(0);
                if is_box {
                    if th != 0 {
                        draw_signed_border(backend, x, y, w, h, th, bcol, clip);
                    }
                    gr_rect(backend, x + inset, y + inset, w - 2 * inset, h - 2 * inset, icol, ipat, clip);
                }
                deco_bcol = bcol;
                deco_th = th;
                effect_icol = icol;
                // Format the string: the F-variants merge `te_ptext` into the
                // `te_ptmplt` template (EmuTOS `ob_format`); the plain variants
                // draw `te_ptext` directly.
                let raw = read_cstr(t, ted.te_ptext);
                let mut fmtbuf = [0u8; MAX_LEN];
                let s: &[u8] = if ob_type == G_FTEXT || ob_type == G_FBOXTEXT {
                    let tmpl = read_cstr(t, ted.te_ptmplt);
                    let n = ob_format(ted.te_just, raw, tmpl, &mut fmtbuf);
                    &fmtbuf[..n]
                } else {
                    raw
                };
                // Text colour from te_color, complemented when SELECTED *and*
                // the object changes colour (EmuTOS gemoblib.c:699-706; the
                // `tmode = MD_TRANS` half has no counterpart, `gsx_tblt` always
                // writes transparently). Otherwise the text stays `tcol` and,
                // for a non-FL3DOBJ object, the trailing XOR inverts it with
                // its ground.
                let ink = if selected && changecol { xor_color(backend, tcol) } else { tcol };
                // EmuTOS: `rc_copy(&t,&c); gr_inside(&c, tmpth);` then the 3D
                // `movetext` nudge, then `gr_gtext(te_just, te_font, te_ptext,
                // &c)` (gemoblib.c:735-757) — so the text sits inside a
                // *positive* border and is drawn in the **TEDINFO's own font**,
                // which a resource may set to SMALL.
                let (cx, cy) = ted_movetext(movetext, selected, x + inset, y + inset);
                gr_gtext(
                    backend,
                    ted.te_just,
                    AesFont::from_te_font(ted.te_font),
                    s,
                    cx,
                    cy,
                    w - 2 * inset,
                    h - 2 * inset,
                    ink,
                    clip,
                );
            }
        }
        G_IMAGE => {
            // The BITBLK interior is read at the tree's ABI width (see the *TEXT
            // arm); an LP64 tree reads it byte-identically to before.
            if ob_spec != 0 {
                // SAFETY: ob_spec of a G_IMAGE points at a BITBLK packed at the
                // tree's ABI width.
                let bi = unsafe { BitBlk::read_abi(ob_spec, t) };
                let bi = &bi;
                // SELECTED (EmuTOS just_draw, gemoblib.c:766-770): XOR the image
                // background (the blit is transparent, so the inverted ground
                // shows through the clear bits) and invert the image ink.
                let col = if selected {
                    let pat = PatternFill {
                        fg: BLACK,
                        bg: WHITE,
                        pattern: &IP_PATTERNS[IP_SOLID as usize],
                        mode: WriteMode::Xor,
                    };
                    backend.fill_pattern(x, y, w, h, pat, clip);
                    xor_color(backend, bi.bi_color as Pel)
                } else {
                    bi.bi_color as Pel
                };
                blit_image(t, backend, bi, x, y, col, clip);
                // The trailing block never XORs a G_IMAGE a second time:
                // `xor_is_ok(G_IMAGE)` is FALSE (`:115-116`), so `changecol`
                // is TRUE for a non-FL3DOBJ image, and an FL3DOBJ one is
                // excluded by the flag test (`:936`).
            }
        }
        G_ICON | G_CICON => {
            // The ICONBLK interior is read at the tree's ABI width (see the
            // *TEXT arm); an LP64 tree reads it byte-identically to before.
            //
            // EmuTOS `just_draw` (gemoblib.c:776-791): one arm for both
            // types. A G_CICON's `ob_spec` is a CICONBLK, which *begins* with
            // a mono ICONBLK (`obdefs.h` `CICONBLK { ICONBLK monoblk; CICON
            // *mainlist; }`), so its head is read exactly like a G_ICON's;
            // `cicon` is NULL for a G_ICON and `((CICONBLK *)spec)->mainlist`
            // for a G_CICON — the variant the **loader** chose at `rsrc_load`
            // (`best_match`, `aes/resource.rs` `fix_cicons`), NULL when it
            // found none — and `gr_gicon` draws colour or mono by it (TASK/A41).
            if ob_spec != 0 {
                // SAFETY: ob_spec of a G_ICON/G_CICON points at an ICONBLK (the
                // CICONBLK head for G_CICON) packed at the tree's ABI width.
                let ib = unsafe { iconblk_read_abi(ob_spec, t) };
                let cicon = if ob_type == G_CICON {
                    // SAFETY: a G_CICON's ob_spec is a whole CICONBLK; its
                    // `mainlist` is 0 or a CICON, both at the tree's ABI width.
                    unsafe {
                        let list = ciconblk_mainlist(ob_spec, t);
                        (list != 0).then(|| Cicon::read_abi(list, t))
                    }
                } else {
                    None
                };
                draw_icon(t, backend, &ib, x, y, ob_state, cicon.as_ref(), clip);
            }
            // `gr_gicon` has rendered the selection itself (fg/bg swap or
            // `sel_data`/darkening); EmuTOS clears the bit so the trailing
            // SELECTED XOR never touches an icon (`state &= ~SELECTED`,
            // gemoblib.c:790).
            ob_state &= !SELECTED;
        }
        // AUD-P3-002: an application-drawn object. Fill a PARMBLK and call its
        // `ub_code` (EmuTOS `just_draw` G_USERDEF, gemoblib.c:792-794:
        // `state = ob_user(tree, obj, &t, spec, state, state)`). The returned
        // state feeds the trailing decorations. A kernel-built tree's callback
        // is called directly, an app tree's runs at EL0 in the current process
        // (see `ob_user`). A null `ob_spec` (no USERBLK)
        // is left to the `_` arm so `ob_user` never dereferences null.
        G_USERDEF if ob_spec != 0 => {
            ob_state = ob_user(t, obj, x, y, w, h, ob_spec, clip, ob_state, ob_state);
        }
        // Remaining sub-types land in later batches.
        _ => {}
    }

    // ── ob_state decorations ────────────────────────────────────────────
    // EmuTOS `just_draw` draws these after the type body, in the order
    // OUTLINED → SHADOWED → CHECKED → CROSSED → DISABLED → SELECTED, then the
    // 3D bevel last (gemoblib.c:867-952). CHECKED is drawn inline in the
    // G_STRING arm (the only checked object type NewTOS renders); the trailing
    // SELECTED XOR is the last decoration below (A85).

    // D3: EmuTOS draws NO extra frame for DEFAULT — `just_draw` never reads
    // the DEFAULT flag; a default button's heavier look comes entirely from
    // its thicker negative `th` (D4, gemobjop.c:63-64). The previously-invented
    // double frame 3–4 px out is removed. OUTLINED is a real EmuTOS decoration
    // (gemoblib.c:869-887): an OUTLINED FL3DBAK background gets the nested 3D
    // outline (`draw_3d_outline`, gemoblib.c:875-877), every other type gets the
    // classic GEM double frame — a black line 3 px out plus a 2 px white ring —
    // so the object's own border becomes the inner line (gemoblib.c:882-885).
    if ob_state & OUTLINED != 0 {
        if d3 == FL3DBAK {
            // D7 (N3): flat-panel 3D outline, not the flat double frame.
            draw_3d_outline(backend, x, y, w, h, clip);
        } else {
            draw_border(backend, x - 3, y - 3, w + 6, h + 6, 1, BLACK, clip);
            draw_border(backend, x - 2, y - 2, w + 4, h + 4, 2, WHITE, clip);
        }
    }

    // SHADOWED: a solid drop shadow to the lower-right, sized by |th| and drawn
    // in the border colour (EmuTOS gemoblib.c:889-901). A th==0 object casts no
    // shadow (the shadow size derives from the border thickness). For a positive
    // (inside) border the shadow hugs the inset content rect; for a negative
    // (outside) border it hugs the full rect.
    if ob_state & SHADOWED != 0 {
        let sth = deco_th.abs();
        if sth != 0 {
            let (sx, sy, sw, sh) = if deco_th > 0 {
                (x + deco_th, y + deco_th, w - 2 * deco_th, h - 2 * deco_th)
            } else {
                (x, y, w, h)
            };
            backend.fill_rect(sx, sy + sh + sth, sw + sth, 2 * sth, deco_bcol, clip);
            backend.fill_rect(sx + sw + sth, sy, 2 * sth, sh + 3 * sth, deco_bcol, clip);
        }
    }

    // CROSSED: two WHITE diagonals across the object (EmuTOS gemoblib.c:910-915,
    // transparent) — visible over a filled/selected box.
    if ob_state & CROSSED != 0 {
        backend.draw_line(x, y, x + w - 1, y + h - 1, WHITE, WriteMode::Transparent, clip, 0xFFFF);
        backend.draw_line(x, y + h - 1, x + w - 1, y, WHITE, WriteMode::Transparent, clip, 0xFFFF);
    }

    // DISABLED: grey the object out with a 50 % transparent dither (EmuTOS
    // `bb_fill(IP_4PATT)`), over the 3D-background colour for a 3D panel.
    if ob_state & DISABLED != 0 {
        let fg = if d3 == FL3DBAK { GRAY_LT } else { WHITE };
        let pat = PatternFill { fg, bg: WHITE, pattern: &STIPPLE_50, mode: WriteMode::Transparent };
        backend.fill_pattern(x, y, w, h, pat, clip);
    }

    // SELECTED (A85 / D-104): the last state decoration, EmuTOS
    // gemoblib.c:932-943 —
    //
    //     if (state & SELECTED)
    //         if (!(flags & FL3DOBJ) && !changecol)
    //             bb_fill(MD_XOR, FIS_SOLID, IP_SOLID, t.g_x, t.g_y, t.g_w, t.g_h);
    //
    // An object that is neither an indicator/activator (FL3DBAK is *not*
    // FL3DOBJ, obdefs.h:92-97) nor colour-changing (`changecol`, preamble) is
    // drawn as normal by its type arm and inverted here; a `changecol` object
    // was swapped in the type body instead, and an FL3DOBJ object shows its
    // press by the bevel and the text nudge alone. `ob_state` is what the type
    // arm left: an icon has cleared SELECTED (`:790`), a G_USERDEF's callback
    // returned the bits it did not handle (`:793`) — so a userdef that hands
    // SELECTED back is inverted by the AES, as in EmuTOS.
    //
    // The rectangle is `t` after the state block's inset by a *positive* `th`
    // (`if (th > 0) gr_inside(&t, th)`, `:889-890`) — `deco_th` is the type
    // arm's final `th` (a G_TITLE's 1, gemobjop.c:44-46; a promoted G_TEXT's
    // 0; a box's spec byte); a negative (outside) border leaves the full rect.
    // `MD_XOR` with the solid pattern complements every device pixel
    // (`vdi/vdi_line.c:484-495` `*work = ~temp`; the kernel backend's
    // `xor_pixel`): a hollow-white box becomes black with white text, an
    // LWHITE ground becomes its RGB complement (not the LBLACK pen), a dither
    // its inverse, and an unfilled object (G_IBOX, G_STRING, unboxed text)
    // inverts whatever lies under it.
    if ob_state & SELECTED != 0 && !d3obj && !changecol {
        let (tx, ty, tw, th_) = if deco_th > 0 {
            (x + deco_th, y + deco_th, w - 2 * deco_th, h - 2 * deco_th)
        } else {
            (x, y, w, h)
        };
        if tw > 0 && th_ > 0 {
            let pat = PatternFill {
                fg: BLACK,
                bg: WHITE,
                pattern: &IP_PATTERNS[IP_SOLID as usize],
                mode: WriteMode::Xor,
            };
            backend.fill_pattern(tx, ty, tw, th_, pat, clip);
        }
    }

    // 3D bevel LAST (EmuTOS gemoblib.c:950-951 calls add_3d_effect after the
    // whole state block, gated on `flags & FL3DOBJ`) over the grown rect
    // (`effect_grect`, :545) with the saved `ob_sst` thickness and the
    // interior colour. Indicator and activator get the *same* bevel — raised
    // until SELECTED, then sunk (`:252`, `:270`); they differ only in the
    // movetext/changecol dynamics above. FL3DBAK is NOT an FL3DOBJ, so it gets
    // NO button bevel — its 3D optic is the draw_3d_outline recess above.
    if d3obj {
        add_3d_effect(backend, x, y, w, h, selected, effect_th, effect_icol, clip);
    }
}

/// The 3D bevel of an indicator/activator — EmuTOS `add_3d_effect`
/// (`gemoblib.c:220-283`), raised while not `selected`, sunk when it is.
///
/// `(x, y, w, h)` is the **grown** rectangle `just_draw` saved before the type
/// body (`effect_grect`, `:545`); an *inside* border (`th > 0`) first shrinks
/// it by `th` on every side (`:231-238`) so the bevel lands inside that border
/// instead of over it — an outside border (`th < 0`) sits beyond the rect and
/// needs no room. Two one-pixel polylines follow, each stopping one pixel
/// short of both ends so the two far corners keep the face colour:
///
/// ```text
/// bottom-left → top-left → top-right      (x, y+h-2) → (x, y) → (x+w-2, y)
///     WHITE, or BLACK when selected                                  (:252)
/// bottom-left → bottom-right → top-right  (x+1, y+h-1) → (x+w-1, y+h-1) → (x+w-1, y+1)
///     LBLACK, or LWHITE when selected                                (:270)
/// ```
///
/// The second colour falls back to BLACK/WHITE when it would vanish into the
/// object's own interior colour `icol`, or lies beyond the workstation's colour
/// count (`:271-272`). Opaque (`MD_REPLACE`), solid line style (`:226-228`).
#[allow(clippy::too_many_arguments)]
pub(crate) fn add_3d_effect(
    backend: &dyn RasterBackend,
    x: i16,
    y: i16,
    w: i16,
    h: i16,
    selected: bool,
    th: i16,
    icol: Pel,
    clip: ClipRect,
) {
    let (x, y, w, h) = if th > 0 { (x + th, y + th, w - 2 * th, h - 2 * th) } else { (x, y, w, h) };
    let mode = WriteMode::Replace;
    let solid = 0xFFFF;

    let hi = if selected { BLACK } else { WHITE };
    backend.draw_line(x, y + h - 2, x, y, hi, mode, clip, solid); // left
    backend.draw_line(x, y, x + w - 2, y, hi, mode, clip, solid); // top

    let mut lo = if selected { GRAY_LT } else { GRAY_DK };
    if lo == icol || backend.num_colors() < lo as u16 {
        lo = if selected { WHITE } else { BLACK };
    }
    backend.draw_line(x + 1, y + h - 1, x + w - 1, y + h - 1, lo, mode, clip, solid); // bottom
    backend.draw_line(x + w - 1, y + h - 1, x + w - 1, y + 1, lo, mode, clip, solid); // right
}

/// Draw the EmuTOS 3D outline for an OUTLINED FL3DBAK background panel
/// (`draw_3d_outline`, gemoblib.c:153-214). It is a 3-pixel nested frame sitting
/// 1..3 px *outside* the rect (offsets +3, +2, +1). The bottom+right edges are
/// all grey (EmuTOS LBLACK = [`GRAY_DK`]); the top+left edges are grey on the
/// outermost line (offset +3) and white on the two inner lines (+2, +1) — a
/// raised-panel optic distinct from both the button bevel ([`add_3d_effect`])
/// and the classic OUTLINED black+white double frame. All draws are opaque
/// ([`WriteMode::Replace`], EmuTOS `MD_REPLACE`) with a solid line mask, and the
/// top-left group is drawn after the bottom-right group so shared corner pixels
/// take the later colour exactly as EmuTOS's `v_pline` order does.
fn draw_3d_outline(backend: &dyn RasterBackend, x: i16, y: i16, w: i16, h: i16, clip: ClipRect) {
    if w < 1 || h < 1 {
        return;
    }
    let mode = WriteMode::Replace;
    let solid = 0xFFFF;
    // Bottom-left -> bottom-right -> top-right: 3 grey nested L-shapes.
    for d in [3i16, 2, 1] {
        let bl_x = x - d;
        let br_x = x + w - 1 + d;
        let bot_y = y + h - 1 + d;
        let top_y = y - d;
        backend.draw_line(bl_x, bot_y, br_x, bot_y, GRAY_DK, mode, clip, solid); // bottom
        backend.draw_line(br_x, bot_y, br_x, top_y, GRAY_DK, mode, clip, solid); // right
    }
    // Bottom-left -> top-left -> top-right: outermost grey, inner two white.
    for d in [3i16, 2, 1] {
        let side_x = x - d;
        let bot_y = y + h - 1 + d;
        let top_y = y - d;
        let tr_x = x + w - 1 + d;
        let col = if d == 3 { GRAY_DK } else { WHITE };
        backend.draw_line(side_x, bot_y, side_x, top_y, col, mode, clip, solid); // left
        backend.draw_line(side_x, top_y, tr_x, top_y, col, mode, clip, solid); // top
    }
}

/// Draw a `t`-pixel border frame around `(x, y, w, h)` as four strips.
fn draw_border(
    backend: &dyn RasterBackend,
    x: i16,
    y: i16,
    w: i16,
    h: i16,
    t: i16,
    col: Pel,
    clip: ClipRect,
) {
    let t = t.max(1);
    backend.fill_rect(x, y, w, t, col, clip); // top
    backend.fill_rect(x, y + h - t, w, t, col, clip); // bottom
    backend.fill_rect(x, y, t, h, col, clip); // left
    backend.fill_rect(x + w - t, y, t, h, col, clip); // right
}

/// Draw a box border honouring the D-089 signed-thickness convention: a
/// positive `th` draws the frame INSIDE `(x, y, w, h)` (EmuTOS `gr_box` with
/// `th > 0` shrinks the nested outlines inward), a negative `th` draws it
/// OUTSIDE (the frame sits in the |th|-pixel margin around the rect, leaving the
/// rect itself for the fill — matching `gr_box`'s outward-growing outlines once
/// the full-rect fill overpaints the shared edge). `th == 0` draws nothing.
///
/// D5: a negative `th` draws **`|th|+1`** pixels, **edge-inclusive** — EmuTOS
/// `gr_box` (gemgraf.c:813-832) does `if (th < 0) th--;` then draws one nested
/// outline per step up to and including `gr_inside(0)`, i.e. the innermost ring
/// lands on the rect's own edge. So `th = -1` is a 2 px frame spanning `x-1..x`,
/// not a single `x-1` ring. Because the border is drawn before the interior
/// fill (see the box/button arm), a *solid* box's fill overpaints that inner
/// (edge) ring, leaving the visible border unchanged; only HOLLOW interiors
/// (buttons, hollow boxes) show the extra edge pixel.
#[allow(clippy::too_many_arguments)]
fn draw_signed_border(backend: &dyn RasterBackend, x: i16, y: i16, w: i16, h: i16, th: i16, col: Pel, clip: ClipRect) {
    match th.cmp(&0) {
        core::cmp::Ordering::Greater => draw_border(backend, x, y, w, h, th, col, clip),
        core::cmp::Ordering::Less => {
            let t = -th;
            draw_border(backend, x - t, y - t, w + 2 * t, h + 2 * t, t + 1, col, clip);
        }
        core::cmp::Ordering::Equal => {}
    }
}

/// The 3D text nudge EmuTOS applies to the TEDINFO family — `G_TEXT`,
/// `G_BOXTEXT`, `G_FTEXT`, `G_FBOXTEXT`, `G_BOXCHAR` (`gemoblib.c:744-752`):
///
/// ```text
/// if (movetext) {
///     if (state & SELECTED) c.g_x++;   /* pressed: right */
///     else                  c.g_y--;   /* raised:  up    */
/// }
/// ```
///
/// Note this is **not** the `G_BUTTON` rule — the string block moves a raised
/// button's label one pixel up *and* left (`gemoblib.c:833-836`) — so the two
/// are kept apart, as in EmuTOS.
#[inline]
fn ted_movetext(movetext: bool, selected: bool, x: i16, y: i16) -> (i16, i16) {
    if !movetext {
        (x, y)
    } else if selected {
        (x + 1, y)
    } else {
        (x, y - 1)
    }
}

/// `gsx_tblt` (EmuTOS `gemgraf.c:500-527`) — blit `text` in `fnt` with the
/// **cell top** at `(x, y)`.
///
/// EmuTOS hands the VDI a *baseline* of `y + gl_hptschar` (IBM) or
/// `y + gl_hsptschar` (SMALL) and never touches `vst_alignment`, so the VDI
/// default (left / baseline) applies and the glyph's first raster row lands
/// at `baseline - top` = `y`. Drawing cell-top-anchored is therefore the same
/// pixels, without a baseline round trip.
fn gsx_tblt(
    backend: &dyn RasterBackend,
    fnt: AesFont,
    x: i16,
    y: i16,
    text: &[u8],
    fg: Pel,
    clip: ClipRect,
) {
    let cw = fnt.cell_w();
    for (i, &c) in text.iter().enumerate() {
        let gx = x + i as i16 * cw;
        match fnt {
            AesFont::Ibm => {
                backend.draw_glyph(gx, y, crate::vdi::font::glyph(c as u16), fg, WriteMode::Transparent, clip)
            }
            AesFont::Small => backend.draw_glyph_sized(
                gx,
                y,
                crate::vdi::font::glyph_small(c as u16),
                crate::vdi::font::SMALL_CELL_W,
                crate::vdi::font::SMALL_CELL_H,
                fg,
                WriteMode::Transparent,
                clip,
            ),
        }
    }
}

/// `gr_gtext` (EmuTOS `gemgraf.c:595-605`) — justify `text` inside the box
/// `(x, y, w, h)` and blit it in `fnt`. Inlines its two helpers:
///
/// * `gsx_tcalc` (`gemgraf.c:480-497`) clamps the text extent to the box
///   (`textw = min(w, len*cw)`, `texth = min(h, ch)`) and then drops **whole
///   characters** that do not fit (`numchs = min(numchs, textw/cw)`). A box
///   shorter than one character cell (`texth / ch == 0`) yields `numchs = 0`
///   — EmuTOS draws *nothing* rather than a vertically cropped line.
/// * `gr_just` (`gemgraf.c:567-589`) centres vertically and justifies
///   horizontally, in both axes only when there is slack and rounding the
///   half-margin **up** (`(n + 1) / 2`).
///
/// This is the path every `G_TEXT`/`G_BOXTEXT`/`G_FTEXT`/`G_FBOXTEXT`/
/// `G_BOXCHAR` object and every icon label takes. `G_STRING`/`G_TITLE`/
/// `G_BUTTON` do **not** come through here — EmuTOS justifies those inline in
/// `just_draw` with a different (unclamped, untruncated, `n/2`) formula, which
/// the call sites reproduce.
#[allow(clippy::too_many_arguments)]
fn gr_gtext(
    backend: &dyn RasterBackend,
    just: i16,
    fnt: AesFont,
    text: &[u8],
    x: i16,
    y: i16,
    w: i16,
    h: i16,
    fg: Pel,
    clip: ClipRect,
) {
    let cw = fnt.cell_w();
    let ch = fnt.cell_h();

    // gsx_tcalc — `saturating_mul` only guards the (unreachable, `read_cstr`
    // caps at 256 chars) i16 overflow EmuTOS would wrap on; the clamp below
    // makes any saturated value indistinguishable from the exact one.
    let mut numchs = text.len() as i16;
    let textw = w.min(numchs.saturating_mul(cw));
    let texth = h.min(ch);
    numchs = if texth / ch != 0 { numchs.min(textw / cw) } else { 0 };
    if numchs <= 0 {
        return;
    }

    // gr_just
    let mut ty = y;
    let slack_h = h - texth;
    if slack_h > 0 {
        ty += (slack_h + 1) / 2;
    }
    let mut tx = x;
    let slack_w = w - textw;
    if slack_w > 0 {
        match just {
            TE_CNTR => tx += (slack_w + 1) / 2,
            TE_RIGHT => tx += slack_w,
            TE_LEFT => {}
            // EmuTOS's `if (TE_CNTR) … else if (TE_RIGHT) …` chain leaves any
            // other `te_just` value at the box origin, like TE_LEFT.
            _ => {}
        }
    }

    gsx_tblt(backend, fnt, tx, ty, &text[..numchs as usize], fg, clip);
}

/// EmuTOS `is_dashes` (`gemoblib.c:455-464`): every character of the string
/// is `'-'`. The caller applies the `len > 3` gate; an empty string is
/// "all dashes" here exactly as in the C (`while(len--)` never runs), which
/// that gate excludes.
fn is_dashes(s: &[u8]) -> bool {
    s.iter().all(|&c| c == b'-')
}

/// Read a NUL-terminated C string from a native pointer in `ob_spec`
/// (trusted GEM pointer, like the other AES managers). Capped at 256.
///
/// For an owned tree ([`ObjTree::owner`], A97) the string is read through
/// the owner's mapping, page by page as [`crate::aes::window::LiveStr`]
/// reads a title: the walk ends at the NUL, at the cap, or at the first
/// page the owner has no contiguous mapping for; an unmapped start is the
/// empty string.
fn read_cstr(t: ObjTree, spec: u64) -> &'static [u8] {
    if spec == 0 {
        return &[];
    }
    let Some(start) = t.ua(spec) else {
        return &[];
    };
    let p = start as *const u8;
    let page = usermem::PAGE_SIZE;
    // SAFETY: GEM contract — `ob_spec` of a string/button is a valid
    // pointer to a NUL-terminated string in the caller's memory; for an
    // owned tree `p` is the owner's page alias and the walk re-resolves at
    // every page boundary before reading on.
    unsafe {
        let mut n = 0usize;
        while n < 256 {
            let va = spec.wrapping_add(n as u64);
            if n != 0 && va & (page - 1) == 0 && t.ua(va) != Some(start.wrapping_add(n as u64)) {
                break;
            }
            if *p.add(n) == 0 {
                break;
            }
            n += 1;
        }
        core::slice::from_raw_parts(p, n)
    }
}

/// Blit a `G_IMAGE` BITBLK at `(x, y)`: a transparent mono blit in colour
/// `col` (set bits drawn, clear bits leave the background). `col` is normally
/// `bi_color`, or its complement for a SELECTED image (EmuTOS `just_draw`).
fn blit_image(t: ObjTree, backend: &dyn RasterBackend, bi: &BitBlk, x: i16, y: i16, col: Pel, clip: ClipRect) {
    let stride = (bi.bi_wb / 2).max(1) as u16; // bytes → 16-bit words
    blit_plane(
        t,
        backend,
        bi.bi_pdata,
        stride,
        bi.bi_x,
        bi.bi_y,
        Rect { x, y, w: bi.bi_wb * 8, h: bi.bi_hl },
        col,
        clip,
    );
}

/// Draw a `G_ICON` ICONBLK — or, with `cicon`, a `G_CICON`'s chosen colour
/// variant — at object origin `(ox, oy)`: EmuTOS `gr_gicon(state, ib, cicon)`
/// (`aes/gemgraf.c:710-807`).
///
/// **Mono** (`cicon == None`): the mask plane in the background colour, the
/// data plane in the foreground colour, then the character and text;
/// SELECTED swaps fg/bg (`:743-751`). This path is unchanged by A41.
///
/// **Colour** (`cicon == Some`, `CONF_WITH_COLOUR_ICONS`):
/// 1. data/mask selection (`:729-741`): the SELECTED image and mask when
///    `state & SELECTED` and the variant has one, else `col_data`/`col_mask`;
///    fg/bg are **not** swapped for the blits;
/// 2. unless `WHITEBAK` is set and the icon's background is white
///    (`:755`): the mask in `bgcol` (`gr_gblt(pmask, pi, bgcol, fgcol)`,
///    transparent), and — for a non-empty label — the label box in `fgcol`
///    when SELECTED, else `bgcol` (`:760-766`);
/// 3. the image: `gr_colourblit` (`:772`), a Memory→Screen `vro_cpyfm`
///    with `S_AND_D` of the device-form pixels ([`blit_device_mem`]);
/// 4. when SELECTED: fg/bg swap for the character and label (`:775-777`),
///    and — for a variant **without** a SELECTED image — the darkening:
///    a copy of the mask ANDed with `0x5555`/`0xAAAA` on alternate rows
///    (`dither_mask`, `:680-700`) is blitted in BLACK (`:778-786`);
/// 5. the character and label as for mono (`:797-806`).
fn draw_icon(
    t: ObjTree,
    backend: &dyn RasterBackend,
    ib: &IconBlk,
    ox: i16,
    oy: i16,
    state: u16,
    cicon: Option<&Cicon>,
    clip: ClipRect,
) {
    let selected = state & SELECTED != 0;
    let chword = ib.ib_char;
    let mut fg = ((chword >> 12) & 0x0f) as Pel;
    let mut bg = ((chword >> 8) & 0x0f) as Pel;
    let stride = (ib.ib_wicon / 16).max(1) as u16; // icon width in words
    let rect = Rect {
        x: ox + ib.ib_xicon,
        y: oy + ib.ib_yicon,
        w: ib.ib_wicon,
        h: ib.ib_hicon,
    };
    match cicon {
        None => {
            if selected {
                core::mem::swap(&mut fg, &mut bg);
            }
            blit_plane(t, backend, ib.ib_pmask, stride, 0, 0, rect, bg, clip);
            blit_plane(t, backend, ib.ib_pdata, stride, 0, 0, rect, fg, clip);
        }
        Some(ci) => {
            let (pdata, pmask) = if selected && ci.sel_data != 0 {
                (ci.sel_data, ci.sel_mask)
            } else {
                (ci.col_data, ci.col_mask)
            };
            let label = if ib.ib_ptext != 0 { read_cstr(t, ib.ib_ptext) } else { &[] };
            if !(state & WHITEBAK != 0 && bg == WHITE) {
                blit_plane(t, backend, pmask, stride, 0, 0, rect, bg, clip);
                if !label.is_empty() {
                    gr_rect(
                        backend,
                        ox + ib.ib_xtext,
                        oy + ib.ib_ytext,
                        ib.ib_wtext,
                        ib.ib_htext,
                        if selected { fg } else { bg },
                        IP_SOLID,
                        clip,
                    );
                }
            }
            // The device-form image is `stride * 16` pixels of 2 bytes per
            // row, `rect.h` rows — resolved as one span through the owner's
            // mapping for an owned tree (A97), skipped if unmapped.
            let img_bytes = (stride as u64) * 16 * 2 * rect.h.max(0) as u64;
            if let Some(pdata) = (pdata != 0).then(|| t.ua_span(pdata, img_bytes)).flatten() {
                // SAFETY: GEM contract — the loader's `fix_cicons` left
                // `ib_wicon × ib_hicon` device-form pixels behind `col_data`
                // / `sel_data` (rows `ib_wicon / 16` words wide).
                unsafe {
                    crate::vdi::blit_device_mem(
                        backend,
                        pdata as *const u16,
                        Rect { x: rect.x, y: rect.y, w: (stride as i16) * 16, h: rect.h },
                        S_AND_D,
                        clip,
                    );
                }
            }
            if selected {
                core::mem::swap(&mut fg, &mut bg);
                if ci.sel_data == 0 {
                    // `dither_mask` (`gemgraf.c:680-700`): every mask word
                    // ANDed with 0x5555 on even rows, 0xAAAA on odd rows,
                    // then `gr_gblt(mask, pi, BLACK, WHITE)` — set bits in
                    // BLACK, transparent.
                    let rows = rect.h.max(0) as usize;
                    let words = stride as usize * rows;
                    // The mask span, through the owner's mapping for an owned
                    // tree (A97); an unmapped mask is not dithered.
                    let mask = if pmask != 0 && words != 0 { t.ua_span(pmask, words as u64 * 2) } else { None };
                    if let Some(pmask) = mask {
                        // SAFETY: GEM contract — `words` mask words behind
                        // `col_mask` (no alignment required).
                        let src = unsafe { MonoBits::from_raw(pmask as *const u8, words) };
                        let mut dith = alloc::vec![0u16; words];
                        for (i, w) in dith.iter_mut().enumerate() {
                            let dither = if (i / stride as usize) & 1 != 0 { 0xAAAA } else { 0x5555 };
                            *w = src.word(i).unwrap_or(0) & dither;
                        }
                        // `dith` is a kernel buffer, never a tree address:
                        // blit it as such, through no owner mapping.
                        blit_plane_kernel(backend, dith.as_ptr() as u64, stride, 0, 0, rect, BLACK, clip);
                    }
                }
            }
        }
    }

    // The overlay character (a drive letter, a trash "T", …): EmuTOS draws it
    // in the **SMALL** font at `ib_xicon + ib_xchar` / `ib_yicon + ib_ychar`,
    // i.e. `ib_xchar`/`ib_ychar` are relative to the icon *bitmap*, not to the
    // object (`gsx_tblt(SMALL, ib->ib_xicon+ib->ib_xchar,
    // ib->ib_yicon+ib->ib_ychar, 1)`, gemgraf.c:800-803 — `ib_xicon`/`ib_yicon`
    // were already made absolute by `just_draw`, gemoblib.c:784-789).
    let c = (chword & 0xff) as u8;
    if c != 0 {
        gsx_tblt(
            backend,
            AesFont::Small,
            ox + ib.ib_xicon + ib.ib_xchar,
            oy + ib.ib_yicon + ib.ib_ychar,
            &[c],
            fg,
            clip,
        );
    }
    if ib.ib_ptext != 0 {
        let s = read_cstr(t, ib.ib_ptext);
        if !s.is_empty() {
            // Label background (EmuTOS `gr_gicon`): a solid `bg`-colour
            // rectangle behind the label so it reads against a patterned
            // desktop, and a SELECTED icon's label shows the swapped
            // (inverted) background. Then the label text in `fg`. The
            // colour path painted its box above, before the image, in
            // EmuTOS's order (`:760-766`).
            if cicon.is_none() {
                gr_rect(
                    backend,
                    ox + ib.ib_xtext,
                    oy + ib.ib_ytext,
                    ib.ib_wtext,
                    ib.ib_htext,
                    bg,
                    IP_SOLID,
                    clip,
                );
            }
            // The label: `gr_gtext(TE_CNTR, SMALL, ib->ib_ptext, pt)` with
            // `pt` = the `ib_xtext..ib_htext` rectangle (gemgraf.c:806). The
            // SMALL font is what makes a resource's label box fit — GEM icon
            // resources size it for 6×6 (EmuTOS's own `icon.rsc` and
            // TeraDesk's `icons.rsc` both use `ib_wtext = 12*6 = 72`,
            // `ib_htext = 6+2 = 8`), so drawing it in the 8×16 font
            // overflows the box in both axes.
            gr_gtext(
                backend,
                TE_CNTR,
                AesFont::Small,
                s,
                ox + ib.ib_xtext,
                oy + ib.ib_ytext,
                ib.ib_wtext,
                ib.ib_htext,
                fg,
                clip,
            );
        }
    }
}

/// Transparent mono blit of one bit-plane (`addr` → big-endian 16-bit
/// words, `stride` words per row) into `dst`, set bits drawn in `fg`.
/// `addr` is a **tree address** (`ib_pmask`/`ib_pdata`/`bi_pdata`): for an
/// owned tree it is resolved as one span through the owner's mapping (A97)
/// and skipped if any page of it is unmapped.
#[allow(clippy::too_many_arguments)]
fn blit_plane(
    t: ObjTree,
    backend: &dyn RasterBackend,
    addr: u64,
    stride: u16,
    src_x: i16,
    src_y: i16,
    dst: Rect,
    fg: Pel,
    clip: ClipRect,
) {
    if addr == 0 || dst.w <= 0 || dst.h <= 0 {
        return;
    }
    let rows = src_y.max(0) as u64 + dst.h as u64;
    let Some(kva) = t.ua_span(addr, stride as u64 * rows * 2) else {
        return;
    };
    blit_plane_kernel(backend, kva, stride, src_x, src_y, dst, fg, clip);
}

/// [`blit_plane`] for a raster at a **kernel** address (`addr` is readable
/// as is: a kernel buffer, or a tree raster already resolved by
/// [`blit_plane`]).
#[allow(clippy::too_many_arguments)]
fn blit_plane_kernel(
    backend: &dyn RasterBackend,
    addr: u64,
    stride: u16,
    src_x: i16,
    src_y: i16,
    dst: Rect,
    fg: Pel,
    clip: ClipRect,
) {
    if addr == 0 || dst.w <= 0 || dst.h <= 0 {
        return;
    }
    let rows = src_y.max(0) as usize + dst.h as usize;
    let total = stride as usize * rows;
    // SAFETY: GEM contract — `addr` points at >= `total` raster words. Its
    // ALIGNMENT is not part of that contract: `addr` is `ib_pmask`/`ib_pdata`/
    // `bi_pdata` straight out of a resource, which an app may have built or
    // relocated itself (see `MonoBits`). `from_raw` requires none.
    let bits = unsafe { MonoBits::from_raw(addr as *const u8, total) };
    backend.blit_mono(
        dst,
        MonoSrc {
            bits,
            stride_words: stride,
            x: src_x,
            y: src_y,
        },
        fg,
        WHITE,
        // Transparent (former `opaque = false`): set bits in `fg`, clear
        // bits leave the background — the desktop icon/image blit path.
        WriteMode::Transparent,
        clip,
    );
}

/// Recursive hit-test for `objc_find`. Returns the deepest matching child
/// index, else `obj` if `(mx, my)` is inside it, else `NIL`.
fn find_hit(
    t: ObjTree,
    obj: i16,
    depth: i16,
    px: i16,
    py: i16,
    mx: i16,
    my: i16,
) -> i16 {
    let ob_flags = t.flags(obj);
    if ob_flags & HIDETREE != 0 {
        return NIL;
    }
    let ob_state = t.state(obj);
    let x = px + t.x(obj);
    let y = py + t.y(obj);
    // Hit-test the same extent `just_draw` renders. D7: EmuTOS grows the hit
    // rect only for an FL3DOBJ object (`ob_actxywh`, gemoblib.c:322-335 keys on
    // `flags & FL3DOBJ`), which is set on FL3DIND/FL3DACT but CLEAR on FL3DBAK —
    // so an FL3DBAK panel is hit-tested at its un-grown rect, in lockstep with
    // the draw path (which no longer grows it either). An FL3DOBJ object is drawn
    // ADJ3DSTD px larger on every side, plus ADJ3DOUT when OUTLINED, so a click
    // on the visible bevel/outline edge must count as inside. SHADOWED objects
    // are the documented exception: EmuTOS `ob_find` (gemoblib.c:1063-1072)
    // hit-tests them at their un-grown relative rect. Children keep the nominal
    // origin `(x, y)`, exactly as `draw_tree` passes it — draw and hit stay
    // consistent at every level.
    let (hx, hy, hw, hh) = if ob_flags & FL3DOBJ != 0 && ob_state & SHADOWED == 0 {
        let mut adj = ADJ3DSTD;
        if ob_state & OUTLINED != 0 {
            adj += ADJ3DOUT;
        }
        (x - adj, y - adj, t.width(obj) + 2 * adj, t.height(obj) + 2 * adj)
    } else {
        (x, y, t.width(obj), t.height(obj))
    };
    let inside = mx >= hx && mx < hx + hw && my >= hy && my < hy + hh;
    if !inside {
        return NIL;
    }

    if depth > 0 {
        // Search children last-to-first (topmost sibling wins).
        let mut found = NIL;
        let mut child = t.head(obj);
        while child != NIL {
            let hit = find_hit(t, child, depth - 1, x, y, mx, my);
            if hit != NIL {
                found = hit; // keep the last (topmost) match
            }
            if child == t.tail(obj) {
                break;
            }
            child = t.next(child);
        }
        if found != NIL {
            return found;
        }
    }

    // Hit on `obj` itself. Per the spec, `objc_find`/`find_hit` does **not**
    // skip DISABLED objects — it checks only `inside()` and `!HIDETREE`
    // (`gemoblib.c:1080`). A DISABLED object under the point is still
    // returned; ignoring a click on a disabled object is the Form Manager's
    // job (`form_do`/`fm_do` filters DISABLED itself, `gemfmlib.c:224`). See
    // `docs/api/aes/object-form.md` ("objc_find returns DISABLED objects").
    obj
}

// Keep LASTOB referenced (used by tree-building callers / future batches).
const _: u16 = LASTOB;
