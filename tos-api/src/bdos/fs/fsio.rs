//! The file transfer engine — EmuTOS `bdos/fsio.c` `xrw` / `ixlseek` with
//! `nextcl` from `bdos/fsfat.c` (TASK/FS08 part 2).
//!
//! Spec: `docs/api/gemdos/_state.md` §3a. One [`Stream`] holds what EmuTOS
//! keeps in an OFD and its DFD for a transfer: the file's start cluster and
//! length (`o_strtcl`, `o_fileln`, `O_DIRTY`) and the four position fields
//! `o_bytnum` / `o_curcl` / `o_currec` / `o_curbyt`. With them a sequential
//! `Fread`/`Fwrite` steps the FAT chain **one link per new cluster**
//! ([`nextcl`]) instead of walking it from the file start on every call, and
//! [`ixlseek`] chains forward from the current cluster where it can.
//!
//! [`xrw`] splits a transfer like EmuTOS: partial head/tail records through
//! the cache's data list, whole records in **one block request per run of
//! contiguous clusters** straight between the device and the caller's
//! buffer ([`usrio`], `bdos/fsio.c:57-74`).
//!
//! Records are the volume's logical sectors (`recsiz`); the file system
//! below assumes they are the device's 512-byte sectors, as it did before.

use super::cache::{self, Kind};
use super::fat::Volume;
use crate::uaccess::Space;
use tos_types::error::{EINTRN, ERANGE};
use tos_types::Long;

/// `CNTMAX` — the most records one `usrio` moves, "16-bit MAXINT"
/// (`bdos/fsio.c:22`); a longer run of contiguous clusters is split.
const CNTMAX: u32 = 0x7FFF;

/// Why a transfer stopped before its end.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Fail {
    /// The caller's buffer could not be accessed (ARCH03 3a): the syscall
    /// tail ends the program.
    Fault,
    /// The block device failed; the TOS code is pending in
    /// [`cache::take_error`] (EmuTOS `longjmp_rwabs`, `bdos/fs.h:60`).
    Device,
}

/// The transfer state of one open file — EmuTOS `OFD` position fields plus
/// the `DFD` fields they move (`docs/api/structures/ofd.md`).
#[derive(Clone, Copy)]
pub struct Stream {
    /// The mounted volume the file lives on (`o_dmd`).
    pub vol: Volume,
    /// `o_strtcl`: first cluster (0 = the file has none yet).
    pub strtcl: u32,
    /// `o_fileln`: file length in bytes.
    pub fileln: u32,
    /// `O_DIRTY`: start cluster or length changed, the directory entry needs
    /// rewriting.
    pub dirty: bool,
    /// `o_bytnum`: byte position in the file.
    pub bytnum: u32,
    /// `o_curcl`: the cluster holding the last byte moved (0 = none yet).
    pub curcl: u32,
    /// `o_currec`: first data-area record of `curcl`.
    pub currec: u32,
    /// `o_curbyt`: bytes of `curcl` behind the position; 0 and `clsizb` both
    /// mean "`curcl` is used up" (`_state.md` §3a).
    pub curbyt: u32,
}

impl Stream {
    /// A file positioned at its start (`o_curcl = o_currec = o_bytnum =
    /// o_curbyt = 0`, as `makopn` leaves a fresh OFD).
    pub fn new(vol: Volume, strtcl: u32, fileln: u32) -> Stream {
        Stream { vol, strtcl, fileln, dirty: false, bytnum: 0, curcl: 0, currec: 0, curbyt: 0 }
    }
}

/// EmuTOS `addit` (`bdos/fsio.c:33-47`): account for `siz` bytes moved.
/// `o_curbyt` only grows for transfers smaller than a cluster.
fn addit(s: &mut Stream, siz: u32) {
    if siz < s.vol.clsizb {
        s.curbyt += siz;
    }
    s.bytnum += siz;
    if s.bytnum > s.fileln {
        s.fileln = s.bytnum;
        s.dirty = true;
    }
}

/// EmuTOS `nextcl` (`bdos/fsfat.c:247-293`): step to the cluster after
/// `o_curcl` (the start cluster when there is none yet), allocating one on a
/// write past the end of the chain. `Ok(false)` at the end of the chain or
/// when no free cluster is left.
pub fn nextcl(s: &mut Stream, wrt: bool) -> Result<bool, Fail> {
    let vol = s.vol;
    let cl = s.curcl;
    let mut eoc;
    let mut cl2;
    if cl == 0 {
        // "initial value"
        cl2 = s.strtcl;
        eoc = cl2 == 0;
    } else {
        cl2 = match vol.fat_entry(cl) {
            Some(n) => n,
            // `cl` is a data cluster (it was checked when it became current),
            // so a failed lookup is a failed FAT read.
            None => return Err(Fail::Device),
        };
        eoc = vol.is_eoc(cl2);
    }

    if wrt && eoc {
        // "end of file, allocate new clusters"
        cl2 = match vol.findfree(cl) {
            Some(c) => c,
            None if cache::error_pending() => return Err(Fail::Device),
            None => return Ok(false), // disk full
        };
        if !vol.set_fat_entry(cl2, Volume::EOC) {
            return Err(Fail::Device);
        }
        if cl != 0 {
            if !vol.set_fat_entry(cl, cl2) {
                return Err(Fail::Device);
            }
        } else {
            s.strtcl = cl2;
            s.dirty = true;
        }
        eoc = false;
    }

    // "treat invalid cluster num as end of chain" — `cl2 < 2` in EmuTOS;
    // a number past the volume's last cluster is treated the same here.
    if eoc || !vol.is_data_cluster(cl2) {
        return Ok(false);
    }
    s.curcl = cl2;
    s.currec = vol.cl2rec(cl2);
    s.curbyt = 0;
    Ok(true)
}

