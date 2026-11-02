//! TASK/PROC08 — the crash-diagnosis readers of the debug feature
//! `crash-walk`: the checked software walk `mmu::diag::walk` and the rights
//! model's page view `memprot::diag::page_view`, against the real pool,
//! ownership table and page tables (memory protection on, verification mode
//! on), set up like `arch03_3b_memprot.rs`.
//!
//! Checked: a page the slot owns walks to a valid L3 leaf in its private
//! arena table, and the model expects exactly that descriptor; the same page
//! for another slot is invalid in the shared table, and the model agrees;
//! a freed page is invalid for everybody and its former private table shows
//! as a free arena page; a `Pexec(4)` thread sees its caller's page through
//! the memory space; the walk names every table it reads, stops at an
//! invalid entry, and refuses to follow a table pointer that is none of
//! ours (it does not read it); `TTBR0_EL1`-style values with an ASID map back
//! to their slot.
//!
//! The feature is on in host test builds through the crate's
//! dev-dependency on itself (`tos-api/Cargo.toml`).

use std::sync::atomic::Ordering;
use std::sync::{Mutex, MutexGuard, Once};

use tos_api::bdos::mem::traps;
use tos_api::bdos::process::memprot::diag::page_view;
use tos_api::bdos::process::mmu::diag::{slot_of_root, table_kind, walk, Outcome, TableKind};
use tos_api::bdos::process::mmu::{self, USER_VA_LIMIT};
use tos_api::bdos::process::{self, memprot, MAX_PROCESSES, PROCESS_TABLE};
use tos_types::{BasePage, Long, MX_TTRAM};

const ST_START: u64 = 0x0040_0000;
const ST_LEN: u64 = 10 * 1024 * 1024;
const TT_START: u64 = 0x0100_0000;
const TT_LEN: u64 = 64 * 1024 * 1024;
const ARENA_TABLES: usize = 512;

/// The kernel high-half offset the user-table code adds to a table's
/// physical address (`mmu::table_at`); on the host it undoes the wrapping
/// `va_to_pa` of a host address.
const VA_OFFSET: u64 = 0xFFFF_0000_0000_0000;

static LOCK: Mutex<()> = Mutex::new(());
static SETUP: Once = Once::new();

#[repr(C, align(4096))]
struct Page([u8; 4096]);

fn lock() -> MutexGuard<'static, ()> {
    let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    SETUP.call_once(|| {
        let arena: Vec<Page> = (0..ARENA_TABLES).map(|_| Page([0; 4096])).collect();
        let arena = Box::leak(arena.into_boxed_slice());
        // SAFETY: once per binary, before the pools exist.
        unsafe {
            memprot::configure(true, false);
            memprot::set_verify(true, true);
            mmu::set_table_arena(arena.as_mut_ptr() as u64, ARENA_TABLES);
            tos_api::bdos::mem::init::init(ST_START, ST_LEN, TT_START, TT_LEN).expect("pools");
            mmu::map_pools().expect("the pools map");
        }
    });
    g
}

/// A process with a basepage of its own (host memory) in a claimed slot
/// with a root.
struct Proc {
    slot: usize,
    bp: *mut BasePage,
}

impl Proc {
    fn new() -> Proc {
        let slot = process::allocate_slot().expect("a slot");
        let b = Box::new([0u8; 256]);
        let bp = Box::into_raw(b) as *mut BasePage;
        let p = &PROCESS_TABLE[slot];
        p.basepage_kva.store(bp, Ordering::Release);
        // SAFETY: under the lock; the slot is not an installed root.
        unsafe { mmu::init_slot_tables(slot) }.unwrap();
        p.l0_pa.store(mmu::l0_pa_for(slot), Ordering::Release);
        p.set_state(process::State::Running);
        // SAFETY: under the lock.
        unsafe { memprot::on_root_built(slot) };
        Proc { slot, bp }
    }

    fn pd(&self) -> *mut core::ffi::c_void {
        process::pd_of(self.slot).cast()
    }

    fn alloc(&self, n: Long, mode: i16) -> u64 {
        process::set_current(process::pd_of(self.slot));
        // SAFETY: under the lock.
        let a = unsafe { traps::mxalloc(n, mode) };
        process::set_current(std::ptr::null_mut());
        assert_ne!(a, 0, "allocation");
        a as u32 as u64
    }
}

impl Drop for Proc {
    fn drop(&mut self) {
        // SAFETY: under the lock.
        unsafe { tos_api::bdos::mem::free_all_owned::free_all_owned(self.pd()) };
        process::free_slot(self.slot);
        // SAFETY: allocated by `Box::new` in `new`.
        drop(unsafe { Box::from_raw(self.bp as *mut [u8; 256]) });
    }
}

