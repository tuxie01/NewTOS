//! GEMDOS console group — output-side leaf functions (PHASE-02 D3).
//!
//! Specs: `docs/api/gemdos/console/*.md`.
//!
//! Every character-device access goes through the BIOS device table
//! ([`crate::bios::chardev`], FS11): the `Ccon*`/`Caux*`/`Cprn*` calls are the
//! BIOS calls of CON (2) / AUX (1) / PRN (0), and `Fread`/`Fwrite` on a device
//! handle use [`device_read`] / [`device_write`] — EmuTOS' `conin`/`cgets`/
//! `tabout` over `Bconin`/`Bconout` of that device (`bdos/bdosmain.c:604-640`).
//!
//! The calls follow the caller's standard handles `run->p_uft[]` (TASK/PROC04
//! point 1, `docs/api/gemdos/_dispatcher.md` §5) in EmuTOS's two ways: a
//! standard handle forced to a **file** is answered before dispatch by
//! [`redirect_to_file`] (`osif`, `bdos/bdosmain.c:498-568`); a standard
//! handle that is a **BIOS handle** names the device the bodies below use,
//! `HXFORM(run->p_uft[n])` ([`std_dev`], `bdos/console.c`). With the default
//! handles that is CON (2) for stdin/stdout, AUX (1) for stdaux, PRN (0) for
//! stdprn.
//!
//! Between the GEMDOS calls and the BIOS devices sits EmuTOS' BDOS console
//! layer (`bdos/console.c:27-582`, TASK/CON01): a column counter per device
//! (`glbcolumn[]`, for tab expansion and line editing), a 64-key typeahead for
//! AUX and CON that `conbrk` fills while output is written and `getch` reads
//! first, and the Ctrl-C / Ctrl-S / Ctrl-Q / Ctrl-X handling in `conbrk`. The
//! AES is not part of this: it takes its keys from the BIOS key queue
//! directly, as EmuTOS' `chkkbd` → `gsx_char` → `Bconstat`/`Bconin(2)` does
//! (`aes/gemdisp.c:158-177`, `vdi/vdi_input.c:220`); only the BDOS character
//! calls ever move a key into the typeahead.
//!
//! All functions take a borrow of [`TrapArgs`] and return a `Long`
//! (`i32` in `x0`). The semantics mirror EmuTOS exactly so binary
//! programs see TOS-bit-compatible behaviour; per-function citations
//! point to the EmuTOS source location used to derive the code.

use core::sync::atomic::{AtomicI16, AtomicI32, AtomicU32, Ordering};

use crate::bios::chardev::{self, DEV_AUX, DEV_CON};
use crate::host::host;
use crate::trap::TrapArgs;
use tos_types::error::EUNDEV;
use tos_types::Long;

// Helper: fetch the host or fall through with `EUNDEV`. In normal
// operation the host is always installed; the early-return path
// exists so a misconfigured boot fails predictably instead of
// dereferencing a null function pointer.
macro_rules! host_or_eundev {
    () => {
        match host() {
            Some(h) => h,
            None => return EUNDEV,
        }
    };
}

// ---------------------------------------------------------------------------
// Standard handles: BIOS device (`HXFORM`) and the file redirection in `osif`.
// ---------------------------------------------------------------------------

/// Standard input (`p_uft[0]`).
const STDIN: u16 = 0;
/// Standard output (`p_uft[1]`).
const STDOUT: u16 = 1;
/// Standard aux (`p_uft[2]`).
const STDAUX: u16 = 2;
/// Standard printer (`p_uft[3]`).
const STDPRN: u16 = 3;

/// The BIOS device of standard handle `std`: `HXFORM(run->p_uft[std])` =
/// `3 + p_uft[std]` (`bdos/console.h:36`), as the `bdos/console.c` bodies
/// compute it. A handle below `H_Print` gives a negative BIOS handle and a
/// file handle `h` gives `3 + h`; neither names a table entry, so the device
/// calls answer `0` there (EmuTOS `bconout` etc.: `handle` outside 0..7 → 0,
/// `bios/bios.c:930-932,973-976,1008-1010,1160-1162`).
fn std_dev(std: u16) -> u16 {
    hxform(crate::bdos::fs::file::std_handle(std))
}

/// EmuTOS `constat(h)` (`bdos/console.c:143-153`): only AUX (1) and CON (2)
/// have an input status — `-1` while their BDOS typeahead holds a key, else
/// the device's `Bconstat`; every other device answers `0`.
fn constat(dev: u16) -> Long {
    match typeahead(dev) {
        Some(t) if t.has_input() => -1,
        Some(_) => chardev::bconstat_handle(dev),
        None => 0,
    }
}

/// EmuTOS `EOF_INDICATOR` (`bdos/bdosmain.c:69`): what a redirected one-byte
/// read returns at end of file or on a read error ("same way as MiNT").
pub const EOF_INDICATOR: Long = 0x0000_ff1a;

/// `funcs[fn].stdio_typ & 0x7f` (`bdos/bdosmain.c:150-197`) for the functions
/// the file block of `osif` covers — `typ && fn && (fn < 12 || 16 <= fn <=
/// 19)` (`:498`). `Crawio` (6) has `stdio_typ 0` and is not among them.
fn redirected_std(fn_id: usize) -> Option<u16> {
    match fn_id {
        0x01 | 0x07 | 0x08 | 0x0A | 0x0B => Some(STDIN),
        0x02 | 0x09 | 0x10 => Some(STDOUT),
        0x03 | 0x04 | 0x12 | 0x13 => Some(STDAUX),
        0x05 | 0x11 => Some(STDPRN),
        _ => None,
    }
}

