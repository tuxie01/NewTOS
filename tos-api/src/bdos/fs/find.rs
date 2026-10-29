//! DTA-based directory search: `Fsfirst` / `Fsnext`, plus `Fsetdta` /
//! `Fgetdta`. Mirrors EmuTOS `bdos/fsdir.c` (`xsfirst`/`xsnext`) and
//! `bdos/fsmain.c` (`xsetdta`/`xgetdta`).
//!
//! Spec: `docs/api/gemdos/file/{4e-fsfirst,4f-fsnext}.md`,
//! `docs/api/gemdos/dta/{1a-fsetdta,2f-fgetdta}.md`,
//! `docs/api/structures/dta.md`.
//!
//! Searches resolve through sub-directories (`findit`). The 21-byte DTA
//! reserved area carries our own resume state — its internal layout is
//! explicitly **not** part of the ABI contract, so we are free to define
//! it:
//!
//! ```text
//!  [0..4]  next raw directory slot to resume at (u32 LE); !0 = exhausted
//!  [4..8]  parent directory first cluster (u32 LE; 0 = root)
//!  [8]     drive number
//!  [9]     effective attribute mask
//!  [10..21] 11-byte FAT search pattern ('?' = wildcard)
//! ```
//!
//! The current-DTA pointer lives in the **running process's** basepage
//! (`p_xdta` at offset 0x20), exactly as EmuTOS keeps it in the process
//! descriptor (`bdos/fsmain.c:396,403-405`; the default `&p_cmdlin` is set
//! at load time, `bdos/proc.c:375`). See [`cur_dta`] for the one caller
//! class that has no process descriptor to store it in.

use core::sync::atomic::{AtomicU64, Ordering};

use super::dir::DirEntry;
use super::fat::Volume;
use crate::trap::TrapArgs;
use crate::uaccess::{Space, UserFault};
use tos_types::error::{EFILNF, ENMFIL, EPTHNF, E_OK};
use tos_types::Long;

/// FAT attribute bits (`bdosdefs.h`).
const FA_RO: u8 = 0x01;
const FA_HIDDEN: u8 = 0x02;
const FA_SYSTEM: u8 = 0x04;
const FA_VOL: u8 = 0x08;
const FA_SUBDIR: u8 = 0x10;
const FA_ARCHIVE: u8 = 0x20;
const FA_LFN: u8 = 0x0F;

const ERASED: u8 = 0xE5;
const END: u8 = 0x00;
const SLOTS_PER_SECTOR: u32 = 16;
/// Sentinel for "DTA not initialised / search exhausted".
const EXHAUSTED: u32 = u32::MAX;

/// Current-DTA cell for callers that are not a process.
///
/// EmuTOS has no such cell: every `Fsetdta`/`Fgetdta`/`Fsfirst` caller is
/// `run`, a process descriptor with a `p_xdta` field. NewTOS has a caller
/// class that is not — **EL1 kernel code** issuing GEMDOS calls with kernel
/// buffers: the kernel-resident EmuCON (`kernel/src/shell.rs`, `Fsetdta` +
/// `Fsfirst` with a DTA on its stack, from the shell loop's console arm) and
/// the boot-time selftests (`Space::Kernel` `TrapArgs`). Such an address is a
/// high-half kernel VA and does not even fit the 32-bit on-wire `p_xdta`
/// field.
///
/// Until DESK43 U10 the built-in (EL1) desktop was a caller too, and this note
/// said the cell would go with the desktop's move to EL0. It does not: the
/// desktop is a process now (`NEWDESK`, its DTA in its `p_xdta`), but EmuCON
/// and the selftests still trap from EL1 — the cell stays as long as they do.
static KERNEL_DTA: AtomicU64 = AtomicU64::new(0);

