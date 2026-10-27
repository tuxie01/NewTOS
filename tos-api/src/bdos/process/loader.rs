//! ELF-to-slot loader — the TPA in the shared address space (ARCH03 stage 1).
//!
//! Combines [`super::elf`] (parser), [`super::reloc`] (relocation),
//! [`super::arena`] (the pool-backed TPA) and [`super::mmu`] (the process
//! root) into a single function: given an ELF image and a process slot,
//! take a TPA block from the BDOS pool, stream the program into it, relocate
//! it for the address it landed at, write the [`BasePage`] at its start and
//! the process's private page 0, and build the process root.
//!
//! ## Layout
//!
//! Every user address is its own physical address (VA = PA, [`super::mmu`]),
//! so the TPA executes where the pool put it — at its block address `B`, the
//! process's **PD pointer** (plan §3.5, EmuTOS `p_lowtpa = p`,
//! `bdos/proc.c:372`):
//!
//! ```text
//!   B + 0x000      BasePage           (sizeof BasePage = 256 B)
//!   B + 0x100      free                (reserved, plan §3.5)
//!   B + 0x1000     program image       (PT_LOAD segments, linked at 0x1000,
//!                                        relocated by B — `sdk/newtos.ld`)
//!   image_end      BSS                 (zeroed)
//!                  free TPA            (the program's to Mshrink)
//!   p_hitpa        SP_EL0              (= B + the TPA length)
//! ```
//!
//! The TPA is the largest free block of the pool the program flags choose
//! (EmuTOS `alloc_tpa`, `bdos/proc.c:460-502`, [`arena::alloc_tpa`]); the
//! program shrinks it at start (`Mshrink`, the SDK start-up code as mintlib's
//! `_crtinit`), the AES-side loader of an accessory right after loading it
//! (EmuTOS `pgmld`, `util/gemdos.c:29-47`; NewTOS `kernel/src/accs.rs`).
//!
//! `p_tbase = B + 0x1000` rather than EmuTOS' `B + 0x100` (plan §3.5): the
//! image keeps the 4-KiB page offsets it was linked with. The basepage
//! carries the TPA and segment extents (`p_lowtpa`/`p_hitpa`/`p_tbase`/…/
//! `p_bbase`) so a libc/crt0 can find its heap and stack; the entry gets `B`
//! in `x0`.
//!
//! Page 0 (`[0, 0x1000)`) is **per process** and not part of the TPA (plan
//! E3, §3.1): the classic system variables, the OSHEADER, the `$5A0` cookie
//! jar, the `os_run` cell and the `Keytbl` area live in the slot's own page
//! ([`mmu::page0_kva`]), EL0 read-only outside the super state (MEM12). The
//! EL0 stubs are in the one shared stub page, the KEYS02 table and the
//! `PARMBLK` scratch in the slot's private read-write page
//! ([`ensure_stub_page`], [`mmu::priv_page_kva`]).
//!
//! ## Deliberate simplifications
//!
//! - **The whole pool is `UserRwExec`.** No `.text` RX / data RW-NX split; a
//!   per-segment permission split is a later hardening step and does not
//!   change the observable program contract.
//! - **Zeroing as EmuTOS `kpgmld`** (`bdos/kpgmld.c:212-221`): the
//!   basepage page and the image span (BSS included) are always zeroed;
//!   the rest of the TPA up to `p_hitpa` only without `PF_FASTLOAD`
//!   ("clear the bss or the whole heap"). The SDK `crt0` therefore does
//!   **not** zero BSS itself.

use core::cell::UnsafeCell;
use core::ffi::c_void;
use core::sync::atomic::Ordering;

use tos_types::prot_from_prgflags;
use tos_types::{
    BasePage, Long, MX_PREFTTRAM, MX_STRAM, NUMSTD, PDCLSIZE, PF_FASTLOAD, PF_STANDARD, PF_TTRAMLOAD,
};

use super::{kalias, usermem};

use crate::bdos::fs::FileImage;

use super::arena;
use super::elf::{self, ElfImage, ParseError, PF_W, PF_X};
use super::mmu::{self, MapError};
use super::reloc::{self, RelocError};
use super::PROCESS_TABLE;

/// Failure modes for [`load_to_slot`].
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum LoadError {
    /// The blob did not parse as a valid AArch64 ELF executable.
    Parse(ParseError),
    /// `slot` is `>= MAX_PROCESSES`.
    InvalidSlot,
    /// A `PT_LOAD` segment lies below the reserved low region. The first
    /// page of the TPA is reserved for the basepage and the classic
    /// low-memory system area (sysvars/OSHEADER/cookie-jar); segments must
    /// start at `USER_VA_BASE + RESERVED_LOW_END` (= 0x1000) or later.
    /// Anything below that produces this error (B1 / Option 1).
    SegmentVaOutOfRange,
    /// A `PT_LOAD` segment's `vaddr + mem_size` overflows the address
    /// arithmetic. (An image that does not fit the largest free block is
    /// [`Self::NoTpaMemory`], as EmuTOS `alloc_tpa` answers it.)
    ProgramTooLarge,
    /// Building the process root failed ([`super::mmu`]).
    MapPage(MapError),
    /// The environment block could not be allocated from the BDOS pool
    /// (`Mxalloc` returned 0). EmuTOS `alloc_env` failure →
    /// `ENSMEM` (`bdos/proc.c:284-288`).
    NoEnvMemory,
    /// The per-process TPA could not be allocated from the BDOS pool
    /// (`Mxalloc` returned 0). Maps to `ENSMEM` — the classic Atari
    /// "not enough memory to load the program" (EmuTOS `bdos/proc.c`
    /// `xexec` → `ENSMEM`).
    NoTpaMemory,
    /// The image is `ELFCLASS32` (ILP32). NewTOS runs native 64-bit
    /// applications only; the ILP32 **build** path was retired on
    /// 2026-09-04 (ADR-020 addendum) and no ILP32 program has been
    /// producible since. Executing one is refused here rather than
    /// left to a runtime path that nothing builds and nothing tests.
    /// Maps to `EPLFMT` (-66) — "invalid program load format", the same
    /// answer EmuTOS gives a header it will not load
    /// (`bdos/kpgmld.c:47-63`).
    Ilp32Unsupported,
    /// The image's dynamic relocations could not be applied — an
    /// unsupported relocation type or dynamic tag, or a table/place outside
    /// the image (ARCH03 stage 0d, [`super::reloc`]). Maps to `EPLFMT`,
    /// like every other "this file is not a program I can load" answer.
    Reloc(RelocError),
}

/// How `Pexec` supplies the child's environment block. Mirrors the
/// `env` argument of EmuTOS `xexec` / `alloc_env` (`bdos/proc.c:413`).
#[derive(Copy, Clone, Debug)]
pub enum EnvSpec<'a> {
    /// No environment is set — `p_env` stays `0`. Used by the
    /// kernel-launch selftests (`load_to_slot`), which never handed the
    /// child an env and predate this plumbing.
    None,
    /// Copy this explicit environment image verbatim. The slice is the
    /// caller's block **including** its terminating double-NUL, i.e.
    /// `envsize` bytes (`bdos/proc.c:150`).
    Explicit(&'a [u8]),
    /// `env == NULL`: duplicate the parent's `run->p_env`
    /// (`bdos/proc.c:419-420`). With no running process `run` is the
    /// kernel's stand-in PD ([`super::standin::kernel_run_env`],
    /// TASK/PPARENT): `initial_basepage`'s empty double-NUL env
    /// (`bdos/bdosmain.c:341`) before the AES, the AES / accdesk / EmuCON
    /// environment (copies of `default_env`, `bios/bios.c:626-636`;
    /// TASK/PROC03) after it. A parent whose
    /// `p_env` is 0, or a kernel with no stand-ins yet, hands down the empty
    /// env.
    Inherit,
}

/// The empty environment: a single double-NUL. Matches EmuTOS's
/// `double_nul` static (`bdos/bdosmain.c:341`).
static EMPTY_ENV: [u8; 2] = [0, 0];

impl From<ParseError> for LoadError {
    fn from(e: ParseError) -> Self {
        Self::Parse(e)
    }
}
impl From<RelocError> for LoadError {
    fn from(e: RelocError) -> Self {
        Self::Reloc(e)
    }
}
impl From<MapError> for LoadError {
    fn from(e: MapError) -> Self {
        Self::MapPage(e)
    }
}

/// Result of a successful load — what the caller needs to drop the
/// CPU into EL0.
#[derive(Copy, Clone, Debug)]
pub struct LoadedImage {
    /// Process slot the image was loaded into.
    pub slot: usize,
    /// User VA of the program's entry point: the TPA base plus the ELF
    /// `e_entry` (the image is relocated for the TPA base).
    pub entry_va: u64,
    /// Phys of `slot`'s `L0` table — feed this to
    /// [`super::mmu::install_ttbr0`] right before the `eret`.
    pub l0_pa: u64,
    /// The **PD pointer**: user VA of the [`BasePage`] populated by
    /// [`populate_basepage`] = the TPA block's address `B` (VA = PA). The
    /// child gets it in `x0`.
    pub basepage_va: u64,
    /// Initial `SP_EL0` for the child: the TPA top (= the basepage's
    /// `p_hitpa`), 16-byte-aligned per AAPCS64 (a page boundary with
    /// protection on). The kernel launch path sets `SP_EL0` to this; the SDK
    /// start-up code moves the stack into the part of the TPA it keeps
    /// (mintlib `_crtinit`: `bp + m`).
    pub sp_el0: u64,
    /// Bytes from the PD pointer to the end of the image (BSS included) —
    /// what EmuTOS `pgmld` keeps of an accessory's TPA (`p_tlen + p_dlen +
    /// p_blen + 0x100`, `util/gemdos.c:40-43`; the NewTOS image starts at
    /// `B + 0x1000`, so it is the image end, not the sum). `sizeof(PD)` for a
    /// `Pexec` 5/7 basepage.
    pub image_len: u64,
}

/// Base of **page 0** — the per-process page at user VA `[0, 0x1000)` that
/// holds the classic low-memory system area and the `Keytbl` area (plan E3;
/// the stubs and the KEYS02 table moved to the stub page and the private
/// read-write page with MEM12). The page-0 VAs below are offsets from it. Since ARCH03 stage 1 the basepage is **not** here any more but
/// at the start of the process's TPA block ([`LoadedImage::basepage_va`]).
pub const USER_VA_BASE: u64 = 0x0;

/// Number of bytes at the start of the TPA reserved for the
/// [`BasePage`].
const BASEPAGE_RESERVED: u64 = core::mem::size_of::<BasePage>() as u64;

/// Size of page 0 and the lowest link address a `PT_LOAD` segment may
/// have. Page 0 holds the classic TOS low-memory system area (`[0x400,
/// 0x670)`: sysvars + OSHEADER + cookie jar, written by [`populate_sysvars`])
/// and the trampolines/vector tables above it; in the TPA, the same first
/// 4 KiB hold the [`BasePage`] (`[0x000, 0x100)`) and the reserved gap above
/// it. The SDK links native apps here (`sdk/newtos.ld` base `0x1000`). B1 /
/// Option 1, `docs/analysis/bios-audit/b1-lowmem-feasibility.md` §Option 1.
const RESERVED_LOW_END: u64 = 0x1000;

// The basepage fits the TPA's first 4 KiB, and the classic sysvar block
// (`[0x400, 0x800)`) fits page 0.
const _: () = assert!(BASEPAGE_RESERVED <= RESERVED_LOW_END);
const _: () = assert!(SYSVAR_AREA_END <= RESERVED_LOW_END);

// A segment's link VA is its byte offset in the TPA: segment `s` is streamed
// to `tpa_bytes[s.vaddr]`, and the TPA executes at its base `B`, so the
// relocation load base (the execution address of link VA 0) is `B`
// (`populate_and_map`). The image must start page-aligned in the TPA (it is
// linked at `RESERVED_LOW_END`, a page boundary), hence the page-aligned TPA.
const _: () = assert!(RESERVED_LOW_END.is_multiple_of(arena::TPA_ALIGN));

/// Where the program image comes from. There is **no** fixed
/// whole-file staging buffer: only the ELF header + program-header table
/// pass through a small buffer ([`HEADER_BUF`]); every `PT_LOAD` payload
/// streams straight into the mapped TPA, so a program is capped only by
/// the TPA size, never by an intermediate buffer.
///
/// * [`ImageSource::Memory`] — an in-memory blob: an embedded registry
///   program or a selftest image (`&'static [u8]`). The whole blob is
///   already resident, so it doubles as its own header window.
/// * [`ImageSource::File`] — a FAT file streamed by byte offset via
///   [`FileImage::read_at`]; the header is read into [`HEADER_BUF`] and
///   each segment's `[p_offset, p_offset + p_filesz)` range is read
///   directly into the TPA VA.
pub enum ImageSource<'a> {
    /// In-memory ELF blob (embedded program / selftest image).
    Memory(&'a [u8]),
    /// FAT file streamed by offset.
    File(FileImage),
}

impl ImageSource<'_> {
    /// Read exactly `dst.len()` bytes of the image starting at byte
    /// `offset` into `dst`. `false` on a short read or IO error (a
    /// segment whose file range is not fully backed by the image); the
    /// parser has already validated `offset + dst.len() <= total_len`,
    /// so a well-formed image always fills `dst` completely.
    fn read_exact_at(&self, offset: u64, dst: &mut [u8]) -> bool {
        match self {
            Self::Memory(b) => {
                let Some(end) = (offset as usize).checked_add(dst.len()) else {
                    return false;
                };
                match b.get(offset as usize..end) {
                    Some(src) => {
                        dst.copy_from_slice(src);
                        true
                    }
                    None => false,
                }
            }
            Self::File(f) => {
                let Ok(off) = u32::try_from(offset) else {
                    return false;
                };
                matches!(f.read_at(off, dst), Some(n) if n == dst.len())
            }
        }
    }
}

