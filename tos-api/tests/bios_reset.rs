//! RST01 — the keyboard reset combination and the OSHEADER reset entry.
//!
//! Spec: `docs/api/bios/_reset-warm-cold.md` §2 (EmuTOS `kbd_int`,
//! `bios/ikbd.c:734-747`) and §3 (`reseth`/`os_res1` → `_main`,
//! `bios/startup.S:82-83,92-93`).

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Mutex, Once};

use tos_api::bios::reset::{
    reset_key, write_stub, ResetKind, KEY_DELETE, MODE_ALT, MODE_CTRL, MODE_LSHIFT, MODE_RSHIFT,
    RESET_STUB_VA, S_NEWTOS_RESET,
};
use tos_api::host::Host;
use tos_api::trap::{TrapArgs, TrapClass};

/// `MODE_CAPS` (`include/biosdefs.h:95`) — outside the reset mask.
const MODE_CAPS: u8 = 0x10;
/// `MODE_HOME`/`MODE_INSERT` (`biosdefs.h:99-100`) — outside the mask too.
const MODE_HOME: u8 = 0x20;
const MODE_INSERT: u8 = 0x40;

/// Every combination of the four modifier bits the test looks at, with each
/// bit outside the mask (`CAPS`, `HOME`, `INSERT`) on and off: the answer
/// for the Delete make code must depend on `CTRL`, `ALT`, `LSHIFT`, `RSHIFT`
/// only, exactly as `(shifty & (ALT|CTRL|LSHIFT)) == (ALT|CTRL)` and
/// `shifty & RSHIFT` say.
#[test]
fn delete_make_under_every_modifier_combination() {
    for sca in 0u8..16 {
        for extra in [0, MODE_CAPS, MODE_HOME, MODE_INSERT, MODE_CAPS | MODE_HOME | MODE_INSERT] {
            let shifty = sca | extra;
            let ctrl = shifty & MODE_CTRL != 0;
            let alt = shifty & MODE_ALT != 0;
            let lshift = shifty & MODE_LSHIFT != 0;
            let rshift = shifty & MODE_RSHIFT != 0;
            let want = if ctrl && alt && !lshift {
                Some(if rshift { ResetKind::Cold } else { ResetKind::Warm })
            } else {
                None
            };
            assert_eq!(reset_key(KEY_DELETE, shifty), want, "shifty {shifty:#04x}");
        }
    }
}

/// The named cases of the spec table, spelled out.
#[test]
fn the_named_combinations() {
    let ca = MODE_CTRL | MODE_ALT;
    assert_eq!(reset_key(0x53, ca), Some(ResetKind::Warm), "Ctrl+Alt+Del");
    assert_eq!(reset_key(0x53, ca | MODE_RSHIFT), Some(ResetKind::Cold), "Ctrl+Alt+RShift+Del");
    assert_eq!(reset_key(0x53, ca | MODE_LSHIFT), None, "LShift cancels warm");
    assert_eq!(
        reset_key(0x53, ca | MODE_LSHIFT | MODE_RSHIFT),
        None,
        "LShift cancels cold too"
    );
    assert_eq!(reset_key(0x53, MODE_CTRL), None, "Ctrl alone");
    assert_eq!(reset_key(0x53, MODE_ALT), None, "Alt alone");
    assert_eq!(reset_key(0x53, 0), None, "plain Delete");
    assert_eq!(reset_key(0x53, ca | MODE_CAPS), Some(ResetKind::Warm), "CapsLock is not in the mask");
}

/// The break code of Delete (`0x53 | KEY_RELEASED` = `0xD3`) never matches:
/// EmuTOS compares the raw byte (`scancode == KEY_DELETE`), not
/// `scancode_only`. Nor does any other key.
#[test]
fn only_the_delete_make_code_restarts() {
    for shifty in [MODE_CTRL | MODE_ALT, MODE_CTRL | MODE_ALT | MODE_RSHIFT] {
        assert_eq!(reset_key(0xD3, shifty), None, "Delete break, shifty {shifty:#04x}");
        for code in 0u8..=0xFF {
            if code != KEY_DELETE {
                assert_eq!(reset_key(code, shifty), None, "byte {code:#04x}");
            }
        }
    }
}

