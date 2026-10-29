//! NewTOS cookie jar — a kernel-owned system-feature registry.
//!
//! Spec / data model: [`docs/api/bios/_cookie-jar.md`]; access ABI:
//! [`docs/api/gemdos/154-ssystem.md`] (FreeMiNT `Ssystem`). Decision:
//! ADR-012 (`docs/MEMORY.md`).
//!
//! The jar is kernel-owned: a fixed-capacity static array (the no-`alloc`
//! analogue of FreeMiNT's fixed `dflt_jar[]`; a growable heap jar was the
//! ADR-012 sketch but `tos-api` is `#![no_std]` without `alloc`, and a
//! fixed array is closer to FreeMiNT's own model). It lives in kernel
//! memory; user programs never get a pointer to it — they read values
//! through the `Ssystem` cookie subfunctions. Single-CPU, single-threaded
//! access (boot seeding + the SVC trap path), so it uses the same
//! `UnsafeCell` discipline as [`crate::aes`]'s process table.

use core::cell::UnsafeCell;

/// Maximum number of cookies. FreeMiNT seeds ~10–15; 32 leaves headroom
/// for `Setcookie` without an allocator.
const JAR_CAP: usize = 32;

// NewTOS cookie tags (4-char big-endian ASCII, as on TOS).
/// `NTOS` — NewTOS identity; value = version (lets ported programs detect
/// the environment). This is the NewTOS-specific cookie.
pub const NTOS: u32 = u32::from_be_bytes(*b"NTOS");
/// `_MCH` — machine type.
pub const MCH: u32 = u32::from_be_bytes(*b"_MCH");
/// `_IDT` — date/time display format.
pub const IDT: u32 = u32::from_be_bytes(*b"_IDT");
/// `_AKP` — keyboard / language (low word = language id).
pub const AKP: u32 = u32::from_be_bytes(*b"_AKP");
/// `EdDI` — the VDI answers `vq_scrninfo` (VDI 102, sub-opcode 1,
/// `INTIN[0] == 2`). Tag `0x45644449` (mintlib `mint/cookie.h:125`
/// `C_EdDI`). Spec: `docs/api/vdi/eddi-vq_scrninfo.md` §2; a NewTOS
/// extension beyond EmuTOS, which has no EdDI (spec §0, E8 in
/// `docs/COMPATIBILITY.md`).
pub const EDDI: u32 = u32::from_be_bytes(*b"EdDI");

/// Value of the [`EDDI`] cookie: a **non-callable marker**, not the address
/// of a function dispatcher (spec §7 E2, option (b)).
///
/// On TOS the value is the address of an m68k dispatcher called with the
/// opcode in `d0` (Pure C convention, spec §2). That is a register
/// convention with no AArch64 mapping, and the NewTOS jar holds 32-bit
/// values (`Slot::value`), so no NewTOS code address could be stored either.
/// Programs detect EdDI by the cookie's **presence** and then call
/// `vq_scrninfo` — that is all the in-tree consumers do (zView
/// `zvdi/vdi.c:28`, HighWire `romvdi.c:70`; Doom does not look yet).
///
/// The value is chosen so that no consumer in the tree calls into it: it is
/// **non-zero** (a `value != 0` presence test still sees the cookie) and
/// **odd**. An odd value is never an m68k code address — `jsr` to it raises
/// an address error — and the EdDI probe `tosdump/core/scrninfo.c` refuses
/// to call a zero or odd value ("not a callable address"). It carries no
/// version: the EdDI version is only ever reported by dispatcher opcode 0,
/// which does not exist here. A program that calls the dispatcher anyway
/// faults — the documented cost of E2 (b).
pub const EDDI_MARKER: u32 = 0x0000_0001;

#[derive(Clone, Copy)]
struct Slot {
    tag: u32,
    value: u32,
}

struct Jar {
    slots: [Slot; JAR_CAP],
    count: usize,
}

