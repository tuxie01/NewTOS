//! MEM12 — page 0 and the super state.
//!
//! Spec: `docs/api/_super-state.md` (approved 2026-10-27), S1–S13; TASK/MEM12.
//! What runs on the host: the page mapping of a fresh root (page 0 EL0
//! read-only and never executable, the stub page EL0 execute-only, the private
//! page read-write), the `Super` state machine with its page-0 toggle, the
//! refusal of a VA = PA publication over the private pages, the validated
//! `conterm` read, the crash report's name for page 0, the new upcall
//! dispositions. The fault itself (an EL0 write to page 0 ending the program)
//! and `Supexec`'s upcall are kernel code and are measured in QEMU
//! (`measurements/measurement-2026-10-2x-mem12-acceptance/`).

use std::sync::atomic::Ordering;
use std::sync::{Mutex, MutexGuard};

use tos_api::bdos::process::mmu::{self, MapError, PageFlags, PAGE_SIZE, PRIV_PAGE_VA, STUB_PAGE_VA};
use tos_api::bdos::process::{self, superstate, Disposition, State, PROCESS_TABLE};
use tos_types::BasePage;

static LOCK: Mutex<()> = Mutex::new(());

fn lock() -> MutexGuard<'static, ()> {
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// The attribute bits of a leaf: the lower attributes [11:0] and PXN/UXN
/// [54:53]. (On the host a "physical" table address does not fit the 48-bit
/// output-address field, so the bits above it are not attributes here.)
fn attrs(desc: u64) -> u64 {
    desc & (0xFFF | (0b11 << 53))
}

/// A slot with a built root, running, as the loader leaves it.
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
        p.set_state(State::Running);
        process::set_current(bp);
        Proc { slot, bp }
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

#[test]
fn a_fresh_root_maps_page_0_read_only_the_stubs_execute_only_the_private_page_read_write() {
    let _g = lock();
    let p = Proc::new();
    let (_, d0) = mmu::translate(p.slot, 0x4F2).expect("page 0 is mapped");
    assert_eq!(attrs(d0), attrs(mmu::page_bits(PageFlags::UserRoNx)), "S2: EL0 read-only, never executable");
    let (_, ds) = mmu::translate(p.slot, STUB_PAGE_VA).expect("the stub page is mapped");
    assert_eq!(attrs(ds), attrs(mmu::page_bits(PageFlags::UserExecOnly)));
    let (_, dp) = mmu::translate(p.slot, PRIV_PAGE_VA + 8).expect("the private page is mapped");
    assert_eq!(attrs(dp), attrs(mmu::page_bits(PageFlags::UserRwNx)));
    // Execute-only: AP = 10, UXN = 0, PXN = 1 (ARM ARM Table D8-65,
    // `PrivRead, UnprivExecute`).
    let x = attrs(mmu::page_bits(PageFlags::UserExecOnly));
    assert_eq!((x >> 6) & 0b11, 0b10, "AP[2:1] = 10");
    assert_eq!(x & (1 << 54), 0, "UXN = 0");
    assert_ne!(x & (1 << 53), 0, "PXN = 1");
    // Page 0 and the private page are the slot's own frames.
    assert_eq!(
        mmu::private_page_alias(p.slot, 0x484),
        mmu::page0_kva(p.slot).map(|k| k as u64 + 0x484)
    );
    assert_eq!(
        mmu::private_page_alias(p.slot, PRIV_PAGE_VA + 0x10),
        mmu::priv_page_kva(p.slot).map(|k| k as u64 + 0x10)
    );
    assert_eq!(mmu::private_page_alias(p.slot, STUB_PAGE_VA), None, "the stub page is no data");
    assert!(mmu::in_private_page(0) && mmu::in_private_page(STUB_PAGE_VA) && mmu::in_private_page(PRIV_PAGE_VA));
    assert!(!mmu::in_private_page(PAGE_SIZE) && !mmu::in_private_page(PRIV_PAGE_VA + PAGE_SIZE));
}