/// Small buffer for the ELF header + program-header table of a streamed
/// file (never a segment payload). 8 KiB holds `Ehdr` (64 B) plus up to
/// ~145 program headers — orders of magnitude more than any real TOS
/// executable, which emit `phoff == 64` and a handful of phdrs.
const HEADER_BUF_SIZE: usize = 8 * 1024;
struct HeaderBuf(UnsafeCell<[u8; HEADER_BUF_SIZE]>);
// SAFETY: accessed only from the single-CPU, BDOS-locked trap path.
unsafe impl Sync for HeaderBuf {}
static HEADER_BUF: HeaderBuf = HeaderBuf(UnsafeCell::new([0u8; HEADER_BUF_SIZE]));

/// Load an ELF blob into the given process slot.
///
/// Sequence:
/// 1. Parse the blob via [`elf::parse`].
/// 2. Take a page-aligned TPA block from the BDOS pool ([`arena::alloc_tpa`]).
/// 3. Stream each `PT_LOAD` segment into it, zero the rest, and apply the
///    image's `R_AARCH64_RELATIVE` relocations for the block address
///    ([`super::reloc`]).
/// 4. Write the basepage at the block start and the slot's page 0, build
///    the slot's root ([`super::mmu::init_slot_tables`]).
/// 5. Return a [`LoadedImage`] ready for the caller's eret.
///
/// This legacy 2-arg entry (the kernel-launch D3/D6/D7 selftests) hands no
/// command tail and no environment, and gives the TPA to the child.
///
/// # Safety
///
/// Caller holds the BDOS giant lock; the slot must not currently
/// be the active TTBR0 (or the TLB must be flushed before the
/// next user-space access).
pub unsafe fn load_to_slot(
    elf_bytes: &[u8],
    slot: usize,
) -> Result<LoadedImage, LoadError> {
    // SAFETY: forwarded — same BDOS-lock / trap-context contract.
    unsafe { load_to_slot_with(elf_bytes, slot, &[], EnvSpec::None, true) }
}

/// Load a FAT [`FileImage`] into `slot`, streaming each `PT_LOAD`
/// segment straight from the file into the TPA — the `Pexec`
/// file-path entry. Segments never pass through a fixed buffer, so the
/// program is limited only by the TPA size, not by any load-buffer cap.
/// See [`load_to_slot_with`] for the `tail` / `env` / ownership
/// semantics (identical; only the image source differs).
///
/// # Safety
///
/// Caller holds the BDOS giant lock; see [`load_to_slot`].
pub unsafe fn load_file_to_slot(
    file: FileImage,
    slot: usize,
    tail: &[u8],
    env: EnvSpec<'_>,
    owned_by_child: bool,
) -> Result<LoadedImage, LoadError> {
    // SAFETY: forwarded — same BDOS-lock / trap-context contract.
    unsafe { load_from_source(ImageSource::File(file), slot, tail, env, owned_by_child) }
}

/// Load an ELF blob into `slot`, additionally populating the child's
/// command tail (`p_cmdlin`) and environment (`p_env`) per `Pexec`.
///
/// This is the full `Pexec`-path entry; [`load_to_slot`] is the
/// tail-/env-less legacy wrapper. See [`load_to_slot`] for the load
/// sequence; the only additions are:
///
///   * **Command tail** (`tail`): copied verbatim into `p_cmdlin`
///     (offset 0x80), truncated to 127 bytes, NUL-terminated —
///     EmuTOS `init_pd_fields` (`bdos/proc.c:378-383`). The classic
///     Atari image is a 1-byte length followed by the text; the copy
///     is byte-for-byte, so `p_cmdlin[0]` is whatever the caller put
///     there (the length under the convention).
///   * **Environment** (`env`): allocated from the BDOS pool and copied
///     verbatim — EmuTOS `alloc_env` (`bdos/proc.c:413-431`). `p_env`
///     receives the block's address, which every process can read (the
///     pool is shared, VA = PA — MEM03).
///   * **Ownership** (`owned_by_child`): EmuTOS `owner = (flag ==
///     PE_LOADGO) ? p : run` (`bdos/proc.c:302-307`) for **both** the TPA and
///     the environment. `true` (`PE_LOADGO`, and every program the kernel
///     launches itself) tags them with the child's PD pointer, so the
///     child's `Pterm` frees them; `false` (`PE_LOAD`) leaves them with the
///     caller (`current_owner`), which gets the PD pointer back and decides.
///
/// # Safety
///
/// Caller holds the BDOS giant lock; see [`load_to_slot`].
pub unsafe fn load_to_slot_with(
    elf_bytes: &[u8],
    slot: usize,
    tail: &[u8],
    env: EnvSpec<'_>,
    owned_by_child: bool,
) -> Result<LoadedImage, LoadError> {
    // SAFETY: forwarded — same BDOS-lock / trap-context contract.
    unsafe {
        load_from_source(ImageSource::Memory(elf_bytes), slot, tail, env, owned_by_child)
    }
}

/// Core loader shared by the in-memory ([`load_to_slot_with`]) and
/// FAT-file ([`load_file_to_slot`]) entry points. Parses the header,
/// allocates + owns the TPA, and streams the segments into it.
///
/// # Safety
///
/// Caller holds the BDOS giant lock; the slot must not currently be the
/// active TTBR0 (or the TLB must be flushed before the next user access).
unsafe fn load_from_source(
    src: ImageSource<'_>,
    slot: usize,
    tail: &[u8],
    env: EnvSpec<'_>,
    owned_by_child: bool,
) -> Result<LoadedImage, LoadError> {
    if slot >= super::MAX_PROCESSES {
        return Err(LoadError::InvalidSlot);
    }

    // 1. Parse the header (before touching the pool, so a malformed image
    //    costs no allocation). An in-memory blob is its own header window;
    //    a streamed file reads its header + phdrs into `HEADER_BUF` (never
    //    a segment payload — those go straight into the TPA in step 3).
    let img: ElfImage = match &src {
        ImageSource::Memory(blob) => elf::parse(blob, blob.len() as u64)?,
        ImageSource::File(file) => {
            // SAFETY: single BDOS-locked trap path; the buffer is not
            // aliased while the returned borrow is live.
            let hdr = unsafe { &mut *HEADER_BUF.0.get() };
            let n = file
                .read_at(0, hdr)
                .ok_or(LoadError::Parse(ParseError::TooShort))?;
            elf::parse(&hdr[..n], file.len() as u64)?
        }
    };

    // 1b. Refuse ILP32 (ELFCLASS32) images. NewTOS is a native 64-bit
    //     system: LP64 is the only application ABI that can be built
    //     (ADR-020 addendum, 2026-09-04 — the `*-ilp32.sh` build scripts
    //     and `crt0-ilp32.S` are gone). The *runtime* width switch
    //     (`Abi::Ilp32`, read in the AES/VDI parameter-block readers)
    //     deliberately stays as the substrate for the future m68k thunk,
    //     which will set it for an emulated process that never comes
    //     through this ELF loader at all. Until then this is the single
    //     place that could mark a process ILP32, so gating it here makes
    //     every downstream 4-byte branch unreachable rather than
    //     silently untested. Lifting the thunk in is a one-line change
    //     to this gate.
    //
    //     Placed before step 2 on purpose: a refused image costs no pool
    //     allocation, exactly like a malformed one.
    if img.is_ilp32() {
        return Err(LoadError::Ilp32Unsupported);
    }

    // 1b'. Where the image ends (BSS included): what the TPA must at least
    //     hold — EmuTOS `needed = tlen + dlen + blen + sizeof(PD)`
    //     (`bdos/proc.c:291`); in NewTOS the image starts at `B + 0x1000`, so
    //     it is the highest segment end. A malformed segment costs no
    //     allocation either.
    let image_end = image_end_of(&img)?;

    // 1c. The program's `PRGFLAGS` (ARCH03 E8b): the NewTOS ELF note, read
    //     before anything is allocated because the flags choose the pool of
    //     the TPA and the environment (EmuTOS `alloc_env`/`alloc_tpa` take
    //     `hdr.h01_flags`, `bdos/proc.c:283,292`). No note → `PF_STANDARD`.
    let prgflags = read_prgflags(&src, &img)?.unwrap_or(PF_STANDARD);

    // 2. The environment first, then the TPA — EmuTOS `xexec`: "allocate
    //    the environment first, depending on memory policy" (`alloc_env`,
    //    `bdos/proc.c:281-287`), then the basepage/TPA (`alloc_tpa`,
    //    `:289-299`, which frees the environment again if it fails). The
    //    order fixes the layout: the environment lies below the TPA, as in
    //    EmuTOS (ARCH03 3c; until then the loader took the TPA first).
    //    Both are allocated as the caller (`owner = run`) and handed to the
    //    child below for `PE_LOADGO` (`set_owner`, `:302-307`).
    // SAFETY: BDOS lock held.
    let env_addr = unsafe { alloc_env_for(env, prgflags) }?;
    let free_env = || {
        if let Some(e) = env_addr {
            // SAFETY: BDOS lock held; `e` is the block just allocated.
            let _ = unsafe { crate::bdos::mem::traps::mfree(e) };
        }
    };

    // 3. Take the TPA block from the BDOS pool. Its address is the child's
    //    PD pointer. `Mxalloc` stamped the caller as owner (`owner = run`);
    //    for `PE_LOADGO` the block goes to the child (`owner = p`,
    //    EmuTOS `bdos/proc.c:302-307`), so the child's `Pterm`
    //    `free_all_owned` sweep reclaims it.
    // SAFETY: BDOS lock held; single-CPU pre-test.
    let Some(tpa) = (unsafe { arena::alloc_tpa(prgflags, image_end) }) else {
        // EmuTOS: "if failed, free env_ptr and return" (`bdos/proc.c:294-299`).
        free_env();
        return Err(LoadError::NoTpaMemory);
    };
    // The TPA takes the program's protection mode (ARCH03 3b, FreeMiNT
    // `load_region`: `alloc_region(map, len, protmode)` with the PRGFLAGS
    // mode, `sys/memory.c:1511-1513`).
    // SAFETY: BDOS lock held; `tpa.base` starts the block just allocated.
    unsafe { super::memprot::set_mode(tpa.base, prot_from_prgflags(prgflags)) };
    if owned_by_child {
        // SAFETY: both blocks are on the allocated list; the owner cookie is
        // a pointer-equality tag, never dereferenced by the sweep.
        unsafe {
            crate::bdos::mem::traps::set_owner(tpa.base as Long, tpa.base as *mut c_void);
            if let Some(e) = env_addr {
                crate::bdos::mem::traps::set_owner(e, tpa.base as *mut c_void);
            }
        }
    }

    // 4. Populate + build the root; free the TPA and the environment on any
    //    failure — no `Pterm` runs for a load that never launched, so we must
    //    not leak the blocks.
    // SAFETY: BDOS lock held; `tpa` is a fresh, exclusively-owned
    // page-aligned region.
    match unsafe {
        let req = ProcessSetup { tail, env_addr, prgflags, image_end };
        populate_and_map(&src, slot, &img, &req, &tpa)
    } {
        Ok(loaded) => {
            // "the program has been correctly loaded in memory, and more I/O
            // errors cannot occur, so it is safe now to finish initializing
            // the new process" (EmuTOS `xexec`, `bdos/proc.c:345-350`).
            // SAFETY: BDOS lock held; the basepage is live at its alias.
            unsafe { init_pd_files(tpa.base_kva as *mut BasePage) };
            Ok(loaded)
        }
        Err(e) => {
            // SAFETY: BDOS lock held; `tpa.base` is still allocated.
            unsafe { arena::free_tpa(tpa.base as Long) };
            free_env();
            Err(e)
        }
    }
}

/// What a load gives the new process besides its image: the command tail,
/// the environment block already allocated for it (EmuTOS order: before the
/// TPA), and its `PRGFLAGS` (see [`load_to_slot_with`]).
#[derive(Copy, Clone)]
struct ProcessSetup<'a> {
    tail: &'a [u8],
    env_addr: Option<Long>,
    prgflags: u32,
    /// Bytes from the PD to the image end ([`image_end_of`]).
    image_end: u64,
}

