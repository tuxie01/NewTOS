//! GEMDOS trap binding (`svc #1`) — EmuTOS's `include/gemdos.h` /
//! `util/gemdos.c` layer over `include/bdosbind.h`'s `trap1()`.
//!
//! DESK43 U6b: this is the binding the kernel desktop used to carry as
//! `kernel/src/desk/dosbind.rs`, merged with the handful of calls the U6a
//! runtime had here. One function per GEMDOS call and one name per function
//! number: the function numbers are `tos_types::gemdos` (shared with the AES
//! shell), the error codes `tos_types::error`.
//!
//! **Names.** Where EmuTOS has a `dos_*` wrapper (`include/gemdos.h`,
//! `util/gemdos.c`) the binding carries that name — [`dos_open`],
//! [`dos_sfirst`], [`dos_conws`], [`dos_free`], … . A call without one keeps
//! its Atari name in lower case (`CLAUDE.md` naming rule): [`cconout`],
//! [`malloc`], [`pterm`]. [`dos_getcookie`] is NewTOS's own (see there).
//!
//! The binding is the same at EL1 and EL0: until DESK43 U10 the built-in
//! desktop called it from the kernel's boot context, a native program calls it
//! from user mode; the trap handler tells the two apart by the caller's `SPSR_EL1.M` (`TrapArgs::space`,
//! ARCH03 stage 3a) and reads a kernel caller's pointers unchanged.
//!
//! ## ABI
//!
//! `docs/ARCHITECTURE.md`, "Native AArch64 callers"; `docs/sdk/05-abi-pitfalls.md`
//! §A1: function id in `x8` (low 16 bits), arguments in `x0..x7` **in their
//! TOS order**, result a sign-extended `LONG` in `x0`; every other register is
//! preserved. The argument order is therefore exactly the `trap1(...)`
//! argument list of `bdosbind.h:22-54` — note the two calls that carry a dead
//! leading word, `Frename(0, old, new)` (`:53`) and `Fdatime(timeptr, handle,
//! wflag)` (`:54`).
//!
//! ## What the trap changed for the EL1 desktop, and what it did not (until DESK43 U10)
//!
//! Before DESK10 the desktop called the `tos_api::bdos::fs` leaf functions
//! directly with a hand-built `TrapArgs`. Two properties of the trap path were
//! established before that rewiring, because they are the only two places
//! where "same arguments" could still mean "different behaviour":
//!
//! * **The DTA.** `Fsetdta`/`Fsfirst`/`Fsnext` keep the current DTA in the
//!   *running process's* basepage (`p_xdta`, EmuTOS `bdos/fsmain.c:396-405`).
//!   The EL1 desktop runs in the kernel's boot context, where
//!   `tos_api::bdos::process::current()` is the null basepage, and
//!   `tos-api/src/bdos/fs/find.rs:52-105` routes exactly that caller to its
//!   `KERNEL_DTA` cell instead.
//! * **Pointer arguments.** A trap from EL0 hands the kernel a *user* VA; the
//!   EL1 desktop hands it a kernel high-half VA. The trap handler records
//!   which of the two a call's pointers are (`TrapArgs::space`): an EL0
//!   program's are read and written with its rights through
//!   `tos_api::uaccess`, an EL1 caller's (`Space::Kernel`) are dereferenced
//!   unchanged.

use core::ffi::CStr;

use alloc::string::String;
use tos_types::gemdos::{
    CCONIS, CCONOUT, CCONWS, CRAWCIN, DCREATE, DDELETE, DFREE, DGETDRV, DSETDRV, DSETPATH, FATTRIB, FCLOSE, FCREATE,
    FDATIME, FDELETE, FOPEN, FREAD, FRENAME, FSETDTA, FSFIRST, FSNEXT, FWRITE, MALLOC, MFREE, MXALLOC,
    PTERM, SSYSTEM,
};
use tos_types::{Long, MX_STRAM, MX_TTRAM};

/// `S_GETCOOKIE` (FreeMiNT `sys/mint/ssystem.h`) — `Ssystem` subfunction 8.
const S_GETCOOKIE: i16 = 8;

/// `S_NEWTOS_KVERSION` — `Ssystem` subfunction 2019, NewTOS's own
/// (`docs/api/gemdos/154-ssystem.md`, SYSINFO01).
const S_NEWTOS_KVERSION: i16 = 2019;

