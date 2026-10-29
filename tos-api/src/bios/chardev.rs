//! BIOS character devices — `Bconstat` / `Bconin` / `Bconout` /
//! `Bcostat` (BIOS 0x01 / 0x02 / 0x03 / 0x08).
//!
//! Spec: `docs/api/bios/{01,02,03,08}-*.md`; the device table and the
//! absent-device profile: `docs/api/gemdos/file/3f-fread.md` "NewTOS
//! implementation notes" (FS11). EmuTOS dispatches each call through a
//! per-handle vector table (`bconstat_vec[]` / `bconin_vec[]` /
//! `bconout_vec[]` / `bcostat_vec[]`, `bios/bios.c:932,976,1009,1161`,
//! filled from `bios/chardev.c:30-45`). NewTOS keeps **one** table of
//! [`CharDevice`] entries indexed by BIOS device number — PRN 0, AUX 1,
//! CON 2, MIDI 3, IKBD 4, raw-CON 5 — and every character-device path goes
//! through it: the BIOS calls here, `Fread`/`Fwrite` on device handles, the
//! `Ccon*`/`Caux*`/`Cprn*` GEMDOS calls and `Midiws`. A later
//! UART/MIDI/printer driver (`TASK/IO01.md`) replaces only its table entry
//! ([`install_device`]).
//!
//! Handles 6 and up are the `Bconmap` table's (BIOS08; EmuTOS'
//! `BCONMAP_AVAILABLE` branch, `bios/bios.c:923-928,967-972,1000-1005,
//! 1152-1157`, which runs before the vector tables): handle `6 + i` is
//! entry `i` of the table, every handle past it answers `0`. The NewTOS
//! table has one entry, the AUX port ([`crate::xbios::bconmap`]), so handle
//! 6 answers as handle 1 — through the same AUX entry, a driver in slot 1
//! included — and handles 7 and up answer `0` ([`device`]).
//!
//! The NewTOS profile has one real character device — the console (PL011 +
//! framebuffer): keyboard **input** on CON (2) only, screen **output** on
//! CON (2) and raw-CON (5). AUX (1) is a serial port with **no cable**
//! ([`AuxNoCable`], `3f-fread.md` item 1a): output accepted and discarded,
//! always ready, no input — on qemu-virt the kernel replaces it at boot with
//! a driver that reads the serial input ring of `Iorec(0)` (XBIOS15,
//! `kernel::console::AuxSerial`). MIDI (3) and IKBD (4) answer as on the
//! reference machine, whose MIDI and IKBD ACIAs always exist (BIOS04, user
//! decision 2026-10-29): output ready and accepted, the byte discarded
//! ([`MidiPort`], [`IkbdPort`]); MIDI input is the `Iorec(2)` ring, which
//! `Bconin(3)` waits on, IKBD input is `char_dummy`. PRN is the
//! documented **absent device** ([`Absent`], EmuTOS' `char_dummy` /
//! `charout_dummy`, `bios/chardev.c:49-58`): status 0, input 0 without
//! blocking, output discarded with 0 — rather than blocking on hardware that
//! does not exist. The console entries and the MIDI wait answer 0 before a
//! host is installed (EmuTOS' `CHARDEV_AVAILABLE` boot gate,
//! `bios/bios.c:920-921,964-965,997-998`).

use core::sync::atomic::{AtomicPtr, Ordering};

use tos_types::Long;

use crate::host;
use crate::trap::TrapArgs;
use crate::xbios::{bconmap, iorec, kbdvecs};

// ---------------------------------------------------------------------------
// The device table.
// ---------------------------------------------------------------------------

/// BIOS device number of the printer port (`PRN:`).
pub const DEV_PRN: u16 = 0;
/// BIOS device number of the serial port (`AUX:`).
pub const DEV_AUX: u16 = 1;
/// BIOS device number of the console (`CON:`).
pub const DEV_CON: u16 = 2;
/// BIOS device number of the MIDI port.
pub const DEV_MIDI: u16 = 3;
/// BIOS device number of the keyboard controller (IKBD).
pub const DEV_IKBD: u16 = 4;
/// BIOS device number of the raw console (screen without VT52).
pub const DEV_RAWCON: u16 = 5;
/// Size of the table: devices 0..5. EmuTOS' `bcon*_vec[]` have eight slots,
/// but with `BCONMAP_AVAILABLE` (the reference build) handles 6 and 7 never
/// reach them — the `Bconmap` branch takes every handle `>= 6` first
/// (`bios/bios.c:923-928`) — so the table ends at 5 and [`device`] routes
/// the rest.
const NDEV: usize = 6;