/// Stream the ELF image into `tpa`, zero its BSS, write the basepage
/// (with the environment block the caller allocated) and the slot's page 0,
/// sync the code caches, build the slot's root and publish the slot metadata.
///
/// Factored out of [`load_from_source`] so the caller can free the TPA
/// on any error (the `?`/early-return points all propagate here).
///
/// # Safety
///
/// Caller holds the BDOS giant lock; `tpa` is a fresh page-aligned
/// pool region; `slot` is bounds-checked.
unsafe fn populate_and_map(
    src: &ImageSource<'_>,
    slot: usize,
    img: &ElfImage,
    req: &ProcessSetup<'_>,
    tpa: &arena::Tpa,
) -> Result<LoadedImage, LoadError> {
    let ProcessSetup { tail, env_addr, prgflags, image_end } = *req;
    let base = tpa.base;

    // Whole-TPA mutable byte view via the kernel high-half alias.
    // SAFETY: the pool block backs `tpa.len` contiguous bytes at
    // `base_kva`, mapped RW in the high half; single-CPU under the lock.
    let tpa_bytes = unsafe {
        core::slice::from_raw_parts_mut(tpa.base_kva, tpa.len as usize)
    };

    // Zero as EmuTOS `kpgmld` (`bdos/kpgmld.c:212-221`, "clear the bss or the
    // whole heap"): the basepage page and the whole image span — so the BSS,
    // and the padding between segments, start zeroed — always; the free TPA
    // above the image only without `PF_FASTLOAD`.
    tpa_bytes[..image_end as usize].fill(0);
    if prgflags & PF_FASTLOAD == 0 {
        tpa_bytes[image_end as usize..].fill(0);
    }

    // Stream each PT_LOAD segment's file bytes directly into the TPA at
    // its link VA (the byte offset in the TPA) — for a FAT file, straight
    // from disk with no fixed intermediate buffer. Track the highest image
    // byte so the cache maintenance below covers exactly text+data.
    let mut image_top = BASEPAGE_RESERVED;
    for seg in img.segments() {
        let dest_off = seg.vaddr as usize;
        let file_size = seg.file_size as usize;

        if file_size > 0 {
            let dst = &mut tpa_bytes[dest_off..dest_off + file_size];
            if !src.read_exact_at(seg.file_offset, dst) {
                return Err(LoadError::Parse(ParseError::SegmentOutOfRange));
            }
        }
        // The BSS tail (`mem_size - file_size`) is already zero from the
        // image-span fill above, so no per-segment re-zero is needed.
        image_top = image_top.max(seg.vaddr + seg.mem_size);
    }

    // Apply the image's `R_AARCH64_RELATIVE` relocations ([`super::reloc`]):
    // every absolute data word the PIE carries becomes `B + addend`. The
    // view is indexed by link VA and the TPA executes at `B`, so `B` is the
    // execution address of link VA 0 — the load base. Done before anything
    // else is written into the TPA and before the cache maintenance below,
    // so the I-cache sync sees the final image. A refused relocation fails
    // the load (`EPLFMT`); the caller frees the TPA, so a half-relocated
    // image never runs.
    reloc::relocate(tpa_bytes, img, base)?;

    // Write the BasePage at the TPA start.
    let hitpa = base + tpa.len;
    populate_basepage(tpa_bytes, img, tail, env_addr, base, hitpa, prgflags);

    // Make the freshly-written program image observable to instruction
    // fetch on real hardware (clean D-cache to PoU + invalidate I-cache).
    // QEMU has no caches, so this is a no-op there. Cortex-A72 caches are
    // PIPT, so by-VA maintenance via the kernel alias hits the same PA the
    // program later fetches.
    // SAFETY: the range is live and mapped in the kernel's high half.
    unsafe { mmu::sync_code_cache(tpa.base_kva as usize, image_top as usize) };

    // Record the program ABI so the GEM trap dispatch reads this
    // process's AES/VDI parameter blocks at the caller's pointer width
    // (ADR-020). Always LP64: step 1b above rejected every ELFCLASS32
    // image, so an ELF-loaded process is 8-byte-word by construction.
    // This is not a hardcoded assumption — it is that gate's postcondition,
    // asserted here so the two cannot drift apart.
    debug_assert!(!img.is_ilp32(), "step 1b must reject ELFCLASS32");

    let entry_va = base + img.entry_va;
    let sp_el0 = hitpa & !0xF;
    // SAFETY: BDOS lock held; the slot belongs to this load.
    let l0_pa = unsafe { install_slot(slot, tpa, entry_va, sp_el0)? };

    Ok(LoadedImage {
        slot,
        entry_va,
        l0_pa,
        basepage_va: base,
        sp_el0,
        image_len: image_end,
    })
}

/// Give `slot` the process around the basepage at `tpa`: its private page 0,
/// its root, and the slot metadata `Pterm` and the trap handler find it by.
/// Returns the root's physical address (`TTBR0_EL1`). `entry_va` 0 marks a
/// basepage without a loaded image (`Pexec` 5/7), started at its `p_tbase`
/// ([`super::pexec::go`]).
///
/// # Safety
///
/// Caller holds the BDOS giant lock; `slot` is claimed for this process and
/// bounds-checked; `tpa` holds the process's basepage.
unsafe fn install_slot(
    slot: usize,
    tpa: &arena::Tpa,
    entry_va: u64,
    sp_el0: u64,
) -> Result<u64, LoadError> {
    // The process's private page 0: the classic TOS low-memory system area
    // (sysvars + OSHEADER + cookie jar + the `os_run` cell), the upcall and
    // `G_USERDEF` trampolines and the KEYS02 vector table (B1 / Option 1,
    // plan E3), and the RST01 reset stub. A 1:1-ported TOS app reads `_sysbase`@0x4F2 /
    // `_p_cookies`@0x5A0 / the OSHEADER directly (e.g. TeraDesk
    // `main.c:56`), so these must be a real, populated instance in the
    // app's own address space.
    // SAFETY: BDOS lock held; the slot's page 0 belongs to this process.
    let page0 = unsafe { mmu::page0_bytes(slot) }.ok_or(LoadError::InvalidSlot)?;
    page0.fill(0);
    populate_sysvars(page0, tpa.base);
    // The private read-write page (MEM12): the KEYS02 table with its
    // defaults; the PARMBLK scratch starts zeroed.
    // SAFETY: BDOS lock held; the slot's private page belongs to this process.
    let private = unsafe { mmu::priv_page_bytes(slot) }.ok_or(LoadError::InvalidSlot)?;
    private.fill(0);
    crate::xbios::kbdvecs::write_table(private);
    // The shared stub page: written once, the same code for every process.
    // SAFETY: BDOS lock held.
    unsafe { ensure_stub_page() };

    // Build the slot's root: the shared tables (pools, publications) plus
    // its own page 0. Nothing is mapped per TPA page any more — the pool
    // the TPA lives in is in every root from boot on (VA = PA).
    // SAFETY: slot bounds-checked; single-CPU under the lock.
    unsafe { mmu::init_slot_tables(slot)? };

    // A new tenant holds no uncached screen pointer yet.
    usermem::clear_uncached(slot);

    // Publish the slot metadata so `Pterm` (via `slot_for(run)`) and the
    // trap handler can find this process.
    let l0_pa = mmu::l0_pa_for(slot);
    let proc_slot = &PROCESS_TABLE[slot];
    proc_slot
        .basepage_kva
        .store(tpa.base_kva as *mut BasePage, Ordering::Release);
    proc_slot.basepage_pa.store(tpa.base, Ordering::Release);
    proc_slot.l0_pa.store(l0_pa, Ordering::Release);
    proc_slot.entry_va.store(entry_va, Ordering::Release);
    proc_slot.sp_el0.store(sp_el0, Ordering::Release);
    proc_slot.set_abi(super::Abi::Lp64);
    // The new root gets its rights (ARCH03 3b; FreeMiNT `init_page_table`,
    // `sys/arch/mprot030.c:825-1062`).
    // SAFETY: BDOS lock held; the slot's root is built and published.
    unsafe { super::memprot::on_root_built(slot) };
    Ok(l0_pa)
}

/// `Pexec(PE_BASEPAGE / PE_BASEPAGEFLAGS)` — a basepage without a program
/// (EmuTOS `xexec`, `bdos/proc.c:210-237`): the environment
/// (`alloc_env(flags, env)`), the TPA — the largest free block of the pool
/// `flags` select (`alloc_tpa((ULONG)path, sizeof(PD), &max)`, `:220`) — the
/// basepage at its start per `init_pd_fields`
/// (`:364-385`: zeroed, `p_lowtpa`, `p_hitpa`, `p_xdta`, `p_env`, the command
/// tail) with `p_flags = flags`, and `slot` set up around it. Both blocks
/// belong to the caller (the allocator stamps `run`; `set_owner(p, run)`,
/// `:229-230`). Only the basepage itself is written. The slot's `entry_va` is
/// 0: `Pexec` 4/6 start it at `p_tbase` ([`super::pexec::go`]).
///
/// On failure nothing stays allocated (`:216-226`).
///
/// # Safety
///
/// Caller holds the BDOS giant lock; `slot` is freshly claimed.
pub unsafe fn create_basepage(
    slot: usize,
    flags: u32,
    tail: &[u8],
    env: EnvSpec<'_>,
) -> Result<LoadedImage, LoadError> {
    if slot >= super::MAX_PROCESSES {
        return Err(LoadError::InvalidSlot);
    }
    // EmuTOS order: the environment first, then the TPA.
    // SAFETY: BDOS lock held.
    let env_addr = unsafe { alloc_env_for(env, flags)? };
    let free_env = || {
        if let Some(e) = env_addr {
            // SAFETY: BDOS lock held; `e` is the block just allocated.
            let _ = unsafe { crate::bdos::mem::traps::mfree(e) };
        }
    };
    // SAFETY: BDOS lock held.
    let Some(tpa) = (unsafe { arena::alloc_tpa(flags, BASEPAGE_RESERVED) }) else {
        free_env();
        return Err(LoadError::NoTpaMemory);
    };
    // The basepage block takes the mode its flags ask for (ARCH03 3b, as a
    // loaded program's TPA).
    // SAFETY: BDOS lock held; `tpa.base` starts the block just allocated.
    unsafe { super::memprot::set_mode(tpa.base, prot_from_prgflags(flags)) };
    // `init_pd_fields`: the basepage, and only the basepage.
    // SAFETY: the block backs at least `BASEPAGE_RESERVED` bytes at its
    // kernel alias, exclusively ours under the lock.
    let bp_bytes =
        unsafe { core::slice::from_raw_parts_mut(tpa.base_kva, BASEPAGE_RESERVED as usize) };
    bp_bytes.fill(0);
    // SAFETY: `bp_bytes` is exactly one page-aligned `BasePage`.
    let bp = unsafe { &mut *(bp_bytes.as_mut_ptr() as *mut BasePage) };
    bp.p_lowtpa = tpa.base as Long;
    bp.p_hitpa = (tpa.base + tpa.len) as Long;
    bp.p_xdta = (tpa.base + core::mem::offset_of!(BasePage, p_cmdlin) as u64) as Long;
    bp.p_env = env_addr.unwrap_or(0);
    bp.p_flags = flags;
    // The standard handles are `init_pd_files`'s, below, once the slot exists.
    let n = core::cmp::min(tail.len(), PDCLSIZE - 1);
    bp.p_cmdlin[..n].copy_from_slice(&tail[..n]);
    bp.p_cmdlin[n] = 0;

    // SAFETY: BDOS lock held; `slot` is ours.
    match unsafe { install_slot(slot, &tpa, 0, 0) } {
        Ok(l0_pa) => {
            // `init_pd_files(p)` (`bdos/proc.c:235`), after the PD is complete.
            // SAFETY: BDOS lock held; the basepage is live at its alias.
            unsafe { init_pd_files(tpa.base_kva as *mut BasePage) };
            Ok(LoadedImage {
                slot,
                entry_va: 0,
                l0_pa,
                basepage_va: tpa.base,
                sp_el0: 0,
                image_len: BASEPAGE_RESERVED,
            })
        }
        Err(e) => {
            // SAFETY: BDOS lock held; both blocks are still ours.
            unsafe { arena::free_tpa(tpa.base as Long) };
            free_env();
            Err(e)
        }
    }
}

/// The standard-handle half of EmuTOS `init_pd_files` (`bdos/proc.c:387-399`)
/// for the new PD `p` (kernel alias): "inherit standard files from me" —
/// each standard handle of `run` that is `> 0` (redirected to a file) is
/// [`ixforce`](crate::bdos::fs::file::ixforce)d into `p`, so the child shares
/// the file and its entry counts the use; every other value is copied. `run`
/// is the running process, or with none the kernel's stand-in PD
/// ([`super::standin::kernel_run_uft`], TASK/PPARENT). Runs once the PD is
/// complete, so a load that fails takes no use (`bdos/proc.c:235,345-350`).
///
/// The rest of `init_pd_files` — `p_curdir[]` with one `dirtbl` use each, then
/// `p_curdrv` (`:401-409`) — from the same `run`
/// ([`crate::bdos::fs::curdir::inherit_from_run`], TASK/PROC04 point 2): the
/// child starts in its creator's drive and directories, and its own
/// `Dsetdrv`/`Dsetpath` change only its PD.
///
/// # Safety
///
/// Caller holds the BDOS giant lock; `p` is the kernel alias of a live
/// basepage that is not the running one.
pub unsafe fn init_pd_files(p: *mut BasePage) {
    let run = crate::bdos::process::current_kva();
    let from = if run.is_null() {
        super::standin::kernel_run_uft()
    } else {
        // SAFETY: the running process's live basepage.
        unsafe { (*run).p_uft }
    };
    for (i, &h) in from.iter().enumerate() {
        if h > 0 {
            // SAFETY: caller contract — `p` is a live basepage.
            let _ = unsafe { crate::bdos::fs::file::ixforce(i as i16, h as i16, p) };
        } else {
            // SAFETY: as above.
            unsafe { (*p).p_uft[i] = h };
        }
    }
    // SAFETY: caller contract — `p` is a live basepage that is not `run`'s.
    unsafe { crate::bdos::fs::curdir::inherit_from_run(p) };
}