/// `FA_VOL` (`include/bdosdefs.h:39`) — the volume-label attribute,
/// [`dos_label`]'s `Fsfirst` mask.
pub const FA_VOL: u8 = 0x08;

/// `F_SETMOD` (`include/bdosdefs.h:45`) — [`dos_chmod`]'s "write the
/// attribute" mode; `F_GETMOD` (0) reads it back instead.
pub const F_SETMOD: i16 = 1;

/// Issue the GEMDOS trap (`svc #1`): function id in `x8`, arguments in
/// `x0..x2`. EmuTOS's `trap1()` (`util/miscasm.S`). Returns the whole `x0`:
/// most calls answer a sign-extended `LONG` ([`Long`]), but `Malloc` answers
/// an address, which on AArch64 is 64 bits wide.
///
/// `x3..x6` are zeroed rather than left with whatever the compiler had in
/// them: the GEMDOS dispatcher's tracer records the *frame*
/// (`tos-api/src/gemdos/trace.rs` logs `x0..x6`), and a trap frame carrying
/// leftovers would put noise into it. No GEMDOS function bound here takes more
/// than three arguments (`bdosbind.h:22-54`).
///
/// `nostack`: an exception taken at EL1 with `SPSel = 0` runs the handler on
/// `SP_EL1` (the built-in desktop's case until DESK43 U10, `link-rpi4.ld:100-102`), and one taken
/// at EL0 switches to `SP_EL1` as well, so the caller's stack is untouched.
/// Memory is deliberately *not* declared clean: `Fread` and `Fsfirst` write
/// through the pointers they are given.
#[inline]
fn trap1(func: u16, a0: u64, a1: u64, a2: u64) -> i64 {
    let ret: i64;
    // SAFETY: GEMDOS trap. The handler restores every register except `x0`,
    // which it overwrites with the result.
    unsafe {
        core::arch::asm!(
            "svc #1",
            inlateout("x0") a0 => ret,
            in("x1") a1,
            in("x2") a2,
            in("x3") 0u64,
            in("x4") 0u64,
            in("x5") 0u64,
            in("x6") 0u64,
            in("x8") func as u64,
            options(nostack),
        );
    }
    ret
}

/// [`trap1`] for a call whose result is a `LONG` (every call but `Malloc`).
#[inline]
fn dos_trap(func: u16, a0: u64, a1: u64, a2: u64) -> Long {
    trap1(func, a0, a1, a2) as Long
}

/// Widen a signed TOS argument into its register slot.
///
/// The m68k ABI pushes a WORD as a WORD and a LONG as a LONG; on AArch64 the
/// whole 64-bit register carries it, and the dispatcher reads back the width
/// the function declares (`TrapArgs::arg_i16`/`arg_i32`). The *upper* bits are
/// therefore unobservable to the callee — but not to the dispatcher's tracer,
/// which logs the raw register. Sign-extending keeps the frame identical to
/// what a hand-built `TrapArgs` would hold.
#[inline]
const fn sx(v: i64) -> u64 {
    v as u64
}

/// A NUL-terminated GEMDOS path buffer built from `s`.
///
/// 130 bytes = EmuTOS `MAXPATHLEN` (`sysconf.h:51-55`, `LEN_ZPATH` 114 +
/// `LEN_ZFNAME` 13 + 1 + rounding); a longer `s` is truncated, which GEMDOS
/// then rejects with `EPTHNF` exactly as it would reject the untruncated name.
fn cpath(s: &str) -> [u8; 130] {
    let mut b = [0u8; 130];
    let src = s.as_bytes();
    let n = src.len().min(b.len() - 1);
    b[..n].copy_from_slice(&src[..n]);
    b
}

// ── Console ────────────────────────────────────────────────────────────────

/// `Cconout(c)` (GEMDOS 0x02) — one character to the console.
///
/// The character travels in a register, so an EL1 caller can use it where it
/// cannot use [`dos_conws`]: `Cconws` reads its string with the unprivileged
/// `ldtrb` (`tos-api/src/gemdos/console.rs`), which faults on kernel-VA
/// literals. The desktop's self-test writer
/// (`programs/desktop/src/selftest.rs`) was that caller while the desktop ran
/// at EL1 (until DESK43 U10) and still writes through it.
// NEWTOS: EmuTOS's `include/gemdos.h` has no `dos_*` wrapper for `Cconout`.
pub fn cconout(c: u8) {
    dos_trap(CCONOUT, c as u64, 0, 0);
}

