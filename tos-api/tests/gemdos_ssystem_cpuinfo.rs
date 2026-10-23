//! **ACC06 part 1** — `Ssystem(2000)` / `Ssystem(2001)` at the GEMDOS trap
//! boundary: the two NewTOS-own subfunctions that carry the machine's ARM
//! clock and die temperature out to an EL0 program.
//!
//! Spec: `docs/api/gemdos/154-ssystem.md` §"NewTOS-own subfunctions (2000+)".
//! The hardware itself (the BCM2711 mailbox property channel) is `board-rpi4`
//! only and cannot run here; what *can* be pinned here — and is what a program
//! actually depends on — is the trap-level contract above the
//! [`Host`](tos_api::host::Host) boundary:
//!
//! 1. with `arg1 == 0` the value comes back in the call's own LONG;
//! 2. with `arg1 != 0` it is written to the caller's LONG and the call is
//!    `E_OK`;
//! 3. a board whose host has no such sensor answers `ERROR` and writes
//!    **nothing** — the honest path the ticket insists on, in place of an
//!    invented number;
//! 4. `arg2` is reserved and must be 0, so a program written against a later,
//!    wider version of the call learns it is talking to an older kernel;
//! 5. the numbers stay 2000/2001, the free range below FreeMiNT's build-gated
//!    3000 and above its reserved 1000–1999 debug block.

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Mutex, Once};

use tos_api::host::Host;
use tos_api::trap::{TrapArgs, TrapClass};

/// Sentinel for "this board has no such sensor" in the two cells below.
/// Outside the range of both `u32` and `i32`, so it can never collide with a
/// real reading.
const ABSENT: i64 = i64::MIN;

static CLOCK_HZ: AtomicI64 = AtomicI64::new(ABSENT);
static TEMP_MDEG: AtomicI64 = AtomicI64::new(ABSENT);

/// The cells are process-wide; keep the cases in this binary from racing.
static RIG: Mutex<()> = Mutex::new(());

/// A host that stands in for a board's sensor wiring. `board-rpi4`'s real
/// implementation asks the VideoCore firmware and returns `None` when it does
/// not answer; here the two cells decide, so both the present and the absent
/// case can be driven in one process.
struct SensorHost;

impl Host for SensorHost {
    fn console_write_byte(&self, _byte: u8) {}

    fn cpu_clock_hz(&self) -> Option<u32> {
        match CLOCK_HZ.load(Ordering::Relaxed) {
            ABSENT => None,
            v => Some(v as u32),
        }
    }

    fn cpu_temperature_mdeg(&self) -> Option<i32> {
        match TEMP_MDEG.load(Ordering::Relaxed) {
            ABSENT => None,
            v => Some(v as i32),
        }
    }
}

static HOST: Once = Once::new();

