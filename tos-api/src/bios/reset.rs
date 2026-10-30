//! Warm and cold restart — the keyboard combination and the OSHEADER reset
//! entry (TASK/RST01).
//!
//! Spec: `docs/api/bios/_reset-warm-cold.md` (EmuTOS `1a7484ab`).
//!
//! # The keyboard combination (§2)
//!
//! [`reset_key`] is EmuTOS `kbd_int`'s first test (`bios/ikbd.c:734-747`):
//! the **make** code of Delete (`0x53`, compared as the raw byte, so the break
//! code `0xD3` never matches) while `shifty & (ALT|CTRL|LSHIFT) == ALT|CTRL`;
//! Right Shift held in addition selects the cold reset. Left Shift held
//! cancels both, and the key is processed normally. The kernel's default
//! `kbdvec` (`kernel::ikbd`) calls it before anything else, as EmuTOS does.
//!
//! # The OSHEADER reset entry (§3)
//!
//! EmuTOS stores `_main` — the full restart path — in `reseth` (+0x04) and
//! `os_res1` (+0x10) (`bios/startup.S:82-83,92-93`). A program that jumps
//! there restarts the machine; TeraDesk's "Reboot" does exactly that
//! (`sdk/apps/teradesk/main.c:2096,2118`). On NewTOS a program runs at EL0
//! and cannot restart anything itself, so both fields point at a small
//! **reset entry** in the shared stub page (`mmu::STUB_PAGE_VA`, MEM12;
//! [`RESET_ENTRY_VA`], [`write_stub`]): it calls the NewTOS-own
//! `Ssystem(S_NEWTOS_RESET, 0, 0)` ([`S_NEWTOS_RESET`] = 2021, the 2000+ block
//! of `docs/api/gemdos/154-ssystem.md`, like `S_NEWTOS_KBDVEC`'s stub), and
//! the kernel restarts the machine. The stub page is mapped EL0
//! execute-only in every memory-protection mode, so the entry is reachable
//! with protection off, on and strict, and its address is below 4 GiB, so it
//! fits the 32-bit fields. Warm or cold follows the `memvalid` magics in the
//! caller's page 0, as on the Atari (`entry_kind`): a program clears them in
//! super state — TeraDesk's "Reboot" does, after `Super(0L)` (MEM12).
//!
//! `os_entry` (+0x00) is the one entry this cannot cover: on m68k it is the
//! instruction `bra.s _main` itself, two bytes, followed by `os_version`
//! (`startup.S:78-81`). An AArch64 instruction is four bytes, so the word at
//! +0x00 would be `os_version << 16 | os_entry` = `0x0404_xxxx`: bits 28:25
//! = `0b0010` put every such word in the SVE encoding space (ARM ARM DDI 0487
//! M.d C4.1, top-level table), never in "Branches, Exception Generating and
//! System instructions" (`0b101x`), and the Cortex-A72 has no SVE. Keeping
//! `os_version` (which programs do read) rules out a jumpable `os_entry`; the
//! field therefore stays 0, and a jump to the OSHEADER base faults as before.

use tos_types::error::{EINVAL, ERROR};
use tos_types::Long;

use crate::bdos::process::loader::{write_stub_code, RESET_STUB_AREA_SIZE, RESET_STUB_AREA_VA, USER_VA_BASE};
use crate::bdos::process::mmu::STUB_PAGE_VA;
use crate::host;
use crate::trap::TrapArgs;

// ─── The keyboard combination ────────────────────────────────────────────

/// `KEY_DELETE` (`bios/ikbd.c:86`): the Delete key's make code.
pub const KEY_DELETE: u8 = 0x53;
/// `MODE_RSHIFT` (`include/biosdefs.h:90`).
pub const MODE_RSHIFT: u8 = 0x01;
/// `MODE_LSHIFT` (`include/biosdefs.h:91`).
pub const MODE_LSHIFT: u8 = 0x02;
/// `MODE_CTRL` (`include/biosdefs.h:92`).
pub const MODE_CTRL: u8 = 0x04;
/// `MODE_ALT` (`include/biosdefs.h:93`).
pub const MODE_ALT: u8 = 0x08;

/// Which restart EmuTOS would run (`bios/bios.h:45,48`).
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ResetKind {
    /// `warm_reset` — Ctrl+Alt+Del; on EmuTOS the system variables survive.
    Warm,
    /// `cold_reset` — Ctrl+Alt+RShift+Del; the RAM configuration is
    /// invalidated.
    Cold,
}

/// EmuTOS `kbd_int`'s reset test (`bios/ikbd.c:734-747`) for one IKBD key
/// byte `scancode` (bit 7 = break) under the modifier state `shifty`
/// (`Kbshift` encoding): `Some(kind)` if the byte restarts the machine.
///
/// ```c
/// if ((scancode == KEY_DELETE)
///     && ((shifty & (MODE_ALT|MODE_CTRL|MODE_LSHIFT)) == (MODE_ALT|MODE_CTRL))) {
///     if (shifty & MODE_RSHIFT) cold_reset(); else warm_reset();
/// }
/// ```
pub const fn reset_key(scancode: u8, shifty: u8) -> Option<ResetKind> {
    if scancode != KEY_DELETE {
        return None;
    }
    if shifty & (MODE_ALT | MODE_CTRL | MODE_LSHIFT) != MODE_ALT | MODE_CTRL {
        return None;
    }
    if shifty & MODE_RSHIFT != 0 {
        Some(ResetKind::Cold)
    } else {
        Some(ResetKind::Warm)
    }
}

