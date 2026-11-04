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

// ─── The rendered diagnosis: serial lines and the screen block ──────────
//
// `bdos::process::crash_diag` collects the readers' answers once and renders
// them twice. The serial lines must stay those of the first PROC08 run; the
// screen block must fit the console and carry the same answers.

use tos_api::bdos::process::crash_diag::{
    self, collect, probe_addresses, write_screen, write_serial, CrashDiag, HwProbe, ProbeDiag, RootOwner,
};
use tos_api::bdos::process::memprot::diag::{BlockView, PageView};
use tos_api::bdos::process::mmu::diag::{Step, Walk};
use tos_api::bios::crash::{CrashKind, CrashReport};
use tos_types::ProtMode;

/// The walk of case `d` of the first PROC08 run: slot 1, root 0x2e6000,
/// its own L0/L1/L2 and the private arena table 454 at L3, entry `l3_index`.
fn d_walk(va: u64, l3_index: usize, l3_desc: u64, outcome: Outcome) -> Walk {
    let step = |table_pa, kind, index, desc| Some(Step { table_pa, kind, index, desc });
    Walk {
        slot: 1,
        va,
        root_pa: 0x2e6000,
        steps: [
            step(0x2e6000, TableKind::SlotL0(1), 0, 0x2e7003),
            step(0x2e7000, TableKind::SlotL1(1), 0, 0x2e8003),
            step(0x2e8000, TableKind::SlotL2(1, 0), 13, 0x11c6003),
            step(0x11c6000, TableKind::Arena { idx: 454, used: true }, l3_index, l3_desc),
        ],
        outcome,
        l2_template: Some(0x1006003),
        l3_also_slots: 0,
        l3_also_template: false,
    }
}

fn d_view(block: Option<BlockView>, holds: bool, expected: u64) -> PageView {
    PageView {
        enabled: true,
        in_pool: true,
        lists_ok: true,
        block,
        live: 0x3,
        slot_live: true,
        slot_kva: 0xffff_0000_01a1_a000,
        space_root: 0x1a1a000,
        group: 0x2,
        temp: 0,
        holds,
        expected: Some(expected),
        expected_shared: Some(0),
        needs_private: Some(true),
        has_private: true,
        shortfalls: 0,
        pending: false,
    }
}

/// Case `d` of `measurements/measurement-2026-11-02-proc08-diag/runs/r1-nowait-serial/SERIAL.txt`
/// (lines 92-106): a data read of a freed pool page (far, not mapped) from
/// the program's own text page (pc, mapped).
fn case_d() -> CrashDiag {
    let far = ProbeDiag {
        hw: HwProbe { label: "far", va: 0x1a2c000, par: 0x80f },
        walk: d_walk(0x1a2c000, 44, 0, Outcome::Invalid { level: 3 }),
        other: None,
        view: d_view(None, false, 0),
    };
    let block = BlockView {
        start: 0x1a1a000,
        end: 0x1a2c000,
        mode: Some(ProtMode::P),
        inherit: 0,
        own: 0x1a1a000,
        further: [0; 4],
        owners: 0x2,
    };
    let pc = ProbeDiag {
        hw: HwProbe { label: "pc", va: 0x1a1b1b0, par: 0xff00_0000_01a1_bb80 },
        walk: d_walk(0x1a1b1b0, 27, 0x0020_0000_01a1_b743, Outcome::Leaf { level: 3, oa: 0x1a1b1b0 }),
        other: None,
        view: d_view(Some(block), true, 0x0020_0000_01a1_b743),
    };
    CrashDiag {
        slot: 1,
        el0: true,
        esr: 0x9200_0007,
        pc: 0x1a1b1b0,
        far: Some(0x1a2c000),
        ttbr0: 0x2e6000,
        root: RootOwner::Slot(1),
        expected: 0x2e6000,
        recorded: 0x2e6000,
        probes: [Some(far), Some(pc)],
        ladder: None,
    }
}

