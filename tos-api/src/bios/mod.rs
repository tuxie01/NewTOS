//! TOS BIOS — Trap 13 (`svc #13`).
//!
//! Spec: `docs/api/bios/`. Bring-up scope: the block-device functions
//! (`Rwabs`, `Mediach`, `Drvmap`) on top of the host-supplied
//! [`block::BlockDev`], plus the console character calls
//! (`Bconstat`/`Bconin`/`Bconout`/`Bcostat`, see [`chardev`]). The cookie
//! jar lands later.
//!
//! Native AArch64 ABI (as for GEMDOS): function id in `x8`, arguments in
//! `x0..`, result in `x0`.

pub mod bcache;
pub mod block;
pub mod bpb;
pub mod chardev;
pub mod crash;
pub mod drive;
pub mod getmpb;
pub mod reset;
pub mod tick;
pub mod vectors;

/// Scan the block device's partition table and (re)build the BIOS drive map
/// — [`drive::mount`], plus the BDOS-side drive log-out that has to go with
/// it.
///
/// The BDOS caches each drive's FAT geometry once it has logged the drive in
/// (EmuTOS `ckdrv`/`drvsel`, `bdos/fsdrive.c:94-105`). A fresh partition scan
/// can move, resize or replace every partition, which is the same situation
/// EmuTOS handles by clearing `drvsel` and freeing the drive's `DMD`
/// (`bdos/bdosmain.c:456-482`) — so every drive is logged out here and will
/// re-read its boot sector on the next access.
///
/// The block cache goes with it as well (FS09, `bcache`): a new scan is a new
/// mapping of the card, so every cached sector is dropped before the MBR is
/// read again — FreeMiNT invalidates its cache when a drive's mapping changes
/// (`bio_set_lshift`, `bio_free_di`; `docs/analysis/fs09-block-cache-2026-10-07.md`
/// §2.7).
pub fn mount() -> usize {
    crate::bdos::fs::fat::log_out_all();
    bcache::invalidate_all();
    drive::mount()
}

use tos_types::error::{ESECNF, EUNDEV, E_CHNG, E_OK};
use tos_types::Long;

use crate::host;
use crate::trap::TrapArgs;

const SECTOR: usize = 512;

// `Rwabs` read/write flag bits (EmuTOS `bios/disk.h:52-61`).
/// Bit 0 — write (vs. read); also `RW_WRITE` (`disk.h:54,56`).
const RW_RW: i16 = 1;
/// Bit 1 — skip the media-change test of the logical path (`disk.h:57`,
/// `blkdev.c:473`). It does **not** skip the `forcechange` test
/// (`blkdev.c:450-451`, ahead of it).
const RW_NOMEDIACH: i16 = 2;
/// Bit 3 — physical (untranslated) I/O (`disk.h:59`). When set, the
/// sector argument is an **absolute device sector**: no partition-relative
/// translation, and the range check is against the whole device rather than
/// the partition (`blkdev.c:438,483-490`).
const RW_NOTRANSLATE: i16 = 8;

// BIOS function ids.
const GETMPB: u16 = 0x00;
const BCONSTAT: u16 = 0x01;
const BCONIN: u16 = 0x02;
const BCONOUT: u16 = 0x03;
const RWABS: u16 = 0x04;
const SETEXC: u16 = 0x05;
const TICKCAL: u16 = 0x06;
const GETBPB: u16 = 0x07;
const BCOSTAT: u16 = 0x08;
const MEDIACH: u16 = 0x09;
const DRVMAP: u16 = 0x0A;
const KBSHIFT: u16 = 0x0B;

/// System-timer period in milliseconds (50 Hz → 20 ms), per EmuTOS.
const TICKCAL_MS: Long = 20;

/// BIOS Trap-13 dispatcher. Returns the full 64-bit register value:
/// most functions return a `Long`, but `Getbpb` returns a native
/// `BPB *` pointer, so the result is widened to `i64`.
pub fn dispatch(args: &TrapArgs) -> i64 {
    match args.func_id() {
        GETMPB => getmpb::getmpb(args) as i64,
        BCONSTAT => chardev::bconstat(args) as i64,
        BCONIN => chardev::bconin(args) as i64,
        BCONOUT => chardev::bconout(args) as i64,
        BCOSTAT => chardev::bcostat(args) as i64,
        RWABS => rwabs(args) as i64,
        SETEXC => vectors::setexc(args),
        TICKCAL => TICKCAL_MS as i64,
        GETBPB => bpb::getbpb(args.arg_u16(0)),
        MEDIACH => mediach(args) as i64,
        DRVMAP => drvmap() as i64,
        KBSHIFT => kbshift(args) as i64,
        // Out-of-range function id (> 0x0B): EmuTOS' trampoline returns
        // the function number itself, not an error code — the `cmp.w d1,d0`
        // / `bx_ret_exc` path leaves the number in D0 (`bios/vectors.S:1323`,
        // `docs/api/bios/_dispatcher.md §3-4`). Every id 0x00..=0x0B has a
        // real handler above, so this arm is only reached for out-of-range
        // ids. `func_id()` is a `u16`, so the value is zero-extended.
        other => other as i64,
    }
}