/// Install [`SensorHost`] once (the GEMDOS dispatcher refuses every call with
/// `EUNDEV` until a host is installed) and set the two cells.
fn rig(clock: i64, temp: i64) {
    HOST.call_once(|| {
        let slot: &'static &'static dyn Host =
            Box::leak(Box::new(&SensorHost as &'static dyn Host));
        tos_api::host::install_host(slot);
    });
    CLOCK_HZ.store(clock, Ordering::Relaxed);
    TEMP_MDEG.store(temp, Ordering::Relaxed);
}

/// `Ssystem` — GEMDOS 0x154 (`docs/api/gemdos/154-ssystem.md`).
const SSYSTEM: u16 = 0x154;
const S_NEWTOS_CPUCLOCK: u64 = 2000;
const S_NEWTOS_CPUTEMP: u64 = 2001;

const E_OK: i32 = 0;
const ERROR: i32 = -1;
const EINVAL: i32 = -25;
const EINVFN: i32 = -32;

/// A Pi 4 at its stock top speed, and a plausible warm die.
const PI4_CLOCK_HZ: i64 = 1_500_000_000;
const WARM_MDEG: i64 = 47_200;

/// Issue a GEMDOS call the way an EL0 program does: function id separate,
/// arguments in `x0..`.
fn gemdos(func: u16, a0: u64, a1: u64, a2: u64) -> i32 {
    let args = TrapArgs {
        regs: [a0, a1, a2, 0, 0, 0, 0, 0],
        func: func as u64,
        svc_imm: TrapClass::Gemdos as u16,
        space: tos_api::uaccess::Space::Kernel,
    };
    tos_api::gemdos::dispatch(&args)
}

#[test]
fn both_values_come_back_in_the_calls_own_long() {
    let _lock = RIG.lock().unwrap_or_else(|e| e.into_inner());
    rig(PI4_CLOCK_HZ, WARM_MDEG);

    assert_eq!(
        gemdos(SSYSTEM, S_NEWTOS_CPUCLOCK, 0, 0),
        PI4_CLOCK_HZ as i32,
        "arg1 == 0: the ARM clock in Hz is the return LONG"
    );
    assert_eq!(
        gemdos(SSYSTEM, S_NEWTOS_CPUTEMP, 0, 0),
        WARM_MDEG as i32,
        "arg1 == 0: the temperature in 1/1000 C is the return LONG"
    );
}

#[test]
fn the_pointer_form_writes_the_long_and_returns_e_ok() {
    let _lock = RIG.lock().unwrap_or_else(|e| e.into_inner());
    rig(PI4_CLOCK_HZ, WARM_MDEG);

    let mut hz: u32 = 0;
    assert_eq!(
        gemdos(SSYSTEM, S_NEWTOS_CPUCLOCK, &mut hz as *mut u32 as u64, 0),
        E_OK,
        "arg1 != 0: the status is E_OK, the value goes to the caller's LONG"
    );
    assert_eq!(hz, PI4_CLOCK_HZ as u32);

    let mut mdeg: i32 = 0;
    assert_eq!(
        gemdos(SSYSTEM, S_NEWTOS_CPUTEMP, &mut mdeg as *mut i32 as u64, 0),
        E_OK
    );
    assert_eq!(mdeg, WARM_MDEG as i32);
}

#[test]
fn a_board_without_the_sensor_answers_error_and_writes_nothing() {
    let _lock = RIG.lock().unwrap_or_else(|e| e.into_inner());
    rig(ABSENT, ABSENT);

    assert_eq!(
        gemdos(SSYSTEM, S_NEWTOS_CPUCLOCK, 0, 0),
        ERROR,
        "no mailbox, no clock: ERROR, never an invented number"
    );
    assert_eq!(gemdos(SSYSTEM, S_NEWTOS_CPUTEMP, 0, 0), ERROR);

    // The pointer form must leave the caller's LONG untouched, so a program
    // that ignores the status cannot mistake a stale buffer for a reading.
    let mut cell: u32 = 0xDEAD_BEEF;
    let p = &mut cell as *mut u32 as u64;
    assert_eq!(gemdos(SSYSTEM, S_NEWTOS_CPUCLOCK, p, 0), ERROR);
    assert_eq!(gemdos(SSYSTEM, S_NEWTOS_CPUTEMP, p, 0), ERROR);
    assert_eq!(cell, 0xDEAD_BEEF, "nothing was written");
}

#[test]
fn arg2_is_reserved_and_a_non_zero_value_is_refused() {
    let _lock = RIG.lock().unwrap_or_else(|e| e.into_inner());
    rig(PI4_CLOCK_HZ, WARM_MDEG);

    // Refused *before* the sensor is read: a future selector must never be
    // answered with the wrong quantity.
    assert_eq!(gemdos(SSYSTEM, S_NEWTOS_CPUCLOCK, 0, 1), EINVAL);
    assert_eq!(gemdos(SSYSTEM, S_NEWTOS_CPUTEMP, 0, 1), EINVAL);

    let mut cell: u32 = 0xDEAD_BEEF;
    let p = &mut cell as *mut u32 as u64;
    assert_eq!(gemdos(SSYSTEM, S_NEWTOS_CPUCLOCK, p, 7), EINVAL);
    assert_eq!(cell, 0xDEAD_BEEF, "a refused call writes nothing");
}

#[test]
fn the_neighbouring_mode_numbers_are_still_unimplemented() {
    let _lock = RIG.lock().unwrap_or_else(|e| e.into_inner());
    rig(PI4_CLOCK_HZ, WARM_MDEG);

    // 1999 is the last number FreeMiNT reserves for its debug block
    // (`sys/mint/ssystem.h`, "reserved 1000 - 1999"); 2023 is the next free
    // NewTOS number (2002 is `S_NEWTOS_KBDVEC` since KEYS02, tested in
    // `xbios_kbdvecs.rs`; 2003-2019 are the SYSINFO01 modes and 2020 the
    // KEYOWNER input mode, tested in `gemdos_ssystem_sysinfo.rs`; 2021 is the
    // RST01 reset entry, tested in `bios_reset.rs`; 2022 is the USB snapshot,
    // tested in `gemdos_ssystem_usbsnap.rs`). Neither may answer with a
    // machine reading.
    for mode in [1999_u64, 2023, 2999] {
        assert_eq!(
            gemdos(SSYSTEM, mode, 0, 0),
            EINVFN,
            "mode {mode} is not implemented and must say so"
        );
    }

    // The feature probe still works with the new modes in place.
    assert_eq!(gemdos(SSYSTEM, u64::from(-1_i16 as u16), 0, 0), E_OK);
}