/// The serial lines of case `d`, verbatim from the run (lines 93-106).
const CASE_D_SERIAL: &str = "\
[crashwalk] slot 1 src=el0 esr=0x92000007 pc=0x1a1b1b0 far=0x1a2c000
[crashwalk] ttbr0=0x00000000002e6000 asid=0 root=slot 1; slot 1 expects 0x2e6000 records 0x2e6000 -> same
[crashwalk] far va=0x1a2c000 walk slot 1 root 0x2e6000: L0[0]=0x00000000002e7003(s1.L0) L1[0]=0x00000000002e8003(s1.L1) L2[13]=0x00000000011c6003(s1.L2g0) L3[44]=0x0000000000000000(arena#454 used) -> INVALID at L3
[crashwalk] far l2 template=0x0000000001006003 l3 also named by slots=0x0 template=no
[crashwalk] far AT S1E0R par=0x000000000000080f: fault fst=0x07 (translation L3) s=0 ptw=0
[crashwalk] far model slot 1: pool=yes block none
[crashwalk] far model slot 1: live=0x3 slot_live=yes slot_kva=0xffff000001a1a000 space=0x1a1a000 group=0x2 temp=0x0 holds=no expect=0x0000000000000000 shared=0x0000000000000000 private needed=yes has=yes shortfalls=0 pending=no
[crashwalk] far summary: model=invalid table=invalid at=fault table==model yes
[crashwalk] pc va=0x1a1b1b0 walk slot 1 root 0x2e6000: L0[0]=0x00000000002e7003(s1.L0) L1[0]=0x00000000002e8003(s1.L1) L2[13]=0x00000000011c6003(s1.L2g0) L3[27]=0x0020000001a1b743(arena#454 used) -> mapped L3 oa=0x1a1b1b0
[crashwalk] pc l2 template=0x0000000001006003 l3 also named by slots=0x0 template=no
[crashwalk] pc AT S1E0R par=0xff00000001a1bb80: ok pa=0x1a1b1b0
[crashwalk] pc model slot 1: pool=yes block 0x1a1a000-0x1a2c000 mode=P own=0x1a1a000 further=0x0,0x0,0x0,0x0 owners=0x2 inherit=0x0
[crashwalk] pc model slot 1: live=0x3 slot_live=yes slot_kva=0xffff000001a1a000 space=0x1a1a000 group=0x2 temp=0x0 holds=yes expect=0x0020000001a1b743 shared=0x0000000000000000 private needed=yes has=yes shortfalls=0 pending=no
[crashwalk] pc summary: model=valid table=valid at=ok table==model yes
";

#[test]
fn the_serial_lines_are_those_of_the_first_run() {
    let mut s = String::new();
    write_serial(&mut s, &case_d()).unwrap();
    assert_eq!(s, CASE_D_SERIAL);
}

#[test]
fn the_screen_block_carries_the_same_answers_in_short_lines() {
    let mut s = String::new();
    write_screen(&mut s, &case_d()).unwrap();
    let want = "\
\n\
crash-walk slot 1 el0: ttbr0 2e6000 asid 0 = slot 1, exp 2e6000 rec 2e6000 same\n\
far 1a2c000 s1: L0[0]=2e7003 s1.L0  L1[0]=2e8003 s1.L1  L2[13]=11c6003 s1.L2g0\n  \
L3[44]=0 arena#454 used -> INVALID at L3  also 0 tpl no\n  \
AT S1E0R par=80f: FAULT translation L3 fst=07 s=0 ptw=0\n  \
model: pool yes, no block\n  \
model: holds no expect 0 shared 0 pend no\n  \
model: priv need yes has yes short 0 live 3 group 2 temp 0 space 1a1a000\n  \
=> model=invalid table=invalid at=fault table==model yes\n\
pc 1a1b1b0 s1: L0[0]=2e7003 s1.L0  L1[0]=2e8003 s1.L1  L2[13]=11c6003 s1.L2g0\n  \
L3[27]=20000001a1b743 arena#454 used -> mapped L3 oa=1a1b1b0  also 0 tpl no\n  \
AT S1E0R par=ff00000001a1bb80: ok pa=1a1b1b0\n  \
model: pool yes block 1a1a000-1a2c000 mode P own 1a1a000 owners 2 inh 0\n  \
model: holds yes expect 20000001a1b743 shared 0 pend no\n  \
model: priv need yes has yes short 0 live 3 group 2 temp 0 space 1a1a000\n  \
=> model=valid table=valid at=ok table==model yes\n";
    assert_eq!(s, want);
    // A blank line, the header, seven lines per address.
    assert_eq!(s.lines().count(), 2 + 2 * 7);
    for l in s.lines() {
        assert!(l.len() <= 80, "{} columns: {l}", l.len());
        assert!(l.bytes().all(|b| (0x20..0x7f).contains(&b)), "printable ASCII only: {l:?}");
    }
}

#[test]
fn the_screen_block_shows_a_foreign_root_and_memprot_off() {
    // TTBR0 holds slot 3's root (the weakly refuted "wrong root" case): a
    // second walk through slot 3's tables follows, the header says
    // DIFFERENT. Memory protection off: one model line.
    let mut d = case_d();
    d.ttbr0 = 0x2f0000 | (7u64 << 48);
    d.root = RootOwner::Slot(3);
    let mut far = d.probes[0].unwrap();
    let mut other = far.walk;
    other.slot = 3;
    other.root_pa = 0x2f0000;
    far.other = Some(other);
    far.view.enabled = false;
    d.probes = [Some(far), None];
    let mut s = String::new();
    write_screen(&mut s, &d).unwrap();
    let lines: Vec<&str> = s.lines().collect();
    assert_eq!(lines[1], "crash-walk slot 1 el0: ttbr0 2f0000 asid 7 = slot 3, exp 2e6000 rec 2e6000 DIFFERENT");
    assert!(lines[2].starts_with("far 1a2c000 s1: L0[0]="), "{}", lines[2]);
    assert!(lines[4].starts_with("far 1a2c000 s3: L0[0]="), "{}", lines[4]);
    assert_eq!(lines[6], "  AT S1E0R par=80f: FAULT translation L3 fst=07 s=0 ptw=0");
    assert_eq!(lines[7], "  model: memprot off");
    assert_eq!(lines[8], "  => model=n/a table=invalid at=fault table==model n/a");
    assert_eq!(lines.len(), 9);
    // The serial lines carry the second walk too.
    let mut ser = String::new();
    write_serial(&mut ser, &d).unwrap();
    assert!(ser.contains("root=slot 3; slot 1 expects 0x2e6000 records 0x2e6000 -> DIFFERENT\n"));
    assert!(ser.contains("[crashwalk] far va=0x1a2c000 walk slot 3 root 0x2f0000:"));
    assert!(ser.contains("[crashwalk] far model slot 1: memprot off\n"));
}

#[test]
fn the_screen_block_names_where_a_short_walk_stopped() {
    // Case `g` of the run: GiB 2, the walk ends at L2; no L3 step.
    let mut d = case_d();
    let mut far = d.probes[0].unwrap();
    far.hw = HwProbe { label: "far", va: 0x9000_0000, par: 0x80d };
    far.walk.va = 0x9000_0000;
    far.walk.steps[1] = Some(Step { table_pa: 0x2e7000, kind: TableKind::SlotL1(1), index: 2, desc: 0x2ea003 });
    far.walk.steps[2] = Some(Step { table_pa: 0x2ea000, kind: TableKind::SlotL2(1, 2), index: 128, desc: 0 });
    far.walk.steps[3] = None;
    far.walk.outcome = Outcome::Invalid { level: 2 };
    far.walk.l2_template = Some(0);
    far.view.in_pool = false;
    far.view.expected = None;
    far.view.expected_shared = None;
    d.probes[0] = Some(far);
    let mut s = String::new();
    write_screen(&mut s, &d).unwrap();
    let lines: Vec<&str> = s.lines().collect();
    assert_eq!(lines[2], "far 90000000 s1: L0[0]=2e7003 s1.L0  L1[2]=2ea003 s1.L1  L2[128]=0 s1.L2g2");
    assert_eq!(lines[3], "  -> INVALID at L2  also 0 tpl no");
    assert_eq!(lines[4], "  AT S1E0R par=80d: FAULT translation L2 fst=06 s=0 ptw=0");
    assert_eq!(lines[5], "  model: pool no, no block");
    assert_eq!(lines[6], "  model: holds no expect n/a shared n/a pend no");
    assert_eq!(lines[8], "  => model=n/a table=invalid at=fault table==model n/a");
}

fn report<'a>(regs: &'a [u64; 31], pc: u64, far: Option<u64>, pstate: u64) -> CrashReport<'a> {
    CrashReport {
        kind: CrashKind::BusError,
        esr: 0x9200_0007,
        far,
        pc,
        pstate,
        regs,
        usp: 0,
        program: None,
        violation: None,
    }
}