/// The current DTA pointer — EmuTOS `run->p_xdta` (`bdos/fsmain.c:396`).
///
/// Falls back to [`KERNEL_DTA`] for the process-less EL1 caller (see there).
///
/// Visible to the crate (not just this module) so the GEMDOS trap tracer can
/// capture the DTA **contents** after a successful `Fsfirst`/`Fsnext` — a
/// captured DTA *address* is worthless in a trace. A plain read of
/// `run->p_xdta` with no side effect; nothing about the search changes.
pub(crate) fn cur_dta() -> u64 {
    let bp = crate::bdos::process::current_kva();
    if bp.is_null() {
        return KERNEL_DTA.load(Ordering::Relaxed);
    }
    // SAFETY: `current_kva()` is the kernel high-half alias of the running
    // process's `BasePage`, valid and mapped for as long as that process
    // runs; we read the single `p_xdta` LONG. Zero-extended: user addresses
    // are below `0x8000_0000` (ARCH03 plan invariant I2), so the field is a
    // non-negative 32-bit address.
    unsafe { (*bp).p_xdta as u32 as u64 }
}

/// The current DTA of a caller in `space` and the address space it lives in
/// (ARCH03 3a/3b): a program's (`Space::User`) is its process's `p_xdta`, a
/// user address read and written with its rights; a kernel caller's
/// (`Space::Kernel` — the built-in shell EmuCON, the selftests) is the
/// [`KERNEL_DTA`] cell, a kernel address — also while some process is the
/// running one, so a kernel DTA never lands in a program's `p_xdta`.
pub(crate) fn cur_dta_in(space: Space) -> (u64, Space) {
    match space {
        Space::Kernel => (KERNEL_DTA.load(Ordering::Relaxed), Space::Kernel),
        Space::User => (cur_dta(), Space::User),
    }
}

/// Store the current DTA pointer of a caller in `space` — EmuTOS
/// `run->p_xdta = addr` (`bdos/fsmain.c:405`) for a program.
///
/// `p_xdta` is the on-wire basepage's 32-bit LONG; every program's DTA is a
/// user address below `0x8000_0000` (ARCH03 plan invariant I2), well inside
/// 32 bits. A kernel caller's DTA is a high-half kernel address that does not
/// fit: it goes to [`KERNEL_DTA`] whether or not a process is running —
/// before ARCH03 3b an EL1 caller with a process current had it cut to 32
/// bits into that process's `p_xdta`.
fn set_cur_dta(addr: u64, space: Space) {
    let bp = crate::bdos::process::current_kva();
    if space == Space::Kernel || bp.is_null() {
        KERNEL_DTA.store(addr, Ordering::Relaxed);
        return;
    }
    // SAFETY: as in [`cur_dta`] — a valid, writable basepage alias.
    unsafe { (*bp).p_xdta = addr as u32 as Long };
}

/// `Fsetdta` (0x1A) — set the current Disk Transfer Address.
///
/// EmuTOS `xsetdta` (`bdos/fsmain.c:403-405`) stores the address in the
/// **running process's** descriptor, so a child started with `Pexec` cannot
/// disturb its parent's DTA: the child has its own `p_xdta` (defaulted to its
/// `p_cmdlin`, `bdos/proc.c:375`), and when it terminates `run` reverts to the
/// parent, whose field was never touched. A kernel caller keeps its own
/// ([`set_cur_dta`]).
pub fn fsetdta(args: &TrapArgs) -> Long {
    set_cur_dta(args.arg_u64(0), args.space);
    E_OK
}

/// `Fgetdta` (0x2F) — return the current DTA pointer (EmuTOS `xgetdta`,
/// `bdos/fsmain.c:394-397`: `return run->p_xdta`).
///
/// A process's DTA round-trips exactly: it is stored in, and read back from,
/// the same 32-bit `p_xdta` field. A DTA set by the process-less EL1 caller is
/// a high-half kernel VA that a `Long` return cannot carry — that caller keeps
/// its own pointer and never reads it back (see [`KERNEL_DTA`]).
///
/// A program's DTA is checked in a debug build only while it is still the
/// one the kernel gave it ([`debug_assert_fgetdta`], TASK/MEM11); the kernel
/// caller's is a kernel VA and is not checked at all.
pub fn fgetdta(args: &TrapArgs) -> Long {
    let (dta, space) = cur_dta_in(args.space);
    if space == Space::User {
        let pd = crate::bdos::process::current() as u64;
        if pd != 0 {
            debug_assert_fgetdta(pd, dta);
        }
    }
    dta as Long
}

