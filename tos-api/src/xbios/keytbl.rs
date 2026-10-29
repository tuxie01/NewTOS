//! XBIOS `Keytbl` (0x10) and `Bioskeys` (0x18) — the keyboard translation
//! tables.
//!
//! Spec: `docs/api/xbios/10-keytbl.md`, `docs/api/xbios/18-bioskeys.md`.
//! EmuTOS source: `bios/ikbd.c:122-134` (`keytbl`), `bios/ikbd.c:136-140`
//! (`bioskeys`), `bios/ikbd.h:57-86` (`struct keytbl`).
//!
//! ```c
//! /* bios/ikbd.c:122-134 */
//! LONG keytbl(const UBYTE* norm, const UBYTE* shft, const UBYTE* caps)
//! {
//!     if (norm != (UBYTE*)-1) current_keytbl.norm = norm;
//!     if (shft != (UBYTE*)-1) current_keytbl.shft = shft;
//!     if (caps != (UBYTE*)-1) current_keytbl.caps = caps;
//!     return (LONG) &current_keytbl;
//! }
//! ```
//!
//! ## Where `current_keytbl` lives
//!
//! EmuTOS returns the address of a BIOS BSS static, and its callers
//! dereference it straight away — the EmuTOS desktop reads
//! `keytab->shift[HIBYTE(thechar)]` to resolve an Alt-letter drive shortcut
//! (`desk/deskmain.c:1138-1145`). On NewTOS a kernel address is not
//! dereferenceable from EL0, so the struct and the six tables it points at are
//! **materialised into the calling process's reserved TPA-low region**
//! ([`crate::bdos::process::loader::KEYTBL_AREA_VA`]) and the returned pointer
//! is the process's own user VA (page 0 is per process, ARCH03 plan E3) —
//! "user address while a process runs, kernel VA otherwise". With no process
//! running (EL1 kernel code such as the boot path) the area is a kernel
//! static and its kernel VA is returned.
//!
//! The area is rewritten on **every** call, so the returned image always
//! reflects the layout currently in force. Its inner layout:
//!
//! ```text
//!  +0x000  struct keytbl        (58 bytes LP64 / 30 bytes ILP32)
//!  +0x040  norm[128]
//!  +0x0C0  shft[128]
//!  +0x140  caps[128]
//!  +0x1C0  altnorm[33]          zero-terminated (scancode, ascii) pairs
//!  +0x1E8  altshft[33]
//!  +0x210  altcaps[33]
//! ```
//!
//! ## The set half
//!
//! EmuTOS stores the *pointer* it was handed; its ACIA ISR then reads the
//! caller's memory directly. NewTOS's translator owns a RAM layout
//! (`kernel/src/keymap.rs`), so an installed table is **taken over by value**
//! ([`crate::host::Host::set_key_table`]) — a non-sentinel argument really does
//! change what the keyboard produces, it is not recorded and ignored. The
//! caller's pointer is *also* remembered ([`OVERRIDE`]) so that a later
//! `Keytbl(-1,-1,-1)` reports back exactly the pointer that was installed, as
//! EmuTOS does. The one residual difference: a program that edits its table
//! bytes *after* installing it does not change the translation until it calls
//! `Keytbl` again — NewTOS has no ISR reading the caller's address space.

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::bdos::process::loader::{KEYTBL_AREA_SIZE, KEYTBL_AREA_VA, USER_VA_BASE};
use crate::bdos::process::{current, current_abi, slot_for, Abi};
use crate::host::{host, KeyTable, KeyTables, ALT_TABLE_LEN};
use crate::trap::TrapArgs;

/// Offsets of the six tables inside the area (see the module doc).
const OFF_NORM: u64 = 0x40;
const OFF_SHFT: u64 = 0xC0;
const OFF_CAPS: u64 = 0x140;
const OFF_ALTNORM: u64 = 0x1C0;
const OFF_ALTSHFT: u64 = 0x1E8;
const OFF_ALTCAPS: u64 = 0x210;

// The area must hold everything the writer lays into it.
const _: () = assert!(OFF_ALTCAPS + ALT_TABLE_LEN as u64 <= KEYTBL_AREA_SIZE);

/// "No caller-installed table" — `u64::MAX` can never be a stored pointer
/// because that value *is* the `(UBYTE *)-1` "keep the current one" sentinel
/// and is therefore never assigned (`bios/ikbd.c:124-132`).
const NO_OVERRIDE: u64 = u64::MAX;

/// The pointers a caller installed through `Keytbl`, in `norm`/`shft`/`caps`
/// order. [`NO_OVERRIDE`] means "still the built-in layout", in which case the
/// emitted struct points at this area's own copy of the table.
static OVERRIDE: [AtomicU64; 3] = [
    AtomicU64::new(NO_OVERRIDE),
    AtomicU64::new(NO_OVERRIDE),
    AtomicU64::new(NO_OVERRIDE),
];

