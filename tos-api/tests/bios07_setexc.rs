//! TASK/BIOS07 — `Setexc` (BIOS 0x05) answers what EmuTOS answers, per
//! vector class (`docs/api/bios/05-setexc.md`, decisions D1–D3 and P3; EmuTOS
//! `references/emutos/bios/bios.c:1070-1080`):
//!
//! * the CPU vectors and `etv_timer`/`etv_critic`/`etv_term` hold a handler
//!   address — the `ret` stub, EmuTOS' `just_rts` — never 0 or a 64-bit
//!   token (BIOS-CORE-004/005); vectors 0/1 the OSHEADER's first two LONGs;
//!   `etv_xtra` 0;
//! * `0x104…0x1FF` read and write the calling process's page-0 system
//!   variable, not a shadow (BIOS-CORE-006);
//! * every address a query answers lies below 2 GiB (the MEM11 invariant);
//! * page 0 carries every shared cell, in every loaded process (D3);
//! * the query test looks at the low 32 bits (P3), a write stores them, the
//!   answer is sign-extended (D1); `num >= 0x200` answers 0 (D2).

use std::sync::atomic::Ordering;
use std::sync::{Mutex, MutexGuard, Once};

use tos_api::bdos::process::{self, hooks, mmu, pexec, PROCESS_TABLE};
use tos_api::bios::reset::RESET_ENTRY_VA;
use tos_api::bios::vectors::{self, setexc, ETV_CRITIC, ETV_TERM, ETV_TIMER, ETV_XTRA, JUST_RTS};
use tos_api::trap::TrapArgs;
use tos_types::BasePage;

static LOCK: Mutex<()> = Mutex::new(());
static SETUP: Once = Once::new();

fn lock() -> MutexGuard<'static, ()> {
    let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    SETUP.call_once(|| {
        // SAFETY: once per binary, before any process; boot drive C: (2).
        let published = unsafe { process::standin::init(2) };
        assert!(published.is_some(), "the stand-in page is published");
    });
    while hooks::pop().is_some() {}
    g
}

/// A loaded, running process whose page 0 holds what the loader puts at the
/// vector cells (`write_page0_vectors`) and the page-0 hook cells
/// (`hooks::PAGE0_HOOKS`), and zeros elsewhere.
struct Proc {
    slot: usize,
    bp: *mut BasePage,
}

impl Proc {
    fn new() -> Proc {
        let slot = process::allocate_slot().expect("a slot");
        let bp = Box::into_raw(Box::new([0u8; 256])) as *mut BasePage;
        let p = &PROCESS_TABLE[slot];
        p.basepage_kva.store(bp, Ordering::Release);
        // SAFETY: under the lock; the slot is not an installed root.
        unsafe { mmu::init_slot_tables(slot) }.unwrap();
        // SAFETY: under the lock; the slot's page 0 belongs to this process.
        unsafe {
            let page0 = mmu::page0_bytes(slot).unwrap();
            page0.fill(0);
            vectors::write_page0_vectors(page0);
            // The page-0 hook cells hold the loader's values (TASK/BIOS17:
            // `swv_vec` is the reset entry, not 0).
            for h in hooks::PAGE0_HOOKS.iter() {
                let at = h.addr as usize;
                page0[at..at + 4].copy_from_slice(&h.default.to_le_bytes());
            }
        }
        p.set_state(process::State::Running);
        p.parent_frame.elr().store(1, Ordering::Release);
        // SAFETY: under the lock; the slot holds a basepage.
        unsafe { pexec::set_kernel_parent(slot) };
        let me = Proc { slot, bp };
        me.run();
        me
    }

    fn run(&self) {
        process::set_current(process::pd_of(self.slot));
    }

    /// The LONG at page-0 address `addr`, as the program reads it.
    fn peek(&self, addr: usize) -> u32 {
        // SAFETY: under the lock; the process's own page 0.
        let p0 = unsafe { mmu::page0_bytes(self.slot) }.unwrap();
        u32::from_le_bytes(p0[addr..addr + 4].try_into().unwrap())
    }

    /// Write the LONG at page-0 address `addr`, as the program does in super
    /// state.
    fn poke(&self, addr: usize, v: u32) {
        // SAFETY: under the lock; the process's own page 0.
        let p0 = unsafe { mmu::page0_bytes(self.slot) }.unwrap();
        p0[addr..addr + 4].copy_from_slice(&v.to_le_bytes());
    }
}

impl Drop for Proc {
    fn drop(&mut self) {
        process::set_current(core::ptr::null_mut());
        process::free_slot(self.slot);
        // SAFETY: allocated in `new` with `Box::into_raw`.
        drop(unsafe { Box::from_raw(self.bp as *mut [u8; 256]) });
    }
}

