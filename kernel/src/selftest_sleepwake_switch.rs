//! Cooperative-kernel step 3b — S2 restartable-block switch + deliver-on-resume
//! self-test (two EL0 processes, through the **real AES path**).
//!
//! Spec: `docs/analysis/coop-kernel/20-sleepwake-substrate-design.md` §4.3 (S2
//! restartable block), §6 (deliver-on-resume), §8.2 sub-steps 3+4, plus the
//! Lead's Option-1 decision (the `Switch::restartable` guard). Companion to the
//! substrate-level [`crate::selftest_sleepwake`] (which drives the WAITING/READY
//! bits directly); this one exercises the whole switch mechanism end-to-end.
//!
//! Gated behind `pub mod`, **not** wired into boot (mirrors
//! [`crate::selftest_upcall`]). Invoke manually from `kernel_main` after the
//! host is installed and before the desktop/AUTO `Pexec` claim slots 0–2.
//!
//! ## What it proves
//!
//!   * **S2 switch (Ausgang b, §4.4).** Process **A** issues a real
//!     `evnt_mesag` (GEM trap `svc #2`, selector 200 in `x0`), finds its message
//!     FIFO empty, and blocks. Because a runnable peer **B** exists, [`sched::
//!     sleep`] returns `Yield`, the handler saves A's frame with the ELR rewound
//!     to the `svc` (`restartable = true`) and switches to B — so B runs while A
//!     is WAITING.
//!   * **`appl_write` wakes the message-waiter (cleanup 1).** B's real
//!     `appl_write` enqueues A's message through `pd::post_words`, which now
//!     WAKES A (WAITING → READY) — there is **no** manual `sched::wake` in this
//!     test anymore, so A's readiness after phase 1 is direct proof that
//!     `appl_write` did the waking. B does **not** `Pwait` (that would yield to
//!     the now-runnable A and resume it in phase 1), so A stays merely woken and
//!     unresumed (its `evnt_mesag` RET_CODE is still the `0x7FFF` sentinel).
//!   * **Restartable resume (the second S2 delta).** In phase 2 the resumer **R**
//!     yields to the runnable A via `Pwait`; A's saved frame is restored with
//!     `x0 = 200` **preserved** (the `restartable` guard skips `apply_switch`'s
//!     `gpr[0] = 0`), so the re-executed `svc #2` re-enters `evnt_mesag`, finds
//!     the message (B's, at the FIFO front), and returns `RET_CODE = 1` with the
//!     message copied out. Had `x0` been clobbered to 0, the re-entered GEM trap
//!     would dispatch nothing and `RET_CODE` would stay at its `0x7FFF` sentinel.
//!   * **Deliver-on-resume (§6).** An upcall is pre-posted for A while it is
//!     woken-but-unresumed; on A's resume `apply_switch` delivers it, A runs the
//!     handler at EL0 (storing a marker), and then — via `Psigreturn` — the
//!     restartable `svc` re-executes and A completes with the correct AES result.
//!
//! ## Orchestration
//!
//! Two peer images. **B** (`B_ELF`, slot 1) runs first — `appl_write` enqueues
//! A's message (and WAKES A) and stores a "ran" marker, then bounces to EL1 via
//! the `svc #0xEC4` sentinel **without** `Pwait`, leaving A READY but unresumed.
//! The kernel pre-posts the upcall and drops B from the run-set. **R**
//! (`PEER_ELF`, slot 2) runs in phase 2 — its `Pwait` yields to the runnable A,
//! driving the restartable resume + deliver-on-resume, after which A bounces
//! back to EL1.

use core::sync::atomic::Ordering;

use tos_api::bdos::process::loader::load_to_slot;
use tos_api::bdos::process::{
    arena, free_slot, post_upcall, sched, set_current, Disposition, State,
    UpcallDesc, PROCESS_TABLE,
};

