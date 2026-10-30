//! Current drive and per-drive current directory **per PD** — EmuTOS
//! `PD.p_curdrv` / `PD.p_curdir[]` with the shared, reference-counted
//! `dirtbl[]` (TASK/PROC04 point 2, with TASK/PROC03; user decision
//! 2026-10-18 "alles EmuTOS-treu").
//!
//! Spec: `docs/api/gemdos/process/_run-ownership-curdir.md` §4,
//! `docs/api/gemdos/directory/{3b-dsetpath,47-dgetpath}.md`,
//! `docs/api/gemdos/drive/{0e-dsetdrv,19-dgetdrv}.md`.
//!
//! EmuTOS (`1a7484ab`):
//!
//! * `p_curdrv` (+0x37) is the drive `Dgetdrv` returns and `Dsetdrv` writes
//!   (`bdos/fsmain.c:420-441`) — **the byte in `run`'s PD**, read and written
//!   there, so a program that writes its own PD+0x37 changes its drive.
//! * `p_curdir[d]` (+0x40) is an index into the global `dirtbl[NCURDIR = 40]`
//!   (`bdos/fs.h:44,342-345`); slot 0 is never allocated and means "none yet";
//!   each slot counts the PD bytes that name it (`incr_curdir_usage` /
//!   `decr_curdir_usage`, `bdos/fsdir.c:1164-1218`).
//! * `init_pd_files` copies `run->p_curdir[]` (one use each) and
//!   `run->p_curdrv` into every PD `xexec` creates (`bdos/proc.c:401-409`);
//!   `ixterm` drops the dying PD's uses (`:126-132`); `Dsetpath` takes a use
//!   of the new slot, stores it, then drops the old (`fsdir.c:1140-1151`).
//!
//! `run` is the running process's PD or, with kernel code on the stack, the
//! stand-in PD the kernel's `run` names (`initial_basepage`, the AES PD, the
//! accdesk PD, the EmuCON PD — [`crate::bdos::process::standin`]). The
//! stand-ins are kernel memory the kernel writes; programs read them only.
//! An accessory executes with **its own** PD as `run` in NewTOS today
//! (`_run-ownership-curdir.md` §1.3 is TASK/PROC03 B5, not built), so its
//! `Dsetpath` moves the accessory PD's directory, copied from accdesk when it
//! was loaded.
//!
//! What NewTOS does differently, all of it reference ([R], §5 — no program
//! can interpret a `p_curdir` byte or see a use count):
//!
//! * A slot holds the directory as its canonical absolute path (`\A\B`,
//!   upper case, no drive letter) and its drive, not a DND pointer: NewTOS has
//!   no DND cache. A path that no longer resolves reads as the root. A media
//!   change frees every slot of its drive ([`free_drive`], `freetree`), so
//!   every PD's directory there reads as the root, what `ckdrv`'s repair
//!   gives in EmuTOS (`bdos/fsdrive.c:120-131`).
//! * The **root** is never given a slot: `p_curdir[d] == 0` *is* the root.
//!   EmuTOS creates a root slot lazily in `ckdrv` (`fsdrive.c:119-131`); the
//!   directory a program sees is the same, a slot is saved.
//! * The PD bytes are program-writable (as in EmuTOS). A byte that does not
//!   name a live slot of its drive reads as the root, and a PD's release only
//!   drops uses of slots that are live — EmuTOS would follow whatever the
//!   byte holds. When the table is full, the use counts are recounted from
//!   every live PD before giving up ([`reclaim`]), so a PD that disappeared
//!   without `ixterm` (an unstarted `PE_LOAD` child freed by `Mfree`, an
//!   accessory — EmuTOS's own leak, D-10 — or a scribbled byte) cannot exhaust
//!   the table for good. With every slot really in use `Dsetpath` still fails
//!   as EmuTOS's does (`EINTRN` → `EPTHNF`, `fsdir.c:1143-1144`).

use core::cell::UnsafeCell;

use tos_types::{BasePage, NUMCURDIR};

/// `NCURDIR` (`bdos/fs.h:44`) — size of `dirtbl[]`; slot 0 is never used.
pub const NCURDIR: usize = 40;