/// The file block of EmuTOS `osif` (`bdos/bdosmain.c:498-568`,
/// `docs/api/gemdos/_dispatcher.md` §5), run by the dispatcher before the
/// function table: when the call's standard handle is forced to a file
/// (`run->p_uft[std] > 0`) the call is answered here with raw byte I/O on that
/// handle — no echo, no tab expansion, no break check, no line editing — and
/// `Some(result)` is returned; otherwise `None` and the call dispatches
/// normally.
///
/// | fn | answer |
/// |----|--------|
/// | Cconin, Cauxin, Crawcin, Cnecin | one byte read, **sign-extended** (`char ctmp`); [`EOF_INDICATOR`] when it is not read |
/// | Cconout, Cauxout, Cprnout | the low byte of the WORD written; `0` |
/// | Cconws | `strlen(s)` bytes written raw; `0` |
/// | Cconrs | bytes read one by one into `p[2..]` until `max` (unsigned — named deviation, see the arm), EOF or CR (stored, not counted; the byte after it read and dropped); `p[1]` = count; `0` |
/// | Cconis, Cauxis | `0` at EOF (or no file), else `-1` |
/// | Cconos, Cprnos, Cauxos | `-1` |
///
/// The xread/xwrite results are dropped as in EmuTOS, except a device error
/// or a fault: EmuTOS unwinds the whole call there (`longjmp_rwabs`, a bus
/// error), NewTOS returns that code ([`crate::uaccess::FAULTED`] lets the
/// syscall tail end the program). Kernel scratch bytes are accessed as kernel
/// memory, the caller's string and record through its space
/// ([`TrapArgs::space`], ARCH03 3a), so memory protection applies as for
/// `Fread`/`Fwrite`.
pub fn redirect_to_file(fn_id: usize, args: &TrapArgs) -> Option<Long> {
    use crate::bdos::fs::file::{eof, std_handle, xread, xwrite};
    use crate::uaccess::Space;

    let std = redirected_std(fn_id)?;
    let h = std_handle(std);
    if h <= 0 {
        return None; // a BIOS handle: the function body uses `HXFORM`
    }
    // `Err` of xread/xwrite ends the call with its code.
    macro_rules! io {
        ($e:expr) => {
            match $e {
                Ok(v) => v,
                Err(code) => return Some(code),
            }
        };
    }
    let rc = match fn_id {
        // Cconin, Cauxin, Crawcin, Cnecin (`:509-519`).
        0x01 | 0x03 | 0x07 | 0x08 => {
            let mut ctmp = 0u8;
            let p = core::ptr::addr_of_mut!(ctmp);
            if io!(xread(h, 1, Space::Kernel, p as u64)) != 1 {
                EOF_INDICATOR
            } else {
                // SAFETY: `p` is this frame's byte, written by the read above
                // through that address.
                let b = unsafe { p.read_volatile() };
                b as i8 as Long // `char ctmp` → sign-extended
            }
        }
        // Cconout, Cauxout, Cprnout (`:521-528`): the char in the int.
        0x02 | 0x04 | 0x05 => {
            let ch = args.arg_u16(0) as u8;
            let _ = io!(xwrite(h, 1, Space::Kernel, core::ptr::addr_of!(ch) as u64));
            0 // dummy
        }
        // Cconws (`:530-533`).
        0x09 => {
            let s = args.arg_u64(0);
            if s != 0 {
                // `strlen(pb2)` in the caller's space; an unreadable byte ends
                // the call (EmuTOS: bus error).
                let mut len: u32 = 0;
                loop {
                    match args.space.get_u8(s + u64::from(len)) {
                        Ok(0) => break,
                        Ok(_) => len += 1,
                        Err(_) => return Some(crate::uaccess::FAULTED),
                    }
                }
                let _ = io!(xwrite(h, len as i32, args.space, s));
            }
            0 // dummy
        }
        // Cconrs (`:535-553`).
        0x0A => {
            let pb2 = args.arg_u64(0);
            if pb2 == 0 {
                return Some(0);
            }
            // `max = *pb2++` is a plain (signed) `char` in EmuTOS
            // (`bdosmain.c:537-539`): from 128 on the `while (max--)` loop
            // counts through the whole `int` range and only CR or EOF stops
            // it, past the record. NewTOS reads the byte unsigned and stops
            // after `max` bytes — named deviation, user decision 2026-10-18
            // (`docs/COMPATIBILITY.md` "Sanctioned deviations", TASK/PROC04);
            // identical for `max` <= 127.
            let Ok(m) = args.space.get_u8(pb2) else {
                return Some(crate::uaccess::FAULTED);
            };
            let mut max = u32::from(m);
            let mut i: u32 = 0;
            while max != 0 {
                max -= 1;
                // `p = pb2 + 1 + i` = record byte `2 + i`.
                let p = pb2 + 2 + u64::from(i);
                if io!(xread(h, 1, args.space, p)) != 1 {
                    break;
                }
                let Ok(c) = args.space.get_u8(p) else {
                    return Some(crate::uaccess::FAULTED);
                };
                if c == 0x0D {
                    // Eat the LF — whatever byte follows the CR.
                    let mut ctmp = 0u8;
                    let _ = io!(xread(h, 1, Space::Kernel, core::ptr::addr_of_mut!(ctmp) as u64));
                    break;
                }
                i += 1;
            }
            if args.space.put_u8(pb2 + 1, i as u8).is_err() {
                return Some(crate::uaccess::FAULTED);
            }
            0
        }
        // Cconis, Cauxis (`:555-558`): `eof` is non-zero at EOF and for a
        // handle with no file (EIHNDL).
        0x0B | 0x12 if eof(h) != 0 => 0,
        // Cconis/Cauxis before EOF, Cconos, Cprnos, Cauxos (`:561-564`).
        _ => -1,
    };
    Some(rc)
}

// ---------------------------------------------------------------------------
// The BDOS console layer (EmuTOS `bdos/console.c:27-582`, TASK/CON01):
// typeahead, column counter, `conbrk`, `conout`, `tabout`, `cookdout`,
// `getch`, `conin`.
// ---------------------------------------------------------------------------

/// Size of a BDOS typeahead buffer (`KBBUFSZ`, `bdos/fs.h:46`).
const KBBUFSZ: u32 = 64;
/// Index mask of a typeahead buffer (`KBBUFMASK`, `bdos/fs.h:47`).
const KBBUFMASK: u32 = KBBUFSZ - 1;

/// One BDOS typeahead buffer (EmuTOS `TYPEAHEAD`, `bdos/console.c:30-34`):
/// whole `Bconin` LONGs (scancode and ASCII), `add`/`remove` counting up and
/// both reset to 0 whenever the buffer runs empty (`buflush`). The counters
/// are compared by their distance, so a buffer that never runs empty cannot
/// overflow them.
struct Typeahead {
    add: AtomicU32,
    remove: AtomicU32,
    glbkbchar: [AtomicI32; KBBUFSZ as usize],
}

impl Typeahead {
    const fn new() -> Typeahead {
        Typeahead {
            add: AtomicU32::new(0),
            remove: AtomicU32::new(0),
            glbkbchar: [const { AtomicI32::new(0) }; KBBUFSZ as usize],
        }
    }

    /// EmuTOS `buflush` (`bdos/console.c:252-255`).
    fn flush(&self) {
        self.add.store(0, Ordering::Relaxed);
        self.remove.store(0, Ordering::Relaxed);
    }

    /// `add > remove` (`bdos/console.c:152,359`).
    fn has_input(&self) -> bool {
        self.add.load(Ordering::Relaxed) != self.remove.load(Ordering::Relaxed)
    }

    /// The `default` arm of `conbrk` (`bdos/console.c:236-239`): store `ch`
    /// while there is room; `false` when the buffer is full (the caller rings
    /// the bell).
    fn push(&self, ch: Long) -> bool {
        let add = self.add.load(Ordering::Relaxed);
        if add.wrapping_sub(self.remove.load(Ordering::Relaxed)) >= KBBUFSZ {
            return false;
        }
        self.glbkbchar[(add & KBBUFMASK) as usize].store(ch, Ordering::Relaxed);
        self.add.store(add.wrapping_add(1), Ordering::Relaxed);
        true
    }

    /// The typeahead half of `getch` (`bdos/console.c:356-366`): the oldest
    /// key, the buffer flushed once it is empty.
    fn pop(&self) -> Option<Long> {
        if !self.has_input() {
            return None;
        }
        let remove = self.remove.load(Ordering::Relaxed);
        let ch = self.glbkbchar[(remove & KBBUFMASK) as usize].load(Ordering::Relaxed);
        self.remove.store(remove.wrapping_add(1), Ordering::Relaxed);
        if !self.has_input() {
            self.flush();
        }
        Some(ch)
    }
}

/// `aux_typeahead` (`bdos/console.c:44`).
static AUX_TYPEAHEAD: Typeahead = Typeahead::new();
/// `con_typeahead` (`bdos/console.c:45`).
static CON_TYPEAHEAD: Typeahead = Typeahead::new();

/// EmuTOS `buffer[h]` (`bdos/console.c:46,115-117`): AUX (1) and CON (2) have
/// a typeahead buffer, PRN (0) has none (`NULL`) — and neither has any other
/// device.
fn typeahead(dev: u16) -> Option<&'static Typeahead> {
    match dev {
        DEV_AUX => Some(&AUX_TYPEAHEAD),
        DEV_CON => Some(&CON_TYPEAHEAD),
        _ => None,
    }
}

/// Cells of [`GLBCOLUMN`]: one per BIOS device table slot 0..7, and one shared
/// by every `HXFORM` value outside the table (a standard handle forced below
/// `H_Print`).
const NCOLUMN: usize = 9;

/// EmuTOS `glbcolumn[]` (`bdos/console.c:54`): the screen column `conout`
/// keeps per device, for tab expansion and `cgets`. EmuTOS has cells for PRN,
/// AUX and CON only and indexes past them for any other device; NewTOS gives
/// every table slot its own cell (unobservable for 0..2).
static GLBCOLUMN: [AtomicI16; NCOLUMN] = [const { AtomicI16::new(0) }; NCOLUMN];

