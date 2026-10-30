//! `Setexc` (BIOS 0x05): the m68k vector cells and the low-memory system
//! variables, as page-0 cells.
//!
//! Spec: [`docs/api/bios/05-setexc.md`], decisions D1–D3 and P3 (user
//! 2026-10-28, TASK/BIOS07). EmuTOS `setexc` reads the LONG at `4 * num`,
//! stores the new LONG unless it is `-1`, and returns the old one
//! (`references/emutos/bios/bios.c:1070-1080`). NewTOS has no flat vector
//! page, but every process has a private page 0 that carries the classic
//! low-memory system area (`bdos::process::loader`), and `Setexc` answers
//! from there for every `num < 0x200` (D3):
//!
//! * `num <= 0x103` — the 256 m68k vectors and the `etv_*` hooks — are
//!   **one cell for every process**, as EmuTOS has one vector table: the
//!   cell lives here ([`VECTORS`]), 32 bits wide like the LONG, and every
//!   loaded process's page 0 carries a copy at `4 * num`, written by the
//!   loader ([`write_page0_vectors`]) and again whenever the cell changes.
//!   A program in super state that writes its copy of `$008`–`$407` directly
//!   changes the cell, as on EmuTOS: the page-0 check takes the value over
//!   at the edges of the next system call ([`adopt_page0_write`],
//!   TASK/BIOS16); `etv_term` (`$408`) changes only through `Setexc`.
//!   A hook installed in such a cell belongs to the process that wrote it
//!   (ADR-031 rule 3, [`hooks::record_install`]). Of these hooks NewTOS calls
//!   only `etv_term` (the kernel's `Pterm` path, [`vector`]); the CPU, trap
//!   and interrupt vectors and `etv_timer`/`etv_critic` are stored and never
//!   called (ADR-031).
//! * `0x104 <= num < 0x200` are the system variables at `$410`–`$7FC`: the
//!   calling process's own page-0 cell.
//! * `num >= 0x200` (and a negative `num`) answers 0 and touches nothing (D2,
//!   a documented deviation: EmuTOS reads and writes RAM there).
//!
//! The answer is the 32-bit cell **sign-extended** into `x0` (D1, the rule of
//! `TrapArgs`); every address a cell holds by default lies below 2 GiB (the
//! MEM11 invariant). A vector NewTOS does not serve holds the shared `ret`
//! stub, EmuTOS' `just_rts` ([`JUST_RTS`], D3).

use crate::bdos::process::hooks::{self, Cell, Hook, Via};
use crate::trap::TrapArgs;
use core::sync::atomic::{AtomicU32, Ordering};

// Canonical TOS hook vector numbers (m68k address / 4; `tosvars.ld:33-36`).

/// `etv_timer` hook — 20 ms system tick (m68k `$0400`).
pub const ETV_TIMER: usize = 0x100;
/// `etv_critic` hook — critical-error handler (m68k `$0404`).
pub const ETV_CRITIC: usize = 0x101;
/// `etv_term` hook — process-terminate hook (m68k `$0408`).
pub const ETV_TERM: usize = 0x102;
/// `etv_xtra` (m68k `$040C`): never called, by EmuTOS either
/// (`references/emutos/tosvars.ld:36`); the last of the shared cells.
pub const ETV_XTRA: usize = 0x103;

/// Number of shared cells: vectors `0..=0xFF` and `etv_timer` … `etv_xtra`.
const NSHARED: usize = ETV_XTRA + 1;

/// Upper bound (exclusive) of the page-0 regime: `num` whose m68k address
/// `4 * num` lies in the reserved low-memory region `[0x000, 0x800)`. Between
/// [`NSHARED`] and this, `Setexc` reads and writes the calling process's own
/// page-0 cell — the `Setexc(0x5A0/4, -1)` cookie-jar idiom (TeraDesk
/// `library/utility/cookie.c:56`) and `memvalid` (`$420`, `Setexc(0x108)`)
/// land here.
const SYSVAR_NUM_LIMIT: usize = 0x200;

/// The shared `ret` stub, EmuTOS' `just_rts` (`xbios::kbdvecs::RET_STUB_VA`,
/// in the stub page every process maps execute-only): the boot value of
/// every vector NewTOS does not serve and of `etv_term` (D3). EmuTOS boots
/// `etv_timer`/`etv_term` with `just_rts` and `etv_critic` with
/// `default_etv_critic` (`bios/bios.c:360-362`); GEMDOS then chains
/// `tikfrk` into `etv_timer` (`bdos/time.c:167`). NewTOS calls neither
/// `etv_timer` nor `etv_critic` (ADR-031), so both hold the stub; a program
/// that chains to the old value branches to a valid entry, never to 0.
pub const JUST_RTS: i64 = crate::xbios::kbdvecs::RET_STUB_VA as i64;