#[test]
fn the_probed_addresses_are_far_and_a_different_pc_page_from_el0() {
    let regs = [0u64; 31];
    // Instruction abort: far = pc, one probe.
    assert_eq!(probe_addresses(&report(&regs, 0x1a2c000, Some(0x1a2c000), 0)), [Some(("far", 0x1a2c000)), None]);
    // Same page, other offset: one probe.
    assert_eq!(probe_addresses(&report(&regs, 0x1a2c010, Some(0x1a2cff8), 0)), [Some(("far", 0x1a2cff8)), None]);
    // Data abort from another page: both.
    assert_eq!(
        probe_addresses(&report(&regs, 0x1a1b1b0, Some(0x1a2c000), 0)),
        [Some(("far", 0x1a2c000)), Some(("pc", 0x1a1b1b0))]
    );
    // A kernel user access (EL1h): the PC is the kernel's, not probed.
    assert_eq!(probe_addresses(&report(&regs, 0xffff_0000_0008_0000, Some(0x1000), 0x5)), [Some(("far", 0x1000)), None]);
    // No fault address from EL0: the PC alone.
    assert_eq!(probe_addresses(&report(&regs, 0x1a1b1b0, None, 0)), [None, Some(("pc", 0x1a1b1b0))]);
}

#[test]
fn collect_renders_the_readers_answers_once_for_both_outputs() {
    let _g = lock();
    let a = Proc::new();
    let b = Proc::new();
    let blk = a.alloc(0x1000, MX_TTRAM);
    let regs = [0u64; 31];
    let r = report(&regs, blk + 0x10, Some(blk + 0x10), 0);
    let hw = [Some(HwProbe { label: "far", va: blk + 0x10, par: blk }), None];

    // TTBR0 holds the slot's own root. A host "physical" table address is
    // the wrapped host address and reaches into bits [63:48] (TTBR0's ASID
    // field), so the BADDR comparison (`roots_same`) cannot hold on the
    // host; the ASID and `same`/`DIFFERENT` are covered by the rendering
    // tests above.
    let root = mmu::l0_pa_for(a.slot);
    // SAFETY: under the lock.
    let d = unsafe { collect(a.slot, &r, root, hw, 0) };
    assert_eq!(d.root, RootOwner::Slot(a.slot));
    assert_eq!((d.expected, d.recorded), (root, root));
    let p = d.probes[0].expect("the far probe");
    assert_eq!(p.walk, walk(a.slot, blk + 0x10));
    // SAFETY: under the lock.
    assert_eq!(p.view, unsafe { page_view(a.slot, blk + 0x10) });
    assert_eq!(p.other, None);
    assert_eq!(d.probes[1], None);
    let s = crash_diag::summary(&p);
    assert_eq!((s.model, s.table, s.at, s.same), ("valid", "valid", "ok", "yes"));
    let mut scr = String::new();
    write_screen(&mut scr, &d).unwrap();
    assert!(scr.contains(&format!(" = slot {}, exp {root:x} rec {root:x} ", a.slot)), "{scr}");
    assert!(scr.contains(&format!("-> mapped L3 oa={:x}", blk + 0x10)), "{scr}");
    assert!(scr.contains("  => model=valid table=valid at=ok table==model yes\n"), "{scr}");

    // TTBR0 holds b's root: a second walk through b's tables, which do not
    // map a's page.
    // SAFETY: under the lock.
    let d = unsafe { collect(a.slot, &r, mmu::l0_pa_for(b.slot), hw, 0) };
    assert_eq!(d.root, RootOwner::Slot(b.slot));
    let other = d.probes[0].unwrap().other.expect("b's walk");
    assert_eq!(other.slot, b.slot);
    assert_eq!(other.outcome, Outcome::Invalid { level: 3 });
    // The empty root and a stranger.
    // SAFETY: under the lock.
    let d = unsafe { collect(a.slot, &r, 0x0123_4000, hw, 0x0123_4000) };
    assert_eq!(d.root, RootOwner::Empty);
    // SAFETY: under the lock.
    let d = unsafe { collect(a.slot, &r, 0x0123_4000, hw, 0) };
    assert_eq!(d.root, RootOwner::Unknown);
    // SAFETY: under the lock.
    unsafe { assert_eq!(traps::mfree(blk as Long), 0) };
}