use crate::arch::aarch64::user_entry::{
    enter_el0_with_resume_capture, restore_boot_ttbr0,
};
use crate::kprintln;
use crate::selftest_ec4::{
    EC4_CAPTURED_ELR, EC4_SENTINEL_SLOT, EC4_TEST_RUNNING,
};

// ── Slots + pids ────────────────────────────────────────────────────────────
const A_SLOT: usize = 0;
const B_SLOT: usize = 1;
const R_SLOT: usize = 2;
/// A's AES pid = its slot index + 1 (`appl::current_pid`).
const A_PID: i16 = A_SLOT as i16 + 1;

// ── EL0 VAs shared by both images (both TPAs map at VA 0) ────────────────────
/// A's upcall handler (`str x1,[x0]; ret`) — matches `A_ELF`.
const A_HANDLER_VA: u64 = 0x1018;
/// The AESPB pointer both images load into `x1`.
const AESPB_VA: u64 = 0x900;
/// The six AESPB array VAs (LP64 layout).
const CONTROL_VA: u64 = 0x930;
const GLOBAL_VA: u64 = 0x990;
const INTIN_VA: u64 = 0x940;
const INTOUT_VA: u64 = 0x960;
const ADDRIN_VA: u64 = 0x970;
const ADDROUT_VA: u64 = 0x9B0;
/// A's `evnt_mesag` output buffer / PEER's `appl_write` source buffer.
const MSG_BUF_VA: u64 = 0x980;
/// A's deliver-on-resume handler marker cell (handler stores `args[1]` here).
const A_HANDLER_MARK_VA: u64 = 0x9C0;
/// PEER's "ran" marker cell (the asm stores [`PEER_MARK`] here).
const PEER_MARK_VA: u64 = 0x800;

// ── Expected values ─────────────────────────────────────────────────────────
/// `RET_CODE` pre-set so a non-firing (clobbered-`x0`) re-execution is visible.
const INTOUT_SENTINEL: i16 = 0x7FFF;
/// The message word B posts to A and A copies out.
const MSG_WORD0: i16 = 0x1234;
/// PEER's "I ran" marker (matches the `movz x10,#0xB0B` in `PEER_ELF`).
const PEER_MARK: u64 = 0xB0B;
/// A's deliver-on-resume handler marker (`args[1]`).
const HANDLER_MARK: u64 = 0xDEAD_BEEF;
/// The upcall type bit (arbitrary; cleared on `Psigreturn`).
const MASK_BIT: u32 = 0x1;
/// ELR captured when A finally does `svc #0xEC4` (= the `b .` after it in `A_ELF`).
const A_SENTINEL_RESUME_ELR: u64 = 0x1010;

// ── Program A: real `evnt_mesag` (svc #2), then `svc #0xEC4`; upcall handler ──
//
//   entry (0x1000):
//     movz x0, #200        ; AES selector
//     movz x1, #0x900      ; AESPB (evnt_mesag)
//     svc  #2              ; blocks; on resume the rewound svc re-executes here
//     svc  #0xEC4          ; bounce to EL1
//     b    .
//   handler (0x1018):
//     str  x1, [x0]        ; *(args[0]) = args[1]  (deliver-on-resume marker)
//     ret                  ; -> LR = TRAMPOLINE_VA (0x700)
#[rustfmt::skip]
static A_ELF: [u8; 0xA0] = [
    // ELF64 header (64 bytes)
    0x7F, b'E', b'L', b'F', 2, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0x03, 0x00, 0xB7, 0x00, 0x01, 0x00, 0x00, 0x00, // e_type ET_DYN (PIE), EM_AARCH64, EV_CURRENT
    0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // e_entry = 0x1000
    0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // e_phoff = 0x40
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // e_shoff = 0
    0x00, 0x00, 0x00, 0x00, 0x40, 0x00, 0x38, 0x00,
    0x01, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00,
    // Program header (56 bytes at 0x40)
    0x01, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, // PT_LOAD, R|X
    0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // p_offset = 0x80
    0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // p_vaddr  = 0x1000
    0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // p_paddr  = 0x1000
    0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // p_filesz = 0x20
    0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // p_memsz  = 0x20
    0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // p_align  = 0x1000
    // padding 0x78..0x80
    0, 0, 0, 0, 0, 0, 0, 0,
    // code at 0x80 (VA 0x1000), 0x20 bytes
    0x00, 0x19, 0x80, 0xD2,  // movz x0, #200
    0x01, 0x20, 0x81, 0xD2,  // movz x1, #0x900
    0x41, 0x00, 0x00, 0xD4,  // svc  #2
    0x81, 0xD8, 0x01, 0xD4,  // svc  #0xEC4
    0x00, 0x00, 0x00, 0x14,  // b .
    0x00, 0x00, 0x00, 0x00,  // pad (VA 0x1014)
    0x01, 0x00, 0x00, 0xF9,  // str x1, [x0]   (handler @ 0x1018)
    0xC0, 0x03, 0x5F, 0xD6,  // ret
];