/// Longest stored directory path (`\A\B`, no drive letter) plus one — the
/// limit `Dsetpath` had before (`dirops.rs`).
pub const PATH_MAX: usize = 64;

/// Default current drive before the stand-in page exists: `C:` (the first
/// hard-disk drive), the value the global current drive had before.
pub const DEFAULT_DRIVE: u8 = 2;

#[derive(Clone, Copy)]
struct Slot {
    /// Number of PD bytes naming this slot; 0 = free.
    uses: u16,
    drive: u8,
    len: u8,
    path: [u8; PATH_MAX],
}

impl Slot {
    const FREE: Slot = Slot { uses: 0, drive: 0, len: 0, path: [0; PATH_MAX] };
}

struct Table(UnsafeCell<[Slot; NCURDIR]>);
// SAFETY: single CPU, BDOS trap path / kernel context under the BDOS lock,
// as every other BDOS file-system table.
unsafe impl Sync for Table {}

/// `dirtbl[]` (`bdos/fsdrive.c:47-54`).
static DIRTBL: Table = Table(UnsafeCell::new([Slot::FREE; NCURDIR]));

fn table() -> &'static mut [Slot; NCURDIR] {
    // SAFETY: single trap path (see `Table`); no reference outlives a call.
    unsafe { &mut *DIRTBL.0.get() }
}

/// A current directory as read out of the table (a copy, so the table may
/// change while it is used).
#[derive(Clone, Copy)]
pub struct CurDir {
    len: usize,
    buf: [u8; PATH_MAX],
}

impl CurDir {
    const ROOT: CurDir = CurDir { len: 0, buf: [0; PATH_MAX] };

    /// The directory as `\A\B` (no drive letter); empty at the root.
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buf[..self.len]).unwrap_or("")
    }
}

/// The kernel pointer to `run`'s PD: the running process's basepage, or with
/// kernel code on the stack the stand-in the kernel's `run` names
/// ([`crate::bdos::process::standin::kernel_run_kva`]). Never null.
pub fn run_pd() -> *mut BasePage {
    let p = crate::bdos::process::current_kva();
    if p.is_null() {
        crate::bdos::process::standin::kernel_run_kva()
    } else {
        p
    }
}

/// `run->p_curdrv` — the current drive (0 = `A:`), as `Dgetdrv` returns it
/// (`bdos/fsmain.c:438-441`) and the drive of a path without `X:`
/// (`dcrack`, `bdos/fsdir.c:1712-1716`).
pub fn current_drive() -> u16 {
    // SAFETY: `run_pd` is a live basepage (a process's, or a stand-in).
    u16::from(unsafe { (*run_pd()).p_curdrv })
}

/// `run->p_curdrv = drv` — `Dsetdrv`'s store (`bdos/fsmain.c:422-423`; the
/// range check is the caller's).
pub fn set_current_drive(drv: u16) {
    // SAFETY: as in `current_drive`; a byte store into that PD.
    unsafe { (*run_pd()).p_curdrv = drv as u8 };
}

/// The slot index `idx`, if it names a live slot of `drive`.
fn live(idx: u8, drive: u16) -> Option<usize> {
    let i = idx as usize;
    let s = table().get(i)?;
    (i != 0 && s.uses != 0 && u16::from(s.drive) == drive).then_some(i)
}

/// `dirtbl[run->p_curdir[drive]]` — the current directory of `drive` for
/// `run`; the root for a drive without one, a drive beyond `p_curdir[]`, or a
/// byte that names no live slot of that drive.
pub fn curdir(drive: u16) -> CurDir {
    if drive as usize >= NUMCURDIR {
        return CurDir::ROOT;
    }
    // SAFETY: as in `current_drive`.
    let idx = unsafe { (*run_pd()).p_curdir[drive as usize] };
    match live(idx, drive) {
        Some(i) => {
            let s = &table()[i];
            let mut c = CurDir::ROOT;
            c.len = s.len as usize;
            c.buf[..c.len].copy_from_slice(&s.path[..c.len]);
            c
        }
        None => CurDir::ROOT,
    }
}