const GIB: u64 = 1 << 30;
const REGION: u64 = 2 << 20;

#[test]
fn an_owned_page_walks_valid_and_the_model_expects_that_descriptor() {
    let _g = lock();
    let a = Proc::new();
    let blk = a.alloc(0x1000, MX_TTRAM);
    let w = walk(a.slot, blk);
    assert_eq!(w.outcome, Outcome::Leaf { level: 3, oa: blk }, "VA = PA");
    assert_eq!(w.root_pa, mmu::l0_pa_for(a.slot));
    let kinds: Vec<TableKind> = w.steps.iter().map(|s| s.expect("four levels").kind).collect();
    assert_eq!(kinds[0], TableKind::SlotL0(a.slot));
    assert_eq!(kinds[1], TableKind::SlotL1(a.slot));
    assert_eq!(kinds[2], TableKind::SlotL2(a.slot, (blk / GIB) as usize));
    assert!(matches!(kinds[3], TableKind::Arena { used: true, .. }), "a private arena table: {:?}", kinds[3]);
    assert_eq!(w.l3_also_slots, 0, "a private table is named by its slot alone");
    assert!(!w.l3_also_template);
    let leaf = w.steps[3].unwrap().desc;
    // Agrees with the unchecked walk.
    assert_eq!(mmu::translate(a.slot, blk), Some((blk, leaf)));

    // SAFETY: under the lock.
    let v = unsafe { page_view(a.slot, blk) };
    assert!(v.enabled && v.in_pool && v.lists_ok && v.slot_live);
    let b = v.block.expect("the block");
    assert_eq!((b.start, b.end), (blk, blk + 0x1000));
    assert_eq!(b.own, a.pd() as u64);
    assert_eq!(b.owners, 1 << a.slot);
    assert!(v.holds);
    assert_eq!(v.group & (1 << a.slot), 1 << a.slot);
    assert_eq!(v.expected, Some(leaf), "the model expects the descriptor the table holds");
    assert_eq!(v.expected_shared, Some(0), "PROT_P: nobody else");
    assert_eq!(v.needs_private, Some(true));
    assert!(v.has_private);
    assert!(!v.pending, "every BDOS operation ends with a sync");
    // SAFETY: under the lock.
    unsafe { assert_eq!(traps::mfree(blk as Long), 0) };
}

#[test]
fn another_slot_sees_the_page_invalid_in_the_shared_table_and_the_model_agrees() {
    let _g = lock();
    let a = Proc::new();
    let b = Proc::new();
    let blk = a.alloc(0x1000, MX_TTRAM);
    let w = walk(b.slot, blk);
    assert_eq!(w.outcome, Outcome::Invalid { level: 3 });
    let l3 = w.steps[3].expect("the L3 was read");
    assert_eq!(l3.desc, 0);
    assert!(matches!(l3.kind, TableKind::Arena { used: true, .. }));
    assert!(w.l3_also_template, "b walks the region's shared L3");
    // SAFETY: under the lock.
    let v = unsafe { page_view(b.slot, blk) };
    assert!(!v.holds);
    assert_eq!(v.block.map(|x| x.owners), Some(1 << a.slot));
    assert_eq!(v.expected, Some(0));
    assert!(!v.has_private);
    // SAFETY: under the lock.
    unsafe { assert_eq!(traps::mfree(blk as Long), 0) };
}

#[test]
fn a_freed_page_is_invalid_and_its_private_table_shows_as_a_free_arena_page() {
    let _g = lock();
    let a = Proc::new();
    let blk = a.alloc(0x1000, MX_TTRAM);
    let private_l3 = walk(a.slot, blk).steps[3].unwrap().table_pa;
    let TableKind::Arena { idx, used: true } = table_kind(private_l3) else {
        panic!("a used arena table");
    };
    // SAFETY: under the lock.
    unsafe { assert_eq!(traps::mfree(blk as Long), 0) };
    // The difference is gone: the slot walks the shared table again, and the
    // page is invalid there.
    let w = walk(a.slot, blk);
    assert_eq!(w.outcome, Outcome::Invalid { level: 3 });
    assert!(w.l3_also_template);
    assert_ne!(w.steps[3].unwrap().table_pa, private_l3);
    // The old private table went back to the arena.
    assert_eq!(table_kind(private_l3), TableKind::Arena { idx, used: false });
    // SAFETY: under the lock.
    let v = unsafe { page_view(a.slot, blk) };
    assert_eq!(v.block, None, "free pool memory");
    assert_eq!(v.expected, Some(0), "nobody's (I8)");
}