// ── Program B: real `appl_write` (enqueue) + marker + `svc #0xEC4` (NO Pwait) ──
//
// The phase-1 poster. Since the cleanup routed `appl_write` through
// `pd::post_words`, its enqueue now WAKES A (WAITING → READY) — so B must NOT
// `Pwait` here, or it would yield to the now-runnable A and resume it in phase
// 1, collapsing the two-phase structure. Instead B bounces straight to EL1,
// leaving A merely woken (READY) but unresumed for phase 2 to drive.
//
//   entry (0x1000):
//     movz x0, #200        ; AES selector
//     movz x1, #0x900      ; AESPB (appl_write)
//     svc  #2              ; appl_write -> enqueue + WAKE A (cleanup 1)
//     movz x9,  #0x800     ; marker addr
//     movz x10, #0xB0B     ; marker value
//     str  x10, [x9]       ; prove this image ran
//     svc  #0xEC4          ; bounce to EL1 (A left READY, unresumed)
//     b    .
#[rustfmt::skip]
static B_ELF: [u8; 0xA0] = [
    // ELF64 header (64 bytes)
    0x7F, b'E', b'L', b'F', 2, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0x03, 0x00, 0xB7, 0x00, 0x01, 0x00, 0x00, 0x00, // e_type ET_DYN (PIE), EM_AARCH64, EV_CURRENT
    0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // e_entry = 0x1000
    0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // e_phoff = 0x40
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // e_shoff = 0
    0x00, 0x00, 0x00, 0x00, 0x40, 0x00, 0x38, 0x00,
    0x01, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00,
    // Program header (56 bytes at 0x40)
    0x01, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, // PT_LOAD, R|X
    0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // p_offset = 0x80
    0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // p_vaddr  = 0x1000
    0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // p_paddr  = 0x1000
    0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // p_filesz = 0x20
    0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // p_memsz  = 0x20
    0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // p_align  = 0x1000
    // padding 0x78..0x80
    0, 0, 0, 0, 0, 0, 0, 0,
    // code at 0x80 (VA 0x1000), 0x20 bytes
    0x00, 0x19, 0x80, 0xD2,  // movz x0, #200
    0x01, 0x20, 0x81, 0xD2,  // movz x1, #0x900
    0x41, 0x00, 0x00, 0xD4,  // svc  #2            (appl_write -> enqueue + wake A)
    0x09, 0x00, 0x81, 0xD2,  // movz x9, #0x800
    0x6A, 0x61, 0x81, 0xD2,  // movz x10, #0xB0B
    0x2A, 0x01, 0x00, 0xF9,  // str  x10, [x9]
    0x81, 0xD8, 0x01, 0xD4,  // svc  #0xEC4        (bounce; NO Pwait)
    0x00, 0x00, 0x00, 0x14,  // b .
];

