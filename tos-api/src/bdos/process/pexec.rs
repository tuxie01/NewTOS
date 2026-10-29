//! `Pexec` (GEMDOS 0x4B) — Phase 06 D3.
//!
//! Spec: [`docs/api/gemdos/process/4b-pexec.md`].
//!
//! All EmuTOS modes: `PE_LOADGO` (0), `PE_LOAD` (3), `PE_GO` (4), and since
//! ARCH03 stage 2 `PE_BASEPAGE` (5), `PE_GOTHENFREE` (6) and
//! `PE_BASEPAGEFLAGS` (7) (EmuTOS `bdos/proc.c:179-361`). Anything else is
//! [`PexecError::UnknownMode`] (`EINVFN`).
//!
//! ## Pipeline (PE_LOADGO)
//!
//! 1. Resolve `path` via [`super::registry`] → `&'static [u8]` ELF
//!    blob (no filesystem yet — see registry doc).
//! 2. [`super::allocate_slot`] → reserves a fresh process slot for
//!    the child.
//! 3. [`super::loader::load_to_slot`] → parses the ELF, copies
//!    segments, populates the basepage, installs page tables.
//! 4. The kernel SVC handler then snapshots the parent's trap frame
//!    into `slot.parent_frame`, stamps `slot.parent_l0_pa` with
//!    the parent's `TTBR0_EL1`, sets `RUN = child.basepage_kva`,
//!    rewrites the trap frame to point at the child's entry, and
//!    swaps TTBR0 to the child.
//! 5. The post-handler `eret` lands in EL0 at the child's entry.
//!
//! Step 4 is in the kernel because it touches `ExceptionFrame`
//! (kernel-private). This module returns the metadata the handler
//! needs to perform that work.

use core::sync::atomic::Ordering;

use tos_types::BasePage;

use super::loader::{
    create_basepage, load_file_to_slot, load_to_slot_with, EnvSpec, LoadError, LoadedImage,
};
use super::registry;
use super::{allocate_slot, free_slot, PROCESS_TABLE};
use crate::bdos::fs::FileImage;

// ─── Mode constants (`include/bdosdefs.h:24-32`) ────────────────────

/// `PE_LOADGO` (0): load executable, basepage owned by child, launch.
pub const PE_LOADGO: u16 = 0;
/// `PE_LOAD` (3): load only, basepage owned by parent, return PD.
pub const PE_LOAD: u16 = 3;
/// `PE_GO` (4): launch already-loaded PD passed in `tail`.
pub const PE_GO: u16 = 4;
/// `PE_BASEPAGE` (5): allocate basepage only, `flags = 0`.
pub const PE_BASEPAGE: u16 = 5;
/// `PE_GOTHENFREE` (6): like `PE_GO`, then re-own basepage to child.
pub const PE_GOTHENFREE: u16 = 6;
/// `PE_BASEPAGEFLAGS` (7): allocate basepage, `flags = (ULONG)path`.
pub const PE_BASEPAGEFLAGS: u16 = 7;

// ─── Result + error types ───────────────────────────────────────────