/// `dos_conws` — `Cconws(string)` (`gemdos.h:35-38`): write `s` to the console
/// (on NewTOS: the screen and, with `serial=on`, the serial line). Returns the
/// GEMDOS result (EmuTOS's wrapper drops it).
pub fn dos_conws(s: &CStr) -> Long {
    dos_trap(CCONWS, s.as_ptr() as u64, 0, 0)
}

/// `Crawcin()` (GEMDOS 0x07, `bdosbind.h:23`) — read one character from the
/// console, **blocking**, without echo and without Ctrl-key handling
/// (`docs/api/gemdos/console/07-crawcin.md`). Returns the `LONG` the BIOS
/// console gives: scancode in bits 16-23, ASCII in bits 0-7.
// NEWTOS: EmuTOS's `include/gemdos.h` has no `dos_*` wrapper for `Crawcin`.
pub fn crawcin() -> Long {
    dos_trap(CRAWCIN, 0, 0, 0)
}

/// `Cconis()` (GEMDOS 0x0B, `bdosbind.h:25`) — is a console character
/// waiting? Non-zero (canonically `-1`) if so, `0` if not
/// (`docs/api/gemdos/console/0b-cconis.md`).
// NEWTOS: EmuTOS's `include/gemdos.h` has no `dos_*` wrapper for `Cconis`.
pub fn cconis() -> Long {
    dos_trap(CCONIS, 0, 0, 0)
}

// ── Files, directories, drives (EmuTOS `include/gemdos.h`) ──────────────────

/// `dos_sdta` — `Fsetdta(buf)` (`gemdos.h:50-53`). Points the current search
/// at `dta`, a writable 44-byte buffer.
pub fn dos_sdta(dta: *mut u8) {
    dos_trap(FSETDTA, dta as u64, 0, 0);
}

/// `dos_sfirst` — `Fsfirst(pspec, attr)` (`gemdos.h:60-63`). Starts a search;
/// `E_OK` on a match, `EFILNF`/`ENMFIL`/`EPTHNF` otherwise.
pub fn dos_sfirst(spec: &str, attr: u8) -> Long {
    let p = cpath(spec);
    dos_trap(FSFIRST, p.as_ptr() as u64, attr as u64, 0)
}

/// `dos_snext` — `Fsnext()` (`gemdos.h:65-68`). Continues the search whose
/// state lives in the current DTA.
pub fn dos_snext() -> Long {
    dos_trap(FSNEXT, 0, 0, 0)
}

/// `dos_open` — `Fopen(pname, access)` (`gemdos.h:70-73`). Returns a handle
/// (≥ 0) or a negative GEMDOS error.
pub fn dos_open(path: &str, access: u16) -> Long {
    let p = cpath(path);
    dos_trap(FOPEN, p.as_ptr() as u64, access as u64, 0)
}

/// `dos_create` — `Fcreate(name, attr)` (`gemdos.h:110-113`). Creates or
/// truncates; returns a handle or a negative GEMDOS error.
pub fn dos_create(path: &str, attr: u16) -> Long {
    let p = cpath(path);
    dos_trap(FCREATE, p.as_ptr() as u64, attr as u64, 0)
}

/// `dos_close` — `Fclose(handle)` (`gemdos.h:75-78`).
pub fn dos_close(handle: i16) -> Long {
    dos_trap(FCLOSE, sx(handle as i64), 0, 0)
}

/// `dos_read` — `Fread(handle, cnt, pbuffer)` (`gemdos.h:80-83`). Returns the
/// number of bytes read, or a negative GEMDOS error.
pub fn dos_read(handle: i16, cnt: i32, buf: *mut u8) -> Long {
    dos_trap(FREAD, sx(handle as i64), sx(cnt as i64), buf as u64)
}

/// `dos_write` — `Fwrite(handle, cnt, pbuffer)` (`gemdos.h:85-88`). Returns
/// the number of bytes written, or a negative GEMDOS error.
pub fn dos_write(handle: i16, cnt: i32, buf: *const u8) -> Long {
    dos_trap(FWRITE, sx(handle as i64), sx(cnt as i64), buf as u64)
}

/// `dos_delete` — `Fdelete(name)` (`gemdos.h:125-128`).
pub fn dos_delete(path: &str) -> Long {
    let p = cpath(path);
    dos_trap(FDELETE, p.as_ptr() as u64, 0, 0)
}