// ─── The ladder (TASK/PROC08 "Stand Leiter") ────────────────────────────
//
// The kernel runs the instructions (`LadderHw`); the order, the stop at the
// first success, the end and both lines are tos-api's. A scripted hardware
// answers `AT` on the faulting address from a list, one answer per rung,
// and logs every call.

use tos_api::bdos::process::crash_diag::{run_ladder, Ladder, LadderEnd, LadderHw, Rung, RUNGS};
use tos_api::bdos::process::mmu::diag::entry_kva;

/// `PAR_EL1` of a translation fault at level 3 (the PROC08 photo).
const F3: u64 = 0x80f;
/// A successful `PAR_EL1` for the PROC08 page.
const OK: u64 = 0xff00_0000_01cb_4b80;
/// The PROC08 fault address (zView, text+0x2e97c) and a stack pointer.
const VA: u64 = 0x1cb_497c;
const USP: u64 = 0x1d7_cf80;

struct Script {
    /// Answers for `AT` on `VA`, in order; the last one repeats.
    far: Vec<u64>,
    sp: u64,
    private: u64,
    log: Vec<String>,
}

impl Script {
    fn new(far: &[u64], sp: u64, private: u64) -> Script {
        Script { far: far.to_vec(), sp, private, log: Vec::new() }
    }
}

