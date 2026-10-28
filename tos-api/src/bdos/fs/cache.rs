//! Sector cache (Phase 09 D2, FS08). Mirrors EmuTOS `bdos/fsbuf.c`.
//!
//! A small write-back cache of 512-byte sectors sitting between the FAT /
//! directory / file layers and the block device, in **two separate lists**
//! like EmuTOS' `bufl[BI_FAT]` / `bufl[BI_DATA]` (`bdos/fsbuf.c:67-71`,
//! `docs/api/gemdos/_state.md` §3): FAT sectors in one, directory and file
//! data sectors in the other. An access searches only its own list
//! (`getbcb`, `bdos/fsbuf.c:121`), so streaming file data can never push the
//! FAT sectors out — before FS08 one shared 16-slot round-robin cache did
//! exactly that, and every `Fread` re-read the FAT from the card
//! (`docs/analysis/fs08-sd-read-2026-10-07.md` §2.3).
//!
//! Each list is kept in most-recently-used order; a miss takes the last
//! invalid slot, else the least recently used one (`bdos/fsbuf.c:130-201`).
//! A FAT slot is cached once and written to **every** FAT copy at
//! write-back (`flush`, `bdos/fsbuf.c:95-102`). Whole-record file transfers
//! bypass the cache ([`direct_read`] / [`direct_write`], EmuTOS `usrio`,
//! `bdos/fsio.c:57-74`).
//!
//! Single block device, single-CPU, IRQ-off trap path — no locking.
//!
//! # Reporting a hard device error (HW03)
//!
//! EmuTOS reports a failed `Rwabs` out of the BDOS with a `longjmp`:
//! `longjmp_rwabs` (`bdos/fs.h:60`) stores the BIOS code in the globals
//! `rwerr`/`errcode`/`errdrv` (`bdos/fsglob.c:40-47`) and jumps to the
//! `setjmp(errbuf)` at the top of `osif`, which "handle[s] as hard error on
//! disk" and returns the code "to caller as if it were the function's
//! return value" (`bdos/bdosmain.c:448-492`, `docs/api/gemdos/_dispatcher.md`
//! §8).
//!
//! There is no `longjmp` here, so the same contract is built from two
//! pieces:
//!
//! * [`flush`] returns the code — `Result` is `#[must_use]`, so a call site
//!   that drops it no longer compiles silently. All nine call sites used to
//!   discard a `bool` (audit finding 5).
//! * the `bool`-typed interior (`read`, `write`, and everything in
//!   `fat.rs`/`dirops.rs` above them) records the code in [`LAST_ERROR`] —
//!   this module's `rwerr` — and the GEMDOS entry point returns it with
//!   [`take_error`]. That is what turns a `Fwrite` short count caused by a
//!   *device error* back into `EWRITF`, instead of letting it pass for
//!   "disk full" (`docs/api/gemdos/file/40-fwrite.md` §"Error conditions").

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicI32, AtomicU32, Ordering};

use tos_types::error::{EUNDEV, E_CHNG, E_OK};
use tos_types::Long;

use crate::bios::block::BlockDev;

/// Buffers per list. EmuTOS has `NUMBUFS = 2` (`bdos/fsbuf.c:26`), sized for
/// ST RAM; the count is not part of the contract (`_state.md` §3), so NewTOS
/// keeps the 16 buffers it had before FS08, split evenly between the lists.
const SLOTS: usize = 8;
const SECTOR: usize = 512;

/// Which buffer list an access goes to — EmuTOS `BI_FAT` / `BI_DATA`, with
/// what a FAT buffer needs to know for its write-back.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// A FAT sector, addressed by its LBA in the **last** FAT copy (EmuTOS
    /// `m_recoff[BT_FAT] = fatrec`, `bdos/fsdrive.c:241`). Write-back also
    /// writes the same data `stride`, `2 * stride`, … sectors earlier, into
    /// the other `copies - 1` FATs (`bdos/fsbuf.c:97-102`).
    Fat {
        /// Sectors between two FAT copies (`fsiz`).
        stride: u32,
        /// Number of FAT copies (1 or 2).
        copies: u8,
    },
    /// A root-directory, sub-directory or file-data sector
    /// (`BT_ROOT` / `BT_DATA`, both in `bufl[BI_DATA]`, `bdos/fsbuf.c:121`).
    Data,
}

