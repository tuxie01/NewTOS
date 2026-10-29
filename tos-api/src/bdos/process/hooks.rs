//! OS hooks installed by programs — ADR-031 (`docs/MEMORY.md`, "ADR-031: OS
//! hooks installed by programs"), TASK/BIOS15.
//!
//! On TOS a program can hang code into places the OS later calls: `Setexc`
//! vectors, page-0 cells it writes in supervisor mode (`hdv_*`, `swv_vec`,
//! …), the `Kbdvbase` table, the XBIOS installers (`Initmous`, `Mfpint`,
//! `Xbtimer`) and the VDI exchange vectors (`vex_*`). NewTOS serves only
//! `kbdvec`, `etv_term`, `vex_wheelv`, `Supexec` and `G_USERDEF`; every other
//! hook is stored (or not even that) and never called. This module is the one
//! mechanism behind the ADR's two rules for them:
//!
//! * **Never silent (rules 1, 4, 5).** The first installation of a hook NewTOS
//!   does not call writes one warning per hook and process ([`note`]). The
//!   detection points feed it: `Setexc` (`bios::vectors`), the page-0 cells
//!   ([`check_page0`] — read only, never called — when a process leaves the
//!   super state, at every trap it makes in super state, and when it ends in
//!   super state), the `Kbdvbase` slots at the first event that would call
//!   them (`xbios::kbdvecs::check_events`), and `Initmous`, `Mfpint`,
//!   `Xbtimer` and the `vex_*` calls in the call itself. Hooks EmuTOS itself
//!   never calls (`etv_xtra`, `hdv_init`, `hdv_boot`, `prt_*`, `aux_*`) have
//!   no warning. The model is the former accessory `kbdvec` warning: a flag
//!   per process slot, given back when the slot is freed ([`release_slot`]).
//!   `tos-api` has no log of its own: the warnings queue here and the kernel
//!   prints them after the trap ([`pop`]).
//! * **Ownership (rule 3).** A hook in a cell every process shares — the
//!   `Setexc` vector cells up to `etv_xtra` (every page 0 carries a copy),
//!   the `vex_*` vectors — belongs to the process that installed it
//!   ([`record_install`]). When the process ends without unhooking
//!   ([`on_terminate`]), the previous value goes back only if the cell still
//!   holds the process's hook (the top of an XBRA chain); otherwise the hook
//!   stays and the kernel warns. After `Ptermres` the hook stays and its
//!   owner stays resident. The system variables in page 0 and the `Kbdvbase`
//!   table are per process and end with it; they need no owner.
//!
//! Everything here runs in trap context on the single CPU (the BDOS giant
//! lock). The one interrupt-time producer, the IKBD decoder, only sets an
//! atomic event bit (`xbios::kbdvecs::saw_event`); the warning is made at the
//! next trap. The state sits behind a hand-rolled flag that is never
//! contended in the kernel and only serialises host-test threads.

use core::cell::UnsafeCell;
use core::fmt;
use core::sync::atomic::{AtomicBool, Ordering};

use super::{mmu, pd_of, slot_for, MAX_PROCESSES};

// ─── The hooks ───────────────────────────────────────────────────────────

