//! XBIOS — Trap 14 (`svc #14`).
//!
//! Spec: `docs/api/xbios/_dispatcher.md`. Native AArch64 ABI (as for
//! GEMDOS / BIOS): function id in `x8`, arguments in `x0..`, result in
//! `x0`. The dispatch returns the full 64-bit `x0` value — most XBIOS
//! calls return a `WORD`/`LONG`, but the screen-base calls
//! (`Physbase`/`Logbase`) return a native pointer, so the result is
//! widened to `i64`.
//!
//! Phase 10 implements the screen group; every unimplemented function
//! (in-range padded slot **or** out-of-range id) returns the **function
//! number**, never [`EINVFN`] — the documented Atari-TOS behaviour
//! (`docs/api/xbios/_dispatcher.md §3`, D-008), identical to the BIOS
//! trampoline. A handful of functions guard absent Atari peripherals with a
//! defined contract value instead (floppy → `EUNDEV`, PSG → `0`, etc.).

pub mod floppy;
pub mod iorec;
pub mod kbdvecs;
pub mod keytbl;
pub mod mfp;
pub mod screen;
pub mod system;

use crate::trap::TrapArgs;
use tos_types::error::EUNDEV;

// XBIOS function ids. Note: Initmous (0x00) and Bioskeys (0x18) are
// intentionally NOT wired — they need NewTOS keyboard/mouse infrastructure
// that does not exist yet, so they fall through to the function-number
// catch-all (honestly unimplemented). See the STOP notes on `dispatch` below.
// Iorec (0x0E) is wired since XBIOS01 (`iorec`).
const PHYSBASE: u16 = 0x02; // screen group (Phase 10)
const LOGBASE: u16 = 0x03;
const GETREZ: u16 = 0x04;
const SETSCREEN: u16 = 0x05;
const SETPALETTE: u16 = 0x06;
const SETCOLOR: u16 = 0x07;
const FLOPRD: u16 = 0x08; // floppy/DMA group → EUNDEV (no controller)
const FLOPWR: u16 = 0x09;
const FLOPFMT: u16 = 0x0A;
const MIDIWS: u16 = 0x0C; // MIDI entry of the BIOS device table (absent: bytes dropped)
const IOREC: u16 = 0x0E; // the IOREC page: records + the live IKBD key ring (XBIOS01)
const RSCONF: u16 = 0x0F; // RS-232 config (PL011 thin adapter)
const KEYTBL: u16 = 0x10; // keyboard translation tables
const RANDOM: u16 = 0x11; // system group (Phase 11)
const PROTOBT: u16 = 0x12; // pure-software boot-sector builder
const FLOPVER: u16 = 0x13;
const SCRDMP: u16 = 0x14; // no printer on the Pi → documented no-op
const CURSCONF: u16 = 0x15;
const SETTIME: u16 = 0x16; // clock group — shares the GEMDOS wall clock
const GETTIME: u16 = 0x17;
const IKBDWS: u16 = 0x19; // IKBD command write — documented no-op (void)
const GIACCESS: u16 = 0x1C; // PSG group → 0 (no YM2149)
const OFFGIBIT: u16 = 0x1D;
const ONGIBIT: u16 = 0x1E;
const DOSOUND: u16 = 0x20;
const SETPRT: u16 = 0x21; // desktop printer config word (pure software)
const KBDVBASE: u16 = 0x22;
const KBRATE: u16 = 0x23; // keyboard auto-repeat timing
const VSYNC: u16 = 0x25;
const FLOPRATE: u16 = 0x29;
const DMAREAD: u16 = 0x2A;
const DMAWRITE: u16 = 0x2B;
const BCONMAP: u16 = 0x2C; // single serial port → returns 0x2c ("absent")
const BLITMODE: u16 = 0x40;

