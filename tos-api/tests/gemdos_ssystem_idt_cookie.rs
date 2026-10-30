//! **DESK13** — the route the desktop takes to the `_IDT` cookie, pinned at
//! the GEMDOS trap boundary.
//!
//! EmuTOS's desktop reads `_IDT` with `Supexec((LONG)get_idt_cookie)`
//! (`references/emutos/desk/desksupp.c:219`), because on m68k the jar is only
//! reachable through the pointer at `$5A0`, in supervisor-only low memory. That
//! is an m68k *mechanism*, and `docs/COMPATIBILITY.md` puts mechanisms on the
//! free side of the line. NewTOS has neither the need nor the call: XBIOS
//! `Supexec` (0x26) is an open ADR (`docs/api/xbios/26-supexec.md`, "NewTOS
//! status: decision deferred"), and the cookie-jar spec already names the
//! replacement — "There is no `Supexec` in the kernel … the low-memory area an
//! app wants is already in its own address space"
//! (`docs/api/bios/_cookie-jar.md` §Status 2026-09-14).
//!
//! So `kernel/src/desk/dosbind.rs::dos_getcookie` issues
//! `Ssystem(S_GETCOOKIE, tag, 0)` over `svc #1` — the same trap the desktop
//! uses for every other system service (DESK10). This test drives exactly that
//! call through `tos_api::gemdos::dispatch`, and asserts the two things the
//! desktop's date/time formatting now rests on:
//!
//! 1. the call answers with the cookie **value in `x0`** when `arg2` is 0
//!    (the desktop cannot use the pointer form: `s_getcookie` stores through an
//!    unprivileged `sttr`, which faults on a kernel address); and
//! 2. the seeded `_IDT` is `0x0000112E` — `docs/api/bios/_cookie-jar.md:137`,
//!    `tos-api/src/cookie.rs` `seed_defaults` — i.e. **24-hour, day-before-
//!    month, `.` separator**, the pinned oracle's value (`tosdump/cases/jar-r1.tsv`
//!    case 53; user decision 2026-10-30, `TASK/COOK02.md`).
//!
//! Point 2 is the ground truth behind DESK13's date decision: `_IDT` says DMY
//! (and, since COOK02, `.`), and the desktop's text view used to print MDY with
//! a dash. The resolution rule now lives in exactly one place
//! (`programs/desktop/src/model.rs`'s `DateFmt::rendered`/`TimeFmt::rendered`), and
//! it reads the word this test pins: with the date preference "Standard"
//! (`DateFmt::Idt`) NEWDESK takes the separator from `_IDT`, so its file list
//! shows `.` like EmuTOS on the oracle.

use std::sync::{Mutex, Once};

use tos_api::host::Host;
use tos_api::trap::{TrapArgs, TrapClass};

/// The cookie jar is a process-wide static; keep the cases in this binary from
/// racing each other on it.
static JAR: Mutex<()> = Mutex::new(());

/// The GEMDOS dispatcher refuses every call with `EUNDEV` until a host is
/// installed (`tos-api/src/gemdos/mod.rs:137-141`). `Ssystem`'s cookie
/// subfunctions touch nothing else of it, so the smallest possible one will do:
/// `console_write_byte` is the trait's only required method.
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

/// `Ssystem` — GEMDOS 0x154 (`docs/api/gemdos/154-ssystem.md`).
const SSYSTEM: u16 = 0x154;
/// `S_GETCOOKIE` — `Ssystem` subfunction 8 (FreeMiNT `sys/mint/ssystem.h`).
const S_GETCOOKIE: u64 = 8;

/// `'_IDT'` big-endian, as `tos_api::cookie::IDT` registers it.
const IDT_TAG: u32 = u32::from_be_bytes(*b"_IDT");
/// A tag that is deliberately not in the jar.
const ABSENT_TAG: u32 = u32::from_be_bytes(*b"ZZZZ");

/// FreeMiNT `EERROR` — `S_GETCOOKIE`'s answer for a tag that is not present
/// (`sys/cookie.c:508-509`, `tos-api/src/gemdos/ssystem.rs`).
const ERROR: i32 = -1;

// `_IDT` bit layout (`references/emutos/include/biosdefs.h:161-173`).
const IDT_24H: u32 = 0x1000;
const IDT_TMASK: u32 = 0x1000;
const IDT_BIT_DM: u32 = 0x100;
const IDT_DMASK: u32 = 0x300;
const IDT_SMASK: u32 = 0xFF;

/// Issue a GEMDOS call the way `desk::dosbind::dos_trap` does: function id
/// separate, arguments in `x0..`.
fn gemdos(func: u16, a0: u64, a1: u64, a2: u64) -> i32 {
    let args = TrapArgs {
        regs: [a0, a1, a2, 0, 0, 0, 0, 0],
        func: func as u64,
        svc_imm: TrapClass::Gemdos as u16,
        space: tos_api::uaccess::Space::Kernel,
    };
    tos_api::gemdos::dispatch(&args)
}

/// `dos_getcookie(tag)` — the desktop's own wrapper, reproduced here so the
/// test exercises the shape the kernel really sends.
fn dos_getcookie(tag: u32) -> Option<u32> {
    let rc = gemdos(SSYSTEM, S_GETCOOKIE, tag as i32 as i64 as u64, 0);
    (rc >= 0).then_some(rc as u32)
}

#[test]
fn the_desktop_reads_idt_through_ssystem_and_gets_the_seeded_word() {
    let _lock = JAR.lock().unwrap_or_else(|e| e.into_inner());
    rig();

    let idt = dos_getcookie(IDT_TAG).expect("_IDT is seeded at boot");
    assert_eq!(idt, 0x0000_112E, "docs/api/bios/_cookie-jar.md:137");

    // The three fields the desktop's `Default` buttons defer to.
    assert_eq!(idt & IDT_TMASK, IDT_24H, "_IDT says 24-hour");
    assert_eq!(idt & IDT_DMASK, IDT_BIT_DM, "_IDT says day before month (DDMMYY)");
    assert_eq!((idt & IDT_SMASK) as u8, b'.', "_IDT's separator is '.'");
}

#[test]
fn a_tag_that_is_not_in_the_jar_answers_error_not_a_value() {
    let _lock = JAR.lock().unwrap_or_else(|e| e.into_inner());
    rig();

    assert_eq!(
        gemdos(SSYSTEM, S_GETCOOKIE, ABSENT_TAG as i32 as i64 as u64, 0),
        ERROR,
        "a missing tag is EERROR, which `dos_getcookie` turns into None"
    );
    assert_eq!(dos_getcookie(ABSENT_TAG), None);
}