/// A hook NewTOS does not call (ADR-031 per-hook table) — one warning key per
/// process.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hook {
    /// m68k exception, trap or interrupt vector `n`: `Setexc(n)` for
    /// `n < 0x100`, `Mfpint`/`Xbtimer` (MFP vector `0x40 + n`), or the page-0
    /// cell `4 * n` written in super state.
    Vector(u8),
    /// `etv_timer` (`Setexc(0x100)`, `$400`).
    EtvTimer,
    /// `etv_critic` (`Setexc(0x101)`, `$404`).
    EtvCritic,
    /// `etv_term` written straight into page 0 (`$408`): NewTOS calls the
    /// `etv_term` installed through `Setexc(0x102)` only.
    EtvTermDirect,
    /// `hdv_rw` (`$476`).
    HdvRw,
    /// `hdv_bpb` (`$472`).
    HdvBpb,
    /// `hdv_mediach` (`$47E`).
    HdvMediach,
    /// `swv_vec` (`$46E`).
    SwvVec,
    /// `exec_os` (`$4FE`).
    ExecOs,
    /// `resvector` (`$42A`).
    Resvector,
    /// `dump_vec` (`$502`).
    DumpVec,
    /// `vblqueue` (`$456`).
    Vblqueue,
    /// `bell_hook` (`$5AC`, `Setexc(0x16B)`).
    BellHook,
    /// `kcl_hook` (`$5B0`, `Setexc(0x16C)`).
    KclHook,
    /// `Kbdvbase` `statvec`.
    Statvec,
    /// `Kbdvbase` `mousevec` (also what `Initmous` installs).
    Mousevec,
    /// `Kbdvbase` `clockvec`.
    Clockvec,
    /// `Kbdvbase` `joyvec`.
    Joyvec,
    /// `Kbdvbase` `midisys`.
    Midisys,
    /// `Kbdvbase` `ikbdsys`.
    Ikbdsys,
    /// A desk accessory's `kbdvec`: keys go to the innermost started program
    /// (`docs/api/xbios/22-kbdvbase.md` Teil D, D9).
    AccessoryKbdvec,
    /// VDI `vex_butv`.
    VexButv,
    /// VDI `vex_motv`.
    VexMotv,
    /// VDI `vex_curv`.
    VexCurv,
    /// VDI `vex_timv`.
    VexTimv,
}

/// The named hooks after the 256 vectors, in key order.
const NAMED: [Hook; 24] = [
    Hook::EtvTimer,
    Hook::EtvCritic,
    Hook::EtvTermDirect,
    Hook::HdvRw,
    Hook::HdvBpb,
    Hook::HdvMediach,
    Hook::SwvVec,
    Hook::ExecOs,
    Hook::Resvector,
    Hook::DumpVec,
    Hook::Vblqueue,
    Hook::BellHook,
    Hook::KclHook,
    Hook::Statvec,
    Hook::Mousevec,
    Hook::Clockvec,
    Hook::Joyvec,
    Hook::Midisys,
    Hook::Ikbdsys,
    Hook::AccessoryKbdvec,
    Hook::VexButv,
    Hook::VexMotv,
    Hook::VexCurv,
    Hook::VexTimv,
];

/// Number of warning keys: the 256 vectors, then [`NAMED`].
const NKEYS: usize = 256 + NAMED.len();
/// Words of the per-process "warned" bit set.
const WORDS: usize = NKEYS.div_ceil(64);

impl Hook {
    /// The warning key: `n` for [`Hook::Vector`], `256 +` the position in
    /// [`NAMED`] otherwise.
    fn key(self) -> usize {
        match self {
            Hook::Vector(n) => n as usize,
            named => {
                let mut i = 0;
                while NAMED[i] != named {
                    i += 1;
                }
                256 + i
            }
        }
    }

    /// Why NewTOS does not call it (ADR-031 table, "Reason").
    pub const fn reason(self) -> &'static str {
        match self {
            Hook::Vector(_) => "AArch64 has no m68k exception, trap or MFP interrupt to dispatch through it",
            Hook::EtvTimer => "NewTOS keeps time natively, no 20 ms timer hook",
            Hook::EtvCritic => "Rwabs has no critical-error retry path; the error code is returned directly",
            Hook::EtvTermDirect => "etv_term is called only when installed through Setexc(0x102)",
            Hook::HdvRw | Hook::HdvBpb | Hook::HdvMediach => {
                "GEMDOS uses the block layer directly, no floppy driver chain"
            }
            Hook::SwvVec => "there is no monitor change",
            Hook::ExecOs => "NewTOS starts its shell through its own boot path",
            Hook::Resvector => "a reset restarts the kernel",
            Hook::DumpVec => "there is no screen-dump hook",
            Hook::Vblqueue => "there is no VBL queue running program code",
            Hook::BellHook | Hook::KclHook => "there is no sound device",
            Hook::Statvec | Hook::Mousevec | Hook::Clockvec | Hook::Joyvec | Hook::Midisys | Hook::Ikbdsys => {
                "of the Kbdvbase table only kbdvec is called"
            }
            Hook::AccessoryKbdvec => {
                "keys go to the innermost started program (docs/api/xbios/22-kbdvbase.md Teil D, D9)"
            }
            Hook::VexButv | Hook::VexMotv | Hook::VexCurv | Hook::VexTimv => {
                "the VDI stores the vector and returns it, but never calls it"
            }
        }
    }
}

