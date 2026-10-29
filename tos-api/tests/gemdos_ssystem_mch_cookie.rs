//! **ARCH06** — the `_MCH` cookie NewTOS seeds at boot, read the way a program
//! reads it: `Ssystem(S_GETCOOKIE, '_MCH', arg2)` through the GEMDOS
//! dispatcher, in both of FreeMiNT's forms (`docs/api/gemdos/154-ssystem.md`,
//! `S_GETCOOKIE`).
//!
//! The value is EmuTOS' `MCH_NOHARD`, `0xFFFFFFFF` — "no ST-compatible
//! hardware" (`references/emutos/include/cookie.h:50`, set by `setvalue_mch`
//! without Atari hardware, `references/emutos/bios/machine.c:400-401`) — per
//! the ADR-012 amendment of 2026-10-28 (`docs/MEMORY.md`). It replaces the
//! provisional `0x00050000`, the registered ARAnyM id
//! (`references/emutos/include/cookie.h:57`).
//!
//! The two forms answer it differently, and that is the point of pinning both:
//!
//! 1. **value form** (`arg2 == 0`): the value comes back in `x0` as the
//!    GEMDOS `LONG`, i.e. `-1` — the same number as the call's "not found"
//!    (`EERROR`, FreeMiNT `sys/cookie.c:508-509`). A caller cannot tell the
//!    two apart from this form alone; FreeMiNT has the same ambiguity for any
//!    cookie whose value is `-1`.
//! 2. **pointer form** (`arg2 != 0`): `E_OK` and the four bytes `0xFFFFFFFF`
//!    written to `*arg2`; an absent tag returns `EERROR` and leaves the buffer
//!    untouched. This is the unambiguous read.

use std::sync::{Mutex, Once};

use tos_api::host::Host;
use tos_api::trap::{TrapArgs, TrapClass};

/// The cookie jar is a process-wide static; keep the cases in this binary from
/// racing each other on it.
static JAR: Mutex<()> = Mutex::new(());

/// The GEMDOS dispatcher refuses every call until a host is installed;
/// `Ssystem`'s cookie subfunctions need nothing else of it.
struct NullHost;

impl Host for NullHost {
    fn console_write_byte(&self, _byte: u8) {}
}

static HOST: Once = Once::new();

/// Install [`NullHost`] once and seed the boot cookie set, as
/// `kernel/src/lib.rs` does.
fn rig() {
    HOST.call_once(|| {
        let slot: &'static &'static dyn Host = Box::leak(Box::new(&NullHost as &'static dyn Host));
        tos_api::host::install_host(slot);
    });
    tos_api::cookie::seed_defaults(0x0000_0100);
}

/// `Ssystem` — GEMDOS 0x154.
const SSYSTEM: u16 = 0x154;
/// `S_GETCOOKIE` — `Ssystem` subfunction 8 (FreeMiNT `sys/mint/ssystem.h`).
const S_GETCOOKIE: u64 = 8;

/// `'_MCH'` big-endian, as `tos_api::cookie::MCH` registers it.
const MCH_TAG: u32 = u32::from_be_bytes(*b"_MCH");
/// A tag that is deliberately not in the jar.
const ABSENT_TAG: u32 = u32::from_be_bytes(*b"ZZZZ");

/// FreeMiNT `EERROR` — `S_GETCOOKIE`'s answer for an absent tag.
const ERROR: i32 = -1;
const E_OK: i32 = 0;

/// Issue a GEMDOS call: function id separate, arguments in `x0..`; an
/// in-kernel caller, so a pointer argument is a plain host address.
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
fn the_seeded_mch_is_emutos_mch_nohard() {
    assert_eq!(tos_api::cookie::MCH_NOHARD, 0xFFFF_FFFF, "references/emutos/include/cookie.h:50");
}

#[test]
fn the_pointer_form_reads_mch_as_ffffffff_with_e_ok() {
    let _lock = JAR.lock().unwrap_or_else(|e| e.into_inner());
    rig();

    let mut v: u32 = 0x5A5A_5A5A;
    let rc = gemdos(SSYSTEM, S_GETCOOKIE, MCH_TAG as i32 as i64 as u64, &raw mut v as u64);
    assert_eq!(rc, E_OK, "the tag is present");
    assert_eq!(v, 0xFFFF_FFFF, "_MCH = MCH_NOHARD");
}

#[test]
fn the_value_form_answers_minus_one_like_a_miss() {
    let _lock = JAR.lock().unwrap_or_else(|e| e.into_inner());
    rig();

    // Present: the value 0xFFFFFFFF as a GEMDOS LONG.
    assert_eq!(gemdos(SSYSTEM, S_GETCOOKIE, MCH_TAG as i32 as i64 as u64, 0), -1);
    // Absent: EERROR — the same number.
    assert_eq!(gemdos(SSYSTEM, S_GETCOOKIE, ABSENT_TAG as i32 as i64 as u64, 0), ERROR);
}

#[test]
fn the_pointer_form_tells_a_miss_from_mch() {
    let _lock = JAR.lock().unwrap_or_else(|e| e.into_inner());
    rig();

    let mut v: u32 = 0x5A5A_5A5A;
    let rc = gemdos(SSYSTEM, S_GETCOOKIE, ABSENT_TAG as i32 as i64 as u64, &raw mut v as u64);
    assert_eq!(rc, ERROR, "an absent tag is EERROR in the pointer form too");
    assert_eq!(v, 0x5A5A_5A5A, "and the buffer is not written");
}