fn setexc_call(num: u64, vector: u64) -> i64 {
    setexc(&TrapArgs::kernel([num, vector, 0, 0, 0, 0, 0, 0], 0x05, 13))
}

fn query(num: u64) -> i64 {
    setexc_call(num, u64::MAX)
}

/// `docs/TOS-REFERENCE-DUMPER.md` `PTR_RAM`: non-null, `>= 0x400`, below
/// 2 GiB, even — the class of `bios-core-r1` cases 46–57.
fn ptr_ram(v: i64) -> bool {
    v != 0 && (0x400..0x8000_0000).contains(&v) && v & 1 == 0
}

// ─── Per vector class ────────────────────────────────────────────────────

#[test]
fn cpu_vectors_answer_the_ret_stub_not_null() {
    let _g = lock();
    let _a = Proc::new();
    // bios-core-r1 cases 46-54 and bios-r1 53/54 (BIOS-CORE-004).
    for num in [0x2u64, 0x3, 0x4, 0x1C, 0x21, 0x22, 0x2D, 0x2E, 0x45] {
        let v = query(num);
        assert_eq!(v, JUST_RTS, "vector 0x{:x}", num);
        assert!(ptr_ram(v), "vector 0x{:x} = 0x{:x}", num, v);
    }
    for num in 2..0x100u64 {
        assert_eq!(query(num), JUST_RTS, "vector 0x{:x}", num);
    }
    // Vectors 0/1: the OSHEADER's first two LONGs (bios.c:120-136):
    // os_entry 0 | os_version, then reseth = the reset entry.
    assert_eq!(query(1), RESET_ENTRY_VA as i64);
    assert_eq!(query(0) & 0xFFFF, 0, "os_entry");
    assert_ne!(query(0) >> 16, 0, "os_version");
}

#[test]
fn etv_hooks_answer_handler_addresses_not_64_bit_tokens() {
    let _g = lock();
    let _a = Proc::new();
    // bios-core-r1 cases 55-57 (BIOS-CORE-005).
    for num in [ETV_TIMER, ETV_CRITIC, ETV_TERM] {
        let v = query(num as u64);
        assert_eq!(v, JUST_RTS, "0x{:x}", num);
        assert!(ptr_ram(v), "0x{:x} = 0x{:x}", num, v);
        assert_eq!(vectors::vector(num), v);
    }
    // Case 58: etv_xtra is 0, as on EmuTOS.
    assert_eq!(query(ETV_XTRA as u64), 0);
}

#[test]
fn system_variables_are_the_process_s_page_0_cells() {
    let _g = lock();
    let a = Proc::new();
    // Case 59 / probe S08 (BIOS-CORE-006): memvalid at $420 = Setexc(0x108).
    a.poke(0x420, 0x7520_19F3);
    assert_eq!(query(0x108), 0x7520_19F3);
    assert_eq!(vectors::vector(0x108), 0, "no shared cell behind 0x108");
    // A write lands in the cell; the old value comes back.
    assert_eq!(setexc_call(0x108, 0x3333_4444), 0x7520_19F3);
    assert_eq!(a.peek(0x420), 0x3333_4444);
    assert_eq!(setexc_call(0x108, 0x7520_19F3), 0x3333_4444);
    // The first and the last system-variable number of the regime.
    a.poke(0x410, 0x0000_1234);
    assert_eq!(query(0x104), 0x1234);
    a.poke(0x7FC, 0x0000_5678);
    assert_eq!(query(0x1FF), 0x5678);
    // Another process has its own page 0: its memvalid is not A's.
    let b = Proc::new();
    assert_eq!(query(0x108), 0);
    drop(b);
    a.run();
    assert_eq!(query(0x108), 0x7520_19F3);
}

#[test]
fn beyond_the_low_memory_region_answers_0_and_writes_nothing() {
    let _g = lock();
    let a = Proc::new();
    // D2: num >= 0x200 and a negative WORD (a huge index) answer 0.
    for num in [0x200u64, 0x201, 0x7FFF, 0xFFFF, 0x8000] {
        assert_eq!(setexc_call(num, 0x0001_0000), 0, "0x{:x}", num);
        assert_eq!(query(num), 0, "0x{:x}", num);
    }
    // The negative WORDs did not wrap into a cell.
    assert_eq!(query(1), RESET_ENTRY_VA as i64);
    for num in 2..0x103u64 {
        assert_eq!(query(num), JUST_RTS, "0x{:x}", num);
    }
    assert_eq!(query(ETV_XTRA as u64), 0);
    assert_eq!(a.peek(0x7FC), 0);
}

