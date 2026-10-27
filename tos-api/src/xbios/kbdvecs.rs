//! The IKBD vector table (`Kbdvbase`, XBIOS 0x22) and the `kbdvec` hook —
//! KEYS02.
//!
//! Spec: `docs/api/xbios/22-kbdvbase.md` (approved 2026-10-04, decisions
//! D1–D8). EmuTOS reference: `bios/aciavecs.S:126-141` (layout),
//! `:146-223` (defaults), `:407-414` (`ikbdraw` → `kbdvec`), `:372-376`
//! (`_kbdvec` → `kbd_int`), `bios/ikbd.c:451-472` (`ikbdsys`-change repeat
//! stop).
//!
//! # Where the table lives (D1, moved by MEM12)
//!
//! Per process, at the start of its **private read-write page**
//! ([`KBDVECS_AREA_VA`](crate::bdos::process::loader::KBDVECS_AREA_VA),
//! `docs/api/xbios/22-kbdvbase.md` Teil D "D1-neu"), written by the loader
//! ([`write_table`]). It used to be in page 0, which is EL0 read-only since
//! MEM12; a program installs its hook by writing a slot, so the table needs a
//! page the program may write. An EL0 handler address means nothing in
//! another address space, so the table belongs to the address space it
//! points into, and it disappears with it (D8). The two default stubs are in
//! the shared stub page ([`write_stubs`]). `Kbdvbase` returns
//! the EL0 VA of the `midivec` slot; `kbdvec` is the slot directly before it
//! (8 bytes, LP64), `mousexvec` before that:
//!
//! ```text
//!  +0x00 mousexvec   +0x08 kbdvec    +0x10 midivec   <- Kbdvbase()
//!  +0x18 vkbderr     +0x20 vmiderr   +0x28 statvec   +0x30 mousevec
//!  +0x38 clockvec    +0x40 joyvec    +0x48 midisys   +0x50 ikbdsys
//!  +0x58 drvstat (byte, always 0, D5)                +0x59 kbdlength (byte, 0)
//! ```
//!
//! # Defaults (B3)
//!
//! Every slot holds a valid EL0 entry point, so a program can read a slot,
//! save it and later branch to it (chaining, A6):
//!
//! * [`RET_STUB_VA`] — a single `ret`, EmuTOS `just_rts`: `mousexvec`,
//!   `midivec`, `vkbderr`, `vmiderr`, `statvec`, `mousevec`, `clockvec`,
//!   `joyvec`, `midisys`, `ikbdsys`;
//! * [`KBDVEC_DEFAULT_VA`] — `kbdvec`, EmuTOS `_kbdvec` → `kbd_int`: it issues
//!   `Ssystem(S_NEWTOS_KBDVEC, scancode, 0)` (GEMDOS 0x154, mode 2002, D6),
//!   which runs the kernel's key decoder (`Host::kbd_int`) for the byte.
//!
//! # What is called (D7 minimum), and whose table (D9)
//!
//! Only `kbdvec`. The kernel's IKBD decoder hands every key byte (`< 0xF6`,
//! make and break) to [`route_key`]. The table consulted is the **innermost
//! started process's** — EmuTOS's `run` without the accessories
//! (`process::emutos_run`, D9, MEM12), whichever process has the CPU when the
//! byte is polled; an accessory's table is never consulted, and a hook an
//! accessory installs is reported once ([`accessory_hook_unwarned`]). While
//! that table's `kbdvec` is still [`KBDVEC_DEFAULT_VA`] — or no started
//! process lives — the decoder runs directly, exactly as before KEYS02.
//! Otherwise the byte goes into that process's 64-entry ring (D3/D3a, overflow drops the newest byte) and the
//! kernel delivers it to the hook as an EL0 upcall (`x0` = scancode, `x1` = 0,
//! D2/D2a) at the next syscall return, one byte per upcall, in order. The other
//! eight slots and `mousexvec` exist with their defaults, but a program hook
//! installed there is **not called** — a documented gap (`docs/STATUS.md`
//! "Blocked"), not a hidden one. `ikbdsys` is never called either (NewTOS has
//! no ACIA interrupt, D4); only its value is read, for the repeat stop.

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};