impl LadderHw for Script {
    fn at_s1e0r(&mut self, va: u64) -> u64 {
        self.log.push(format!("at {va:x}"));
        match va {
            VA => {
                if self.far.len() > 1 {
                    self.far.remove(0)
                } else {
                    self.far[0]
                }
            }
            USP => self.sp,
            mmu::PRIV_PAGE_VA => self.private,
            _ => panic!("AT on an unexpected address {va:x}"),
        }
    }

    fn maintain(&mut self, rung: Rung, va: u64, lines: &[u64]) {
        assert_eq!(va, VA);
        self.log.push(format!("{} {}", rung.name(), lines.len()));
    }
}

/// The PROC08 photo as a `CrashDiag`: an instruction abort (far = pc) in
/// slot 2, root right, the page mapped in arena table 361, `AT` fault L3.
fn proc08_photo() -> CrashDiag {
    let step = |table_pa, kind, index, desc| Some(Step { table_pa, kind, index, desc });
    let walk = Walk {
        slot: 2,
        va: VA,
        root_pa: 0x2f0000,
        steps: [
            step(0x2f0000, TableKind::SlotL0(2), 0, 0x2f1003),
            step(0x2f1000, TableKind::SlotL1(2), 0, 0x2f2003),
            step(0x2f2000, TableKind::SlotL2(2, 0), 14, 0x1169003),
            step(0x1169000, TableKind::Arena { idx: 361, used: true }, 180, 0x0020_0000_01cb_4743),
        ],
        outcome: Outcome::Leaf { level: 3, oa: VA },
        l2_template: Some(0x1006003),
        l3_also_slots: 0,
        l3_also_template: false,
    };
    let mut d = case_d();
    d.slot = 2;
    d.esr = 0x8200_0007;
    d.pc = VA;
    d.far = Some(VA);
    d.probes = [
        Some(ProbeDiag {
            hw: HwProbe { label: "far", va: VA, par: F3 },
            walk,
            other: None,
            view: d_view(None, true, 0x0020_0000_01cb_4743),
        }),
        None,
    ];
    d
}