impl fmt::Display for Hook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Hook::Vector(n) => return write!(f, "vector 0x{:02x} (${:03X})", n, u32::from(*n) * 4),
            Hook::EtvTimer => "etv_timer",
            Hook::EtvCritic => "etv_critic",
            Hook::EtvTermDirect => "etv_term ($408, written directly)",
            Hook::HdvRw => "hdv_rw",
            Hook::HdvBpb => "hdv_bpb",
            Hook::HdvMediach => "hdv_mediach",
            Hook::SwvVec => "swv_vec",
            Hook::ExecOs => "exec_os",
            Hook::Resvector => "resvector",
            Hook::DumpVec => "dump_vec",
            Hook::Vblqueue => "vblqueue",
            Hook::BellHook => "bell_hook",
            Hook::KclHook => "kcl_hook",
            Hook::Statvec => "statvec",
            Hook::Mousevec => "mousevec",
            Hook::Clockvec => "clockvec",
            Hook::Joyvec => "joyvec",
            Hook::Midisys => "midisys",
            Hook::Ikbdsys => "ikbdsys",
            Hook::AccessoryKbdvec => "kbdvec of a desk accessory",
            Hook::VexButv => "vex_butv",
            Hook::VexMotv => "vex_motv",
            Hook::VexCurv => "vex_curv",
            Hook::VexTimv => "vex_timv",
        };
        f.write_str(name)
    }
}

/// Where a hook was seen (ADR-031 rule 5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Via {
    /// `Setexc` (BIOS 0x05).
    Setexc,
    /// A page-0 cell, compared with its default.
    Page0,
    /// A `Kbdvbase` slot, at an event that would call it.
    Kbdvbase,
    /// `Initmous` (XBIOS 0x00).
    Initmous,
    /// `Mfpint` (XBIOS 0x0D).
    Mfpint,
    /// `Xbtimer` (XBIOS 0x1F).
    Xbtimer,
    /// A VDI `vex_*` call.
    Vex,
}

impl fmt::Display for Via {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Via::Setexc => "Setexc",
            Via::Page0 => "page 0",
            Via::Kbdvbase => "Kbdvbase",
            Via::Initmous => "Initmous",
            Via::Mfpint => "Mfpint",
            Via::Xbtimer => "Xbtimer",
            Via::Vex => "the VDI",
        })
    }
}

// ─── Page-0 hook cells (ADR-031 rule 4) ──────────────────────────────────

/// A page-0 hook cell: its address, the hook, and the value the loader leaves
/// there (`loader::populate_sysvars` on a zeroed page — a host test checks
/// the two agree). The vector cells `$008`–`$40B` are not in this table: their
/// copy in page 0 is compared with the shared cell ([`check_page0`]).
#[derive(Clone, Copy, Debug)]
pub struct Page0Hook {
    /// m68k address of the LONG.
    pub addr: u16,
    /// The hook it holds.
    pub hook: Hook,
    /// Its value in a freshly loaded process.
    pub default: u32,
}