use crate::bdos::process::loader::{
    write_stub_code, KBDVECS_AREA_SIZE, KBDVECS_AREA_VA, KBDVEC_STUB_AREA_SIZE, KBDVEC_STUB_AREA_VA,
    USER_VA_BASE,
};
use crate::bdos::process::mmu::{PRIV_PAGE_VA, STUB_PAGE_VA};
use crate::bdos::process::{current, slot_for, MAX_PROCESSES, PROCESS_TABLE};

// ─── Layout ──────────────────────────────────────────────────────────────

/// Offset of `mousexvec` inside the table (`aciavecs.S:126-128`).
pub const OFF_MOUSEXVEC: u64 = 0x00;
/// Offset of `kbdvec` (`aciavecs.S:129`, "must be right before _kbdvecs").
pub const OFF_KBDVEC: u64 = 0x08;
/// Offset of `midivec` — the address `Kbdvbase` returns (`aciavecs.S:131`).
pub const OFF_KBDVBASE: u64 = 0x10;
/// Offset of `ikbdsys`, the ninth slot (`aciavecs.S:139`).
pub const OFF_IKBDSYS: u64 = OFF_KBDVBASE + 8 * 8;
/// Offset of the `drvstat` byte (`ikbdstate`, `aciavecs.S:140`; D5).
pub const OFF_DRVSTAT: u64 = OFF_KBDVBASE + 9 * 8;
/// Offset of the `kbdlength` byte (`aciavecs.S:141`).
pub const OFF_KBDLENGTH: u64 = OFF_DRVSTAT + 1;

const _: () = assert!(OFF_KBDLENGTH < KBDVECS_AREA_SIZE);

/// EL0 VA of the `ret` stub (EmuTOS `just_rts`, `aciavecs.S:377`).
pub const RET_STUB_VA: u64 = USER_VA_BASE + KBDVEC_STUB_AREA_VA;
/// EL0 VA of the `kbdvec_default` stub (EmuTOS `_kbdvec`, `aciavecs.S:372-376`).
pub const KBDVEC_DEFAULT_VA: u64 = USER_VA_BASE + KBDVEC_STUB_AREA_VA + 0x08;

/// `Ssystem` mode the `kbdvec_default` stub issues — NewTOS-own block 2000+
/// (`docs/api/gemdos/154-ssystem.md`; decision D6).
pub const S_NEWTOS_KBDVEC: u16 = 2002;

/// `ret` (encoding checked with clang/llvm-objdump).
const RET_STUB_CODE: [u32; 1] = [0xD65F_03C0];

/// The `kbdvec_default` stub. Entered like any `kbdvec` (AAPCS64, `x0` =
/// scancode, `x1` = 0), it calls `Ssystem(2002, scancode, 0)` and returns to
/// its caller — the upcall trampoline, or a program hook that chained to it.
///
/// ```text
///   aa0003e1  mov x1, x0        ; arg1 = scancode
///   d280fa40  mov x0, #2002     ; mode S_NEWTOS_KBDVEC
///   d2800002  mov x2, #0        ; arg2 = 0
///   d2802a88  mov x8, #0x154    ; Ssystem
///   d4000021  svc #1
///   d65f03c0  ret
/// ```
///
/// Encodings checked with clang (`-target aarch64-unknown-none-elf`) +
/// llvm-objdump.
const KBDVEC_DEFAULT_CODE: [u32; 6] = [
    0xAA00_03E1,
    0xD280_FA40,
    0xD280_0002,
    0xD280_2A88,
    0xD400_0021,
    0xD65F_03C0,
];