// ── Program PEER (R): `appl_write` + marker + `Pwait` + `svc #0xEC4` ───────────
//
// The phase-2 resumer. Its `Pwait` yields to the (now-runnable) A, driving the
// S2 restartable resume + deliver-on-resume. Its own `appl_write` enqueues a
// second message; A pops the FRONT (B's phase-1 message) on resume.
//
//   entry (0x1000):
//     movz x0, #200        ; AES selector
//     movz x1, #0x900      ; AESPB (appl_write)
//     svc  #2              ; appl_write -> enqueue a message into A's FIFO
//     movz x9,  #0x800     ; marker addr
//     movz x10, #0xB0B     ; marker value
//     str  x10, [x9]       ; prove this image ran
//     mov  x8,  #0x4D      ; GEMDOS Pwait
//     svc  #1              ; as R: yield to A
//     svc  #0xEC4          ; bounce to EL1 (reached only if Pwait was a no-op)
//     b    .
#[rustfmt::skip]
static PEER_ELF: [u8; 0xA8] = [
    // ELF64 header (64 bytes)
    0x7F, b'E', b'L', b'F', 2, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0x03, 0x00, 0xB7, 0x00, 0x01, 0x00, 0x00, 0x00, // e_type ET_DYN (PIE), EM_AARCH64, EV_CURRENT
    0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // e_entry = 0x1000
    0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // e_phoff = 0x40
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // e_shoff = 0
    0x00, 0x00, 0x00, 0x00, 0x40, 0x00, 0x38, 0x00,
    0x01, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00,
    // Program header (56 bytes at 0x40)
    0x01, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, // PT_LOAD, R|X
    0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // p_offset = 0x80
    0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // p_vaddr  = 0x1000
    0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // p_paddr  = 0x1000
    0x28, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // p_filesz = 0x28
    0x28, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // p_memsz  = 0x28
    0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // p_align  = 0x1000
    // padding 0x78..0x80
    0, 0, 0, 0, 0, 0, 0, 0,
    // code at 0x80 (VA 0x1000), 0x28 bytes
    0x00, 0x19, 0x80, 0xD2,  // movz x0, #200
    0x01, 0x20, 0x81, 0xD2,  // movz x1, #0x900
    0x41, 0x00, 0x00, 0xD4,  // svc  #2            (appl_write)
    0x09, 0x00, 0x81, 0xD2,  // movz x9, #0x800
    0x6A, 0x61, 0x81, 0xD2,  // movz x10, #0xB0B
    0x2A, 0x01, 0x00, 0xF9,  // str  x10, [x9]
    0xA8, 0x09, 0x80, 0xD2,  // mov  x8, #0x4D
    0x21, 0x00, 0x00, 0xD4,  // svc  #1            (Pwait)
    0x81, 0xD8, 0x01, 0xD4,  // svc  #0xEC4        (bounce, reached only as B)
    0x00, 0x00, 0x00, 0x14,  // b .
];

// ── Little-endian page accessors (page 0 = VA [0, 0x1000)) ───────────────────
fn wr_u64(page: &mut [u8; 4096], off: u64, v: u64) {
    page[off as usize..off as usize + 8].copy_from_slice(&v.to_le_bytes());
}
fn wr_i16(page: &mut [u8; 4096], off: u64, v: i16) {
    page[off as usize..off as usize + 2].copy_from_slice(&v.to_le_bytes());
}
fn rd_u64(page: &[u8; 4096], off: u64) -> u64 {
    let mut b = [0u8; 8];
    b.copy_from_slice(&page[off as usize..off as usize + 8]);
    u64::from_le_bytes(b)
}
fn rd_i16(page: &[u8; 4096], off: u64) -> i16 {
    let mut b = [0u8; 2];
    b.copy_from_slice(&page[off as usize..off as usize + 2]);
    i16::from_le_bytes(b)
}