/// The column cell of BIOS device `dev`.
fn glbcolumn(dev: u16) -> &'static AtomicI16 {
    &GLBCOLUMN[(dev as usize).min(NCOLUMN - 1)]
}

/// The typeahead and column half of EmuTOS `stdhdl_init`
/// (`bdos/console.c:114-125`, run once from `osinit`, `bdos/bdosmain.c:347`):
/// both typeahead buffers empty, every column 0. Called with the boot PDs
/// (`bdos::process::standin::init`).
pub fn stdhdl_init_console() {
    AUX_TYPEAHEAD.flush();
    CON_TYPEAHEAD.flush();
    for c in &GLBCOLUMN {
        c.store(0, Ordering::Relaxed);
    }
}

/// `conbrk` saw a Ctrl-C: the typeahead is flushed and `terminate()`
/// (`Pterm(-32)`) is requested ([`crate::bdos::process::request_terminate`];
/// the SVC handler performs it after the call). EmuTOS' `terminate()` does not
/// return, so whoever receives this stops at once and does nothing more.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Terminated;

/// EmuTOS `conbrk(h)` (`bdos/console.c:206-244`), run before every byte
/// `conout` writes and at the start of `Cconin`/`Cnecin`: if device `dev`
/// reports input (`Bconstat`), take one key with `Bconin` —
///
/// * Ctrl-C: flush the typeahead and terminate ([`Terminated`]);
/// * Ctrl-S: stop — keep reading keys (blocking) until a Ctrl-Q;
/// * Ctrl-Q: resume;
/// * Ctrl-X: flush the typeahead, then store the Ctrl-X like any other key;
/// * any other key: into the typeahead, or the bell (`Bconout(dev, 7)`) when
///   it is full.
///
/// While stopped every key read is handled the same way, so a key typed
/// during the pause is kept. EmuTOS has no typeahead for PRN
/// (`buffer[0] = NULL`) and would dereference it if PRN had input; NewTOS
/// leaves the key in the device there — no table entry but AUX and CON has
/// input, so this is never observable.
///
/// `Bconstat` of CON gives the polled keyboard of `board-rpi4` a turn when the
/// key ring is empty (`kernel/src/host_env.rs` `fill_from_device`) — the
/// stand-in for the keyboard interrupt that fills EmuTOS' queue while output
/// is written. The blocking reads of the pause never give way to a `kbdvec`
/// hook: keys may already be in the typeahead.
fn conbrk(dev: u16) -> Result<(), Terminated> {
    if chardev::bconstat_handle(dev) == 0 {
        return Ok(());
    }
    let (Some(buf), Some(d)) = (typeahead(dev), chardev::device(dev)) else {
        return Ok(());
    };
    let mut stop = false;
    loop {
        let ch = d.bconin();
        match ch as u8 {
            CTRL_C => {
                buf.flush();
                crate::bdos::process::request_terminate(CTRLC_RC);
                return Err(Terminated);
            }
            CTRL_S => stop = true,
            CTRL_Q => stop = false,
            c => {
                if c == CTRL_X {
                    buf.flush();
                }
                if !buf.push(ch) {
                    let _ = d.bconout(BEL);
                }
            }
        }
        if !stop {
            return Ok(());
        }
    }
}

/// EmuTOS `conout(h, ch)` (`bdos/console.c:263-273`): `conbrk`, then the byte
/// to the device (`Bconout`, result ignored), then the column: `+1` for a byte
/// `>= ' '` (as `unsigned char`, so 0x80..0xFF count), `0` after a CR, `-1`
/// after a BS — CR and BS compared on the whole `ch`, as EmuTOS compares the
/// `int`.
fn conout(dev: u16, ch: u16) -> Result<(), Terminated> {
    conbrk(dev)?;
    chardev::bconout_handle(dev, ch as u8);
    let col = glbcolumn(dev);
    if ch as u8 >= b' ' {
        col.fetch_add(1, Ordering::Relaxed);
    } else if ch == u16::from(CR) {
        col.store(0, Ordering::Relaxed);
    } else if ch == u16::from(BS) {
        col.fetch_sub(1, Ordering::Relaxed);
    }
    Ok(())
}

/// EmuTOS `tabout(h, ch)` (`bdos/console.c:292-303`): a TAB is written as
/// spaces up to the next multiple of 8 of the tracked column — at least one —
/// anything else goes to [`conout`].
fn tabout(dev: u16, ch: u16) -> Result<(), Terminated> {
    if ch == u16::from(TAB) {
        loop {
            conout(dev, u16::from(b' '))?;
            if glbcolumn(dev).load(Ordering::Relaxed) & 7 == 0 {
                return Ok(());
            }
        }
    }
    conout(dev, ch)
}

/// EmuTOS `getch(h)` (`bdos/console.c:351-369`): the oldest key of the
/// typeahead (AUX, CON), else the device's `Bconin` — blocking and never
/// giving way to a `kbdvec` hook (the read of `cgets`, which may already have
/// stored keys). A device past the table answers `0` (`bios/bios.c:973-976`).
fn getch(dev: u16) -> Long {
    if let Some(ch) = typeahead(dev).and_then(Typeahead::pop) {
        return ch;
    }
    chardev::device(dev).map_or(0, |d| d.bconin())
}

/// [`getch`] for the single-key reads with no side effect before their key
/// (`Cconin`, `Cnecin`, `Crawcin`, a one-byte `Fread`): `None` when the
/// `Bconin` has to give way to the running process's `kbdvec` hook (KEYS02;
/// the caller requests the restart and returns without side effects). Keys
/// already in the typeahead stay there across the restart.
fn getch_restartable(dev: u16) -> Option<Long> {
    if let Some(ch) = typeahead(dev).and_then(Typeahead::pop) {
        return Some(ch);
    }
    match chardev::device(dev) {
        Some(d) => d.bconin_restartable(),
        None => Some(0),
    }
}

// ---------------------------------------------------------------------------
// 0x02 Cconout — write a single byte to the console.
// EmuTOS: bdos/console.c:78  `xconout(WORD ch)`.
// Spec:   docs/api/gemdos/console/02-cconout.md
// ---------------------------------------------------------------------------

/// `Cconout(ch)` — write `ch` to the console output with tab expansion.
/// Always returns `1` (per `docs/api/gemdos/console/02-cconout.md`
/// and discrepancy `D-001`: EmuTOS unconditionally returns `1`,
/// even though Compendium and Profibuch document the value as
/// "undefined" or "the function code"). NewTOS follows EmuTOS.
///
/// EmuTOS `xconout` = `tabout(HXFORM(run->p_uft[1]), ch)`
/// (`bdos/console.c:279-283`, [`std_dev`]; CON by default) with the whole
/// WORD `ch`: [`tabout`] → [`conout`] → [`conbrk`] — a pending Ctrl-C ends
/// the call and requests `Pterm(-32)`, Ctrl-S pauses until Ctrl-Q, other keys
/// go into the BDOS typeahead.
pub fn cconout(args: &TrapArgs) -> Long {
    let _ = host_or_eundev!();
    // A Ctrl-C has requested the terminate; the SVC handler performs it.
    let _ = tabout(std_dev(STDOUT), args.arg_u16(0));
    1
}

// ---------------------------------------------------------------------------
// AUX BIOS handle. EmuTOS' `xauxin`/`xauxout` resolve stdaux from
// `run->p_uft[2]` (default `H_Aux = -2`) and call `Bconin(HXFORM(p_uft[2]))` /
// `Bconout(HXFORM(p_uft[2]), ch)` where `HXFORM(h) = 3 + h`, i.e. BIOS handle 1
// by default (`docs/api/gemdos/console/{03-cauxin,04-cauxout}.md`) — `DEV_AUX`
// of the device table.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// 0x03 Cauxin — read a single byte from AUX.
// EmuTOS: bdos/console.c:432 `xauxin()` = `Bconin(1)` — a pure pass-through
// (docs/api/gemdos/console/03-cauxin.md).
// ---------------------------------------------------------------------------

