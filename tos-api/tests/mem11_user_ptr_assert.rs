//! MEM11 — the debug-build check on pointer results leaving the kernel as a
//! `LONG`: `Malloc`, `Mxalloc` and `Fgetdta` (`TASK/MEM11.md`).
//!
//! A TOS address is a signed 32-bit `LONG`; the GEMDOS return path
//! sign-extends it (`kernel/src/arch/aarch64/exceptions.rs`), so a user
//! pointer at or above `0x8000_0000` reads as a negative error code. The
//! check ([`tos_api::bdos::process::usermem::debug_assert_user_ptr`]) fires
//! on such a pointer in a debug build; the size answer of the `-1` probe
//! (EmuTOS `xmxalloc`, `references/emutos/bdos/umem.c:247-276`) and the `0`
//! failure answer are not pointers to it. The `should_panic` cases are
//! `debug_assertions`-only: a release build compiles the check away.
//!
//! A debug assertion checks a kernel invariant, never user input (user
//! decision 2026-10-26): `Fgetdta` checks only the DTA the kernel gave the
//! process (`&p_cmdlin`, EmuTOS `bdos/proc.c:375`), never one the program
//! set with `Fsetdta`, which takes any address (EmuTOS `xsetdta`,
//! `bdos/fsmain.c:403-405`).

use std::sync::{Mutex, MutexGuard, Once};

use tos_api::bdos::mem::stats::{pool_stats, PoolKind};
use tos_api::bdos::fs::find::{debug_assert_fgetdta, default_dta};
use tos_api::bdos::process::usermem::{debug_assert_user_ptr, USER_PTR_END};
use tos_api::bdos::process::{self, PROCESS_TABLE};
use tos_api::trap::TrapArgs;
use tos_api::uaccess::Space;
use tos_types::{BasePage, Long, MX_PREFTTRAM, MX_STRAM, PF_TTRAMMEM};

const ST_START: u64 = 0x0040_0000;
const ST_LEN: u64 = 10 * 1024 * 1024;
const TT_START: u64 = 0x0100_0000;
const TT_LEN: u64 = 64 * 1024 * 1024;

static LOCK: Mutex<()> = Mutex::new(());
static SETUP: Once = Once::new();

fn lock() -> MutexGuard<'static, ()> {
    let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    SETUP.call_once(|| {
        // SAFETY: once per binary, before any allocation.
        unsafe { tos_api::bdos::mem::init::init(ST_START, ST_LEN, TT_START, TT_LEN) }
            .expect("the test pools");
    });
    g
}

/// A running process with a 256-byte basepage, its `p_flags` and `p_xdta`
/// set.
struct Proc {
    slot: usize,
    bp: *mut BasePage,
}

impl Proc {
    fn new(flags: u32, xdta: u32) -> Proc {
        let slot = process::allocate_slot().expect("a slot");
        let mut b = Box::new([0u8; 256]);
        b[0x20..0x24].copy_from_slice(&xdta.to_le_bytes()); // p_xdta
        b[0x28..0x2C].copy_from_slice(&flags.to_le_bytes()); // p_flags
        let bp = Box::into_raw(b) as *mut BasePage;
        PROCESS_TABLE[slot]
            .basepage_kva
            .store(bp, std::sync::atomic::Ordering::Release);
        process::set_current(process::pd_of(slot));
        Proc { slot, bp }
    }
}

impl Drop for Proc {
    fn drop(&mut self) {
        process::set_current(std::ptr::null_mut());
        process::free_slot(self.slot);
        // SAFETY: the slot no longer names the box.
        drop(unsafe { Box::from_raw(self.bp as *mut [u8; 256]) });
    }
}

fn args(func: u64, regs: [u64; 2], space: Space) -> TrapArgs {
    TrapArgs {
        regs: [regs[0], regs[1], 0, 0, 0, 0, 0, 0],
        func,
        svc_imm: 1,
        space,
    }
}

fn malloc(amount: i64) -> Long {
    tos_api::gemdos::memory::malloc(&args(0x48, [amount as u64, 0], Space::User))
}

fn mxalloc(amount: i64, mode: i16) -> Long {
    tos_api::gemdos::memory::mxalloc(&args(0x44, [amount as u64, mode as u16 as u64], Space::User))
}

fn mfree(a: Long) -> Long {
    tos_api::gemdos::memory::mfree(&args(0x49, [a as u64, 0], Space::User))
}

fn fgetdta(space: Space) -> Long {
    tos_api::bdos::fs::find::fgetdta(&args(0x2F, [0, 0], space))
}

#[test]
fn the_last_address_below_2_gib_and_null_pass() {
    assert_eq!(USER_PTR_END, 0x8000_0000);
    debug_assert_user_ptr("Malloc", 0);
    debug_assert_user_ptr("Malloc", 0x7FFF_FFFF);
}

#[test]
#[cfg_attr(debug_assertions, should_panic(expected = "Mxalloc: user pointer 0x80000000"))]
fn the_first_address_at_2_gib_fires() {
    debug_assert_user_ptr("Mxalloc", 0x8000_0000);
}