/// The `current_keytbl` image used when no EL0 process is running — EL1
/// kernel code such as the boot path, which can dereference a kernel VA.
#[repr(align(8))]
struct KernelArea(UnsafeCell<[u8; KEYTBL_AREA_SIZE as usize]>);
// SAFETY: single boot CPU, cooperative (non-preemptive) trap path — the same
// discipline as `xbios::kbdvecs::KERNEL_TABLE`.
unsafe impl Sync for KernelArea {}
static KERNEL_AREA: KernelArea = KernelArea(UnsafeCell::new([0; KEYTBL_AREA_SIZE as usize]));

/// Whether an argument is the `(UBYTE *)-1` "keep the current table" sentinel
/// (`bios/ikbd.c:124`). All-ones at either caller pointer width: an LP64
/// caller passes `0xFFFF_FFFF_FFFF_FFFF`, an ILP32 one the zero-extended
/// `0x0000_0000_FFFF_FFFF` (a sign-extended one lands on the LP64 value).
fn is_keep(v: u64) -> bool {
    v == u64::MAX || v == u32::MAX as u64
}

/// Write `bytes` at `dst` (a kernel-addressable byte address).
///
/// # Safety
/// `dst .. dst + bytes.len()` must be writable kernel-addressable memory.
unsafe fn put(dst: u64, bytes: &[u8]) {
    // SAFETY: forwarded to this function's contract; the source is a slice we
    // own, and the two ranges never overlap (the area is not a `KeyTables`).
    unsafe { core::ptr::copy_nonoverlapping(bytes.as_ptr(), dst as *mut u8, bytes.len()) };
}

/// Write one pointer-sized field of the `struct keytbl` at the caller's ABI
/// width: 8 bytes LP64, 4 bytes ILP32 (the m68k `const UBYTE *`).
///
/// # Safety
/// `dst` must be writable kernel-addressable memory of the field's width.
unsafe fn put_ptr(dst: u64, v: u64, abi: Abi) {
    // SAFETY: forwarded to this function's contract. The area is 8-aligned and
    // every pointer field offset is a multiple of the field width, but write
    // unaligned anyway so the layout constants stay free to move.
    unsafe {
        match abi {
            Abi::Lp64 => (dst as *mut u64).write_unaligned(v),
            Abi::Ilp32 => (dst as *mut u32).write_unaligned(v as u32),
        }
    }
}

/// Lay the `struct keytbl` and the six tables it points at into the area at
/// `write_base`, with the pointer fields carrying `emit_base`-relative
/// addresses. Returns nothing; the caller returns `emit_base`.
///
/// # Safety
/// `write_base .. write_base + KEYTBL_AREA_SIZE` must be writable
/// kernel-addressable memory.
unsafe fn write_area(write_base: u64, emit_base: u64, t: &KeyTables, abi: Abi) {
    // SAFETY: forwarded to this function's contract; every offset below is
    // inside `KEYTBL_AREA_SIZE` (checked by the const assert above).
    unsafe {
        put(write_base + OFF_NORM, &t.norm);
        put(write_base + OFF_SHFT, &t.shft);
        put(write_base + OFF_CAPS, &t.caps);
        put(write_base + OFF_ALTNORM, &t.altnorm);
        put(write_base + OFF_ALTSHFT, &t.altshft);
        put(write_base + OFF_ALTCAPS, &t.altcaps);

        // `struct keytbl` (`bios/ikbd.h:57-86`): norm, shft, caps, altnorm,
        // altshft, altcaps, dead, features. A caller-installed table is
        // reported with the pointer the caller gave (EmuTOS stores it
        // verbatim); otherwise the address of this area's own copy.
        let w = match abi {
            Abi::Lp64 => 8,
            Abi::Ilp32 => 4,
        };
        let primary = [OFF_NORM, OFF_SHFT, OFF_CAPS];
        for (i, off) in primary.iter().enumerate() {
            let ovr = OVERRIDE[i].load(Ordering::Relaxed);
            let p = if ovr == NO_OVERRIDE { emit_base + off } else { ovr };
            put_ptr(write_base + i as u64 * w, p, abi);
        }
        put_ptr(write_base + 3 * w, emit_base + OFF_ALTNORM, abi);
        put_ptr(write_base + 4 * w, emit_base + OFF_ALTSHFT, abi);
        put_ptr(write_base + 5 * w, emit_base + OFF_ALTCAPS, abi);
        // `dead` — NewTOS' translator has no dead-key tables, exactly like
        // EmuTOS' US and German layouts (`keyb_us.h`/`keyb_de.h` both end
        // `..., NULL, 0`).
        put_ptr(write_base + 6 * w, 0, abi);
        // `features` — a WORD in either ABI; 0 = no `DUAL_KEYBOARD`.
        (((write_base + 7 * w) as *mut u16).write_unaligned(0));
    }
}

