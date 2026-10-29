//! XBIOS01 — `Iorec` (XBIOS 0x0E) and the IOREC page; BIOS11 with it.
//!
//! Spec: `docs/api/xbios/0e-iorec.md` (contract F1-F4, user decisions E1-E6),
//! EmuTOS `bios/xbios.c:336-348`, `bios/ikbd.c:157-173,236-283,898-909`.
//! The host here is the kernel's wiring of the console keyboard in miniature:
//! status, reads and pushes all go through the one IKBD ring in the shared
//! IOREC page (`tos_api::xbios::iorec`), so a write into the record — what a
//! program does through `Iorec(1)`'s pointer — acts on `Bconstat(2)` and
//! `Bconin(2)` exactly as it will in the kernel. The EL0 write itself and the
//! key injection are measured in QEMU and against the oracle
//! (`measurements/measurement-2026-10-29-xbios01/`).

use std::sync::atomic::Ordering;
use std::sync::{Mutex, MutexGuard, OnceLock};

use tos_api::bdos::process::mmu::{self, PageFlags, IOREC_PAGE_VA};
use tos_api::bdos::process::{self, State, PROCESS_TABLE};
use tos_api::host::Host;
use tos_api::trap::{TrapArgs, TrapClass};
use tos_api::xbios::iorec;
use tos_types::BasePage;

/// The console as the kernel host wires it: the IOREC ring and nothing else.
struct RingHost;

impl Host for RingHost {
    fn console_write_byte(&self, _byte: u8) {}
    fn console_try_read(&self) -> Option<u8> {
        loop {
            let e = iorec::pop_key()?;
            if e & 0xFF != 0 {
                return Some(e as u8);
            }
        }
    }
    fn console_try_read_key(&self) -> Option<u16> {
        iorec::pop_key().map(iorec::word_from_entry)
    }
    fn console_has_input(&self) -> bool {
        iorec::key_pending()
    }
    fn console_read_entry_blocking(&self) -> u32 {
        iorec::pop_key().expect("a blocking read on an empty ring: the test queued too few keys")
    }
    fn console_read_entry_restartable(&self) -> Option<u32> {
        Some(self.console_read_entry_blocking())
    }
}

static HOST: OnceLock<()> = OnceLock::new();
static LOCK: Mutex<()> = Mutex::new(());

