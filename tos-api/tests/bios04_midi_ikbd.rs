//! TASK/BIOS04 — BIOS devices 3 (MIDI) and 4 (IKBD) answer as on the
//! reference machine, whose MIDI and IKBD ACIAs always exist (user decision
//! 2026-10-29).
//!
//! Spec: `docs/api/bios/{01,02,03,08}-*.md` (BIOS04 notes),
//! `docs/api/gemdos/file/3f-fread.md` "NewTOS implementation notes" item 1,
//! `docs/api/xbios/0c-midiws.md`. EmuTOS: `bios/chardev.c:30-45` (the slots,
//! `Bcostat` 3/4 swapped), `bios/midi.c:22-95` (`bconstat3`, `bconin3`,
//! `bcostat3`, `bconout3`), `bios/ikbd.c:916-937` (`bcostat4`, `bconout4`).
//!
//! | | 3 (MIDI) | 4 (IKBD) |
//! |---|---|---|
//! | `Bconstat` | `-1` while the `Iorec(2)` ring holds a byte, else 0 | 0 |
//! | `Bconin` | waits for a byte in that ring | 0 |
//! | `Bcostat` | -1 (IKBD entry) | -1 (MIDI entry) |
//! | `Bconout` | 1, byte discarded | 1, byte discarded |
//!
//! Nothing in the kernel fills the MIDI ring, so a byte in it is one a
//! program put there through the record `Iorec(2)` returns — what this test
//! does, as the EL0 store lands on the same page. The empty-ring wait itself
//! cannot end on the host; its give-way to a `kbdvec` hook is pinned in
//! `tests/xbios_kbdvecs.rs` (`midi_wait_gives_way_without_side_effects`), its
//! endless wait is measured against the oracle with
//! `tosdump/probes/mdprobe.c`.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard};

use tos_api::bdos::process::mmu::{self, IOREC_PAGE_VA};
use tos_api::host::{install_host, Host};
use tos_api::trap::{TrapArgs, TrapClass};
use tos_api::xbios::iorec::{OFF_IBUF, OFF_IBUFHD, OFF_IBUFSIZ, OFF_IBUFTL};

/// Counts the input pumps of a wait and records console output.
struct PumpHost;

static PUMPS: AtomicU32 = AtomicU32::new(0);
static CON_OUT: Mutex<Vec<u8>> = Mutex::new(Vec::new());

impl Host for PumpHost {
    fn console_write_byte(&self, byte: u8) {
        CON_OUT.lock().unwrap().push(byte);
    }
    fn pump_input(&self) {
        PUMPS.fetch_add(1, Ordering::SeqCst);
    }
}

static HOST: PumpHost = PumpHost;
static HOST_REF: &dyn Host = &HOST;

/// The host is process-wide and stays installed once installed; the tests
/// run one at a time.
static LOCK: Mutex<()> = Mutex::new(());

/// Take the lock; the first test to run checks the boot gate — before a
/// host is installed `Bconin(3)` returns 0 at once (EmuTOS'
/// `CHARDEV_AVAILABLE`, `bios/bios.c:964-965`) — and installs the host.
fn setup() -> MutexGuard<'static, ()> {
    let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    if tos_api::host::host().is_none() {
        assert_eq!(bios(BCONIN, 3, 0), 0, "Bconin(3) before the host: boot gate");
        install_host(&HOST_REF);
    }
    CON_OUT.lock().unwrap().clear();
    g
}

const BCONSTAT: u16 = 0x01;
const BCONIN: u16 = 0x02;
const BCONOUT: u16 = 0x03;
const BCOSTAT: u16 = 0x08;
const MIDIWS: u16 = 0x0C;
const IOREC: u16 = 0x0E;

fn trap(class: TrapClass, func: u16, a0: u64, a1: u64) -> i64 {
    let args = TrapArgs {
        regs: [a0, a1, 0, 0, 0, 0, 0, 0],
        func: func as u64,
        svc_imm: class as u16,
        space: tos_api::uaccess::Space::Kernel,
    };
    match class {
        TrapClass::Bios => tos_api::bios::dispatch(&args),
        TrapClass::Xbios => tos_api::xbios::dispatch(&args),
        _ => unreachable!("only BIOS and XBIOS calls here"),
    }
}

fn bios(func: u16, a0: u64, a1: u64) -> i64 {
    trap(TrapClass::Bios, func, a0, a1)
}

/// The kernel alias of a user address in the IOREC page.
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