/// Metadata the kernel SVC handler needs after a `Pexec` dispatch.
/// For a launch mode (`PE_LOADGO` / `PE_GO`) the handler redirects the
/// trap frame to the child and switches `TTBR0_EL1`; for `PE_LOAD` it
/// returns [`Self::handle`] to the parent without launching.
#[derive(Copy, Clone, Debug)]
pub struct LoadedChild {
    /// Slot index in [`PROCESS_TABLE`] holding the child.
    pub slot: usize,
    /// User VA of the child's entry point.
    pub entry_va: u64,
    /// Phys of the child's `L0` table — feeds `TTBR0_EL1`.
    pub l0_pa: u64,
    /// The child's **PD pointer** — the user VA of its
    /// [`tos_types::BasePage`] (the start of its TPA, VA = PA) — passed to
    /// the child entry in `x0` per NewTOS's AArch64 process ABI.
    pub basepage_va: u64,
    /// Initial `SP_EL0` for the child: the TPA top (`p_hitpa`), 16-byte
    /// aligned. The kernel launch path sets `SP_EL0` to this before the eret.
    pub sp_el0: u64,
    /// Bytes from the PD to the end of the image, BSS included
    /// ([`LoadedImage::image_len`]) — what the accessory loader keeps of the
    /// TPA (EmuTOS `pgmld`, `util/gemdos.c:40-43`). 0 for the go modes.
    pub image_len: u64,
    /// What `PE_LOAD` returns and `PE_GO` takes back (in `tail`): the
    /// child's **PD pointer** (`basepage_va`) — a real basepage
    /// address the caller can read (`p_tlen`, `p_env`, …) in the shared
    /// address space (ARCH03 stage 1), as EmuTOS returns `(long)cur_p`
    /// (`bdos/proc.c:360`).
    pub handle: u64,
    /// `true` for `PE_LOADGO` / `PE_GO` (kernel launches the child);
    /// `false` for `PE_LOAD` (kernel returns `handle` to the parent).
    pub launch: bool,
}

/// Why `Pexec` failed.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PexecError {
    /// `mode` is not one of the documented [`PE_LOADGO`] /
    /// [`PE_LOAD`] / etc. constants. Mapped to `EINVFN` (-32).
    UnknownMode,
    /// `path` did not match any registered program.
    /// Mapped to `EFILNF` (-33).
    NotFound,
    /// [`allocate_slot`] returned `None` — process table is full.
    /// Mapped to `ENSMEM` (-39).
    NoSlot,
    /// `PE_GO`'s `tail` is not the PD pointer of a program loaded with
    /// `PE_LOAD` (or a basepage of mode 5/7) and not yet started, or the
    /// caller does not own it. Mapped to `EIMBA` (-40) — FreeMiNT's
    /// `EFAULT` for a foreign or invalid PD (`sys/k_exec.c:372-381`), whose
    /// value is the same −40.
    BadHandle,
    /// No room to record the child as an owner of its basepage/environment
    /// ([`crate::bdos::mem::region`]). Mapped to `ENSMEM`.
    NoOwnerRoom,
    /// The ELF blob was malformed, or no free block holds its image + BSS
    /// (EmuTOS `alloc_tpa` → `NULL`), or the env pool allocation failed.
    /// Mapped to `EPLFMT` (-66) for parser errors and `ENSMEM` for the
    /// size/allocation shortfalls.
    LoadFailed(LoadError),
    /// The load met a media change that the restart did not get past: the
    /// re-login's `Getbpb` refused the drive, or the change was still there
    /// after `gemdos::RESTART_LIMIT` restarts (TASK/FS22). Mapped to `E_CHNG`
    /// (-14), what EmuTOS' `osif` returns for the refused `Getbpb`
    /// (`bdos/bdosmain.c:471-476`).
    MediaChange,
}

impl PexecError {
    /// The GEMDOS error code `Pexec` reports for this failure — the value a
    /// caller of `Pexec` (or of the shell loop's `dos_exec`) gets back.
    ///
    /// A *memory/size* shortfall maps to `ENSMEM` (EmuTOS `bdos/proc.c:295-300`:
    /// no TPA → `ENSMEM`); a malformed image / bad header maps to `EPLFMT`
    /// (`bdos/kpgmld.c:47-63`). `ProgramTooLarge` is a segment end beyond the
    /// address range — a size shortfall, so `ENSMEM`. A failed TPA pool
    /// allocation (`NoTpaMemory`) and the env allocation shortfall
    /// (`alloc_env`, `proc.c:284-288`) are likewise `ENSMEM`. A well-formed
    /// but ILP32 image (`Ilp32Unsupported`) is a format NewTOS refuses to
    /// run, so it takes the `EPLFMT` arm too.
    pub const fn errno(self) -> i32 {
        use tos_types::error;
        match self {
            PexecError::UnknownMode => error::EINVFN,
            PexecError::NotFound => error::EFILNF,
            PexecError::NoSlot | PexecError::NoOwnerRoom => error::ENSMEM,
            PexecError::BadHandle => error::EIMBA,
            PexecError::MediaChange | PexecError::LoadFailed(LoadError::MediaChange) => {
                error::E_CHNG
            }
            PexecError::LoadFailed(le) => match le {
                LoadError::ProgramTooLarge
                | LoadError::NoEnvMemory
                | LoadError::NoTpaMemory => error::ENSMEM,
                _ => error::EPLFMT,
            },
        }
    }
}

