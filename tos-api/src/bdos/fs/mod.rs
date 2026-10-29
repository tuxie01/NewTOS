//! BDOS file system (Phase 09).
//!
//! Layered after EmuTOS `bdos/`: [`fat`] is the FAT-table / cluster-chain
//! reader (mirrors `fsfat.c`) — the foundation the directory traversal
//! and the file handles build on. Read-only in D1; the sector cache
//! (D2), path parser (D3) and the GEMDOS file/dir calls follow.

pub mod cache;
pub mod curdir;
pub mod dir;
pub mod dirops;
pub mod drive;
pub mod fat;
pub mod file;
pub mod find;
pub mod fsio;
pub mod path;

use crate::trap::TrapArgs;
use crate::uaccess::UserFault;

/// Longest path a GEMDOS call reads from its caller, in bytes (without the
/// NUL). A longer string is truncated here, as the raw reader did.
pub(crate) const PATH_ARG_MAX: usize = 128;

/// Buffer a path argument is copied into ([`path_arg`]).
pub(crate) type PathBuf = [u8; PATH_ARG_MAX];

/// Read the NUL-terminated path string of pointer argument `i` of `args`
/// (max [`PATH_ARG_MAX`] bytes) into `buf` and return it as text.
///
/// The pointer is read in the caller's space ([`TrapArgs::space`], ARCH03
/// stage 3a): an EL0 program's path through the fault-safe unprivileged
/// loads of [`crate::uaccess`] — a pointer it cannot read stops the call and
/// the syscall tail ends the program (EmuTOS: bus error in the trap,
/// `docs/api/bios/_exception-crash.md`) — and an in-kernel caller's (the
/// desktop's `svc`, the AES fiber, the kernel's direct calls) as the kernel
/// address it is.
///
/// `Ok(None)` for a null pointer or a string that is not UTF-8 — the cases
/// the raw reader answered with `None` and each call maps to its own error
/// code; `Err` after a fault (the code the call returns then never reaches
/// the program).
pub(crate) fn path_arg<'b>(
    args: &TrapArgs,
    i: usize,
    buf: &'b mut PathBuf,
) -> Result<Option<&'b str>, UserFault> {
    let ptr = args.arg_u64(i);
    if ptr == 0 {
        return Ok(None);
    }
    let len = args.space.read_cstr(ptr, buf)?;
    Ok(core::str::from_utf8(&buf[..len]).ok())
}

/// A resolved on-disk program file, streamable by byte offset without
/// the SFT handle table. The program loader opens one of these and reads
/// each `PT_LOAD` segment's file range straight into the mapped TPA, so
/// there is no fixed intermediate buffer capping the program size.
///
/// Holds the mounted [`fat::Volume`], the file's first cluster and its
/// length — the minimum to position a [`fsio::Stream`] for a range read.
#[derive(Clone, Copy)]
pub struct FileImage {
    vol: fat::Volume,
    start_cluster: u32,
    file_len: u32,
}

impl FileImage {
    /// Resolve `path` to a file on its drive. Returns `None` for a
    /// path/IO error or if the name is a directory (the loader only
    /// streams regular files).
    pub fn open(path: &str) -> Option<FileImage> {
        let (vol, parent, name11) = path::findit(path).ok()?;
        let entry = dir::find_in_dir(&vol, parent, &name11)?;
        if entry.attrib & 0x10 != 0 {
            return None; // a sub-directory, not a file
        }
        Some(FileImage {
            vol,
            start_cluster: entry.cluster,
            file_len: entry.size,
        })
    }

    /// File length in bytes (the ELF total image size).
    pub fn len(&self) -> u32 {
        self.file_len
    }

    /// `true` for a zero-length file (no cluster chain).
    pub fn is_empty(&self) -> bool {
        self.file_len == 0
    }

    /// Read up to `dst.len()` bytes starting at byte `offset` into `dst`.
    /// Returns the number of bytes read — fewer than `dst.len()` only at end
    /// of file or where the cluster chain ends early — or `None` on a
    /// block-device read error.
    ///
    /// The same engine as `Fread` (FS08, [`fsio`]): positioned with
    /// `ixlseek`, whole records read straight into `dst` in one block request
    /// per run of contiguous clusters, partial ones through the cache.
    pub fn read_at(&self, offset: u32, dst: &mut [u8]) -> Option<usize> {
        if offset >= self.file_len {
            return Some(0);
        }
        let want = dst.len().min((self.file_len - offset) as usize) as u32;
        if want == 0 {
            return Some(0);
        }
        let mut s = fsio::Stream::new(self.vol, self.start_cluster, self.file_len);
        let pos = fsio::ixlseek(&mut s, i64::from(offset));
        if pos < 0 {
            // A chain shorter than the length (EINTRN) reads nothing, as
            // before; a device error during the walk is a read error.
            return if cache::error_pending() { None } else { Some(0) };
        }
        read_stream(&mut s, &mut dst[..want as usize])
    }
}

/// Read `dst.len()` bytes (at most) from the position of `s` into the kernel
/// buffer `dst`. `None` on a device error.
fn read_stream(s: &mut fsio::Stream, dst: &mut [u8]) -> Option<usize> {
    let space = crate::uaccess::Space::Kernel;
    match fsio::xrw(s, false, dst.len() as u32, space, dst.as_mut_ptr() as u64) {
        Ok(n) => Some(n as usize),
        // A kernel buffer cannot fault; a device failure is a read error.
        Err(_) => None,
    }
}

/// Read a whole file at `path` into `out` (truncated to `out.len()`).
/// Returns the number of bytes read, or `None` on a path/IO error or if
/// the name is a directory. Used by the AES resource loader, the shell and
/// the keyboard-table loader to pull a file off the FAT file system without
/// going through the SFT handle table.
///
/// Runs under the restart after a media change, as every one of these reads
/// is a GEMDOS call in EmuTOS (`dos_open`/`dos_read`/`dos_sfirst` through
/// `trap #1`, `osif`'s restart; TASK/FS22): a read that meets a change logs
/// the drive in again and is made once more. A run holds nothing but `out`,
/// which the next run writes again; the caller sees only the last run's
/// result. `None` also when the restart gives up (`E_CHNG`).
pub fn read_file(path: &str, out: &mut [u8]) -> Option<usize> {
    crate::gemdos::restartable(|| read_file_once(path, out)).ok().flatten()
}

/// One run of [`read_file`].
fn read_file_once(path: &str, out: &mut [u8]) -> Option<usize> {
    let (vol, parent, name11) = path::findit(path).ok()?;
    let entry = dir::find_in_dir(&vol, parent, &name11)?;
    if entry.attrib & 0x10 != 0 {
        return None; // a sub-directory, not a file
    }
    let want = (entry.size as usize).min(out.len());
    let mut s = fsio::Stream::new(vol, entry.cluster, entry.size);
    read_stream(&mut s, &mut out[..want])
}
