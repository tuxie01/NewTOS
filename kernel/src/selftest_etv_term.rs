//! Cooperative-kernel step 2 — `etv_term` process-terminate hook, end-to-end
//! self-test (BIOS finding H1, etv_term part).
//!
//! Spec / source:
//!   * EmuTOS `xterm` (`references/emutos/bdos/proc.c:601-618`): at the very
//!     start of termination, `userterm = Setexc(0x102, -1)` then
//!     `protect_v(userterm)` — a bare `jsr (a0)` with NO arguments
//!     (`util/miscasm.S:213-220`), `etv_term` being `void (*)(void)`
//!     (`include/tosvars.h:112`) — runs the installed hook; termination then
//!     continues unconditionally (`ixterm`/`termuser`).
//!   * The upcall mechanism it rides on: `docs/analysis/coop-kernel/`
//!     `10-upcall-signal-design.md` (step 1 primitive + step 2 disposition).
//!
//! It builds on the step-1 upcall primitive with a `Terminate` disposition:
//! the etv_term handler runs at EL0, and on its `Psigreturn` the process is
//! terminated instead of resumed.
//!
//! This module is gated behind `pub mod` and is **not** wired into normal
//! boot (mirrors [`crate::selftest_upcall`] / [`crate::selftest_d6`]).
//!
//! ## What it exercises (two sub-tests)
//!
//! **A — handler installed.** A tiny EL0 program installs an etv_term handler
//! via the real `Setexc(0x102, handler)` BIOS path, then calls `Pterm(42)`.
//! The kernel's `handle_pterm` reads `vector(ETV_TERM)`, sees a real handler,
//! and delivers an EL0 upcall (`Terminate` disposition). The handler stores a
//! marker (proving it ran at EL0) and `ret`s into the 0x700 trampoline, which
//! calls `Psigreturn` (GEMDOS 0x11A); the disposition then terminates the
//! process. Asserts: (a) the handler ran, (b) the process actually terminated
//! (slot freed, `RUN` null, the post-`Pterm` code did NOT run).
//!
//! **B — no handler installed.** A tiny EL0 program calls `Pterm(42)` with the
//! etv_term slot at its boot value, the `ret` stub (`JUST_RTS`). `handle_pterm`
//! treats it as "no handler" and terminates directly (no upcall, no fault).
//! Asserts: (c) the process terminated directly.
//!
//! ## The EL0 programs
//!
//! ```text
//!   Program A (image @ VA 0x1000):
//!     0x1000  mov  x0, #0x102     ; ETV_TERM vector number
//!     0x1004  adr  x1, .+0x2C     ; handler VA (0x1030), PC-relative
//!     0x1008  mov  x8, #0x05      ; BIOS Setexc
//!     0x100C  svc  #13            ; Setexc(0x102, handler)
//!     0x1010  mov  x0, #0x2A      ; rc = 42
//!     0x1014  mov  x8, #0x4C      ; GEMDOS Pterm
//!     0x1018  svc  #1             ; etv_term upcall runs, then terminate
//!     ; --- failure fallthrough: only if wrongly resumed ---
//!     0x101C  mov  x9, #0x910     ; RESUMED_ADDR
//!     0x1020  mov  x10, #1
//!     0x1024  str  x10, [x9]      ; resumed marker = 1 (FAIL)
//!     0x1028  svc  #0xEC4         ; bail to EL1 to report FAIL
//!     0x102C  b    .
//!   handler (0x1030):
//!     0x1030  mov  x9, #0x900     ; FLAG_ADDR
//!     0x1034  mov  x10, #0x1234   ; marker
//!     0x1038  str  x10, [x9]      ; *(0x900) = 0x1234
//!     0x103C  ret                 ; -> LR = TRAMPOLINE_VA (0x700)
//!
//!   Program B (image @ VA 0x1000):
//!     0x1000  mov  x0, #0x2A      ; rc = 42
//!     0x1004  mov  x8, #0x4C      ; GEMDOS Pterm
//!     0x1008  svc  #1             ; boot etv_term -> terminate directly
//!     ; --- failure fallthrough: only if it did NOT terminate ---
//!     0x100C  mov  x9, #0x910     ; RESUMED_ADDR
//!     0x1010  mov  x10, #1
//!     0x1014  str  x10, [x9]
//!     0x1018  svc  #0xEC4         ; bail to EL1
//!     0x101C  b    .
//! ```
//!
//! The failure fallthrough's `svc #0xEC4` reuses the shared EC4 sentinel
//! return-to-EL1 path ([`crate::selftest_ec4`]) so a broken terminate-after
//! reports a clean FAIL instead of spinning. The success path returns to EL1
//! through the standard `Pterm` frame restore (the D6 pattern).