/// One BIOS character device — the four routines EmuTOS keeps per handle in
/// its `bcon*_vec[]` tables. Return values are the BIOS contract
/// (`docs/api/bios/{01,02,03,08}-*.md`).
pub trait CharDevice: Sync {
    /// `Bconstat`: `-1` when an input byte is ready, else `0`.
    fn bconstat(&self) -> Long;
    /// `Bconin`: the next input byte (for CON the packed key LONG). Blocks
    /// on a real device until input arrives, and **never gives way** — the
    /// read the GEMDOS line reader (`Fread` → `cgets`) uses.
    fn bconin(&self) -> Long;
    /// `Bconin` as the trap entry point: like [`Self::bconin`], but `None`
    /// when the read has to give way to the running process's `kbdvec` hook
    /// (KEYS02; the caller then requests a restart, see
    /// [`crate::xbios::kbdvecs::request_restart`]). Only for reads with no
    /// side effect before the byte arrives. Default: never gives way.
    fn bconin_restartable(&self) -> Option<Long> {
        Some(self.bconin())
    }
    /// `Bconout`: send `byte`; non-zero = accepted, `0` = refused/absent.
    fn bconout(&self, byte: u8) -> Long;
    /// The device's own output status (`-1` ready, `0` busy/absent). Note
    /// that `Bcostat(3)`/`Bcostat(4)` swap slots ([`bcostat_handle`]).
    fn bcostat(&self) -> Long;
}

/// The documented absent device (deliberate profile, not a stub): EmuTOS'
/// `char_dummy` / `charout_dummy` (`bios/chardev.c:49-58`), also its Bconmap
/// dummy map entry for AUX (`bios/serport.c:96`) and `bconin0`/`bcostat0`/
/// `bconout0` without a printer port (`bios/parport.c:123-172`). Every call
/// answers `0` at once: no input ready, input `0` without blocking, output
/// discarded and refused, output not ready.
pub struct Absent;

impl CharDevice for Absent {
    fn bconstat(&self) -> Long {
        0
    }
    fn bconin(&self) -> Long {
        0
    }
    fn bconout(&self, _byte: u8) -> Long {
        0
    }
    fn bcostat(&self) -> Long {
        0
    }
}

/// AUX (device 1) as "port present, no cable" (deliberate profile, not a
/// stub; `docs/api/gemdos/file/3f-fread.md` "NewTOS implementation notes"
/// item 1a, option A). On a real ST the RS-232 port always exists:
/// EmuTOS' `bconout1` with an MFP port queues the byte and returns `1`
/// (`bios/serport.c:289-330`, no CTS wait — `flowctrl` is never read) and
/// `bcostat1` is `-1` while the out-ring has room (`serport.c:271-283`);
/// NewTOS discards at once, so the "ring" never fills. With no cable no byte
/// ever arrives: `bconstat1` on an empty iorec is `0` (`serport.c:255-258`).
/// `Bconin` returns `0` **without blocking** — a documented divergence from
/// `bconin1`, which would wait forever (`serport.c:260-263`). An absent AUX
/// (`Bconout` 0) made `Fwrite(2, …)` return 0 and mintlib's stderr write
/// loop retry forever (FS11 acceptance finding 1). A UART driver
/// (`TASK/IO01.md`) replaces the whole entry — on qemu-virt the kernel's
/// `AuxSerial` does (XBIOS15; it keeps this entry's output side).
pub struct AuxNoCable;

impl CharDevice for AuxNoCable {
    fn bconstat(&self) -> Long {
        0
    }
    fn bconin(&self) -> Long {
        0
    }
    fn bconout(&self, _byte: u8) -> Long {
        1
    }
    fn bcostat(&self) -> Long {
        -1
    }
}