// The stubs fit their area; the 2002 baked into the `mov` is S_NEWTOS_KBDVEC.
const _: () = assert!(
    KBDVEC_DEFAULT_VA - USER_VA_BASE + 4 * KBDVEC_DEFAULT_CODE.len() as u64
        <= KBDVEC_STUB_AREA_VA + KBDVEC_STUB_AREA_SIZE
);
const _: () = assert!(RET_STUB_VA + 4 <= KBDVEC_DEFAULT_VA);
const _: () = assert!((KBDVEC_DEFAULT_CODE[1] >> 5) & 0xFFFF == S_NEWTOS_KBDVEC as u32);

/// The default value of slot `off` (every slot but `kbdvec` is `just_rts`).
const fn default_of(off: u64) -> u64 {
    if off == OFF_KBDVEC {
        KBDVEC_DEFAULT_VA
    } else {
        RET_STUB_VA
    }
}

/// Byte offset of the table in the private read-write page.
const TABLE_OFF: usize = (KBDVECS_AREA_VA - PRIV_PAGE_VA) as usize;

/// Write the table with its defaults into a freshly loaded process's private
/// read-write page `page` (the kernel alias of the whole page; EmuTOS
/// `init_acia_vecs`, `aciavecs.S:146-223`).
pub fn write_table(page: &mut [u8]) {
    let base = TABLE_OFF;
    page[base..base + KBDVECS_AREA_SIZE as usize].fill(0);
    let mut off = OFF_MOUSEXVEC;
    while off <= OFF_IKBDSYS {
        let o = base + off as usize;
        page[o..o + 8].copy_from_slice(&default_of(off).to_le_bytes());
        off += 8;
    }
    // `drvstat` / `kbdlength` stay 0 (D5): a packet is never observable
    // half-received, the HID translator feeds whole reports.
}

/// Write the two default stubs into the shared stub page `page` (the kernel
/// alias of the whole page) — once, by `loader::ensure_stub_page`.
pub fn write_stubs(page: &mut [u8]) {
    let stub = (KBDVEC_STUB_AREA_VA - STUB_PAGE_VA) as usize;
    page[stub..stub + KBDVEC_STUB_AREA_SIZE as usize].fill(0);
    write_stub_code(page, RET_STUB_VA, &RET_STUB_CODE);
    write_stub_code(page, KBDVEC_DEFAULT_VA, &KBDVEC_DEFAULT_CODE);
}

// ─── The table with no process running ───────────────────────────────────

/// The table an EL1 caller (kernel code such as the boot path — `RUN`
/// names no process) gets from `Kbdvbase`: same layout, same default values.
/// Nothing ever calls through it — with no process running the decoder always
/// takes the default path — so it only answers the pointer (the same "kernel
/// VA when no process runs" rule as `Keytbl`, `xbios/keytbl.rs`).
#[repr(C, align(8))]
struct KernelTable(UnsafeCell<[u64; (KBDVECS_AREA_SIZE / 8) as usize]>);
// SAFETY: single boot CPU, cooperative trap path; written once, then only read.
unsafe impl Sync for KernelTable {}
static KERNEL_TABLE: KernelTable = KernelTable(UnsafeCell::new([
    RET_STUB_VA,       // mousexvec
    KBDVEC_DEFAULT_VA, // kbdvec
    RET_STUB_VA,       // midivec
    RET_STUB_VA,       // vkbderr
    RET_STUB_VA,       // vmiderr
    RET_STUB_VA,       // statvec
    RET_STUB_VA,       // mousevec
    RET_STUB_VA,       // clockvec
    RET_STUB_VA,       // joyvec
    RET_STUB_VA,       // midisys
    RET_STUB_VA,       // ikbdsys
    0,                 // drvstat, kbdlength, padding
]));

/// `Kbdvbase()` (XBIOS 0x22): the running process's table, as its own EL0
/// VA of the `midivec` slot (full 64 bits in `x0`); the kernel table's address
/// when no process runs. Stable for the life of the process.
pub fn kbdvbase() -> i64 {
    if slot_for(current()).is_some() {
        (USER_VA_BASE + KBDVECS_AREA_VA + OFF_KBDVBASE) as i64
    } else {
        KERNEL_TABLE.0.get() as u64 as i64 + OFF_KBDVBASE as i64
    }
}