/// Write the six AESPB array pointers (identical layout in both images).
fn write_aespb_ptrs(page: &mut [u8; 4096]) {
    wr_u64(page, AESPB_VA, CONTROL_VA);
    wr_u64(page, AESPB_VA + 8, GLOBAL_VA);
    wr_u64(page, AESPB_VA + 16, INTIN_VA);
    wr_u64(page, AESPB_VA + 24, INTOUT_VA);
    wr_u64(page, AESPB_VA + 32, ADDRIN_VA);
    wr_u64(page, AESPB_VA + 40, ADDROUT_VA);
}

/// Run the step-3b switch + deliver-on-resume self-test.
pub fn run() {
    kprintln!("");
    kprintln!("[sw3b] === step-3b S2 switch + deliver-on-resume test ===");

    // 1. Load A (slot 0) and the peer image into slots 1 (B) and 2 (R).
    // SAFETY: boot CPU, single-threaded; slots 0–2 are dedicated to this test.
    let a = match unsafe { load_to_slot(&A_ELF, A_SLOT) } {
        Ok(i) => i,
        Err(e) => {
            kprintln!("[sw3b] === FAIL: load A: {:?} ===", e);
            return;
        }
    };
    let b = match unsafe { load_to_slot(&B_ELF, B_SLOT) } {
        Ok(i) => i,
        Err(e) => {
            kprintln!("[sw3b] === FAIL: load B: {:?} ===", e);
            cleanup();
            return;
        }
    };
    let r = match unsafe { load_to_slot(&PEER_ELF, R_SLOT) } {
        Ok(i) => i,
        Err(e) => {
            kprintln!("[sw3b] === FAIL: load R: {:?} ===", e);
            cleanup();
            return;
        }
    };
    kprintln!(
        "[sw3b] loaded A(slot{} l0=0x{:x}) B(slot{}) R(slot{})",
        a.slot, a.l0_pa, b.slot, r.slot,
    );

    // 2. Pre-fill the AESPBs in each TPA (kernel-side, via the high-half alias).
    // SAFETY: single-CPU pre-test; the slots are exclusively ours.
    {
        let Some(pa) = (unsafe { arena::page_bytes_mut(A_SLOT) }) else {
            kprintln!("[sw3b] === FAIL: no TPA page for A ===");
            cleanup();
            return;
        };
        write_aespb_ptrs(pa);
        wr_i16(pa, CONTROL_VA, 23); // evnt_mesag opcode
        // control[2] = OUT_LEN: the dispatcher copies exactly this many words
        // back (`aes/gemsuper.c:443-444`), so the one word A wants — RET_CODE —
        // must be declared. gemlib's binding says the same:
        // `AES_PARAMS(23,0,1,1,0)` (`a_evnt_mesag.c`).
        wr_i16(pa, CONTROL_VA + 4, 1);
        wr_u64(pa, ADDRIN_VA, MSG_BUF_VA); // addr_in[0] = output buffer
        wr_i16(pa, INTOUT_VA, INTOUT_SENTINEL); // RET_CODE sentinel
    }
    for slot in [B_SLOT, R_SLOT] {
        // SAFETY: as above.
        let Some(pp) = (unsafe { arena::page_bytes_mut(slot) }) else {
            kprintln!("[sw3b] === FAIL: no TPA page for peer slot {} ===", slot);
            cleanup();
            return;
        };
        write_aespb_ptrs(pp);
        wr_i16(pp, CONTROL_VA, 12); // appl_write opcode
        wr_i16(pp, CONTROL_VA + 4, 1); // OUT_LEN 1 (gemlib `AES_PARAMS(12,2,1,1,0)`)
        wr_i16(pp, INTIN_VA, A_PID); // int_in[0] = target pid (A)
        wr_i16(pp, INTIN_VA + 2, 16); // int_in[1] = byte length (8 words)
        wr_u64(pp, ADDRIN_VA, MSG_BUF_VA); // addr_in[0] = message source
        wr_i16(pp, MSG_BUF_VA, MSG_WORD0); // the message word
    }

    // 3. Register A's AES process descriptor so its pid-1 message FIFO exists
    //    (both `evnt_mesag`'s `by_pid` and B's `appl_write` target it).
    tos_api::aes::register_process(A_SLOT, b"A.PRG");

    // 4. Make A the running peer with B admitted alongside it. R stays out of
    //    the run-set until phase 2 so B's Pwait finds no peer (a no-op).
    PROCESS_TABLE[A_SLOT].set_state(State::Running);
    sched::admit(A_SLOT);
    sched::admit(B_SLOT);
    set_current(tos_api::bdos::process::pd_of(A_SLOT));
    sched::set_running(A_SLOT);

    // 5. Phase 1: enter A; it blocks in evnt_mesag, yields to B; B enqueues A's
    //    message, marks that it ran, Pwaits (no-op), and bounces to EL1.
    EC4_SENTINEL_SLOT.store(A_SLOT, Ordering::Release);
    EC4_TEST_RUNNING.store(true, Ordering::Release);
    kprintln!("[sw3b] phase 1: enter A (evnt_mesag -> yield to B) …");
    // SAFETY: A's L0 is fully built; single-CPU; parent_frame outlives the trip.
    unsafe {
        enter_el0_with_resume_capture(
            a.entry_va,
            a.basepage_va,
            a.l0_pa,
            a.sp_el0,
            &PROCESS_TABLE[A_SLOT].parent_frame,
        );
    }
    // SAFETY: post-excursion — restore the no-process TTBR0.
    unsafe { restore_boot_ttbr0() };

    // Verify the switch happened AND cleanup 1: B ran (its marker) while A was
    // blocked, and B's `appl_write` WOKE A (WAITING → READY) via `pd::post_words`
    // — with the manual `sched::wake` removed, A's readiness is now proof that
    // `appl_write` did the waking. B does not `Pwait`, so A is woken but NOT yet
    // resumed: its `evnt_mesag` has not completed (RET_CODE still the sentinel).
    let b_ran = {
        // SAFETY: single-CPU; slot is ours.
        match unsafe { arena::page_bytes_mut(B_SLOT) } {
            Some(p) => rd_u64(p, PEER_MARK_VA) == PEER_MARK,
            None => false,
        }
    };
    let a_woken = PROCESS_TABLE[A_SLOT].runnable.load(Ordering::Acquire)
        && PROCESS_TABLE[A_SLOT].wait_cond.load(Ordering::Acquire) == 0;
    let a_phase1_ret = {
        // SAFETY: single-CPU; slot is ours.
        match unsafe { arena::page_bytes_mut(A_SLOT) } {
            Some(p) => rd_i16(p, INTOUT_VA),
            None => 0,
        }
    };
    let a_not_resumed = a_phase1_ret == INTOUT_SENTINEL;

    // 6. Pre-post the deliver-on-resume upcall for A (its message is already
    //    enqueued and A is already woken by `appl_write` — no manual `sched::wake`
    //    needed anymore). Drop B from the run-set so only A and R remain runnable.
    post_upcall(
        A_SLOT,
        UpcallDesc {
            // The image is relocated for its TPA block: link VA →
            // block address + link VA (ARCH03 stage 1).
            handler_va: tos_api::bdos::process::pd_of(A_SLOT) as u64 + A_HANDLER_VA,
            args: [A_HANDLER_MARK_VA, HANDLER_MARK, 0, 0],
            mask_bit: MASK_BIT,
            disposition: Disposition::Resume,
        },
    );
    sched::unready(B_SLOT);

    // 7. Phase 2: enter R; its Pwait yields to the now-runnable A, driving the
    //    restartable resume (x0 = 200 preserved) + deliver-on-resume. A then
    //    completes evnt_mesag and bounces to EL1.
    sched::admit(R_SLOT);
    PROCESS_TABLE[R_SLOT].set_state(State::Running);
    set_current(tos_api::bdos::process::pd_of(R_SLOT));
    sched::set_running(R_SLOT);
    EC4_SENTINEL_SLOT.store(R_SLOT, Ordering::Release);
    EC4_TEST_RUNNING.store(true, Ordering::Release);
    kprintln!("[sw3b] phase 2: enter R (Pwait -> resume A, restart svc#2) …");
    // SAFETY: as phase 1.
    unsafe {
        enter_el0_with_resume_capture(
            r.entry_va,
            r.basepage_va,
            r.l0_pa,
            r.sp_el0,
            &PROCESS_TABLE[R_SLOT].parent_frame,
        );
    }
    // SAFETY: post-excursion.
    unsafe { restore_boot_ttbr0() };

    // 8. Read back A's results (the re-executed evnt_mesag + the handler).
    let (a_ret, a_msg, a_mark) = {
        // SAFETY: single-CPU; slot is ours.
        match unsafe { arena::page_bytes_mut(A_SLOT) } {
            Some(p) => (
                rd_i16(p, INTOUT_VA),
                rd_i16(p, MSG_BUF_VA),
                rd_u64(p, A_HANDLER_MARK_VA),
            ),
            None => (0, 0, 0),
        }
    };
    let captured_elr = EC4_CAPTURED_ELR.load(Ordering::Acquire);
    let a_depth = PROCESS_TABLE[A_SLOT].upcall_depth.load(Ordering::Acquire);

    // 9. Verdict.
    let mut fails = 0;
    let mut check = |cond: bool, label: &str| {
        kprintln!("[sw3b]   {}: {}", label, if cond { "OK" } else { "FAIL" });
        if !cond {
            fails += 1;
        }
    };
    check(b_ran, "(1) peer B ran while A slept (switch happened)");
    check(
        a_woken && a_not_resumed,
        "(1) appl_write woke A (READY) without resuming it (RET still sentinel)",
    );
    check(a_mark == HANDLER_MARK, "(3) deliver-on-resume handler ran at EL0");
    check(
        a_ret == 1,
        "(2) restartable svc#2 re-executed with x0=200 (evnt_mesag RET_CODE=1)",
    );
    check(a_msg == MSG_WORD0, "(2) A received the message B posted");
    // A's image runs relocated at its TPA block (ARCH03 stage 1).
    check(
        captured_elr == a.basepage_va + A_SENTINEL_RESUME_ELR,
        "(2) A resumed past its svc (sentinel ELR == block + 0x1010)",
    );
    check(a_depth == 0, "    upcall_depth back to 0 (push/pop balanced)");

    cleanup();

    if fails == 0 {
        kprintln!("[sw3b] === PASS ===");
    } else {
        kprintln!("[sw3b] === FAIL: {} check(s) ===", fails);
    }
}

/// Free the three slots' TPAs, mark them free, and drop the running pointer +
/// sentinel arming.
fn cleanup() {
    EC4_TEST_RUNNING.store(false, Ordering::Release);
    set_current(core::ptr::null_mut());
    // Reset the scheduler's on-CPU slot to NONE so a later caller (the desktop's
    // `evnt_multi`) does not read a stale, now-freed slot index as `current()`.
    sched::set_running(tos_api::bdos::process::MAX_PROCESSES);
    for slot in [A_SLOT, B_SLOT, R_SLOT] {
        sched::unready(slot);
        let bp = tos_api::bdos::process::pd_of(slot);
        if !bp.is_null() {
            // SAFETY: single-CPU pre-test; `bp` owns the slot's TPA block.
            unsafe {
                tos_api::bdos::mem::free_all_owned::free_all_owned(bp.cast());
            }
        }
        free_slot(slot);
    }
}