// ─── Mode dispatch ──────────────────────────────────────────────────

/// `Pexec(mode, path, tail, env)` — the GEMDOS-side dispatch.
///
/// `path_bytes` is the registry key / file path (NUL-terminated user
/// string; caller is responsible for the bytes it hands in — Pexec does
/// **not** scan past the slice's `len`).
///
/// The pointer arguments are overloaded by mode (EmuTOS `xexec`). For the
/// **load** modes (`PE_LOADGO` / `PE_LOAD`) and the **basepage** modes
/// (`PE_BASEPAGE` / `PE_BASEPAGEFLAGS`), `tail` is the command-tail image and
/// `env` the environment source (both materialised by the kernel SVC handler
/// from the caller's address space, since they are user pointers);
/// `path_value` is the raw `path` argument — the `PRGFLAGS` long of
/// `PE_BASEPAGEFLAGS`. For **`PE_GO` / `PE_GOTHENFREE`**, `go_handle` is the
/// PD pointer (the raw `tail` argument) of a program `PE_LOAD` or a basepage
/// mode produced; `tail`/`env` are unused.
///
/// # Safety
///
/// Caller (the kernel SVC handler) holds the BDOS giant lock; the
/// caller is the boot CPU. The slot allocated here lives until the
/// child terminates via [`super::pterm::terminate`].
pub unsafe fn dispatch(
    mode: u16,
    path_bytes: &[u8],
    path_value: u64,
    go_handle: u64,
    tail: &[u8],
    env: EnvSpec<'_>,
) -> Result<LoadedChild, PexecError> {
    match mode {
        // PE_LOADGO: load + launch, env+basepage owned by the child.
        // PE_LOAD: load only, basepage owned by the parent, returned as
        // a handle.
        PE_LOADGO => unsafe { load(path_bytes, tail, env, true, true) },
        PE_LOAD => unsafe { load(path_bytes, tail, env, false, false) },
        // PE_GO / PE_GOTHENFREE: launch the PD a prior PE_LOAD or
        // PE_BASEPAGE(FLAGS) returned; 6 hands its memory to the child.
        PE_GO => unsafe { go(go_handle, false) },
        PE_GOTHENFREE => unsafe { go(go_handle, true) },
        // PE_BASEPAGE: flags 0 (EmuTOS `path = 0`, `bdos/proc.c:211`);
        // PE_BASEPAGEFLAGS: flags = (ULONG)path (`:213`).
        PE_BASEPAGE => unsafe { create(0, tail, env) },
        PE_BASEPAGEFLAGS => unsafe { create(path_value as u32, tail, env) },
        _ => Err(PexecError::UnknownMode),
    }
}