/// MIDI (device 3) as on the reference machine, whose MIDI ACIA always
/// exists (deliberate profile, not a stub; BIOS04, user decision
/// 2026-10-29): EmuTOS' `bconstat3`/`bconin3`/`bcostat3`/`bconout3`
/// (`bios/midi.c:22-95`) with `CONF_WITH_MIDI_ACIA` = 1, the oracle build
/// (`include/config.h:994-996`).
///
/// - `Bconstat(3)`: `-1` while the MIDI ring of `Iorec(2)` holds a byte, else
///   `0` (`bconstat3`, `:22-33`). No kernel producer fills that ring, so it
///   is `0` unless a program put a byte there ([`iorec::midi_pending`]).
/// - `Bconin(3)`: waits until the ring holds a byte and returns it
///   zero-extended (`bconin3`, `:35-62`) — with no MIDI driver that is
///   forever, as on EmuTOS with nothing on the MIDI cable. The wait keeps
///   the input pumped and, on the trap path, gives way to a `kbdvec` hook,
///   like the AUX wait on qemu-virt (XBIOS15/XBIOS18, [`midi_read`]).
/// - Output status: `-1` (`bcostat3` with an empty ACIA transmit register,
///   `:66-81`); [`bcostat_handle`]'s swap makes it the answer of
///   `Bcostat(4)`.
/// - `Bconout(3)`: the byte is discarded — there is no MIDI transmitter —
///   and the call returns `1` (`bconout3`, `:84-95`).
///
/// A MIDI driver (`TASK/IO01.md`) replaces the whole entry and fills the
/// ring.
pub struct MidiPort;

impl CharDevice for MidiPort {
    fn bconstat(&self) -> Long {
        if iorec::midi_pending() {
            -1
        } else {
            0
        }
    }
    fn bconin(&self) -> Long {
        // Never gives way: `midi_read(false)` only returns with a byte (or
        // at once before a host is installed).
        midi_read(false).unwrap_or(0)
    }
    fn bconin_restartable(&self) -> Option<Long> {
        midi_read(true)
    }
    fn bconout(&self, _byte: u8) -> Long {
        1
    }
    fn bcostat(&self) -> Long {
        -1
    }
}

/// `bconin3`'s wait (`bios/midi.c:37-38`, `while (!bconstat3());`) and read:
/// the oldest byte of the MIDI ring ([`iorec::pop_midi`]), zero-extended.
/// Between checks the host pumps its input with interrupts briefly open
/// ([`host::Host::pump_input`]: timer tick, USB keyboard and mouse), as
/// EmuTOS' interrupts keep running while `bconin3` spins. Like `bconin3`
/// and the AUX wait it never gives the CPU to another process (user
/// 2026-10-29, XBIOS18).
///
/// With `give_way` (the trap entry, [`MidiPort`]'s `bconin_restartable`),
/// returns `None` as soon as the running process has bytes queued for its
/// `kbdvec` hook ([`kbdvecs::pending_for_running`]; KEYS02/XBIOS18,
/// `docs/api/xbios/22-kbdvbase.md` C3 "O1"): EmuTOS' IKBD interrupt jumps
/// through `kbdvec` while `bconin3` spins, NewTOS delivers the bytes on the
/// way back to EL0 — the caller requests a restart, the hook runs, and the
/// `svc` waits again. Nothing is consumed before the byte arrives, so the
/// restart has no side effect.
///
/// Before a host is installed it returns `0` at once: EmuTOS' boot gate
/// (`CHARDEV_AVAILABLE`, `bios/bios.c:964-965`), and with no host there is
/// nothing to pump.
fn midi_read(give_way: bool) -> Option<Long> {
    let Some(h) = host::host() else {
        return Some(0);
    };
    loop {
        if let Some(b) = iorec::pop_midi() {
            return Some(Long::from(b));
        }
        h.pump_input();
        if give_way && kbdvecs::pending_for_running() {
            return None;
        }
    }
}