#[test]
fn a_pexec4_thread_sees_its_callers_page_through_the_memory_space() {
    let _g = lock();
    let parent = Proc::new();
    let child = Proc::new();
    let mine = parent.alloc(0x1000, MX_TTRAM);
    PROCESS_TABLE[child.slot].space_pd.store(parent.pd().cast(), Ordering::Release);
    // SAFETY: under the lock.
    unsafe { memprot::on_launch(child.slot, parent.pd().cast()) };
    let w = walk(child.slot, mine);
    assert!(matches!(w.outcome, Outcome::Leaf { level: 3, .. }));
    // SAFETY: under the lock.
    let v = unsafe { page_view(child.slot, mine) };
    assert_eq!(v.space_root, parent.pd() as u64);
    assert_eq!(v.group, (1 << parent.slot) | (1 << child.slot));
    assert!(v.holds);
    assert_eq!(v.expected, Some(w.steps[3].unwrap().desc));
    // And the parent keeps it.
    // SAFETY: under the lock.
    let pv = unsafe { page_view(parent.slot, mine) };
    assert!(pv.holds);
    assert_eq!(pv.expected, Some(walk(parent.slot, mine).steps[3].unwrap().desc));
    PROCESS_TABLE[child.slot].space_pd.store(std::ptr::null_mut(), Ordering::Release);
}

#[test]
fn the_walk_stops_at_an_invalid_entry_and_at_the_edges() {
    let _g = lock();
    let a = Proc::new();
    // GiB 2 holds nothing on the host: the slot's own L2 there is empty.
    let w = walk(a.slot, 2 * GIB + 0x1000);
    assert_eq!(w.outcome, Outcome::Invalid { level: 2 });
    assert_eq!(w.steps[2].unwrap().kind, TableKind::SlotL2(a.slot, 2));
    assert_eq!(w.steps[3], None);
    assert_eq!(walk(a.slot, USER_VA_LIMIT).outcome, Outcome::OutOfRange);
    assert_eq!(walk(MAX_PROCESSES, 0x1000).outcome, Outcome::BadSlot);
    // Page 0 lives in the slot's own [0, 2 MiB) L3.
    let w0 = walk(a.slot, 0x10);
    assert_eq!(w0.steps[3].unwrap().kind, TableKind::SlotL3Zero(a.slot));
}

#[test]
fn a_table_pointer_that_is_none_of_ours_is_not_followed() {
    let _g = lock();
    let a = Proc::new();
    let va = 2 * GIB + 5 * REGION;
    let l2 = walk(a.slot, va).steps[2].expect("the slot's L2 was read");
    assert_eq!(l2.kind, TableKind::SlotL2(a.slot, 2));
    // Corrupt the slot's own L2 entry for the region with a table descriptor
    // naming memory that is no table of ours, walk, restore.
    let entry = (l2.table_pa.wrapping_add(VA_OFFSET) as *mut u64).wrapping_add(l2.index);
    let bad = 0x0DEA_D000u64;
    // SAFETY: the slot's own static L2 (its address from the walk), under
    // the lock; restored below.
    unsafe {
        assert_eq!(entry.read_volatile(), 0);
        entry.write_volatile(bad | 0b11);
    }
    let w = walk(a.slot, va);
    // SAFETY: as above.
    unsafe { entry.write_volatile(0) };
    assert_eq!(w.outcome, Outcome::BadTable { level: 3, pa: bad });
    assert_eq!(w.steps[2].unwrap().desc, bad | 0b11);
    assert_eq!(w.steps[3], None, "not read");
    assert_eq!(table_kind(bad), TableKind::Unknown);
    assert_eq!(table_kind(mmu::l0_pa_for(a.slot) + 8), TableKind::Unknown, "misaligned");
    assert_eq!(walk(a.slot, va).outcome, Outcome::Invalid { level: 2 }, "restored");
}

#[test]
fn a_ttbr0_value_maps_back_to_its_slot() {
    let _g = lock();
    let a = Proc::new();
    let root = mmu::l0_pa_for(a.slot);
    assert_eq!(table_kind(root), TableKind::SlotL0(a.slot));
    assert_eq!(slot_of_root(root), Some(a.slot));
    assert_eq!(slot_of_root(root | (5u64 << 48)), Some(a.slot), "the ASID is ignored");
    assert_eq!(slot_of_root(0x0123_4000), None);
}