/// Put one byte into the MIDI ring the way a program can: through the record
/// `Iorec(2)` returns, advance `ibuftl` by one (wrapping at `ibufsiz`) and
/// store the byte at `ibuf[ibuftl]` — the producer side of EmuTOS' ring.
fn put_midi_byte(b: u8) {
    let rec = trap(TrapClass::Xbios, IOREC, 2, 0) as u64;
    // SAFETY: an 8-aligned offset inside the page.
    let ibuf = unsafe { (user_cell(rec + OFF_IBUF as u64) as *const u64).read_volatile() };
    let size = rd16(rec + OFF_IBUFSIZ as u64);
    let mut tail = rd16(rec + OFF_IBUFTL as u64) + 1;
    if tail >= size {
        tail = 0;
    }
    // SAFETY: `ibuf + tail` is inside the MIDI ring of the page.
    unsafe { user_cell(ibuf + tail as u64).write_volatile(b) };
    wr16(rec + OFF_IBUFTL as u64, tail);
}

fn midi_indices() -> (i16, i16) {
    let rec = trap(TrapClass::Xbios, IOREC, 2, 0) as u64;
    (rd16(rec + OFF_IBUFHD as u64), rd16(rec + OFF_IBUFTL as u64))
}

/// The profile of both devices (the table above, without the MIDI wait).
#[test]
fn midi_and_ikbd_profile() {
    let _g = setup();

    // IKBD (4): input is char_dummy, output ready and accepted.
    assert_eq!(bios(BCONSTAT, 4, 0), 0, "Bconstat(4) char_dummy");
    assert_eq!(bios(BCONIN, 4, 0), 0, "Bconin(4) char_dummy, no wait");
    assert_eq!(bios(BCONOUT, 4, 0x11), 1, "Bconout(4) bconout4 -> 1");
    // MIDI (3): no input waiting, output ready and accepted.
    assert_eq!(bios(BCONSTAT, 3, 0), 0, "Bconstat(3) empty ring");
    assert_eq!(bios(BCONOUT, 3, 0xFE), 1, "Bconout(3) -> 1");
    // Bcostat, swapped slots: both ready.
    assert_eq!(bios(BCOSTAT, 3, 0), -1, "Bcostat(3) = bcostat4");
    assert_eq!(bios(BCOSTAT, 4, 0), -1, "Bcostat(4) = bcostat3");
    // The full WORD reaches Bconout; the low byte is what would be sent.
    assert_eq!(bios(BCONOUT, 3, 0x1FE), 1, "Bconout(3, 0x1FE)");
    assert_eq!(bios(BCONOUT, 4, 0xFFFF), 1, "Bconout(4, 0xFFFF)");
    assert!(CON_OUT.lock().unwrap().is_empty(), "nothing reached the console");
    assert_eq!(bios(BCONSTAT, 3, 0), 0, "output does not feed the input ring");
}

/// `Bconin(3)` reads the `Iorec(2)` ring as `bconin3` does: the byte
/// zero-extended, in order, `head` following; `Bconstat(3)` turns -1 while a
/// byte waits. With a byte already there it returns without pumping.
#[test]
fn bconin3_reads_the_midi_ring() {
    let _g = setup();
    assert_eq!(bios(BCONSTAT, 3, 0), 0);
    put_midi_byte(0xFE);
    put_midi_byte(0x90);
    put_midi_byte(0x3C);
    assert_eq!(bios(BCONSTAT, 3, 0), -1, "Bconstat(3) with bytes waiting");
    PUMPS.store(0, Ordering::SeqCst);
    assert_eq!(bios(BCONIN, 3, 0), 0xFE, "zero-extended, not -2");
    assert_eq!(bios(BCONIN, 3, 0), 0x90);
    assert_eq!(bios(BCONSTAT, 3, 0), -1);
    assert_eq!(bios(BCONIN, 3, 0), 0x3C);
    assert_eq!(PUMPS.load(Ordering::SeqCst), 0, "no wait, no pump");
    assert_eq!(bios(BCONSTAT, 3, 0), 0, "ring empty again");
    let (head, tail) = midi_indices();
    assert_eq!(head, tail, "head caught up with tail");
    assert!(!tos_api::xbios::kbdvecs::take_restart(), "no restart requested");

    // Around the end of the 128-byte ring.
    for i in 0..200u32 {
        put_midi_byte(i as u8);
        assert_eq!(bios(BCONIN, 3, 0), i64::from(i as u8), "byte {i}");
    }
    assert_eq!(bios(BCONSTAT, 3, 0), 0);
}

/// `Midiws` still sends `cnt + 1` bytes through the MIDI entry; each is
/// accepted and dropped, the call is `void` (0) and nothing reaches the
/// MIDI input ring or the console.
#[test]
fn midiws_through_the_midi_entry() {
    let _g = setup();
    let data = b"\x90\x3c\x40";
    assert_eq!(trap(TrapClass::Xbios, MIDIWS, 2, data.as_ptr() as u64), 0);
    assert_eq!(bios(BCONSTAT, 3, 0), 0);
    assert!(CON_OUT.lock().unwrap().is_empty());
}