fn ladder_of(far: &[u64], sp: u64, private: u64) -> (Ladder, Vec<String>) {
    let mut hw = Script::new(far, sp, private);
    let l = run_ladder(&mut hw, &proc08_photo(), USP).expect("a probed address");
    (l, hw.log)
}

fn lines_of(l: &Ladder) -> (String, String) {
    let mut d = proc08_photo();
    d.ladder = Some(*l);
    let mut scr = String::new();
    write_screen(&mut scr, &d).unwrap();
    let mut ser = String::new();
    write_serial(&mut ser, &d).unwrap();
    let pick = |s: &str, key: &str| s.lines().find(|x| x.contains(key)).expect("a ladder line").to_string();
    (pick(&scr, "ladder:"), pick(&ser, "ladder va="))
}

#[test]
fn the_ladder_runs_its_rungs_in_order_and_stops_at_the_first_success() {
    // R1 of the analysis: only the all-level TLBI on the address helps.
    let (l, log) = ladder_of(&[F3, F3, F3, F3, F3, OK], F3, OK);
    assert_eq!(
        log,
        [
            "at 1cb497c", // L1
            "at 1d7cf80",
            "at 7f000",
            "at 1cb497c", // L2
            "dsb 4",
            "at 1cb497c",
            "dc 4",
            "at 1cb497c",
            "vaale1 4",
            "at 1cb497c",
            "vaae1 4",
            "at 1cb497c",
        ]
    );
    assert_eq!(l.end(), LadderEnd::FixedBy(Rung::Vaae1is));
    assert_eq!(l.after, [Some(F3), Some(F3), Some(F3), Some(F3), Some(F3), Some(OK), None]);
    assert_eq!((l.sp_par, l.priv_par), (Some(F3), Some(OK)));
    assert_eq!((l.label, l.va, l.start, l.sp), ("far", VA, F3, USP));
}

#[test]
fn the_ladder_ends_name_the_candidates() {
    // The expected outputs of the analysis, section 9.1.
    let end = |far: &[u64]| ladder_of(far, F3, OK).0.end();
    assert_eq!(end(&[OK]), LadderEnd::Repeat, "R4: transient");
    assert_eq!(end(&[F3, OK]), LadderEnd::FixedBy(Rung::Neighbours));
    assert_eq!(end(&[F3, F3, OK]), LadderEnd::FixedBy(Rung::Barrier));
    assert_eq!(end(&[F3, F3, F3, OK]), LadderEnd::FixedBy(Rung::CleanLines), "R2");
    assert_eq!(end(&[F3, F3, F3, F3, OK]), LadderEnd::FixedBy(Rung::Vaale1is), "the contradiction (RXCLRD)");
    assert_eq!(end(&[F3, F3, F3, F3, F3, OK]), LadderEnd::FixedBy(Rung::Vaae1is), "R1");
    assert_eq!(end(&[F3, F3, F3, F3, F3, F3, OK]), LadderEnd::FixedBy(Rung::Vmalle1is), "R3");
    let (l, log) = ladder_of(&[F3], F3, OK);
    assert_eq!(l.end(), LadderEnd::None);
    assert_eq!(l.after, [Some(F3); 7], "every rung ran");
    assert_eq!(log.iter().filter(|x| x.starts_with("at 1cb497c")).count(), RUNGS.len());
    assert_eq!(log.last().map(String::as_str), Some("at 1cb497c"));
}

