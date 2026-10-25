//! AES File Selector (`fsel_input` 90 / `fsel_exinput` 91) — Batch 1.
//!
//! Spec: [`docs/api/aes/resource-fileselector.md`]. The selector reuses
//! the real EmuTOS **FSELECTR** object tree, converted to native form
//! (`tools/mkrsc.py convert references/emutos/aes/gem.rsc`) and **embedded**
//! in the kernel as the built-in default — like EmuTOS links `gem.rsc` into
//! ROM (`erd.c`) — so the selector always works on a bare card; a
//! `C:\GEMSYS\RSC\GEM.RSC` on the drive overrides it when present, so the
//! resource can be replaced without a kernel rebuild. We point its directory /
//! selection / file-name `TEDINFO`s' `te_ptext` at our own buffers — and
//! nothing else about them: they stay `G_FTEXT`/`G_FBOXTEXT` and keep the
//! resource's template, validation string and length, so our buffers hold the
//! **raw** form those templates merge (`fmt_str`, EmuTOS
//! `util/optimize.c:43`). We fill the visible slots from a directory read
//! ([`bdos::fs::list_dir`], `Fsfirst`/`Fsnext`-style), draw it, and run a
//! modal loop: a click selects a file, `OK`/`Cancel` exit.
//!
//! The dialog's 3D look is [`fs_start`], EmuTOS's own one-shot start-up pass
//! (`aes/gemfslib.c:59-152`): it tags the 3D flags **and** makes room for the
//! bevels by moving/resizing every object it touches.
//!
//! **Implemented:** directory list + scroll bar/arrows + drive grid +
//! click-to-select + double-click confirm + OK/Cancel + write-back, plus
//! (EmuTOS `fs_input`) keyboard editing of the `FSDIRECT` path and
//! `FSSELECT` name fields — typing a new path + Enter re-lists it — and
//! walking into a sub-directory by clicking it. The listing matches EmuTOS
//! `fs_active`: folders sort first, then files, both alphabetically, and
//! `.`/`..`/dot-files are filtered out (`gemfslib.c:298,312`). Up-navigation
//! is the `FCLSBOX` close-box gadget (EmuTOS up-one-level, `gemfslib.c:839`)
//! or editing the `FSDIRECT` path + Enter. `fsel_exinput` shows the caller's
//! custom dialog title (`FS_TITLE = addr_in[2]`), else the default
//! "ITEM SELECTOR", centred in the FSTITLE caption (`gemfslib.c:691-693`).

use super::form::{fm_dial, gl_rcenter};
use super::object::{self, Object, ObjTree, TedInfo};
use super::graf::gl_hbox;
use super::{resource, Aespb};
use crate::host::host;
use crate::uaccess::{Space, UserFault};
use crate::vdi::backend::RasterBackend;
use core::cell::UnsafeCell;
use tos_types::aes::{
    ADJ3DSTD, DISABLED, ED_CHAR, ED_END, ED_INIT, FL3DACT, FL3DBAK, FL3DIND, FMD_FINISH, FMD_START,
    G_FBOXTEXT, IP_4PATT, ROOT, SELECTED,
};

// FSELECTR object indices (EmuTOS gem.rsc, verified against the converted
// resource).
const FSTITLE: i16 = 1; // dialog title caption (G_STRING); default "ITEM SELECTOR"
const FSDIRECT: i16 = 3;
const FSDRVTXT: i16 = 4; // "Drive:" caption (G_STRING) above the drive grid
const FSSELECT: i16 = 5;
const FSDRIVES: i16 = 6; // IBOX holding the 26 drive-letter gadgets
const FS1STDRV: i16 = 7; // first drive-letter gadget (A); A..Z = 7..32
const FSLSTDRV: i16 = 32; // last drive-letter gadget (Z)
const FILEAREA: i16 = 33; // IBOX holding closer + title + scroll bar + list
const FCLSBOX: i16 = 34; // close-box gadget = up-one-level (EmuTOS fs_back)
const FTITLE: i16 = 35; // file-box caption (shows the current mask)
const SCRLBAR: i16 = 36; // G_BOX holding the arrows + slider
const FUPAROW: i16 = 37;
const FDNAROW: i16 = 38;
const FSVSLID: i16 = 39; // slider track
const FSVELEV: i16 = 40; // slider thumb
const FILEBOX: i16 = 41;
const F1NAME: i16 = 42;
const NM_NAMES: usize = 9; // F1NAME..F9NAME
const FSOK: i16 = 51;
const FSCANCEL: i16 = 52;
/// Number of drive-letter rows per column in the FSELECTR grid — EmuTOS
/// `DRIVE_ROWS` (`gemfslib.c:40`): 3 columns of 9 cover the 26 drives.
const DRIVE_ROWS: i16 = 9;
/// Free-string index of the default file-selector title in GEM.RSC (EmuTOS
/// `rs_str(ITEMSLCT)`, `gemfslib.c:692`). Verified against `aes/gem.rsc`:
/// free-string 14 = "ITEM SELECTOR".
const ITEMSLCT: usize = 14;

const MAX_DEPTH: i16 = 16;

const GEM_CAP: usize = 8 * 1024;

/// The AES's own file-selector resource (EmuTOS `gem.rsc`), converted to the
/// native LP64 NRSC format and **compiled into the kernel** — the same way
/// EmuTOS links `gem.rsc` into ROM (via `erd.c`). Unlike every app resource
/// (which `rsrc_load`s from disk), the file selector must always be available,
/// so its resource is embedded rather than read from `C:\GEMSYS\RSC\GEM.RSC`.
///
/// The blob is the **pre-relocation** NRSC (internal pointers stored as byte
/// offsets), byte-identical to what the disk file held; `load_fselectr` copies
/// it into `SYS.buf` and relocates it in place, exactly as the old disk path
/// did. Regenerate with:
///
/// ```text
/// python3 tools/mkrsc.py convert references/emutos/aes/gem.rsc \
///     tos-api/src/aes/gem_fsel.nrsc
/// ```
///
/// (also wired as `cargo xtask gen-fsel-rsc`).
const GEM_FSEL_RSC: &[u8] = include_bytes!("gem_fsel.nrsc");
/// Compile-time guarantee that `SYS.buf` can hold the embedded resource
/// (blob size + relocation is fully in-place). Raise [`GEM_CAP`] if the
/// converted `gem.rsc` ever outgrows the buffer.
const _: () = assert!(GEM_CAP >= GEM_FSEL_RSC.len());
/// Maximum directory entries held for scrolling. EmuTOS sizes this
/// dynamically from free RAM (`gemfslib.c:655`, "usually thousands"); we
/// use a large fixed cap in static BSS instead. Dynamic RAM sizing would
/// mean reworking the static `SYS` arrays (`all`/`all_attr`) into heap
/// allocations plus a `dos_avail_anyram()` equivalent — deferred (out of
/// scope for this package). 2048 covers a full FAT16 root directory (512
/// entries) with wide margin; the ~35 KB of BSS is negligible on RPi4.
/// Entries beyond the cap are silently ignored, exactly as Atari TOS /
/// EmuTOS do (`gemfslib.c:290-292`).
const MAX_LIST: usize = 2048;
/// Double-click window (milliseconds).
const DCLICK_MS: u64 = 400;

/// The AES's own GEM.RSC (file selector) + the dynamic text buffers its
/// TEDINFOs point at. Single-CPU, like the rest of the AES.
struct Sys {
    buf: UnsafeCell<[u8; GEM_CAP]>,
    root: UnsafeCell<*mut Object>, // FSELECTR root, null until loaded
    dir: UnsafeCell<[u8; 96]>,     // FSDIRECT path text
    sel: UnsafeCell<[u8; 16]>,     // FSSELECT filename text
    title: UnsafeCell<[u8; 64]>,   // FSTITLE custom-title text (fsel_exinput)
    names: UnsafeCell<[[u8; 16]; NM_NAMES]>, // visible slot text
    all: UnsafeCell<[[u8; 16]; MAX_LIST]>,   // full directory listing
    all_attr: UnsafeCell<[u8; MAX_LIST]>,    // per-entry attribute byte
    count: UnsafeCell<usize>,      // entries in `all`
    top: UnsafeCell<usize>,        // index of the first visible entry
}
unsafe impl Sync for Sys {}

static SYS: Sys = Sys {
    buf: UnsafeCell::new([0u8; GEM_CAP]),
    root: UnsafeCell::new(core::ptr::null_mut()),
    dir: UnsafeCell::new([0u8; 96]),
    sel: UnsafeCell::new([0u8; 16]),
    title: UnsafeCell::new([0u8; 64]),
    names: UnsafeCell::new([[0u8; 16]; NM_NAMES]),
    all: UnsafeCell::new([[0u8; 16]; MAX_LIST]),
    all_attr: UnsafeCell::new([0u8; MAX_LIST]),
    count: UnsafeCell::new(0),
    top: UnsafeCell::new(0),
};

/// FAT/TOS directory attribute bit (a sub-directory entry).
const FA_SUBDIR: u8 = 0x10;