/// `incr_curdir_usage` (`bdos/fsdir.c:1164-1188`): the slot holding
/// (`drive`, `path`) with one use more, or the first free slot taking it.
fn incr(drive: u16, path: &[u8]) -> Option<usize> {
    if let Some(i) = find(drive, path) {
        let s = &mut table()[i];
        s.uses = s.uses.saturating_add(1);
        return Some(i);
    }
    let i = match first_free() {
        Some(i) => i,
        None => {
            reclaim();
            first_free()?
        }
    };
    let s = &mut table()[i];
    s.uses = 1;
    s.drive = drive as u8;
    s.len = path.len() as u8;
    s.path = [0; PATH_MAX];
    s.path[..path.len()].copy_from_slice(path);
    Some(i)
}

/// The live slot holding (`drive`, `path`).
fn find(drive: u16, path: &[u8]) -> Option<usize> {
    let t = table();
    (1..NCURDIR).find(|&i| {
        let s = &t[i];
        s.uses != 0 && u16::from(s.drive) == drive && &s.path[..s.len as usize] == path
    })
}

/// The first free slot (slot 0 is never used, `bdos/fsdir.c:1172,1177`).
fn first_free() -> Option<usize> {
    let t = table();
    (1..NCURDIR).find(|&i| t[i].uses == 0)
}

/// `decr_curdir_usage` (`bdos/fsdir.c:1194-1218`): indices outside
/// `1..NCURDIR` are ignored; the count does not go below 0; at 0 the slot is
/// free.
fn decr(idx: u8) {
    let i = idx as usize;
    if i == 0 || i >= NCURDIR {
        return;
    }
    let s = &mut table()[i];
    s.uses = s.uses.saturating_sub(1);
}

/// `Dsetpath`'s store (`bdos/fsdir.c:1129-1151`): make `path` (canonical
/// `\A\B`, empty = root) the current directory of `drive` in `run`'s PD — the
/// old slot noted first, a use of the new slot taken, then the old dropped.
/// The old byte is judged **before** the new slot is taken (`olddir` read
/// before `findit`, `:1129-1134`): a stale byte (its slot freed by
/// [`free_drive`]) is the root `ckdrv` repairs it to (`bdos/fsdrive.c:121-130`)
/// and drops nothing, even when `incr` reuses that index (TASK/FS28).
/// `false` (and `run` unchanged) for a drive beyond `p_curdir[]`, a path
/// longer than [`PATH_MAX`] - 1, or a full table (EmuTOS's `EINTRN`, which
/// `xchdir` turns into `EPTHNF`).
pub fn set_curdir(drive: u16, path: &str) -> bool {
    if drive as usize >= NUMCURDIR || path.len() >= PATH_MAX {
        return false;
    }
    let pd = run_pd();
    // SAFETY: as in `current_drive`; a byte load from that PD.
    let old = unsafe { (*pd).p_curdir[drive as usize] };
    let old_live = live(old, drive).is_some();
    let new = if path.is_empty() {
        0
    } else {
        match incr(drive, path.as_bytes()) {
            Some(i) => i as u8,
            None => return false,
        }
    };
    // SAFETY: as in `current_drive`; a byte store into that PD.
    unsafe { (*pd).p_curdir[drive as usize] = new };
    if old_live {
        decr(old);
    }
    true
}

/// The second half of `init_pd_files` (`bdos/proc.c:401-409`) for the new PD
/// `p`, from the creating `run`, `from`: every `p_curdir[i]` copied with one
/// more use of its slot, then `p_curdrv`. A byte of `from` that names no live
/// slot of its drive is copied as 0 (the root it reads as).
///
/// # Safety
///
/// `p` and `from` are kernel pointers to live basepages; `p` is not `from`.
pub unsafe fn inherit(p: *mut BasePage, from: *const BasePage) {
    for d in 0..NUMCURDIR {
        // SAFETY: caller contract.
        let idx = unsafe { (*from).p_curdir[d] };
        let v = match live(idx, d as u16) {
            Some(i) => {
                let s = &mut table()[i];
                s.uses = s.uses.saturating_add(1);
                i as u8
            }
            None => 0,
        };
        // SAFETY: caller contract.
        unsafe { (*p).p_curdir[d] = v };
    }
    // SAFETY: caller contract.
    unsafe { (*p).p_curdrv = (*from).p_curdrv };
}

