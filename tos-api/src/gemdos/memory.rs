//! GEMDOS memory group (`Maddalt`, `Mxalloc`, `Malloc`, `Mfree`,
//! `Mshrink`).
//!
//! The trap entries in this module decode the AAPCS64 argument
//! registers and forward into [`crate::bdos::mem::traps`]. Argument
//! ordering follows the Atari ABI exactly so source-level ports see
//! the same parameters in the same positions.
//!
//! ## One address space (ARCH03 stage 1)
//!
//! Every user address is its own physical address and both pools are
//! mapped for every process ([`crate::bdos::process::mmu`]), so the pool
//! allocator's answer **is** the answer: `Malloc`/`Mxalloc` return the
//! block's `m_start`, `Mfree`/`Mshrink` take it back, and `Malloc(-1)` is the
//! largest free block of the selected pool — EmuTOS `bdos/umem.c`, for an
//! EL0 program and for a kernel caller alike. (Until stage 1 an EL0 caller
//! got a per-process window address instead, ADR-021; that window and its
//! one-page alignment slack are gone.)
//!
//! What still depends on the caller: `Malloc` picks its pool from the
//! running process's `PF_TTRAMMEM` flag (EmuTOS `xmalloc`); `Mfree` and
//! `Mshrink` act only for an **owner** of the block (ARCH03 stage 2d, plan
//! E15 — FreeMiNT's rule, `sys/dosmem.c:330-455`), and `Mfree` drops only the
//! caller's ownership: the block goes back to the pool when its last owner
//! lets go ([`crate::bdos::mem::region`]); and a block that went back to the
//! pool is reported to the AES ([`crate::aes::resource::on_mfree`]), which
//! may hold it as a resource blob.

use crate::bdos::mem::region::{self, Detached, RegionError};
use crate::bdos::mem::traps;
use crate::bdos::process;
use crate::trap::TrapArgs;
use tos_types::error::{EIMBA, E_OK};
use tos_types::{Long, MX_PREFTTRAM, MX_STRAM, PF_TTRAMMEM};

/// `Maddalt(start, size)` — GEMDOS 0x14.
///
/// Spec:
/// [`docs/api/gemdos/memory/14-maddalt.md`](../../../../docs/api/gemdos/memory/14-maddalt.md).
///
/// Argument register layout:
/// - `x0` = `start`  (Atari `LONG`, sign-extended to `i64` on entry)
/// - `x1` = `size`   (Atari `LONG`)
///
/// `Maddalt` registers kernel-trusted physical RAM and is never an EL0-app
/// operation, so it always forwards to the raw pool primitive.
pub fn maddalt(args: &TrapArgs) -> Long {
    let start = args.arg_i32(0);
    let size = args.arg_i32(1);
    // SAFETY: NewTOS is single-CPU pre-Phase 06 and SVC handlers run
    // with IRQs masked, so the BDOS layer is exclusively held for
    // the duration of the dispatch. Phase 06 will replace this with
    // an explicit BDOS giant lock.
    unsafe { crate::bdos::mem::traps::maddalt(start, size) }
}

/// `Mxalloc(amount, mode)` — GEMDOS 0x44.
///
/// Spec:
/// [`docs/api/gemdos/memory/44-mxalloc.md`](../../../../docs/api/gemdos/memory/44-mxalloc.md).
///
/// Argument register layout:
/// - `x0` = `amount` (Atari `LONG`)
/// - `x1` = `mode`   (Atari `WORD`)
///
/// The returned pointer is the block's address, valid in every process
/// (VA = PA); `amount == -1` returns the largest free block of the selected
/// pool(s) (EmuTOS `xmxalloc`, `bdos/umem.c`).
pub fn mxalloc(args: &TrapArgs) -> Long {
    let amount = args.arg_i32(0);
    let mode = args.arg_i16(1);
    // SAFETY: NewTOS is single-CPU and SVC handlers run with IRQs masked, so
    // the BDOS layer is exclusively held for the duration of the dispatch.
    pointer_result("Mxalloc", amount, unsafe { traps::mxalloc(amount, mode) })
}