/// Relocate the NRSC blob already sitting in `SYS.buf`, tag its 3D flags, cache
/// and return the FSELECTR tree — or null if the header does not validate
/// (without caching). Shared by the disk-override and embedded-fallback paths.
///
/// `len` is how many bytes of `buf` the blob really occupies: the bytes read
/// from the override file, or the size of the embedded blob. `read_file` stops
/// at [`GEM_CAP`] without saying so, so an override larger than the buffer
/// arrives truncated — and must be refused rather than relocated past what was
/// read (EmuTOS `rs_readit`, `aes/gemrslib.c:782-783`).
fn relocate_and_cache(buf: &mut [u8; GEM_CAP], len: usize) -> *mut Object {
    // SAFETY: buf holds `len` bytes of a candidate NRSC blob; relocate
    // validates the header against `len` and rewrites the byte offsets to
    // absolute pointers in place.
    if unsafe { resource::relocate_blob(buf.as_mut_ptr(), len) } {
        let root = unsafe { resource::tree_root(buf.as_mut_ptr(), 0) };
        fs_start(root);
        // SAFETY: single-CPU AES.
        unsafe { *SYS.root.get() = root };
        return root;
    }
    core::ptr::null_mut()
}

/// Obtain the FSELECTR tree (or null). Tries a disk override first, then falls
/// back to the built-in resource:
///
/// 1. **Disk override** — if `C:\GEMSYS\RSC\GEM.RSC` exists (a native NRSC blob,
///    same format `mkrsc.py` emits), load and use it. This lets a user replace
///    the file-selector resource — a different design, more languages — without
///    rebuilding the kernel. Only accepted if it relocates to a valid tree.
/// 2. **Embedded fallback** — otherwise use [`GEM_FSEL_RSC`], the `gem.rsc`
///    compiled into the kernel (EmuTOS links its `gem.rsc` into ROM the same
///    way). The always-present baseline: the selector works on a bare card.
fn load_fselectr() -> *mut Object {
    // SAFETY: single-CPU AES.
    let cached = unsafe { *SYS.root.get() };
    if !cached.is_null() {
        return cached;
    }
    // SAFETY: single-CPU AES; exclusive owner of the buffer.
    let buf = unsafe { &mut *SYS.buf.get() };

    // 1. Disk override.
    if let Some(len) = crate::bdos::fs::read_file("C:\\GEMSYS\\RSC\\GEM.RSC", buf) {
        if len > 0 {
            let root = relocate_and_cache(buf, len);
            if !root.is_null() {
                return root;
            }
            // Present but invalid — fall through to the embedded blob below.
        }
    }

    // 2. Embedded fallback. The `const _` assert guarantees the fit.
    buf[..GEM_FSEL_RSC.len()].copy_from_slice(GEM_FSEL_RSC);
    relocate_and_cache(buf, GEM_FSEL_RSC.len())
}

/// Initialise the file selector — EmuTOS `fs_start` (`aes/gemfslib.c:59-172`),
/// the `CONF_WITH_3D_OBJECTS` branch.
///
/// The FSELECTR resource ships **flat**: it carries no 3D flags and its
/// coordinates leave no room for a bevel. `fs_start` is the one-shot pass that
/// turns it into the 3D dialog — it tags each object with its 3D kind *and*
/// moves/resizes it, because `just_draw` renders a 3D object two pixels larger
/// on every side than it measures (`gemoblib.c:504-509`). Flags without the
/// geometry give exactly the half-3D look A64 reports: bevels sitting on the
/// flat coordinates, overlapping their neighbours instead of the surrounding
/// boxes growing with them.
///
/// **The `#if` is decided, not forgotten.** EmuTOS gates this whole function
/// on `CONF_WITH_3D_OBJECTS` and has an `#else` branch that instead equalises
/// the slider widths to `gl_wbox` for the flat look. NewTOS builds the
/// Falcon030/TOS4 profile, where `CONF_WITH_3D_OBJECTS` is 1 (spec
/// `docs/api/aes/resource-fileselector.md`, "Build-time switches"), so only
/// the 3D branch exists here; the flat branch is deliberately not carried.
///
/// EmuTOS's closing `ob_center(tree, &gl_rfs)` (`:152`) is **not** here: NewTOS
/// loads the tree lazily on the first `fsel_input` rather than once at AES
/// start-up, so the centring runs per call in [`center`], which returns the
/// `gl_rfs` rectangle `fm_dial` needs.
///
/// Runs exactly once per tree: `relocate_and_cache` calls it on the successful
/// relocation, right before the root is cached, and every later
/// `load_fselectr` returns that cached root. The adjustments are cumulative,
/// so a second call would double them.
fn fs_start(tree: *mut Object) {
    // SAFETY: `tree` is a relocated FSELECTR root; every index below is one of
    // its 53 objects (checked against the converted resource).
    unsafe {
        let ob = |i: i16| &mut *tree.add(i as usize);

        // Objects inside FILEAREA (the IBOX holding the closer, the title, the
        // scroll bar and the filenames), plus FILEAREA itself (`:66-110`).
        let o = ob(FCLSBOX);
        o.ob_flags |= FL3DACT;
        o.ob_x += ADJ3DSTD;
        o.ob_y += ADJ3DSTD;

        let o = ob(FTITLE);
        o.ob_flags |= FL3DBAK;
        o.ob_x += 2 * ADJ3DSTD - 1;
        o.ob_height += 2 * ADJ3DSTD;
        // "use pattern 4, just like TOS4" (`:81-82`): rewrite only the inside
        // pattern nibble of the TEDINFO colour word.
        let spec = o.ob_spec;
        if spec != 0 {
            let ted = &mut *(spec as *mut TedInfo);
            ted.te_color = ((ted.te_color as u16 & 0xff8f) | IP_4PATT << 4) as i16;
        }

        let o = ob(FILEBOX);
        o.ob_y += 3 * ADJ3DSTD - 1;
        o.ob_width += ADJ3DSTD;

        let o = ob(SCRLBAR);
        o.ob_x += 2 * ADJ3DSTD - 1;
        o.ob_y += 3 * ADJ3DSTD - 1;

        let o = ob(FUPAROW);
        o.ob_flags |= FL3DACT;
        o.ob_x += ADJ3DSTD;
        o.ob_y += ADJ3DSTD;
        o.ob_width -= 2 * ADJ3DSTD;

        let o = ob(FSVSLID);
        o.ob_y += 3 * ADJ3DSTD;
        o.ob_height -= 6 * ADJ3DSTD;
        // The slider track takes pattern 4 as well (`:98`) — here the colour
        // word is the low half of the box `ob_spec`, so the mask keeps the
        // thickness byte above it.
        o.ob_spec = (o.ob_spec & 0xffff_ff8f) | (u64::from(IP_4PATT) << 4);

        // Only x/w for the elevator: y/h are set per list in `fs_format`
        // (`:99-101`).
        let o = ob(FSVELEV);
        o.ob_flags |= FL3DACT;
        o.ob_x += ADJ3DSTD;
        o.ob_width -= 2 * ADJ3DSTD;

        let o = ob(FDNAROW);
        o.ob_flags |= FL3DACT;
        o.ob_x += ADJ3DSTD;
        o.ob_y -= ADJ3DSTD;
        o.ob_width -= 2 * ADJ3DSTD;

        let o = ob(FILEAREA);
        o.ob_y -= 6; // "squeeze things together vertically" (`:108`)
        o.ob_width += 2 * ADJ3DSTD - 1;
        o.ob_height += 3 * ADJ3DSTD - 1;

        // The drive-letter boxes and their IBOX (`:117-131`): each box moves
        // right by 2*ADJ3DSTD and down by 2*ADJ3DSTD plus a growing per-row
        // offset, the row counter restarting at the top of every column.
        let mut row = 0i16;
        for d in FS1STDRV..=FSLSTDRV {
            if row >= DRIVE_ROWS {
                row = 0;
            }
            let o = ob(d);
            o.ob_flags |= FL3DIND;
            o.ob_x += 2 * ADJ3DSTD;
            o.ob_y += 2 * ADJ3DSTD + row * (3 * ADJ3DSTD - 1);
            row += 1;
        }
        let o = ob(FSDRIVES);
        o.ob_height += 2 * ADJ3DSTD + DRIVE_ROWS * (3 * ADJ3DSTD - 1);
        o.ob_width += 4 * ADJ3DSTD;

        // The remaining ROOT children, plus ROOT (`:133-150`). FSOK/FSCANCEL
        // are re-aligned on FILEAREA — left edge and right edge — so they keep
        // clear of FSDRIVES in low resolutions.
        ob(FSDIRECT).ob_flags |= FL3DBAK;
        ob(FSSELECT).ob_flags |= FL3DBAK;
        ob(FSDRVTXT).ob_flags |= FL3DBAK;

        let (area_x, area_w) = (ob(FILEAREA).ob_x, ob(FILEAREA).ob_width);
        let o = ob(FSOK);
        o.ob_flags |= FL3DACT;
        o.ob_x = area_x;
        o.ob_y += ADJ3DSTD;
        let ok_y = o.ob_y;

        let o = ob(FSCANCEL);
        o.ob_flags |= FL3DACT;
        o.ob_x = area_x + area_w - o.ob_width;
        o.ob_y = ok_y;

        let o = ob(ROOT);
        o.ob_flags |= FL3DBAK;
        o.ob_height += 2 * ADJ3DSTD;
    }
}