fn setup() -> MutexGuard<'static, ()> {
    let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    HOST.get_or_init(|| {
        let h: &'static RingHost = Box::leak(Box::new(RingHost));
        let slot: &'static &'static dyn Host = Box::leak(Box::new(h as &'static dyn Host));
        tos_api::host::install_host(slot);
    });
    process::set_current(core::ptr::null_mut());
    while iorec::pop_key().is_some() {}
    g
}

fn xbios(func: u16, a0: u64) -> i64 {
    let args = TrapArgs {
        regs: [a0, 0, 0, 0, 0, 0, 0, 0],
        func: func as u64,
        svc_imm: TrapClass::Xbios as u16,
        space: tos_api::uaccess::Space::Kernel,
    };
    tos_api::xbios::dispatch(&args)
}

fn bios(func: u16, a0: u64) -> i64 {
    let args = TrapArgs {
        regs: [a0, 0, 0, 0, 0, 0, 0, 0],
        func: func as u64,
        svc_imm: TrapClass::Bios as u16,
        space: tos_api::uaccess::Space::Kernel,
    };
    tos_api::bios::dispatch(&args)
}

const IOREC: u16 = 0x0E;
const BCONSTAT: u16 = 0x01;
const BCONIN: u16 = 0x02;

/// The kernel alias of a user address in the IOREC page — what an EL0 store
/// through `Iorec`'s pointer lands on.
fn user_cell(va: u64) -> *mut u8 {
    assert!((IOREC_PAGE_VA..IOREC_PAGE_VA + 0x1000).contains(&va));
    // SAFETY: inside the static IOREC page.
    unsafe { mmu::iorec_page_kva().add((va - IOREC_PAGE_VA) as usize) }
}
fn rd16(va: u64) -> i16 {
    // SAFETY: an even offset inside the page.
    unsafe { (user_cell(va) as *const i16).read_volatile() }
}
fn wr16(va: u64, v: i16) {
    // SAFETY: as above.
    unsafe { (user_cell(va) as *mut i16).write_volatile(v) }
}
fn rd64(va: u64) -> u64 {
    // SAFETY: an 8-aligned offset inside the page.
    unsafe { (user_cell(va) as *const u64).read_volatile() }
}

/// A running process with its own page 0 (for `conterm`), as the loader
/// leaves it.
struct Proc {
    slot: usize,
    bp: *mut BasePage,
}

impl Proc {
    fn new(conterm: u8) -> Proc {
        let slot = process::allocate_slot().expect("a slot");
        let bp = Box::into_raw(Box::new([0u8; 256])) as *mut BasePage;
        let p = &PROCESS_TABLE[slot];
        p.basepage_kva.store(bp, Ordering::Release);
        // SAFETY: under the lock; the slot is not an installed root.
        unsafe { mmu::init_slot_tables(slot) }.unwrap();
        p.set_state(State::Running);
        process::set_current(bp);
        // SAFETY: under the lock; the slot's page 0.
        unsafe { mmu::page0_bytes(slot) }.unwrap()[0x484] = conterm;
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
fn iorec_returns_the_three_records_and_minus_one_for_any_other_device() {
    let _g = setup();
    let r0 = xbios(IOREC, 0);
    let r1 = xbios(IOREC, 1);
    let r2 = xbios(IOREC, 2);
    for r in [r0, r1, r2] {
        assert!(r > 0 && r % 2 == 0 && r < 0x8000_0000, "a pointer below 2 GiB, even: {r:#x}");
        assert!((IOREC_PAGE_VA as i64..IOREC_PAGE_VA as i64 + 0x1000).contains(&r));
    }
    assert!(r0 != r1 && r1 != r2 && r0 != r2);
    assert_eq!(r1, iorec::IKBD_IOREC_VA as i64, "the kbdvec hook's x1 (E6) is Iorec(1)");
    assert_eq!(xbios(IOREC, 1), r1, "stable (I08)");
    // The selector is a signed WORD: 3, -1, 0x7FFF, -32768 (I04-I07). Upper
    // register bits are not part of it.
    for d in [3u64, 0xFFFF, 0x7FFF, 0x8000, 0xFFFF_FFFF_FFFF_FFFF] {
        assert_eq!(xbios(IOREC, d), -1, "devno {d:#x}");
    }
    assert_eq!(xbios(IOREC, 0x1_0001), r1, "only the low WORD selects");
}

#[test]
fn the_records_hold_the_oracle_values() {
    let _g = setup();
    let r0 = xbios(IOREC, 0) as u64;
    let r1 = xbios(IOREC, 1) as u64;
    let r2 = xbios(IOREC, 2) as u64;
    // _IOREC (LP64, F1): ibuf @0, ibufsiz @8, ibufhd @10, ibuftl @12,
    // ibuflow @14, ibufhi @16.
    let rec = |r: u64| (rd64(r), rd16(r + 8), rd16(r + 10), rd16(r + 12), rd16(r + 14), rd16(r + 16));
    let (buf1, size1, hd1, tl1, lo1, hi1) = rec(r1);
    assert_eq!((size1, hd1 == tl1, lo1, hi1), (256, true, 64, 192), "IKBD (I02b)");
    assert!((IOREC_PAGE_VA..IOREC_PAGE_VA + 0x1000 - 256).contains(&buf1));
    let (_, size2, hd2, tl2, lo2, hi2) = rec(r2);
    assert_eq!((size2, hd2, tl2, lo2, hi2), (128, 0, 0, 32, 96), "MIDI (I03b)");
    // EXT_IOREC: in @0, out @24, then baudrate, flowctrl, ucr, datamask @48.
    for io in [r0, r0 + 24] {
        let (_, size, hd, tl, lo, hi) = rec(io);
        assert_eq!((size, hd, tl, lo, hi), (256, 0, 0, 64, 192), "RS-232 (I01b/I01c)");
    }
    // SAFETY: inside the page.
    let line = unsafe { std::slice::from_raw_parts(user_cell(r0 + 48), 4) };
    assert_eq!(line, [1, 0, 0x88, 0xFF], "baud 1, flow 0, ucr 0x88, datamask 0xff");
}

#[test]
fn every_process_maps_the_iorec_page_read_write_never_executable() {
    let _g = setup();
    let p = Proc::new(7);
    let (_, d) = mmu::translate(p.slot, IOREC_PAGE_VA + 0x48).expect("the IOREC page is mapped");
    let attrs = |desc: u64| desc & (0xFFF | (0b11 << 53));
    assert_eq!(attrs(d), attrs(mmu::page_bits(PageFlags::UserRwNx)));
    assert_eq!(
        mmu::private_page_alias(p.slot, IOREC_PAGE_VA + 0x48),
        Some(mmu::iorec_page_kva() as u64 + 0x48),
        "the kernel's alias of a user pointer into the page is the shared page"
    );
    assert!(mmu::in_private_page(IOREC_PAGE_VA));
    // No VA = PA publication may shadow it.
    // SAFETY: under the lock; refused before any table is touched.
    let r = unsafe { mmu::map_shared(IOREC_PAGE_VA, mmu::PAGE_SIZE, PageFlags::UserRoNx, false) };
    assert_eq!(r, Err(mmu::MapError::VaOutOfRange));
}

#[test]
fn the_flush_head_eq_tail_empties_the_queue_bconstat_sees() {
    let _g = setup();
    let io = xbios(IOREC, 1) as u64;
    for w in [0x1E61u16, 0x3062, 0x2E63] {
        assert!(iorec::push_key(iorec::entry_from_word(w, 0)));
    }
    assert_eq!(bios(BCONSTAT, 2), -1, "three keys waiting");
    assert_eq!(rd16(io + 12) - rd16(io + 10), 12, "tail - head = 3 entries of 4 bytes");
    // io->ibufhd = io->ibuftl (the program's flush, XCIOFL).
    wr16(io + 10, rd16(io + 12));
    assert_eq!(bios(BCONSTAT, 2), 0, "bconstat2 tests head == tail (bios/ikbd.c:242)");
    // A key after the flush is the next one read.
    assert!(iorec::push_key(iorec::entry_from_word(0x1C0D, 0)));
    assert_eq!(bios(BCONIN, 2), 0x001C_000D);
}

#[test]
fn bconin_keeps_the_shift_state_of_the_key_press_with_conterm_bit_3() {
    // BIOS11 / K16: Shift+b pressed, Shift released, then conterm |= 8 and
    // Bconin(2): EmuTOS 0x02300042 — the shift state is the one pushed with
    // the key (bios/ikbd.c:898-909), not the one at the read.
    let _g = setup();
    let _p = Proc::new(7 | 8);
    assert!(iorec::push_key(iorec::entry_from_word(0x3042, 0x02)));
    assert_eq!(bios(BCONIN, 2), 0x0230_0042);
    // K01-K05: plain keys carry shift state 0 (or the modifiers they were
    // typed with).
    assert!(iorec::push_key(iorec::entry_from_word(0x1E61, 0)));
    assert_eq!(bios(BCONIN, 2), 0x001E_0061);
    assert!(iorec::push_key(iorec::entry_from_word(0x2E03, 0x04)));
    assert_eq!(bios(BCONIN, 2), 0x042E_0003, "Ctrl+c with conterm & 8");
}

#[test]
fn without_conterm_bit_3_the_shift_byte_is_cleared() {
    let _g = setup();
    {
        let _p = Proc::new(7);
        assert!(iorec::push_key(iorec::entry_from_word(0x3042, 0x02)));
        assert_eq!(bios(BCONIN, 2), 0x0030_0042, "value &= 0x00ffffff (bios/ikbd.c:279-280)");
    }
    // No process (boot, EmuCON): the boot conterm 7, bit 3 clear.
    assert!(iorec::push_key(iorec::entry_from_word(0x3042, 0x02)));
    assert_eq!(bios(BCONIN, 2), 0x0030_0042);
}

#[test]
fn a_program_cannot_point_the_kernel_outside_its_ring() {
    let _g = setup();
    let io = xbios(IOREC, 1) as u64;
    assert!(iorec::push_key(1));
    // Garbage indices: the ring is emptied, never read out of bounds.
    wr16(io + 12, 0x7FF0);
    assert_eq!(bios(BCONSTAT, 2), 0);
    assert_eq!((rd16(io + 10), rd16(io + 12)), (0, 0));
    // A replaced buffer and size (E4 (a)): the kernel's own come back.
    let buf = rd64(io);
    // SAFETY: an 8-aligned offset inside the page.
    unsafe { (user_cell(io) as *mut u64).write_volatile(0x0040_0000) };
    wr16(io + 8, 4096);
    iorec::check_records();
    assert_eq!((rd64(io), rd16(io + 8)), (buf, 256));
    // Both repairs are reported, once (`kernel::console::report_iorec_repairs`
    // prints them).
    let w = iorec::take_warnings(None);
    assert_eq!(w & (iorec::FAULT_BUF | iorec::FAULT_INDEX), iorec::FAULT_BUF | iorec::FAULT_INDEX);
    assert!(iorec::push_key(2));
    assert_eq!(bios(BCONIN, 2), 2, "the ring works on");
}