/// The DTA a process starts with, as its 32-bit `p_xdta` holds it: the
/// address of its own `p_cmdlin` (EmuTOS `bdos/proc.c:375`, NewTOS
/// `loader` and `standin`), for the PD pointer `pd` (the basepage's user
/// address, [`crate::bdos::process::pd_of`]).
pub fn default_dta(pd: u64) -> u64 {
    let cmdlin = core::mem::offset_of!(tos_types::BasePage, p_cmdlin) as u64;
    pd.wrapping_add(cmdlin) as u32 as u64
}

/// Debug-build check on the `Fgetdta` result `dta` of the process whose PD
/// pointer is `pd` (TASK/MEM11): a debug assertion checks a **kernel**
/// invariant, never user input.
///
/// * `dta` is the kernel-provided default ([`default_dta`]): the basepage
///   lies in memory the kernel handed out, so `&p_cmdlin` must be a user
///   pointer below `0x8000_0000`
///   ([`crate::bdos::process::usermem::debug_assert_user_ptr`]). Checked on
///   the full `pd + 0x80`, not on the 32 bits `p_xdta` keeps of it.
/// * any other `dta` was set by the program with `Fsetdta`, which takes any
///   address (EmuTOS `xsetdta`, `bdos/fsmain.c:403-405`). It is returned as
///   is, also at or above `0x8000_0000`; a DTA the program cannot write ends
///   it at the next `Fsfirst`/`Fsnext` (a user-access fault,
///   [`crate::uaccess`]), as the bus error does on EmuTOS.
#[track_caller]
pub fn debug_assert_fgetdta(pd: u64, dta: u64) {
    if dta == default_dta(pd) {
        let cmdlin = core::mem::offset_of!(tos_types::BasePage, p_cmdlin) as u64;
        crate::bdos::process::usermem::debug_assert_user_ptr("Fgetdta", pd.wrapping_add(cmdlin));
    }
}

/// Expand a `name.ext` pattern (with `*`/`?`) into the 11-byte FAT
/// short-name form where every wildcard position is `?`.
fn fcb_pattern(name: &str) -> [u8; 11] {
    let mut out = [b' '; 11];
    let b = name.as_bytes();
    let mut i = 0;

    // 8-char name field.
    let mut p = 0;
    while i < b.len() && b[i] != b'.' {
        let c = b[i].to_ascii_uppercase();
        if c == b'*' {
            while p < 8 {
                out[p] = b'?';
                p += 1;
            }
            while i < b.len() && b[i] != b'.' {
                i += 1;
            }
            break;
        }
        if p < 8 {
            out[p] = c;
            p += 1;
        }
        i += 1;
    }

    // 3-char extension field.
    if i < b.len() && b[i] == b'.' {
        i += 1;
        let mut x = 8;
        while i < b.len() {
            let c = b[i].to_ascii_uppercase();
            if c == b'*' {
                while x < 11 {
                    out[x] = b'?';
                    x += 1;
                }
                break;
            }
            if x < 11 {
                out[x] = c;
                x += 1;
            }
            i += 1;
        }
    }
    out
}

/// Match an 11-byte FAT name against a pattern (`?` = any character).
fn pattern_match(pat: &[u8; 11], name: &[u8; 11]) -> bool {
    pat.iter().zip(name).all(|(&p, &n)| p == b'?' || p == n)
}

/// Whether an entry's attributes pass the search mask, per the EmuTOS
/// `scan` rules: `att == FA_VOL` matches labels exclusively; otherwise
/// volume labels are skipped and hidden/system/sub-dir entries appear
/// only if their bit is requested (FA_RO / FA_ARCHIVE never restrict).
fn attr_ok(att: u8, entry_attr: u8) -> bool {
    if att == FA_VOL {
        return entry_attr & FA_VOL != 0;
    }
    if entry_attr & FA_VOL != 0 {
        return false;
    }
    let restricted = FA_HIDDEN | FA_SYSTEM | FA_SUBDIR;
    entry_attr & restricted & !att == 0
}