/// IKBD (device 4) as on the reference machine, whose IKBD ACIA always exists
/// (deliberate profile, not a stub; BIOS04, user decision 2026-10-29;
/// `CONF_WITH_IKBD_ACIA` = 1 in the oracle build, `include/config.h:1001-1003`):
/// output ready and accepted, the byte discarded — EmuTOS' `bcostat4`
/// (`bios/ikbd.c:916-930`, `-1` with an empty ACIA transmit register, and
/// `-1` "OK (but output will be ignored)" on a build without one) and
/// `bconout4` (`:933-937`, `ikbd_writeb` and `1`). NewTOS has no IKBD
/// transmitter and no IKBD command interpreter (`docs/api/xbios/19-ikbdws.md`),
/// so the byte has no effect. Input is `char_dummy` (`bconstat_init[4]`,
/// `bconin_init[4]`, `bios/chardev.c:30-36`): `Bconstat(4)` and `Bconin(4)`
/// are `0` at once. [`bcostat_handle`]'s swap makes the output status the
/// answer of `Bcostat(3)`.
pub struct IkbdPort;

impl CharDevice for IkbdPort {
    fn bconstat(&self) -> Long {
        0
    }
    fn bconin(&self) -> Long {
        0
    }
    fn bconout(&self, _byte: u8) -> Long {
        1
    }
    fn bcostat(&self) -> Long {
        -1
    }
}

/// CON (device 2): keyboard input from the console key ring (`bconin2` /
/// `bconstat2`, the IKBD queue), screen output (`bconout2`), output always
/// ready (`bcostat2`, `bios/chardev.c:112-118`: the PL011 TX drains by
/// polling). The only console-input device: IKBD (4) is an *output* channel
/// whose input slot is `char_dummy` (`bconin_init[4]`,
/// `bios/chardev.c:34-36`) — treating 4 as console input made `Bconin(4)`
/// block forever (the handle-4 hang).
struct Console;

impl CharDevice for Console {
    fn bconstat(&self) -> Long {
        match host::host() {
            Some(h) if h.console_has_input() => -1,
            _ => 0,
        }
    }
    fn bconin(&self) -> Long {
        match host::host() {
            Some(h) => pack_console_key(h.console_read_entry_blocking()),
            None => 0,
        }
    }
    fn bconin_restartable(&self) -> Option<Long> {
        match host::host() {
            Some(h) => h.console_read_entry_restartable().map(pack_console_key),
            None => Some(0),
        }
    }
    fn bconout(&self, byte: u8) -> Long {
        match host::host() {
            Some(h) => {
                h.console_write_byte(byte);
                1
            }
            None => 0,
        }
    }
    fn bcostat(&self) -> Long {
        match host::host() {
            Some(_) => -1,
            None => 0,
        }
    }
}

/// Raw-CON (device 5): screen output without VT52 (`bconout5` →
/// `ascii_out`, `bios/chardev.c:97-106`); its input and output-status slots
/// are `char_dummy` (`bconin_init[5]`, `bcostat_init[5]`), so status and
/// input are `0` — `Bconout(5)` writes and returns `1`, `Bcostat(5)` is `0`.
///
/// The byte goes straight to the alpha screen's `ascii_out`
/// ([`crate::vdi::alpha_rawout`]): every byte is a glyph, no escape
/// automaton runs and its state is left as it is (`docs/api/bios/_vt52.md`
/// §9, decision D5). Like EmuTOS' `bconout5` (without
/// `CONF_SERIAL_CONSOLE`) it is not mirrored to the serial line.
struct RawConsole;

impl CharDevice for RawConsole {
    fn bconstat(&self) -> Long {
        0
    }
    fn bconin(&self) -> Long {
        0
    }
    fn bconout(&self, byte: u8) -> Long {
        match host::host() {
            Some(h) => {
                if let Some(b) = h.raster() {
                    crate::vdi::alpha_rawout(b, byte);
                }
                1
            }
            None => 0,
        }
    }
    fn bcostat(&self) -> Long {
        0
    }
}

static ABSENT: &dyn CharDevice = &Absent;
static AUX_NO_CABLE: &dyn CharDevice = &AuxNoCable;
static CONSOLE: &dyn CharDevice = &Console;
static MIDI_PORT: &dyn CharDevice = &MidiPort;
static IKBD_PORT: &dyn CharDevice = &IkbdPort;
static RAW_CONSOLE: &dyn CharDevice = &RawConsole;

/// A table cell: a pointer to a `&'static dyn CharDevice` (a fat pointer
/// cannot be swapped atomically, the cell holding it can) — the same scheme
/// [`crate::host`] uses for the host.
type Slot = AtomicPtr<&'static dyn CharDevice>;