/// `dos_chmod` — `Fattrib(name, wrt, mod)` (`gemdos.h:120-123`). With
/// `wrt = F_SETMOD` the file's attribute byte becomes `mode`; the return is the
/// resulting attribute or a negative GEMDOS error.
pub fn dos_chmod(path: &str, wrt: i16, mode: u8) -> Long {
    let p = cpath(path);
    dos_trap(FATTRIB, p.as_ptr() as u64, sx(wrt as i64), mode as u64)
}

/// `dos_rename` — `Frename(p1, p2)` (`gemdos.h:130-133`), which expands to
/// `trap1(0x56, 0, oldname, newname)` (`bdosbind.h:53`): the leading word is
/// the historical unused argument, so the names sit in `x1`/`x2`.
pub fn dos_rename(old: &str, new: &str) -> Long {
    let o = cpath(old);
    let n = cpath(new);
    dos_trap(FRENAME, 0, o.as_ptr() as u64, n.as_ptr() as u64)
}

/// `dos_mkdir` — `Dcreate(path)` (`gemdos.h:115-118`).
pub fn dos_mkdir(path: &str) -> Long {
    let p = cpath(path);
    dos_trap(DCREATE, p.as_ptr() as u64, 0, 0)
}

/// `dos_rmdir` — `Ddelete(path)` (`gemdos.h:135-138`).
pub fn dos_rmdir(path: &str) -> Long {
    let p = cpath(path);
    dos_trap(DDELETE, p.as_ptr() as u64, 0, 0)
}

/// `dos_setdt(h, time, date)` — `Fdatime(buf, h, TRUE)` over a two-word
/// buffer holding `time` then `date` (EmuTOS `util/gemdos.c:49-56`, declared
/// `gemdos.h:17`). Stamps an open handle with a packed FAT time/date, which is
/// how `deskdir.c:457` preserves a copied file's timestamp.
///
/// EmuTOS's `UWORD buf[2]` is big-endian m68k; NewTOS writes the two words
/// little-endian, the layout `tos-api/src/bdos/fs/file.rs:838-869` reads back.
/// The buffer's byte order is not part of the TOS contract — the *field* order
/// (time, then date) is.
pub fn dos_setdt(handle: i16, time: u16, date: u16) -> Long {
    let t = time.to_le_bytes();
    let d = date.to_le_bytes();
    let buf = [t[0], t[1], d[0], d[1]];
    dos_trap(FDATIME, buf.as_ptr() as u64, sx(handle as i64), 1)
}

/// `dos_gdrv` — `Dgetdrv()` (`gemdos.h:45-48`, `bdosbind.h:30`). The calling
/// process's current drive, 0-based (`0` = `A:`).
pub fn dos_gdrv() -> i16 {
    dos_trap(DGETDRV, 0, 0, 0) as i16
}

/// `dos_sdrv` — `Dsetdrv(newdrv)` (`gemdos.h:105-108`, `bdosbind.h:26`). Sets
/// the current drive (0-based; an out-of-range index is silently ignored) and
/// **returns the drive bitmap**, bit `n` = drive `n` is present
/// (`docs/api/gemdos/drive/0e-dsetdrv.md`). The bitmap comes back regardless of
/// whether the drive actually changed: EmuTOS's desktop reads the drive map as
/// `dos_sdrv(dos_gdrv())` (`deskapp.c:714`, `deskins.c:297`,
/// `desksupp.c:1575,1819`).
pub fn dos_sdrv(newdrv: i16) -> Long {
    dos_trap(DSETDRV, sx(newdrv as i64), 0, 0)
}

/// `dos_space` — `Dfree(buf, drv)` (`util/gemdos.c:77-90`, `bdosbind.h:34`),
/// the **total** and **available** byte counts of a drive:
///
/// ```c
/// if (Dfree(buf,drv) < 0L) return;        /* 0 = default, 1 = A for gemdos */
/// mult = buf[3] * buf[2];                 /* sectors/cluster * bytes/sector */
/// *ptotal = mult * buf[1];                /* total clusters   */
/// *pavail = mult * buf[0];                /* free clusters    */
/// ```
///
/// `drv` is **1-based** (`1` = `A:`, `0` = current). `(0, 0)` on error, which
/// is what EmuTOS leaves in its two output longs.
///
/// The buffer is four 32-bit LONGs in GEMDOS's on-wire layout
/// (`docs/api/gemdos/directory/36-dfree.md`) and the arithmetic is done in 64
/// bits here: a 500 MB FAT16 partition already overflows a signed 32-bit byte
/// count at 2 GB, and EmuTOS's `LONG` result silently wraps there.
pub fn dos_space(drv: i16) -> (u64, u64) {
    let mut buf = [0u32; 4];
    if dos_trap(DFREE, buf.as_mut_ptr() as u64, sx(drv as i64), 0) < 0 {
        return (0, 0);
    }
    let mult = buf[3] as u64 * buf[2] as u64;
    (mult * buf[1] as u64, mult * buf[0] as u64)
}