/// `Pexec(PE_BASEPAGE / PE_BASEPAGEFLAGS)` — a basepage, no program
/// ([`create_basepage`], EmuTOS `bdos/proc.c:210-237`), returned as its PD
/// pointer. Its slot waits, like a `PE_LOAD` program's, for mode 4/6 or for
/// the end of the caller that owns it ([`release_unstarted_of`]).
///
/// # Safety
///
/// Caller's BDOS-lock contract; see [`dispatch`].
unsafe fn create(flags: u32, tail: &[u8], env: EnvSpec<'_>) -> Result<LoadedChild, PexecError> {
    // Environment, TPA, mode, root: one change of rights (MEMPROT-FAST).
    // SAFETY: BDOS lock held (caller contract); nothing here runs at EL0.
    let _batch = unsafe { super::memprot::batch() };
    let slot_idx = allocate_slot().ok_or(PexecError::NoSlot)?;
    // SAFETY: the slot is freshly claimed; BDOS lock held.
    let img = match unsafe { create_basepage(slot_idx, flags, tail, env) } {
        Ok(img) => img,
        Err(e) => {
            free_slot(slot_idx);
            return Err(PexecError::LoadFailed(e));
        }
    };
    PROCESS_TABLE[slot_idx]
        .load_owner
        .store(super::current(), Ordering::Release);
    Ok(LoadedChild {
        slot: slot_idx,
        entry_va: 0,
        l0_pa: img.l0_pa,
        basepage_va: img.basepage_va,
        sp_el0: 0,
        image_len: img.image_len,
        handle: img.basepage_va,
        launch: false,
    })
}

/// Load `path`'s program into a fresh slot. `launch` selects
/// `PE_LOADGO` (true: the kernel's `Pexec` handler transfers control to
/// the child) vs `PE_LOAD` (false: the handler returns the PD pointer to
/// the caller and the slot stays [`State::Loaded`]; the kernel launcher
/// and the accessory loader also pass `false` and start the program
/// themselves).
///
/// `owned_by_child` is EmuTOS' `owner = (flag == PE_LOADGO) ? p : run`
/// (`bdos/proc.c:302-307`) for the TPA and the environment block: `true`
/// for `PE_LOADGO` and for every program the kernel starts itself (its
/// `Pterm` frees them), `false` for `PE_LOAD` (the caller owns them and
/// may `Mfree` them). The `parent_slot` link and `p_parent` are set at
/// **launch** time ([`set_parent`] — EmuTOS `proc_go`, `bdos/proc.c:541`)
/// so `Pterm` returns to whoever launched it.
///
/// `tail` is the command-tail image copied verbatim into the child's
/// `p_cmdlin`; `env` is the environment source (`EnvSpec::Explicit` /
/// `Inherit`; `None` only for the legacy self-test loads — EmuTOS gives every
/// process an environment, `alloc_env`, `bdos/proc.c:413-432`).
///
/// # Safety
///
/// Caller's BDOS-lock contract; see [`dispatch`].
pub unsafe fn load(
    path_bytes: &[u8],
    tail: &[u8],
    env: EnvSpec<'_>,
    launch: bool,
    owned_by_child: bool,
) -> Result<LoadedChild, PexecError> {
    // EmuTOS runs `xexec` inside `osif`, so a media change met anywhere in
    // the load restarts the whole `Pexec` after the re-login
    // (`bdos/bdosmain.c:449-486`); every NewTOS caller of this function — the
    // `Pexec` trap handler, the kernel launcher, the accessory loader — is
    // outside `gemdos::dispatch`, so the load runs under the same restart
    // here (TASK/FS22). A run that met a change has failed and left nothing
    // behind — see [`load_once`] — so the restart starts clean, as after
    // `xexec`'s own clean-up (`bdos/proc.c:319-332`).
    // SAFETY: caller's BDOS-lock contract, forwarded to every run.
    let run = || unsafe { load_once(path_bytes, tail, env, launch, owned_by_child) };
    crate::gemdos::restartable(run).unwrap_or(Err(PexecError::MediaChange))
}

