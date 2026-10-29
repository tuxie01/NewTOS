//! TASK/BIOS08 — BIOS handles 6 and up are the `Bconmap` table's.
//!
//! Spec: `docs/api/bios/{01,02,03,08}-*.md` (BIOS08 notes),
//! `docs/api/xbios/2c-bconmap.md`. EmuTOS runs the `Bconmap` branch before
//! the vector tables in all four calls (`bios/bios.c:923-928`, `:967-972`,
//! `:1000-1005`, `:1152-1157`): `map_index = handle - 6` in a `WORD`,
//! `>= maptabsize` → 0, `>= 0` → `maptab[map_index]`'s routine. NewTOS has
//! EmuTOS' one-entry table (XBIOS05): entry 0 is the AUX port, so handle 6
//! answers as handle 1 — through the same device entry, a driver in slot 1
//! included — and every handle from 7 up answers 0.
//!
//! | handle | `Bconstat` | `Bconin` | `Bcostat` | `Bconout` |
//! |---|---|---|---|---|
//! | 6 | as 1 | as 1 (the trap entry, gives way like 1) | as 1 (no 3/4 swap) | as 1 |
//! | 7, 8, 9, … | 0 | 0, device not called | 0 | 0, byte not sent |
//!
//! Measured against the oracle: `bios-r1` 69/70, `bios-core-r1` 65/66 and the
//! audit probe's B04/B05, `measurements/measurement-2026-10-30-bios08/`.

use std::sync::atomic::{AtomicI32, AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard};

use tos_api::bios::chardev::{self, install_device, CharDevice, DEV_AUX};
use tos_api::host::{install_host, Host};
use tos_api::trap::{TrapArgs, TrapClass};
use tos_api::xbios::kbdvecs::take_restart;

struct NullHost;

impl Host for NullHost {
    fn console_write_byte(&self, _byte: u8) {}
}

static HOST: NullHost = NullHost;
static HOST_REF: &dyn Host = &HOST;

static LOCK: Mutex<()> = Mutex::new(());

fn setup() -> MutexGuard<'static, ()> {
    let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    if tos_api::host::host().is_none() {
        install_host(&HOST_REF);
    }
    let _ = take_restart();
    g
}

/// A port that records every call, with answers distinct from every
/// built-in entry: status `-1`, input `0x1234`, output status `-1`, output
/// `7`. Its trap-entry read gives way `give_way` times first.
struct RecordingPort {
    calls: AtomicU32,
    restartable_reads: AtomicU32,
    give_way: AtomicU32,
    last_out: AtomicI32,
}

impl RecordingPort {
    const fn new() -> RecordingPort {
        RecordingPort {
            calls: AtomicU32::new(0),
            restartable_reads: AtomicU32::new(0),
            give_way: AtomicU32::new(0),
            last_out: AtomicI32::new(-1),
        }
    }
    fn reset(&self) {
        self.calls.store(0, Ordering::SeqCst);
        self.restartable_reads.store(0, Ordering::SeqCst);
        self.give_way.store(0, Ordering::SeqCst);
        self.last_out.store(-1, Ordering::SeqCst);
    }
    fn calls(&self) -> u32 {
        self.calls.load(Ordering::SeqCst)
    }
}

impl CharDevice for RecordingPort {
    fn bconstat(&self) -> i32 {
        self.calls.fetch_add(1, Ordering::SeqCst);
        -1
    }
    fn bconin(&self) -> i32 {
        self.calls.fetch_add(1, Ordering::SeqCst);
        0x1234
    }
    fn bconin_restartable(&self) -> Option<i32> {
        self.restartable_reads.fetch_add(1, Ordering::SeqCst);
        if self.give_way.load(Ordering::SeqCst) > 0 {
            self.give_way.fetch_sub(1, Ordering::SeqCst);
            self.calls.fetch_add(1, Ordering::SeqCst);
            return None;
        }
        Some(self.bconin())
    }
    fn bconout(&self, byte: u8) -> i32 {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.last_out.store(i32::from(byte), Ordering::SeqCst);
        7
    }
    fn bcostat(&self) -> i32 {
        self.calls.fetch_add(1, Ordering::SeqCst);
        -1
    }
}

static PORT: RecordingPort = RecordingPort::new();
static PORT_REF: &dyn CharDevice = &PORT;

/// `PORT` as device 1 (AUX) for the life of the guard.
struct AuxIs(&'static &'static dyn CharDevice);

impl AuxIs {
    fn port() -> AuxIs {
        PORT.reset();
        AuxIs(install_device(DEV_AUX, &PORT_REF).expect("DEV_AUX is a table slot"))
    }
}

impl Drop for AuxIs {
    fn drop(&mut self) {
        install_device(DEV_AUX, self.0);
    }
}

const BCONSTAT: u16 = 0x01;
const BCONIN: u16 = 0x02;
const BCONOUT: u16 = 0x03;
const BCOSTAT: u16 = 0x08;
const BCONMAP: u16 = 0x2C;

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

/// A BIOS call with `handle` passed as a program passes a `WORD`.
fn bios(func: u16, handle: i16, a1: u16) -> i64 {
    trap(TrapClass::Bios, func, handle as u16 as u64, u64::from(a1))
}