use core::sync::atomic::Ordering;

use tos_api::bdos::process::loader::load_to_slot;
use tos_api::bdos::process::{current, free_slot, set_current, slot, State, PROCESS_TABLE};
use tos_api::bios::vectors::{setexc, ETV_TERM, JUST_RTS};
use tos_api::trap::TrapArgs;

use crate::arch::aarch64::user_entry::{
    enter_el0_with_resume_capture, restore_boot_ttbr0,
};
use crate::kprintln;
use crate::selftest_ec4::{EC4_SENTINEL_SLOT, EC4_TEST_RUNNING};

// EL0 data cells (must match the ELF blobs below, whose entry is `e_entry`
// 0x1000, relocated to the TPA block). Both cells sit in page 0's reserved
// gap `[0x670, 0x1000)` (zeroed at load, mapped `UserRwExec`, disjoint from
// the 0x700 trampoline), read back through the kernel alias of page 0.
const FLAG_ADDR: u64 = 0x900;
const RESUMED_ADDR: u64 = 0x910;
/// Marker the etv_term handler stores at [`FLAG_ADDR`] — a match proves it
/// ran at EL0 (Program A's `mov x10, #0x1234`).
const FLAG_MARKER: u64 = 0x1234;

/// The two slots the sub-tests claim (free this early in boot).
const SLOT_A: usize = 0;
const SLOT_B: usize = 1;

// ─── Program A: installs etv_term, then Pterm (image @ VA 0x1000) ────────
//
// One PT_LOAD: p_vaddr = 0x1000, p_offset = 0x80, p_filesz = p_memsz = 0x40.
// Code bytes assembled + verified with `aarch64-elf-as` (see module docs).
#[rustfmt::skip]
static ETV_TERM_ELF_A: [u8; 0xC0] = [
    // ─── ELF64 header (64 bytes) ─────────────────────────────────
    0x7F, b'E', b'L', b'F', 2, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0x03, 0x00,              // e_type    = ET_DYN (PIE; ET_EXEC is refused since ARCH03 0d)
    0xB7, 0x00,              // e_machine = EM_AARCH64
    0x01, 0x00, 0x00, 0x00,  // e_version
    0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // e_entry = 0x1000
    0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // e_phoff = 0x40
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // e_shoff = 0
    0x00, 0x00, 0x00, 0x00,  // e_flags
    0x40, 0x00,              // e_ehsize    = 64
    0x38, 0x00,              // e_phentsize = 56
    0x01, 0x00,              // e_phnum     = 1
    0x40, 0x00,              // e_shentsize = 64 (unused)
    0x00, 0x00,              // e_shnum     = 0
    0x00, 0x00,              // e_shstrndx  = 0

    // ─── Program header (56 bytes at 0x40) ───────────────────────
    0x01, 0x00, 0x00, 0x00,  // p_type  = PT_LOAD
    0x05, 0x00, 0x00, 0x00,  // p_flags = PF_R | PF_X
    0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // p_offset = 0x80
    0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // p_vaddr  = 0x1000
    0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // p_paddr  = 0x1000
    0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // p_filesz = 0x40
    0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // p_memsz  = 0x40
    0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // p_align  = 0x1000

    // ─── padding 0x78..0x80 (8 bytes) ────────────────────────────
    0, 0, 0, 0, 0, 0, 0, 0,

    // ─── code at 0x80 (VA 0x1000) ────────────────────────────────
    0x40, 0x20, 0x80, 0xD2,  // mov x0, #0x102
    0x61, 0x01, 0x00, 0x10,  // adr x1, .+0x2C   (handler VA 0x1030, PC-relative)
    0xA8, 0x00, 0x80, 0xD2,  // mov x8, #0x5
    0xA1, 0x01, 0x00, 0xD4,  // svc #13
    0x40, 0x05, 0x80, 0xD2,  // mov x0, #0x2a
    0x88, 0x09, 0x80, 0xD2,  // mov x8, #0x4c
    0x21, 0x00, 0x00, 0xD4,  // svc #1
    0x09, 0x22, 0x81, 0xD2,  // mov x9, #0x910
    0x2A, 0x00, 0x80, 0xD2,  // mov x10, #1
    0x2A, 0x01, 0x00, 0xF9,  // str x10, [x9]
    0x81, 0xD8, 0x01, 0xD4,  // svc #0xec4
    0x00, 0x00, 0x00, 0x14,  // b .
    // handler @ VA 0x1030
    0x09, 0x20, 0x81, 0xD2,  // mov x9, #0x900
    0x8A, 0x46, 0x82, 0xD2,  // mov x10, #0x1234
    0x2A, 0x01, 0x00, 0xF9,  // str x10, [x9]
    0xC0, 0x03, 0x5F, 0xD6,  // ret
];