/// `Cauxin()` — pass-through to `Bconin(HXFORM(p_uft[2]))`, exactly as
/// EmuTOS' `xauxin` (`bdos/console.c:434`): by default `Bconin(1)`, the AUX
/// entry of the device table. On real Pi 4 that entry is "port present, no
/// cable" (`docs/api/gemdos/console/03-cauxin.md`, FS11 note; `3f-fread.md`
/// item 1a), so the pass-through returns `0` at once instead of waiting for
/// a byte that never arrives; on qemu-virt the kernel's serial driver
/// replaces the entry (XBIOS15) and `Cauxin` returns the next received byte,
/// waiting for it as EmuTOS' `bconin1`.
pub fn cauxin(_: &TrapArgs) -> Long {
    chardev::bconin_handle(std_dev(STDAUX))
}

// ---------------------------------------------------------------------------
// 0x04 Cauxout — write a single byte to AUX.
// EmuTOS: bdos/console.c:331 `xauxout(ch)` = `Bconout(1, ch)` — a pure
// pass-through (docs/api/gemdos/console/04-cauxout.md).
// ---------------------------------------------------------------------------

/// `Cauxout(ch)` — pass-through to `Bconout(HXFORM(p_uft[2]), ch)`, exactly
/// as EmuTOS' `xauxout` (`bdos/console.c:333`): by default the AUX entry of
/// the device table. The no-cable entry
/// discards the byte and returns `1`
/// (`docs/api/gemdos/console/04-cauxout.md`, option A).
pub fn cauxout(args: &TrapArgs) -> Long {
    let ch = args.arg_u16(0) as u8;
    chardev::bconout_handle(std_dev(STDAUX), ch)
}

// ---------------------------------------------------------------------------
// 0x05 Cprnout — write a single byte to the printer.
// EmuTOS: bdos/console.c:340 `xprtout(WORD ch)` = `Bconout(0, ch)`.
// Spec: docs/api/gemdos/console/05-cprnout.md (`0` = printer busy/absent, a
// non-negative ready/busy signal, *not* a TOS error code).
// ---------------------------------------------------------------------------

/// `Cprnout(ch)` — pass-through to `Bconout(HXFORM(p_uft[3]), ch)`
/// (`bdos/console.c:342`): by default `Bconout(0, ch)`, the PRN entry of the
/// device table. With the absent-device entry (no printer port) the byte is
/// never accepted and the call returns `0` at once — the value EmuTOS'
/// `bconout0` returns without `CONF_WITH_PRINTER_PORT`
/// (`bios/parport.c:144-172`; FS11 note in the spec).
pub fn cprnout(args: &TrapArgs) -> Long {
    let ch = args.arg_u16(0) as u8;
    chardev::bconout_handle(std_dev(STDPRN), ch)
}

// ---------------------------------------------------------------------------
// 0x06 Crawio — raw console I/O.
// EmuTOS: bdos/console.c:170 `xrawio(WORD w)`.
// If `w == 0xFF`, attempt to read a character (non-blocking, raw).
// Otherwise write `w` to the console. Returns the read character
// (low byte) on success, 0 if no input available, or for writes
// returns the byte written.
// ---------------------------------------------------------------------------

/// `Crawio(w)` — raw console I/O without echo or Ctrl-C handling.
///
/// Per `docs/api/gemdos/console/06-crawio.md`: the read path
/// (`w == 0x00FF`) is **non-blocking** and returns the packed console
/// LONG (scancode in bits 16-23, ASCII in bits 0-7) or `0` if no input;
/// the write path returns **`0` always** (not the written byte —
/// EmuTOS `xrawio` falls through to `Bconout(1, w)` and discards
/// the BIOS return).
///
/// The read side uses stdin's device, the write side stdout's
/// (`HXFORM(run->p_uft[0])` / `[1]`, `bdos/console.c:447,451`, [`std_dev`]).
/// `Crawio` is never answered by the file redirection ([`redirect_to_file`],
/// `stdio_typ 0`): a standard handle forced to file handle `h` gives device
/// `3 + h`, which no table entry serves — the read answers `0` (`constat`),
/// the write goes nowhere.
pub fn crawio(args: &TrapArgs) -> Long {
    let w = args.arg_u16(0);
    let _ = host_or_eundev!();
    rawio(w)
}

/// The body of [`crawio`] — EmuTOS `xrawio(parm)` (`bdos/console.c:437-451`)
/// for an in-kernel caller: VDI `v_curtext` writes each character through
/// `Crawio` (`vdi/vdi_esc.c:201-206`). Without a host it does nothing and
/// answers `0`.
pub(crate) fn rawio(w: u16) -> Long {
    let Some(h) = host() else {
        return 0;
    };
    if w == 0x00FF {
        // `constat(i) ? getch(i) : 0` (`bdos/console.c:445-449`): a key in
        // the BDOS typeahead first, else the device's own input — the whole
        // `Bconin` LONG, so a non-ASCII key (arrow / function) keeps its
        // scancode. `constat` said a key is there, so `getch` does not wait.
        let dev = std_dev(STDIN);
        if constat(dev) != 0 { getch(dev) } else { 0 }
    } else {
        let dev = std_dev(STDOUT);
        if dev == DEV_CON {
            h.console_write_byte(w as u8);
        } else {
            chardev::bconout_handle(dev, w as u8);
        }
        0
    }
}

// ---------------------------------------------------------------------------
// Blocking console input (PHASE-02 D3, unblocked by PHASE-06).
//
// For the CON handle the blocking reads return the packed console LONG
// (scancode in bits 16-23, ASCII in bits 0-7 — see `pack_console_key` and
// `docs/api/gemdos/console/01-cconin.md`), built from the IKBD key word
// delivered by `Host::console_read_key_blocking`. Non-ASCII keys
// (arrows / function keys) arrive with ASCII 0 and a non-zero scancode.
// Ctrl-C (ASCII 0x03) maps to `terminate()` = `Pterm(-32)`; a leaf GEMDOS
// function cannot redirect the trap frame, so it records the request via
// [`crate::bdos::process::request_terminate`] and the kernel SVC handler
// performs the redirect after dispatch.
// ---------------------------------------------------------------------------

/// ASCII ETX — Ctrl-C. EmuTOS `terminate()` exit code is `-32`.
const CTRL_C: u8 = 0x03;
/// Exit code EmuTOS passes to `xterm` on a console Ctrl-C
/// (`bdos/console.c:101`, `#define terminate() xterm(-32)`). The `Pexec`
/// caller receives it as `pterm::parent_exit_code(CTRLC_RC)` = 65504 (PROC06).
pub const CTRLC_RC: i32 = -32;
/// ASCII DC2 — Ctrl-R. `cgets` redisplays the current line
/// (`bdos/console.c:571`).
const CTRL_R: u8 = 0x12;
/// ASCII NAK — Ctrl-U. `cgets` kills the line, `retlen = 0`
/// (`bdos/console.c:566`).
const CTRL_U: u8 = 0x15;
/// ASCII CAN — Ctrl-X. `cgets` erases the whole line
/// (`bdos/console.c:560`); `conbrk` flushes the typeahead (`:232`).
const CTRL_X: u8 = 0x18;
/// ASCII DC3 — Ctrl-S. `conbrk` stops output (`bdos/console.c:226`).
const CTRL_S: u8 = 0x13;
/// ASCII DC1 — Ctrl-Q. `conbrk` resumes output (`bdos/console.c:229`).
const CTRL_Q: u8 = 0x11;
/// Carriage return (`cr`, `bdos/console.c:95`).
const CR: u8 = 0x0D;
/// Line feed (`lf`, `bdos/console.c:96`).
const LF: u8 = 0x0A;
/// Horizontal tab (`tab`, `bdos/console.c:97`).
const TAB: u8 = 0x09;
/// Rubout / DEL (`rub`, `bdos/console.c:98`).
const RUB: u8 = 0x7F;
/// Backspace (`bs`, `bdos/console.c:99`).
const BS: u8 = 0x08;
/// The bell `conbrk` rings when the typeahead is full (`Bconout(h, 7)`,
/// `bdos/console.c:239`).
const BEL: u8 = 0x07;