/// Scan directory `dir` (first cluster; 0 = root) from raw slot `start`,
/// returning the next entry matching `pattern`/`att` together with the
/// slot index to resume at. `None` at the end-of-directory marker or once
/// the directory's sectors are exhausted.
fn scan(
    vol: &Volume,
    dir: u32,
    start: u32,
    pattern: &[u8; 11],
    att: u8,
) -> Option<(DirEntry, u32)> {
    let mut slot = start;
    let mut sbuf = [0u8; 512];
    let mut loaded = u32::MAX;

    loop {
        let sec = slot / SLOTS_PER_SECTOR;
        if sec != loaded {
            let rel = vol.dir_nth_sector(dir, sec)?;
            if !vol.read_rel_sector(rel, &mut sbuf) {
                return None;
            }
            loaded = sec;
        }
        let off = (slot % SLOTS_PER_SECTOR) as usize * 32;
        let e = &sbuf[off..off + 32];
        slot += 1;

        match e[0] {
            END => return None,
            ERASED => continue,
            _ => {}
        }
        if e[0x0B] == FA_LFN {
            continue;
        }
        let entry = DirEntry::parse(e);
        if attr_ok(att, entry.attrib) && pattern_match(pattern, &entry.name) {
            return Some((entry, slot));
        }
    }
}

/// FAT 11-byte name → `NAME.EXT\0` (EmuTOS `makbuf` formatting).
fn name_83(fat: &[u8; 11]) -> [u8; 14] {
    let mut out = [0u8; 14];
    let mut k = 0;
    for &c in &fat[0..8] {
        if c == b' ' {
            break;
        }
        out[k] = c;
        k += 1;
    }
    if fat[8] != b' ' {
        out[k] = b'.';
        k += 1;
        for &c in &fat[8..11] {
            if c == b' ' {
                break;
            }
            out[k] = c;
            k += 1;
        }
    }
    out
}

/// List directory `path` for the AES file selector, reproducing EmuTOS
/// `fs_active` (`gemfslib.c:287-307`). The final path component is the
/// wildcard mask (e.g. `C:\SUB\*.TXT`); the parent is scanned with an
/// all-match pattern (EmuTOS `dos_sfirst(dir\*.*, FA_SUBDIR)`) and each
/// entry is filtered exactly like the reference loop:
///
/// * `.`, `..` and every dot-file are skipped (`d_fname[0] != '.'`,
///   `gemfslib.c:298`),
/// * sub-directories are kept **unconditionally** (so folder navigation
///   keeps working under a restrictive mask), and
/// * plain files are kept only when they match the mask
///   (`(d_attrib & FA_SUBDIR) || wildcmp(pspec, d_fname)`, `gemfslib.c:300`).
///
/// Each kept `names[i]` gets `NAME.EXT\0` (NUL-padded), `attrs[i]` the
/// attribute byte. Entries are written **unsorted** (the caller sorts, like
/// `fs_active`). Like Atari TOS / EmuTOS (`gemfslib.c:290-292`), entries
/// past the buffer are silently ignored. Returns the number written. Used
/// by the AES file selector (`fsel_input`).
///
/// Runs under the restart after a media change: EmuTOS' file selector lists
/// through `dos_sfirst`/`dos_snext`, GEMDOS calls with `osif`'s restart
/// (`aes/gemfslib.c:285-310`; TASK/FS22). A run that met a change is made
/// once more after the re-login; it holds nothing but `names`/`attrs`, which
/// the next run writes again. 0 when the restart gives up (`E_CHNG`).
pub fn list_dir(path: &str, names: &mut [[u8; 16]], attrs: &mut [u8]) -> usize {
    crate::gemdos::restartable(|| list_dir_once(path, names, attrs)).unwrap_or(0)
}

/// One run of [`list_dir`].
fn list_dir_once(path: &str, names: &mut [[u8; 16]], attrs: &mut [u8]) -> usize {
    let (vol, parent, comp) = match super::path::resolve_parent(path) {
        Ok(x) => x,
        Err(_) => return 0,
    };
    let mask = fcb_pattern(comp);
    // Scan every entry (EmuTOS lists the directory with `*.*`); the real
    // mask is applied to plain files only, below.
    let any = [b'?'; 11];
    let cap = names.len().min(attrs.len());
    let mut slot = 0u32;
    let mut n = 0;
    while n < cap {
        match scan(&vol, parent, slot, &any, FA_SUBDIR) {
            Some((entry, next)) => {
                slot = next;
                // Skip `.`, `..` and all dot-files (EmuTOS gemfslib.c:298).
                if entry.name[0] == b'.' {
                    continue;
                }
                // Folders always; files only when they match the mask
                // (EmuTOS gemfslib.c:300).
                let is_dir = entry.attrib & FA_SUBDIR != 0;
                if !is_dir && !pattern_match(&mask, &entry.name) {
                    continue;
                }
                let disp = name_83(&entry.name);
                names[n] = [0u8; 16];
                names[n][..14].copy_from_slice(&disp);
                attrs[n] = entry.attrib;
                n += 1;
            }
            None => break,
        }
    }
    n
}