const _: () = assert!(JUST_RTS > 0 && JUST_RTS < 0x8000_0000);

/// The boot value of shared cell `num`.
///
/// * vectors 0 and 1 (`$000`/`$004`): the first two LONGs of the OSHEADER —
///   EmuTOS copies them there where the hardware does not route the ROM to
///   address 0 (`references/emutos/bios/bios.c:120-136`): `os_entry` and
///   `os_version`, then `reseth`, the reset entry;
/// * vectors `2..=0xFF` and `etv_timer`/`etv_critic`/`etv_term`: [`JUST_RTS`];
/// * `etv_xtra`: 0, as on EmuTOS (`bios-core-r1` case 58).
const fn default_of(num: usize) -> u32 {
    match num {
        0 => crate::bdos::process::loader::OSHEADER_LONG0,
        1 => crate::bios::reset::RESET_ENTRY_VA as u32,
        ETV_XTRA => 0,
        _ => JUST_RTS as u32,
    }
}

/// The shared cells (`num <= ETV_XTRA`), each a 32-bit LONG like the m68k
/// cell it stands for.
static VECTORS: [AtomicU32; NSHARED] = {
    let mut v = [const { AtomicU32::new(0) }; NSHARED];
    let mut i = 0;
    while i < NSHARED {
        v[i] = AtomicU32::new(default_of(i));
        i += 1;
    }
    v
};

/// A 32-bit cell as the LONG result in `x0`: sign-extended (D1).
const fn long(v: u32) -> i64 {
    v as i32 as i64
}

/// `Setexc` (0x05) — get (when `vector` is `-1`) or set the cell for vector
/// or system variable `num`, returning the previous value (EmuTOS `setexc`,
/// `references/emutos/bios/bios.c:1070-1080`; the three regimes are the
/// module doc's).
///
/// The query test looks at the low 32 bits, the LONG EmuTOS compares with
/// `-1` (`bios.c:1074`; P3): `0x0000_0000_FFFF_FFFF` queries as `-1` does
/// (measured, `tosdump/probes/sxprobe.c` Q03/Q04). A write stores the low 32
/// bits. A write reports OS hooks ([`note_write`], ADR-031).
pub fn setexc(args: &TrapArgs) -> i64 {
    // A negative WORD becomes a huge index and falls through to the final 0.
    let num = args.arg_i16(0) as usize;
    let new = args.arg_u64(1) as u32;
    let query = new == u32::MAX;

    if num < NSHARED {
        let old = VECTORS[num].load(Ordering::Relaxed);
        if !query {
            store_shared(num, new);
            note_write(num, long(old), new);
        }
        return long(old);
    }

    // The system variables: the calling process's own page-0 cell, through
    // the page's kernel alias (ARCH03 stage 3a, as `bios::tick` and `Keytbl`
    // do) — the very cell the program sees at VA `4 * num`, without a
    // user-mode access that could fault or depend on the installed `TTBR0`.
    // With no process running (an in-kernel caller such as the B1 selftest,
    // which installs a loaded image's `TTBR0` itself) the cell is reached by
    // a fault-recovered user-mode access at VA `4 * num` (`uaccess`).
    if num < SYSVAR_NUM_LIMIT {
        let Some(page0) = crate::bdos::process::current_page0_kva() else {
            let va = (num * 4) as u64;
            let Ok(old) = crate::uaccess::get_user_u32(va) else {
                return i64::from(crate::uaccess::FAULTED);
            };
            if !query && crate::uaccess::put_user_u32(va, new).is_err() {
                return i64::from(crate::uaccess::FAULTED);
            }
            return long(old);
        };
        // SAFETY: `4 * num + 4 <= 0x800` lies inside the running process's
        // 4-KiB page 0, a static kernel page per slot reached through its
        // kernel alias; the access is unaligned-safe and single-CPU under
        // the trap.
        let cell = unsafe { page0.add(num * 4) } as *mut [u8; 4];
        let old = u32::from_le_bytes(unsafe { cell.read_unaligned() });
        if !query {
            // SAFETY: as above — the process's own sysvar cell.
            unsafe { cell.write_unaligned(new.to_le_bytes()) };
            note_write(num, long(old), new);
        }
        return long(old);
    }

    0
}

/// Store `v` into shared cell `num` and into the copy every loaded process's
/// page 0 carries at `4 * num`.
fn store_shared(num: usize, v: u32) {
    VECTORS[num].store(v, Ordering::Relaxed);
    for slot in 0..crate::bdos::process::MAX_PROCESSES {
        // SAFETY: shared cells are written only from the trap path (BDOS giant
        // lock, single CPU); `page_bytes_mut` yields only a loaded slot's page
        // 0, and `4 * num + 4 <= 0x410` lies inside it.
        if let Some(page) = unsafe { crate::bdos::process::arena::page_bytes_mut(slot) } {
            page[4 * num..4 * num + 4].copy_from_slice(&v.to_le_bytes());
        }
    }
}