// ---------------------------------------------------------------------------
// 0x01 Cconin — read one char from the console, **with echo**, Ctrl-C
// terminates. EmuTOS: bdos/console.c:386 `xconin()`.
// Spec: docs/api/gemdos/console/01-cconin.md
// ---------------------------------------------------------------------------

/// `Cconin()` — blocking read of one character with echo. Ctrl-C
/// terminates the calling process (`Pterm(-32)`). Returns the packed
/// console LONG (scancode in bits 16-23, ASCII in bits 0-7).
///
/// EmuTOS `xconin` (`bdos/console.c:396-408`) on stdin's device
/// `HXFORM(run->p_uft[0])` ([`std_dev`]): [`conbrk`], then `conin` — the key
/// from [`getch`] (BDOS typeahead first) echoed unconditionally through
/// [`conout`] on the same device (also the NUL of a scancode-only key, and the
/// Ctrl-C itself) — then the Ctrl-C check.
pub fn cconin(_: &TrapArgs) -> Long {
    let dev = std_dev(STDIN);
    if conbrk(dev).is_err() {
        return 0; // value ignored — handler redirects to Pterm.
    }
    // `Bconin` of that entry gives way to a `kbdvec` hook (KEYS02): ask for
    // the restart, nothing has been echoed yet.
    let Some(ch) = getch_restartable(dev) else {
        crate::xbios::kbdvecs::request_restart();
        return 0;
    };
    if conout(dev, u16::from(ch as u8)).is_err() {
        return 0;
    }
    if ch as u8 == CTRL_C {
        crate::bdos::process::request_terminate(CTRLC_RC);
        return 0; // value ignored — handler redirects to Pterm.
    }
    ch
}

// ---------------------------------------------------------------------------
// 0x07 Crawcin — raw console input: blocking, **no echo, no Ctrl-C**.
// EmuTOS: bdos/console.c:367 `xrawcin()`.
// Spec: docs/api/gemdos/console/07-crawcin.md
// ---------------------------------------------------------------------------

/// `Crawcin()` — blocking raw read of one character. No echo, no
/// Ctrl-C handling (a Ctrl-C byte is returned as `0x03`). Returns the
/// packed console LONG (scancode in bits 16-23, ASCII in bits 0-7).
///
/// The device is stdin's, `HXFORM(run->p_uft[0])` (`bdos/console.c:377`).
pub fn crawcin(_: &TrapArgs) -> Long {
    // `getch(h)` (`bdos/console.c:375-378`): the BDOS typeahead first, else
    // `Bconin` of that entry; no `conbrk`.
    match getch_restartable(std_dev(STDIN)) {
        Some(ch) => ch,
        None => {
            crate::xbios::kbdvecs::request_restart();
            0 // discarded — the trap handler re-executes the call
        }
    }
}

// ---------------------------------------------------------------------------
// 0x08 Cnecin — read one char, **no echo**, Ctrl-C terminates.
// EmuTOS: bdos/console.c:411 `xnecin()`.
// Spec: docs/api/gemdos/console/08-cnecin.md
// ---------------------------------------------------------------------------

/// `Cnecin()` — blocking read of one character without echo. Ctrl-C
/// terminates the calling process (`Pterm(-32)`). Returns the packed
/// console LONG (scancode in bits 16-23, ASCII in bits 0-7).
///
/// The device is stdin's, `HXFORM(run->p_uft[0])` (`bdos/console.c:419`).
pub fn cnecin(_: &TrapArgs) -> Long {
    // `xnecin` (`bdos/console.c:414-426`): `conbrk`, `getch`, Ctrl-C check.
    let dev = std_dev(STDIN);
    if conbrk(dev).is_err() {
        return 0; // value ignored — handler redirects to Pterm.
    }
    let Some(key) = getch_restartable(dev) else {
        crate::xbios::kbdvecs::request_restart();
        return 0;
    };
    if key as u8 == CTRL_C {
        crate::bdos::process::request_terminate(CTRLC_RC);
        return 0; // value ignored — handler redirects to Pterm.
    }
    key
}

// ---------------------------------------------------------------------------
// 0x0A Cconrs — read an edited line into a caller buffer.
// EmuTOS: bdos/console.c:519 `xconrs(char *p)` → `cgets()`.
// Spec: docs/api/gemdos/console/0a-cconrs.md
// ---------------------------------------------------------------------------

/// Device-level I/O backend for the [`cconrs`] line editor (EmuTOS `cgets`),
/// abstracted out of the trap path so the editing logic ([`edit_line`]) can
/// be exercised by a QEMU/EL1 selftest against a scripted input + captured
/// echo, without a live console or user-memory access.
///
/// The trap path implements it over a BIOS device and the caller's buffer
/// ([`DeviceLineIo`]: [`getch`], [`conout`], `glbcolumn`); a selftest
/// implements it over an in-memory input script and buffer (see
/// `kernel::selftest_console`).
pub trait LineEditIo {
    /// The next input byte: the low byte of EmuTOS `getch(h)` (`char ch`,
    /// `bdos/console.c:542,548`) — BDOS typeahead first, else a blocking
    /// `Bconin`.
    fn read_byte(&mut self) -> u8;
    /// EmuTOS `conout(h, ch)`: `conbrk`, the byte to the device, the column
    /// update. `Err` when `conbrk` saw a Ctrl-C (terminate requested).
    fn conout(&mut self, b: u8) -> Result<(), Terminated>;
    /// The device's tracked column, `glbcolumn[h]`.
    fn column(&self) -> i16;
    /// Store a data byte at buffer position `i` (0-based within the data
    /// area, i.e. the record's `p[2 + i]`). `i < max` is guaranteed.
    fn store_byte(&mut self, i: usize, b: u8);
    /// Read back a data byte previously stored at position `i` — used by
    /// `backsp` and the Ctrl-R redisplay.
    fn load_byte(&self, i: usize) -> u8;
    /// `true` once a [`Self::store_byte`]/[`Self::load_byte`] could not reach
    /// the buffer (ARCH03 3a: the caller's record is not writable/readable
    /// for it). [`edit_line`] then stops with [`LineEdit::Faulted`]. A
    /// backend over memory that cannot fault keeps the default `false`.
    fn faulted(&self) -> bool {
        false
    }
}

/// Outcome of [`edit_line`].
pub enum LineEdit {
    /// The user finished the line (CR/LF) or filled the buffer; the value
    /// is the number of data bytes stored (the record's `p[1]`).
    Done(usize),
    /// The user pressed Ctrl-C (or `conbrk` saw one during an echo); the
    /// caller must terminate the process (`Pterm(-32)`) and must **not**
    /// write the length byte.
    Terminated,
    /// The record could not be written (or read back): the buffer access
    /// faulted ([`LineEditIo::faulted`]). EmuTOS takes the bus error at the
    /// first such byte; the caller stops the call there.
    Faulted,
}

/// Why [`edit_line`] stopped early.
enum Stop {
    Terminated,
    Faulted,
}

impl From<Terminated> for Stop {
    fn from(_: Terminated) -> Stop {
        Stop::Terminated
    }
}

/// EmuTOS `cookdout(h, ch)` (`bdos/console.c:312-325`) over a
/// [`LineEditIo`]: TAB expanded (`tabout`, `:292-303`), a control byte below
/// `' '` as `^` and `ch | 0x40`, anything else as is.
fn io_cookdout<Io: LineEditIo>(io: &mut Io, ch: u8) -> Result<(), Terminated> {
    if ch == TAB {
        // `tabout`: spaces to the next multiple of 8, at least one.
        loop {
            io.conout(b' ')?;
            if io.column() & 7 == 0 {
                return Ok(());
            }
        }
    }
    let mut ch = ch;
    if ch < b' ' {
        io.conout(b'^')?;
        ch |= 0x40;
    }
    io.conout(ch)
}