/// [`inherit`] from `run` ([`run_pd`]) — what `init_pd_files` does for a PD
/// the loader creates.
///
/// # Safety
///
/// `p` is the kernel pointer to a live basepage that is not `run`'s.
pub unsafe fn inherit_from_run(p: *mut BasePage) {
    // SAFETY: forwarded; `run_pd` is live.
    unsafe { inherit(p, run_pd()) }
}

/// The directory half of `ixterm` (`bdos/proc.c:126-132`) for the PD `p`
/// (kernel pointer): one use of every slot its bytes name dropped. The bytes
/// stay as they are (as in EmuTOS); a byte naming no live slot of its drive
/// is skipped.
///
/// # Safety
///
/// `p` is the kernel pointer to a live basepage, or null (nothing happens).
pub unsafe fn release(p: *const BasePage) {
    if p.is_null() {
        return;
    }
    for d in 0..NUMCURDIR {
        // SAFETY: caller contract.
        let idx = unsafe { (*p).p_curdir[d] };
        if live(idx, d as u16).is_some() {
            decr(idx);
        }
    }
}

/// [`release`] for the PD at user address `pd` (a PD pointer as
/// [`crate::bdos::process::pd_of`] gives it); null does nothing.
///
/// # Safety
///
/// `pd` is null or the user address of a live basepage.
pub unsafe fn release_pd(pd: *mut BasePage) {
    if pd.is_null() {
        return;
    }
    let kva = crate::bdos::process::kalias(pd as u64) as *const BasePage;
    // SAFETY: caller contract.
    unsafe { release(kva) };
}

/// The `dirtbl[]` half of EmuTOS `freetree` (`bdos/bdosmain.c:356-378`),
/// run by `osif`'s re-login after a media change on `drive`: every slot of
/// that drive is emptied (`dnd = NULL`, `use = 0`). The PDs' `p_curdir[]`
/// bytes are left as they are and read as the root from now on ([`curdir`]),
/// which is what `ckdrv`'s repair gives them in EmuTOS (`bdos/fsdrive.c:
/// 120-131`): after a media change every process's current directory on the
/// drive is its root (`docs/api/gemdos/process/_run-ownership-curdir.md`
/// §4.5). NewTOS keeps no DND tree, so this is all of `freetree`.
pub fn free_drive(drive: u16) {
    for s in table().iter_mut().skip(1) {
        if s.uses != 0 && u16::from(s.drive) == drive {
            *s = Slot::FREE;
        }
    }
}

/// Recount every slot's uses from the PDs that are alive — every process
/// slot with a basepage and every stand-in — and free the slots nobody names
/// (see the module notes; reference only).
fn reclaim() {
    let mut counts = [0u16; NCURDIR];
    let mut count = |p: *const BasePage| {
        if p.is_null() {
            return;
        }
        for d in 0..NUMCURDIR {
            // SAFETY: a live basepage (callers below).
            let idx = unsafe { (*p).p_curdir[d] };
            let i = idx as usize;
            let t = table();
            if i != 0 && i < NCURDIR && t[i].uses != 0 && usize::from(t[i].drive) == d {
                counts[i] = counts[i].saturating_add(1);
            }
        }
    };
    for p in crate::bdos::process::PROCESS_TABLE.iter() {
        let kva = p.basepage_kva.load(core::sync::atomic::Ordering::Acquire);
        count(kva);
    }
    for p in crate::bdos::process::standin::live_pds() {
        count(p);
    }
    let t = table();
    for (i, s) in t.iter_mut().enumerate().skip(1) {
        s.uses = counts[i];
    }
}

/// Number of `dirtbl[]` slots in use — for tests and diagnostics.
pub fn slots_in_use() -> usize {
    table().iter().skip(1).filter(|s| s.uses != 0).count()
}

/// The use count of the slot holding (`drive`, `path`), 0 if none — for
/// tests and diagnostics.
pub fn uses_of(drive: u16, path: &str) -> u16 {
    find(drive, path.as_bytes()).map_or(0, |i| table()[i].uses)
}