/// One directory entry with the fields the desktop needs for display and
/// sorting (richer than [`list_dir`]'s name+attribute).
#[derive(Clone, Copy)]
pub struct DirInfo {
    /// `NAME.EXT\0` display name (trim-padded).
    pub name: [u8; 14],
    /// `FA_*` attribute byte (bit `FA_SUBDIR` = directory).
    pub attrib: u8,
    /// File length in bytes.
    pub size: u32,
    /// Packed FAT date.
    pub date: u16,
    /// Packed FAT time.
    pub time: u16,
}

impl Default for DirInfo {
    fn default() -> Self {
        Self { name: [0; 14], attrib: 0, size: 0, date: 0, time: 0 }
    }
}

/// Like [`list_dir`] but also captures size and date/time per entry, for
/// the desktop's text view and sort orders. Returns the number filled.
///
/// Its callers are kernel paths — the shell's scrap clearing, the
/// accessory loader, the AUTO folder and the boot-image lookup — that EmuTOS
/// makes as `Fsfirst`/`Fsnext` calls (e.g. `aes/gemsclib.c:93-100`), so it
/// runs under the restart after a media change like [`list_dir`]
/// (TASK/FS22). 0 when the restart gives up (`E_CHNG`).
pub fn list_dir_ex(path: &str, out: &mut [DirInfo]) -> usize {
    crate::gemdos::restartable(|| list_dir_ex_once(path, out)).unwrap_or(0)
}

/// One run of [`list_dir_ex`].
fn list_dir_ex_once(path: &str, out: &mut [DirInfo]) -> usize {
    let (vol, parent, comp) = match super::path::resolve_parent(path) {
        Ok(x) => x,
        Err(_) => return 0,
    };
    let pattern = fcb_pattern(comp);
    let mut slot = 0u32;
    let mut n = 0;
    while n < out.len() {
        match scan(&vol, parent, slot, &pattern, FA_SUBDIR) {
            Some((entry, next)) => {
                out[n] = DirInfo {
                    name: name_83(&entry.name),
                    attrib: entry.attrib,
                    size: entry.size,
                    date: entry.date,
                    time: entry.time,
                };
                n += 1;
                slot = next;
            }
            None => break,
        }
    }
    n
}

/// Write the user-visible portion of the DTA from a found entry
/// (EmuTOS `makbuf`, `bdos/fsdir.c:1851-1862`): attribute, time, date,
/// size in native byte order (little-endian on AArch64), then `d_fname`:
/// only the `8.3` name and its NUL, as `packit` writes it
/// (`bdos/fsdir.c:1297-1324`) -- the bytes after the NUL keep their old
/// content (FS23).
///
/// `dta` is a 44-byte DTA in `space` ([`cur_dta_in`]); a DTA the program
/// cannot write is a user-access fault (ARCH03 3a, [`crate::uaccess`]).
fn makbuf(space: Space, dta: u64, e: &DirEntry) -> Result<(), UserFault> {
    // Bytes 21..30 of the DTA: attrib, time, date, size.
    let mut out = [0u8; 9 + 14];
    out[0] = e.attrib;
    out[1..3].copy_from_slice(&e.time.to_le_bytes());
    out[3..5].copy_from_slice(&e.date.to_le_bytes());
    out[5..9].copy_from_slice(&e.size.to_le_bytes());

    // d_fname (offset 30): "NAME.EXT" and its NUL, nothing after it.
    // `name_83` (the `packit` counterpart) yields at most 12 characters, so
    // the NUL always lies within the 14 bytes.
    let name = name_83(&e.name);
    let len = name.iter().position(|&c| c == 0).unwrap_or(name.len() - 1) + 1;
    out[9..9 + len].copy_from_slice(&name[..len]);
    space.copy_to(dta + 21, &out[..9 + len])
}