impl Kind {
    fn list(self) -> usize {
        match self {
            Kind::Fat { .. } => 0,
            Kind::Data => 1,
        }
    }
}

/// How often the cache re-attempts the write-back of one and the same
/// dirty slot before it stops trying on its own.
///
/// A retry is legitimate — the Pi measurement of 2026-09-25 had twelve of
/// twenty-three sector writes fail and land on a later attempt
/// (`docs/api/hw/emmc-audit.md` §3.2). An *unbounded* retry is not: before
/// HW03 a slot whose write-back failed kept its dirty flag and was tried
/// again by every later `flush()`, for ever, without anyone learning of it.
///
/// The number is a policy choice, not a specification value, and is named
/// here as one. EmuTOS bounds the same thing in two places and lower:
/// `RWABS_RETRIES = 1` at the BIOS layer ("on real machine might want to
/// increase this", `bios/blkdev.h:34`) and "retry at most once" in its SD
/// driver (`bios/sd.c:167-169`). Three gives a transient card busy two more
/// chances than EmuTOS' one and still terminates.
const WRITEBACK_ATTEMPTS: u8 = 3;

struct Slot {
    lba: u64,
    /// The drive the sector belongs to — EmuTOS `b_bufdrv` (`bdos/fs.h:
    /// 290-299`). Read while `valid`: the media test of a hit asks this
    /// drive (`bdos/fsbuf.c:179`), a write-back is that drive's `Rwabs`, and
    /// a re-login drops the drive's slots ([`invalidate_drive`]).
    drv: u16,
    valid: bool,
    dirty: bool,
    /// The list's kind the slot was filled for; for a FAT slot this carries
    /// the mirror geometry its write-back needs.
    kind: Kind,
    /// Failed write-back attempts for the data currently in this slot.
    /// Reset to 0 by a successful write-back and by a fresh [`write`] of
    /// the same sector (new data is a new request).
    wb_fails: u8,
    /// TOS error code of this slot's last failed write-back, so a later
    /// access can report *that* failure rather than a guess.
    wb_err: Long,
    data: [u8; SECTOR],
}

/// One buffer list: the slots plus their order, most recently used first
/// (`b_link` order in EmuTOS).
struct List {
    slots: [Slot; SLOTS],
    /// Slot indices, head = most recently used, tail = least.
    order: [u8; SLOTS],
}

const EMPTY_SLOT: Slot = Slot {
    lba: 0,
    drv: 0,
    valid: false,
    dirty: false,
    kind: Kind::Data,
    wb_fails: 0,
    wb_err: E_OK,
    data: [0u8; SECTOR],
};

const fn initial_order() -> [u8; SLOTS] {
    let mut o = [0u8; SLOTS];
    let mut i = 0;
    while i < SLOTS {
        o[i] = i as u8;
        i += 1;
    }
    o
}

struct Cache {
    lists: UnsafeCell<[List; 2]>,
}
// SAFETY: accessed only from the single-CPU, IRQ-off trap path.
unsafe impl Sync for Cache {}

static CACHE: Cache = Cache {
    lists: UnsafeCell::new(
        [const { List { slots: [EMPTY_SLOT; SLOTS], order: initial_order() } }; 2],
    ),
};

static HITS: AtomicU32 = AtomicU32::new(0);
static MISSES: AtomicU32 = AtomicU32::new(0);
/// Misses per list, `[FAT, data]` — the device reads the cache issued.
static LIST_MISSES: [AtomicU32; 2] = [AtomicU32::new(0), AtomicU32::new(0)];
/// Whole-record transfers that bypassed the cache: `(requests, sectors)`.
static DIRECT_REQS: AtomicU32 = AtomicU32::new(0);
static DIRECT_SECTORS: AtomicU32 = AtomicU32::new(0);

/// The hard error of the most recent failed block access — EmuTOS `rwerr` /
/// `errcode` (`bdos/fsglob.c:40-41`). `E_OK` when there is none pending.
static LAST_ERROR: AtomicI32 = AtomicI32::new(E_OK);

/// The drive of the first media change met in the running GEMDOS call —
/// EmuTOS `errdrv` when `errcode == E_CHNG` (`bdos/fsbuf.c:185-187`,
/// `bdos/fs.h:64-68`); [`NO_DRIVE`] when there is none. Separate from
/// [`LAST_ERROR`], which the calls' own error paths take ([`take_error`]):
/// the dispatcher needs it after the call returned ([`take_media_change`]).
static MEDIA_CHANGE_DRV: AtomicI32 = AtomicI32::new(NO_DRIVE);
const NO_DRIVE: i32 = -1;