#[test]
fn the_ladder_does_not_run_when_the_first_at_succeeded() {
    let mut d = proc08_photo();
    let mut p = d.probes[0].unwrap();
    p.hw.par = OK;
    d.probes[0] = Some(p);
    let mut hw = Script::new(&[F3], F3, F3);
    let l = run_ladder(&mut hw, &d, USP).unwrap();
    assert!(hw.log.is_empty(), "no instruction: {:?}", hw.log);
    assert_eq!(l.end(), LadderEnd::NotRun);
    assert_eq!(l.after, [None; 7]);
    // No probed address: no ladder.
    d.probes = [None, None];
    assert_eq!(run_ladder(&mut hw, &d, USP), None);
}

#[test]
fn the_ladder_cleans_the_lines_of_every_walk_it_read() {
    let mut d = proc08_photo();
    let mut p = d.probes[0].unwrap();
    let mut other = p.walk;
    other.slot = 3;
    other.steps[3] = None; // stopped at L2
    p.other = Some(other);
    d.probes[0] = Some(p);
    let mut hw = Script::new(&[F3, F3, F3, OK], F3, OK);
    let l = run_ladder(&mut hw, &d, USP).unwrap();
    assert_eq!(l.n_lines, 7);
    let want: Vec<u64> = p.walk.steps.iter().chain(other.steps.iter()).flatten().map(entry_kva).collect();
    assert_eq!(&l.lines[..7], &want[..]);
    assert!(hw.log.contains(&"dc 7".to_string()));
    // The address of a descriptor: its table's kernel alias plus 8 per entry.
    let s = p.walk.steps[3].unwrap();
    assert_eq!(entry_kva(&s) - entry_kva(&Step { index: 0, ..s }), 180 * 8);
}

#[test]
fn the_ladder_lines_on_screen_and_serial() {
    // R1.
    let (l, _) = ladder_of(&[F3, F3, F3, F3, F3, OK], F3, OK);
    let (scr, ser) = lines_of(&l);
    assert_eq!(scr, "  ladder: L1 F3 sp F3 priv ok far F3 dsb F3 dc F3 vaale1 F3 vaae1 ok => fixed-by vaae1");
    let k = |i: usize| entry_kva(&proc08_photo().probes[0].unwrap().walk.steps[i].unwrap());
    assert_eq!(
        ser,
        format!(
            "[crashwalk] far ladder va=0x1cb497c: usp=0x1d7cf80 priv=0x7f000 lines=0x{:x},0x{:x},0x{:x},0x{:x}; \
             L1 par=0x000000000000080f sp+priv sp=0x000000000000080f priv=0xff00000001cb4b80 far par=0x000000000000080f \
             dsb par=0x000000000000080f dc par=0x000000000000080f vaale1 par=0x000000000000080f \
             vaae1 par=0xff00000001cb4b80 => fixed-by vaae1",
            k(0),
            k(1),
            k(2),
            k(3)
        )
    );
    // R4, R2, R3, none, not run.
    assert_eq!(lines_of(&ladder_of(&[OK], F3, OK).0).0, "  ladder: L1 ok => repeat");
    assert_eq!(
        lines_of(&ladder_of(&[F3, F3, F3, OK], F3, OK).0).0,
        "  ladder: L1 F3 sp F3 priv ok far F3 dsb F3 dc ok => fixed-by dc"
    );
    assert_eq!(
        lines_of(&ladder_of(&[F3, F3, F3, F3, F3, F3, OK], F3, OK).0).0,
        "  ladder: L1 F3 sp F3 priv ok far F3 dsb F3 dc F3 vaale1 F3 vaae1 F3 vmalle1 ok => fixed-by vmalle1"
    );
    let (none, none_ser) = lines_of(&ladder_of(&[F3], F3, OK).0);
    assert_eq!(none, "  ladder: L1 F3 sp F3 priv ok far F3 dsb F3 dc F3 vaale1 F3 vaae1 F3 vmalle1 F3 => none");
    assert!(none_ser.ends_with(" vmalle1 par=0x000000000000080f => none"), "{none_ser}");
    assert_eq!(lines_of(&ladder_of(&[F3, OK], F3, OK).0).0, "  ladder: L1 F3 sp F3 priv ok far ok => fixed-by sp+priv");
    // A fault other than translation shows its FST: a permission fault L3
    // (FST 0b001111) on the stack page.
    assert_eq!(
        lines_of(&ladder_of(&[F3, OK], 0x81f, OK).0).0,
        "  ladder: L1 F3 sp x0f priv ok far ok => fixed-by sp+priv"
    );
    let mut ok = ladder_of(&[F3], F3, OK).0;
    ok.start = OK;
    ok.after = [None; 7];
    let (scr, ser) = lines_of(&ok);
    assert_eq!(scr, "  ladder: not run, AT ok");
    assert_eq!(ser, "[crashwalk] far ladder va=0x1cb497c: not run, AT ok par=0xff00000001cb4b80");
}