/// One run of [`load`] — EmuTOS `xexec`'s load modes from `ixsfirst` on
/// (`bdos/proc.c:255-360`).
///
/// A run that meets a media change fails and leaves nothing behind, which is
/// what the restart in [`load`] needs: before the slot is taken (resolving
/// the file, `ixsfirst`/`xopen`, `:256-268`) nothing is held; the loader
/// reads the header before it allocates (`kpgmhdrld`, "we haven't allocated
/// anything yet", `:271-274`) and frees the environment and the TPA on any
/// later failure, a media change included (`xmfree(cur_p->p_env);
/// xmfree(cur_p)`, `:322-332`; `loader::load_file_to_slot`); the slot is
/// released below. EmuTOS' third step, `xclose(fh)`, has no counterpart: a
/// [`FileImage`] is no open file (no `sft` entry, no OFD).
///
/// # Safety
///
/// Caller's BDOS-lock contract; see [`dispatch`].
unsafe fn load_once(
    path_bytes: &[u8],
    tail: &[u8],
    env: EnvSpec<'_>,
    launch: bool,
    owned_by_child: bool,
) -> Result<LoadedChild, PexecError> {
    // Resolve the program source without yet consuming a slot (so a
    // `NotFound` costs nothing): an embedded registry entry first (test
    // programs), otherwise open the FAT16 file for streaming. The file's
    // PT_LOAD segments are read straight into the mapped TPA by the
    // loader — there is no fixed intermediate buffer capping the program
    // size, so a multi-hundred-KiB GEM app loads the same as a tiny one.
    let blob: Option<&[u8]> = registry::lookup(path_bytes);
    let file: Option<FileImage> = match blob {
        Some(_) => None,
        None => {
            let end = path_bytes.iter().position(|&c| c == 0).unwrap_or(path_bytes.len());
            let path =
                core::str::from_utf8(&path_bytes[..end]).map_err(|_| PexecError::NotFound)?;
            Some(FileImage::open(path).ok_or(PexecError::NotFound)?)
        }
    };
    // Environment, TPA, mode, hand-over to the child, root: one change of
    // rights, entered when this guard drops (MEMPROT-FAST). The loader
    // writes the image through the kernel alias, never through user tables.
    // SAFETY: BDOS lock held (caller contract).
    let _batch = unsafe { super::memprot::batch() };
    let slot_idx = allocate_slot().ok_or(PexecError::NoSlot)?;

    // SAFETY: slot is freshly claimed; loader populates the slot's
    // basepage / root / entry / stack atomics, the command tail and the
    // environment.
    let loaded: Result<LoadedImage, LoadError> = match (blob, file) {
        (Some(b), _) => unsafe { load_to_slot_with(b, slot_idx, tail, env, owned_by_child) },
        (_, Some(f)) => unsafe { load_file_to_slot(f, slot_idx, tail, env, owned_by_child) },
        // Unreachable: `blob`/`file` resolved above, one is always `Some`.
        (None, None) => Err(LoadError::Parse(super::elf::ParseError::TooShort)),
    };
    let loaded: LoadedImage = match loaded {
        Ok(img) => img,
        Err(e) => {
            // Roll back the slot reservation so the table doesn't
            // leak entries on a malformed image.
            free_slot(slot_idx);
            return Err(PexecError::LoadFailed(e));
        }
    };

    // `PE_LOAD`: remember whose program this is, so the slot goes away with
    // its owner if it is never started ([`release_unstarted_of`]).
    if !launch {
        PROCESS_TABLE[loaded.slot]
            .load_owner
            .store(super::current(), Ordering::Release);
    }

    Ok(LoadedChild {
        slot: loaded.slot,
        entry_va: loaded.entry_va,
        l0_pa: loaded.l0_pa,
        basepage_va: loaded.basepage_va,
        sp_el0: loaded.sp_el0,
        image_len: loaded.image_len,
        handle: loaded.basepage_va,
        launch,
    })
}

/// Convenience `PE_LOADGO` launcher with no command tail and an
/// inherited environment (the common "run this program" case).
///
/// # Safety
///
/// See [`load`].
pub unsafe fn load_go(path_bytes: &[u8]) -> Result<LoadedChild, PexecError> {
    unsafe { load(path_bytes, &[], EnvSpec::Inherit, true, true) }
}