/// `Kbshift` (0x0B) — get or set the keyboard modifier-state byte.
///
/// `flag < 0` (the `-1` query idiom) returns the current shift-state
/// bitmask without changing it; `flag >= 0` overwrites the modifier state
/// with `flag` **truncated to a byte** (EmuTOS `bios/ikbd.c:145` stores
/// `(UBYTE)flag` into `shifty`). Always returns the **previous** value,
/// zero-extended into the LONG (bits 8-31 clear), per
/// `docs/api/bios/0b-kbshift.md`. The state lives in the IKBD decoder and
/// is reached through the host trait (there is no `os_kbshift` low-memory
/// alias on the Pi — spec "Pi / BCM2711 binding").
fn kbshift(args: &TrapArgs) -> Long {
    let flag = args.arg_i16(0);
    let h = match host::host() {
        Some(h) => h,
        None => return 0,
    };
    let old = h.keyboard_shift_state() as Long;
    if flag >= 0 {
        h.set_keyboard_shift_state(flag as u8);
    }
    old
}

/// `Rwabs` (0x04) — read/write absolute sectors.
///
/// `x0` = r_w (bit 0: 0 read / 1 write; bit 3: `RW_NOTRANSLATE`, physical
/// I/O), `x1` = buffer, `x2` = count, `x3` = first sector (or `-1` ⇒ use
/// `x5`), `x4` = drive, `x5` = lfirst.
///
/// EmuTOS `blkdev.c:438` splits on `RW_NOTRANSLATE` (bit 3): the common
/// **logical** path translates a partition-relative sector to an absolute
/// LBA and range-checks against the partition (`blkdev.c:440-466`); the
/// **physical** path treats the sector as an absolute device sector, with
/// no partition translation and a range check against the whole device
/// (`blkdev.c:483-490`). The device is validated up front in both modes.
///
/// The logical path's `forcechange` handshake (BIOS-STOR-001) follows the
/// order of `blkdev_rwabs` (`docs/api/bios/04-rwabs.md`, order table): an
/// armed drive fails every logical request with `E_CHNG`, `RW_NOMEDIACH`
/// included (step 5); a write starting at logical sector 0 arms it (step 6)
/// and then fails the media-change test itself unless `RW_NOMEDIACH` is set
/// (step 10) — so it writes nothing and returns `E_CHNG`. The physical path
/// never arms (the arming is in the logical branch, `blkdev.c:438-453`).
///
/// Deviations from EmuTOS that are out of scope for L1 (HW-adapted or
/// deferred, unchanged here): the retry loop / `etv_critic` chain
/// (H1/M1), a media-change *probe* behind step 10 (the SD is non-removable,
/// so [`media_change`] reports only `forcechange`), the `RW_NORETRIES` retry
/// count (single PIO attempt), and `RW_NOBYTESWAP` (AArch64 is
/// little-endian). See `docs/api/bios/04-rwabs.md` and
/// `docs/analysis/bios-audit/bios-findings.md`.
fn rwabs(args: &TrapArgs) -> Long {
    let r_w = args.arg_i16(0);
    let adr = args.arg_u64(1);
    let numb = args.arg_u16(2) as usize;
    let first = args.arg_i16(3);
    let drive = args.arg_u16(4);
    let lfirst = args.arg_i32(5);

    // Resolve the drive to its partition (start LBA + length). EmuTOS
    // validates the device up front in both logical (`blkdev.c:440-441`)
    // and physical (`blkdev.c:484-485`) mode; on the single-block-device Pi
    // the drive table stands in for `units[]`.
    let part = match drive::resolve(drive) {
        Some(p) => p,
        None => return EUNDEV,
    };
    // Through the block cache (FS09): logical and physical requests are both
    // keyed by the absolute LBA, so the two modes share one coherent copy.
    let blk = match bcache::device() {
        Some(b) => b,
        None => return EUNDEV,
    };
    let sector = if first == -1 {
        lfirst as u32 as u64
    } else {
        first as u16 as u64
    };
    let write = r_w & RW_RW != 0;
    let logical = r_w & RW_NOTRANSLATE == 0;
    if logical {
        // Step 5 (`blkdev.c:450-451`): a drive whose logical sector 0 was
        // written fails every logical request until `Getbpb` — ahead of the
        // `RW_NOMEDIACH` test, so that flag does not get past it.
        if drive::forcechange(drive) {
            return E_CHNG;
        }
        // Step 6 (`blkdev.c:452-453`): a write starting at logical sector 0
        // arms it, before the range and media-change checks of this call.
        if write && sector == 0 {
            drive::arm_forcechange(drive);
        }
    }
    // Step 10 (`blkdev.c:473-477`), evaluated here because the count-0
    // return below needs it too; it is *reported* after the range check.
    let media_changed =
        logical && r_w & RW_NOMEDIACH == 0 && media_change(drive) != MEDIANOCHANGE;
    if numb == 0 {
        // BIOS-STOR-003 (open, not this change): EmuTOS range-checks a count
        // of 0 as well (`blkdev.c:461-463`); here it skips the range check.
        return if media_changed { E_CHNG } else { E_OK };
    }
    let lba = if !logical {
        // Physical: the sector is already an absolute device sector — no
        // partition translation. Range-check against the device's total
        // sector count (EmuTOS `blkdev.c:488-490`, which uses `>=` and skips
        // the check when the unit size is unknown/zero).
        let total = blk.block_count();
        if total != 0 && sector + numb as u64 >= total {
            return ESECNF;
        }
        sector
    } else {
        // Logical: partition-relative sector → absolute LBA, range-checked
        // against the partition (EmuTOS `blkdev.c:461-466`, which uses `>`).
        if sector + numb as u64 > part.count as u64 {
            return ESECNF;
        }
        part.start_lba as u64 + sector
    };
    if media_changed {
        // Step 10 reported: the media-change test failed (`blkdev.c:474-477`)
        // — a write that just armed `forcechange` transfers nothing.
        return E_CHNG;
    }
    let len = numb * SECTOR;

    // `adr` is the caller's buffer for `len` bytes, in the caller's space
    // (ARCH03 3a): a program's buffer is checked with its rights — readable
    // for a write to the device, writable for a read from it — and then
    // reached through its kernel alias (`Space::kptr`, plan §3.6), so the
    // block driver's PIO loop never touches a user address. A buffer the
    // program may not use ends the call (and, through the syscall tail, the
    // program — EmuTOS: bus error in `Rwabs`). An in-kernel caller's buffer
    // is a kernel address.
    let Ok(buf) = args.space.kptr(adr, len as u64, !write) else {
        return crate::uaccess::FAULTED;
    };
    // SAFETY: `buf..buf+len` was just checked accessible (or is a kernel
    // caller's own buffer) and is reached through the kernel mapping.
    let result = unsafe {
        if write {
            blk.write_blocks(lba, core::slice::from_raw_parts(buf as *const u8, len))
        } else {
            blk.read_blocks(lba, core::slice::from_raw_parts_mut(buf as *mut u8, len))
        }
    };
    // The driver's own code, passed through — EmuTOS `blkdev_rwabs` returns
    // `retval` from `disk_rw`/`floppy_rw` unchanged (`bios/blkdev.c:512-523`),
    // so a caller sees `EWRITF`/`EREADF` for a failed transfer but `EDRVNR`
    // for a card that did not answer. Before HW03 the device could only say
    // "false" and this function invented the code.
    match result {
        Ok(()) => E_OK,
        Err(code) => code,
    }
}