// ── Write-back bookkeeping (HW03 point 4: a retry must be visible) ────
/// Write-backs that failed, counting every attempt.
static WB_FAILURES: AtomicU32 = AtomicU32::new(0);
/// Write-backs that were a **repeat** attempt at a slot that had already
/// failed at least once — the retries that used to be silent.
static WB_RETRIES: AtomicU32 = AtomicU32::new(0);
/// Slots that used up [`WRITEBACK_ATTEMPTS`] and are no longer retried
/// automatically. Their data is still in the cache and still dirty.
static WB_EXHAUSTED: AtomicU32 = AtomicU32::new(0);

/// The card, through the block cache (FS09) that sits below these lists.
fn block() -> Option<&'static dyn BlockDev> {
    crate::bios::bcache::device()
}

fn lists() -> &'static mut [List; 2] {
    // SAFETY: single trap path; never concurrently borrowed.
    unsafe { &mut *CACHE.lists.get() }
}

/// Record a hard device error for the running GEMDOS call — EmuTOS
/// `errcode = rwerr` in `longjmp_rwabs` (`bdos/fs.h:66`).
fn set_error(code: Long) {
    LAST_ERROR.store(code, Ordering::Relaxed);
}

/// Forget any pending hard error and media change. Called at the top of a
/// GEMDOS call, the place EmuTOS establishes a fresh `setjmp(errbuf)`
/// context (`bdos/bdosmain.c:442-449`).
pub fn clear_error() {
    LAST_ERROR.store(E_OK, Ordering::Relaxed);
    MEDIA_CHANGE_DRV.store(NO_DRIVE, Ordering::Relaxed);
}

/// A media change on drive `drv` was met: the access fails with `E_CHNG`,
/// and the drive is kept for the dispatcher's re-login — EmuTOS `errdrv =
/// drv; rwerr = errcode = E_CHNG; longjmp(errbuf, 1)` (`bdos/fsbuf.c:
/// 185-188`, `bdos/fs.h:64-68`). EmuTOS jumps at the first one; here the
/// call runs on through its error paths, so the **first** drive of the call
/// is the one kept.
fn signal_media_change(drv: u16) -> Long {
    set_error(E_CHNG);
    let _ = MEDIA_CHANGE_DRV.compare_exchange(
        NO_DRIVE,
        i32::from(drv),
        Ordering::Relaxed,
        Ordering::Relaxed,
    );
    E_CHNG
}

/// The drive of the media change the running GEMDOS call met, if any, and
/// clear it — what `osif`'s `setjmp` branch finds in `errdrv` when
/// `errcode == E_CHNG` (`bdos/bdosmain.c:456-459`).
pub fn take_media_change() -> Option<u16> {
    let d = MEDIA_CHANGE_DRV.swap(NO_DRIVE, Ordering::Relaxed);
    (d != NO_DRIVE).then_some(d as u16)
}

/// What `getbcb` does with a buffer hit (`bdos/fsbuf.c:177-191`): it asks
/// `Mediach(b_bufdrv)` before the record is used. `Ok(true)` = use the
/// buffer (`0`, or a negative driver answer); `Ok(false)` = the media may
/// have changed (`1`), read the record again as on a miss (`goto doio`);
/// `Err(E_CHNG)` = the media has changed (`2`).
///
/// NewTOS's `Mediach` answers `0`, `2` or `EUNDEV` (the SD card has no
/// "may have changed", `docs/api/bios/09-mediach.md`); the `1` branch is
/// kept so the BDOS follows the BIOS contract, not today's BIOS.
fn hit_test(drv: u16) -> Result<bool, Long> {
    match crate::bios::media_change(drv) {
        crate::bios::MEDIACHANGE => Err(signal_media_change(drv)),
        crate::bios::MEDIAMAYCHANGE => Ok(false),
        _ => Ok(true),
    }
}