/// Write the [`BasePage`] into the reserved window at the start of the
/// TPA (`tpa[0..sizeof(BasePage)]`). `base` is the TPA's address — the PD
/// pointer — and every address field is relative to it (EmuTOS
/// `init_pd_fields`, `bdos/proc.c:363-384`, and `kpgmld.c:123-145`).
///
/// Sources:
///   * `p_lowtpa` / `p_hitpa` — TPA extents: `[base, hitpa)` (`p_hitpa = p +
///     max`, `bdos/proc.c:374`, `max` the largest block `alloc_tpa` took).
///     `p_hitpa` is the TPA top, i.e. the initial stack pointer; the heap
///     lives between the BSS end (`p_bbase + p_blen`) and the
///     downward-growing stack.
///   * `p_tbase` / `p_tlen` — first executable `PT_LOAD` segment.
///   * `p_dbase` / `p_dlen` — first non-executable, writable
///     `PT_LOAD` segment, file portion only.
///   * `p_bbase` / `p_blen` — same segment's BSS tail
///     (`mem_size - file_size`).
///   * `p_xdta` — points at `p_cmdlin` per Atari convention; the
///     program may override via `Fsetdta`.
///   * `p_flags` — the program's `PRGFLAGS` (its ELF note, else
///     [`PF_STANDARD`]; EmuTOS `p->p_flags = hdr.h01_flags`,
///     `bdos/proc.c:313`).
///   * `p_uft` — zero here (EmuTOS `bzero`, `bdos/proc.c:370`); the
///     caller's handles are inherited by [`init_pd_files`] once the load has
///     succeeded.
///
/// `p_cmdlin` receives the caller's command `tail` (verbatim, truncated
/// to 127 bytes, NUL-terminated — EmuTOS `init_pd_fields`
/// `bdos/proc.c:378-383`); `p_env` receives the address of the
/// copied environment block (`env_addr`), or `0` when none was
/// allocated. `p_parent` is set when the process is started (EmuTOS
/// `proc_go`, `bdos/proc.c:541` — `pexec::set_parent`); `p_curdir`,
/// `p_curdrv` (filled by [`init_pd_files`]) and the saved-register cells are
/// left zero.
fn populate_basepage(
    tpa: &mut [u8],
    img: &ElfImage,
    tail: &[u8],
    env_addr: Option<Long>,
    base: u64,
    hitpa: u64,
    prgflags: u32,
) {
    let mut text_base: u64 = 0;
    let mut text_len: u64 = 0;
    let mut data_base: u64 = 0;
    let mut data_len: u64 = 0;
    let mut bss_base: u64 = 0;
    let mut bss_len: u64 = 0;
    let (mut have_text, mut have_data) = (false, false);

    for seg in img.segments() {
        let is_x = (seg.flags & PF_X) != 0;
        let is_w = (seg.flags & PF_W) != 0;
        if is_x && !have_text {
            have_text = true;
            text_base = base + seg.vaddr;
            text_len = seg.file_size;
        } else if is_w && !is_x && !have_data {
            have_data = true;
            data_base = base + seg.vaddr;
            data_len = seg.file_size;
            bss_base = base + seg.vaddr + seg.file_size;
            bss_len = seg.mem_size - seg.file_size;
        }
    }

    // Carve the basepage window out of the TPA. Splitting first gives
    // us a non-aliased mutable handle on exactly the 256 bytes the
    // BasePage owns.
    let (bp_bytes, _rest) = tpa.split_at_mut(BASEPAGE_RESERVED as usize);

    // SAFETY: `bp_bytes` is exactly `size_of::<BasePage>()` bytes,
    // page-aligned (and therefore 4-aligned, satisfying
    // `align_of::<BasePage>() == 4`), and we hold the only mutable
    // borrow over it for the lifetime of `bp`.
    let bp = unsafe { &mut *(bp_bytes.as_mut_ptr() as *mut BasePage) };

    bp.p_lowtpa = base as Long;
    bp.p_hitpa = hitpa as Long;
    bp.p_tbase = text_base as Long;
    bp.p_tlen = text_len as Long;
    bp.p_dbase = data_base as Long;
    bp.p_dlen = data_len as Long;
    bp.p_bbase = bss_base as Long;
    bp.p_blen = bss_len as Long;
    bp.p_xdta = (base + core::mem::offset_of!(BasePage, p_cmdlin) as u64) as Long;
    bp.p_parent = 0;
    bp.p_flags = prgflags;
    bp.p_env = env_addr.unwrap_or(0);
    // `p_uft` stays as `init_pd_fields`'s `bzero` left it until
    // `init_pd_files` fills it (a handle `ixforce` refuses keeps the 0).
    bp.p_uft = [0; NUMSTD];
    bp.p_lddrv = 0;
    bp.p_curdrv = 0;

    // Command tail → `p_cmdlin` (offset 0x80). EmuTOS `init_pd_fields`
    // copies the caller's `tail` byte-for-byte up to `PDCLSIZE` or the
    // first NUL, then appends a NUL. We cap the copy at `PDCLSIZE - 1`
    // (127) so the terminating NUL always fits inside the 128-byte
    // buffer (the spec's "127-byte usable region + trailing NUL"); the
    // caller's `tail` carries no embedded NUL (it was read up to the
    // first one), so this is the same observable image. Under the Atari
    // convention `tail[0]` is the length byte, preserved verbatim.
    let n = core::cmp::min(tail.len(), PDCLSIZE - 1);
    bp.p_cmdlin[..n].copy_from_slice(&tail[..n]);
    bp.p_cmdlin[n] = 0;
}

// ── Classic low-memory system area (B1 / Option 1) ──────────────────────
//
// Per-process EL0-VA layout of the reserved low region the loader populates
// so a 1:1-ported TOS app can dereference the classic absolute addresses
// (`_sysbase`@0x4F2, `_p_cookies`@0x5A0, the OSHEADER). Every value/behaviour
// replicates EmuTOS 1:1 (`references/emutos/...`), cited at each write.
// Spec: `docs/analysis/bios-audit/b1-lowmem-feasibility.md` §Option 1,
// `docs/api/bios/_boot-init.md`, `docs/api/structures/osheader.md`.

/// OSHEADER instance base. `sysbase`@0x4F2 points here. The struct is 48
/// bytes (`OSHEADER` = 0x30, `docs/api/structures/osheader.md`), so it spans
/// `[0x5C0, 0x5F0)`, ending exactly at the cookie jar.
const OSHEADER_VA: u32 = 0x5C0;

/// Cookie-jar base. `p_cookies`@0x5A0 points here.
const COOKIE_JAR_VA: u32 = 0x5F0;

/// Cookie-jar capacity in `{tag, value}` slots (8 bytes each). The region
/// `[0x5F0, 0x670)` holds 16 slots and stays below [`RESERVED_LOW_END`].
///
/// This is the **mirror's** capacity and is smaller than the kernel jar's
/// (`cookie::capacity()` == 32): 16 slots carry 15 cookies plus the classic
/// `{0, capacity}` terminator. [`write_cookie_jar`] truncates a longer kernel
/// jar rather than overrunning the window, and always emits the terminator —
/// see its doc comment for why that is the defined answer.
const COOKIE_JAR_SLOTS: u32 = 16;

// The jar must hold at least the boot-seeded cookie set plus the terminator,
// so the mirror a freshly-loaded process reads is never truncated.
const _: () = assert!(COOKIE_JAR_SLOTS >= 6); // NTOS _MCH _IDT _AKP EdDI + {0, n}

/// Exclusive end of the classic low-memory system area (= jar end, 0x670).
const SYSVAR_AREA_END: u64 = COOKIE_JAR_VA as u64 + (COOKIE_JAR_SLOTS as u64) * 8;

/// Page-0 VA of the **`os_run` cell** — the "current PD" variable OSHEADER
/// `os_run` (+0x28) points at (EmuTOS `_run`, `bios/startup.S:110-111`).
/// Plan Frage 12: a cell in the free part of page 0 below the OSHEADER;
/// `0x5B8` is the first 8-byte-aligned address after the last documented
/// system variable (`_kcl_hook` `0x5B0`, `references/emutos/tosvars.ld`,
/// which reserves the space up to the OSHEADER for "undocumented variables").
/// Written as 8 bytes (the PD pointer, upper half 0), so a 4-byte and an
/// 8-byte read both yield it.
pub const OS_RUN_CELL_VA: u64 = 0x5B8;

const _: () = assert!(OS_RUN_CELL_VA >= 0x5B4 && OS_RUN_CELL_VA + 8 <= OSHEADER_VA as u64);

// ── The stub page and the private read-write page (MEM12) ───────────────
//
// Page 0 holds only what TOS fixes at low addresses (sysvars, OSHEADER,
// `os_run` cell, cookie jar) and the `Keytbl` area; it is EL0 read-only in
// user state and never executable (`docs/api/_super-state.md` S2). The EL0
// code that used to sit in its gap lives in the one shared **stub page**
// (`mmu::STUB_PAGE_VA`, EL0 execute-only), and what a program may write in
// the per-slot **private read-write page** (`mmu::PRIV_PAGE_VA`). All VAs
// stay below 4 GiB: the reset stub's address goes into 32-bit OSHEADER
// fields.

/// EL0 VA of the upcall trampoline (cooperative-kernel building block 1,
/// `docs/analysis/coop-kernel/10-upcall-signal-design.md` §4.1): the first
/// stub of the stub page. The kernel-side model of FreeMiNT's per-process
/// `user_things` trampoline (`user_things.S`), at a fixed VA rather than via
/// `tp_ptr`.
pub const TRAMPOLINE_VA: u64 = mmu::STUB_PAGE_VA;

/// EL0 VA of the **AES `G_USERDEF` callout stub** (AES-as-thread design,
/// `docs/analysis/coop-kernel/30-aes-thread-design.md` §4), in the stub page.
/// The kernel delivers this VA as an upcall handler with `x0 = ub_code`,
/// `x1 = parmblk_ptr`; the stub calls `ub_code(parmblk_ptr)` at EL0
/// (user→user, no PXN) and issues `Psigreturn` with `ub_code`'s WORD result
/// still in `w0`, which the kernel reads from the trap frame (MEM12: no
/// result slot in page 0 any more).
pub const USERDEF_TRAMPOLINE_VA: u64 = mmu::STUB_PAGE_VA + 0x20;

/// EL0 VA of the **PARMBLK scratch region** the kernel writes before an AES
/// `G_USERDEF` callout and the app's `ub_code` reads (design §4) — and may
/// write, as on EmuTOS, where the `PARMBLK` lies on the AES's writable stack
/// (WinDom's `xedit` changes `pb_x`/`pb_w`, MEM12). In the private read-write
/// page. ABI-wide: a LP64 `PARMBLK` is 40 bytes, an ILP32 one 32 bytes.
pub const PARMBLK_SCRATCH_VA: u64 = mmu::PRIV_PAGE_VA + 0x100;

/// EL0 VA of the **`Keytbl` area**: the `struct keytbl` XBIOS `Keytbl` (0x10)
/// returns, followed by the three 128-byte scancode→ASCII tables and the three
/// AltGr override arrays it points at. In page 0, above the classic system
/// area. Written through the kernel alias (`page0_kva(slot) + offset`) when
/// `Keytbl` is called by a running EL0 process, and read back by that process
/// at this low-half VA — EmuTOS' desktop dereferences the returned pointer
/// directly (`desk/deskmain.c:1144-1145`). See `crate::xbios::keytbl` for the
/// layout inside the area.
pub const KEYTBL_AREA_VA: u64 = 0x768;

/// Size of the [`KEYTBL_AREA_VA`] region, in bytes.
pub const KEYTBL_AREA_SIZE: u64 = 0x238;

const _: () = assert!(KEYTBL_AREA_VA >= SYSVAR_AREA_END);
const _: () = assert!(KEYTBL_AREA_VA + KEYTBL_AREA_SIZE <= RESERVED_LOW_END);

/// EL0 VA of the per-process **IKBD vector table** (KEYS02, spec
/// `docs/api/xbios/22-kbdvbase.md` B1, location D1-neu of Teil D): `mousexvec`,
/// `kbdvec`, the nine `struct kbdvecs` slots and the `drvstat`/`kbdlength`
/// bytes, in EmuTOS order (`bios/aciavecs.S:126-141`) with 8-byte slots. At
/// the start of the private read-write page — a program installs its hook by
/// writing a slot. Written at load by
/// [`crate::xbios::kbdvecs::write_table`]; `Kbdvbase` returns
/// `KBDVECS_AREA_VA + 0x10` (the `midivec` slot), so `kbdvec` sits at `-8`.
pub const KBDVECS_AREA_VA: u64 = mmu::PRIV_PAGE_VA;

/// Size of the [`KBDVECS_AREA_VA`] table region, in bytes (0x5A used).
pub const KBDVECS_AREA_SIZE: u64 = 0x60;

/// EL0 VA of the **IKBD default stubs** (spec B3): the `ret` stub every
/// unhooked slot points at (EmuTOS `just_rts`) and the `kbdvec_default` stub
/// (EmuTOS `_kbdvec` → `kbd_int`). In the stub page.
pub const KBDVEC_STUB_AREA_VA: u64 = mmu::STUB_PAGE_VA + 0x40;

/// Size of the [`KBDVEC_STUB_AREA_VA`] region, in bytes.
pub const KBDVEC_STUB_AREA_SIZE: u64 = 0x40;

/// EL0 VA of the **reset stub** (TASK/RST01): the code OSHEADER `reseth`
/// (+0x04) and `os_res1` (+0x10) point at — EmuTOS stores `_main` in both
/// (`bios/startup.S:82-83,92-93`). It calls `Ssystem(S_NEWTOS_RESET)`; see
/// [`crate::bios::reset`]. In the stub page.
pub const RESET_STUB_AREA_VA: u64 = mmu::STUB_PAGE_VA + 0x80;

/// Size of the [`RESET_STUB_AREA_VA`] region, in bytes.
pub const RESET_STUB_AREA_SIZE: u64 = 0x20;