#[test]
fn the_size_probes_and_real_blocks_pass_through_the_trap_entries() {
    let _g = lock();
    let _p = Proc::new(PF_TTRAMMEM, ST_START as u32);
    let st = pool_stats(PoolKind::Stram).largest_free;
    let tt = pool_stats(PoolKind::Ttram).largest_free;
    // The `-1` probe answers a size, not a pointer.
    assert_eq!(malloc(-1), st.max(tt));
    assert_eq!(mxalloc(-1, MX_STRAM), st);
    assert_eq!(mxalloc(-1, MX_PREFTTRAM), st.max(tt));
    // A real block is a pool address below 2 GiB; a refusal is 0.
    let a = malloc(4096);
    assert!(a > 0 && (a as u64) < USER_PTR_END, "{a:#x}");
    let b = mxalloc(4096, MX_STRAM);
    assert!(b > 0 && (b as u64) < USER_PTR_END, "{b:#x}");
    assert_eq!(malloc(0), 0);
    assert_eq!(mxalloc(i32::MAX as i64, MX_STRAM), 0);
    assert_eq!(mfree(a), 0);
    assert_eq!(mfree(b), 0);
}

#[test]
fn a_program_dta_below_2_gib_passes() {
    let _g = lock();
    let _p = Proc::new(0, 0x0040_0080);
    assert_eq!(fgetdta(Space::User), 0x0040_0080);
}

/// A program may `Fsetdta` any address: one at or above 2 GiB comes back
/// from `Fgetdta` as the same 32 bits (a negative `LONG`, as EmuTOS's
/// `return run->p_xdta`), without the debug check firing.
#[test]
fn a_program_set_dta_at_or_above_2_gib_round_trips() {
    let _g = lock();
    let p = Proc::new(0, 0);
    let pd = process::pd_of(p.slot) as u64;
    for want in [0x8000_0000u32, 0xDEAD_0000, 0xFFFF_FFF0] {
        assert_ne!(default_dta(pd), want as u64, "test address collides with the default DTA");
        tos_api::bdos::fs::find::fsetdta(&args(0x1A, [want as u64, 0], Space::User));
        assert_eq!(fgetdta(Space::User), want as Long, "{want:#x}");
    }
}

/// The default DTA is `&p_cmdlin` of the PD, as the 32-bit `p_xdta` holds it.
#[test]
fn the_default_dta_is_p_cmdlin() {
    assert_eq!(default_dta(0x0040_0000), 0x0040_0080);
    assert_eq!(default_dta(0x7FFF_FF80), 0x8000_0000);
    assert_eq!(default_dta(0x1_0040_0000), 0x0040_0080, "the 32 bits p_xdta keeps");
}

#[test]
fn a_kernel_given_dta_below_2_gib_passes() {
    debug_assert_fgetdta(0x0040_0000, 0x0040_0080);
    debug_assert_fgetdta(0x7FFF_FE00, 0x7FFF_FE80);
}

/// The kernel gave the process a basepage whose `p_cmdlin` sits at 2 GiB:
/// a broken kernel invariant, which the debug build reports.
#[test]
#[cfg_attr(debug_assertions, should_panic(expected = "Fgetdta: user pointer 0x80000000"))]
fn a_kernel_given_dta_at_2_gib_fires() {
    debug_assert_fgetdta(0x7FFF_FF80, 0x8000_0000);
}

/// The check is on the full `pd + 0x80`, not on the 32 bits `p_xdta` keeps:
/// a basepage above 4 GiB whose truncated `&p_cmdlin` looks low still fires.
#[test]
#[cfg_attr(debug_assertions, should_panic(expected = "Fgetdta: user pointer 0x100400080"))]
fn a_kernel_given_dta_above_4_gib_fires_on_the_full_address() {
    debug_assert_fgetdta(0x1_0040_0000, 0x0040_0080);
}

/// A program-set DTA is user input: at or above 2 GiB it passes, also next
/// to a basepage whose default would fire.
#[test]
fn a_program_set_dta_never_fires() {
    debug_assert_fgetdta(0x0040_0000, 0x8000_0000);
    debug_assert_fgetdta(0x0040_0000, 0xDEAD_0000);
    debug_assert_fgetdta(0x7FFF_FF80, 0xDEAD_0000);
}

/// Through the trap entry: the default DTA of a running process is checked.
/// On the host the test basepage is heap memory; whether its `&p_cmdlin`
/// is below 2 GiB decides the expected outcome.
#[test]
fn the_trap_entry_checks_the_default_dta() {
    let _g = lock();
    let p = Proc::new(0, 0);
    let pd = process::pd_of(p.slot) as u64;
    let dflt = default_dta(pd);
    // SAFETY: `p.bp` is the test's own 256-byte basepage box.
    unsafe { (*p.bp).p_xdta = dflt as u32 as Long };
    let r = std::panic::catch_unwind(|| fgetdta(Space::User));
    let fires = cfg!(debug_assertions) && pd + 0x80 >= USER_PTR_END;
    match r {
        Ok(v) => {
            assert!(!fires, "expected the debug check to fire for pd {pd:#x}");
            assert_eq!(v, dflt as u32 as Long);
        }
        Err(_) => assert!(fires, "unexpected panic for pd {pd:#x}"),
    }
}
