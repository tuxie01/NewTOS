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
///
/// Then the boot scan of every drive found ([`bpb::scan`], EmuTOS
/// `pun_info_setup` after `disk_init_all`, `bios/blkdev.c:190-195`): each
/// drive's BPB slot is built, which is the `recsiz` a logical `Rwabs` tests
/// (BIOS-STOR-006), and the run-time sector-size limit of `Getbpb` is set
/// (BIOS-STOR-004).
pub fn mount() -> usize {
    crate::bdos::fs::fat::log_out_all();
    bcache::invalidate_all();
    let n = drive::mount();
    bpb::scan();
    n
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
/// Bit 3 — physical (untranslated) I/O (`disk.h:59`). When set, `drive`
/// is a **physical unit** ([`drive::SD_UNIT`]) and the sector argument an
/// **absolute device sector**: no partition-relative translation, and the
/// range check is against the whole device rather than the partition
/// (`blkdev.c:438,483-490`).
const RW_NOTRANSLATE: i16 = 8;
/// `NUMFLOPPIES` (`bios/disk.h:18`): drives / units 0 and 1 are the floppies.
const NUMFLOPPIES: u16 = 2;

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
/// `x5`), `x4` = drive (physical unit with `RW_NOTRANSLATE`), `x5` = lfirst.
///
/// EmuTOS `blkdev_rwabs` (`bios/blkdev.c:406-540`), in the order of
/// `docs/api/bios/04-rwabs.md` (order table): the floppy NULL-buffer feature
/// first (step 1, BIOS-STOR-007), then the split on `RW_NOTRANSLATE`
/// (`blkdev.c:438`) — the **logical** path translates a partition-relative
/// sector of a drive ([`logical_lba`]), the **physical** path takes an
/// absolute sector of a physical unit ([`physical_lba`], BIOS-STOR-002).
/// A count of 0 passes every check of either path and then transfers
/// nothing (the driver is called with a zero count, `bios/sd.c:163-164`).
///
/// Deviations from EmuTOS that are out of scope here (HW-adapted or
/// deferred, unchanged): the retry loop / `etv_critic` chain (H1/M1), a
/// media-change *probe* behind step 10 (the SD is non-removable, so
/// [`media_change`] reports only `forcechange`), the `RW_NORETRIES` retry
/// count (single PIO attempt), `RW_NOBYTESWAP` (AArch64 is little-endian),
/// and the logical-to-physical scaling of step 7 (`blkdev.c:456-458`) — see
/// [`logical_lba`]. See `docs/api/bios/04-rwabs.md` and
/// `docs/analysis/bios-audit/bios-findings.md`.
fn rwabs(args: &TrapArgs) -> Long {
    let r_w = args.arg_i16(0);
    let adr = args.arg_u64(1);
    let numb = args.arg_u16(2) as usize;
    let first = args.arg_i16(3);
    let drive = args.arg_u16(4);
    let lfirst = args.arg_i32(5);

    // Step 1 (`blkdev.c:418-428`): the floppy-only handler's undocumented
    // feature (FastCopy III) — a NULL buffer on unit 0 or 1 sets that drive's
    // media-change state to `numb` and returns 0, ahead of every drive test
    // and in either mode (BIOS-STOR-007, decided: match EmuTOS). A: and B: do
    // not exist on the Pi (the drive table starts at C:, `drive.rs`), so
    // `Mediach`, `Getbpb` and `Rwabs` of them stay `EUNDEV`/0/`EUNDEV` and the
    // state set here has no reader — as on an EmuTOS without floppy drives
    // (audit §1, `EMUTOS-falcon-nofloppy`). A negative `drive` is not
    // included: that writes outside EmuTOS' `blkdev[]` (spec edge case 11).
    if drive < NUMFLOPPIES && adr == 0 {
        return E_OK;
    }
    // Step 2 (`blkdev.c:430-431`): `first` is an unsigned 16-bit sector
    // unless it is -1, which selects the 32-bit `lfirst`.
    let sector = if first == -1 {
        lfirst as u32 as u64
    } else {
        first as u16 as u64
    };
    // Through the block cache (FS09): logical and physical requests are both
    // keyed by the absolute LBA, so the two modes share one coherent copy.
    // No device means no drive and no unit: `EUNDEV` in either mode.
    let blk = match bcache::device() {
        Some(b) => b,
        None => return EUNDEV,
    };
    let write = r_w & RW_RW != 0;
    let target = if r_w & RW_NOTRANSLATE == 0 {
        logical_lba(r_w, drive, sector, numb)
    } else {
        physical_lba(drive, sector, numb, blk.block_count())
    };
    let lba = match target {
        Ok(lba) => lba,
        Err(code) => return code,
    };
    if numb == 0 {
        // The transfer loop runs once with a zero count (`blkdev.c:510-528`),
        // which the SD driver answers 0 without touching the card or the
        // buffer (`bios/sd.c:163-164`).
        return E_OK;
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

/// The logical path of `blkdev_rwabs` (`bios/blkdev.c:439-481`), steps 3-10
/// of the order table in `docs/api/bios/04-rwabs.md`: the absolute LBA of
/// `drive`'s partition-relative `sector`, or the code that ends the call.
///
/// * Step 3 (`:440-441`): an absent drive is `EUNDEV`.
/// * Step 4 (`:443-444`): a drive whose BPB is invalid — the last `Getbpb`
///   refused its boot sector ([`bpb::recsiz`] 0) — is `ESECNF`, before
///   anything else (BIOS-STOR-006).
/// * Steps 5, 6 (`:450-453`, BIOS-STOR-001): an armed `forcechange` fails
///   every logical request with `E_CHNG`, `RW_NOMEDIACH` included; a write
///   starting at logical sector 0 arms it, before the range and media-change
///   checks of this call.
/// * Step 7 (`:456-458`) is not done: NewTOS addresses the partition in
///   512-byte sectors whatever the BPB's `recsiz`. Since the boot scan sets
///   the `Getbpb` limit (BIOS-STOR-004) a larger `recsiz` is logged in only
///   on a card that had such a partition at boot.
/// * Step 8 (`:461-463`): the range check against the partition, with `>`,
///   for every count including 0 (BIOS-STOR-003).
/// * Step 10 (`:473-477`): unless `RW_NOMEDIACH`, a media change fails the
///   call with `E_CHNG` — this is how the arming write itself writes nothing.
fn logical_lba(r_w: i16, drive: u16, sector: u64, numb: usize) -> Result<u64, Long> {
    let part = drive::resolve(drive).ok_or(EUNDEV)?;
    if bpb::recsiz(drive) == 0 {
        return Err(ESECNF);
    }
    if drive::forcechange(drive) {
        return Err(E_CHNG);
    }
    if r_w & RW_RW != 0 && sector == 0 {
        drive::arm_forcechange(drive);
    }
    if sector + numb as u64 > part.count as u64 {
        return Err(ESECNF);
    }
    if r_w & RW_NOMEDIACH == 0 && media_change(drive) != MEDIANOCHANGE {
        return Err(E_CHNG);
    }
    Ok(part.start_lba as u64 + sector)
}

/// The physical path of `blkdev_rwabs` (`bios/blkdev.c:482-504`): `unit` is
/// a physical unit number, `sector` an absolute sector of it.
///
/// * `:484-485`: the SD card is unit [`drive::SD_UNIT`] (26); every other
///   unit is `EUNDEV` (BIOS-STOR-002, decided 2026-10-28).
/// * `:488-490`: the range check against the whole device, with `>=`, skipped
///   when the device reports no size (`total == 0`).
/// * `:496-503`: `UNIT_CHANGED` is never set — the SD is non-removable
///   (`docs/api/bios/09-mediach.md`) — so there is no `E_CHNG` here.
///
/// The physical path never arms `forcechange` (the arming is in the logical
/// branch, `blkdev.c:438-453`).
fn physical_lba(unit: u16, sector: u64, numb: usize, total: u64) -> Result<u64, Long> {
    if unit != drive::SD_UNIT {
        return Err(EUNDEV);
    }
    if total != 0 && sector + numb as u64 >= total {
        return Err(ESECNF);
    }
    Ok(sector)
}

/// `Mediach` (0x09) — media-change probe. `drv` is in `x0`
/// (`docs/api/bios/09-mediach.md` §"Trap interface"). The trap forwards to
/// [`media_change`], as EmuTOS' `mediach` forwards to `hdv_mediach`
/// (`bios/bios.c:1184`).
fn mediach(args: &TrapArgs) -> Long {
    media_change(args.arg_u16(0))
}

/// `MEDIANOCHANGE` — the media has not changed (`bios/disk.h:69`).
pub const MEDIANOCHANGE: Long = 0;
/// `MEDIAMAYCHANGE` — the media may have changed (`bios/disk.h:70`). Never
/// answered on the Pi (no probe, `docs/api/bios/09-mediach.md`); named for
/// the BDOS, which follows the contract (`bdos::fs::cache`).
pub const MEDIAMAYCHANGE: Long = 1;
/// `MEDIACHANGE` — the media has changed (`bios/disk.h:71`).
pub const MEDIACHANGE: Long = 2;

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
///
/// Also the `Mediach` the BDOS asks before it uses a cached record (EmuTOS
/// `getbcb`, `bdos/fsbuf.c:179`) and the media test of the BDOS's own
/// device accesses (`bdos::fs::cache`, `docs/api/gemdos/_state.md` §4a).
pub fn media_change(drive: u16) -> Long {
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