// The stubs follow one another inside the stub page; the table and the
// PARMBLK (LP64: 40 bytes) inside the private page, apart.
const _: () = assert!(USERDEF_TRAMPOLINE_VA >= TRAMPOLINE_VA + 4 * TRAMPOLINE_CODE.len() as u64);
const _: () = assert!(KBDVEC_STUB_AREA_VA >= USERDEF_TRAMPOLINE_VA + 4 * USERDEF_STUB_CODE.len() as u64);
const _: () = assert!(RESET_STUB_AREA_VA >= KBDVEC_STUB_AREA_VA + KBDVEC_STUB_AREA_SIZE);
const _: () = assert!(RESET_STUB_AREA_VA + RESET_STUB_AREA_SIZE <= mmu::STUB_PAGE_VA + mmu::PAGE_SIZE);
const _: () = assert!(KBDVECS_AREA_VA.is_multiple_of(8));
const _: () = assert!(PARMBLK_SCRATCH_VA >= KBDVECS_AREA_VA + KBDVECS_AREA_SIZE);
const _: () = assert!(PARMBLK_SCRATCH_VA + 40 <= mmu::PRIV_PAGE_VA + mmu::PAGE_SIZE);

/// OSHEADER `os_version` (+0x02): Falcon TOS 4.04. EmuTOS' own default is
/// 0x206 (`config.h:1367-1372`); NewTOS reports 4.04 — user-pinned.
const OS_VERSION: u16 = 0x0404;
/// OSHEADER `os_conf` (+0x1C): `(country << 1) | PAL` (`osheader.md`
/// §os_conf). NewTOS is US/English (`i18n::current_lang == En`, cookie
/// `_AKP == US`) and ignores PAL → country US(0), PAL 0 → 0.
const OS_CONF: u16 = 0x0000;
/// OSHEADER `os_date` (+0x18): OS build date, BCD `MMDDYYYY` — the real build
/// date since DESK12, from [`crate::build`], which is where EmuTOS gets it too
/// (`tools/mkheader.awk` generates `obj/header.h`'s `OS_DATE`). It was a
/// 1980-01-01 placeholder until a build stamp existed.
const OS_DATE: u32 = crate::build::OS_DATE;
/// OSHEADER `os_dosdate` (+0x1E): the same date, GEMDOS-packed
/// (`(year-1980)<<9 | month<<5 | day`) — `obj/header.h`'s `OS_DOSDATE`.
const OS_DOSDATE: u16 = crate::build::OS_DOSDATE;

/// RAM-config validity magics (`references/emutos/bios/memory.S`), written on
/// cold boot at the end of `memconf` (memory.S:588-590).
const MEMVALID_MAGIC: u32 = 0x7520_19F3; // $420 memvalid (memory.S:126,588)
const MEMVAL2_MAGIC: u32 = 0x2376_98AA; //  $43A memval2  (memory.S:129,589)
const MEMVAL3_MAGIC: u32 = 0x5555_AAAA; //  $51A memval3  (memory.S:132,590)

/// `ramvalid` ($5A8) magic: validates `ramtop` ($5A4)
/// (`references/emutos/include/tosvars.h:78` — `RAMVALID_MAGIC 0x1357BD13`).
const RAMVALID_MAGIC: u32 = 0x1357_BD13;

/// `conterm` ($484, BYTE) boot default: keyclick + key-repeat + system-bell
/// enabled (`references/emutos/bios/ikbd.c:1100` — `conterm = 7`). Bit 3
/// (`& 8`) requests the `Bconin` keyboard-shift-state fold
/// (`bios/ikbd.c:279-280`); the default leaves it clear, so `Bconin` keeps
/// returning bits 24-31 = 0 exactly as before this sysvar was populated.
pub(crate) const CONTERM_DEFAULT: u8 = 7;

/// Write a 32-bit little-endian `LONG` at byte offset `off` (any alignment;
/// the classic sysvars are 4-byte and not all 4-aligned, e.g. `sysbase`@0x4F2).
#[inline]
fn wr_long(buf: &mut [u8], off: usize, v: u32) {
    buf[off..off + 4].copy_from_slice(&v.to_le_bytes());
}
/// Write a 16-bit little-endian `WORD` at byte offset `off` (any alignment).
#[inline]
fn wr_word(buf: &mut [u8], off: usize, v: u16) {
    buf[off..off + 2].copy_from_slice(&v.to_le_bytes());
}

/// Populate the classic TOS low-memory system area in the process's page 0
/// (B1 / Option 1). `pd` is the process's PD pointer, for the `os_run` cell.
///
/// Writes, per-process and through the kernel high-half alias `tpa` (the
/// slot's page 0):
///   * an **OSHEADER instance** at [`OSHEADER_VA`]
///     (`docs/api/structures/osheader.md`),
///   * the **classic system variables** at their fixed `$400`-region
///     addresses (`docs/api/bios/_boot-init.md` §7, EmuTOS `tosvars.ld`),
///   * a **cookie jar** at [`COOKIE_JAR_VA`], mirroring the kernel jar
///     ([`crate::cookie`]) in the classic `{tag, value}` array form
///     (`docs/api/bios/_cookie-jar.md`) so `_p_cookies`@0x5A0 reaches a
///     well-formed jar.
///
/// `tpa` is the page-0 byte view. Called before the code-cache maintenance
/// in [`populate_and_map`] so its `dsb ish` makes the writes observable to
/// EL0 before the process runs.
fn populate_sysvars(tpa: &mut [u8], pd: u64) {
    // ── OSHEADER instance (osheader.md; EmuTOS bios/startup.S:73-114
    //    `_os_header`) ─────────────────────────────────────────────────
    let oh = OSHEADER_VA as usize;
    // os_entry (+0x00): EmuTOS `bra.s _main` (startup.S:78-79) — an m68k
    // instruction, two bytes, followed by `os_version`. An AArch64
    // instruction is four bytes, so the word here would be
    // `0x0404_xxxx` with `os_version` on top, which no AArch64 branch
    // encodes (SVE encoding space; `bios::reset` module doc). Left 0; the
    // jumpable reset entries are `reseth` and `os_res1` below.
    // os_version (+0x02):
    wr_word(tpa, oh + 0x02, OS_VERSION);
    // reseth (+0x04): EmuTOS `.dc.l _main` (startup.S:82-83), the full
    // restart. NewTOS: the reset stub (stub page), which ends in the restart
    // (`bios::reset`, `Ssystem(S_NEWTOS_RESET)`; TASK/RST01).
    wr_long(tpa, oh + 0x04, crate::bios::reset::RESET_STUB_VA as u32);
    // os_beg (+0x08): "base address of the operating system"
    // (`biosdefs.h:51`). EmuTOS stores the OS-image self-pointer
    // `_os_header` (startup.S:88-89), which is the very same address
    // `sysbase`@$4F2 holds — so an app's `hdr->os_beg == hdr` check holds.
    // NewTOS' OSHEADER instance lives at [`OSHEADER_VA`] in the process's
    // own EL0 VA, so that is the faithful analogue of `_os_header`.
    // os_end (+0x0C): "end of ST-RAM statically used by the OS"
    // (`biosdefs.h:52`; EmuTOS `__endvdibss`, startup.S:90-91 — the same point
    // `bmem_init` calls `_end_os_stram`, biosmem.c:66). NewTOS' OS budget in
    // the ST-RAM window ends at the fixed `membot`
    // (`kernel::bios::machine::layout::STRAM_OS_RESERVE`), which is exactly
    // where the ST-RAM pool begins, so
    // `os_end == end_os == membot == stram_start`. (Until BB2 both
    // fields carried the pool *bounds* instead, which made the classic
    // "free ST-RAM = memtop - os_end" come out as 0.)
    let (stram_start, stram_end) = crate::bdos::mem::init::stram_range();
    wr_long(tpa, oh + 0x08, OSHEADER_VA);
    wr_long(tpa, oh + 0x0C, stram_start as u32);
    // os_rsvl (+0x10): EmuTOS `os_res1: .dc.l _main` (startup.S:92-93) —
    // the same restart entry as `reseth`, not 0 (osheader.md §os_rsvl,
    // corrected 2026-10-17). TeraDesk's "Reboot" jumps through it
    // (`main.c:2096`). NewTOS: the reset stub, as for `reseth`.
    wr_long(tpa, oh + 0x10, crate::bios::reset::RESET_STUB_VA as u32);
    // os_magic (+0x14): pointer to the GEM_MUPB — populated only once GEM/AES
    // is reimplemented into a per-process GEM_MUPB (osheader.md §os_magic);
    // its `gm_init` (AES entry) has no faithful native EL0-VA value yet, so
    // it is left 0 rather than guessed (flagged to the lead). 0.
    // os_date (+0x18):
    wr_long(tpa, oh + 0x18, OS_DATE);
    // os_conf (+0x1C):
    wr_word(tpa, oh + 0x1C, OS_CONF);
    // os_dosdate (+0x1E):
    wr_word(tpa, oh + 0x1E, OS_DOSDATE);
    // os_root (+0x20): BDOS quick-pool head (EL1-only). Left 0.
    // os_kbshift (+0x24): ptr to the kbshift byte (EL1-only). Left 0.
    // os_run (+0x28): `PD**` → the cell holding the current-PD ptr (EmuTOS
    // startup.S:110-111, `_boot-init.md` §4). The cell is page 0's
    // [`OS_RUN_CELL_VA`] and holds this process's PD pointer: page 0 is the
    // process's own, and while it runs the current process is itself (plan
    // Frage 12, ARCH03 stage 1 — the basepage is no longer at VA 0). A desk
    // accessory's cell is rewritten to EmuTOS's `run` whenever it gets the CPU
    // (`super::set_current`, TASK/PROC03 B5).
    wr_long(tpa, oh + 0x28, OS_RUN_CELL_VA as u32);
    tpa[OS_RUN_CELL_VA as usize..OS_RUN_CELL_VA as usize + 8].copy_from_slice(&pd.to_le_bytes());
    // os_dummy (+0x2C): OS signature. EmuTOS 'ETOS'; NewTOS 'NTOS'
    // (osheader.md §os_dummy).
    wr_long(tpa, oh + 0x2C, u32::from_be_bytes(*b"NTOS"));

    // ── Classic system variables ($400 region, _boot-init.md §7; EmuTOS
    //    tosvars.ld + memory.S) ─────────────────────────────────────────
    // memvalid/memval2/memval3: RAM-config validity magics (cold-boot sets
    // them at the end of memconf).
    wr_long(tpa, 0x420, MEMVALID_MAGIC);
    wr_long(tpa, 0x43A, MEMVAL2_MAGIC);
    wr_long(tpa, 0x51A, MEMVAL3_MAGIC);
    // resvalid/resvector ($426/$42A): warm-reset vector pair — EmuTOS never
    // sets them (cold-boot value 0, _boot-init.md §5); an app pokes them.
    wr_long(tpa, 0x426, 0);
    wr_long(tpa, 0x42A, 0);
    // phystop ($42E) = physical top of ST-RAM; membot ($432) = start of TPA;
    // memtop ($436) = end of TPA (biosmem.c:66-72). NewTOS: the ST-RAM pool
    // bounds (no video-RAM carve-out, so phystop == memtop == pool end).
    wr_long(tpa, 0x42E, stram_end as u32);
    wr_long(tpa, 0x432, stram_start as u32);
    wr_long(tpa, 0x436, stram_end as u32);
    // bootdev ($446): default boot drive (bios.c:798-799). NewTOS boots from
    // the first available hard disk (C:), i.e. the lowest set bit in the
    // drive map (drive.rs FIRST_HD == C: == 2).
    let bootdev = crate::bdos::fs::drive::boot_drive();
    wr_word(tpa, 0x446, bootdev);
    // cmdload ($482): load COMMAND.PRG instead of the ROM shell when != 0;
    // boot-config flag, default 0 (ROM-shell path, EmuTOS `bios.c:850` gate,
    // `_boot-init.md` §7c). NewTOS has no COMMAND.PRG override, so 0.
    wr_word(tpa, 0x482, 0);
    // conterm ($484, BYTE): console attribute bits, boot default 7 = keyclick
    // + key-repeat + system-bell on (EmuTOS `bios/ikbd.c:1100`). Bit 3
    // (`& 8`) is the `Bconin` keyboard-shift-state fold request
    // (`bios/ikbd.c:279`); read per-process by `pack_console_key`
    // (`bios/chardev.rs`). Single byte — the surrounding TPA is already
    // zeroed, so the $485 alignment gap stays 0.
    tpa[0x484] = CONTERM_DEFAULT;
    // dumpflg ($4EE): hardcopy/screen-dump-in-progress flag, boot default -1
    // (EmuTOS `bios/bios.c:357` — `dumpflg = -1;`); consumed by `Scrdmp`
    // (XBIOS, out of scope). `-1` as a WORD is 0xFFFF.
    wr_word(tpa, 0x4EE, (-1i16) as u16);
    // _hz_200 ($4BA): 200 Hz system-tick counter (EmuTOS `bios/vectors.S:503`).
    // Seeded here from the live kernel counter so the cell is already correct
    // before the process runs; from then on the 200 Hz timer interrupt keeps
    // the *running* process's copy fresh (`bios::tick::advance` → `publish`).
    wr_long(tpa, crate::bios::tick::HZ_200_ADDR, crate::bios::tick::hz_200());
    // drvbits ($4C2): physical drive bitmap (Drvmap, blkdev_init).
    wr_long(tpa, 0x4C2, crate::bios::drive::drvbits());
    // sysbase ($4F2): pointer to the OSHEADER (bios.c:358) — the field whose
    // absence crashed TeraDesk (`*(long*)0x4F2`). Adjacent shell_p@0x4F6 is
    // left 0, so an LP64 8-byte read of $4F2 still yields OSHEADER_VA.
    wr_long(tpa, 0x4F2, OSHEADER_VA);
    // end_os ($4FA): end of the OS's static ST-RAM. EmuTOS `bmem_init`
    // sets `end_os = _end_os_stram` and then `membot = end_os`
    // (biosmem.c:66-67), so the two carry the same value at boot and only
    // diverge if a later `balloc_stram(bottom)` moves both (biosmem.c:125).
    // NewTOS has no `balloc_stram`, so this is the ST-RAM pool start.
    wr_long(tpa, 0x4FA, stram_start as u32);
    // p_cookies ($5A0): pointer to the cookie jar (cookie_init, util/cookie.c).
    wr_long(tpa, 0x5A0, COOKIE_JAR_VA);
    // ramtop ($5A4) / ramvalid ($5A8): top of the alt-RAM ("TT-RAM") region,
    // and the magic that validates it. EmuTOS `ttram_detect` sets
    // `ramtop = TTRAM_START + detected_size` (NULL when there is none) and
    // *always* stores `ramvalid = RAMVALID_MAGIC`, "even if ramtop == 0,
    // like TOS 1.6+" (`bios/memory2.c:115-124`, and `:128-129` without TT-RAM). NewTOS' alt-RAM extent is
    // the boot-seeded `pmdalt` pool, so `ramtop` is its exclusive end (0
    // when the board has no alt-RAM, which is EmuTOS' `ramtop == NULL`).
    //
    // Note for the LP64 app ABI (ADR-020): `ramtop` sits directly above
    // `p_cookies`, so an app that reads `*(void**)0x5A0` as **eight** bytes
    // now gets `(ramtop << 32) | jar` instead of the bare jar pointer. That
    // is the classic-TOS layout, which is 32-bit by construction; the read
    // width is the app/toolchain's business (mintlib `sysvars.h:60` declares
    // `ramtop` as `unsigned long *` at $5a4). TeraDesk's live cookie path
    // reads the jar through `Setexc(0x5A0/4, -1)`
    // (`library/utility/cookie.c:56`), which is unaffected.
    let (_ttram_start, ttram_end) = crate::bdos::mem::init::ttram_range();
    wr_long(tpa, 0x5A4, ttram_end as u32);
    wr_long(tpa, 0x5A8, RAMVALID_MAGIC);
    // prt_stat ($506) / prt_vec ($50A): printer status/output redirection
    // vectors. EmuTOS points them at `just_rts` (a no-op RTS stub) in
    // `chardev_init` (`_boot-init.md` §7c). NewTOS has no printer device and
    // no m68k vector page to install, so they stay 0 (no redirection =
    // printer absent) — HW-absence, like the OSHEADER's m68k-only
    // `os_entry` instruction left 0 above.
    wr_long(tpa, 0x506, 0);
    wr_long(tpa, 0x50A, 0);

    // ── Cookie jar (classic array form, _cookie-jar.md; EmuTOS
    //    util/cookie.c:26-35) ───────────────────────────────────────────
    write_cookie_jar(tpa);
}