struct JarCell(UnsafeCell<Jar>);
// SAFETY: the jar is touched only from the single boot CPU — at boot
// (`seed_defaults`) and from the synchronous SVC handler (`Ssystem`),
// never concurrently.
unsafe impl Sync for JarCell {}

static JAR: JarCell = JarCell(UnsafeCell::new(Jar {
    slots: [Slot { tag: 0, value: 0 }; JAR_CAP],
    count: 0,
}));

#[inline]
#[allow(clippy::mut_from_ref)]
unsafe fn jar() -> &'static mut Jar {
    // SAFETY: single-CPU, non-reentrant access (see `JarCell`).
    unsafe { &mut *JAR.0.get() }
}

/// A cookie tag is valid iff **every byte is non-zero** (FreeMiNT
/// `set_cookie`): this keeps the slot-number space (high word 0) free for
/// `Getcookie` enumeration.
pub fn valid_tag(tag: u32) -> bool {
    tag & 0xFF00_0000 != 0
        && tag & 0x00FF_0000 != 0
        && tag & 0x0000_FF00 != 0
        && tag & 0x0000_00FF != 0
}

/// Add a cookie, or update its value if the tag already exists. Returns
/// `false` if the tag is invalid or the jar is full (new tag, no room).
///
/// On success this also pulls every loaded process's `$5A0` mirror back in
/// step (see [`publish`]).
pub fn set(tag: u32, value: u32) -> bool {
    if !valid_tag(tag) {
        return false;
    }
    // The exclusive borrow of the jar ends with this block: `publish` reads
    // the jar again (through `count`/`slot_tag`/`get`), so it must not run
    // while a `&mut Jar` is still live.
    let changed = {
        // SAFETY: single-CPU access.
        let j = unsafe { jar() };
        match (0..j.count).find(|&i| j.slots[i].tag == tag) {
            Some(i) => {
                j.slots[i].value = value;
                true
            }
            None if j.count < JAR_CAP => {
                j.slots[j.count] = Slot { tag, value };
                j.count += 1;
                true
            }
            None => false, // jar full
        }
    };
    if changed {
        publish();
    }
    changed
}

/// Push the jar's current content into the `$5A0` mirror of every process
/// that owns a TPA.
///
/// The jar is kernel-owned (ADR-012), but a 1:1-ported TOS program does not
/// reach it that way: it dereferences `_p_cookies`@`$5A0` and scans the array
/// (TeraDesk `library/utility/cookie.c:28,56`). NewTOS materialises that array
/// per process, inside the process's own low-memory area
/// (`bdos::process::loader::write_cookie_jar`). Without this call the mirror
/// would be a snapshot of load time and a cookie set afterwards would never
/// reach a running program — which is exactly the gap
/// `docs/api/bios/_cookie-jar.md` §OFFEN records.
///
/// Called from **inside** [`set`] / [`del`] rather than from their callers, so
/// the invariant "whoever changes the kernel jar pulls the mirrors along" is
/// structural: a future third mutator gets it for free. At boot
/// ([`seed_defaults`]) no process owns a TPA yet, so it is a no-op there.
#[inline]
fn publish() {
    crate::bdos::process::loader::refresh_cookie_mirrors();
}

/// Look up a cookie value by tag.
pub fn get(tag: u32) -> Option<u32> {
    // SAFETY: single-CPU access; read-only use of the exclusive ref.
    let j = unsafe { jar() };
    (0..j.count).find(|&i| j.slots[i].tag == tag).map(|i| j.slots[i].value)
}

/// Number of cookies **present** (occupied slots). Not the value returned
/// for the `tag == 0` capacity query — see [`capacity`].
pub fn count() -> u32 {
    // SAFETY: single-CPU access.
    unsafe { jar() }.count as u32
}

/// Total slot **capacity** of the jar — what FreeMiNT stores in the
/// terminator slot (`sys/cookie.c:322-326`) and returns for
/// `S_GETCOOKIE(tag == 0)`. This is the fixed jar size, preserved verbatim
/// on every `set_cookie` append; it is **not** the occupied count
/// ([`count`]).
pub fn capacity() -> u32 {
    JAR_CAP as u32
}