/// The named page-0 hooks EmuTOS calls (`references/emutos/tosvars.ld:33-97`;
/// callers in `docs/analysis/os-hooks-adr-proposal-2026-10-28.md` §1.1–1.2).
/// Not here: `etv_xtra` (`$40C`), `hdv_init` (`$46A`), `hdv_boot` (`$47A`),
/// `prt_stat`/`prt_vec`/`aux_stat`/`aux_vec` (`$506`–`$512`) — EmuTOS never
/// calls them (rule 1). The CPU vector cells `$008`–`$3FC` and `etv_timer`,
/// `etv_critic`, `etv_term` (`$400`–`$408`) are checked as well
/// ([`check_page0`]); `$000`/`$004` are the reset SSP/PC, no hook.
pub const PAGE0_HOOKS: [Page0Hook; 10] = [
    Page0Hook { addr: 0x42A, hook: Hook::Resvector, default: 0 },
    Page0Hook { addr: 0x456, hook: Hook::Vblqueue, default: 0 },
    Page0Hook { addr: 0x46E, hook: Hook::SwvVec, default: 0 },
    Page0Hook { addr: 0x472, hook: Hook::HdvBpb, default: 0 },
    Page0Hook { addr: 0x476, hook: Hook::HdvRw, default: 0 },
    Page0Hook { addr: 0x47E, hook: Hook::HdvMediach, default: 0 },
    Page0Hook { addr: 0x4FE, hook: Hook::ExecOs, default: 0 },
    Page0Hook { addr: 0x502, hook: Hook::DumpVec, default: 0 },
    Page0Hook { addr: 0x5AC, hook: Hook::BellHook, default: 0 },
    Page0Hook { addr: 0x5B0, hook: Hook::KclHook, default: 0 },
];

/// First CPU vector whose page-0 cell is a hook (vectors 0/1 are the reset
/// SSP and PC).
const FIRST_PAGE0_VECTOR: usize = 2;

/// The hook behind the page-0 copy of shared vector cell `num`
/// (`FIRST_PAGE0_VECTOR..=0x102`).
fn page0_vector_hook(num: usize) -> Hook {
    match num {
        0x100 => Hook::EtvTimer,
        0x101 => Hook::EtvCritic,
        0x102 => Hook::EtvTermDirect,
        n => Hook::Vector(n as u8),
    }
}

/// Compare `slot`'s page-0 hook cells with their defaults and warn about each
/// one that differs (ADR-031 rule 4: read only, for the warning; the value is
/// never called or acted on). The vector cells `$008`–`$408` are compared with
/// the shared cell they copy (`bios::vectors`, TASK/BIOS07): a `Setexc` write
/// changes both, so only a direct write of the copy differs. Called when the
/// process leaves the super state, after each trap it makes in super state,
/// when it ends in super state, and after a `Setexc` write into its page 0.
pub fn check_page0(slot: usize) {
    let Some(p0) = mmu::page0_kva(slot) else {
        return;
    };
    // SAFETY: `p0` is the kernel alias of the slot's 4-KiB page 0; every cell
    // read below lies inside it (`addr + 4 <= 0x5B4`). Unaligned-safe reads
    // (the page holds little-endian LONGs, as on every NewTOS target); single
    // CPU under the trap.
    let rd = |addr: usize| unsafe { (p0.add(addr) as *const u32).read_unaligned() };
    for h in PAGE0_HOOKS.iter() {
        let v = rd(h.addr as usize);
        if v != h.default {
            note(slot, h.hook, u64::from(v), Via::Page0);
        }
    }
    for n in FIRST_PAGE0_VECTOR..=crate::bios::vectors::ETV_TERM {
        let v = rd(4 * n);
        if Some(v) != crate::bios::vectors::shared_cell(n) {
            note(slot, page0_vector_hook(n), u64::from(v), Via::Page0);
        }
    }
}

// ─── Shared cells and their owners (ADR-031 rule 3) ──────────────────────