/// Disable drive-letter gadgets for drives that are not present, and mark
/// the current drive `cur` (`A`..`Z`) selected — like the EmuTOS selector.
fn set_drive_states(tree: *mut Object, cur: u8) {
    let bits = crate::bios::drive::drvbits();
    let cur_idx = if cur.is_ascii_uppercase() { (cur - b'A') as i16 } else { 2 };
    // SAFETY: valid FSELECTR tree.
    unsafe {
        for d in FS1STDRV..=FSLSTDRV {
            let idx = d - FS1STDRV; // 0 = A … 25 = Z
            let o = &mut *tree.add(d as usize);
            o.ob_state &= !(DISABLED | SELECTED);
            if bits & (1 << idx) == 0 {
                o.ob_state |= DISABLED;
            } else if idx == cur_idx {
                o.ob_state |= SELECTED;
            }
        }
    }
}

/// Convert a plain filename into the **raw** form an `FFFFFFFF.FFF` TEDINFO
/// template expects — EmuTOS `fmt_str` (`util/optimize.c:43-69`): at most 8
/// characters before the first dot (excess eaten), then, if an extension is
/// present, blank-fill to 8 and copy at most 3 more. The dot itself is *not*
/// stored: it is a literal in the template, which `ob_format` supplies.
///
/// `SAMPLE.PRG` → `SAMPLE  PRG`, `TESTPROG.C` → `TESTPROGC`, `TEST` → `TEST`.
/// `out` must hold 12 bytes (11 + NUL); the result is always NUL-terminated.
fn fmt_str(instr: &[u8], out: &mut [u8]) {
    for b in out.iter_mut() {
        *b = 0;
    }
    let n = instr.iter().position(|&b| b == 0).unwrap_or(instr.len());
    let mut q = 0usize;
    let mut p = 0usize;
    // Copy up to 8 bytes before the (first) dot; eat the excess.
    while p < n {
        let c = instr[p];
        p += 1;
        if c == b'.' {
            break;
        }
        if q < 8 {
            out[q] = c;
            q += 1;
        }
    }
    // Any extension present: pad to column 8, then copy up to 3 more bytes.
    if p < n {
        while q < 8 {
            out[q] = b' ';
            q += 1;
        }
        while q < 11 && p < n {
            out[q] = instr[p];
            q += 1;
            p += 1;
        }
    }
    out[q] = 0;
}

/// The reverse of [`fmt_str`] — EmuTOS `unfmt_str` (`util/optimize.c:77-96`):
/// the first 8 columns with their blanks squeezed out, then a `.` and the
/// remainder if any. `SAMPLE  PRG` → `SAMPLE.PRG`.
fn unfmt_str(instr: &[u8], out: &mut [u8]) {
    for b in out.iter_mut() {
        *b = 0;
    }
    let n = instr.iter().position(|&b| b == 0).unwrap_or(instr.len());
    let mut w = 0usize;
    let mut p = 0usize;
    while p < n && p < 8 {
        let c = instr[p];
        p += 1;
        if c != b' ' && w + 1 < out.len() {
            out[w] = c;
            w += 1;
        }
    }
    if p < n {
        if w + 1 < out.len() {
            out[w] = b'.';
            w += 1;
        }
        while p < n && w + 1 < out.len() {
            out[w] = instr[p];
            w += 1;
            p += 1;
        }
    }
    out[w] = 0;
}

/// Point object `idx`'s TEDINFO text at `ptr` — and change **nothing else**.
///
/// The GEM.RSC file-selector fields are `G_FTEXT`/`G_FBOXTEXT`: their visible
/// content is `ob_format(te_ptext, te_ptmplt)` (`gemoblib.c:718-720`), the raw
/// string merged into a template that carries the literals — the `Selection:`
/// caption, the `.` between name and extension, and the underscores that *are*
/// the field's visible extent. EmuTOS keeps that template and stores the raw
/// form in `te_ptext` (`fs_input`/`fs_format` via `inf_sset` and `fmt_str`,
/// `gemfslib.c:685-689,365-376`).
///
/// NewTOS keeps its own buffers (it needs a path longer than the field for the
/// list + mask split), so this repoints `te_ptext` at them — and leaves
/// `ob_type`, `te_ptmplt`, `te_pvalid`, `te_txtlen`, `te_color` and `te_just`
/// exactly as the resource has them. The buffers hold the **raw** form
/// (`fmt_str` for the 8.3 fields), which is what the template expects, so
/// `just_draw` draws what EmuTOS draws and `edit_field` gets the template and
/// validation codes it is written against (`find_pos`/`check`).
///
/// A63: the earlier version down-converted the fields to `G_BOXTEXT`/`G_TEXT`
/// and cleared `te_ptmplt`/`te_pvalid`, which is what dropped the underscores,
/// the `Selection:` caption and the 8.3 column alignment all at once.
///
/// `cap` is the caller's buffer size; a field is only repointed when the buffer
/// can hold the `te_txtlen` the resource declares, because every writer bounds
/// itself by `te_txtlen` (`ins_char`, EmuTOS `gemobed.c:117`). All three
/// call sites satisfy this (96/16/16 against 39/12/13), so the guard never
/// fires — it is here so a future resource change cannot silently turn into an
/// out-of-bounds write. The `te_txtlen` override that used to stand here was
/// the A37 work-around: it is no longer needed now that the converted blob
/// reserves the declared length behind `te_ptext`
/// (`tests/aes_rsrc_tedinfo_reserve.rs`) — but our buffers, not the blob's, are
/// what has to be big enough.
fn set_ted(tree: *mut Object, idx: i16, ptr: *mut u8, cap: usize) {
    // SAFETY: idx is a valid TEDINFO-bearing object in the tree.
    unsafe {
        let spec = (*tree.add(idx as usize)).ob_spec;
        if spec == 0 {
            return;
        }
        let t = &mut *(spec as *mut TedInfo);
        if (t.te_txtlen.max(1) as usize) <= cap {
            t.te_ptext = ptr as u64;
        }
    }
}

/// Copy the caller's NUL-terminated string at `src` (in `space`, the
/// caller's — ARCH03 3a) into `dst`, at most `dst.len() - 1` bytes, the rest
/// zeroed. `src == 0` leaves `dst` empty. `Err` — fault recorded — when the
/// program cannot read the string; the call must stop.
fn copy_in(space: Space, dst: &mut [u8], src: u64) -> Result<(), UserFault> {
    for b in dst.iter_mut() {
        *b = 0;
    }
    if src == 0 {
        return Ok(());
    }
    let max = dst.len() - 1;
    space.read_cstr(src, &mut dst[..max]).map(|_| ())
}

/// Write the NUL-terminated string in `src` back into the caller's buffer at
/// `dst` (in `space`). `dst == 0` writes nothing. `Err` — fault recorded —
/// when the program cannot write it.
fn write_out(space: Space, dst: u64, src: &[u8]) -> Result<(), UserFault> {
    if dst == 0 {
        return Ok(());
    }
    let n = src.iter().position(|&b| b == 0).unwrap_or(src.len());
    space.copy_to(dst, &src[..n])?;
    space.put_u8(dst + n as u64, 0)
}

/// `fsel_input` (90) — no caller title, so FSTITLE shows the default
/// "ITEM SELECTOR" (EmuTOS `fs_input(..., NULL)`, gemsuper.c:323).
pub fn fsel_input(pb: &Aespb) {
    run(pb, 0);
}

/// `fsel_exinput` (91) — `fsel_input` plus a caller-supplied dialog title in
/// `addr_in[2]` (`FS_TITLE`, gemsuper.c:326 / aesdefs.h:464), shown centred in
/// the FSTITLE caption. A null title falls back to the default "ITEM SELECTOR".
pub fn fsel_exinput(pb: &Aespb) {
    run(pb, pb.addrin(2));
}