/// Write the `$5A0` cookie-jar mirror into the reserved low region of one
/// process's TPA: the kernel jar ([`crate::cookie`]) rendered in the classic
/// `{tag, value}` LONG-pair array form, terminated with `{0, capacity}`
/// (`docs/api/bios/_cookie-jar.md`; EmuTOS `util/cookie.c:26-35`). Native
/// programs normally read cookies by value through the `Ssystem` subset
/// (ADR-012); this array is the direct-`$5A0` compatibility path a 1:1-ported
/// TOS program uses (TeraDesk `library/utility/cookie.c:28,56`).
///
/// Called from [`populate_sysvars`] at load time **and** from
/// [`refresh_cookie_mirrors`] whenever the kernel jar changes, so both paths
/// produce a bit-identical mirror and cannot drift apart.
///
/// **Capacity.** The kernel jar holds up to `cookie::capacity()` (32) cookies;
/// the mirror window `[COOKIE_JAR_VA, SYSVAR_AREA_END)` holds
/// [`COOKIE_JAR_SLOTS`] (16) slots, i.e. **15 cookies plus the terminator**. A
/// kernel jar with more than 15 entries is therefore **truncated** — but the
/// last written slot is always the classic `{0, COOKIE_JAR_SLOTS}` terminator,
/// so an application's linear scan still stops inside the window instead of
/// running off its end. Truncation cannot happen with the boot-seeded set
/// (5 cookies); it needs 11 further `Ssystem(S_SETCOOKIE)` calls.
///
/// The terminator's `value` is the **mirror's** slot capacity, not the kernel
/// jar's: EmuTOS' terminator word describes the array the reader is scanning
/// (`cookie_init`, `util/cookie.c:33`), and that array is this one.
///
/// Slots past the terminator keep whatever an earlier, longer jar left there
/// — exactly as EmuTOS' in-place `cookie_add` does; a reader stops at
/// `tag == 0` and never sees them.
fn write_cookie_jar(tpa: &mut [u8]) {
    let jar = COOKIE_JAR_VA as usize;
    let mut slot = 0u32;
    let n = crate::cookie::count();
    for i in 1..=n {
        if slot >= COOKIE_JAR_SLOTS - 1 {
            break; // leave the last slot for the terminator
        }
        if let Some(tag) = crate::cookie::slot_tag(i) {
            let value = crate::cookie::get(tag).unwrap_or(0);
            let off = jar + (slot as usize) * 8;
            wr_long(tpa, off, tag);
            wr_long(tpa, off + 4, value);
            slot += 1;
        }
    }
    // Terminator {tag = 0, value = capacity} (EmuTOS cookie_init). `slot` is
    // capped at COOKIE_JAR_SLOTS - 1 above, so this pair always lands inside
    // the window (see the SYSVAR_AREA_END const assert).
    let off = jar + (slot as usize) * 8;
    wr_long(tpa, off, 0);
    wr_long(tpa, off + 4, COOKIE_JAR_SLOTS);
}

/// Re-write the `$5A0` cookie-jar mirror of **every process that owns a TPA**,
/// so a cookie added/updated/removed after load reaches a program that is
/// already running.
///
/// Called from [`crate::cookie::set`] / [`crate::cookie::del`] — i.e. from the
/// jar itself rather than from each caller — so "whoever changes the kernel jar
/// pulls the mirrors along" holds by construction and a future third mutator
/// cannot forget it.
///
/// **Why every process, not just the caller.** Classic TOS has exactly *one*
/// jar: every program dereferences the same `$5A0` cell and sees the same
/// array (`docs/api/bios/_cookie-jar.md` §"How applications access it"). The
/// per-process copy is a NewTOS artefact of materialising the `$400` low-memory
/// area inside each TPA (B1 / Option 1,
/// `docs/analysis/bios-audit/b1-lowmem-feasibility.md`), not a TOS-visible
/// property. Refreshing only the calling process would make the *observable*
/// jar differ between two live programs, which classic TOS cannot do — and the
/// case is reachable today: a `Pexec`'d child's `Ssystem(S_SETCOOKIE)` must
/// still be there for its parent (the desktop) after the child exits.
///
/// **Which slots.** Every slot with a published `basepage_kva`, which is
/// exactly [`arena::page_bytes_mut`]'s own "has a loaded TPA" criterion. The
/// slot *state* is deliberately not part of the test: [`load_to_slot`]
/// publishes `basepage_kva` without touching `state`, so a loaded-but-not-yet-
/// launched process (and the kernel self-tests that load without
/// `allocate_slot`) would be skipped by a `state != Free` filter although its
/// TPA is live and its mirror stale.
///
/// Cost: [`super::MAX_PROCESSES`] (16) pointer loads and, per live process,
/// at most `COOKIE_JAR_SLOTS * 8` = 128 bytes written — on an operation that
/// happens a handful of times per boot.
///
/// No cache maintenance is needed: the kernel writes **data** through the TPA's
/// high-half alias and EL0 reads the same physical bytes through its low-half
/// mapping. Both mappings are Normal cacheable and AArch64 data caches behave
/// as PIPT, so the two VAs hit the same cache line; single-CPU program order
/// then makes the write visible to the `eret` that follows. (The loader's
/// `sync_code_cache` exists for the *instructions* it writes, not for this.)
pub fn refresh_cookie_mirrors() {
    for slot in 0..super::MAX_PROCESSES {
        // SAFETY: the jar is mutated only from the synchronous SVC trap path
        // (and from boot seeding, when no process exists), which is exactly
        // the BDOS-giant-lock / single-CPU context `page_bytes_mut` requires;
        // a published `basepage_kva` is the kernel-writable base of that
        // slot's mapped TPA.
        if let Some(page) = unsafe { arena::page_bytes_mut(slot) } {
            write_cookie_jar(page);
        }
    }
}

/// The five AArch64 instructions of the EL0 upcall trampoline (design §4.2),
/// stored as little-endian words. On handler `ret` (PC = LR =
/// [`TRAMPOLINE_VA`]) it calls `Psigreturn` (GEMDOS 0x11A) — bit-exact with
/// FreeMiNT's `move.w #0x011a` (`user_things.S`) — to resume the interrupted
/// EL0 context; the tail is FreeMiNT's `pc_valid_return` safety net
/// (`Pterm(-998)`), taken only if `Psigreturn` ever wrongly returns. No EL0
/// stack push is needed: the handler argument and return address travel in
/// registers (x0..x3, LR), not on the stack, per AAPCS64 (design §4.2).
///
/// Encodings verified with `aarch64-elf-as`:
/// ```text
///   d2802348  mov x8, #0x11A   ; Psigreturn function id
///   d4000021  svc #1           ; GEMDOS trap — does not return
///   d29f8340  mov x0, #0xFC1A  ; -998 as UWORD exit code
///   d2800988  mov x8, #0x4C    ; Pterm
///   d4000021  svc #1
/// ```
pub const TRAMPOLINE_CODE: [u32; 5] =
    [0xD280_2348, 0xD400_0021, 0xD29F_8340, 0xD280_0988, 0xD400_0021];

/// Has [`ensure_stub_page`] written the stub page?
static STUB_PAGE_READY: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

/// Write every EL0 stub into the shared stub page (`mmu::STUB_PAGE_VA`,
/// MEM12) — the upcall trampoline, the `G_USERDEF` callout stub, the KEYS02
/// `kbdvec` stubs and the RST01 reset stub — once, the first time a process
/// is installed, then make them visible to instruction fetch (clean D-cache
/// to PoU, invalidate I-cache: QEMU is cache-less, the A72 is not). The page
/// is EL0 execute-only; no process can change it.
///
/// # Safety
///
/// Caller holds the BDOS giant lock; EL1.
pub unsafe fn ensure_stub_page() {
    if STUB_PAGE_READY.load(Ordering::Acquire) {
        return;
    }
    // SAFETY: caller's lock contract; nothing executes from the page yet.
    let page = unsafe { mmu::stub_page_bytes() };
    page.fill(0);
    write_stub_code(page, TRAMPOLINE_VA, &TRAMPOLINE_CODE);
    write_stub_code(page, USERDEF_TRAMPOLINE_VA, &USERDEF_STUB_CODE);
    crate::xbios::kbdvecs::write_stubs(page);
    crate::bios::reset::write_stub(page);
    // SAFETY: the stub page is live and mapped in the kernel's high half.
    unsafe { mmu::sync_code_cache(page.as_ptr() as usize, page.len()) };
    STUB_PAGE_READY.store(true, Ordering::Release);
}

/// Store `code` (little-endian A64 words) at EL0 VA `va` of the stub page
/// `page`.
pub fn write_stub_code(page: &mut [u8], va: u64, code: &[u32]) {
    let base = (va - mmu::STUB_PAGE_VA) as usize;
    for (i, insn) in code.iter().enumerate() {
        let off = base + i * 4;
        page[off..off + 4].copy_from_slice(&insn.to_le_bytes());
    }
}

/// The five AArch64 instructions of the AES `G_USERDEF` callout stub
/// (AES-as-thread design §4). Delivered as an EL0 upcall with `x0 = ub_code`,
/// `x1 = parmblk_ptr` (AAPCS64); it calls `ub_code(parmblk_ptr)` at EL0
/// (user→user — no PXN, exactly XaAES' `progdef_callout`), then issues
/// `Psigreturn` (GEMDOS 0x11A) with the WORD result still in `w0`, so the
/// kernel reads it from the trap frame and resumes the stalled AES fiber.
/// Until MEM12 the stub stored the WORD in page 0 (`str w0, [#0x760]`), a
/// write the now read-only page 0 refuses.
///
/// Encodings: the five words of the former seven-word stub that remain
/// (verified with `aarch64-elf-as` then):
/// ```text
///   aa0003e9  mov x9, x0        ; x9 = ub_code
///   aa0103e0  mov x0, x1        ; x0 = parmblk_ptr (arg0 to ub_code)
///   d63f0120  blr x9            ; ub_code(parmblk_ptr) -> WORD in w0
///   d2802348  mov x8, #0x11A    ; Psigreturn
///   d4000021  svc #1            ; GEMDOS trap — resumes the AES fiber
/// ```
pub const USERDEF_STUB_CODE: [u32; 5] = [0xAA00_03E9, 0xAA01_03E0, 0xD63F_0120, 0xD280_2348, 0xD400_0021];

/// Resolve an [`EnvSpec`] into an optional pool address for `p_env`,
/// allocating + copying the environment via the BDOS pool. Mirrors
/// EmuTOS `alloc_env` (`bdos/proc.c:413-431`). The block belongs to the
/// caller (`owner = run`); the `PE_LOADGO` hand-off to the child
/// (`set_owner`, `bdos/proc.c:302-307`) is the loader's, once the TPA — the
/// child's PD — exists ([`load_from_source`]).
///
/// # Safety
///
/// Caller holds the BDOS giant lock.
unsafe fn alloc_env_for(env: EnvSpec<'_>, prgflags: u32) -> Result<Option<Long>, LoadError> {
    // Materialise the source env image (including its terminating
    // double-NUL) as a byte slice, then copy it into a fresh pool block.
    match env {
        EnvSpec::None => Ok(None),
        EnvSpec::Explicit(img) => {
            // SAFETY: BDOS lock held.
            let phys = unsafe { alloc_env(img, prgflags)? };
            Ok(Some(phys))
        }
        EnvSpec::Inherit => {
            // env == NULL → duplicate run->p_env (EmuTOS proc.c:419-420).
            // SAFETY: BDOS lock held; `current_kva` is the running basepage's
            // kernel alias or null.
            let phys = unsafe {
                let src = inherited_env(crate::bdos::process::current_kva());
                alloc_env(src, prgflags)?
            };
            Ok(Some(phys))
        }
    }
}