const fn slot(entry: &'static &'static dyn CharDevice) -> Slot {
    AtomicPtr::new(entry as *const &'static dyn CharDevice as *mut &'static dyn CharDevice)
}

/// The table, indexed by BIOS device number (EmuTOS' `bcon*_init[]`,
/// `bios/chardev.c:30-45`).
static DEVICES: [Slot; NDEV] = [
    slot(&ABSENT),       // 0 PRN
    slot(&AUX_NO_CABLE), // 1 AUX
    slot(&CONSOLE),      // 2 CON
    slot(&MIDI_PORT),    // 3 MIDI
    slot(&IKBD_PORT),    // 4 IKBD
    slot(&RAW_CONSOLE),  // 5 raw CON
];

/// Replace the table entry of BIOS device `dev` with `entry` — the one place a
/// driver (`TASK/IO01.md`) plugs in. Returns the previous entry, or `None`
/// (nothing changed) when `dev` is not a table slot (`> 5`; handles 6 and up
/// are the `Bconmap` table's, [`device`]).
pub fn install_device(
    dev: u16,
    entry: &'static &'static dyn CharDevice,
) -> Option<&'static &'static dyn CharDevice> {
    let cell = DEVICES.get(dev as usize)?;
    let new = entry as *const &'static dyn CharDevice as *mut &'static dyn CharDevice;
    let prev = cell.swap(new, Ordering::AcqRel);
    // SAFETY: every value in the table came from a `&'static &'static dyn
    // CharDevice` (the initialisers above or an earlier call).
    Some(unsafe { &*prev })
}

/// The entry in table slot `dev` (0..5), or `None` past the table.
fn slot_entry(dev: u16) -> Option<&'static dyn CharDevice> {
    let cell = DEVICES.get(dev as usize)?;
    // SAFETY: as in `install_device`.
    Some(unsafe { *cell.load(Ordering::Acquire) })
}

/// The entry a BIOS character call on `handle` reaches, or `None` (the call
/// answers `0`) — EmuTOS' dispatch in `bconstat`/`bconin`/`bconout`/
/// `bcostat` (`bios/bios.c:923-933`, `:967-977`, `:1000-1010`,
/// `:1152-1162`), with `handle` a `WORD`:
///
/// 1. the `Bconmap` branch first: `map_index = handle -
///    BCONMAP_START_HANDLE` in a `WORD`; `>= maptabsize` → `0`; `>= 0` → the
///    port of table entry `map_index` ([`bconmap::entry_device`]: entry 0 is
///    the AUX port, device 1's entry — EmuTOS' `maptab[0]` holds the
///    `bconstat1`.. routines device 1's vectors come from,
///    `bios/serport.c:98,972,996-1003`);
/// 2. else `0 <= handle` → table slot `handle` (now `< 6`);
/// 3. else (negative) → `0`.
///
/// With one entry: 6 → the AUX entry, 7 and up → `None`. A negative `handle`
/// below `-32762` wraps to a large index, past the table like EmuTOS'.
pub fn device(handle: u16) -> Option<&'static dyn CharDevice> {
    let h = handle as i16;
    let map_index = h.wrapping_sub(bconmap::BCONMAP_START_HANDLE);
    if map_index >= bconmap::MAPTABSIZE {
        return None;
    }
    if map_index >= 0 {
        return slot_entry(bconmap::entry_device(map_index));
    }
    if h < 0 {
        return None;
    }
    slot_entry(handle)
}

/// `Bconstat` core keyed by BIOS device number.
pub fn bconstat_handle(dev: u16) -> Long {
    device(dev).map_or(0, |d| d.bconstat())
}

/// `Bconout` core keyed by BIOS device number. Shared with the GEMDOS
/// pass-throughs (`Cconout`/`Cconws` = device 2, `Cauxout` = `Bconout(1)`,
/// EmuTOS `xauxout`, `bdos/console.c:331`; `Cprnout` = `Bconout(0)`) and
/// `Fwrite` on a device handle.
pub fn bconout_handle(dev: u16, byte: u8) -> Long {
    device(dev).map_or(0, |d| d.bconout(byte))
}