/// The shared body of `fsel_input`/`fsel_exinput`. The caller's path,
/// selection and title strings are read — and the path and selection written
/// back — in the caller's space (ARCH03 3a, [`copy_in`]/[`write_out`]); one
/// the program cannot use ends the call (and the program) before the dialog
/// is drawn, or — for the write-back — after it was taken down.
fn run(pb: &Aespb, label: u64) {
    let space = pb.space();
    let path_p = pb.addrin(0);
    let sel_p = pb.addrin(1);

    // EmuTOS `fs_input`: bail out immediately on a NULL path pointer
    // (gemfslib.c:630-631; spec resource-fileselector.md:328,411,447). The
    // dialog never opens: int_out[0] = 0 (did not run), int_out[1] = 0.
    if path_p == 0 {
        pb.set_intout(1, 0);
        pb.set_ret(0);
        return;
    }

    let tree = load_fselectr();
    let Some(backend) = host().and_then(|h| h.raster()) else {
        pb.set_intout(1, 0);
        pb.set_ret(0);
        return;
    };
    if tree.is_null() {
        pb.set_intout(1, 0);
        pb.set_ret(0);
        return;
    }

    // Copy inputs into our buffers and point the dialog's TEDINFOs at them
    // (short, non-overlapping borrows for the single-CPU AES).
    // SAFETY: single-CPU AES; the resource TEDINFOs are distinct memory.
    unsafe {
        if copy_in(space, &mut *SYS.dir.get(), path_p).is_err() {
            return;
        }
        // EmuTOS `fs_input`: an empty path defaults to "<curdrive>:\*.*"
        // (`strcpy(pipath,"A:\\*.*"); *pipath += dos_gdrv()`,
        // gemfslib.c:634-638; spec resource-fileselector.md:446). The default
        // is written back to the caller on exit (via `write_out` below).
        {
            let dir = &mut *SYS.dir.get();
            if dir[0] == 0 {
                let drv = b'A' + crate::bdos::fs::drive::current_drive() as u8;
                dir[..6].copy_from_slice(&[drv, b':', b'\\', b'*', b'.', b'*']);
            }
        }
        // The FSSELECT field is `FFFFFFFF.FFF`-templated, so its buffer holds
        // the **raw** (dot-less, blank-padded) form — EmuTOS `fs_input`:
        // `fmt_str(pisel, selname); inf_sset(tree, FSSELECT, selname)`
        // (gemfslib.c:688-689). The caller's plain name is converted here and
        // converted back on the way out.
        {
            let mut plain = [0u8; 32];
            if copy_in(space, &mut plain, sel_p).is_err() {
                return;
            }
            fmt_str(&plain, &mut *SYS.sel.get());
        }
        set_ted(tree, FSDIRECT, (*SYS.dir.get()).as_mut_ptr(), 96);
        set_ted(tree, FSSELECT, (*SYS.sel.get()).as_mut_ptr(), 16);
        let names = &mut *SYS.names.get();
        for i in 0..NM_NAMES {
            set_ted(tree, F1NAME + i as i16, names[i].as_mut_ptr(), 16);
        }
    }

    // Centre *before* listing so fs_format draws the box at its final spot.
    let cur = unsafe { (*SYS.dir.get())[0] };
    set_drive_states(tree, cur);
    // `gl_rfs` — the rectangle the dialog occupies, from `ob_center`
    // (EmuTOS `fs_start`, gemfslib.c:153,158).
    let gl_rfs = center(tree);
    // EmuTOS `fs_input` sets the dialog title — the caller's custom label or the
    // default "ITEM SELECTOR" — and centres it (gemfslib.c:691-693).
    if !apply_title(tree, space, label) {
        return;
    }
    relist(tree); // read the directory, fill the slots + slider
    // "set clip and start form fill-in by drawing the form" — EmuTOS
    // `fm_dial(FMD_START, &gl_rcenter, &gl_rfs)` immediately before the first
    // `ob_draw` (gemfslib.c:707-709).
    let gl_rcenter = gl_rcenter();
    fm_dial(FMD_START, gl_rcenter, gl_rfs);
    // The FSELECTR tree is loaded from GEM.RSC as native (LP64) `Object`s, so it
    // must be read at the LP64 width even when an ILP32 app is current — force
    // the accessor's ABI here rather than reading `current_abi()`.
    object::draw_root(ObjTree::lp64(tree as *const Object), MAX_DEPTH, backend);

    // `fm_do(tree, FSSELECT)` (`aes/gemfslib.c:718`) runs the dialog between
    // `fm_own(TRUE)` and `fm_own(FALSE)` (`aes/gemfmlib.c:287,349`): the
    // screen lock, and the mouse — and keyboard — for the caller, so a press
    // in the selector goes to its owner and not to another process's parked
    // `MU_BUTTON` (A118). EmuTOS takes them around each `fm_do` round; this
    // loop gives the CPU to no one between its rounds, so one bracket around
    // it is the same. The dispatcher waited for the lock before the dialog
    // was drawn ([`super::window::update_block`]).
    super::window::fm_own(true);
    let button = modal(tree, backend);
    super::window::fm_own(false);

    // Write the (possibly updated) path + selection back to the caller.
    // SAFETY: single-CPU AES.
    // A buffer the program cannot write records the fault; the dialog is
    // still taken down below, then the syscall tail ends the program.
    unsafe {
        let _ = write_out(space, path_p, &*SYS.dir.get());
        // "unfmt_str(ad_fname, selname); strcpy(pisel, selname)"
        // (gemfslib.c:915-916): the caller gets the plain name back, dot
        // restored, not the template-raw form the field carries.
        let mut plain = [0u8; 16];
        unfmt_str(&*SYS.sel.get(), &mut plain);
        let _ = write_out(space, sel_p, &plain);
    }
    // "start the redraw" — EmuTOS `fm_dial(FMD_FINISH, &gl_rcenter, &gl_rfs)`
    // (gemfslib.c:918-919), after the caller's buffers are filled and before the
    // exit button is reported. This is what takes the dialog off the screen:
    // `w_drawdesk(gl_rfs)` repaints the desktop background under it and
    // `w_update(DESKWH, gl_rfs, …)` re-asks every overlapped window for its
    // content. Without it the selector stays visible after the call returns.
    fm_dial(FMD_FINISH, gl_rcenter, gl_rfs);
    pb.set_intout(1, button);
    pb.set_ret(1);
}

/// Read the current directory (`SYS.dir`) into the full list, sort it
/// (EmuTOS `fs_active`: folders first, then files, both alphabetical),
/// reset the scroll position, and refill the visible slots + slider.
///
/// `list_dir` writes directly into the static `SYS` arrays (no large stack
/// buffer), so `MAX_LIST` can be raised without risking the kernel stack.
fn relist(tree: *mut Object) {
    // SAFETY: single-CPU AES; `dir` and `all`/`all_attr` are distinct cells.
    let n = unsafe {
        let dir = &*SYS.dir.get();
        let dlen = dir.iter().position(|&b| b == 0).unwrap_or(dir.len());
        let dpath = core::str::from_utf8(&dir[..dlen]).unwrap_or("C:\\*.*");
        let all = &mut *SYS.all.get();
        let all_attr = &mut *SYS.all_attr.get();
        crate::bdos::fs::find::list_dir(dpath, all, all_attr)
    };
    sort_entries(n);
    // SAFETY: single-CPU AES.
    unsafe {
        *SYS.count.get() = n;
        *SYS.top.get() = 0;
    }
    fs_format(tree);
}

/// Sort the first `n` directory entries in place — folders before files,
/// then alphabetically. This is EmuTOS `fs_active`'s shell sort
/// (`gemfslib.c:312-326`) driven by the folder/file prefix key from
/// `fs_add` (`gemfslib.c:254`) and `strcmp` from `fs_comp`
/// (`gemfslib.c:243`), applied to our two parallel `SYS` arrays.
fn sort_entries(n: usize) {
    // SAFETY: single-CPU AES.
    unsafe {
        let all = &mut *SYS.all.get();
        let attr = &mut *SYS.all_attr.get();
        let mut gap = n / 2;
        while gap > 0 {
            let mut i = gap;
            while i < n {
                let mut j = i - gap;
                loop {
                    if entry_cmp(&all[j], attr[j], &all[j + gap], attr[j + gap])
                        != core::cmp::Ordering::Greater
                    {
                        break;
                    }
                    all.swap(j, j + gap);
                    attr.swap(j, j + gap);
                    if j < gap {
                        break;
                    }
                    j -= gap;
                }
                i += 1;
            }
            gap /= 2;
        }
    }
}

/// Compare two list entries by EmuTOS's sort key: the folder/file prefix
/// (`\007` for a sub-directory, `' '` for a file, so folders sort lowest —
/// `fs_add`, `gemfslib.c:254`) then the display name via `strcmp`
/// (`fs_comp`, `gemfslib.c:243`). Names are NUL-padded, so a plain array
/// compare is `strcmp`-equivalent.
fn entry_cmp(na: &[u8; 16], aa: u8, nb: &[u8; 16], ab: u8) -> core::cmp::Ordering {
    let pa = if aa & FA_SUBDIR != 0 { 0x07u8 } else { b' ' };
    let pb = if ab & FA_SUBDIR != 0 { 0x07u8 } else { b' ' };
    pa.cmp(&pb).then_with(|| na.cmp(nb))
}