/// Tag of the 1-based `n`-th cookie (slot enumeration). `None` if out of
/// range.
pub fn slot_tag(n: u32) -> Option<u32> {
    if n == 0 {
        return None;
    }
    // SAFETY: single-CPU access.
    let j = unsafe { jar() };
    let idx = (n - 1) as usize;
    (idx < j.count).then(|| j.slots[idx].tag)
}

/// Remove a cookie by tag, compacting the jar. Returns `true` if it was
/// present.
///
/// On success this also pulls every loaded process's `$5A0` mirror back in
/// step (see [`publish`]), so the removed tag stops being visible there too.
pub fn del(tag: u32) -> bool {
    // Borrow scoped exactly as in `set`, and for the same reason.
    let removed = {
        // SAFETY: single-CPU access.
        let j = unsafe { jar() };
        match (0..j.count).find(|&i| j.slots[i].tag == tag) {
            Some(i) => {
                for k in i..j.count - 1 {
                    j.slots[k] = j.slots[k + 1];
                }
                j.count -= 1;
                true
            }
            None => false,
        }
    };
    if removed {
        publish();
    }
    removed
}

/// Seed the NewTOS default cookie set at boot (replaces EmuTOS'
/// `fill_cookie_jar`). Only the tags that are meaningful for NewTOS are
/// present; the Atari-hardware cookies (`_VDO`/`_SND`/`_FDC`/`_FPU`/`_CPU`)
/// are deliberately omitted (ADR-012 amendment 2026-10-28; `_VDO` is open
/// until TASK/ARCH06's measurement is decided). `_MCH` is fixed by that
/// amendment ([`MCH_NOHARD`]) and `EdDI` by spec decision E2
/// ([`EDDI_MARKER`]); `_IDT` and `_AKP` are bring-up policy.
pub fn seed_defaults(version: u32) {
    set(NTOS, version); // NewTOS identity + version
    set(MCH, MCH_NOHARD); // machine id: EmuTOS MCH_NOHARD, no Atari hardware
    set(IDT, 0x0000_112F); // date/time: '/' sep, DD-MM-YY, 24-hour
    set(AKP, 0x0000_0000); // keyboard language: US
    set(EDDI, EDDI_MARKER); // VDI answers vq_scrninfo; marker, no dispatcher (E2)
}

/// Value of the [`MCH`] cookie: EmuTOS' `MCH_NOHARD`, "no ST-compatible
/// hardware" (`references/emutos/include/cookie.h:50`), the value EmuTOS'
/// `setvalue_mch` reports in a build without Atari hardware
/// (`references/emutos/bios/machine.c:400-401`, the `#else` branch of
/// `CONF_ATARI_HARDWARE`). That is NewTOS's situation: no Shifter, Videl,
/// MFP or FDC. User decision, ADR-012 amendment 2026-10-28
/// (`docs/MEMORY.md`, TASK/ARCH05, TASK/ARCH06); the NewTOS identity is the
/// [`NTOS`] cookie, not `_MCH`.
///
/// It replaces the provisional `0x0005_0000`, which is the registered
/// **ARAnyM** id (`MCH_ARANYM`, `references/emutos/include/cookie.h:57`).
/// The pinned make-512 oracle never reports `MCH_NOHARD`, so every `_MCH`
/// cookie case is an ACCEPTED deviation (`docs/COMPATIBILITY.md`).
///
/// Read through `Ssystem(S_GETCOOKIE, '_MCH', 0)` (value in `x0`) it comes
/// back as `-1`, which is also that call's "not found" (`EERROR`, FreeMiNT
/// `sys/cookie.c:508-509`): only the pointer form (`arg2 != 0`, return code
/// `E_OK`) tells the two apart. The same holds on FreeMiNT for any cookie
/// whose value is `-1`.
pub const MCH_NOHARD: u32 = 0xFFFF_FFFF;