/// A hook cell every process shares, so it needs an owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cell {
    /// The shared `Setexc` cell `num` (`num <= 0x103`: the CPU vectors and
    /// the `etv_*` hooks; `bios::vectors`, every page-0 copy included).
    Vector(u16),
    /// VDI `vex_butv`.
    VexButv,
    /// VDI `vex_motv`.
    VexMotv,
    /// VDI `vex_curv`.
    VexCurv,
    /// VDI `vex_timv`.
    VexTimv,
}

impl Cell {
    fn get(self) -> u64 {
        match self {
            Cell::Vector(n) => crate::bios::vectors::vector(n as usize) as u64,
            vex => crate::vdi::mouse::vex_cell(vex).map_or(0, |c| c.load(Ordering::Relaxed)),
        }
    }

    fn set(self, v: u64) {
        match self {
            Cell::Vector(n) => crate::bios::vectors::set_vector(n as usize, v as i64),
            vex => {
                if let Some(c) = crate::vdi::mouse::vex_cell(vex) {
                    c.store(v, Ordering::Relaxed);
                }
            }
        }
    }
}

impl fmt::Display for Cell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Cell::Vector(n) => write!(f, "Setexc vector 0x{:03x}", n),
            Cell::VexButv => f.write_str("vex_butv"),
            Cell::VexMotv => f.write_str("vex_motv"),
            Cell::VexCurv => f.write_str("vex_curv"),
            Cell::VexTimv => f.write_str("vex_timv"),
        }
    }
}

/// One hook a process installed in a shared cell: what it wrote last and what
/// the cell held before its first write.
#[derive(Clone, Copy, Debug)]
struct Own {
    cell: Cell,
    installed: u64,
    prev: u64,
}

/// Hooks in shared cells tracked per process. Generous: TeraDesk, the only
/// ported program with such hooks, installs three (`etv_critic`, two vectors).
pub const OWN_CAP: usize = 16;

// ─── The warnings ────────────────────────────────────────────────────────

/// What a warning reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A hook NewTOS never calls was installed; `value` is the hook.
    NotServed(Hook, Via),
    /// The process ended without unhooking `Cell`, and another hook (`other`)
    /// chains on top of it, so its hook (`value`) stays installed.
    LeftInstalled(Cell),
    /// More than [`OWN_CAP`] hooks of one process: `Cell` (= `value`) has no
    /// owner record and stays installed when the process ends.
    Untracked(Cell),
}

/// One warning for the log.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Warning {
    /// The process slot.
    pub slot: usize,
    /// Its PD (user address) when the warning was made.
    pub pd: u64,
    /// What happened.
    pub kind: Kind,
    /// The hook's value.
    pub value: u64,
    /// [`Kind::LeftInstalled`]: the value now in the cell; else 0.
    pub other: u64,
}

impl fmt::Display for Warning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[hook] warning: slot {} (PD 0x{:08x}) ", self.slot, self.pd)?;
        match self.kind {
            Kind::NotServed(hook, via) => write!(
                f,
                "installed {} = 0x{:x} via {}; NewTOS never calls it: {} (ADR-031)",
                hook,
                self.value,
                via,
                hook.reason()
            ),
            Kind::LeftInstalled(cell) => write!(
                f,
                "ended without unhooking {} = 0x{:x}; another hook (0x{:x}) chains on top, \
                 so it stays installed (ADR-031 rule 3)",
                cell, self.value, self.other
            ),
            Kind::Untracked(cell) => write!(
                f,
                "installed {} = 0x{:x}; more than {} hooks of one program are not tracked, \
                 this one stays installed after the program ends (ADR-031 rule 3)",
                cell, self.value, OWN_CAP
            ),
        }
    }
}

/// Warnings waiting for the kernel's log; a full queue drops the newest and
/// counts it ([`dropped`]). Warnings are rare — once per hook and process.
const QCAP: usize = 32;

struct State {
    /// Per slot: the hooks already warned about in this tenancy.
    warned: [[u64; WORDS]; MAX_PROCESSES],
    /// Per slot: hooks in shared cells.
    own: [[Option<Own>; OWN_CAP]; MAX_PROCESSES],
    queue: [Option<Warning>; QCAP],
    head: usize,
    len: usize,
    dropped: u32,
}