/// `Mediach` (0x09) — media-change probe. `drv` is in `x0`
/// (`docs/api/bios/09-mediach.md` §"Trap interface"). The trap forwards to
/// [`media_change`], as EmuTOS' `mediach` forwards to `hdv_mediach`
/// (`bios/bios.c:1184`).
fn mediach(args: &TrapArgs) -> Long {
    media_change(args.arg_u16(0))
}

/// `MEDIANOCHANGE` — the media has not changed (`bios/disk.h:69`).
const MEDIANOCHANGE: Long = 0;
/// `MEDIACHANGE` — the media has changed (`bios/disk.h:71`).
const MEDIACHANGE: Long = 2;

/// EmuTOS `blkdev_mediach` (`bios/blkdev.c:783-820`): the media-change state
/// of a logical drive, shared by the `Mediach` trap and `Rwabs`' media-change
/// test (step 10 of `docs/api/bios/04-rwabs.md`).
///
/// An invalid / unmounted drive returns `EUNDEV` (`blkdev.c:789-790`). An
/// armed `forcechange` returns `MEDIACHANGE` (2) until `Getbpb` clears it
/// (`blkdev.c:802-803`), without storing a latch (`09-mediach.md`, edge case
/// 3). Otherwise EmuTOS asks the driver (`flop_mediach`/`disk_mediach`); the
/// Pi's SD card is treated as non-removable — no probe, no latched
/// `mediachange` — so the answer is `MEDIANOCHANGE` (spec §"Pi / BCM2711
/// binding", `docs/HARDWARE-PROFILE.md` §3.4).
fn media_change(drive: u16) -> Long {
    if drive::resolve(drive).is_none() {
        return EUNDEV; // out-of-range / unmounted drive
    }
    if drive::forcechange(drive) {
        return MEDIACHANGE;
    }
    MEDIANOCHANGE
}

/// `Drvmap` (0x0A) — bitmap of present drives (bit `n` = drive `n`),
/// built from the partition table.
fn drvmap() -> Long {
    drive::drvbits() as Long
}