/// EmuTOS `newline(h, startcol)` (`bdos/console.c:480-489`): CR, LF, then
/// spaces back to the start column. EmuTOS counts `while (startcol)` down
/// from an `int`, so a negative start column (`glbcolumn` below 0 after a BS
/// at column 0) would write about 2^32 spaces; NewTOS writes them only while
/// `startcol > 0` (pending a user decision, `0a-cconrs.md` "NewTOS
/// implementation notes"; identical for every column >= 0).
fn newline<Io: LineEditIo>(io: &mut Io, startcol: i32) -> Result<(), Terminated> {
    io.conout(CR)?;
    io.conout(LF)?;
    let mut n = startcol;
    while n > 0 {
        io.conout(b' ')?;
        n -= 1;
    }
    Ok(())
}

/// EmuTOS `backsp(h, cbuf, retlen, col)` (`bdos/console.c:494-525`): drop the
/// last byte, recompute the column the remaining line ends at from the start
/// column `col` (TAB to the next multiple of 8, a control byte 2 columns,
/// anything else 1), then `BS SP BS` until the tracked column is there.
fn backsp<Io: LineEditIo>(io: &mut Io, retlen: usize, col: i32) -> Result<usize, Stop> {
    let retlen = retlen.saturating_sub(1);
    let mut col = col;
    for i in 0..retlen {
        let ch = io.load_byte(i);
        if io.faulted() {
            return Err(Stop::Faulted);
        }
        if ch == TAB {
            col += 8;
            col &= !7;
        } else if ch < b' ' {
            col += 2;
        } else {
            col += 1;
        }
    }
    while i32::from(io.column()) > col {
        io.conout(BS)?;
        io.conout(b' ')?;
        io.conout(BS)?;
    }
    Ok(retlen)
}

/// The `Cconrs`/`Fread` line editor — EmuTOS `cgets(h, maxlen, buf)`
/// (`bdos/console.c:540-582`) over any [`LineEditIo`]. The start column is
/// the device's column on entry; keys are read until CR/LF or until `max`
/// bytes are stored:
///
/// | Key                    | Action |
/// |------------------------|--------|
/// | CR (0x0D) / LF (0x0A)  | echo CR, finish, return `Done(len)`. |
/// | BS (0x08) / RUB (0x7F) | `backsp`: drop the last byte, erase back to its column. |
/// | Ctrl-C (0x03)          | return `Terminated`. |
/// | Ctrl-X (0x18)          | `backsp` until the line is empty. |
/// | Ctrl-U (0x15)          | echo `#`, `newline` to the start column, `len = 0`. |
/// | Ctrl-R (0x12)          | echo `#`, `newline` to the start column, re-echo the line. |
/// | any other byte         | store in `buf[len++]`, echo through `cookdout` (TAB expanded, control bytes as `^X`). |
///
/// Every echo goes through [`LineEditIo::conout`], whose `conbrk` may see a
/// Ctrl-C (→ `Terminated`) or pause on Ctrl-S.
pub fn edit_line<Io: LineEditIo>(io: &mut Io, max: usize) -> LineEdit {
    match cgets(io, max) {
        Ok(n) => LineEdit::Done(n),
        Err(Stop::Terminated) => LineEdit::Terminated,
        Err(Stop::Faulted) => LineEdit::Faulted,
    }
}

fn cgets<Io: LineEditIo>(io: &mut Io, max: usize) -> Result<usize, Stop> {
    let stcol = i32::from(io.column());
    let mut retlen: usize = 0;
    while retlen < max {
        let ch = io.read_byte();
        match ch {
            CR | LF => {
                io.conout(CR)?;
                return Ok(retlen);
            }
            BS | RUB => retlen = backsp(io, retlen, stcol)?,
            CTRL_C => return Err(Stop::Terminated),
            CTRL_X => loop {
                retlen = backsp(io, retlen, stcol)?;
                if retlen == 0 {
                    break;
                }
            },
            CTRL_U => {
                io.conout(b'#')?;
                newline(io, stcol)?;
                retlen = 0;
            }
            CTRL_R => {
                io.conout(b'#')?;
                newline(io, stcol)?;
                for i in 0..retlen {
                    let c = io.load_byte(i);
                    if io.faulted() {
                        return Err(Stop::Faulted);
                    }
                    io_cookdout(io, c)?;
                }
            }
            _ => {
                io.store_byte(retlen, ch);
                if io.faulted() {
                    return Err(Stop::Faulted);
                }
                retlen += 1;
                io_cookdout(io, ch)?;
            }
        }
    }
    Ok(retlen)
}

/// `Cconrs(p)` — read an edited line into the caller's record.
///
/// `p[0]` (IN) = max length, `p[1]` (OUT) = actual length, `p[2..]` =
/// buffer. Blocking, with the [`edit_line`] line editor (EmuTOS `cgets`).
/// The buffer is **not** NUL-terminated — callers use `p[1]` as the length
/// (TOS contract). Returns `0` (`xconrs` is `void`).
///
/// The device is stdin's, `HXFORM(run->p_uft[0])` (`bdos/console.c:535`,
/// [`std_dev`]), for the input and for the echo. A BIOS handle with no table
/// entry (`p_uft[0]` below `H_Print`) answers `Bconin` 0 and refuses the echo
/// (`bios/bios.c:973-976,1008-1010`), so `cgets` stores NULs until `max`.
pub fn cconrs(args: &TrapArgs) -> Long {
    let _ = host_or_eundev!();
    let p = args.arg_u64(0);
    if p == 0 {
        return 0;
    }
    // The record is in the caller's space; a record the program cannot read
    // or write ends the call at the first such byte and — through the syscall
    // tail — the program (EmuTOS: bus error in `cgets`,
    // `docs/api/bios/_exception-crash.md`).
    let Ok(max) = args.space.get_u8(p) else {
        return crate::uaccess::FAULTED;
    };
    let buf = SpaceBuf { space: args.space, base: p + 2 };
    let mut io = DeviceLineIo { dev: std_dev(STDIN), buf: &buf, faulted: core::cell::Cell::new(false) };
    let len = match edit_line(&mut io, max as usize) {
        LineEdit::Done(n) => n,
        LineEdit::Terminated => {
            crate::bdos::process::request_terminate(CTRLC_RC);
            return 0; // value ignored — handler redirects to Pterm.
        }
        LineEdit::Faulted => return crate::uaccess::FAULTED,
    };
    // `p + 1` is the record's length byte.
    if args.space.put_u8(p + 1, len as u8).is_err() {
        return crate::uaccess::FAULTED;
    }
    0
}

// ---------------------------------------------------------------------------
// 0x09 Cconws — write a NUL-terminated string.
// EmuTOS: bdos/console.c:96  `xconws(char *s)`.
// Returns the number of bytes written (LONG).
// Spec: docs/api/gemdos/console/09-cconws.md
// ---------------------------------------------------------------------------