/// Put the currently scrolled window of filenames into the nine slot
/// TEDINFOs, size and place the elevator, and redraw the file box — EmuTOS
/// `fs_format` (`aes/gemfslib.c:359-396`).
///
/// A63: each slot's string is the **raw** form its `_ ________.___ ` template
/// expects, not the plain name — the folder/file indicator byte from `fs_add`
/// (`\007` for a directory, blank for a file, `:254`) followed by `fmt_str` of
/// the name (`:371-372`). `ob_format` then merges it into the template, which
/// supplies the `.` and pads the unused slots with underscores; that is what
/// gives the list its fixed 8.3 columns and turns a slot past the end of the
/// listing into a row of underscores. A slot past the end gets a single blank
/// (`:376-377`), i.e. the indicator only.
///
/// EmuTOS also (re)asserts `ob_type = G_FBOXTEXT` and `ob_state = NORMAL` on
/// every slot here (`:380-381`); the state half is what clears a previous
/// selection.
fn fs_format(tree: *mut Object) {
    // SAFETY: single-CPU AES.
    unsafe {
        let all = &*SYS.all.get();
        let attr = &*SYS.all_attr.get();
        let top = *SYS.top.get();
        let count = *SYS.count.get();
        let names = &mut *SYS.names.get();
        for (i, n) in names.iter_mut().enumerate() {
            *n = [0; 16];
            if top + i < count {
                let e = top + i;
                n[0] = if attr[e] & FA_SUBDIR != 0 { 0x07 } else { b' ' };
                fmt_str(&all[e], &mut n[1..13]);
            } else {
                n[0] = b' ';
            }
        }
        for i in 0..NM_NAMES as i16 {
            let o = &mut *tree.add((F1NAME + i) as usize);
            o.ob_type = G_FBOXTEXT;
            o.ob_state &= !SELECTED;
        }
    }
    size_elevator(tree);
    if let Some(backend) = host().and_then(|h| h.raster()) {
        // LP64 accessor for the kernel-built FSELECTR tree (see `run`).
        let t = ObjTree::lp64(tree as *const Object);
        object::draw_obj_t(t, FILEBOX, MAX_DEPTH, backend);
        object::draw_obj_t(t, FSVSLID, MAX_DEPTH, backend);
    }
}

/// Size and position the slider thumb (FSVELEV) for the current scroll state
/// — the second half of EmuTOS `fs_format` (`gemfslib.c:383-396`).
///
/// The `CONF_WITH_3D_OBJECTS` block at the end (`:388-391`) is the counterpart
/// of [`fs_start`]'s FSVELEV adjustment: `fs_start` narrows the thumb by the
/// bevel horizontally, this insets it vertically (`y += ADJ3DSTD`,
/// `h -= 2*ADJ3DSTD`). Without it a full-track thumb would be drawn two pixels
/// above and below its track, since a 3D object renders `ADJ3DSTD` larger on
/// every side. EmuTOS notes the assumption the subtraction rests on: twice the
/// adjustment is smaller than the `gl_hbox` floor the height is clamped to.
fn size_elevator(tree: *mut Object) {
    // SAFETY: single-CPU AES; valid tree.
    unsafe {
        let count = *SYS.count.get();
        let track_h = (*tree.add(FSVSLID as usize)).ob_height;
        let mut y = 0i16;
        let mut h = track_h;
        if count > NM_NAMES {
            h = (track_h * NM_NAMES as i16 / count as i16).max(gl_hbox());
            y = (track_h - h) * *SYS.top.get() as i16 / (count - NM_NAMES) as i16;
        }
        let thumb = &mut *tree.add(FSVELEV as usize);
        thumb.ob_y = y + ADJ3DSTD;
        thumb.ob_height = h - 2 * ADJ3DSTD;
    }
}

/// Scroll the list by `delta` rows (clamped); reformat if it moved.
fn scroll(tree: *mut Object, delta: i32) {
    // SAFETY: single-CPU AES.
    let moved = unsafe {
        let count = *SYS.count.get();
        if count <= NM_NAMES {
            false
        } else {
            let maxtop = (count - NM_NAMES) as i32;
            let old = *SYS.top.get();
            let new = (old as i32 + delta).clamp(0, maxtop) as usize;
            *SYS.top.get() = new;
            new != old
        }
    };
    if moved {
        fs_format(tree);
    }
}

/// Navigate the directory path into a sub-directory `name` from the list
/// (EmuTOS fsel folder handling): insert the folder name before the mask.
/// `.` is a no-op; `..` walks one level up (never past the drive root).
fn nav_into_folder(name: &[u8]) {
    let nlen = name.iter().position(|&b| b == 0).unwrap_or(name.len());
    let nm = &name[..nlen];
    if nm == b"." {
        return;
    }
    // SAFETY: single-CPU AES.
    unsafe {
        let dir = &mut *SYS.dir.get();
        let dlen = dir.iter().position(|&b| b == 0).unwrap_or(dir.len());
        // Split into the directory prefix (up to & incl. the last '\') and
        // the mask (the final path component).
        let mask_start = match dir[..dlen].iter().rposition(|&b| b == b'\\') {
            Some(p) => p + 1,
            None => dlen,
        };
        let mut mask = [0u8; 16];
        let mk = (dlen - mask_start).min(15);
        mask[..mk].copy_from_slice(&dir[mask_start..mask_start + mk]);

        let mut prefix = [0u8; 96];
        prefix[..mask_start].copy_from_slice(&dir[..mask_start]);
        let mut plen = mask_start;

        if nm == b".." {
            // Walk up: drop the trailing '\', then back to the previous '\',
            // but never before the drive root ("X:\").
            if plen >= 1 && prefix[plen - 1] == b'\\' {
                plen -= 1;
            }
            let root = if plen >= 2 && prefix[1] == b':' { 3 } else { 0 };
            while plen > root && prefix[plen - 1] != b'\\' {
                plen -= 1;
            }
        } else {
            // Walk down: append the folder name + '\'.
            for &c in nm {
                if plen < 95 {
                    prefix[plen] = c;
                    plen += 1;
                }
            }
            if plen < 95 {
                prefix[plen] = b'\\';
                plen += 1;
            }
        }

        // Rebuild dir = prefix + mask.
        *dir = [0; 96];
        dir[..plen].copy_from_slice(&prefix[..plen]);
        let mut w = plen;
        for i in 0..mk {
            if mask[i] == 0 {
                break;
            }
            if w < 95 {
                dir[w] = mask[i];
                w += 1;
            }
        }
    }
}

/// Whether the directory field (`SYS.dir`, edited in place) now differs
/// from the path last listed (EmuTOS `path_changed` — a `strncmp`, i.e. the
/// logical NUL-terminated strings, not the raw buffers, so a shortened path
/// with stale tail bytes past its NUL still compares equal).
fn path_differs(listed: &[u8; 96]) -> bool {
    // SAFETY: single-CPU AES.
    let dir = unsafe { &*SYS.dir.get() };
    let dlen = dir.iter().position(|&b| b == 0).unwrap_or(dir.len());
    let llen = listed.iter().position(|&b| b == 0).unwrap_or(listed.len());
    dlen != llen || dir[..dlen] != listed[..llen]
}

/// Set the directory buffer to `X:\*.*` for a clicked drive letter.
fn set_drive(ch: u8) {
    // SAFETY: single-CPU AES.
    unsafe {
        let dir = &mut *SYS.dir.get();
        *dir = [0; 96];
        dir[..6].copy_from_slice(&[ch, b':', b'\\', b'*', b'.', b'*']);
    }
}

/// Draw the file selector on `C:\*.*` **without** the modal loop — for a
/// non-interactive showcase. Returns once the populated dialog is drawn.
pub fn preview() {
    let tree = load_fselectr();
    let Some(backend) = host().and_then(|h| h.raster()) else {
        return;
    };
    if tree.is_null() {
        return;
    }
    // SAFETY: single-CPU AES.
    unsafe {
        let dir = &mut *SYS.dir.get();
        *dir = [0; 96];
        dir[..6].copy_from_slice(b"C:\\*.*");
        set_ted(tree, FSDIRECT, dir.as_mut_ptr(), 96);
        set_ted(tree, FSSELECT, (*SYS.sel.get()).as_mut_ptr(), 16);
        let names = &mut *SYS.names.get();
        for i in 0..NM_NAMES {
            set_ted(tree, F1NAME + i as i16, names[i].as_mut_ptr(), 16);
        }
    }
    set_drive_states(tree, b'C');
    center(tree);
    relist(tree);
    // LP64 accessor for the kernel-built FSELECTR tree (see `run`).
    object::draw_root(ObjTree::lp64(tree as *const Object), MAX_DEPTH, backend);
}

/// Centre the FSELECTR root on screen and return its **outer** rectangle —
/// EmuTOS `fs_start`: `ob_center(tree, &gl_rfs)` (`gemfslib.c:153,158`). The
/// returned rect is `gl_rfs`, the area `fm_dial` puts up and takes down again
/// (`gemfslib.c:707-708,919`); it includes the 3 px `OUTLINED` ring of the
/// FSELECTR root (`gem_rsc.c:169-173`), which the ROOT box alone does not cover.
fn center(tree: *mut Object) -> (i16, i16, i16, i16) {
    // The FSELECTR tree is native (LP64) `Object`s from GEM.RSC — read it at the
    // LP64 width even while an ILP32 app is current (see `run`).
    super::form::ob_center(ObjTree::lp64(tree as *const Object)).unwrap_or((0, 0, 0, 0))
}