/// `Bcostat` core keyed by BIOS handle, **with EmuTOS' slot swap**:
/// `bcostat_init[3] = bcostat4` (IKBD) and `bcostat_init[4] = bcostat3`
/// (MIDI) (`bios/chardev.c:38-42`), unlike `Bconin`/`Bconout`. So
/// `Bcostat(3)` reads the IKBD entry's status and `Bcostat(4)` the MIDI
/// entry's (`docs/api/bios/08-bcostat.md`). The swap touches only the
/// vector-table handles 3/4; a `Bconmap` handle reads its port's own status
/// (`maptab[i].Bcostat`, `bios/bios.c:1157`) — 6 is AUX's.
pub fn bcostat_handle(handle: u16) -> Long {
    let dev = match handle {
        DEV_MIDI => DEV_IKBD,
        DEV_IKBD => DEV_MIDI,
        h => h,
    };
    device(dev).map_or(0, |d| d.bcostat())
}

/// `Bconstat` (0x01) — input-device status of the device's table entry:
/// `-1` when a byte is ready, else `0` (also the boot-guard / absent-device
/// value). Handle 6 is the AUX entry, 7 and up `0` ([`device`], BIOS08).
pub fn bconstat(args: &TrapArgs) -> Long {
    bconstat_handle(args.arg_u16(0))
}

/// Per-process `conterm` sysvar address ($484) — the classic TOS
/// console-attribute byte the loader's `populate_sysvars` writes into every
/// TPA (default 7, EmuTOS `bios/ikbd.c:1100`).
const CONTERM_ADDR: usize = 0x484;
/// `conterm` bit 3: fold the keyboard shift state into `Bconin`'s return
/// LONG (EmuTOS `bios/ikbd.c:279`).
const CONTERM_SHIFT_FOLD: u8 = 0x08;

/// `conterm` bit 1: keyboard auto-repeat enabled (EmuTOS `bios/ikbd.c:475`).
pub const CONTERM_KEY_REPEAT: u8 = 0x02;

/// The `conterm` byte (`$484`) in effect for the keyboard right now: the
/// running process's own copy, or the boot value `7` (EmuTOS `kbd_init`,
/// `bios/ikbd.c:1100`) when no process runs (boot, EL1 kernel code).
///
/// Read through the **kernel alias of the running process's page 0**, like
/// `bios::tick`'s `_hz_200` publish — not through the user mapping — so
/// it is safe from any EL1 context regardless of which `TTBR0` is installed.
/// Used by the key-repeat handler (`kernel::ikbd::poll_repeat`, conterm bit 1
/// = [`CONTERM_KEY_REPEAT`]).
pub fn current_conterm() -> u8 {
    let Some(page0) = crate::bdos::process::current_page0_kva() else {
        return crate::bdos::process::loader::CONTERM_DEFAULT;
    };
    // SAFETY: `page0` is the kernel alias of the running process's page 0
    // (a static kernel page per slot); `$484` lies in the loader-populated
    // sysvar area (`populate_sysvars`). Same reasoning as
    // `bios::tick::publish`.
    let raw = unsafe { page0.add(CONTERM_ADDR).read_volatile() };
    // A program may write `conterm` in super state (MEM12,
    // `docs/api/_super-state.md` S10): the kernel takes only the documented
    // bits — 0 key click, 1 key repeat, 2 bell, 3 `Bconin` shift state
    // (EmuTOS `bios/ikbd.c:279,475,495,1100`) — and ignores the rest.
    raw & CONTERM_DOCUMENTED
}

/// The `conterm` bits the kernel reads (S10): 0..3.
const CONTERM_DOCUMENTED: u8 = 0x0F;