/// The media tests of the logical `Rwabs` every BDOS device access is
/// (`longjmp_rwabs`, `bdos/fs.h:60-77`, flag 0 or 1, never
/// `RW_NOMEDIACH`): a pending `forcechange` (step 5 of
/// `docs/api/bios/04-rwabs.md`) or a media-change answer (step 10) fails it
/// with `E_CHNG`. NewTOS's BDOS reaches the card through the block cache,
/// not through `Rwabs`, so it asks here before every device access: the read
/// of a miss, a write-back, a whole-record transfer.
fn rwabs_test(drv: u16) -> Result<(), Long> {
    if crate::bios::drive::forcechange(drv)
        || crate::bios::media_change(drv) > crate::bios::MEDIANOCHANGE
    {
        return Err(signal_media_change(drv));
    }
    Ok(())
}

/// EmuTOS `mark_bcbs_invalid` (`bdos/bdosmain.c:411-424`): every slot of
/// drive `drv`, in both lists, is made invalid. Dirty slots are
/// **discarded, not written back** — the media they belonged to is gone
/// (`docs/api/gemdos/_state.md` §3, §4a). Called by the re-login after a
/// media change.
pub fn invalidate_drive(drv: u16) {
    for list in lists().iter_mut() {
        for s in list.slots.iter_mut() {
            if s.valid && s.drv == drv {
                s.valid = false;
                s.dirty = false;
                s.wb_fails = 0;
                s.wb_err = E_OK;
            }
        }
    }
}

/// Is a hard device error pending (without taking it)? Lets a caller tell
/// "no free cluster" from "the FAT could not be read".
pub fn error_pending() -> bool {
    LAST_ERROR.load(Ordering::Relaxed) != E_OK
}

/// Take the pending hard device error, if any, and clear it. The GEMDOS
/// entry points return it as their own result — `osif`'s "return the error
/// to caller as if it were the function's return value"
/// (`docs/api/gemdos/_dispatcher.md` §8).
pub fn take_error() -> Option<Long> {
    let e = LAST_ERROR.swap(E_OK, Ordering::Relaxed);
    if e == E_OK {
        None
    } else {
        Some(e)
    }
}

/// Write one dirty slot back to the device — EmuTOS `flush(b)`
/// (`bdos/fsbuf.c:85-105`): the sector itself and, for a FAT slot, the same
/// data into every other FAT copy.
///
/// On success the slot is clean. **On failure the slot keeps its data, its
/// `valid` flag and its dirty flag** — losing it would throw away the only
/// up-to-date copy of the sector (audit finding 4: before HW03 the
/// eviction path cleared `dirty` unconditionally and then overwrote the
/// slot, so a FAT sector could disappear with no trace anywhere).
///
/// This is a deliberate, named deviation from EmuTOS: its `flush()` sets
/// `b_bufdrv = -1` *before* the write ("invalidate in case of error",
/// `bdos/fsbuf.c:93`) and its `osif` error handler then drops every buffer
/// of the drive (`mark_bcbs_invalid`, `bdos/bdosmain.c:490`). EmuTOS does
/// that because a `longjmp` cannot carry the buffer back — the buffer is
/// unreachable from the point the jump lands. We do not jump, so we are not
/// forced to discard, and the ticket's integrity requirement is exactly
/// that no sector disappears because its write-back failed.
///
/// The write is the logical `Rwabs` of `flush` (`bdos/fsbuf.c:95,100`), so
/// it meets that call's media tests first ([`rwabs_test`]): with a media
/// change pending on the slot's drive it fails with `E_CHNG` without
/// touching the device (and without counting as a failed write-back); the
/// slot stays as it is until the re-login drops it ([`invalidate_drive`]).
fn write_back(s: &mut Slot) -> Result<(), Long> {
    rwabs_test(s.drv)?;
    if s.wb_fails > 0 {
        WB_RETRIES.fetch_add(1, Ordering::Relaxed);
    }
    let r = match block() {
        Some(b) => {
            let mut r = b.write_blocks(s.lba, &s.data);
            if let (Ok(()), Kind::Fat { stride, copies }) = (r, s.kind) {
                // "flush to both fats" (`bdos/fsbuf.c:97-102`): the slot is
                // keyed by the last copy; the others lie `stride` earlier each.
                for k in 1..u64::from(copies) {
                    r = b.write_blocks(s.lba - k * u64::from(stride), &s.data);
                    if r.is_err() {
                        break;
                    }
                }
            }
            r
        }
        // No block device behind a mounted volume: the same code `Rwabs`
        // gives an unknown device (`bios/mod.rs`, EmuTOS `blkdev.c:440-441`).
        None => Err(EUNDEV),
    };
    match r {
        Ok(()) => {
            s.dirty = false;
            s.wb_fails = 0;
            s.wb_err = E_OK;
            Ok(())
        }
        Err(code) => {
            WB_FAILURES.fetch_add(1, Ordering::Relaxed);
            s.wb_fails = s.wb_fails.saturating_add(1);
            s.wb_err = code;
            if s.wb_fails == WRITEBACK_ATTEMPTS {
                WB_EXHAUSTED.fetch_add(1, Ordering::Relaxed);
            }
            set_error(code);
            Err(code)
        }
    }
}