#[test]
fn super_toggles_page_0_and_answers_as_x20_inq_x20_usr_x20_sup() {
    let _g = lock();
    let p = Proc::new();
    let sp = 0x0012_3450u64;
    let rw = attrs(mmu::page_bits(PageFlags::UserRwNx));
    let ro = attrs(mmu::page_bits(PageFlags::UserRoNx));
    let page0 = || attrs(mmu::translate(p.slot, 0).unwrap().1);
    // SAFETY (all `super_call`s): under the lock; the slot's root is built.
    unsafe {
        // S1: off at load. S3: the inquiry changes nothing.
        assert!(!superstate::is_super(p.slot));
        assert_eq!(superstate::super_call(p.slot, 1, sp), 0, "Super(1L) in user state");
        assert_eq!(page0(), ro);
        // S4: Super(0L) enters and hands back the caller's SP as the token.
        assert_eq!(superstate::super_call(p.slot, 0, sp), sp as i64);
        assert!(superstate::is_super(p.slot));
        assert_eq!(page0(), rw, "page 0 EL0-writable in super state");
        assert_eq!(superstate::super_call(p.slot, 1, sp), -1, "Super(1L) in super state");
        // S5: Super(token) = SuperToUser leaves.
        assert_eq!(superstate::super_call(p.slot, sp, sp + 0x40), (sp + 0x40) as i64);
        assert!(!superstate::is_super(p.slot));
        assert_eq!(page0(), ro);
        // O2 = a: an explicit stack from user state enters too and is not
        // used as a stack — the result is still the caller's SP.
        assert_eq!(superstate::super_call(p.slot, 0x7000, sp), sp as i64);
        assert!(superstate::is_super(p.slot));
        superstate::set_super(p.slot, false);
        assert_eq!(page0(), ro);
    }
}

#[test]
fn the_super_state_ends_with_the_process() {
    let _g = lock();
    let slot;
    {
        let p = Proc::new();
        slot = p.slot;
        // SAFETY: under the lock; built root.
        unsafe { superstate::set_super(slot, true) };
        assert!(superstate::is_super(slot));
    }
    // S7: freed with the process; the next tenant starts in user state with
    // page 0 read-only.
    assert!(!superstate::is_super(slot));
    let q = Proc::new();
    assert!(!superstate::is_super(q.slot));
    assert_eq!(attrs(mmu::translate(q.slot, 0).unwrap().1), attrs(mmu::page_bits(PageFlags::UserRoNx)));
}

#[test]
fn no_publication_may_cover_the_private_pages() {
    let _g = lock();
    for va in [STUB_PAGE_VA, PRIV_PAGE_VA, STUB_PAGE_VA - PAGE_SIZE] {
        let len = if va < STUB_PAGE_VA { 2 * PAGE_SIZE } else { PAGE_SIZE };
        // SAFETY: under the lock; refused before any table is touched.
        let r = unsafe { mmu::map_shared(va, len, PageFlags::UserRoNx, false) };
        assert_eq!(r, Err(MapError::VaOutOfRange), "va {va:#x}");
    }
}

#[test]
fn conterm_is_read_through_its_documented_bits_only() {
    let _g = lock();
    let p = Proc::new();
    // SAFETY: under the lock; the slot's page 0.
    let page0 = unsafe { mmu::page0_bytes(p.slot) }.unwrap();
    page0[0x484] = 0xF7; // a program wrote junk above bit 3 in super state
    assert_eq!(tos_api::bios::chardev::current_conterm(), 0x07, "S10: bits 0..3 only");
    page0[0x484] = 0x00; // TeraDesk's former LP64 bug: everything off
    assert_eq!(tos_api::bios::chardev::current_conterm(), 0x00);
    page0[0x484] = 0x0F;
    assert_eq!(tos_api::bios::chardev::current_conterm(), 0x0F);
}

#[test]
fn the_crash_report_names_page_0_and_the_stub_page() {
    use tos_api::bios::crash::RegionKind;
    assert_eq!(process::memprot::region_kind(0x484), Some(RegionKind::SystemVariables));
    assert_eq!(process::memprot::region_kind(STUB_PAGE_VA + 4), Some(RegionKind::StubPage));
    assert_eq!(RegionKind::SystemVariables.name(), "page-0 system-variable");
}

#[test]
fn the_new_dispositions_round_trip() {
    for d in [
        Disposition::SupexecReturn { prev_super: false },
        Disposition::SupexecReturn { prev_super: true },
        Disposition::ResumeSuper { prev_super: false },
        Disposition::ResumeSuper { prev_super: true },
        Disposition::Resume,
        Disposition::UserdefReturn,
        Disposition::Terminate { rc: 0xFFFF, reserve: Some(0x1234) },
    ] {
        assert_eq!(Disposition::decode(d.encode()), d);
    }
}