/// The console `Bconin`/`Cconin` return LONG of an IKBD ring entry
/// (`shifty << 24 | scancode << 16 | ascii`, XBIOS01 E2): bits 0-7 ASCII (0
/// for non-ASCII keys), bits 16-23 the IKBD scancode, bits 24-31 the shift
/// state **at the time the key was pushed** — kept only when the calling
/// process's `conterm` has bit 3 set, else cleared.
///
/// EmuTOS `bconin2` (`references/emutos/bios/ikbd.c:269-283`): the IOREC entry
/// carries `kb_last.k.shifty` from the push (`bios/ikbd.c:898-909`), and
/// `if (!(conterm & 8)) value &= 0x00ffffffL` (`:279-280`). Before XBIOS01
/// NewTOS folded the *current* `Kbshift` state in at read time — BIOS11: a
/// Shift released before the read was lost. With no process running (boot,
/// EmuCON, an EL1 selftest) [`current_conterm`] is the boot value 7 (bit 3
/// clear). Every caller is a CON-input path (`Bconin(2)`,
/// `Cconin`/`Cnecin`/`Crawcin`/`Crawio` — all `bconin2` under EmuTOS). See
/// `docs/api/bios/02-bconin.md`, `docs/api/gemdos/console/01-cconin.md`.
pub fn pack_console_key(entry: u32) -> Long {
    let v = if current_conterm() & CONTERM_SHIFT_FOLD != 0 {
        entry
    } else {
        entry & 0x00FF_FFFF
    };
    v as Long
}

/// `Bconin` core keyed by BIOS device number, as the trap entry: the entry's
/// [`CharDevice::bconin_restartable`]. For CON (2) it blocks and returns the
/// packed console LONG (IKBD scancode in bits 16-23, ASCII in bits 0-7) — or,
/// when a `kbdvec` hook has bytes waiting, requests a restart (KEYS02) and
/// returns `0`, which the trap handler discards; the qemu-virt AUX driver's
/// wait gives way the same way (XBIOS18), and so does the MIDI wait
/// ([`MidiPort`], BIOS04). An absent device returns `0` immediately.
///
/// Shared with the GEMDOS pass-throughs (`Cauxin` = `Bconin(1)`, EmuTOS
/// `xauxin`, `bdos/console.c:432`; `Cconin`/`Cnecin`/`Crawcin` = `Bconin(2)`)
/// and the one-byte `Fread` on a device (`conin`).
pub fn bconin_handle(dev: u16) -> Long {
    match device(dev) {
        Some(d) => match d.bconin_restartable() {
            Some(v) => v,
            None => {
                // A `kbdvec` hook has bytes waiting: restart after it ran
                // (KEYS02); the result is discarded by the trap handler.
                crate::xbios::kbdvecs::request_restart();
                0
            }
        },
        None => 0,
    }
}

/// `Bconin` (0x02) — blocking read from an input device. For the CON
/// handle (2) the packed LONG carries the IKBD scancode in bits 16-23 and
/// the ASCII value in bits 0-7 (non-ASCII keys → ASCII 0 + scancode), per
/// `docs/api/bios/02-bconin.md`. MIDI (3) waits for a byte in the `Iorec(2)`
/// ring ([`MidiPort`]); IKBD (4) and absent devices return `0` without
/// blocking. Handle 6 reads the AUX entry as handle 1 does (on qemu-virt it
/// waits), 7 and up return `0` ([`device`], BIOS08).
pub fn bconin(args: &TrapArgs) -> Long {
    bconin_handle(args.arg_u16(0))
}

/// `Bconout` (0x03) — write a byte to an output device through its table
/// entry. Console handles write to the screen and return `1` (byte accepted,
/// `bconout2`/`bconout5`); the no-cable AUX, MIDI and IKBD discard and
/// return `1` ([`AuxNoCable`], [`MidiPort`], [`IkbdPort`]); absent devices
/// discard and return `0` (`charout_dummy`). Handle 6 is the AUX entry
/// (`1`), 7 and up return `0` ([`device`], BIOS08).
pub fn bconout(args: &TrapArgs) -> Long {
    bconout_handle(args.arg_u16(0), args.arg_u16(1) as u8)
}

/// `Bcostat` (0x08) — output-device status through the table, with the
/// IKBD/MIDI slot swap ([`bcostat_handle`]). Ready (`-1`) for CON
/// (`bcostat2`), the no-cable AUX ([`AuxNoCable`]) and both swapped slots 3
/// and 4 ([`IkbdPort`], [`MidiPort`]); raw-CON's slot is `char_dummy`
/// (`bcostat_init[5]`), so it reports busy (`0`) although `Bconout(5)`
/// writes — deliberately **not** the same handle set as `Bconout`. Absent
/// devices and the boot guard: `0`. Handle 6 is the AUX entry (`-1`), 7 and
/// up `0` ([`device`], BIOS08).
pub fn bcostat(args: &TrapArgs) -> Long {
    bcostat_handle(args.arg_u16(0))
}