/// The stub the loader writes at `RESET_STUB_VA`: `Ssystem(2021, 0, 0)`
/// (`svc #1`, `x8 = 0x154`), then `ret` — in the shared stub page since
/// MEM12 (`mmu::STUB_PAGE_VA`, EL0 execute-only).
#[test]
fn the_reset_stub_calls_ssystem_2021() {
    use tos_api::bdos::process::mmu::{PAGE_SIZE, STUB_PAGE_VA};
    assert_eq!(S_NEWTOS_RESET, 2021);
    assert!((STUB_PAGE_VA..STUB_PAGE_VA + PAGE_SIZE).contains(&RESET_STUB_VA));
    assert!(RESET_STUB_VA <= u32::MAX as u64, "fits the 32-bit OSHEADER fields");
    let mut page = vec![0xEEu8; 0x1000];
    write_stub(&mut page);
    let at = (RESET_STUB_VA - STUB_PAGE_VA) as usize;
    let words: Vec<u32> = (0..6)
        .map(|i| u32::from_le_bytes(page[at + 4 * i..at + 4 * i + 4].try_into().unwrap()))
        .collect();
    assert_eq!(
        words,
        [0xD280_FCA0, 0xD280_0001, 0xD280_0002, 0xD280_2A88, 0xD400_0021, 0xD65F_03C0]
    );
    // Its area is zeroed past the code, nothing outside it is touched.
    assert!(page[at + 24..at + 0x20].iter().all(|&b| b == 0));
    assert_eq!(page[at - 1], 0xEE);
    assert_eq!(page[at + 0x20], 0xEE);
}

// ── Ssystem(S_NEWTOS_RESET) at the trap boundary ──────────────────────────

/// What the host was asked to do: 0 = nothing, 1 = warm, 2 = cold.
static ASKED: AtomicU8 = AtomicU8::new(0);
static RIG: Mutex<()> = Mutex::new(());

/// A host without a restart mechanism that records the request and returns,
/// as `board-qemu-virt` does.
struct NoRestartHost;

impl Host for NoRestartHost {
    fn console_write_byte(&self, _byte: u8) {}

    fn system_restart(&self, kind: ResetKind) {
        ASKED.store(
            match kind {
                ResetKind::Warm => 1,
                ResetKind::Cold => 2,
            },
            Ordering::Relaxed,
        );
    }
}

static HOST: Once = Once::new();

fn rig() {
    HOST.call_once(|| {
        let slot: &'static &'static dyn Host =
            Box::leak(Box::new(&NoRestartHost as &'static dyn Host));
        tos_api::host::install_host(slot);
    });
    ASKED.store(0, Ordering::Relaxed);
}

fn ssystem(mode: u64, a1: u64, a2: u64) -> i32 {
    tos_api::gemdos::dispatch(&TrapArgs {
        regs: [mode, a1, a2, 0, 0, 0, 0, 0],
        func: 0x154,
        svc_imm: TrapClass::Gemdos as u16,
        space: tos_api::uaccess::Space::Kernel,
    })
}

const ERROR: i32 = -1;
const EINVAL: i32 = -25;

/// A host that cannot restart is asked, returns, and the call says `ERROR`
/// (the stub then `ret`s to its caller). A kernel caller has no page-0
/// magics to read, so the request is a cold one.
#[test]
fn a_host_without_restart_answers_error() {
    let _lock = RIG.lock().unwrap_or_else(|e| e.into_inner());
    rig();
    assert_eq!(ssystem(2021, 0, 0), ERROR);
    assert_eq!(ASKED.load(Ordering::Relaxed), 2);
}

/// `arg1`/`arg2` are reserved: a non-zero value is refused before anything
/// happens.
#[test]
fn reserved_arguments_must_be_zero() {
    let _lock = RIG.lock().unwrap_or_else(|e| e.into_inner());
    rig();
    assert_eq!(ssystem(2021, 1, 0), EINVAL);
    assert_eq!(ssystem(2021, 0, 1), EINVAL);
    assert_eq!(ASKED.load(Ordering::Relaxed), 0, "refused: the host was not asked");
}