/// `dos_label` — the volume label of a drive (`util/gemdos.c:59-74`):
///
/// ```c
/// Fsetdta(&dta);
/// strcpy(path, " :\\*.*");
/// path[0] = (drive + 'A') - 1;
/// if (!Fsfirst(path,0x08)) { strcpy(plabel,dta.d_fname); return TRUE; }
/// return FALSE;
/// ```
///
/// `drive` is 1-based, as [`dos_space`]'s is. A search mask of exactly
/// [`FA_VOL`] matches volume labels **only** — nothing else in the root can
/// come back (`tos-api/src/bdos/fs/find.rs:190-199`, EmuTOS `bdos/fsdir.c`
/// `scan`). [`None`] when the drive carries no label.
pub fn dos_label(drv: i16) -> Option<String> {
    let mut dta = [0u8; 44];
    dos_sdta(dta.as_mut_ptr());
    let mut path = String::new();
    path.push((b'A' + (drv as u8).saturating_sub(1)) as char);
    path.push_str(":\\*.*");
    if dos_sfirst(&path, FA_VOL) != 0 {
        return None;
    }
    // `d_fname` is the last 14 bytes of the 44-byte DTA.
    let name = &dta[30..44];
    let n = name.iter().position(|&b| b == 0).unwrap_or(name.len());
    core::str::from_utf8(&name[..n]).ok().map(String::from)
}

/// `dos_chdir` — `Dsetpath(pdrvpath)` (`gemdos.h:95-98`, `bdosbind.h:36`). Sets
/// the current directory **of the drive the path names** — not the current
/// drive, which is [`dos_sdrv`]'s job.
///
/// `E_OK` on success, `EPTHNF` when the path does not resolve to an existing
/// directory (`docs/api/gemdos/directory/3b-dsetpath.md`). A trailing separator
/// is accepted.
pub fn dos_chdir(path: &str) -> Long {
    let p = cpath(path);
    dos_trap(DSETPATH, p.as_ptr() as u64, 0, 0)
}

// ── Memory ─────────────────────────────────────────────────────────────────

/// `Malloc(amount)` — allocate `amount` bytes of the TPA pool; the block's
/// address, or `None` when GEMDOS answers 0 (no block that large).
///
/// `amount == -1` (the size of the largest free block) is [`malloc_avail`].
// NEWTOS: EmuTOS's `gemdos.h` allocates through `Mxalloc` (`dos_alloc_*`,
// `util/gemdos.c:94-127`); a program's heap uses plain `Malloc`
// (`heap.rs`, after mintlib).
pub fn malloc(amount: usize) -> Option<core::ptr::NonNull<u8>> {
    let r = trap1(MALLOC, amount as u64, 0, 0);
    if r <= 0 {
        None
    } else {
        core::ptr::NonNull::new(r as usize as *mut u8)
    }
}

/// `Malloc(-1)` — the size of the largest free block.
// NEWTOS: as [`malloc`].
pub fn malloc_avail() -> i64 {
    trap1(MALLOC, (-1i64) as u64, 0, 0)
}

/// `dos_free` — `Mfree(maddr)` (`util/gemdos.c:130-133`): return a block
/// obtained from [`malloc`]. `E_OK` (0) or a negative GEMDOS error (`EIMBA`).
pub fn dos_free(block: *mut u8) -> Long {
    dos_trap(MFREE, block as u64, 0, 0)
}

/// `dos_avail_stram` — `Mxalloc(-1L, MX_STRAM)` (`util/gemdos.c:100-104`): the
/// **largest free block** in ST-RAM, in bytes. `amount == -1` is `Mxalloc`'s
/// probe form — it measures and allocates nothing
/// (`docs/api/gemdos/memory/44-mxalloc.md`).
pub fn dos_avail_stram() -> Long {
    dos_trap(MXALLOC, sx(-1), sx(MX_STRAM as i64), 0)
}