/// `Keytbl` (0x10) — install none, some or all of the three primary
/// scancode→ASCII tables and return the address of the `struct keytbl`
/// describing the layout now in force (`bios/ikbd.c:122-134`).
///
/// Returns 0 when the board has no keyboard translator to describe (no
/// [`crate::host::Host::key_tables`]); there is then no `current_keytbl` to
/// point at, and a caller must not dereference the result.
pub fn keytbl(args: &TrapArgs) -> i64 {
    let Some(h) = host() else {
        return 0;
    };

    // ── set half (`bios/ikbd.c:124-132`): a non-sentinel argument installs
    //    that table. NewTOS takes the 128 bytes over by value (see module doc)
    //    and remembers the caller's pointer for the struct it reports back.
    const WHICH: [KeyTable; 3] = [KeyTable::Norm, KeyTable::Shft, KeyTable::Caps];
    for (i, which) in WHICH.iter().enumerate() {
        let p = args.arg_u64(i);
        if is_keep(p) {
            continue;
        }
        let mut tab = [0u8; 128];
        // `p` is the caller's table pointer, which the `Keytbl` contract
        // requires to address 128 readable bytes ("Each non-sentinel pointer
        // must reference a 128-byte scancode→ASCII table",
        // `docs/api/xbios/10-keytbl.md`). Read in the caller's space (ARCH03
        // 3a, `crate::uaccess`): a table the program cannot read ends the
        // call and the program, with the tables before it already installed
        // (EmuTOS would take the bus error later, when a key is decoded).
        if args.space.copy_from(&mut tab, p).is_err() {
            return i64::from(crate::uaccess::FAULTED);
        }
        h.set_key_table(*which, &tab);
        OVERRIDE[i].store(p, Ordering::Relaxed);
    }

    publish(h)
}

/// `Bioskeys` (0x18) — restore the power-up keyboard tables
/// (`bios/ikbd.c:136-140`: `current_keytbl = *get_keytbl();`).
///
/// EmuTOS copies the whole country-default `struct keytbl` over
/// `current_keytbl`, so every pointer a program installed through `Keytbl` is
/// gone and all eight fields are the defaults again. NewTOS forgets the
/// remembered caller pointers ([`OVERRIDE`]), so the reported `norm`/`shft`/
/// `caps` point at the area's own copies again, and has the translator put
/// its power-up layout back ([`crate::host::Host::reset_key_tables`]) — the
/// translation, not only the report, returns to the default.
///
/// **Return value.** `Bioskeys` is `void`; on EmuTOS `d0` keeps what
/// `get_keytbl` returned, the address of the country-default `struct keytbl`
/// (pinned build `obj/ikbd.o`: `jsr _get_keytbl; moveal %d0,%a0`, then only
/// `%a0` is used; measured `0x00E06E8C`, ROM). NewTOS has no separate default
/// struct; it answers the address of the `struct keytbl` it has just
/// rewritten with the default tables — the same image `Keytbl(-1,-1,-1)` now
/// returns — or 0 on a board without a keyboard translator, as `Keytbl` does.
pub fn bioskeys() -> i64 {
    for o in &OVERRIDE {
        o.store(NO_OVERRIDE, Ordering::Relaxed);
    }
    let Some(h) = host() else {
        return 0;
    };
    h.reset_key_tables();
    publish(h)
}

/// Lay the `struct keytbl` of the layout now in force into the caller's
/// address space and return its address (the common tail of `Keytbl` and
/// `Bioskeys`); 0 when the board has no keyboard translator.
fn publish(h: &dyn crate::host::Host) -> i64 {
    let Some(tables) = h.key_tables() else {
        return 0;
    };

    // ── the returned address. A running EL0 process gets the image in its own
    //    reserved TPA-low region (written through the kernel high-half alias,
    //    read back at the low-half user VA); otherwise the kernel static.
    let (write_base, emit_base) = match slot_for(current()) {
        Some(slot) => match crate::bdos::process::mmu::page0_kva(slot) {
            Some(page0) => (page0 as u64 + KEYTBL_AREA_VA, USER_VA_BASE + KEYTBL_AREA_VA),
            None => (KERNEL_AREA.0.get() as u64, KERNEL_AREA.0.get() as u64),
        },
        None => (KERNEL_AREA.0.get() as u64, KERNEL_AREA.0.get() as u64),
    };

    // SAFETY: `write_base` is either the kernel static area or the kernel
    // high-half alias of the process's reserved page-0 window, both writable
    // and `KEYTBL_AREA_SIZE` bytes long (the reservation is enforced by the
    // const asserts in `loader.rs`). Single-CPU under the trap.
    unsafe { write_area(write_base, emit_base, &tables, current_abi()) };
    emit_base as i64
}