/// May this slot's write-back still be attempted automatically?
///
/// `false` once the slot has used up [`WRITEBACK_ATTEMPTS`]. The sector
/// stays in the cache and stays dirty; what stops is the driver traffic —
/// every later `flush()` reports the error straight away instead of
/// re-issuing a write that has already failed three times.
fn retryable(s: &Slot) -> bool {
    s.wb_fails < WRITEBACK_ATTEMPTS
}

impl List {
    /// Move the slot at order position `pos` to the head (most recently
    /// used) — `*q = b->b_link; b->b_link = *phdr; *phdr = b`
    /// (`bdos/fsbuf.c:197-199`).
    fn promote(&mut self, pos: usize) {
        let idx = self.order[pos];
        self.order.copy_within(0..pos, 1);
        self.order[0] = idx;
    }

    /// Order position of the valid slot holding `lba`, if any.
    fn find(&self, lba: u64) -> Option<usize> {
        (0..SLOTS).find(|&p| {
            let s = &self.slots[self.order[p] as usize];
            s.valid && s.lba == lba
        })
    }

    /// Pick the victim for a miss — `getbcb`'s choice (`bdos/fsbuf.c:
    /// 130-157`): the **last** invalid slot in list order, else the tail
    /// (least recently used), made ready by [`List::evict`]. Returns its
    /// order position.
    fn victim(&mut self) -> Result<usize, Long> {
        let pos = (0..SLOTS)
            .rev()
            .find(|&p| !self.slots[self.order[p] as usize].valid)
            .unwrap_or(SLOTS - 1);
        self.evict(pos)
    }

    /// Make the slot at order position `pos` ready to take another record: a
    /// dirty slot is written back first (`bdos/fsbuf.c:162-163`).
    ///
    /// When the slot is dirty and cannot be written back, it is **not**
    /// handed out — the caller's access fails rather than silently
    /// overwriting a sector that never reached the card (HW03). The slot is
    /// moved to the head, so the next miss picks a different victim and one
    /// bad sector does not block the whole list.
    fn evict(&mut self, pos: usize) -> Result<usize, Long> {
        let s = &mut self.slots[self.order[pos] as usize];
        if s.valid && s.dirty {
            let r = if retryable(s) {
                write_back(s)
            } else {
                // Out of attempts: do not touch the device again, but do not
                // hand the slot out either — its sector is still only here.
                set_error(s.wb_err);
                Err(s.wb_err)
            };
            if let Err(code) = r {
                self.promote(pos);
                return Err(code);
            }
        }
        Ok(pos)
    }

    /// EmuTOS `getbcb` (`bdos/fsbuf.c:114-202`): the slot holding `lba` of
    /// drive `drv`, read in on a miss unless `fill` is `false` (the caller
    /// overwrites all 512 bytes), moved to the head. Returns the slot index.
    ///
    /// A hit is used only after the media test of `getbcb` ([`hit_test`],
    /// `bdos/fsbuf.c:177-191`); "may have changed" reads the record again
    /// into the same slot (`goto doio`). A miss — and that re-read — is the
    /// logical `Rwabs` of `longjmp_rwabs` (`bdos/fsbuf.c:165`) and meets its
    /// media tests ([`rwabs_test`]) whether or not the old contents are read:
    /// EmuTOS always reads.
    fn get(&mut self, kind: Kind, drv: u16, lba: u64, fill: bool) -> Result<usize, Long> {
        let pos = match self.find(lba) {
            Some(pos) => {
                let idx = self.order[pos] as usize;
                if hit_test(self.slots[idx].drv)? {
                    HITS.fetch_add(1, Ordering::Relaxed);
                    self.promote(pos);
                    return Ok(idx);
                }
                MISSES.fetch_add(1, Ordering::Relaxed);
                LIST_MISSES[kind.list()].fetch_add(1, Ordering::Relaxed);
                self.evict(pos)?
            }
            None => {
                MISSES.fetch_add(1, Ordering::Relaxed);
                LIST_MISSES[kind.list()].fetch_add(1, Ordering::Relaxed);
                self.victim()?
            }
        };
        let idx = self.order[pos] as usize;
        let s = &mut self.slots[idx];
        // "in case longjmp_rwabs() fails" (`bdos/fsbuf.c:164`).
        s.valid = false;
        rwabs_test(drv)?;
        if fill {
            let r = match block() {
                Some(b) => b.read_blocks(lba, &mut s.data),
                None => Err(EUNDEV),
            };
            if let Err(code) = r {
                set_error(code);
                return Err(code);
            }
        }
        s.lba = lba;
        s.drv = drv;
        s.kind = kind;
        s.valid = true;
        s.dirty = false;
        s.wb_fails = 0;
        s.wb_err = E_OK;
        self.promote(pos);
        Ok(idx)
    }
}