/// Write the shared cells into a freshly loaded process's page 0 `page0`
/// (the loader's `populate_sysvars`): the process sees at `4 * num` what
/// `Setexc(num, -1)` answers.
pub fn write_page0_vectors(page0: &mut [u8]) {
    for (num, cell) in VECTORS.iter().enumerate() {
        page0[4 * num..4 * num + 4].copy_from_slice(&cell.load(Ordering::Relaxed).to_le_bytes());
    }
}

/// The value of shared cell `num` as `Setexc(num, -1)` answers it (D1), or 0
/// for a `num` that is no shared cell. NewTOS' `Pterm` reads `etv_term`
/// through this and delivers the handler as an EL0 upcall unless it is
/// [`JUST_RTS`] (whose call would do nothing) or 0 (BIOS finding H1).
pub fn vector(num: usize) -> i64 {
    match VECTORS.get(num) {
        Some(c) => long(c.load(Ordering::Relaxed)),
        None => 0,
    }
}

/// The raw 32-bit value of shared cell `num` — what every page 0 carries at
/// `4 * num` unless the process wrote its copy itself (the page-0 hook check,
/// [`hooks::check_page0`]). `None` for a `num` that is no shared cell.
pub(crate) fn shared_cell(num: usize) -> Option<u32> {
    VECTORS.get(num).map(|c| c.load(Ordering::Relaxed))
}

/// Put `v` into shared cell `num` — the owner's `Pterm` giving a hook cell
/// its previous value back (ADR-031 rule 3, [`hooks::on_terminate`]), every
/// page-0 copy included. No-op for a `num` that is no shared cell.
pub(crate) fn set_vector(num: usize, v: i64) {
    if num < NSHARED {
        store_shared(num, v as u32);
    }
}

/// The hook behind shared cell `num` that NewTOS does not call (ADR-031):
/// the CPU, trap and interrupt vectors and `etv_timer`/`etv_critic`. `None`
/// for `etv_term` (served, `kernel` `handle_pterm`) and `etv_xtra` (EmuTOS
/// never calls it).
fn unserved_hook(num: usize) -> Option<Hook> {
    match num {
        0..=0xFF => Some(Hook::Vector(num as u8)),
        ETV_TIMER => Some(Hook::EtvTimer),
        ETV_CRITIC => Some(Hook::EtvCritic),
        _ => None,
    }
}

/// The OS-hook half of a `Setexc` write of `new` into cell `num`, which held
/// `old` (both as `Setexc` answers them), for the running process (none: the
/// kernel, nothing to report; ADR-031, [`hooks`]):
///
/// * shared cell (`num <= ETV_XTRA`): [`note_shared_write`];
/// * system variable: the page-0 hook check (`bell_hook`/`kcl_hook` are
///   `0x16B`/`0x16C`; rule 4).
fn note_write(num: usize, old: i64, new: u32) {
    let slot = crate::bdos::process::slot_for(crate::bdos::process::current());
    if num >= NSHARED {
        if let Some(slot) = slot {
            hooks::check_page0(slot);
        }
        return;
    }
    note_shared_write(slot, num, old, new, Via::Setexc);
}

/// The OS-hook half of a write of `new` into shared cell `num`, which held
/// `old` (as `Setexc` answers it), by process `slot` (none: the kernel,
/// nothing to report), seen via `via` (`Setexc`, or page 0 for an adopted
/// direct write, [`adopt_page0_write`]): the process becomes the owner of the
/// cell, so its `Pterm` can give the cell back (ADR-031 rule 3); a hook
/// NewTOS does not call ([`unserved_hook`]) set to anything but its boot value
/// warns once per process (rules 1, 5).
fn note_shared_write(slot: Option<usize>, num: usize, old: i64, new: u32, via: Via) {
    hooks::record_install(slot, Cell::Vector(num as u16), old as u64, long(new) as u64);
    if let (Some(slot), Some(hook)) = (slot, unserved_hook(num)) {
        if new != default_of(num) {
            hooks::note(slot, hook, u64::from(new), via);
        }
    }
}

/// Is shared cell `num` one whose page-0 copy, written directly by a program
/// in super state, becomes the cell ([`adopt_page0_write`])? The vectors
/// `2..=0xFF` and `etv_timer`/`etv_critic` (`$008`–`$407`), the cells NewTOS
/// only stores (TASK/BIOS16, option B). Not `etv_term` (`$408`): NewTOS calls
/// it at `Pterm`, and only a value installed through `Setexc(0x102)` is
/// called (ADR-031: page 0 is never acted on to call something). Not vectors
/// 0/1 (the OSHEADER's LONGs) nor `etv_xtra` (`$40C`): outside the page-0
/// check until measured (user decision 2026-10-30).
pub(crate) const fn adopts_page0_write(num: usize) -> bool {
    num >= 2 && num < ETV_TERM
}