/// Set the FSTITLE caption and centre it — EmuTOS `fs_input`
/// (`gemfslib.c:691-693`): `obj->ob_spec = pilabel ? pilabel : rs_str(ITEMSLCT)`
/// then `centre_title(tree, FSTITLE)`. The caller's custom `label` (copied into
/// our own buffer so the tree never holds a raw EL0 pointer) takes precedence;
/// otherwise the default is the `ITEMSLCT` free string read from the loaded
/// GEM.RSC blob (the `rs_str` analog). The tree is cached across calls, so
/// ob_spec is rewritten every call — a prior custom title never leaks into a
/// later default-title dialog.
///
/// `label` is in the caller's `space` (ARCH03 3a); `false` — fault recorded,
/// the call must stop — when the program cannot read it.
fn apply_title(tree: *mut Object, space: Space, label: u64) -> bool {
    // SAFETY: single-CPU AES; valid FSELECTR tree; SYS.buf holds the blob.
    unsafe {
        let title_ptr: *const u8 = if label != 0 {
            if copy_in(space, &mut *SYS.title.get(), label).is_err() {
                return false;
            }
            (*SYS.title.get()).as_ptr()
        } else {
            resource::blob_frstr(SYS.buf.get() as *const u8, ITEMSLCT)
        };
        (*tree.add(FSTITLE as usize)).ob_spec = title_ptr as u64;
        centre_title(tree, FSTITLE);
    }
    true
}

/// Centre the `objnum` G_STRING caption horizontally in the root — EmuTOS
/// `centre_title` (`gemfslib.c:179-185`): `ob_x = (root.ob_width -
/// strlen(ob_spec)*gl_wchar) / 2`. Unlike EmuTOS (whose G_STRING draw is not
/// width-clipped) NewTOS clips a string's glyphs to `ob_width`
/// (`object::draw_text`, the icon-label behaviour), so we also widen the object
/// to the title's pixel width; the visible result — a fully drawn, centred
/// caption — matches EmuTOS.
fn centre_title(tree: *mut Object, objnum: i16) {
    // SAFETY: single-CPU AES; valid tree; `objnum` is a G_STRING child of ROOT.
    unsafe {
        let root_w = (*tree).ob_width;
        let ob = &mut *tree.add(objnum as usize);
        // strlen(ob_spec): the title is our own buffer or a GEM.RSC frstr
        // string, both NUL-terminated (capped like `object::read_cstr` at 255).
        let tw = if ob.ob_spec == 0 {
            0
        } else {
            let p = ob.ob_spec as *const u8;
            let mut n = 0i16;
            while n < 255 && *p.add(n as usize) != 0 {
                n += 1;
            }
            n * crate::vdi::font::CELL_W
        };
        ob.ob_x = (root_w - tw) / 2;
        ob.ob_width = tw;
    }
}

/// Whether the directory path (`SYS.dir`, path+mask) is at the drive root, so
/// FCLSBOX up-navigation must do nothing — EmuTOS `fs_back`/FCLSBOX
/// (`gemfslib.c:840-844`): stop if there is no path separator (`pstr == locstr`)
/// or the character before the last separator is the drive colon (`X:\`).
fn at_drive_root() -> bool {
    // SAFETY: single-CPU AES.
    let dir = unsafe { &*SYS.dir.get() };
    let dlen = dir.iter().position(|&b| b == 0).unwrap_or(dir.len());
    match dir[..dlen].iter().rposition(|&b| b == b'\\') {
        None => true,                  // no separator (locstr like "*.*")
        Some(0) => true,               // separator at the very start
        Some(p) => dir[p - 1] == b':', // char before last separator is ':'
    }
}

/// Modal loop (EmuTOS `fs_input`): the `FSDIRECT` path field and `FSSELECT`
/// selection field are keyboard-editable; typing a new path + Enter re-lists
/// it, a click on a folder walks into it, arrows scroll, a click selects a
/// file (double-click confirms), a drive letter re-lists that drive,
/// `OK`/`Cancel` exit. Returns 1 (OK) or 0 (Cancel).
fn modal(tree: *mut Object, backend: &dyn RasterBackend) -> i16 {
    let Some(h) = host() else {
        return 0;
    };
    let tp = tree as *const Object;
    // The FSELECTR tree is native (LP64) `Object`s from GEM.RSC; read it at the
    // LP64 width even while an ILP32 app is current (see `run`). Thread this one
    // accessor through every edit / hit-test / draw / state helper below.
    let t = ObjTree::lp64(tp);
    // Snapshot the listed path so a manual edit of FSDIRECT is detected.
    // SAFETY: single-CPU AES.
    let mut listed: [u8; 96] = unsafe { *SYS.dir.get() };
    // Focus the selection field for editing (EmuTOS `fm_do(tree, FSSELECT)`).
    let mut focus = FSSELECT;
    let mut idx = object::edit_field_t(t, focus, 0, 0, ED_INIT);

    let mut prev = crate::vdi::mouse_buttons();
    let mut cur_sel: i16 = -1;
    let mut last_hit: i16 = -1;
    let mut last_t: u64 = 0;

    // Re-list the (manually edited) directory path and re-arm editing.
    let apply_path = |tree: *mut Object, focus: &mut i16, idx: &mut i16, listed: &mut [u8; 96]| {
        object::edit_field_t(t, *focus, 0, *idx, ED_END);
        relist(tree);
        // SAFETY: single-CPU AES.
        *listed = unsafe { *SYS.dir.get() };
        object::draw_obj_t(t, FSDIRECT, 0, backend);
        *focus = FSSELECT;
        *idx = object::edit_field_t(t, *focus, 0, 0, ED_INIT);
    };

    loop {
        // Keyboard: route to the focused editable field.
        if let Some(w) = h.console_try_read_key() {
            // `fs_input` runs the dialog through `fm_do` (`aes/gemfslib.c:718`),
            // whose `ev_multi` returning `MU_KEYBD` ends in `ev_rets`
            // (`aes/gemevlib.c:268`): pending button transitions are cleared.
            let _ = super::input::ev_rets();
            match (w & 0xff) as u8 {
                b'\r' | b'\n' => {
                    // Enter: re-list if the path was edited, else confirm.
                    if path_differs(&listed) {
                        cur_sel = -1;
                        apply_path(tree, &mut focus, &mut idx, &mut listed);
                    } else {
                        return 1;
                    }
                }
                b'\t' => {
                    object::edit_field_t(t, focus, 0, idx, ED_END);
                    focus = if focus == FSSELECT { FSDIRECT } else { FSSELECT };
                    idx = object::edit_field_t(t, focus, 0, 0, ED_INIT);
                }
                _ => {
                    // Whole key word (scancode << 8 | ascii) → arrows move the caret.
                    idx = object::edit_field_t(t, focus, w as i16, idx, ED_CHAR);
                }
            }
        }

        let cur = crate::vdi::mouse_buttons();
        if cur & 0x01 != 0 && prev & 0x01 == 0 {
            // `fm_do`'s `ev_multi` returned `MU_BUTTON` on this press; its
            // `ev_rets` (`aes/gemevlib.c:268`) consumed it.
            super::input::ev_consume();
            let (mx, my) = crate::vdi::mouse_position();
            let hit = object::hit_test_t(t, 0, MAX_DEPTH, mx, my);
            let now = h.monotonic_millis();
            if hit == FSOK {
                // Like TOS: if the mask was edited, OK re-lists, not exits.
                if path_differs(&listed) {
                    cur_sel = -1;
                    apply_path(tree, &mut focus, &mut idx, &mut listed);
                } else {
                    return 1;
                }
            } else if hit == FSCANCEL {
                return 0;
            } else if hit == FUPAROW {
                scroll(tree, -1);
            } else if hit == FDNAROW {
                scroll(tree, 1);
            } else if hit == FCLSBOX {
                // EmuTOS FCLSBOX (gemfslib.c:839-848): walk one directory level
                // up, unless already at the drive root ("X:\"). This is the
                // up-navigation control that replaces the (WP7a-filtered) ".."
                // list entry; reuse the existing ".." path-shortening + relist
                // (`nav_into_folder`/`apply_path`), guarded so we never shorten
                // past the drive root.
                if !at_drive_root() {
                    nav_into_folder(b"..");
                    cur_sel = -1;
                    apply_path(tree, &mut focus, &mut idx, &mut listed);
                }
            } else if hit == FSDIRECT || hit == FSSELECT {
                // Click into an editable field → focus it (caret to its end).
                // The caret is an XOR line (`curfld`, A111): take it off
                // before it is put on again, also for the same field.
                object::edit_field_t(t, focus, 0, idx, ED_END);
                focus = hit;
                idx = object::edit_field_t(t, focus, 0, 0, ED_INIT);
            } else if (F1NAME..F1NAME + NM_NAMES as i16).contains(&hit) {
                let slot = (hit - F1NAME) as usize;
                // SAFETY: single-CPU AES.
                let (entry, count) = unsafe { (*SYS.top.get() + slot, *SYS.count.get()) };
                if entry < count {
                    // SAFETY: single-CPU AES; entry < count.
                    let (name, is_dir) = unsafe {
                        ((*SYS.all.get())[entry], (*SYS.all_attr.get())[entry] & FA_SUBDIR != 0)
                    };
                    if is_dir {
                        // Folder: walk into it and re-list (EmuTOS single click).
                        nav_into_folder(&name);
                        cur_sel = -1;
                        apply_path(tree, &mut focus, &mut idx, &mut listed);
                    } else {
                        // File: select it; a double-click confirms (= OK).
                        if cur_sel >= 0 {
                            object::set_state_bit_t(t, cur_sel, SELECTED, false);
                        }
                        object::set_state_bit_t(t, hit, SELECTED, true);
                        cur_sel = hit;
                        // The caret (an XOR line, `curfld`, A111) comes off
                        // before the selection text changes under it.
                        object::edit_field_t(t, focus, 0, idx, ED_END);
                        // "strcpy(ad_fname, selname + 1)" (gemfslib.c:827):
                        // the clicked slot's raw string minus its indicator
                        // byte — i.e. `fmt_str` of the listed name.
                        // SAFETY: single-CPU AES.
                        unsafe { fmt_str(&name, &mut *SYS.sel.get()) };
                        // Show the new name — `EDINIT` draws the caret only
                        // (`gemobed.c:363-365`) — then re-arm editing on the
                        // selection field (caret at end).
                        object::draw_obj_t(t, FSSELECT, 0, backend);
                        focus = FSSELECT;
                        idx = object::edit_field_t(t, focus, 0, 0, ED_INIT);
                        if hit == last_hit && now.saturating_sub(last_t) < DCLICK_MS {
                            return 1;
                        }
                    }
                }
            } else if (FS1STDRV..=FSLSTDRV).contains(&hit) {
                // SAFETY: valid tree; BOXCHAR ob_spec packs the char at bits 24..31.
                let ch = unsafe { ((*tree.add(hit as usize)).ob_spec >> 24) as u8 };
                if ch.is_ascii_uppercase() {
                    set_drive(ch);
                    set_drive_states(tree, ch);
                    cur_sel = -1;
                    object::edit_field_t(t, focus, 0, idx, ED_END);
                    relist(tree);
                    // SAFETY: single-CPU AES.
                    listed = unsafe { *SYS.dir.get() };
                    object::draw_obj_t(t, 0, MAX_DEPTH, backend); // drive states + dir + list
                    focus = FSSELECT;
                    idx = object::edit_field_t(t, focus, 0, 0, ED_INIT); // caret last
                }
            }
            last_hit = hit;
            last_t = now;
        }
        prev = cur;
        h.pump_input();
    }
}