struct Shared(UnsafeCell<State>);
// SAFETY: every access goes through `with`, which holds `BUSY`.
unsafe impl Sync for Shared {}

static STATE: Shared = Shared(UnsafeCell::new(State {
    warned: [[0; WORDS]; MAX_PROCESSES],
    own: [[None; OWN_CAP]; MAX_PROCESSES],
    queue: [None; QCAP],
    head: 0,
    len: 0,
    dropped: 0,
}));
static BUSY: AtomicBool = AtomicBool::new(false);
/// Fast path for [`pop`]: something is queued.
static ANY: AtomicBool = AtomicBool::new(false);

/// Run `f` on the state. Never called from an interrupt handler, so in the
/// kernel the flag is always free; host-test threads wait for it.
fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    while BUSY
        .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        core::hint::spin_loop();
    }
    // SAFETY: `BUSY` is held; nothing else touches `STATE` until it is
    // released below. `f` never calls back into this module's `with`.
    let r = f(unsafe { &mut *STATE.0.get() });
    BUSY.store(false, Ordering::Release);
    r
}

impl State {
    fn push(&mut self, w: Warning) {
        if self.len == QCAP {
            self.dropped = self.dropped.saturating_add(1);
            return;
        }
        let at = (self.head + self.len) % QCAP;
        self.queue[at] = Some(w);
        self.len += 1;
        ANY.store(true, Ordering::Release);
    }
}

/// Warn that process `slot` installed `hook` = `value`, seen via `via` —
/// once per hook and process (ADR-031 rule 1). Out-of-range slots are
/// ignored: a hook the kernel itself installs is NewTOS's own.
pub fn note(slot: usize, hook: Hook, value: u64, via: Via) {
    if slot >= MAX_PROCESSES {
        return;
    }
    let pd = pd_of(slot) as u64;
    let key = hook.key();
    with(|s| {
        let word = &mut s.warned[slot][key / 64];
        let bit = 1u64 << (key % 64);
        if *word & bit != 0 {
            return;
        }
        *word |= bit;
        s.push(Warning { slot, pd, kind: Kind::NotServed(hook, via), value, other: 0 });
    });
}

/// [`note`] for the running process; nothing when no process runs (an
/// in-kernel caller).
pub fn note_running(hook: Hook, value: u64, via: Via) {
    if let Some(slot) = slot_for(super::current()) {
        note(slot, hook, value, via);
    }
}

/// The oldest waiting warning, for the kernel's log.
pub fn pop() -> Option<Warning> {
    if !ANY.load(Ordering::Acquire) {
        return None;
    }
    with(|s| {
        if s.len == 0 {
            ANY.store(false, Ordering::Release);
            return None;
        }
        let w = s.queue[s.head].take();
        s.head = (s.head + 1) % QCAP;
        s.len -= 1;
        if s.len == 0 {
            ANY.store(false, Ordering::Release);
        }
        w
    })
}

/// Warnings lost to a full queue since the last call; resets the count.
pub fn dropped() -> u32 {
    with(|s| core::mem::take(&mut s.dropped))
}

/// Record that `slot` wrote `new` into the shared `cell`, which held `old`
/// (ADR-031 rule 3). The first write makes the process the owner and keeps
/// `old` as the value to go back to; a later write of exactly that value is
/// the process unhooking itself. `None` (no process: the kernel) owns
/// nothing.
pub fn record_install(slot: Option<usize>, cell: Cell, old: u64, new: u64) {
    let Some(slot) = slot.filter(|&s| s < MAX_PROCESSES) else {
        return;
    };
    let pd = pd_of(slot) as u64;
    with(|s| {
        let own = &mut s.own[slot];
        if let Some(i) = own.iter().position(|o| o.is_some_and(|o| o.cell == cell)) {
            let rec = own[i].as_mut().expect("position found it");
            if new == rec.prev {
                own[i] = None;
            } else {
                rec.installed = new;
            }
            return;
        }
        if new == old {
            return;
        }
        match own.iter().position(Option::is_none) {
            Some(i) => own[i] = Some(Own { cell, installed: new, prev: old }),
            None => s.push(Warning { slot, pd, kind: Kind::Untracked(cell), value: new, other: 0 }),
        }
    });
}