/// Read slot `off` of process `slot`'s table through the kernel alias of its
/// private read-write page (`mmu::priv_page_kva`). `None` for a slot with no
/// loaded image.
fn read_slot(slot: usize, off: u64) -> Option<u64> {
    if PROCESS_TABLE.get(slot)?.basepage_kva.load(Ordering::Acquire).is_null() {
        return None;
    }
    let page = crate::bdos::process::mmu::priv_page_kva(slot)? as u64;
    let va = page + TABLE_OFF as u64 + off;
    // SAFETY: `page` is the kernel alias of the process's private page (a
    // static kernel page); the table lies inside it (loader asserts) and the
    // slot is 8-aligned. The program may rewrite it at any time, hence the
    // volatile read.
    Some(unsafe { (va as *const u64).read_volatile() })
}

/// The `kbdvec` slot of process `slot` (what an upcall for it jumps to).
pub fn kbdvec_of(slot: usize) -> Option<u64> {
    read_slot(slot, OFF_KBDVEC)
}

/// The `ikbdsys` value in force for the key-repeat stop (EmuTOS
/// `kbdvecs.ikbdsys`, `bios/ikbd.c:453,507,903`): the table that receives
/// the input ([`hook_owner`], D9), or the default [`RET_STUB_VA`] when no
/// started process lives — every table starts with that value, so moving
/// between the desktop and an unhooked program does not count as a change.
pub fn current_ikbdsys() -> u64 {
    hook_owner()
        .and_then(|s| read_slot(s, OFF_IKBDSYS))
        .unwrap_or(RET_STUB_VA)
}

// ─── The per-process byte ring (D3/D3a) ──────────────────────────────────

/// Ring capacity per process (D3a).
pub const RING_LEN: usize = 64;

struct Ring {
    buf: [AtomicU8; RING_LEN],
    /// Index of the oldest byte.
    head: AtomicUsize,
    /// Number of bytes queued.
    len: AtomicUsize,
}

impl Ring {
    const fn new() -> Self {
        Self {
            buf: [const { AtomicU8::new(0) }; RING_LEN],
            head: AtomicUsize::new(0),
            len: AtomicUsize::new(0),
        }
    }
}

/// One ring per process slot. Producer is the input pump (`route_key`),
/// consumer the trap return path (`pop`); both run on the single CPU in the
/// synchronous trap context, never in an interrupt, so they never overlap.
/// (INPUT01's timer-interrupt pump decodes pointer motion only; key bytes
/// stay held for the trap-side pump — `kernel::input_gate`.)
static RINGS: [Ring; MAX_PROCESSES] = [const { Ring::new() }; MAX_PROCESSES];

/// Append `byte` to `slot`'s ring. A full ring drops the **newest** byte
/// (D3a) — the bytes already queued keep their order. Returns whether the
/// byte was stored.
pub fn push(slot: usize, byte: u8) -> bool {
    let Some(r) = RINGS.get(slot) else {
        return false;
    };
    let len = r.len.load(Ordering::Acquire);
    if len >= RING_LEN {
        return false;
    }
    let at = (r.head.load(Ordering::Acquire) + len) % RING_LEN;
    r.buf[at].store(byte, Ordering::Release);
    r.len.store(len + 1, Ordering::Release);
    true
}

/// Take the oldest byte of `slot`'s ring.
pub fn pop(slot: usize) -> Option<u8> {
    let r = RINGS.get(slot)?;
    let len = r.len.load(Ordering::Acquire);
    if len == 0 {
        return None;
    }
    let head = r.head.load(Ordering::Acquire);
    let b = r.buf[head].load(Ordering::Acquire);
    r.head.store((head + 1) % RING_LEN, Ordering::Release);
    r.len.store(len - 1, Ordering::Release);
    Some(b)
}