/// Process `slot` wrote `v` straight into its page-0 copy of shared cell
/// `num` (super state; found by the page-0 check, [`hooks::check_page0`], at
/// the edges of a system call): take it over as the cell, as EmuTOS has one
/// memory cell that a direct write changes (`references/emutos/bios/bios.c:1070-1080`;
/// TASK/BIOS16). Every page-0 copy gets the value, `Setexc(num, -1)` answers
/// it; the process owns it like a `Setexc` write (its `Pterm` puts the
/// previous value back while the cell still holds it, `Ptermres` keeps it),
/// and an unserved hook warns once. Nothing is ever called through it. No-op
/// for a `num` that [`adopts_page0_write`] excludes.
pub(crate) fn adopt_page0_write(slot: usize, num: usize, v: u32) {
    if !adopts_page0_write(num) {
        return;
    }
    let old = VECTORS[num].load(Ordering::Relaxed);
    if old == v {
        return;
    }
    store_shared(num, v);
    note_shared_write(Some(slot), num, long(old), v, Via::Page0);
}


// ─── `mousexvec` — the extended-mouse scancode vector ──────────────────────
//
// EmuTOS reserves a BIOS-level vector for the IKBD scancodes that are *not*
// keys but extra mouse buttons and wheel detents: `_mousexvec: .ds.l 1
// // mouse-routine for additional buttons` (`bios/aciavecs.S:127`), declared
// `extern void (*mousexvec)(WORD scancode);` (`include/biosext.h:78`).
// `init_acia_vecs` seeds it with the `just_rts` no-op (`aciavecs.S:149-151`),
// `kbd_int` calls it for the seven extended-mouse scancodes instead of
// decoding them as keys (`bios/ikbd.c:748-760`), and the VDI installs its own
// handler at workstation open: `mousexvec = vdi_mousex_handler`
// (`vdi/vdi_mouse.c:612`).
//
// It is **not** part of the `Kbdvbase()` table: `_mousexvec` sits two LONGs
// *below* `_kbdvecs` (`aciavecs.S:127-131`), so no program can reach it
// through the documented positive offsets. It is a BIOS-internal hand-off
// from the keyboard decoder to the mouse driver, and that is all it is here.
// (Since KEYS02 each process's EL0 table — `xbios::kbdvecs` — carries a
// `mousexvec` slot at `Kbdvbase() - 16` to keep the EmuTOS layout; it holds
// the `just_rts` stub and is never called. The live vector is this cell.)
//
// NewTOS' IKBD decoder lives in the kernel crate and the VDI in this one, so
// the cell has to live on the `tos-api` side of the dependency edge; the
// kernel's `ikbd::decode_key` calls [`mousexvec`], the VDI's
// `vdimouse_init` equivalent calls [`set_mousexvec`].

/// The installed extended-mouse handler, as a `fn(u8)` address. `0` is the
/// power-up "nobody installed one" state and is the exact behavioural
/// equivalent of EmuTOS' `just_rts` default (`aciavecs.S:149-151`).
static MOUSEXVEC: core::sync::atomic::AtomicUsize =
    core::sync::atomic::AtomicUsize::new(0);

/// Install the extended-mouse handler — EmuTOS `vdimouse_init`'s
/// `mousexvec = vdi_mousex_handler` (`vdi/vdi_mouse.c:612`). Idempotent.
pub fn set_mousexvec(f: fn(u8)) {
    MOUSEXVEC.store(f as usize, Ordering::Relaxed);
}

/// Call the extended-mouse handler with `scancode` — EmuTOS `kbd_int`'s
/// `mousexvec(scancode)` (`bios/ikbd.c:758`). A no-op while no handler is
/// installed (EmuTOS' `just_rts`).
pub fn mousexvec(scancode: u8) {
    let p = MOUSEXVEC.load(Ordering::Relaxed);
    if p == 0 {
        return;
    }
    // Go through a data pointer rather than an integer so no int-to-fn
    // transmute lint fires (same shape as the kernel-built `G_USERDEF`
    // callout, `aes/object.rs:2741-2746`).
    let code = p as *const ();
    // SAFETY: the cell is written only by `set_mousexvec`, always with a live
    // `fn(u8)` of this binary; `0` (never installed) was rejected above.
    let f: fn(u8) = unsafe { core::mem::transmute(code) };
    f(scancode);
}