/// `Cconws(s)` — write zero-terminated string `s` to the console.
/// Returns the number of string bytes written (D-009; a TAB counts once).
///
/// EmuTOS `xconws` → `prt_line(HXFORM(run->p_uft[1]), s)` = `tabout` per
/// byte (`bdos/console.c:459-472`, [`std_dev`]): tab expansion from the
/// tracked column, and before every byte [`conbrk`] — a pending Ctrl-C stops
/// the write (the rest of the string is not emitted) and requests
/// `Pterm(-32)`, Ctrl-S pauses until Ctrl-Q, other keys go into the BDOS
/// typeahead.
///
/// # User pointer
///
/// `s` is read in the caller's space ([`crate::trap::TrapArgs::space`]):
/// an EL0 program's pointer with fault-safe unprivileged loads
/// ([`crate::uaccess`]), an in-kernel caller's as the kernel address it is.
/// A pointer the program cannot read ends the call and — through the
/// kernel's syscall tail — the caller (EmuTOS: bus error in the trap → `dopanic` →
/// `Pterm(-1)`, `docs/api/bios/_exception-crash.md`).
pub fn cconws(args: &TrapArgs) -> Long {
    let mut p = args.arg_ptr(0);
    if p.is_null() {
        return 0;
    }
    let _ = host_or_eundev!();
    let dev = std_dev(STDOUT);
    let mut count: Long = 0;
    // Hard cap to prevent runaway loops if the caller forgot the NUL.
    // 64 KiB matches TOS's effective practical limit (Cconrs buffers).
    const MAX: Long = 65_535;
    while count < MAX {
        // `s` is a *user* pointer: read through `uaccess` (unprivileged
        // `ldtrb`, permission-checked as EL0 — the access the trap contract
        // calls for). A bad pointer no longer halts the machine (ARCH03 0c):
        // the read faults recoverably, the bytes before it are already
        // written (as on EmuTOS, whose `prt_line` takes the bus error in the
        // middle of the string), and the syscall tail terminates the caller
        // with the crash report — the count returned here never reaches it.
        let Ok(b) = args.space.get_u8(p as u64) else {
            break;
        };
        if b == 0 {
            break;
        }
        if tabout(dev, u16::from(b)).is_err() {
            break; // Ctrl-C: stop writing; SVC handler redirects to Pterm.
        }
        // SAFETY: same provenance as `p`; bounded by `MAX` above.
        p = unsafe { p.add(1) };
        count += 1;
    }
    count
}

// ---------------------------------------------------------------------------
// 0x0B Cconis — non-blocking console-input status.
// EmuTOS: bdos/console.c:108 `xconstat()` → `Bconstat(2)`.
// Returns -1 if input ready, 0 otherwise. (LONG, but spec uses the
// "all-bits-set" form which is sign-extended.)
// ---------------------------------------------------------------------------

/// `Cconis()` — `constat(HXFORM(run->p_uft[0]))` (`bdos/console.c:161`):
/// by default `Bconstat(2)` of the CON entry, `-1` if a console byte is
/// available, else `0` (NewTOS has no BDOS typeahead buffer to check first);
/// a stdin device other than AUX/CON answers `0` ([`constat`]).
pub fn cconis(_: &TrapArgs) -> Long {
    constat(std_dev(STDIN))
}

// ---------------------------------------------------------------------------
// 0x10 Cconos — console-output status. EmuTOS routes to Bcostat(2).
// ---------------------------------------------------------------------------

/// `Cconos()` — `Bcostat(HXFORM(run->p_uft[1]))` (`bdos/console.c:170`): by
/// default the CON entry, `-1` (the console is always ready to accept a
/// byte).
pub fn cconos(_: &TrapArgs) -> Long {
    chardev::bcostat_handle(std_dev(STDOUT))
}

// ---------------------------------------------------------------------------
// 0x11 Cprnos — printer output status = Bcostat(0).
// Spec: docs/api/gemdos/console/11-cprnos.md (FS11 note).
// ---------------------------------------------------------------------------

/// `Cprnos()` — `Bcostat(HXFORM(run->p_uft[3]))` (`bdos/console.c:179`): by
/// default the PRN entry; `0` (not ready) for the absent printer, as EmuTOS
/// without `CONF_WITH_PRINTER_PORT` (`bios/parport.c:139-140`).
pub fn cprnos(_: &TrapArgs) -> Long {
    chardev::bcostat_handle(std_dev(STDPRN))
}

// ---------------------------------------------------------------------------
// 0x12 Cauxis — AUX input status = constat(1) → Bconstat(1).
// Spec: docs/api/gemdos/console/12-cauxis.md (FS11 note).
// ---------------------------------------------------------------------------

/// `Cauxis()` — `constat(HXFORM(run->p_uft[2]))` (`bdos/console.c:188`): by
/// default `Bconstat(1)` of the AUX entry (no BDOS typeahead buffer to check
/// first); `0` — the no-cable AUX entry never has input. A stdaux device
/// other than AUX/CON answers `0` ([`constat`]).
pub fn cauxis(_: &TrapArgs) -> Long {
    constat(std_dev(STDAUX))
}

// ---------------------------------------------------------------------------
// 0x13 Cauxos — AUX output status = Bcostat(1).
// Spec: docs/api/gemdos/console/13-cauxos.md (FS11 note).
// ---------------------------------------------------------------------------

/// `Cauxos()` — `Bcostat(HXFORM(run->p_uft[2]))` (`bdos/console.c:197`): by
/// default the AUX entry, `-1` (ready) for the no-cable entry
/// (`docs/api/gemdos/console/13-cauxos.md`, option A).
pub fn cauxos(_: &TrapArgs) -> Long {
    chardev::bcostat_handle(std_dev(STDAUX))
}

// ---------------------------------------------------------------------------
// Fread / Fwrite on a character-device handle (FS11).
// EmuTOS: bdos/bdosmain.c:604-640 (`osif` device block) with `conin`
// (bdos/console.c:384-390), `cgets` (:540-581) and `tabout` (:292-302).
// Spec: docs/api/gemdos/file/{3f-fread,40-fwrite}.md "Character-device path"
// and "NewTOS implementation notes".
// ---------------------------------------------------------------------------

/// BDOS handle of the console (`H_Console`, `bdos/console.h`).
const H_CONSOLE: i16 = -1;

/// BDOS character-device handle (`-1` CON, `-2` AUX, `-3` PRN) → BIOS device
/// number: EmuTOS `HXFORM(h) = 3 + h` (`bdos/console.h:36`).
fn hxform(numl: i16) -> u16 {
    (3 + numl) as u16
}

/// Byte access to the caller's buffer for the device paths: an index into
/// the buffer, `None` / `false` when that byte cannot be reached (ARCH03 3a).
trait DevBuf {
    fn get(&self, i: u64) -> Option<u8>;
    fn put(&self, i: u64, b: u8) -> bool;
}

/// [`DevBuf`] over the caller's address space.
struct SpaceBuf {
    space: crate::uaccess::Space,
    base: u64,
}

impl DevBuf for SpaceBuf {
    fn get(&self, i: u64) -> Option<u8> {
        self.space.get_u8(self.base + i).ok()
    }
    fn put(&self, i: u64, b: u8) -> bool {
        self.space.put_u8(self.base + i, b).is_ok()
    }
}

/// [`LineEditIo`] over BIOS device `dev` and the caller's buffer — the
/// `cgets(h, …)` of `Cconrs` and `Fread`: input from [`getch`] (`char ch`,
/// `console.c:542`), echo through [`conout`], the column `glbcolumn[dev]`.
struct DeviceLineIo<'a, B: DevBuf> {
    dev: u16,
    buf: &'a B,
    faulted: core::cell::Cell<bool>,
}

impl<B: DevBuf> LineEditIo for DeviceLineIo<'_, B> {
    fn read_byte(&mut self) -> u8 {
        // Never gives way to a `kbdvec` hook: keys may already be stored.
        getch(self.dev) as u8
    }
    fn conout(&mut self, b: u8) -> Result<(), Terminated> {
        conout(self.dev, u16::from(b))
    }
    fn column(&self) -> i16 {
        glbcolumn(self.dev).load(Ordering::Relaxed)
    }
    fn store_byte(&mut self, i: usize, b: u8) {
        if !self.buf.put(i as u64, b) {
            self.faulted.set(true);
        }
    }
    fn load_byte(&self, i: usize) -> u8 {
        self.buf.get(i as u64).unwrap_or_else(|| {
            self.faulted.set(true);
            0
        })
    }
    fn faulted(&self) -> bool {
        self.faulted.get()
    }
}