/// The four answers of a handle, `Bconout` with `'A'`.
fn answers(handle: i16) -> [i64; 4] {
    [
        bios(BCONSTAT, handle, 0),
        bios(BCONIN, handle, 0),
        bios(BCOSTAT, handle, 0),
        bios(BCONOUT, handle, u16::from(b'A')),
    ]
}

/// The handles past the one-entry table: 7..9 (the oracle's further ports),
/// the far end of the `WORD` range and the negative handles (vector-table
/// guard; -32768 wraps in EmuTOS' `WORD` subtraction to a large index).
const PAST_THE_TABLE: [i16; 9] = [7, 8, 9, 10, 11, 0x7FFF, -1, -2, -32768];

#[test]
fn handle_6_answers_as_handle_1_with_the_built_in_aux() {
    let _g = setup();
    // The no-cable AUX entry (rpi4 profile): Bconstat 0, Bconin 0 at once,
    // Bcostat -1, Bconout 1 (docs/api/gemdos/file/3f-fread.md item 1a).
    assert_eq!(answers(1), [0, 0, -1, 1], "handle 1, the AUX entry");
    assert_eq!(answers(6), answers(1), "handle 6 = maptab[0] = AUX");
    assert!(!take_restart(), "the no-cable read does not wait");
}

#[test]
fn handle_6_reaches_the_driver_in_slot_1() {
    let _g = setup();
    let _aux = AuxIs::port();
    for h in [1i16, 6] {
        PORT.reset();
        assert_eq!(bios(BCONSTAT, h, 0), -1, "Bconstat({h})");
        assert_eq!(bios(BCONIN, h, 0), 0x1234, "Bconin({h})");
        assert_eq!(bios(BCOSTAT, h, 0), -1, "Bcostat({h})");
        assert_eq!(bios(BCONOUT, h, 0x5A), 7, "Bconout({h})");
        assert_eq!(PORT.last_out.load(Ordering::SeqCst), 0x5A, "Bconout({h}) byte");
        assert_eq!(PORT.calls(), 4, "handle {h}: every call reached the AUX entry");
        assert_eq!(
            PORT.restartable_reads.load(Ordering::SeqCst),
            1,
            "Bconin({h}) uses the trap entry (the wait that gives way)"
        );
    }
}

#[test]
fn bconin_6_gives_way_like_bconin_1() {
    let _g = setup();
    let _aux = AuxIs::port();
    for h in [1i16, 6] {
        PORT.reset();
        PORT.give_way.store(1, Ordering::SeqCst);
        assert_eq!(bios(BCONIN, h, 0), 0, "Bconin({h}): the placeholder");
        assert!(take_restart(), "Bconin({h}): a read that gave way requests the restart");
        assert_eq!(bios(BCONIN, h, 0), 0x1234, "Bconin({h}): the re-executed read");
        assert!(!take_restart(), "Bconin({h}): completes");
    }
}

#[test]
fn bcostat_6_is_not_swapped() {
    let _g = setup();
    // The 3/4 swap is a vector-table matter (bios/chardev.c:38-42); a Bconmap
    // handle reads its port's own status (maptab[0].Bcostat = bcostat1).
    let _aux = AuxIs::port();
    assert_eq!(bios(BCOSTAT, 6, 0), -1);
    assert_eq!(PORT.calls(), 1, "Bcostat(6) read the AUX entry");
}

#[test]
fn handles_past_the_table_answer_0_and_reach_no_device() {
    let _g = setup();
    let _aux = AuxIs::port();
    for h in PAST_THE_TABLE {
        PORT.reset();
        assert_eq!(answers(h), [0, 0, 0, 0], "handle {h}");
        assert_eq!(PORT.calls(), 0, "handle {h}: no device called");
        assert_eq!(PORT.restartable_reads.load(Ordering::SeqCst), 0, "handle {h}: no read");
        assert!(!take_restart(), "handle {h}: no restart");
    }
}

#[test]
fn the_table_ends_at_5() {
    let _g = setup();
    // Handles 6 and up are the Bconmap table's: no table slot to install into.
    for dev in [6u16, 7, 8, 9] {
        assert!(install_device(dev, &PORT_REF).is_none(), "install_device({dev})");
    }
    assert!(chardev::device(5).is_some());
    assert!(chardev::device(6).is_some(), "6 is the AUX entry");
    assert!(chardev::device(7).is_none());
}

#[test]
fn handle_6_stays_aux_after_bconmap_6() {
    let _g = setup();
    let _aux = AuxIs::port();
    // The only valid switch with one entry (XBIOS05): 6 stays mapped.
    assert_eq!(trap(TrapClass::Xbios, BCONMAP, 6, 0), 6);
    assert_eq!(trap(TrapClass::Xbios, BCONMAP, 0xFFFF, 0), 6, "Bconmap(-1)");
    assert_eq!(bios(BCONOUT, 6, 0x33), 7);
    assert_eq!(PORT.last_out.load(Ordering::SeqCst), 0x33);
    // Bconmap(7) is refused (0) and changes nothing: 7 stays past the table.
    assert_eq!(trap(TrapClass::Xbios, BCONMAP, 7, 0), 0);
    assert_eq!(bios(BCONOUT, 7, 0x34), 0);
    assert_eq!(PORT.last_out.load(Ordering::SeqCst), 0x33);
}
