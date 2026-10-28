//! GEMDOS (Trap #1) dispatcher and function table.
//!
//! Spec: `docs/api/gemdos/_dispatcher.md`,
//! `docs/api/gemdos/console/*.md`.
//!
//! NewTOS encodes the function ID in `x8` (AAPCS64 reserved slot) and
//! takes up to seven argument registers (`x0..x6`); see
//! [`crate::trap::TrapArgs`]. The return value is a sign-extended
//! `Long` (`i32` in `x0`).
//!
//! The full GEMDOS console group is wired below. The three
//! input-with-Ctrl-C functions (Cconin 0x01, Cnecin 0x08, Cconrs 0x0A)
//! defer their Ctrl-C `terminate()` to the kernel SVC handler via
//! [`crate::bdos::process::request_terminate`] (a leaf function cannot
//! redirect the trap frame itself); the other console leaves dispatch
//! directly. Before the table, [`console::redirect_to_file`] answers a
//! character function whose standard handle is forced to a file (EmuTOS
//! `osif`, `bdos/bdosmain.c:498-568`).

use crate::host::host;
use crate::trap::TrapArgs;
use tos_types::error::{EINVFN, EUNDEV, E_CHNG};
use tos_types::Long;

pub mod console;
pub mod memory;
pub mod ssystem;
pub mod time;
pub mod trace;
pub mod version;

/// Highest GEMDOS function ID dispatched by NewTOS. Matches EmuTOS
/// `MAX_FNCALL` (= `bdos/bdosmain.c`).
pub const MAX_FN: usize = 0x57;

/// Per-function entry. `fp` is the implementation; a `None` slot
/// means "unimplemented" and yields `EINVFN`.
type GemdosFn = fn(&TrapArgs) -> Long;

/// Function table indexed by function ID. Built at compile time; only
/// non-`None` slots dispatch.
const TABLE: [Option<GemdosFn>; MAX_FN + 1] = build_table();

const fn build_table() -> [Option<GemdosFn>; MAX_FN + 1] {
    let mut t: [Option<GemdosFn>; MAX_FN + 1] = [None; MAX_FN + 1];
    // --- Console group (PHASE-02 D3; input side unblocked PHASE-06) -
    t[0x01] = Some(console::cconin);
    t[0x02] = Some(console::cconout);
    t[0x03] = Some(console::cauxin);
    t[0x04] = Some(console::cauxout);
    t[0x05] = Some(console::cprnout);
    t[0x06] = Some(console::crawio);
    t[0x07] = Some(console::crawcin);
    t[0x08] = Some(console::cnecin);
    t[0x09] = Some(console::cconws);
    t[0x0A] = Some(console::cconrs);
    t[0x0B] = Some(console::cconis);
    t[0x10] = Some(console::cconos);
    t[0x11] = Some(console::cprnos);
    t[0x12] = Some(console::cauxis);
    t[0x13] = Some(console::cauxos);
    // Ctrl-C in the blocking input functions (Cconin/Cnecin/Cconrs)
    // records a deferred terminate consumed by the kernel SVC handler.

    // --- Memory group (PHASE-04 D7..D11) --------------------------
    t[0x14] = Some(memory::maddalt);
    t[0x44] = Some(memory::mxalloc);
    t[0x48] = Some(memory::malloc);
    t[0x49] = Some(memory::mfree);
    t[0x4A] = Some(memory::mshrink);

    // --- Time / version (PHASE-07 D2 + D5) ------------------------
    t[0x2A] = Some(time::tgetdate);
    t[0x2B] = Some(time::tsetdate);
    t[0x2C] = Some(time::tgettime);
    t[0x2D] = Some(time::tsettime);
    t[0x30] = Some(version::sversion);

    // --- File group, read side (PHASE-09 D4) ----------------------
    t[0x3D] = Some(crate::bdos::fs::file::fopen);
    t[0x3E] = Some(crate::bdos::fs::file::fclose);
    t[0x3F] = Some(crate::bdos::fs::file::fread);
    t[0x42] = Some(crate::bdos::fs::file::fseek);
    t[0x45] = Some(crate::bdos::fs::file::fdup);
    t[0x46] = Some(crate::bdos::fs::file::fforce);

    // --- File group, write side (PHASE-09 D7) ---------------------
    t[0x3C] = Some(crate::bdos::fs::file::fcreate);
    t[0x40] = Some(crate::bdos::fs::file::fwrite);
    t[0x41] = Some(crate::bdos::fs::file::fdelete);
    t[0x43] = Some(crate::bdos::fs::file::fattrib);
    t[0x56] = Some(crate::bdos::fs::file::frename);
    t[0x57] = Some(crate::bdos::fs::file::fdatime);

    // --- DTA + directory search (PHASE-09 D5) ---------------------
    t[0x1A] = Some(crate::bdos::fs::find::fsetdta);
    t[0x2F] = Some(crate::bdos::fs::find::fgetdta);
    t[0x4E] = Some(crate::bdos::fs::find::fsfirst);
    t[0x4F] = Some(crate::bdos::fs::find::fsnext);

    // --- Drive group (PHASE-09 D6) --------------------------------
    t[0x0E] = Some(crate::bdos::fs::drive::dsetdrv);
    t[0x19] = Some(crate::bdos::fs::drive::dgetdrv);
    t[0x36] = Some(crate::bdos::fs::drive::dfree);

    // --- Directory create/remove + current path (sub-directories) -
    t[0x39] = Some(crate::bdos::fs::dirops::dcreate);
    t[0x3A] = Some(crate::bdos::fs::dirops::ddelete);
    t[0x3B] = Some(crate::bdos::fs::dirops::dsetpath);
    t[0x47] = Some(crate::bdos::fs::dirops::dgetpath);
    t
}

