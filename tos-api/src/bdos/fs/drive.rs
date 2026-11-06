//! GEMDOS drive calls: `Dgetdrv` / `Dsetdrv` / `Dfree`. Mirrors EmuTOS
//! `bdos/fsmain.c` (`xgetdrv`/`xsetdrv`) and `bdos/fsfat.c` (`xgetfree`).
//!
//! Spec: `docs/api/gemdos/drive/{0e-dsetdrv,19-dgetdrv}.md`,
//! `docs/api/gemdos/directory/36-dfree.md`.
//!
//! The current drive is `run->p_curdrv`, the byte at PD+0x37 of `run`'s PD
//! (TASK/PROC04 point 2, [`super::curdir`]): the running process's, or with
//! kernel code on the stack the stand-in PD the kernel's `run` names. It is
//! also the implicit drive used by `Fopen`/`Fsfirst` when a path carries no
//! `X:` prefix.

use super::curdir;
use super::fat::Volume;
use crate::bios::drive as bios_drive;
use crate::trap::TrapArgs;
use tos_types::error::{EINTRN, ERROR, E_OK};
use tos_types::Long;

const BLKDEVNUM: u16 = 16;

/// The current drive index (0 = A:) — `run->p_curdrv`. Used by the path
/// parsers when a path has no explicit `X:` prefix.
pub fn current_drive() -> u16 {
    curdir::current_drive()
}

/// `dos_sdrv(bootdev)` (`aes/geminit.c:796-803`) — put the current drive
/// back to the boot device, which is what a resolution change does on TOS 3
/// and EmuTOS ("resolution change always resets the default drive to the boot
/// device"). Called from the kernel's AES session loop on a restart (A106),
/// where `run` is the AES PD (accdesk has ended), so it is the AES PD's
/// `p_curdrv` that changes and the next accdesk inherits it
/// (`_run-ownership-curdir.md` §4.4).
///
/// NewTOS has no boot-device BIOS variable; the boot drive is the first hard
/// disk in the BIOS drive map, which is the derivation the process loader
/// already writes into `bootdev` (`$446`) for every TPA
/// (`bdos/process/loader.rs`). With no drives mounted at all the current
/// drive is left alone — there is nothing to switch to.
pub fn set_boot_drive() {
    let drvbits = bios_drive::drvbits();
    if drvbits != 0 {
        curdir::set_current_drive(drvbits.trailing_zeros() as u16);
    }
}

/// The boot drive, EmuTOS `bootdev` (`bios/bios.c:798-799`): the first
/// drive of the BIOS drive map (NewTOS boots from the first hard disk, `C:`),
/// or the current drive when no drive is mounted at all. The one derivation
/// behind the `bootdev` system variable (`$446`) every process sees
/// (`bdos/process/loader.rs` `populate_sysvars`) and the drive letter of the
/// default environment `"PATH=\0X:\\\0"` (`bios/bios.c:626-636`,
/// [`crate::bdos::process::standin`]).
pub fn boot_drive() -> u16 {
    let drvbits = bios_drive::drvbits();
    if drvbits != 0 {
        drvbits.trailing_zeros() as u16
    } else {
        current_drive()
    }
}

/// `Dgetdrv` (0x19) — return the current drive (0 = A:): `run->p_curdrv`
/// (`bdos/fsmain.c:438-441`).
pub fn dgetdrv(_args: &TrapArgs) -> Long {
    current_drive() as Long
}

/// `Dsetdrv` (0x0E) — set the current drive (0-based) in `run`'s PD
/// (`run->p_curdrv = drv`, `bdos/fsmain.c:420-426`), return the BIOS drive
/// bitmap. Out-of-range indices are silently ignored (no existence check),
/// per the historical Atari behaviour.
pub fn dsetdrv(args: &TrapArgs) -> Long {
    let drv = args.arg_u16(0);
    if drv < BLKDEVNUM {
        curdir::set_current_drive(drv);
    }
    bios_drive::drvbits() as Long
}

/// `Dfree` (0x36) — fill a 4-LONG buffer with free/total clusters,
/// sector size and cluster size. `drv`: 0 = current, 1 = A:, 2 = B:, …
/// (note the `+1` offset, unlike `Dsetdrv`). Returns `ERROR` (-1) if the
/// drive cannot be mounted, and the device's code with nothing stored if a FAT
/// sector cannot be read (`xgetfree` stores only after the count,
/// `bdos/fsfat.c:361-364`).
pub fn dfree(args: &TrapArgs) -> Long {
    let buf = args.arg_u64(0);
    let drv = args.arg_u16(1);
    let index = if drv == 0 {
        current_drive()
    } else {
        drv - 1
    };

    let vol = match Volume::mount(index) {
        Some(v) => v,
        None => return ERROR,
    };
    let Some(free) = vol.count_free_clusters() else {
        return super::cache::take_error().unwrap_or(EINTRN);
    };

    // 4 LONGs: free clusters, total clusters, bytes/sector, sectors/cluster.
    let out = [free, vol.numcl, vol.recsiz, vol.clsiz];
    // `buf` is the caller's 16-byte buffer, in the caller's space (ARCH03
    // 3a, `crate::uaccess`; a kernel buffer for an EL1 caller).
    let mut bytes = [0u8; 16];
    for (i, v) in out.iter().enumerate() {
        bytes[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
    }
    if args.space.copy_to(buf, &bytes).is_err() {
        return crate::uaccess::FAULTED;
    }
    E_OK
}