/// Process `slot` ends (`Pterm`, `Ptermres`, a kill): settle the hooks it
/// installed in shared cells (ADR-031 rule 3). `resident` (`Ptermres`): the
/// hooks stay, the code they point at stays resident. Otherwise a cell that
/// still holds the process's hook gets its previous value back; a cell another
/// hook has been chained onto keeps it, with a warning; a cell the process
/// already unhooked is left alone.
pub fn on_terminate(slot: usize, resident: bool) {
    if slot >= MAX_PROCESSES {
        return;
    }
    let pd = pd_of(slot) as u64;
    with(|s| {
        for i in 0..OWN_CAP {
            let Some(rec) = s.own[slot][i].take() else {
                continue;
            };
            if resident {
                continue;
            }
            let now = rec.cell.get();
            if now == rec.installed {
                rec.cell.set(rec.prev);
            } else if now != rec.prev {
                s.push(Warning { slot, pd, kind: Kind::LeftInstalled(rec.cell), value: rec.installed, other: now });
            }
        }
    });
}

/// The slot is freed (`free_slot`): its next tenant warns afresh and owns
/// nothing. Warnings already queued keep their slot number and PD.
pub fn release_slot(slot: usize) {
    if slot >= MAX_PROCESSES {
        return;
    }
    with(|s| {
        s.warned[slot] = [0; WORDS];
        s.own[slot] = [None; OWN_CAP];
    });
}

/// The checks the kernel runs after every trap: the `Kbdvbase` slots an
/// input event since the last trap would have called
/// (`xbios::kbdvecs::check_events`), and the page-0 cells of a process that
/// is in super state (it may have hooked a cell the call just used, e.g.
/// TeraDesk's `force_mediach`, which restores `hdv_*` before it leaves).
pub fn after_trap() {
    crate::xbios::kbdvecs::check_events();
    if let Some(slot) = slot_for(super::current()) {
        if super::superstate::is_super(slot) {
            check_page0(slot);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_distinct_and_fit_the_bit_set() {
        let mut seen = [false; NKEYS];
        for n in 0..=255u8 {
            let k = Hook::Vector(n).key();
            assert!(!seen[k]);
            seen[k] = true;
        }
        for h in NAMED {
            let k = h.key();
            assert!(k < WORDS * 64 && !seen[k], "{:?}", h);
            seen[k] = true;
        }
    }

    #[test]
    fn page0_hooks_are_not_the_cells_emutos_never_calls() {
        // etv_xtra, hdv_init, hdv_boot, prt_stat, prt_vec, aux_stat, aux_vec.
        for silent in [0x40C, 0x46A, 0x47A, 0x506, 0x50A, 0x50E, 0x512] {
            assert!(PAGE0_HOOKS.iter().all(|h| h.addr != silent), "${:X}", silent);
        }
    }

    #[test]
    fn a_warning_names_hook_value_detection_point_and_adr() {
        let w = Warning {
            slot: 3,
            pd: 0x0109_7000,
            kind: Kind::NotServed(Hook::HdvMediach, Via::Page0),
            value: 0x0123_4560,
            other: 0,
        };
        let s = format!("{}", w);
        assert!(s.starts_with("[hook] warning: slot 3 (PD 0x01097000) installed hdv_mediach = 0x1234560 via page 0;"));
        assert!(s.ends_with("(ADR-031)"));
        let v = format!("{}", Hook::Vector(0x45));
        assert_eq!(v, "vector 0x45 ($114)");
    }
}