// ─── Program B: no handler, just Pterm (image @ VA 0x1000) ───────────────
//
// p_filesz = p_memsz = 0x20.
#[rustfmt::skip]
static ETV_TERM_ELF_B: [u8; 0xA0] = [
    // ─── ELF64 header (64 bytes) ─────────────────────────────────
    0x7F, b'E', b'L', b'F', 2, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0x03, 0x00, 0xB7, 0x00, 0x01, 0x00, 0x00, 0x00, // e_type ET_DYN (PIE), EM_AARCH64, EV_CURRENT
    0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // e_entry = 0x1000
    0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // e_phoff = 0x40
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // e_shoff = 0
    0x00, 0x00, 0x00, 0x00, 0x40, 0x00, 0x38, 0x00,
    0x01, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00,

    // ─── Program header (56 bytes at 0x40) ───────────────────────
    0x01, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00,
    0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // p_offset = 0x80
    0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // p_vaddr  = 0x1000
    0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // p_paddr  = 0x1000
    0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // p_filesz = 0x20
    0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // p_memsz  = 0x20
    0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // p_align  = 0x1000

    // ─── padding 0x78..0x80 (8 bytes) ────────────────────────────
    0, 0, 0, 0, 0, 0, 0, 0,

    // ─── code at 0x80 (VA 0x1000) ────────────────────────────────
    0x40, 0x05, 0x80, 0xD2,  // mov x0, #0x2a
    0x88, 0x09, 0x80, 0xD2,  // mov x8, #0x4c
    0x21, 0x00, 0x00, 0xD4,  // svc #1
    0x09, 0x22, 0x81, 0xD2,  // mov x9, #0x910
    0x2A, 0x00, 0x80, 0xD2,  // mov x10, #1
    0x2A, 0x01, 0x00, 0xF9,  // str x10, [x9]
    0x81, 0xD8, 0x01, 0xD4,  // svc #0xec4
    0x00, 0x00, 0x00, 0x14,  // b .
];

/// Result of one sub-test excursion (read back after the EL0 round-trip).
struct SubResult {
    /// The etv_term handler's marker at [`FLAG_ADDR`] (0 if it never ran).
    flag: u64,
    /// The "wrongly resumed" marker at [`RESUMED_ADDR`] (1 = the post-`Pterm`
    /// code ran, i.e. the process resumed instead of terminating).
    resumed: u64,
    /// `true` if the slot is `Free` again (the process terminated + released).
    slot_freed: bool,
    /// `true` if `RUN` is null again (the process is no longer current).
    run_cleared: bool,
}

/// Run one sub-test: load `elf` into `slot`, drop to EL0, read back the
/// markers + slot state. The etv_term vector must already be in the desired
/// state (installed by the program for A; left at the boot value for B).
fn run_subtest(elf: &[u8], slot_idx: usize) -> Option<SubResult> {
    // SAFETY: boot CPU, single-threaded; the slot is dedicated to this test.
    let loaded = match unsafe { load_to_slot(elf, slot_idx) } {
        Ok(img) => img,
        Err(e) => {
            kprintln!("[etvterm] load failed for slot {}: {:?}", slot_idx, e);
            return None;
        }
    };

    // Make this the running process so `handle_pterm` / the upcall delivery
    // (`slot_for(current())`) target it, and `terminate` finds it via `RUN`.
    let proc_slot = slot(slot_idx)?;
    proc_slot.set_state(State::Running);
    set_current(tos_api::bdos::process::pd_of(slot_idx));

    // Capture pointers to the two page-0 data cells via the kernel alias of
    // the slot's page 0 BEFORE the excursion. Page 0 is a static kernel page
    // per slot: the success path releases the slot but nothing rewrites the
    // page until the slot's next load — so the markers are still readable
    // afterwards.
    let page0 = tos_api::bdos::process::mmu::page0_kva(slot_idx)? as usize;
    let flag_ptr = (page0 + FLAG_ADDR as usize) as *const u64;
    let resumed_ptr = (page0 + RESUMED_ADDR as usize) as *const u64;

    // Arm the EC4 sentinel as the failure-fallback return-to-EL1 path (used
    // only if terminate-after is broken and the program resumes past `Pterm`).
    EC4_SENTINEL_SLOT.store(slot_idx, Ordering::Release);
    EC4_TEST_RUNNING.store(true, Ordering::Release);

    // Drop to EL0. On success the process terminates and control returns here
    // through the standard `Pterm` frame restore (the slot's `parent_frame`
    // is stamped by the helper).
    // SAFETY: the loaded slot's L0 is fully built; single-CPU; the
    // parent_frame atomics live in the static PROCESS_TABLE.
    unsafe {
        enter_el0_with_resume_capture(
            loaded.entry_va,
            loaded.basepage_va,
            loaded.l0_pa,
            loaded.sp_el0,
            &PROCESS_TABLE[slot_idx].parent_frame,
        );
    }
    // SAFETY: post-excursion; restore the no-process TTBR0 before more kernel
    // work touches low-half VAs.
    unsafe { restore_boot_ttbr0() };
    EC4_TEST_RUNNING.store(false, Ordering::Release);

    // SAFETY: single-CPU; the high-half alias is permanently mapped pool
    // memory and the block bytes are intact (see the capture comment above).
    let flag = unsafe { flag_ptr.read_volatile() };
    let resumed = unsafe { resumed_ptr.read_volatile() };
    let slot_freed = proc_slot.read_state() == State::Free;
    let run_cleared = current().is_null();

    // Failure-path cleanup: if the process wrongly resumed (EC4 fallback), the
    // slot is still allocated — release it so a later `Pexec` can reuse it.
    if !slot_freed {
        set_current(core::ptr::null_mut());
        let bp = tos_api::bdos::process::pd_of(slot_idx);
        if !bp.is_null() {
            // SAFETY: single-CPU; `bp` owns the slot's TPA block.
            unsafe {
                tos_api::bdos::mem::free_all_owned::free_all_owned(bp.cast());
            }
        }
        free_slot(slot_idx);
    }

    Some(SubResult {
        flag,
        resumed,
        slot_freed,
        run_cleared,
    })
}