/// `dos_avail_altram` — `Mxalloc(-1L, MX_TTRAM)` (`util/gemdos.c:106-113`,
/// `CONF_WITH_ALT_RAM`): the largest free block in alternate RAM, `0` when the
/// board has none (`tos-api/src/bdos/mem/traps.rs:274-281`, `pmdalt` null ⇒
/// `0`).
pub fn dos_avail_altram() -> Long {
    dos_trap(MXALLOC, sx(-1), sx(MX_TTRAM as i64), 0)
}

// ── Process ────────────────────────────────────────────────────────────────

/// `Pterm(retcode)` — end the program with exit code `rc`. Never returns.
// NEWTOS: EmuTOS's `gemdos.h` has no wrapper; its desktop issues `Pterm`
// from `desk/deskstart.S`.
pub fn pterm(rc: i16) -> ! {
    trap1(PTERM, rc as i64 as u64, 0, 0);
    // Pterm does not return; guard against a runaway PC (as sdk/crt0.S does).
    loop {
        core::hint::spin_loop();
    }
}

// ── System ─────────────────────────────────────────────────────────────────

/// Read one cookie **by value** — `Ssystem(S_GETCOOKIE, tag, 0)`
/// (`docs/api/gemdos/154-ssystem.md`, `tos-api/src/gemdos/ssystem.rs`).
/// [`None`] if the tag is not in the jar (the call returns
/// [`ERROR`](tos_types::error::ERROR)).
///
/// ## Why this and not `Supexec`
///
/// EmuTOS's desktop reads `_IDT` with `Supexec((LONG)get_idt_cookie)`
/// (`desksupp.c:219`) — not because the *cookie* needs privilege, but because
/// on m68k the jar is reached through the pointer at `$5A0`, in supervisor-only
/// low memory. That is an m68k mechanism, and `docs/COMPATIBILITY.md` puts
/// mechanisms on the free side of the line. XBIOS `Supexec` (0x26) is an open
/// ADR on NewTOS (`docs/api/xbios/26-supexec.md`) and the cookie-jar spec names
/// this call as its replacement (`docs/api/bios/_cookie-jar.md` §Status
/// 2026-09-14). The third argument is 0, so the value comes back in `x0` and
/// no pointer crosses the trap.
///
/// **The value form cannot express a cookie whose value has bit 31 set** — it
/// would be indistinguishable from the negative error return. That is
/// `Ssystem`'s own shape, and the alternative (`arg2` = a pointer to write into)
/// is closed to an EL1 caller: `s_getcookie` stores through an *unprivileged*
/// `sttr`, which faults on a kernel address. A caller reading a pointer-valued
/// cookie must not use this.
// NEWTOS: no EmuTOS counterpart (EmuTOS reads the jar under `Supexec`).
pub fn dos_getcookie(tag: u32) -> Option<u32> {
    let rc = dos_trap(SSYSTEM, sx(S_GETCOOKIE as i64), sx(tag as i32 as i64), 0);
    (rc >= 0).then_some(rc as u32)
}

/// The running kernel's version string — `Ssystem(S_NEWTOS_KVERSION, buf,
/// len)` (`docs/api/gemdos/154-ssystem.md`, mode 2019): the OS name and
/// version followed by the build stamp, `"NewTOS 0.7 2026-11-04 (1452cc15)"`.
///
/// The kernel copies at most `buf.len() - 1` characters and always a NUL
/// (FreeMiNT's `S_KNAME` rule). `Ok(n)` is the length before that NUL;
/// `Err(rc)` the call's negative return — `EINVFN` from a kernel without the
/// mode (EmuTOS has no `Ssystem` at all; a NewTOS kernel before SYSINFO01 has
/// no 2019), `ERROR` from one that reports no version. An empty `buf` is not
/// sent to the kernel (`len = 0` would write nothing): `Ok(0)`.
///
/// The buffer is the caller's, so it is a user address from an EL0 program;
/// the kernel writes it with the caller's rights (`TrapArgs::space`).
// NEWTOS: no EmuTOS counterpart (`Ssystem` is a FreeMiNT call; 2019 is NewTOS's).
pub fn dos_kversion(buf: &mut [u8]) -> Result<usize, Long> {
    if buf.is_empty() {
        return Ok(0);
    }
    let len = buf.len().min(i32::MAX as usize);
    let rc = dos_trap(SSYSTEM, sx(S_NEWTOS_KVERSION as i64), buf.as_mut_ptr() as u64, sx(len as i64));
    if rc < 0 {
        return Err(rc);
    }
    Ok(buf[..len].iter().position(|&b| b == 0).unwrap_or(len))
}