#[cfg(test)]
mod tests {
    //! A63 / A64 — the file selector's start-up pass and its formatted fields.
    //!
    //! These live **in-crate** because both subjects are module-private: the
    //! 3D/geometry pass [`fs_start`] runs once when the resource is relocated
    //! (`relocate_and_cache`), and [`set_ted`] repoints the TEDINFOs on every
    //! call. Neither is reachable from `tests/`, and neither needs a raster
    //! backend — they only rewrite the object tree.
    //!
    //! The fixture is the shipped `gem_fsel.nrsc` blob itself, relocated the
    //! same way `relocate_and_cache` relocates it, so the numbers below are
    //! measured against the real EmuTOS `FSELECTR` resource.

    use super::*;
    use tos_types::aes::FL3DMASK;

    /// A relocated, writable copy of the embedded `FSELECTR` resource.
    /// The `Vec<u64>` backing store gives the 8-byte alignment the native
    /// `Object`/`TedInfo` records are dereferenced at; it must outlive the
    /// returned pointer, hence the tuple.
    fn fresh_tree() -> (Vec<u64>, *mut Object) {
        let mut buf = vec![0u64; GEM_CAP / 8];
        // SAFETY: `buf` is 8-byte aligned and `GEM_CAP` bytes long.
        let base = buf.as_mut_ptr() as *mut u8;
        // SAFETY: the blob fits (`const _` assert in the module).
        unsafe {
            core::ptr::copy_nonoverlapping(GEM_FSEL_RSC.as_ptr(), base, GEM_FSEL_RSC.len());
            assert!(resource::relocate_blob(base, GEM_FSEL_RSC.len()));
            let root = resource::tree_root(base, 0);
            (buf, root)
        }
    }

    /// `(ob_x, ob_y, ob_width, ob_height)` of `idx`.
    fn xywh(tree: *mut Object, idx: i16) -> (i16, i16, i16, i16) {
        // SAFETY: `idx` is a valid FSELECTR object.
        unsafe {
            let o = &*tree.add(idx as usize);
            (o.ob_x, o.ob_y, o.ob_width, o.ob_height)
        }
    }

    fn flags(tree: *mut Object, idx: i16) -> u16 {
        // SAFETY: `idx` is a valid FSELECTR object.
        unsafe { (*tree.add(idx as usize)).ob_flags }
    }