/// EmuTOS `usrio` (`bdos/fsio.c:57-74`): move `num` whole records starting
/// at data-area record `strt` directly between the device and the caller's
/// buffer at `ubuf`, in one block request. The caller's range is checked
/// with its rights and reached through its kernel alias, as BIOS `Rwabs`
/// does (`tos-api/src/bios/mod.rs`).
fn usrio(s: &Stream, wrt: bool, num: u32, strt: u32, space: Space, ubuf: u64) -> Result<(), Fail> {
    let bytes = num as usize * s.vol.recsiz as usize;
    // Reading from the device writes the caller's buffer, and vice versa.
    let kva = space.kptr(ubuf, bytes as u64, !wrt).map_err(|_| Fail::Fault)?;
    let lba = s.vol.data_lba(strt);
    let ok = if wrt {
        // SAFETY: `kva .. kva + bytes` was just checked readable by the
        // caller (or is a kernel caller's buffer) and stays mapped for the
        // call; nothing else aliases it on the single trap path.
        let buf = unsafe { core::slice::from_raw_parts(kva as *const u8, bytes) };
        cache::direct_write(s.vol.drive(), lba, buf)
    } else {
        // SAFETY: as above, checked writable.
        let buf = unsafe { core::slice::from_raw_parts_mut(kva as *mut u8, bytes) };
        cache::direct_read(s.vol.drive(), lba, buf)
    };
    if ok { Ok(()) } else { Err(Fail::Device) }
}

/// `getrec` + `memcpy` for a partial record: `n` bytes at offset `off` of
/// data-area record `rec`, through the cache's data list. A write reads the
/// record first and marks it dirty (`getrec(…, 1)`, `bdos/fsbuf.c:231`).
fn partial(
    s: &Stream,
    wrt: bool,
    rec: u32,
    off: u32,
    n: u32,
    space: Space,
    ubuf: u64,
) -> Result<(), Fail> {
    let (a, b) = (off as usize, (off + n) as usize);
    let r = cache::with_sector(Kind::Data, s.vol.drive(), s.vol.data_lba(rec), wrt, |buf| {
        if wrt {
            space.copy_from(&mut buf[a..b], ubuf)
        } else {
            space.copy_to(ubuf, &buf[a..b])
        }
    });
    match r {
        None => Err(Fail::Device),
        Some(Err(_)) => Err(Fail::Fault),
        Some(Ok(())) => Ok(()),
    }
}

/// EmuTOS `xrw_recs` (`bdos/fsio.c:84-187`): `numrecs` whole records from
/// data-area record `startrec` on. `Ok(None)` when the end of the cluster
/// chain was reached, else the advanced buffer address.
fn xrw_recs(
    s: &mut Stream,
    wrt: bool,
    startrec: u32,
    mut numrecs: u32,
    space: Space,
    mut ubuf: u64,
) -> Result<Option<u64>, Fail> {
    let clsiz = s.vol.clsiz;
    let recsiz = s.vol.recsiz;

    // 'header' records: the rest of the current cluster (`:98-117`).
    let mut hdrrec = startrec % clsiz;
    if hdrrec != 0 {
        hdrrec = (clsiz - hdrrec).min(numrecs);
        usrio(s, wrt, hdrrec, startrec, space, ubuf)?;
        let nbytes = hdrrec * recsiz;
        ubuf += u64::from(nbytes);
        addit(s, nbytes);
        numrecs -= hdrrec;
    }

    let tailrec = numrecs % clsiz;

    // whole (middle) clusters, contiguous ones merged (`:125-169`).
    let mut numclus = numrecs / clsiz;
    let mut last = 0u32;
    let mut nrecs = 0u32;
    let mut first_time = true;
    loop {
        let mut end_of_chain = false;
        if numclus != 0 {
            end_of_chain = !nextcl(s, wrt)?;
        }
        if first_time {
            last = s.currec;
            first_time = false;
        }
        // "if necessary, complete pending data transfer"
        if (end_of_chain
            || numclus == 0
            || s.currec != last + nrecs
            || nrecs + clsiz > CNTMAX)
            && nrecs != 0
        {
            usrio(s, wrt, nrecs, last, space, ubuf)?;
            let nbytes = nrecs * recsiz;
            addit(s, nbytes);
            ubuf += u64::from(nbytes);
            last = s.currec;
            nrecs = 0;
        }
        if end_of_chain {
            return Ok(None);
        }
        if numclus == 0 {
            break;
        }
        nrecs += clsiz;
        numclus -= 1;
    }

    // 'tail' records (`:174-184`).
    if tailrec != 0 {
        if !nextcl(s, wrt)? {
            return Ok(None);
        }
        usrio(s, wrt, tailrec, s.currec, space, ubuf)?;
        let nbytes = tailrec * recsiz;
        addit(s, nbytes);
        ubuf += u64::from(nbytes);
    }
    Ok(Some(ubuf))
}