/// Dispatch a GEMDOS trap. Called from the kernel's synchronous-EL0
/// exception handler with the captured register state.
///
/// Returns the value to load into `x0` on `eret`.
pub fn dispatch(args: &TrapArgs) -> Long {
    // GEMDOS trap tracer: this is the funnel `docs/TOS-REFERENCE-DUMPER.md`
    // §10 wants logged — every file-system request an application makes passes
    // through it exactly once. The hooks wrap the *whole* body, early returns
    // included: "the application asked for a function this kernel does not
    // have" (`EINVFN` above `MAX_FN`) is one of the most informative lines a
    // dispatcher log can carry, and a hook placed after that check would drop
    // exactly those. Both hooks have empty bodies (and `t0` folds away)
    // without the `gemdos-trace` feature, leaving `dispatch_inner` to inline
    // straight back into this function. The tracer's own file writes call
    // `bdos::fs::file::*` directly and therefore never re-enter here; see
    // `trace`'s module docs for the full re-entrancy argument.
    let t0 = trace::call_begin();
    let ret = dispatch_inner(args);
    trace::call_end(args, t0, ret);
    ret
}

/// The dispatch itself; [`dispatch`] only wraps it in the tracer.
///
/// EmuTOS `osif` (`bdos/bdosmain.c:431-492`): each run of the call starts
/// with a fresh hard-error context (`restrt:` + `setjmp(errbuf)`,
/// `:442-449`); when the run met a media change, the drive is logged in
/// again ([`media_change_relog`]) and the call is run once more from the
/// start with the same arguments (`goto restrt`, `:486`), its first return
/// value dropped (`docs/api/gemdos/_state.md` §4a, TASK/FS20).
///
/// The loop ends: a successful re-login has read the boot sector through
/// `Getbpb`, which disarms the drive's `forcechange`
/// (`crate::bios::bpb::geometry`), and its buffers are gone; a GEMDOS call
/// issues no `Rwabs` that could arm it again.
#[inline]
fn dispatch_inner(args: &TrapArgs) -> Long {
    if host().is_none() {
        // Misconfiguration: panic-shaped but encoded as a TOS error so
        // the trap path remains reentrant.
        return EUNDEV;
    }
    loop {
        // A fresh hard-error context for this run — EmuTOS re-arms
        // `setjmp(errbuf)` at the top of `osif` (`bdos/bdosmain.c:442-449`,
        // `docs/api/gemdos/_dispatcher.md` §8). Without it a device error
        // nobody consumed could leak into the next call's result (HW03).
        crate::bdos::fs::cache::clear_error();
        let rc = run_call(args);
        match crate::bdos::fs::cache::take_media_change() {
            None => return rc,
            Some(drv) => {
                if let Err(code) = media_change_relog(drv) {
                    return code;
                }
            }
        }
    }
}

/// One run of the call: the MiNT extensions, the redirected character
/// functions, then the table (`osif` after `restrt:`).
fn run_call(args: &TrapArgs) -> Long {
    let id = args.func_id() as usize;
    // MiNT extensions live above MAX_FN and are special-cased before the
    // static table lookup (FreeMiNT compatibility, ADR-012).
    if id == ssystem::SSYSTEM {
        return ssystem::ssystem(args);
    }
    if id > MAX_FN {
        return EINVFN;
    }
    // A character function whose standard handle is forced to a file is
    // answered from that file before the table (`osif`,
    // `bdos/bdosmain.c:498-568`, `_dispatcher.md` §5; TASK/PROC04 point 1).
    if let Some(rc) = console::redirect_to_file(id, args) {
        return rc;
    }
    match TABLE[id] {
        Some(f) => f(args),
        None => EINVFN,
    }
}

/// `osif`'s media-change block (`bdos/bdosmain.c:456-487`) for drive `drv`,
/// in its order: the drive's open files are freed (`offree`), it is logged
/// out (the DMD, `drvtbl[errdrv] = NULL`), its current directories are
/// dropped (`freetree`), its buffers discarded (`mark_bcbs_invalid`), then it
/// is logged in again with a fresh `Getbpb` + `log_media` — the **new** boot
/// sector's geometry.
///
/// `Err(E_CHNG)` when `Getbpb` refuses the drive (`:471-476`): the drive
/// stays logged out (`drvsel` bit clear) and the next call logs it in through
/// `ckdrv`. EmuTOS' `ENSMEM` for a failing `log_media` (`:478-482`) has no
/// NewTOS cause: `bpb::parse` already refuses the one boot sector
/// `log_media` would (`fsiz == 0`).
fn media_change_relog(drv: u16) -> Result<(), Long> {
    use crate::bdos::fs::{cache, curdir, fat, file};
    file::free_drive_files(drv);
    fat::log_out(drv);
    curdir::free_drive(drv);
    cache::invalidate_drive(drv);
    match fat::Volume::mount(drv) {
        Some(_) => Ok(()),
        None => Err(E_CHNG),
    }
}