/// What `alloc_env(flags, NULL)` copies (EmuTOS `bdos/proc.c:419-421`):
/// `run->p_env` up to and with its double NUL. `run` is the running
/// process's basepage (kernel alias); with no running process (`null`) it is
/// the kernel's stand-in PD (TASK/PPARENT, [`super::standin::kernel_run_env`]):
/// `initial_basepage`'s `double_nul` before the AES, the AES / accdesk /
/// EmuCON environment after it (TASK/PROC03). A `p_env` of 0, or no stand-ins
/// yet, gives the empty env.
///
/// # Safety
///
/// `run` is null or the kernel alias of a live basepage whose `p_env` is 0
/// or a readable environment block.
pub(super) unsafe fn inherited_env(run: *const BasePage) -> &'static [u8] {
    if run.is_null() {
        return super::standin::kernel_run_env().unwrap_or(&EMPTY_ENV);
    }
    // SAFETY: caller contract.
    let penv = unsafe { (*run).p_env };
    if penv == 0 {
        &EMPTY_ENV
    } else {
        // SAFETY: caller contract — a readable block.
        unsafe { env_slice_from_phys(penv) }
    }
}

/// Allocate a pool block for `img` and copy it verbatim, returning the
/// block's pool address. `img` is the env content **including** its
/// terminating double-NUL (its length is EmuTOS's `envsize`). The size
/// is rounded up to even (`bdos/proc.c:421`) and the rounding tail is
/// zero-filled.
///
/// # Safety
///
/// Caller holds the BDOS giant lock.
pub(super) unsafe fn alloc_env(img: &[u8], prgflags: u32) -> Result<Long, LoadError> {
    // Round the byte count up to even; never below the 2-byte empty env.
    let size = (((img.len() + 1) & !1).max(2)) as Long;
    // EmuTOS `alloc_env`: `(flags & PF_TTRAMLOAD) ? MX_PREFTTRAM : MX_STRAM`
    // (`bdos/proc.c:425`).
    // SAFETY: BDOS lock held.
    let phys = unsafe { crate::bdos::mem::traps::mxalloc(size, pool_mode(prgflags)) };
    if phys == 0 {
        return Err(LoadError::NoEnvMemory);
    }
    // The environment takes the program's protection mode (ARCH03 3b,
    // FreeMiNT `create_env` with the prgflags mode and `mark_region(env,
    // …)` after the load, `sys/memory.c:1403-1407`, `sys/k_exec.c:398`).
    // SAFETY: BDOS lock held; `phys` starts the block just allocated.
    unsafe { super::memprot::set_mode(phys as u32 as u64, prot_from_prgflags(prgflags)) };
    // Copy verbatim into the pool block (through its kernel alias),
    // zero-padding the even-rounding tail.
    let dst = kalias(phys as u64) as *mut u8;
    // SAFETY: `dst` aliases `size` bytes of live, RW-mapped pool memory
    // (whole DRAM is linearly mapped in the high half); `img` is a
    // distinct, live slice.
    unsafe {
        core::ptr::copy_nonoverlapping(img.as_ptr(), dst, img.len());
        for i in img.len()..size as usize {
            dst.add(i).write(0);
        }
    }
    Ok(phys)
}

/// The pool a program's environment comes from, by its `PRGFLAGS`: TT-RAM
/// preferred with `PF_TTRAMLOAD`, ST-RAM only without — EmuTOS `alloc_env`
/// (`bdos/proc.c:425`). The TPA's pool is [`arena::choose_pool`]'s
/// (`alloc_tpa`, `:460-502`, with the bits 31-28 tie-break).
pub const fn pool_mode(prgflags: u32) -> i16 {
    if prgflags & PF_TTRAMLOAD != 0 {
        MX_PREFTTRAM
    } else {
        MX_STRAM
    }
}

/// The `PRGFLAGS` an image carries in its NewTOS note (ARCH03 E8b), or
/// `None` without one. Reads every `PT_NOTE` from the source — for a
/// streamed file into [`HEADER_BUF`], whose header contents the parse has
/// already consumed — and hands it to [`elf::find_prgflags_note`].
///
/// # Errors
///
/// [`LoadError::Parse`] with [`ParseError::BadNoteSegment`] when a note
/// segment cannot be read, or what [`elf::find_prgflags_note`] reports.
fn read_prgflags(src: &ImageSource<'_>, img: &ElfImage) -> Result<Option<u32>, LoadError> {
    // The note segments fit the header buffer (the parser caps them).
    const _: () = assert!(elf::MAX_NOTE_BYTES as usize <= HEADER_BUF_SIZE);
    let mut found: Option<u32> = None;
    for seg in img.notes() {
        let (start, len) = (seg.file_offset as usize, seg.file_size as usize);
        let bytes: &[u8] = match src {
            ImageSource::Memory(blob) => blob
                .get(start..start + len)
                .ok_or(LoadError::Parse(ParseError::BadNoteSegment))?,
            ImageSource::File(_) => {
                // SAFETY: single BDOS-locked trap path; the parse that used
                // the buffer has returned (its result is `Copy`) and nothing
                // else borrows it; this borrow ends with the iteration.
                let buf = unsafe { &mut *HEADER_BUF.0.get() };
                if !src.read_exact_at(seg.file_offset, &mut buf[..len]) {
                    return Err(LoadError::Parse(ParseError::BadNoteSegment));
                }
                &buf[..len]
            }
        };
        if let Some((v, _)) = elf::find_prgflags_note(bytes, seg.align)? {
            if found.is_some() {
                return Err(LoadError::Parse(ParseError::BadNote));
            }
            found = Some(v);
        }
    }
    Ok(found)
}

/// Borrow the environment block at pool address `penv` (a user address,
/// read through its kernel alias) as a byte slice, including its
/// terminating double-NUL. Scans for the
/// `\0\0` terminator, bounded by [`ENV_SCAN_LIMIT`] against a malformed
/// (unterminated) parent env.
///
/// # Safety
///
/// Caller holds the BDOS giant lock and `penv` addresses a live pool
/// environment block (invariant of every `p_env` this loader writes).
unsafe fn env_slice_from_phys(penv: Long) -> &'static [u8] {
    let base = kalias(penv as u32 as u64) as *const u8;
    let mut n = 0usize;
    while n + 1 < ENV_SCAN_LIMIT {
        // SAFETY: `n + 1 < ENV_SCAN_LIMIT`; pool memory is mapped.
        let b0 = unsafe { base.add(n).read() };
        let b1 = unsafe { base.add(n + 1).read() };
        if b0 == 0 && b1 == 0 {
            n += 2; // include the terminating double-NUL
            break;
        }
        n += 1;
    }
    // SAFETY: `base .. base+n` is within the scanned, mapped range.
    unsafe { core::slice::from_raw_parts(base, n) }
}

/// Upper bound on a parent environment scan (`env_slice_from_phys`),
/// guarding against an unterminated block. Generous versus real Atari
/// environments (a handful of `KEY=value` records).
const ENV_SCAN_LIMIT: usize = 4096;

/// The end of the image in bytes from the PD (= the TPA offset of its last
/// byte + 1, BSS included): EmuTOS' `needed` without the basepage term, which
/// the NewTOS layout already contains (`bdos/proc.c:291`). The image starts at
/// [`RESERVED_LOW_END`] at the earliest.
///
/// A `PT_LOAD` (link VA = byte offset in the TPA) must start at or after
/// [`RESERVED_LOW_END`] — the BasePage and its reserved gap occupy the TPA's
/// first page. How large the image may be is the TPA's business: a program
/// the largest free block cannot hold gets `ENSMEM` from [`arena::alloc_tpa`],
/// as from EmuTOS `alloc_tpa`.
fn image_end_of(img: &ElfImage) -> Result<u64, LoadError> {
    let mut end = RESERVED_LOW_END;
    for seg in img.segments() {
        // Segments below the reserved low region would overlap the basepage,
        // so they are rejected — the SDK links native apps at
        // RESERVED_LOW_END (`sdk/newtos.ld`).
        if seg.vaddr < RESERVED_LOW_END {
            return Err(LoadError::SegmentVaOutOfRange);
        }
        let e = seg
            .vaddr
            .checked_add(seg.mem_size)
            .filter(|&e| e <= i32::MAX as u64)
            .ok_or(LoadError::ProgramTooLarge)?;
        // PF_R is implicit for any LOAD; PF_W and PF_X drive the text/data
        // classification in `populate_basepage`. The whole pool is mapped
        // UserRwExec, so the flags don't gate a mapping decision — a
        // per-segment RX/RW split is a later hardening step.
        let _ = seg.flags & (PF_W | PF_X);
        end = end.max(e);
    }
    Ok(end)
}

#[cfg(test)]
mod cookie_mirror_tests {
    //! The `$5A0` cookie-jar mirror (`docs/api/bios/_cookie-jar.md`):
    //! a cookie set/removed **after** a process was loaded must reach that
    //! process's mirror, and the mirror must stay a well-formed
    //! `{tag, value}`-array terminated with `{0, capacity}` even when the
    //! kernel jar holds more cookies than the mirror window has slots.
    //!
    //! A single `#[test]` on purpose: both the kernel jar and the process
    //! table are process-wide statics, so two cookie tests running in
    //! parallel in the same test binary would interfere.

    use super::*;
    use crate::bdos::process::{allocate_slot, free_slot};

    /// Scan a mirror the way an application does (TeraDesk
    /// `library/utility/cookie.c:28-36`): walk `{tag, value}` LONG pairs from
    /// the jar base, stop at `tag == 0`. Returns the value, plus the number of
    /// pairs walked. Refuses to read past the mirror window, so a missing
    /// terminator shows up as `None` rather than as a runaway scan.
    fn scan(page: &[u8], want: u32) -> (Option<u32>, usize) {
        let base = COOKIE_JAR_VA as usize;
        for i in 0..COOKIE_JAR_SLOTS as usize {
            let off = base + i * 8;
            let tag = u32::from_le_bytes(page[off..off + 4].try_into().unwrap());
            let val = u32::from_le_bytes(page[off + 4..off + 8].try_into().unwrap());
            if tag == 0 {
                return (None, i);
            }
            if tag == want {
                return (Some(val), i);
            }
        }
        (None, COOKIE_JAR_SLOTS as usize)
    }

    /// Value of the terminator slot reached by a scan, or `None` if the scan
    /// ran out of window without meeting one.
    fn terminator(page: &[u8]) -> Option<u32> {
        let base = COOKIE_JAR_VA as usize;
        for i in 0..COOKIE_JAR_SLOTS as usize {
            let off = base + i * 8;
            let tag = u32::from_le_bytes(page[off..off + 4].try_into().unwrap());
            if tag == 0 {
                return Some(u32::from_le_bytes(
                    page[off + 4..off + 8].try_into().unwrap(),
                ));
            }
        }
        None
    }

    #[test]
    fn a_cookie_set_after_load_reaches_a_running_process_mirror() {
        const TAG: u32 = u32::from_be_bytes(*b"TST1");
        const OTHER: u32 = u32::from_be_bytes(*b"TST2");

        // A loaded process: a basepage published as the slot's
        // `basepage_kva`, exactly what `load_to_slot` publishes; its mirror
        // lives in the slot's page 0 (ARCH03 stage 1).
        let slot = allocate_slot().expect("a free process slot");
        let bp_box: Box<[u8; 4096]> = Box::new([0u8; 4096]);
        let bp_ptr = Box::into_raw(bp_box);
        PROCESS_TABLE[slot]
            .basepage_kva
            .store(bp_ptr.cast(), Ordering::Release);

        // Load time: the mirror is written once, from the jar as it is now.
        // SAFETY: the slot's page 0 is this test's for its whole run.
        let page: &mut [u8; 4096] = unsafe { mmu::page0_bytes(slot) }.expect("page 0");
        page.fill(0);
        write_cookie_jar(page);
        let before = scan(page, TAG).0;

        // …and now a cookie appears after the process is already loaded.
        assert!(crate::cookie::set(TAG, 0xC0FF_EE01));
        let (after, _) = scan(page, TAG);

        assert_eq!(before, None, "TST1 must not be in the load-time snapshot");
        assert_eq!(
            after,
            Some(0xC0FF_EE01),
            "a cookie set after load must reach the $5A0 mirror",
        );

        // An update of an existing tag must land too, not just an append.
        assert!(crate::cookie::set(TAG, 0xC0FF_EE02));
        assert_eq!(scan(page, TAG).0, Some(0xC0FF_EE02));

        // …and a removal must take it out of the mirror again.
        assert!(crate::cookie::set(OTHER, 0x1234_5678));
        assert!(crate::cookie::del(TAG));
        assert_eq!(scan(page, TAG).0, None, "a deleted cookie must vanish");
        assert_eq!(
            scan(page, OTHER).0,
            Some(0x1234_5678),
            "deleting one cookie must not disturb its neighbours",
        );
        assert!(crate::cookie::del(OTHER));

        // Capacity: the kernel jar holds more (32) than the mirror window
        // (16 slots = 15 cookies + terminator). Overfilling must truncate and
        // still leave the classic {0, capacity} pair as the last written slot,
        // so an application's scan stops inside the window.
        let mut filled = Vec::new();
        for i in 0..20u32 {
            let tag = u32::from_be_bytes([b'X', b'X', b'0' + (i / 10) as u8, b'0' + (i % 10) as u8]);
            assert!(crate::cookie::set(tag, 0xA000 + i));
            filled.push(tag);
        }
        assert_eq!(
            terminator(page),
            Some(COOKIE_JAR_SLOTS),
            "the mirror must still end in {{0, COOKIE_JAR_SLOTS}}",
        );
        // The terminator sits at slot 15: 15 cookies mirrored, the rest cut.
        assert_eq!(scan(page, 0xDEAD_BEEF).1, (COOKIE_JAR_SLOTS - 1) as usize);
        assert_eq!(scan(page, filled[0]).0, Some(0xA000));
        assert_eq!(scan(page, filled[14]).0, Some(0xA000 + 14));
        assert_eq!(scan(page, filled[15]).0, None, "slot 16 is truncated away");

        // Restore the process-wide statics for any other test in this binary.
        for tag in filled {
            assert!(crate::cookie::del(tag));
        }
        free_slot(slot);
        // SAFETY: the slot no longer references the buffer; reclaim it.
        drop(unsafe { Box::from_raw(bp_ptr) });
    }
}