/// Reset the etv_term vector back to its boot value, the `ret` stub, via the
/// real `Setexc(0x102, JUST_RTS)` path — undoes an install so a
/// later sub-test (or the desktop's app launches) sees "no handler".
fn reset_etv_term() {
    let mut regs = [0u64; 8];
    regs[0] = ETV_TERM as u64;
    regs[1] = JUST_RTS as u64;
    let args = TrapArgs {
        regs,
        func: 0x05, // BIOS Setexc
        svc_imm: 13,
        space: tos_api::uaccess::Space::Kernel,
    };
    setexc(&args);
}

/// Run the etv_term end-to-end self-test (cooperative-kernel step 2).
pub fn run() {
    kprintln!("");
    kprintln!("[etvterm] === etv_term process-terminate hook test (coop-kernel step 2) ===");

    let mut fails = 0;
    let mut check = |cond: bool, label: &str| {
        kprintln!("[etvterm]   {}: {}", label, if cond { "OK" } else { "FAIL" });
        if !cond {
            fails += 1;
        }
    };

    // Start from a clean slate: no etv_term handler installed.
    reset_etv_term();

    // ── Sub-test A: real handler installed via Setexc, then Pterm ──
    kprintln!("[etvterm] A: install etv_term via Setexc(0x102,handler), then Pterm(42) …");
    match run_subtest(&ETV_TERM_ELF_A, SLOT_A) {
        Some(r) => {
            check(
                r.flag == FLAG_MARKER,
                "(a) etv_term handler ran at EL0 (flag@0x900 == 0x1234)",
            );
            check(
                r.resumed == 0,
                "(b) process did NOT resume the Pterm caller (resumed@0x910 == 0)",
            );
            check(r.slot_freed, "(b) process terminated: slot freed");
            check(r.run_cleared, "(b) process terminated: RUN cleared");
        }
        None => {
            check(false, "(A) sub-test ran");
        }
    }

    // Undo A's install so B sees the boot value.
    reset_etv_term();

    // ── Sub-test B: no handler installed → terminate directly ──
    kprintln!("[etvterm] B: no handler (boot ret stub), Pterm(42) terminates directly …");
    match run_subtest(&ETV_TERM_ELF_B, SLOT_B) {
        Some(r) => {
            check(
                r.flag == 0,
                "(c) no handler fired (flag@0x900 == 0)",
            );
            check(
                r.resumed == 0,
                "(c) process did NOT resume the Pterm caller (resumed@0x910 == 0)",
            );
            check(r.slot_freed, "(c) terminated directly: slot freed (no fault)");
            check(r.run_cleared, "(c) terminated directly: RUN cleared");
        }
        None => {
            check(false, "(B) sub-test ran");
        }
    }

    // Leave etv_term at its boot value for the rest of boot.
    reset_etv_term();

    if fails == 0 {
        kprintln!("[etvterm] === etv_term PASS ===");
    } else {
        kprintln!("[etvterm] === etv_term FAILED ({}) ===", fails);
    }
}