/// XBIOS Trap-14 dispatcher. Returns the value to load into `x0`.
pub fn dispatch(args: &TrapArgs) -> i64 {
    note_installer(args);
    match args.func_id() {
        PHYSBASE => screen::physbase(),
        LOGBASE => screen::logbase(),
        GETREZ => screen::getrez(),
        SETSCREEN => screen::setscreen(args),
        SETPALETTE => screen::setpalette(args),
        SETCOLOR => screen::setcolor(args),
        CURSCONF => screen::cursconf(args),
        // Floppy / DMA transfer group: no WD1772 floppy controller and no
        // Atari ACSI/DMA bus on the Pi, so the canonical "device not present"
        // error `EUNDEV` (-15) is returned — programs detect a missing floppy
        // by this value; the generic function-number would break that idiom.
        // `Protobt` (0x12) is pure software and does real work.
        // Spec: `docs/api/xbios/_floppy-stubs.md`.
        FLOPRD | FLOPWR | FLOPFMT | FLOPVER | FLOPRATE | DMAREAD | DMAWRITE => EUNDEV as i64,
        PROTOBT => floppy::protobt(args),
        // Absent Atari peripherals → the contract's documented value for
        // "hardware not present", not the generic function-number catch-all.
        // Scrdmp: no printer / default dump_vec is a no-op (14-scrdmp.md);
        // Midiws: through the MIDI device-table entry, absent → bytes
        // dropped (0c-midiws.md);
        // Bconmap: single serial port → 0x2c "unavailable" (2c-bconmap.md).
        SCRDMP => system::scrdmp(),
        MIDIWS => system::midiws(args),
        BCONMAP => system::bconmap(),
        // RS-232 config: thin adapter over the PL011 (no MFP). See
        // `docs/api/xbios/0f-rsconf.md`.
        RSCONF => system::rsconf(args),
        // Keyboard translation tables. The returned `struct keytbl` and the
        // six tables it points at are materialised into the caller's own
        // address space, so an EL0 program can dereference the result exactly
        // as EmuTOS' desktop does (`desk/deskmain.c:1144-1145`). See
        // `docs/api/xbios/10-keytbl.md`.
        KEYTBL => keytbl::keytbl(args),
        // Device records: 0 RS-232, 1 IKBD (the live console key queue), 2
        // MIDI, otherwise -1 (`bios/xbios.c:336-348`). The records sit in a
        // page every process maps read-write, so `io->ibufhd = io->ibuftl`
        // flushes the keyboard as on TOS. See `docs/api/xbios/0e-iorec.md`.
        IOREC => iorec::iorec(args),
        RANDOM => system::random(),
        // Clock group: Settime/Gettime read/write the same wall clock as
        // GEMDOS Tgetdate/Tgettime (no separate RTC on NewTOS). Settime
        // returns void → 0; Gettime returns the packed LONG (zero-extended).
        SETTIME => {
            crate::gemdos::time::xbios_settime(args.arg_i32(0) as u32);
            0
        }
        GETTIME => crate::gemdos::time::xbios_gettime() as i64,
        // IKBD command write: no ACIA transport → documented no-op (void).
        // See `docs/api/xbios/19-ikbdws.md`.
        IKBDWS => system::ikbdws(),
        // YM2149 PSG group: no sound chip → plausible-zero / no-op values
        // (not the function-number pattern). See `docs/api/xbios/_psg-group.md`.
        GIACCESS => system::giaccess(),
        OFFGIBIT => system::offgibit(),
        ONGIBIT => system::ongibit(),
        DOSOUND => system::dosound(),
        // Desktop printer config word: pure software (parport.c:49-57),
        // independent of printer hardware. See `docs/api/xbios/_psg-group.md`
        // (Setprt entry).
        SETPRT => system::setprt(args),
        KBDVBASE => kbdvecs::kbdvbase(),
        // MFP vector installers: warned (ADR-031), EmuTOS' d0 (`mfp`, BIOS15).
        MFPINT => mfp::mfpint(args),
        XBTIMER => mfp::xbtimer(args),
        // Keyboard auto-repeat timing get/set. See `docs/api/xbios/23-kbrate.md`.
        KBRATE => system::kbrate(args),
        VSYNC => system::vsync(),
        // No Atari BLiTTER on the Pi → contract status word 0x0000
        // (not the function number). See `docs/api/xbios/40-blitmode.md`.
        BLITMODE => system::blitmode(),
        // Every other id — an in-range but unimplemented slot (e.g. Initmous
        // 0x00, Bioskeys 0x18, the MFP 0x1A/0x1B
        // (Jdisint/Jenabint) and NVRAM 0x2E slots, Supexec 0x26, the TT-shifter
        // 0x50-0x57 range) **or** an out-of-range id (> LAST_ENTRY 0x57) —
        // returns the function number itself, zero-extended. This is the
        // documented Atari-TOS behaviour for both the `xbios_unimpl` padded
        // slot and the trampoline's range check (`docs/api/xbios/_dispatcher.md
        // §3`, D-008), identical to the BIOS trampoline (`bios/mod.rs`). A
        // caller testing `result < 0` sees "not an error".
        other => other as i64,
    }
}

// Hook installers (ADR-031): `Initmous` is answered by the function-number
// arm of `dispatch` but reported first (`note_installer`); `Mfpint` and
// `Xbtimer` have their own arms (`mfp`, TASK/BIOS15).
const INITMOUS: u16 = 0x00;
const MFPINT: u16 = 0x0D;
const XBTIMER: u16 = 0x1F;

/// `Initmous` installs a hook NewTOS never calls (ADR-031 rule 5, "in the
/// call itself"): warn once per process. The call itself stays what it was —
/// the dispatcher's function-number arm. `Initmous(type, param, vec)` installs
/// `vec` as `kbdvecs.mousevec` for a valid `type` (1, 2, 4) and a non-null
/// `vec` (`references/emutos/bios/mouse.c:105-117`); type 0 and errors
/// install `just_rts` (`:118-120`), no program hook. (`Mfpint`, `Xbtimer`:
/// [`mfp`].)
fn note_installer(args: &TrapArgs) {
    use crate::bdos::process::hooks::{note_running, Hook, Via};
    if args.func_id() == INITMOUS {
        let vec = args.arg_u64(2);
        if matches!(args.arg_i16(0), 1 | 2 | 4) && vec != 0 {
            note_running(Hook::Mousevec, vec, Via::Initmous);
        }
    }
}