/// `Pexec(PE_GO / PE_GOTHENFREE, 0, tail, 0)` — launch an already-loaded
/// program. `tail` is the PD pointer a prior `PE_LOAD` or `PE_BASEPAGE(FLAGS)`
/// returned. Resolve it to its slot and hand the kernel a launch-mode
/// [`LoadedChild`].
///
/// Ownership (EmuTOS `bdos/proc.c:238-248`): `PE_GO` leaves it unchanged —
/// the TPA and the environment stay the caller's, and what the child
/// allocates is the child's (`run = p`), freed at its `Pterm` (plan E6);
/// `then_free` (`PE_GOTHENFREE`) first gives the basepage block and the block
/// `p_env` names to the child (`set_owner(p, p); set_owner(p->p_env, p)`), so
/// the child's `Pterm` frees them.
///
/// Where it starts: [`start_of`] — on a stack at the basepage's current
/// `p_hitpa`, at the ELF entry of a loaded program or the `p_tbase` of a
/// mode-5/7 basepage (EmuTOS `proc_go`, `:544,548`).
///
/// The child keeps the root the loader built for it. In stage 1 every root
/// shares the same tables except page 0, so "the caller's address space"
/// (plan E6, FreeMiNT `FORK_SHAREVM`) is the child's too; page 0 stays the
/// child's own so its `os_run` cell names the child and its KEYS02 table is
/// its own (plan E3/E4).
///
/// # Safety
///
/// Caller holds the BDOS giant lock.
pub unsafe fn go(handle: u64, then_free: bool) -> Result<LoadedChild, PexecError> {
    use crate::bdos::mem::{region, traps};
    // The ownership and memory-space changes below are one change of rights,
    // entered when this guard drops (MEMPROT-FAST). Nothing here runs at EL0
    // or touches user memory through the user tables.
    // SAFETY: BDOS lock held (caller contract).
    let _batch = unsafe { super::memprot::batch() };
    let slot_idx = resolve_handle(handle).ok_or(PexecError::BadHandle)?;
    let slot = &PROCESS_TABLE[slot_idx];
    // Only an owner of the PD may start it (FreeMiNT: `addr2mem(curproc,
    // base)` → `EFAULT`, `sys/k_exec.c:372-381`; plan E15). The caller owns
    // it as itself or through the memory space it shares (a mode-4 child).
    let pd = handle as u32 as tos_types::Long;
    let me = traps::current_owner();
    let space: *mut core::ffi::c_void = super::current_space().cast();
    // SAFETY: BDOS lock held.
    let caller = unsafe {
        if region::is_owner(pd, me) {
            me
        } else if !space.is_null() && region::is_owner(pd, space) {
            space
        } else {
            return Err(PexecError::BadHandle);
        }
    };
    let kva = slot.basepage_kva.load(Ordering::Acquire);
    // SAFETY: `kva` is the kernel alias of the slot's live basepage (the
    // slot resolved, so its image is loaded).
    let bp = unsafe { &*kva };
    let env = bp.p_env;
    let (entry_va, sp_el0) = start_of(slot.entry_va.load(Ordering::Acquire), bp);
    // The child becomes an owner of its basepage and its environment
    // (FreeMiNT `attach_region(p, env); attach_region(p, base)`,
    // `sys/k_exec.c:518-520`), so it may shrink/free them (E15). Mode 6 then
    // drops the caller (`mkfree` → `detach_region`, `:586-590`) — EmuTOS'
    // `set_owner(p, p); set_owner(p->p_env, p)` (`bdos/proc.c:238-242`).
    let child = handle as *mut core::ffi::c_void;
    // SAFETY: BDOS lock held. An `env` that starts no block (0, or not a
    // pool block) has nobody to own it: `NoBlock` is not an error here.
    unsafe {
        for blk in [pd, env] {
            let r = if then_free {
                region::hand_over(blk, caller, child)
            } else {
                region::attach(blk, child)
            };
            match r {
                Ok(()) | Err(region::RegionError::NoBlock) => {}
                Err(_) => return Err(PexecError::NoOwnerRoom),
            }
        }
    }
    // A mode-4 child runs in its caller's memory space (plan E6, FreeMiNT
    // `FORK_SHAREVM`): it owns what the caller owns. Mode 6 starts a process
    // of its own (FreeMiNT forks without `FORK_SHAREVM`).
    // An EL1 caller (no running process) lends no memory space.
    let space_pd = if then_free || super::current().is_null() {
        core::ptr::null_mut()
    } else if !space.is_null() {
        space as *mut BasePage
    } else {
        me as *mut BasePage
    };
    // SAFETY: BDOS lock held. Stores `space_pd` and enters what the new
    // grouping changes in the rights (MEMPROT-FAST).
    unsafe { super::memprot::set_space(slot_idx, space_pd) };
    slot.load_owner.store(core::ptr::null_mut(), Ordering::Release);
    Ok(LoadedChild {
        slot: slot_idx,
        entry_va,
        l0_pa: slot.l0_pa.load(Ordering::Acquire),
        basepage_va: super::pd_of(slot_idx) as u64,
        sp_el0,
        image_len: 0,
        handle,
        launch: true,
    })
}