/// `Malloc(amount)` — GEMDOS 0x48.
///
/// Spec:
/// [`docs/api/gemdos/memory/48-malloc.md`](../../../../docs/api/gemdos/memory/48-malloc.md).
///
/// Argument register layout:
/// - `x0` = `amount` (Atari `LONG`)
///
/// Policy wrapper around `Mxalloc`: EmuTOS `xmalloc` picks the mode from the
/// running process's `PF_TTRAMMEM` flag (`bdos/umem.c:116`). A kernel caller
/// (no running process) keeps `traps::malloc` (kernel sentinel =
/// `PF_TTRAMMEM`).
pub fn malloc(args: &TrapArgs) -> Long {
    let amount = args.arg_i32(0);
    let run = process::current_kva();
    if run.is_null() {
        // SAFETY: see `mxalloc`.
        return pointer_result("Malloc", amount, unsafe { traps::malloc(amount) });
    }
    // EmuTOS `xmalloc`: `PF_TTRAMMEM ? MX_PREFTTRAM : MX_STRAM`, read from the
    // running process's own basepage.
    // SAFETY: `run` is the kernel alias of the running process's basepage.
    let flags = unsafe { (*run).p_flags };
    let mode = if flags & PF_TTRAMMEM != 0 {
        MX_PREFTTRAM
    } else {
        MX_STRAM
    };
    // SAFETY: see `mxalloc`.
    pointer_result("Malloc", amount, unsafe { traps::mxalloc(amount, mode) })
}

/// The `Malloc`/`Mxalloc` result `ret` of a call with `amount`, checked on its
/// way out (TASK/MEM11): unless `amount == -1` — the largest-free-block probe,
/// whose answer is a size — `ret` is a block address or `0`, and must be a
/// user pointer below `0x8000_0000`
/// ([`crate::bdos::process::usermem::debug_assert_user_ptr`]). Neither
/// function returns a negative error code (`traps::mxalloc`: `0` on failure),
/// so a negative `ret` here is an address that would be misread as one.
fn pointer_result(func: &str, amount: Long, ret: Long) -> Long {
    if amount != -1 {
        process::usermem::debug_assert_user_ptr(func, ret as u32 as u64);
    }
    ret
}

/// Which identity of the caller owns the block at `addr`: its own PD (or the
/// kernel sentinel for an EL1 caller, [`traps::current_owner`]) first, then
/// the memory space it shares as a `Pexec(PE_GO)` child
/// ([`process::current_space`], plan E6). `None`: not an owner.
///
/// # Safety
///
/// BDOS giant lock held.
unsafe fn owning_identity(addr: Long) -> Option<*mut core::ffi::c_void> {
    let me = traps::current_owner();
    let space: *mut core::ffi::c_void = process::current_space().cast();
    // SAFETY: lock held (caller contract).
    unsafe {
        if region::is_owner(addr, me) {
            Some(me)
        } else if !space.is_null() && region::is_owner(addr, space) {
            Some(space)
        } else {
            None
        }
    }
}

/// `Mfree` semantics on behalf of the running process (E15): drop the
/// caller's ownership of the block at `addr`; the block is freed when it was
/// the last owner, and then the AES and the process table hear of it.
/// `EIMBA` for an address that starts no block or a block the caller does
/// not own.
///
/// # Safety
///
/// BDOS giant lock held.
unsafe fn release_as_caller(addr: Long) -> Long {
    // SAFETY: lock held (caller contract).
    let Some(who) = (unsafe { owning_identity(addr) }) else {
        // Not the caller's: an `F_KEEP` block the system inherited may be
        // freed by anybody (FreeMiNT `sys_m_free`, `sys/dosmem.c:365-398`).
        // SAFETY: lock held.
        if unsafe { region::release_kept(addr) } {
            note_user_free(addr);
            return E_OK;
        }
        return EIMBA;
    };
    // SAFETY: lock held.
    match unsafe { region::detach(addr, who) } {
        Ok(Detached::Freed) => {
            note_user_free(addr);
            // A PD that was waiting for `Pexec` 4/6 lived in this block.
            process::pexec::release_unstarted_at(addr as u32 as u64);
            E_OK
        }
        Ok(Detached::Kept) => E_OK,
        Err(RegionError::NoBlock | RegionError::NotOwner | RegionError::Full) => EIMBA,
    }
}