    fn ted(tree: *mut Object, idx: i16) -> &'static mut TedInfo {
        // SAFETY: `idx` is a TEDINFO-bearing FSELECTR object of a live tree.
        unsafe { &mut *((*tree.add(idx as usize)).ob_spec as *mut TedInfo) }
    }

    fn cstr(p: u64) -> Vec<u8> {
        let mut v = Vec::new();
        if p == 0 {
            return v;
        }
        // SAFETY: the resource strings are NUL-terminated inside the blob.
        unsafe {
            let mut i = 0;
            while *(p as *const u8).add(i) != 0 {
                v.push(*(p as *const u8).add(i));
                i += 1;
            }
        }
        v
    }

    // ─── A64: the 3D start-up pass ──────────────────────────────────────

    /// EmuTOS `fs_start` (`aes/gemfslib.c:60-152`) does not only tag flags —
    /// it moves and resizes every object the bevel touches. These are the
    /// thirteen adjustments, each against the value the resource ships.
    #[test]
    fn fs_start_applies_every_geometry_adjustment_of_the_3d_pass() {
        let (_buf, tree) = fresh_tree();
        fs_start(tree);

        // `:72-73` FCLSBOX x/y += ADJ3DSTD (resource 0,0,16,16)
        assert_eq!(xywh(tree, FCLSBOX), (2, 2, 16, 16), "FCLSBOX");
        // `:79-80` FTITLE x += 2*ADJ3DSTD-1, height += 2*ADJ3DSTD (16,0,160,16)
        assert_eq!(xywh(tree, FTITLE), (19, 0, 160, 20), "FTITLE");
        // `:83-84` FILEBOX y += 3*ADJ3DSTD-1, width += ADJ3DSTD (0,16,152,176)
        assert_eq!(xywh(tree, FILEBOX), (0, 21, 154, 176), "FILEBOX");
        // `:86-87` SCRLBAR x += 2*ADJ3DSTD-1, y += 3*ADJ3DSTD-1 (152,16,24,176)
        assert_eq!(xywh(tree, SCRLBAR), (155, 21, 24, 176), "SCRLBAR");
        // `:90-92` FUPAROW x/y += ADJ3DSTD, width -= 2*ADJ3DSTD (0,0,24,16)
        assert_eq!(xywh(tree, FUPAROW), (2, 2, 20, 16), "FUPAROW");
        // `:94-95` FSVSLID y += 3*ADJ3DSTD, height -= 6*ADJ3DSTD (0,16,24,144)
        assert_eq!(xywh(tree, FSVSLID), (0, 22, 24, 132), "FSVSLID");
        // `:99-101` FSVELEV x += ADJ3DSTD, width -= 2*ADJ3DSTD; y/h stay
        // dynamic (`fs_format`) — resource (0,0,24,16)
        assert_eq!(xywh(tree, FSVELEV), (2, 0, 20, 16), "FSVELEV");
        // `:103-106` FDNAROW x += ADJ3DSTD, y -= ADJ3DSTD, w -= 2*ADJ3DSTD
        assert_eq!(xywh(tree, FDNAROW), (2, 158, 20, 16), "FDNAROW");
        // `:108-110` FILEAREA y -= 6, width += 2*ADJ3DSTD-1, h += 3*ADJ3DSTD-1
        assert_eq!(xywh(tree, FILEAREA), (24, 106, 179, 197), "FILEAREA");
        // `:122-128` every drive box: x += 2*ADJ3DSTD, y += 2*ADJ3DSTD +
        // row*(3*ADJ3DSTD-1), row cycling 0..DRIVE_ROWS (9) down each column.
        assert_eq!(xywh(tree, FS1STDRV), (4, 4, 24, 16), "drive A");
        assert_eq!(xywh(tree, FS1STDRV + 1), (4, 25, 24, 16), "drive B");
        assert_eq!(xywh(tree, FS1STDRV + 8), (4, 172, 24, 16), "drive I");
        assert_eq!(xywh(tree, FS1STDRV + 9), (36, 4, 24, 16), "drive J (row wraps)");
        // Z is the 26th gadget: column 2, row 7 (resource y 112) — 112 + 4 + 7*5.
        assert_eq!(xywh(tree, FSLSTDRV), (68, 151, 24, 16), "drive Z");
        // `:130-131` FSDRIVES height += 2*ADJ3DSTD + DRIVE_ROWS*(3*ADJ3DSTD-1),
        // width += 4*ADJ3DSTD (resource 216,96,88,144)
        assert_eq!(xywh(tree, FSDRIVES), (216, 96, 96, 193), "FSDRIVES");
        // `:139-147` FSOK.x = FILEAREA.x, y += ADJ3DSTD; FSCANCEL right-aligned
        // on FILEAREA, same y (resource 56,320,72,16 / 184,320,72,16)
        assert_eq!(xywh(tree, FSOK), (24, 322, 72, 16), "FSOK");
        assert_eq!(xywh(tree, FSCANCEL), (131, 322, 72, 16), "FSCANCEL");
        // `:150` ROOT height += 2*ADJ3DSTD (resource 0,0,320,352)
        assert_eq!(xywh(tree, ROOT), (0, 0, 320, 356), "ROOT");
    }

    /// The four 3D flags the old `set_3d_flags` never set (`:72`, `:78`,
    /// `:138`, `:149`), plus the ten it did.
    #[test]
    fn fs_start_sets_every_3d_flag_emutos_sets() {
        let (_buf, tree) = fresh_tree();
        fs_start(tree);

        for b in [FSOK, FSCANCEL, FUPAROW, FDNAROW, FSVELEV, FCLSBOX] {
            assert_eq!(flags(tree, b) & FL3DACT, FL3DACT, "FL3DACT on {b}");
        }
        for b in [FSDIRECT, FSSELECT, FSDRVTXT, FTITLE, ROOT] {
            assert_eq!(flags(tree, b) & FL3DMASK, FL3DBAK, "FL3DBAK on {b}");
        }
        for d in FS1STDRV..=FSLSTDRV {
            assert_eq!(flags(tree, d) & FL3DMASK, FL3DIND, "FL3DIND on {d}");
        }
    }

    /// `:81` and `:98` — "use pattern 4, just like TOS4": the FTITLE TEDINFO
    /// colour word and the FSVSLID box `ob_spec` both get `IP_4PATT` in their
    /// pattern nibble, leaving every other field alone.
    #[test]
    fn fs_start_switches_ftitle_and_fsvslid_to_the_50_percent_pattern() {
        let (_buf, tree) = fresh_tree();
        let before_color = ted(tree, FTITLE).te_color as u16;
        // SAFETY: FSVSLID is a valid G_BOX of the live tree.
        let before_spec = unsafe { (*tree.add(FSVSLID as usize)).ob_spec };
        assert_eq!(before_color, 0x11A1, "resource FTITLE te_color");
        assert_eq!(before_spec, 0x00FF_1111, "resource FSVSLID ob_spec");

        fs_start(tree);

        assert_eq!(ted(tree, FTITLE).te_color as u16, 0x11C1, "FTITLE te_color");
        // SAFETY: as above.
        assert_eq!(unsafe { (*tree.add(FSVSLID as usize)).ob_spec }, 0x00FF_1141, "FSVSLID ob_spec");
    }

    // ─── A63: the formatted fields ──────────────────────────────────────

    /// The repointed fields must stay `G_FTEXT`/`G_FBOXTEXT` and keep the
    /// resource's template, validation string and text length — only
    /// `te_ptext` moves to our buffer. Down-converting them to
    /// `G_BOXTEXT`/`G_TEXT` and clearing `te_ptmplt` is what lost the
    /// underscores, the `Selection:` caption and the 8.3 columns.
    #[test]
    fn set_ted_keeps_the_formatted_type_template_and_length() {
        let (_buf, tree) = fresh_tree();
        fs_start(tree);
        let mut dir = [0u8; 96];
        let mut sel = [0u8; 16];
        let mut name = [0u8; 16];
        set_ted(tree, FSDIRECT, dir.as_mut_ptr(), dir.len());
        set_ted(tree, FSSELECT, sel.as_mut_ptr(), sel.len());
        set_ted(tree, F1NAME, name.as_mut_ptr(), name.len());

        // SAFETY: valid tree.
        unsafe {
            assert_eq!((*tree.add(FSDIRECT as usize)).ob_type, G_FBOXTEXT, "FSDIRECT type");
            assert_eq!((*tree.add(FSSELECT as usize)).ob_type, G_FBOXTEXT, "FSSELECT type");
            assert_eq!((*tree.add(F1NAME as usize)).ob_type, G_FBOXTEXT, "F1NAME type");
        }
        assert_eq!(cstr(ted(tree, FSDIRECT).te_ptmplt), b"_".repeat(38), "FSDIRECT template");
        assert_eq!(cstr(ted(tree, FSSELECT).te_ptmplt), b"Selection: ________.___", "FSSELECT template");
        assert_eq!(cstr(ted(tree, F1NAME).te_ptmplt), b"_ ________.___ ", "F1NAME template");
        assert_eq!(cstr(ted(tree, FSDIRECT).te_pvalid), b"P".repeat(38), "FSDIRECT pvalid");
        assert_eq!(cstr(ted(tree, FSSELECT).te_pvalid), b"F".repeat(11), "FSSELECT pvalid");
        assert_eq!(ted(tree, FSDIRECT).te_txtlen, 39, "FSDIRECT te_txtlen");
        assert_eq!(ted(tree, FSSELECT).te_txtlen, 12, "FSSELECT te_txtlen");
        assert_eq!(ted(tree, F1NAME).te_txtlen, 13, "F1NAME te_txtlen");
        assert_eq!(ted(tree, FSDIRECT).te_ptext, dir.as_ptr() as u64, "FSDIRECT te_ptext");
    }

    /// EmuTOS `fmt_str`/`unfmt_str` (`util/optimize.c:43,77`): the raw form
    /// of a filename for an `FFFFFFFF.FFF` template is dot-less and
    /// space-padded to 8 characters.
    #[test]
    fn fmt_str_and_unfmt_str_convert_between_plain_and_raw_8_3() {
        let cases: &[(&[u8], &[u8])] = &[
            (b"SAMPLE.PRG", b"SAMPLE  PRG"),
            (b"TESTPROG.C", b"TESTPROGC"),
            (b"TEST", b"TEST"),
            (b"TESTWINDOW.C", b"TESTWINDC"),
            (b"TEST.A.B.C", b"TEST    A.B"),
            (b"TESTTESTTEST", b"TESTTEST"),
        ];
        for (plain, raw) in cases {
            let mut out = [0u8; 16];
            fmt_str(plain, &mut out);
            assert_eq!(&out[..raw.len()], *raw, "fmt_str({:?})", core::str::from_utf8(plain));
            assert_eq!(out[raw.len()], 0, "fmt_str NUL");
        }
        // unfmt_str is the reverse of the 8.3 cases (EmuTOS's own example).
        let mut back = [0u8; 16];
        unfmt_str(b"SAMPLE  PRG", &mut back);
        assert_eq!(&back[..10], b"SAMPLE.PRG");
        unfmt_str(b"TEST", &mut back);
        assert_eq!(&back[..4], b"TEST");
    }

    /// EmuTOS `fs_format` (`aes/gemfslib.c:365-376`): each visible slot's raw
    /// string is the folder/file indicator byte followed by the `fmt_str`
    /// name; a slot past the end of the list is a single blank, so
    /// `ob_format` fills the whole template with underscores.
    #[test]
    fn fs_format_fills_the_slots_with_the_indicator_and_the_raw_name() {
        let (_buf, tree) = fresh_tree();
        fs_start(tree);
        // SAFETY: single-CPU AES; the test owns the statics for its duration.
        unsafe {
            let all = &mut *SYS.all.get();
            let attr = &mut *SYS.all_attr.get();
            all[0] = [0; 16];
            all[0][..10].copy_from_slice(b"CONFIG.TXT");
            attr[0] = 0;
            all[1] = [0; 16];
            all[1][..5].copy_from_slice(b"APPS\0");
            attr[1] = FA_SUBDIR;
            *SYS.count.get() = 2;
            *SYS.top.get() = 0;
        }
        fs_format(tree);
        // SAFETY: as above.
        let names = unsafe { &*SYS.names.get() };
        assert_eq!(&names[0][..12], b" CONFIG  TXT", "file slot");
        assert_eq!(names[0][12], 0, "file slot NUL");
        assert_eq!(&names[1][..5], b"\x07APPS", "folder slot");
        assert_eq!(names[1][5], 0, "folder slot NUL");
        assert_eq!(&names[2][..2], b" \0", "empty slot is one blank");
    }
}