/// Where `Pexec` 4/6 start a PD (EmuTOS `proc_go`, `bdos/proc.c:536-577`):
/// on a stack at `p_hitpa` as the basepage says **now** — the caller may
/// have lowered it after an `Mshrink` (the shell idiom) — rounded down to 16
/// for AAPCS64 (`:544`); at the program's ELF entry (`entry_va`, which the
/// SDK links at `p_tbase`), or, for a basepage of mode 5/7 (`entry_va == 0`,
/// nothing loaded), at the `p_tbase` its caller wrote (`:548`).
pub fn start_of(entry_va: u64, bp: &BasePage) -> (u64, u64) {
    let entry = if entry_va != 0 { entry_va } else { bp.p_tbase as u32 as u64 };
    (entry, (bp.p_hitpa as u32 as u64) & !0xF)
}

/// Find the slot holding a program that was loaded with `PE_LOAD` and not
/// started yet, by its PD pointer. A desk accessory is `Loaded` too until
/// its first dispatch, but it sits in the cooperative run-set from its
/// `pstart` on (`aes::acc::register_process`), so it never matches.
fn resolve_handle(handle: u64) -> Option<usize> {
    if handle == 0 {
        return None;
    }
    (0..PROCESS_TABLE.len()).find(|&i| {
        let p = &PROCESS_TABLE[i];
        p.read_state() == super::State::Loaded
            && !p.runnable.load(Ordering::Acquire)
            && super::pd_of(i) as u64 == handle
    })
}

/// Record the launch linkage of `child` (EmuTOS `proc_go`,
/// `bdos/proc.c:541` — `p->p_parent = run`): the child's `p_parent`
/// becomes the caller's PD pointer; for a **desk accessory** caller
/// EmuTOS's `run` instead — the program in front, or the accdesk PD between
/// programs ([`super::run_for_caller`], TASK/PROC03 B5,
/// `_run-ownership-curdir.md` §1.3); when the caller is no process (an EL1
/// caller such as the in-kernel EmuCON), the stand-in PD that is EmuTOS's
/// `run` there — the EmuCON PD while EmuCON runs ([`write_parent`],
/// TASK/PPARENT, TASK/PROC03). The rights the child inherits follow the
/// **real** caller — the accessory itself; none from a kernel caller — as
/// does its `Pterm` return (`parent_slot`, set by the caller of this
/// function).
///
/// # Safety
///
/// Caller holds the BDOS giant lock; `child` holds a loaded image.
pub unsafe fn set_parent(child: usize, parent_pd: *mut BasePage) {
    // SAFETY: caller contract.
    unsafe { write_parent(child, parent_pd) };
    // Its rights as a started process: a `Pexec(4)` child those of the space
    // it shares, any other child of a process the parent's blocks it
    // inherits unless strict (ARCH03 3b, E5/E6).
    // SAFETY: BDOS lock held (caller contract).
    unsafe { super::memprot::on_launch(child, parent_pd) };
}