#[cfg(test)]
mod basepage_segment_tests {
    //! The six basepage segment fields (`p_tbase/p_tlen/p_dbase/p_dlen/
    //! p_bbase/p_blen`, `docs/api/structures/basepage.md`) as
    //! [`populate_basepage`] derives them from the `PT_LOAD` table
    //! (TASK/T05). EmuTOS fills them from the PRG header
    //! (`bdos/kpgmld.c:123-145`): `p_dbase = p_tbase + p_tlen`,
    //! `p_bbase = p_dbase + p_dlen`, `p_blen` = the BSS size. Here the
    //! text is the first executable segment and data/BSS the first
    //! writable non-executable one, which is the two-`PT_LOAD` shape
    //! `sdk/newtos.ld` emits (text R-X, data RW- with `.bss` as its
    //! memsz-only tail).
    //!
    //! No process slot, pool or page table is touched: `populate_basepage`
    //! writes into a plain byte buffer, so these run as ordinary host tests.

    use super::*;
    use crate::bdos::process::elf::{PF_R, PF_W, PF_X};

    // ET_DYN: the loader accepts position-independent executables only
    // since ARCH03 stage 0d (ET_EXEC -> ParseError::NotPositionIndependent).
    const ET_DYN: u16 = 3;
    const EM_AARCH64: u16 = 0xb7;
    const PT_LOAD: u32 = 1;
    const EHDR: usize = 64;
    const PHDR: usize = 56;

    fn put16(b: &mut [u8], off: usize, v: u16) {
        b[off..off + 2].copy_from_slice(&v.to_le_bytes());
    }
    fn put32(b: &mut [u8], off: usize, v: u32) {
        b[off..off + 4].copy_from_slice(&v.to_le_bytes());
    }
    fn put64(b: &mut [u8], off: usize, v: u64) {
        b[off..off + 8].copy_from_slice(&v.to_le_bytes());
    }

    /// `(flags, vaddr, filesz, memsz)` per `PT_LOAD`.
    type Seg = (u32, u64, u64, u64);

    /// A well-formed ELF64 AArch64 PIE (`ET_DYN`) whose `PT_LOAD` table is
    /// exactly `segs`; every segment's file bytes are placed after the
    /// headers (contents irrelevant here — only the table is read).
    fn elf_with(segs: &[Seg]) -> Vec<u8> {
        let hdr_len = EHDR + PHDR * segs.len();
        let payload: u64 = segs.iter().map(|s| s.2).sum();
        let mut b = vec![0u8; hdr_len + payload as usize];
        b[0..4].copy_from_slice(b"\x7fELF");
        b[4] = 2; // ELFCLASS64
        b[5] = 1; // ELFDATA2LSB
        b[6] = 1; // EV_CURRENT
        put16(&mut b, 16, ET_DYN);
        put16(&mut b, 18, EM_AARCH64);
        put32(&mut b, 20, 1);
        put64(&mut b, 24, RESERVED_LOW_END); // e_entry = first text byte
        put64(&mut b, 32, EHDR as u64); // e_phoff
        put16(&mut b, 54, PHDR as u16); // e_phentsize
        put16(&mut b, 56, segs.len() as u16); // e_phnum

        let mut off = hdr_len as u64;
        for (i, &(flags, vaddr, filesz, memsz)) in segs.iter().enumerate() {
            let p = EHDR + i * PHDR;
            put32(&mut b, p, PT_LOAD);
            put32(&mut b, p + 4, flags);
            put64(&mut b, p + 8, off);
            put64(&mut b, p + 16, vaddr);
            put64(&mut b, p + 32, filesz);
            put64(&mut b, p + 40, memsz);
            off += filesz;
        }
        b
    }

    /// Parse `segs` into an image, write its basepage into a scratch
    /// window and hand the six fields back as
    /// `(tbase, tlen, dbase, dlen, bbase, blen)`.
    fn fields_of(segs: &[Seg]) -> (u64, u64, u64, u64, u64, u64) {
        let blob = elf_with(segs);
        let img = elf::parse(&blob, blob.len() as u64).expect("well-formed ELF64");
        assert_eq!(img.segments().len(), segs.len(), "every PT_LOAD must be seen");
        let mut tpa = vec![0u8; RESERVED_LOW_END as usize];
        populate_basepage(&mut tpa, &img, &[], None, 0, 0x40_0000, PF_STANDARD);
        let rd = |off: usize| u32::from_le_bytes(tpa[off..off + 4].try_into().unwrap()) as u64;
        (rd(0x08), rd(0x0C), rd(0x10), rd(0x14), rd(0x18), rd(0x1C))
    }

    /// The `sdk/newtos.ld` shape: text R-X at 0x1000, data RW- right
    /// behind it with a BSS tail. All six fields are filled and satisfy
    /// the EmuTOS relations (`kpgmld.c:131-132`): `p_dbase = p_tbase +
    /// p_tlen`, `p_bbase = p_dbase + p_dlen`, and `p_bbase + p_blen` is
    /// the end of the image (the linker's `_end`).
    #[test]
    fn two_segments_fill_all_six_fields() {
        const TEXT_LEN: u64 = 0x1230;
        const DATA_LEN: u64 = 0x70;
        const BSS_LEN: u64 = 0x1e40;
        let text_base = RESERVED_LOW_END;
        let data_base = text_base + TEXT_LEN;
        let end = data_base + DATA_LEN + BSS_LEN;

        let (tbase, tlen, dbase, dlen, bbase, blen) = fields_of(&[
            (PF_R | PF_X, text_base, TEXT_LEN, TEXT_LEN),
            (PF_R | PF_W, data_base, DATA_LEN, DATA_LEN + BSS_LEN),
        ]);

        assert_eq!((tbase, tlen), (text_base, TEXT_LEN), "p_tbase/p_tlen = the R-X segment");
        assert_eq!((dbase, dlen), (data_base, DATA_LEN), "p_dbase/p_dlen = the RW- file part");
        assert_eq!((bbase, blen), (data_base + DATA_LEN, BSS_LEN), "p_bbase/p_blen = its BSS tail");

        assert!(tbase < dbase && dbase <= bbase, "p_tbase < p_dbase <= p_bbase");
        assert_eq!(dbase, tbase + tlen, "p_dbase = p_tbase + p_tlen (kpgmld.c:132)");
        assert_eq!(bbase, dbase + dlen, "p_bbase = p_dbase + p_dlen (kpgmld.c:131)");
        assert_eq!(bbase + blen, end, "p_bbase + p_blen = _end");
    }

    /// Regression: the pre-T05 single RWX segment (the shape
    /// `tools/mk-hello-ncp.py` still emits) classifies as text only —
    /// `p_tbase/p_tlen` = the whole segment's file part, the four
    /// data/BSS fields zero. Unchanged by T05: the loader was not
    /// touched, the linker script was.
    #[test]
    fn single_rwx_segment_yields_text_only() {
        const LEN: u64 = 0x800;
        const BSS: u64 = 0x100;
        let (tbase, tlen, dbase, dlen, bbase, blen) =
            fields_of(&[(PF_R | PF_W | PF_X, RESERVED_LOW_END, LEN, LEN + BSS)]);

        assert_eq!((tbase, tlen), (RESERVED_LOW_END, LEN));
        assert_eq!((dbase, dlen, bbase, blen), (0, 0, 0, 0), "an RWX segment carries no data fields");
    }

    /// ARCH03 stage 1: the basepage sits at the start of the TPA block `B`
    /// (EmuTOS `p_lowtpa = p`, `bdos/proc.c:372`) and the image executes
    /// relocated at `B + link VA`, so every address field is `B`-relative;
    /// the lengths do not change.
    #[test]
    fn address_fields_are_relative_to_the_tpa_block() {
        const B: u64 = 0x0109_7000;
        let blob = elf_with(&[
            (PF_R | PF_X, RESERVED_LOW_END, 0x400, 0x400),
            (PF_R | PF_W, RESERVED_LOW_END + 0x400, 0x80, 0x100),
        ]);
        let img = elf::parse(&blob, blob.len() as u64).expect("well-formed ELF64");
        let mut tpa = vec![0u8; RESERVED_LOW_END as usize];
        populate_basepage(&mut tpa, &img, &[3, b'a', b'b', b'c'], Some(0x0200_0010), B, B + 0x0123_4000, PF_STANDARD);
        let rd = |off: usize| u32::from_le_bytes(tpa[off..off + 4].try_into().unwrap()) as u64;
        assert_eq!(rd(0x00), B, "p_lowtpa = the PD pointer");
        assert_eq!(rd(0x04), B + 0x0123_4000, "p_hitpa = TPA top (PD + the block length)");
        assert_eq!((rd(0x08), rd(0x0C)), (B + 0x1000, 0x400), "p_tbase/p_tlen");
        assert_eq!((rd(0x10), rd(0x14)), (B + 0x1400, 0x80), "p_dbase/p_dlen");
        assert_eq!((rd(0x18), rd(0x1C)), (B + 0x1480, 0x80), "p_bbase/p_blen");
        assert_eq!(rd(0x20), B + 0x80, "p_xdta = &p_cmdlin");
        assert_eq!(rd(0x24), 0, "p_parent: set at launch, not at load");
        assert_eq!(rd(0x2C), 0x0200_0010, "p_env = the environment block's address");
        assert_eq!(&tpa[0x80..0x85], &[3, b'a', b'b', b'c', 0], "p_cmdlin");
    }

    /// The `os_run` cell (plan Frage 12): OSHEADER `os_run` points at a
    /// page-0 cell that holds the process's own PD pointer, as an 8-byte
    /// value so a 4- and an 8-byte read both yield it.
    #[test]
    fn os_run_names_the_processs_own_basepage() {
        let mut page0 = vec![0u8; RESERVED_LOW_END as usize];
        populate_sysvars(&mut page0, 0x0109_7000);
        let rd32 = |off: usize| u32::from_le_bytes(page0[off..off + 4].try_into().unwrap());
        let sysbase = rd32(0x4F2) as usize;
        assert_eq!(sysbase, OSHEADER_VA as usize);
        let cell = rd32(sysbase + 0x28) as usize;
        assert_eq!(cell as u64, OS_RUN_CELL_VA);
        assert_eq!(u64::from_le_bytes(page0[cell..cell + 8].try_into().unwrap()), 0x0109_7000);
    }

    /// RST01: `reseth` (+0x04) and `os_res1` (+0x10) both hold the reset
    /// stub's address, as EmuTOS stores `_main` in both
    /// (`bios/startup.S:82-83,92-93`), and the stub is in the stub page where
    /// the loader writes it (MEM12). `os_entry` stays 0 (no AArch64 branch
    /// fits it).
    #[test]
    fn osheader_reset_entries_point_at_the_reset_stub() {
        let mut page0 = vec![0u8; RESERVED_LOW_END as usize];
        populate_sysvars(&mut page0, 0x0109_7000);
        let mut stubs = vec![0u8; mmu::PAGE_SIZE as usize];
        crate::bios::reset::write_stub(&mut stubs);
        let rd32 = |off: usize| u32::from_le_bytes(page0[off..off + 4].try_into().unwrap());
        let oh = rd32(0x4F2) as usize;
        let stub = crate::bios::reset::RESET_STUB_VA as u32;
        assert_eq!(rd32(oh + 0x04), stub, "reseth");
        assert_eq!(rd32(oh + 0x10), stub, "os_res1");
        assert_eq!(rd32(oh) & 0xFFFF, 0, "os_entry");
        assert_eq!(rd32(oh) >> 16, OS_VERSION as u32, "os_version untouched");
        // TeraDesk's LP64-fixed read `*(uint32_t *)(os_start + 0x10)` lands
        // on the first stub instruction: `mov x0, #2021`.
        let at = (stub as u64 - mmu::STUB_PAGE_VA) as usize;
        assert_eq!(u32::from_le_bytes(stubs[at..at + 4].try_into().unwrap()), 0xD280_FCA0);
    }

    /// A data segment with no BSS (`memsz == filesz`) gives `p_blen = 0`
    /// with `p_bbase` still the end of the data — the shape a program
    /// without uninitialised data has, and what the pre-T05 orphan
    /// `.init_array` placement accidentally produced for every program.
    #[test]
    fn data_without_bss_has_zero_blen() {
        let (tbase, tlen, dbase, dlen, bbase, blen) = fields_of(&[
            (PF_R | PF_X, RESERVED_LOW_END, 0x400, 0x400),
            (PF_R | PF_W, RESERVED_LOW_END + 0x400, 0x80, 0x80),
        ]);
        assert_eq!(dbase, tbase + tlen);
        assert_eq!((bbase, blen), (dbase + dlen, 0));
    }
}