/// EmuTOS `getrec` (`bdos/fsbuf.c:209-235`): run `f` on the cached contents
/// of sector `lba` of drive `drv` in `kind`'s list, reading it in on a miss;
/// with `write` the slot is marked dirty afterwards (`:231-232`).
///
/// `None` means the access failed; the TOS code is in [`take_error`].
pub fn with_sector<R>(
    kind: Kind,
    drv: u16,
    lba: u64,
    write: bool,
    f: impl FnOnce(&mut [u8; SECTOR]) -> R,
) -> Option<R> {
    let list = &mut lists()[kind.list()];
    let idx = list.get(kind, drv, lba, true).ok()?;
    let s = &mut list.slots[idx];
    let r = f(&mut s.data);
    if write {
        s.dirty = true;
        // New content for this sector: the old content's failed attempts
        // do not count against it.
        s.wb_fails = 0;
        s.wb_err = E_OK;
    }
    Some(r)
}

/// Read sector `lba` of drive `drv` into `out`, through `kind`'s list.
///
/// `false` means the access failed; the TOS code is in [`take_error`].
pub fn read(kind: Kind, drv: u16, lba: u64, out: &mut [u8; SECTOR]) -> bool {
    with_sector(kind, drv, lba, false, |d| out.copy_from_slice(d)).is_some()
}

/// Write sector `lba` of drive `drv` from `data` into `kind`'s list
/// (write-back: the slot is marked dirty and flushed later). A miss does not
/// read the old contents first — all 512 bytes are replaced, so the read
/// would be unobservable; it still meets the media tests of that read.
///
/// `false` means no slot could be made available because another sector's
/// write-back failed, or a media change is pending; the TOS code is in
/// [`take_error`].
pub fn write(kind: Kind, drv: u16, lba: u64, data: &[u8; SECTOR]) -> bool {
    let list = &mut lists()[kind.list()];
    let Ok(idx) = list.get(kind, drv, lba, false) else {
        return false;
    };
    let s = &mut list.slots[idx];
    s.data.copy_from_slice(data);
    s.dirty = true;
    s.wb_fails = 0;
    s.wb_err = E_OK;
    true
}

/// The data-list half of EmuTOS `usrio` (`bdos/fsio.c:61-71`): every cached
/// data sector in `lba .. lba + count` is written back if dirty and then
/// invalidated, so a whole-record transfer that bypasses the cache neither
/// reads stale card contents nor leaves a stale copy behind.
fn usrio_invalidate(lba: u64, count: u64) -> Result<(), Long> {
    let list = &mut lists()[Kind::Data.list()];
    for s in list.slots.iter_mut() {
        if s.valid && s.lba >= lba && s.lba < lba + count {
            if s.dirty {
                if !retryable(s) {
                    set_error(s.wb_err);
                    return Err(s.wb_err);
                }
                write_back(s)?;
            }
            s.valid = false;
        }
    }
    Ok(())
}