/// `p_parent` of a program the **kernel launcher** starts (`kernel/src/
/// launch.rs`: the AUTO folder, the desktop's and the `#Z` program — EmuTOS
/// `Pexec(PE_LOADGO)` from the BIOS, `bios/bios.c:693`, and `dos_exec` from
/// the shell, `aes/gemshlib.c:645`): `run` there is kernel code, so the
/// child's parent is the stand-in PD of that moment —
/// [`super::standin::kernel_run`]: `initial_basepage` before the AES, the
/// accdesk PD in the shell loop (TASK/PPARENT, corrected by TASK/PROC03:
/// `aes/gemshlib.c:600-611,645` run in the accdesk process). Only the
/// basepage field: the launcher changes no rights (a kernel caller lends
/// none).
///
/// # Safety
///
/// Caller holds the BDOS giant lock; `child` holds a loaded image.
pub unsafe fn set_kernel_parent(child: usize) {
    // SAFETY: caller contract.
    unsafe { write_parent(child, core::ptr::null_mut()) };
}

/// `p->p_parent = run` (EmuTOS `proc_go`, `bdos/proc.c:541`) with NewTOS's
/// `run` ([`super::run_for_caller`]): `parent_pd`; [`super::emutos_run`]
/// when `parent_pd` is a desk accessory (TASK/PROC03 B5); or — `null`, no
/// process is running — the stand-in PD [`super::standin::kernel_run`]
/// (still `0` before the stand-ins exist). Then `run = p` (`:575`): the
/// child is the innermost started process ([`super::note_started`]).
///
/// # Safety
///
/// Caller holds the BDOS giant lock; `child` holds a loaded image.
unsafe fn write_parent(child: usize, parent_pd: *mut BasePage) {
    let kva = PROCESS_TABLE[child].basepage_kva.load(Ordering::Acquire);
    if kva.is_null() {
        return;
    }
    let run = super::run_for_caller(parent_pd);
    let value = run as u64 as tos_types::Long;
    // SAFETY: `kva` is the kernel alias of the child's live basepage;
    // `p_parent` is a plain 32-bit LONG (user addresses are below 4 GiB).
    unsafe { (*kva).p_parent = value };
    // What `Pterm` compares the live field with (TASK/PROC03 B4,
    // `pterm::return_target`).
    PROCESS_TABLE[child].launch_parent.store(value as u32, Ordering::Release);
    // `run = p` (`bdos/proc.c:575`): the innermost started process now.
    super::note_started(child);
}

/// Release every slot holding a program that `owner` loaded with `PE_LOAD`
/// and never started — called when `owner` terminates. Its TPA and
/// environment were `owner`'s blocks (`owner = run`) and go with
/// `free_all_owned`; the slot would otherwise outlive them.
pub fn release_unstarted_of(owner: *mut BasePage) {
    if owner.is_null() {
        return;
    }
    for (i, p) in PROCESS_TABLE.iter().enumerate() {
        if p.read_state() == super::State::Loaded
            && p.load_owner.load(Ordering::Acquire) == owner
        {
            free_slot(i);
        }
    }
}

/// The last owner freed the block at `pd` (an `Mfree`/`Mshrink(0)` by a
/// program): if a PD waiting for `Pexec` 4/6 lived there, it is gone — its
/// slot is released, so nobody can start freed memory.
pub fn release_unstarted_at(pd: u64) {
    if pd == 0 {
        return;
    }
    if let Some(i) = resolve_handle(pd) {
        if !PROCESS_TABLE[i].load_owner.load(Ordering::Acquire).is_null() {
            free_slot(i);
        }
    }
}

/// Convenience accessor for the child slot's [`BasePage`] VA on the
/// kernel side — the handler reads this so it can stamp `RUN` to
/// the child's basepage right before the eret.
#[inline]
pub fn child_basepage_kva(slot: usize) -> *mut BasePage {
    PROCESS_TABLE[slot].basepage_kva.load(Ordering::Acquire)
}