/// Store our resume state into the DTA reserved area (bytes 0..21 of the
/// 44-byte DTA at `dta` in `space`).
fn put_state(
    space: Space,
    dta: u64,
    next_slot: u32,
    dir: u32,
    drive: u8,
    att: u8,
    pattern: &[u8; 11],
) -> Result<(), UserFault> {
    let mut r = [0u8; 21];
    r[0..4].copy_from_slice(&next_slot.to_le_bytes());
    r[4..8].copy_from_slice(&dir.to_le_bytes());
    r[8] = drive;
    r[9] = att;
    r[10..21].copy_from_slice(pattern);
    space.copy_to(dta, &r)
}

/// Load our resume state from the DTA reserved area (written by
/// [`put_state`]).
fn get_state(space: Space, dta: u64) -> Result<(u32, u32, u8, u8, [u8; 11]), UserFault> {
    let mut r = [0u8; 21];
    space.copy_from(&mut r, dta)?;
    let next = u32::from_le_bytes([r[0], r[1], r[2], r[3]]);
    let dir = u32::from_le_bytes([r[4], r[5], r[6], r[7]]);
    let mut pat = [0u8; 11];
    pat.copy_from_slice(&r[10..21]);
    Ok((next, dir, r[8], r[9], pat))
}

/// `Fsfirst` (0x4E) — begin a directory search, filling the current DTA.
pub fn fsfirst(args: &TrapArgs) -> Long {
    let (dta, space) = cur_dta_in(args.space);
    if dta == 0 {
        return EFILNF;
    }
    // Path argument(s) in the caller's space (ARCH03 3a, `fs::path_arg`).
    let mut path_buf: crate::bdos::fs::PathBuf = [0; crate::bdos::fs::PATH_ARG_MAX];
    let path = match crate::bdos::fs::path_arg(args, 0, &mut path_buf) {
        Ok(Some(p)) => p,
        Ok(None) => return EPTHNF,
        Err(_) => return crate::uaccess::FAULTED,
    };
    let att = args.arg_u16(1) as u8;
    let eff_att = if att == FA_VOL {
        att
    } else {
        att | FA_ARCHIVE | FA_RO
    };

    // Resolve the parent directory; the final component is the (possibly
    // wildcarded) search pattern.
    let (vol, parent, comp) = match super::path::resolve_parent(path) {
        Ok(x) => x,
        // The directory portion of the search, an unmountable drive included,
        // resolves to `EPTHNF` (EmuTOS `ixsfirst` `findit` NULL → `EPTHNF`,
        // `fsdir.c:463`).
        Err(e) => return super::path::drive_notfound(e, EPTHNF),
    };
    let drive = super::path::split_drive(path).0 as u8;
    let pattern = fcb_pattern(comp);

    // Mark uninitialised, then search from slot 0 of the parent directory.
    if put_state(space, dta, EXHAUSTED, parent, drive, eff_att, &pattern).is_err() {
        return crate::uaccess::FAULTED;
    }
    match scan(&vol, parent, 0, &pattern, eff_att) {
        Some((entry, next)) => {
            if put_state(space, dta, next, parent, drive, eff_att, &pattern)
                .and_then(|()| makbuf(space, dta, &entry))
                .is_err()
            {
                return crate::uaccess::FAULTED;
            }
            E_OK
        }
        None => EFILNF,
    }
}

/// `Fsnext` (0x4F) — return the next match into the current DTA.
pub fn fsnext(args: &TrapArgs) -> Long {
    let (dta, space) = cur_dta_in(args.space);
    if dta == 0 {
        return ENMFIL;
    }
    let Ok((next, parent, drive, att, pattern)) = get_state(space, dta) else {
        return crate::uaccess::FAULTED;
    };
    if next == EXHAUSTED {
        return ENMFIL;
    }
    let vol = match Volume::mount(drive as u16) {
        Some(v) => v,
        None => return ENMFIL,
    };
    match scan(&vol, parent, next, &pattern, att) {
        Some((entry, after)) => {
            if put_state(space, dta, after, parent, drive, att, &pattern)
                .and_then(|()| makbuf(space, dta, &entry))
                .is_err()
            {
                return crate::uaccess::FAULTED;
            }
            E_OK
        }
        None => {
            // Mark exhausted.
            if put_state(space, dta, EXHAUSTED, parent, drive, att, &pattern).is_err() {
                return crate::uaccess::FAULTED;
            }
            ENMFIL
        }
    }
}