/// Tell the AES that a program freed the block at `m` — it may be a
/// resource blob or icon buffer `rsrc_load` registered, which is an
/// ordinary process-owned block in EmuTOS too (`rs_readit` →
/// `dos_alloc_anyram`, `aes/gemrslib.c:775`) and freed by whoever passes
/// its address — TeraDesk's `xrsrc_free` = `Mfree(ap_rscmem)` (A76,
/// `docs/api/gemdos/memory/49-mfree.md` § "AES resource block"). The AES
/// entry must not free that key a second time on `rsrc_free`.
fn note_user_free(m: Long) {
    crate::aes::resource::on_mfree(m);
}

/// `Mfree(addr)` — GEMDOS 0x49.
///
/// Spec:
/// [`docs/api/gemdos/memory/49-mfree.md`](../../../../docs/api/gemdos/memory/49-mfree.md).
///
/// Argument register layout:
/// - `x0` = `addr` (Atari `LONG`, the block start address)
///
/// `EIMBA` when `addr` starts no allocated block (EmuTOS `xmfree`,
/// `bdos/umem.c:135-159`) — and, since ARCH03 stage 2d, when the caller is
/// not an owner of it (plan E15, FreeMiNT `sys_m_free`'s `EFAULT`, the same
/// −40). An owner's `Mfree` drops its ownership; the block returns to the
/// pool with its last owner (FreeMiNT `detach_region`).
pub fn mfree(args: &TrapArgs) -> Long {
    let addr = args.arg_i32(0);
    // SAFETY: see `mxalloc`.
    unsafe { release_as_caller(addr) }
}

/// `Mshrink(n, blk, len)` — GEMDOS 0x4A.
///
/// Spec:
/// [`docs/api/gemdos/memory/4a-mshrink.md`](../../../../docs/api/gemdos/memory/4a-mshrink.md).
///
/// Argument register layout:
/// - `x0` = `n`   (Atari `WORD`, ignored — historic CP/M field)
/// - `x1` = `blk` (Atari `LONG`, block start address)
/// - `x2` = `len` (Atari `LONG`, new block length in bytes)
///
/// `blk` is a block address as `Malloc` returned it — including a basepage,
/// which is the start of its TPA block (EmuTOS `xsetblk`, `bdos/umem.c:170`).
/// `len == 0` frees the whole block (EmuTOS's `Mshrink(len=0) == Mfree`).
/// Only an owner may shrink (ARCH03 stage 2d, plan E15 — FreeMiNT
/// `sys_m_shrink` finds only the caller's regions, `sys/dosmem.c:410-455`);
/// anyone else gets `EIMBA`, and `len == 0` follows `Mfree`'s owner rule.
pub fn mshrink(args: &TrapArgs) -> Long {
    let n = args.arg_i16(0);
    let blk = args.arg_i32(1);
    let len = args.arg_i32(2);
    // SAFETY: see `mxalloc`.
    unsafe {
        if owning_identity(blk).is_none() {
            return EIMBA;
        }
        if len == 0 {
            return release_as_caller(blk);
        }
        traps::mshrink(n, blk, len)
    }
}

#[cfg(test)]
mod tests {
    use super::pointer_result;

    #[test]
    fn a_size_probe_is_not_checked_as_a_pointer() {
        // `Malloc(-1)` answers a size; even one that is not a valid pointer
        // passes through unchecked.
        assert_eq!(pointer_result("Malloc", -1, 0x8000_0000_u32 as i32), i32::MIN);
        assert_eq!(pointer_result("Mxalloc", -1, -1), -1);
    }

    #[test]
    fn a_failed_allocation_and_the_last_low_address_pass() {
        assert_eq!(pointer_result("Malloc", 16, 0), 0);
        assert_eq!(pointer_result("Mxalloc", 16, 0x7FFF_F000), 0x7FFF_F000);
    }

    #[test]
    #[cfg_attr(debug_assertions, should_panic(expected = "Malloc: user pointer 0x80000000"))]
    fn a_block_at_2_gib_is_caught_in_a_debug_build() {
        pointer_result("Malloc", 16, 0x8000_0000_u32 as i32);
    }
}