/// Bytes waiting in `slot`'s ring.
pub fn pending(slot: usize) -> usize {
    RINGS.get(slot).map_or(0, |r| r.len.load(Ordering::Acquire))
}

/// `true` when the running process has hook bytes waiting — the condition
/// under which a blocking wait must give way so they can be delivered.
pub fn pending_for_running() -> bool {
    slot_for(current()).is_some_and(|s| pending(s) != 0)
}

/// Drop everything queued for `slot` — process exit (`free_slot`, D8): the
/// hook and its address space are gone, the bytes have nowhere to go.
pub fn release_slot(slot: usize) {
    if let Some(r) = RINGS.get(slot) {
        r.len.store(0, Ordering::Release);
        r.head.store(0, Ordering::Release);
    }
    if let Some(w) = ACC_WARNED.get(slot) {
        w.store(false, Ordering::Release);
    }
}

/// One flag per slot: the warning of [`accessory_hook_unwarned`] was given.
static ACC_WARNED: [AtomicBool; MAX_PROCESSES] = [const { AtomicBool::new(false) }; MAX_PROCESSES];

/// `true` exactly once per accessory tenancy of `slot`: when `slot` is a desk
/// accessory whose `kbdvec` slot no longer holds the default — it installed a
/// hook that, by D9, is never called (on EmuTOS an accessory's hook is
/// global). The kernel then logs a warning, so the deviation never bites
/// silently (user decision 2026-10-27, TASK/MEM12).
pub fn accessory_hook_unwarned(slot: usize) -> bool {
    if !crate::bdos::process::is_accessory(slot) {
        return false;
    }
    match kbdvec_of(slot) {
        Some(v) if v != KBDVEC_DEFAULT_VA => {}
        _ => return false,
    }
    ACC_WARNED.get(slot).is_some_and(|w| !w.swap(true, Ordering::AcqRel))
}

/// Decide where the key byte `byte` (`< 0xF6`, bit 7 = break) goes — EmuTOS
/// `ikbdraw`'s jump through `kbdvec` (`aciavecs.S:407-414`).
///
/// The table is the innermost started process's ([`hook_owner`], D9), not
/// necessarily the running one's. Returns `false` when the caller must run
/// the default decoder itself: no started process lives, or its `kbdvec` is
/// still [`KBDVEC_DEFAULT_VA`] (the fast path — normal programs see no
/// change). Returns `true` when the byte belongs to the program's hook: it was
/// queued for upcall delivery at that process's next return to EL0 (or
/// dropped on a full ring, D3a); a hook owner waiting in a blocking call is
/// woken for it.
pub fn route_key(byte: u8) -> bool {
    let Some(slot) = hook_owner() else {
        return false;
    };
    match kbdvec_of(slot) {
        Some(v) if v != KBDVEC_DEFAULT_VA => {
            push(slot, byte);
            if slot_for(current()) != Some(slot) {
                crate::bdos::process::sched::wake_for_upcall(slot);
            }
            true
        }
        _ => false,
    }
}

/// Whose `Kbdvbase` table an input event goes to (D9, MEM12): the innermost
/// started process — EmuTOS's `run` without the accessories
/// (`process::emutos_run`, which names a stand-in PD, no slot, between
/// programs). `None` when no started process lives.
pub fn hook_owner() -> Option<usize> {
    slot_for(crate::bdos::process::emutos_run())
}

// ─── Restart of a blocking read (delivery while waiting) ─────────────────

/// Set by a blocking key read that gave way because hook bytes are waiting
/// (see `Host::console_read_key_restartable`): the trap handler must discard
/// the call's result, rewind the `svc`, and deliver the hook upcall first.
static RESTART: AtomicBool = AtomicBool::new(false);

/// Ask the trap handler to restart the current call after the hook ran.
pub fn request_restart() {
    RESTART.store(true, Ordering::Release);
}

/// Consume a [`request_restart`].
pub fn take_restart() -> bool {
    RESTART.swap(false, Ordering::AcqRel)
}