/// Read `buf.len() / 512` sectors of drive `drv` from `lba` straight into
/// `buf` in **one** block request, bypassing the cache — EmuTOS `usrio(0, …)`
/// (`bdos/fsio.c:57-74`), whose `longjmp_rwabs` meets the media tests of a
/// logical `Rwabs` ([`rwabs_test`]).
///
/// `false` means the access failed; the TOS code is in [`take_error`].
pub fn direct_read(drv: u16, lba: u64, buf: &mut [u8]) -> bool {
    let count = (buf.len() / SECTOR) as u64;
    if usrio_invalidate(lba, count).is_err() || rwabs_test(drv).is_err() {
        return false;
    }
    DIRECT_REQS.fetch_add(1, Ordering::Relaxed);
    DIRECT_SECTORS.fetch_add(count as u32, Ordering::Relaxed);
    let r = match block() {
        Some(b) => b.read_blocks(lba, buf),
        None => Err(EUNDEV),
    };
    match r {
        Ok(()) => true,
        Err(code) => {
            set_error(code);
            false
        }
    }
}

/// Write `buf.len() / 512` sectors from `buf` to `lba` of drive `drv` in
/// **one** block request, bypassing the cache — EmuTOS `usrio(1, …)`, with
/// the same media tests as [`direct_read`].
///
/// `false` means the access failed; the TOS code is in [`take_error`].
pub fn direct_write(drv: u16, lba: u64, buf: &[u8]) -> bool {
    let count = (buf.len() / SECTOR) as u64;
    if usrio_invalidate(lba, count).is_err() || rwabs_test(drv).is_err() {
        return false;
    }
    DIRECT_REQS.fetch_add(1, Ordering::Relaxed);
    DIRECT_SECTORS.fetch_add(count as u32, Ordering::Relaxed);
    let r = match block() {
        Some(b) => b.write_blocks(lba, buf),
        None => Err(EUNDEV),
    };
    match r {
        Ok(()) => true,
        Err(code) => {
            set_error(code);
            false
        }
    }
}

/// Write back all dirty sectors to the block device — the loop at the end
/// of EmuTOS `ixclose` (`bdos/fsopnclo.c:576-579`), FAT list first.
///
/// Returns the TOS error code of the **first** failure, after having tried
/// every other dirty slot: one bad sector must not keep the rest of the
/// volume's metadata in memory. Sectors that fail stay in the cache, dirty
/// (see [`write_back`]).
pub fn flush() -> Result<(), Long> {
    let mut first_err = None;
    for list in lists().iter_mut() {
        for s in list.slots.iter_mut() {
            if !(s.valid && s.dirty) {
                continue;
            }
            if !retryable(s) {
                // Already given up on automatically; still an error for the
                // caller, because this sector is not on the card.
                let code = s.wb_err;
                set_error(code);
                first_err.get_or_insert(code);
                continue;
            }
            if let Err(code) = write_back(s) {
                first_err.get_or_insert(code);
            }
        }
    }
    match first_err {
        Some(code) => Err(code),
        None => Ok(()),
    }
}

/// Cache statistics: `(hits, misses)` over both lists.
pub fn stats() -> (u32, u32) {
    (HITS.load(Ordering::Relaxed), MISSES.load(Ordering::Relaxed))
}

/// Device reads issued per list on a miss: `(fat, data)`.
pub fn list_misses() -> (u32, u32) {
    (LIST_MISSES[0].load(Ordering::Relaxed), LIST_MISSES[1].load(Ordering::Relaxed))
}

/// Whole-record transfers that bypassed the cache: `(requests, sectors)`.
pub fn direct_stats() -> (u32, u32) {
    (DIRECT_REQS.load(Ordering::Relaxed), DIRECT_SECTORS.load(Ordering::Relaxed))
}

/// Write-back statistics: `(failures, retries, exhausted, dirty_slots)`.
///
/// * `failures` — write-back attempts that the device rejected, every
///   attempt counted.
/// * `retries` — of those attempts, the ones that were a repeat at a slot
///   that had already failed. This is the number that used to be invisible.
/// * `exhausted` — slots that used up [`WRITEBACK_ATTEMPTS`]; their sector
///   is still in the cache and still dirty, but no longer written back on
///   its own.
/// * `dirty_slots` — dirty slots right now, i.e. sectors the card has not
///   got yet.
pub fn writeback_stats() -> (u32, u32, u32, u32) {
    let dirty = lists()
        .iter()
        .flat_map(|l| l.slots.iter())
        .filter(|s| s.valid && s.dirty)
        .count() as u32;
    (
        WB_FAILURES.load(Ordering::Relaxed),
        WB_RETRIES.load(Ordering::Relaxed),
        WB_EXHAUSTED.load(Ordering::Relaxed),
        dirty,
    )
}