/// EmuTOS `xrw` (`bdos/fsio.c:208-284`): move `len` bytes between the file
/// position of `s` and the caller's buffer `ubuf` (in `space`). Returns the
/// bytes moved — fewer than `len` when the chain ends (end of file on a
/// read, disk full on a write).
///
/// On `Err` the position fields describe the bytes moved before the stop.
pub fn xrw(s: &mut Stream, wrt: bool, mut len: u32, space: Space, mut ubuf: u64) -> Result<u32, Fail> {
    let recsiz = s.vol.recsiz;
    let clsiz = s.vol.clsiz;
    let bytpos = s.bytnum;

    let mut recn = s.curbyt / recsiz;
    let bytn = s.curbyt % recsiz;
    recn += s.currec;

    // header bytes (`:228-245`)
    if bytn != 0 {
        let lenxfr = len.min(recsiz - bytn);
        partial(s, wrt, recn, bytn, lenxfr, space, ubuf)?;
        addit(s, lenxfr);
        len -= lenxfr;
        recn += 1;
        ubuf += u64::from(lenxfr);
    }

    // middle records, then the tail (`:250-258`)
    let lentail = len % recsiz;
    let numrecs = (len - lentail) / recsiz;
    if numrecs != 0 {
        match xrw_recs(s, wrt, recn, numrecs, space, ubuf)? {
            Some(next) => ubuf = next,
            None => return Ok(s.bytnum - bytpos), // end of cluster chain
        }
    }

    // tail bytes (`:261-278`)
    if lentail != 0 {
        let mut recn = s.curbyt / recsiz;
        if recn == 0 || recn == clsiz {
            if !nextcl(s, wrt)? {
                return Ok(s.bytnum - bytpos);
            }
            recn = 0;
        }
        partial(s, wrt, s.currec + recn, 0, lentail, space, ubuf)?;
        addit(s, lentail);
    }

    Ok(s.bytnum - bytpos)
}

/// EmuTOS `ixlseek` (`bdos/fsio.c:354-426`): position `s` at byte `n`.
/// Returns `n`, `ERANGE` outside `0..=fileln`, `EINTRN` for a cluster chain
/// shorter than the length says, or the device's code if a FAT read fails.
pub fn ixlseek(s: &mut Stream, n: i64) -> Long {
    if n < 0 || n > i64::from(s.fileln) {
        return ERANGE;
    }
    let n = n as u32;
    if n == 0 {
        s.curcl = 0;
        s.currec = 0;
        s.bytnum = 0;
        s.curbyt = 0;
        return 0;
    }
    let vol = s.vol;
    let clsizb = vol.clsizb;

    // the desired position in units of 1 cluster
    let mut clnum = n / clsizb;
    let mut clx;
    if s.curcl != 0 && n >= s.bytnum {
        // "OK, we can chain forward"
        let mut curnum = s.bytnum / clsizb;
        if (s.curbyt == 0 || s.curbyt == clsizb) && s.bytnum != 0 {
            curnum = curnum.saturating_sub(1);
        }
        clnum = clnum.saturating_sub(curnum);
        clx = s.curcl;
    } else {
        // "we have to start at the beginning"
        clx = s.strtcl;
    }

    // "go one less if on cluster boundary" — xrw steps before it transfers.
    if n.is_multiple_of(clsizb) {
        clnum = clnum.saturating_sub(1);
    }

    for _ in 0..clnum {
        clx = match vol.fat_entry(clx) {
            Some(c) if !vol.is_eoc(c) && vol.is_data_cluster(c) => c,
            _ if cache::error_pending() => return cache::take_error().unwrap_or(EINTRN),
            // "FAT chain is shorter than filesize says ..."
            _ => return EINTRN,
        };
    }
    if !vol.is_data_cluster(clx) {
        return EINTRN; // a file with a length but no (valid) start cluster
    }

    s.curcl = clx;
    s.currec = vol.cl2rec(clx);
    s.bytnum = n;
    s.curbyt = n % clsizb;
    n as Long
}