// ─── The OSHEADER reset entry ────────────────────────────────────────────

/// `Ssystem` mode the reset entry issues — NewTOS-own block 2000+
/// (`docs/api/gemdos/154-ssystem.md`): restart the machine. NewTOS-internal,
/// like `S_NEWTOS_KBDVEC` (2002); a program reaches it through the OSHEADER
/// reset entry, not by number.
pub const S_NEWTOS_RESET: u16 = 2021;

/// EL0 VA of the reset entry — the value of OSHEADER `reseth` (+0x04) and
/// `os_res1` (+0x10).
pub const RESET_ENTRY_VA: u64 = USER_VA_BASE + RESET_STUB_AREA_VA;

/// The reset entry. Entered by a jump or a call with no arguments (EmuTOS
/// `_main` takes none), it issues `Ssystem(2021, 0, 0)`. That call does not
/// return on a machine that can restart; where it does (a board without a
/// restart mechanism), the entry returns `ERROR` to its caller instead of
/// running into whatever follows it.
///
/// ```text
///   d280fca0  mov x0, #2021     ; mode S_NEWTOS_RESET
///   d2800001  mov x1, #0        ; arg1 = 0
///   d2800002  mov x2, #0        ; arg2 = 0
///   d2802a88  mov x8, #0x154    ; Ssystem
///   d4000021  svc #1
///   d65f03c0  ret
/// ```
///
/// `movz`: `0xD280_0000 | imm16 << 5 | Rd` (ARM ARM C6.2 MOVZ, 64-bit, hw=0);
/// the `mov x8`/`svc`/`ret` words are the ones the `kbdvec_default` stub uses
/// (`xbios::kbdvecs`).
const RESET_ENTRY_CODE: [u32; 6] = [
    0xD280_FCA0,
    0xD280_0001,
    0xD280_0002,
    0xD280_2A88,
    0xD400_0021,
    0xD65F_03C0,
];

// The entry fits its area, and the 2021 baked into the first `mov` is
// S_NEWTOS_RESET; the entry address fits the 32-bit OSHEADER fields.
const _: () = assert!(4 * RESET_ENTRY_CODE.len() as u64 <= RESET_STUB_AREA_SIZE);
const _: () = assert!((RESET_ENTRY_CODE[0] >> 5) & 0xFFFF == S_NEWTOS_RESET as u32);
const _: () = assert!(RESET_ENTRY_CODE[0] & 0x1F == 0); // Rd = x0
const _: () = assert!(RESET_ENTRY_VA <= u32::MAX as u64);

/// Write the reset entry into the shared stub page `page` (the kernel alias
/// of the whole page). Called once by `loader::ensure_stub_page`, next to the
/// other stubs, before the code-cache maintenance.
pub fn write_stub(page: &mut [u8]) {
    let at = (RESET_STUB_AREA_VA - STUB_PAGE_VA) as usize;
    page[at..at + RESET_STUB_AREA_SIZE as usize].fill(0);
    write_stub_code(page, RESET_ENTRY_VA, &RESET_ENTRY_CODE);
}

/// `memvalid` / `memval2` / `memval3` and their magics (EmuTOS `memchk`,
/// `bios/memory.S:125-140`; the loader writes the same values).
const MEMVALID: [(u64, u32); 3] = [(0x420, 0x7520_19F3), (0x43A, 0x2376_98AA), (0x51A, 0x5555_AAAA)];

/// Which restart a jump to `_main` would be on EmuTOS: warm while all three
/// RAM-configuration magics in the caller's system area are intact, cold
/// otherwise (`memchk`, `bios/memory.S:125-140` → `memconf`). TeraDesk
/// clears `memvalid`/`memval2` before it jumps (`main.c:2097-2098`), so its
/// "Reboot" is a cold start, as on TOS. An unreadable area counts as cold.
fn entry_kind(args: &TrapArgs) -> ResetKind {
    let intact = args.space.is_user()
        && MEMVALID
            .iter()
            .all(|&(va, magic)| args.space.get_u32(USER_VA_BASE + va) == Ok(magic));
    if intact {
        ResetKind::Warm
    } else {
        ResetKind::Cold
    }
}

/// `S_NEWTOS_RESET` (2021): restart the machine, as a jump to EmuTOS `_main`
/// does. `arg1`/`arg2` reserved, must be 0.
///
/// * does not return — the machine restarts (`board-rpi4`);
/// * `EINVAL` — `arg1` or `arg2` is not 0; nothing happened;
/// * `ERROR` — this host has no restart mechanism (`board-qemu-virt`, a
///   hostless build).
pub(crate) fn s_newtos_reset(args: &TrapArgs) -> Long {
    if args.arg_u64(1) != 0 || args.arg_u64(2) != 0 {
        return EINVAL;
    }
    let kind = entry_kind(args);
    match host::host() {
        Some(h) => {
            h.system_restart(kind);
            ERROR
        }
        None => ERROR,
    }
}