/// `Fread` on a character-device handle `numl` (`-1..-3`): `len` bytes into
/// `ubuf` in `space` (EmuTOS `osif`, `bdos/bdosmain.c:608-620`).
pub(crate) fn device_read(numl: i16, space: crate::uaccess::Space, len: i32, ubuf: u64) -> Long {
    device_read_with(numl, len, &SpaceBuf { space, base: ubuf })
}

fn device_read_with<B: DevBuf>(numl: i16, len: i32, buf: &B) -> Long {
    let dev = hxform(numl);
    if chardev::device(dev).is_none() {
        return 0;
    }
    // "HUGE read": the high word of the LONG is tested (`pw[2]`), which also
    // catches every negative `len`.
    if (len as u32) >> 16 != 0 {
        return 0;
    }
    // `pw[3]` is a signed WORD: 32768..65535 are negative here.
    let lo = len as u16 as i16;
    if lo == 1 {
        // `conin(dev)` (`console.c:384-390`): one key from `getch` (BDOS
        // typeahead first), echoed through `conout`, stored as its low byte —
        // CR and Ctrl-C included, no line editing. No side effect happens
        // before the key arrives, so the read may give way to a `kbdvec` hook
        // (KEYS02) and be re-executed.
        let Some(key) = getch_restartable(dev) else {
            crate::xbios::kbdvecs::request_restart();
            return 0; // discarded — the trap handler re-executes the call
        };
        let ch = key as u8;
        if conout(dev, u16::from(ch)).is_err() {
            return 0; // Ctrl-C in the echo's `conbrk`: handler redirects to Pterm.
        }
        if !buf.put(0, ch) {
            return crate::uaccess::FAULTED;
        }
        return 1;
    }
    if lo <= 0 {
        // `cgets` with `maxlen <= 0`: the loop never runs (`console.c:546`).
        return 0;
    }
    // `cgets(dev, len, buf)` = the `Cconrs` line editor over this device.
    let mut io = DeviceLineIo { dev, buf, faulted: core::cell::Cell::new(false) };
    match edit_line(&mut io, lo as usize) {
        LineEdit::Done(n) => n as Long,
        LineEdit::Terminated => {
            crate::bdos::process::request_terminate(CTRLC_RC);
            0 // value ignored — handler redirects to Pterm.
        }
        LineEdit::Faulted => crate::uaccess::FAULTED,
    }
}

/// `Fwrite` on a character-device handle `numl` (`-1..-3`): `count` bytes
/// from `ubuf` in `space` (EmuTOS `osif`, `bdos/bdosmain.c:622-640`).
///
/// `count <= 0` returns `count` itself. Each byte is read from the buffer
/// first (an unreadable byte → [`crate::uaccess::FAULTED`], the bytes before
/// it stay written). CON: [`tabout`] (`:631`; a Ctrl-C in its `conbrk` ends
/// the call with the number of bytes before it and requests the terminate),
/// result `count`. Any other device: the raw byte to its `Bconout`, no break
/// check; a `0` from `Bconout` ends the call with the number of bytes before
/// it.
pub(crate) fn device_write(numl: i16, space: crate::uaccess::Space, count: i32, ubuf: u64) -> Long {
    device_write_with(numl, count, &SpaceBuf { space, base: ubuf })
}

fn device_write_with<B: DevBuf>(numl: i16, count: i32, buf: &B) -> Long {
    if count <= 0 {
        return count;
    }
    let dev = hxform(numl);
    let Some(d) = chardev::device(dev) else {
        return 0;
    };
    for n in 0..count as u64 {
        let Some(b) = buf.get(n) else {
            return crate::uaccess::FAULTED;
        };
        if numl == H_CONSOLE {
            if tabout(dev, u16::from(b)).is_err() {
                return n as Long; // Ctrl-C: SVC handler redirects to Pterm.
            }
        } else if d.bconout(b) == 0 {
            return n as Long;
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Scripted line-editor backend whose buffer becomes unreachable from
    /// byte `fault_at` on (ARCH03 3a: the caller's record is not writable).
    struct FaultingIo {
        script: &'static [u8],
        at: usize,
        buf: [u8; 16],
        fault_at: usize,
        faulted: bool,
        echoed: usize,
        col: i16,
    }

    impl LineEditIo for FaultingIo {
        fn read_byte(&mut self) -> u8 {
            let b = self.script[self.at];
            self.at += 1;
            b
        }
        fn conout(&mut self, b: u8) -> Result<(), Terminated> {
            self.echoed += 1;
            // The `conout` column rule.
            match b {
                0x20..=0xFF => self.col += 1,
                CR => self.col = 0,
                BS => self.col -= 1,
                _ => {}
            }
            Ok(())
        }
        fn column(&self) -> i16 {
            self.col
        }
        fn store_byte(&mut self, i: usize, b: u8) {
            if i >= self.fault_at {
                self.faulted = true;
            } else {
                self.buf[i] = b;
            }
        }
        fn load_byte(&self, i: usize) -> u8 {
            self.buf[i]
        }
        fn faulted(&self) -> bool {
            self.faulted
        }
    }

    #[test]
    fn edit_line_stops_at_the_first_store_that_faults() {
        let mut io = FaultingIo { script: b"abcd\r", at: 0, buf: [0; 16], fault_at: 2, faulted: false, echoed: 0, col: 0 };
        assert!(matches!(edit_line(&mut io, 10), LineEdit::Faulted));
        // 'a' and 'b' were stored and echoed; 'c' faulted and was not echoed,
        // and no further key was read.
        assert_eq!(&io.buf[..2], b"ab");
        assert_eq!(io.echoed, 2);
        assert_eq!(io.at, 3);
    }

    /// A device-path buffer whose bytes from `fault_at` on are unreachable.
    struct FaultBuf {
        data: [core::cell::Cell<u8>; 8],
        fault_at: u64,
    }

    impl FaultBuf {
        fn new(fault_at: u64) -> FaultBuf {
            FaultBuf { data: Default::default(), fault_at }
        }
    }

    impl DevBuf for FaultBuf {
        fn get(&self, i: u64) -> Option<u8> {
            (i < self.fault_at).then(|| self.data[i as usize].get())
        }
        fn put(&self, i: u64, b: u8) -> bool {
            if i >= self.fault_at {
                return false;
            }
            self.data[i as usize].set(b);
            true
        }
    }

    /// WT4 (`40-fwrite.md`): the byte is read from the user buffer before the
    /// device call — an unreadable byte 0 is `FAULTED`, not the device's
    /// result (the absent PRN's short count 0, the no-cable AUX's count).
    #[test]
    fn wt4_fwrite_reads_the_byte_before_the_device_call() {
        for numl in [-2, -3] {
            assert_eq!(device_write_with(numl, 3, &FaultBuf::new(0)), crate::uaccess::FAULTED);
        }
        // Readable: the absent PRN refuses the first byte → short count 0;
        // the no-cable AUX accepts every byte → 3 (`3f-fread.md` item 1a).
        assert_eq!(device_write_with(-3, 3, &FaultBuf::new(8)), 0);
        assert_eq!(device_write_with(-2, 3, &FaultBuf::new(8)), 3);
    }

    /// The device read stops with `FAULTED` at the first byte it cannot store
    /// (ARCH03 3a, `3f-fread.md` implementation notes).
    #[test]
    fn fread_device_store_fault() {
        assert_eq!(device_read_with(-2, 1, &FaultBuf::new(0)), crate::uaccess::FAULTED);
        let b = FaultBuf::new(2);
        assert_eq!(device_read_with(-3, 4, &b), crate::uaccess::FAULTED);
        assert_eq!(device_read_with(-3, 2, &FaultBuf::new(2)), 2);
    }

    #[test]
    fn edit_line_without_faults_is_unchanged() {
        let mut io = FaultingIo { script: b"ab\x08c\r", at: 0, buf: [0; 16], fault_at: 16, faulted: false, echoed: 0, col: 0 };
        assert!(matches!(edit_line(&mut io, 10), LineEdit::Done(2)));
        assert_eq!(&io.buf[..2], b"ac");
    }
}