#[test]
fn the_ladder_line_sits_under_the_first_address_and_fits_the_pi_screen() {
    // Case d with a ladder on far: one line more, right after far's summary,
    // before the pc lines; the serial line likewise.
    let mut d = case_d();
    let mut hw = Script::new(&[F3], F3, OK);
    let far = d.probes[0].unwrap();
    let mut l = run_ladder(&mut hw, &proc08_photo(), USP).unwrap();
    l.va = far.hw.va;
    d.ladder = Some(l);
    let mut s = String::new();
    write_screen(&mut s, &d).unwrap();
    let lines: Vec<&str> = s.lines().collect();
    assert_eq!(lines.len(), 2 + 2 * 7 + 1);
    assert!(lines[8].starts_with("  => model=invalid"), "{}", lines[8]);
    assert!(lines[9].starts_with("  ladder: L1 F3 "), "{}", lines[9]);
    assert!(lines[10].starts_with("pc 1a1b1b0 s1:"), "{}", lines[10]);
    for l in &lines {
        // 240 columns at 1920x1080; the ladder line with every rung and the
        // widest results stays far below.
        assert!(l.len() <= if l.contains("ladder:") { 107 } else { 80 }, "{} columns: {l}", l.len());
        assert!(l.bytes().all(|b| (0x20..0x7f).contains(&b)), "printable ASCII only: {l:?}");
    }
    // The other lines are those without a ladder.
    let mut plain = String::new();
    d.ladder = None;
    write_screen(&mut plain, &d).unwrap();
    let without: Vec<&str> = lines.iter().copied().filter(|l| !l.contains("ladder:")).collect();
    assert_eq!(without, plain.lines().collect::<Vec<_>>());
    let mut ser = String::new();
    d.ladder = Some(l);
    write_serial(&mut ser, &d).unwrap();
    let sl: Vec<&str> = ser.lines().collect();
    assert!(sl[7].starts_with("[crashwalk] far summary:"), "{}", sl[7]);
    assert!(sl[8].starts_with("[crashwalk] far ladder va=0x1a2c000: usp="), "{}", sl[8]);
    assert!(sl[9].starts_with("[crashwalk] pc va="), "{}", sl[9]);
    // The widest screen line: every rung ran, every result before the last
    // a three-character `x..`, the last rung fixed it. 107 of the 240
    // columns at 1920x1080.
    let mut wide = l;
    wide.start = 0x81f;
    wide.sp_par = Some(0x81f);
    wide.priv_par = Some(0x81f);
    wide.after = [Some(0x81f), Some(0x81f), Some(0x81f), Some(0x81f), Some(0x81f), Some(0x81f), Some(OK)];
    let (w, _) = lines_of(&wide);
    assert_eq!(
        w,
        "  ladder: L1 x0f sp x0f priv x0f far x0f dsb x0f dc x0f vaale1 x0f vaae1 x0f vmalle1 ok => fixed-by vmalle1"
    );
    assert_eq!(w.len(), 107);
}