#[test]
fn every_address_a_query_answers_is_below_2_gib() {
    let _g = lock();
    let a = Proc::new();
    // The MEM11 invariant for the shared cells (all of them hold an address
    // or 0 by default) and for the page-0 cells the loader would fill.
    for num in 0..0x104u64 {
        let v = query(num);
        assert!((0..0x8000_0000).contains(&v), "0x{:x} = 0x{:x}", num, v);
        assert_eq!(v as u64 >> 32, 0, "0x{:x}: no high half", num);
    }
    a.poke(0x5A0, 0x0000_05F0); // _p_cookies, as the loader writes it
    assert_eq!(query(0x168), 0x5F0);
}

// ─── Page 0 carries the shared cells (D3) ────────────────────────────────

#[test]
fn page_0_carries_every_shared_cell_in_every_process() {
    let _g = lock();
    let a = Proc::new();
    let b = Proc::new();
    for num in 0..0x104usize {
        assert_eq!(i64::from(a.peek(4 * num) as i32), query(num as u64), "A 0x{:x}", num);
        assert_eq!(i64::from(b.peek(4 * num) as i32), query(num as u64), "B 0x{:x}", num);
    }
    // A write by B reaches A's copy too: one cell, as on EmuTOS.
    let old = setexc_call(0x45, 0x0001_2340);
    assert_eq!(old, JUST_RTS);
    assert_eq!(a.peek(0x114), 0x0001_2340);
    assert_eq!(b.peek(0x114), 0x0001_2340);
    // So does etv_term, the hook NewTOS calls.
    setexc_call(ETV_TERM as u64, 0x0001_2380);
    assert_eq!(a.peek(0x408), 0x0001_2380);
    assert_eq!(vectors::vector(ETV_TERM), 0x0001_2380);
    // A process loaded afterwards starts with the current cells.
    let c = Proc::new();
    assert_eq!(c.peek(0x114), 0x0001_2340);
    assert_eq!(c.peek(0x408), 0x0001_2380);
    // Put back (the owner record would do it at Pterm).
    setexc_call(0x45, JUST_RTS as u64);
    setexc_call(ETV_TERM as u64, JUST_RTS as u64);
    assert_eq!(a.peek(0x114), JUST_RTS as u32);
    assert_eq!(c.peek(0x408), JUST_RTS as u32);
    drop(c);
    drop(b);
    drop(a);
    while hooks::pop().is_some() {}
}

#[test]
fn a_direct_write_of_the_etv_term_copy_does_not_change_the_shared_cell() {
    let _g = lock();
    let a = Proc::new();
    // ADR-031: NewTOS calls etv_term, and only what Setexc installed. (The
    // copies of $008-$407 written directly become the cell since TASK/BIOS16,
    // tests/bios16_page0_write.rs.)
    a.poke(0x408, 0x0001_3000);
    assert_eq!(query(ETV_TERM as u64), JUST_RTS);
    assert_eq!(vectors::vector(ETV_TERM), JUST_RTS);
    // The page-0 check sees it (etv_term written straight into page 0).
    hooks::check_page0(a.slot);
    let w = hooks::pop().expect("a warning");
    assert_eq!(w.kind, hooks::Kind::NotServed(hooks::Hook::EtvTermDirect, hooks::Via::Page0));
    assert_eq!(w.value, 0x0001_3000);
    assert!(hooks::pop().is_none(), "the other copies equal their cells");
}

// ─── Width (D1, P3) ──────────────────────────────────────────────────────

#[test]
fn the_query_test_looks_at_the_low_32_bits() {
    let _g = lock();
    let a = Proc::new();
    // tosdump/probes/sxprobe.c Q01-Q06 on vector 0x20.
    let old = setexc_call(0x20, 0x8000_0001);
    assert_eq!(old, JUST_RTS);
    // The LONG 0x80000001 comes back sign-extended (D1).
    assert_eq!(query(0x20), 0xFFFF_FFFF_8000_0001u64 as i64);
    // 0x00000000_FFFFFFFF is -1 as a LONG: a query, not a write (P3).
    assert_eq!(setexc_call(0x20, 0x0000_0000_FFFF_FFFF), 0xFFFF_FFFF_8000_0001u64 as i64);
    assert_eq!(query(0x20), 0xFFFF_FFFF_8000_0001u64 as i64);
    assert_eq!(a.peek(0x80), 0x8000_0001);
    // A write stores the low 32 bits.
    setexc_call(0x20, 0x0000_0001_0000_2000);
    assert_eq!(query(0x20), 0x2000);
    assert_eq!(setexc_call(0x20, old as u64), 0x2000);
    assert_eq!(query(0x20), JUST_RTS);
    // The system-variable regime tests the same way.
    a.poke(0x420, 0x1234_5678);
    assert_eq!(setexc_call(0x108, 0x0000_0000_FFFF_FFFF), 0x1234_5678);
    assert_eq!(a.peek(0x420), 0x1234_5678);
    while hooks::pop().is_some() {}
}
