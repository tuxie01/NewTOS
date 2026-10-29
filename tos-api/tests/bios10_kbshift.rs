//! BIOS10 — `Kbshift` (BIOS 0x0B) keeps the whole shift byte, like EmuTOS'
//! `shifty`.
//!
//! Spec: `docs/api/bios/0b-kbshift.md` (decision D1 = 1), EmuTOS
//! `bios/ikbd.c:145-153` (`kbshift`), `:776-883` (`kbd_int`'s changes to
//! `shifty`). The host here is the kernel's wiring in miniature: the trap's
//! query and set go to one `tos_api::bios::shifty::Shifty`, the byte the
//! kernel's IKBD decoder owns, and key bytes go through the same byte. The
//! oracle values are the audit probe lines K01-K13
//! (`measurements/measurement-2026-10-28-bios-core-audit/runs/PROBE-EMUTOS-tt/BCPROBE.LOG`)
//! and `tosdump/probes/ksprobe.c` on EmuTOS
//! (`measurements/measurement-2026-10-29-bios10/RUNINFO.TXT`).

use std::sync::{Mutex, MutexGuard, OnceLock};

use tos_api::bios::shifty::{Shifty, KEY_ALT, KEY_HOME, KEY_INSERT, KEY_LSHIFT, KEY_RELEASED};
use tos_api::host::Host;
use tos_api::trap::{TrapArgs, TrapClass};

/// The decoder's byte, as the kernel holds it (`kernel/src/ikbd.rs`).
static SHIFTY: Shifty = Shifty::new();

/// The kernel host's two shift-state methods (`kernel/src/host_env.rs`).
struct KbdHost;

impl Host for KbdHost {
    fn console_write_byte(&self, _byte: u8) {}
    fn keyboard_shift_state(&self) -> u8 {
        SHIFTY.get()
    }
    fn set_keyboard_shift_state(&self, mask: u8) {
        SHIFTY.set(mask);
    }
}

static HOST: OnceLock<()> = OnceLock::new();
static LOCK: Mutex<()> = Mutex::new(());

fn setup() -> MutexGuard<'static, ()> {
    let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    HOST.get_or_init(|| {
        let h: &'static KbdHost = Box::leak(Box::new(KbdHost));
        let slot: &'static &'static dyn Host = Box::leak(Box::new(h as &'static dyn Host));
        tos_api::host::install_host(slot);
    });
    SHIFTY.set(0);
    g
}

const KBSHIFT: u16 = 0x0B;

/// `Kbshift(flag)` as a program issues it: the WORD sign-extended into `x0`
/// (`tosdump/abi/newtos_svc.c`).
fn kbshift(flag: i16) -> i64 {
    let args = TrapArgs {
        regs: [flag as i64 as u64, 0, 0, 0, 0, 0, 0, 0],
        func: KBSHIFT as u64,
        svc_imm: TrapClass::Bios as u16,
        space: tos_api::uaccess::Space::Kernel,
    };
    tos_api::bios::dispatch(&args)
}

#[test]
fn audit_probe_k01_to_k13_match_the_oracle() {
    // EmuTOS (PROBE-EMUTOS-tt BCPROBE.LOG), ret = the previous byte:
    //   K01 0x00, K02 0x00, K03 0x10, K04 0x10, K05 0xff, K06 0xff, K07 0xff,
    //   K08 0xff, K09 0x81, K10 0x81, K11 0x81, K12 0x81, K13 0x00.
    let _g = setup();
    let s0 = kbshift(-1);
    let got = [
        s0,
        kbshift(0x10),
        kbshift(-1),
        kbshift(0x1FF),
        kbshift(-1),
        kbshift(-2),
        kbshift(-1),
        kbshift(0x7F81),
        kbshift(-1),
        kbshift(-32768),
        kbshift(-1),
        kbshift((s0 & 0xFFFF) as i16),
        kbshift(-1),
    ];
    assert_eq!(
        got,
        [0x00, 0x00, 0x10, 0x10, 0xFF, 0xFF, 0xFF, 0xFF, 0x81, 0x81, 0x81, 0x81, 0x00]
    );
}

#[test]
fn set_stores_every_byte_value_and_the_query_returns_it() {
    let _g = setup();
    for v in 0..=0xFFi16 {
        kbshift(v);
        assert_eq!(kbshift(-1), v as i64, "Kbshift({v:#x}) then Kbshift(-1)");
        assert_eq!(SHIFTY.get() as i16, v, "the decoder's byte");
    }
    // The high byte of the WORD is dropped (bios/ikbd.c:150, UBYTE store).
    kbshift(0x0104);
    assert_eq!(kbshift(-1), 0x04);
}

#[test]
fn a_query_returns_the_byte_and_changes_nothing() {
    let _g = setup();
    SHIFTY.set(0xA8);
    for flag in [-1i16, -2, -0x100, i16::MIN] {
        assert_eq!(kbshift(flag), 0xA8, "Kbshift({flag}) is a query");
    }
    assert_eq!(SHIFTY.get(), 0xA8);
}

#[test]
fn the_return_is_the_previous_byte_zero_extended() {
    let _g = setup();
    SHIFTY.set(0xFF);
    assert_eq!(kbshift(0x00), 0xFF, "no sign extension of bit 7");
    assert_eq!(kbshift(0x80), 0x00);
    assert_eq!(kbshift(-1), 0x80);
}

#[test]
fn the_key_path_sets_home_and_insert_with_alt_and_keeps_a_stored_bit_7() {
    // ksprobe T01-T24 on EmuTOS (tt = falcontc).
    let _g = setup();
    let brk = |c: u8| c | KEY_RELEASED;
    let seq = [
        KEY_ALT, KEY_HOME, brk(KEY_HOME), brk(KEY_ALT), // A
        KEY_ALT, KEY_INSERT, brk(KEY_INSERT), brk(KEY_ALT), // B
        KEY_HOME, brk(KEY_HOME), // C
        KEY_ALT, KEY_HOME, brk(KEY_ALT), brk(KEY_HOME), // D
        KEY_ALT, KEY_INSERT, KEY_HOME, brk(KEY_HOME), brk(KEY_INSERT), brk(KEY_ALT), // E
    ];
    let mut changes = Vec::new();
    let mut prev = kbshift(-1);
    for b in seq {
        SHIFTY.key(b);
        let now = kbshift(-1);
        if now != prev {
            changes.push(now);
            prev = now;
        }
    }
    assert_eq!(
        changes,
        [0x08, 0x28, 0x08, 0x00, 0x08, 0x48, 0x08, 0x00, 0x08, 0x28, 0x20, 0x00, 0x08, 0x48, 0x68, 0x48, 0x08, 0x00]
    );
    // F: the program stores bit 7, the keys change only their own bits.
    assert_eq!(kbshift(0x80), 0x00);
    let mut changes = Vec::new();
    let mut prev = kbshift(-1);
    for b in [KEY_LSHIFT, 0x1E, brk(0x1E), brk(KEY_LSHIFT), KEY_ALT, KEY_HOME, brk(KEY_HOME), brk(KEY_ALT)] {
        SHIFTY.key(b);
        let now = kbshift(-1);
        if now != prev {
            changes.push(now);
            prev = now;
        }
    }
    assert_eq!(changes, [0x82, 0x80, 0x88, 0xA8, 0x88, 0x80]);
}
