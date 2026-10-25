//! Desk-accessory loading — EmuTOS `count_accs`/`load_one_acc`/`load_accs`/
//! `free_accs` (`aes/geminit.c:236-315`), the kernel-side half of the `.NAC`
//! lifecycle (ACC02).
//!
//! Spec: [`docs/api/aes/accessories.md`] §1 (the six phases), §2 (the boot
//! flag), §7 (what the `.NAC` extension costs and the two search locations).
//! The AES-side half — the accessory process list and `wait_for_accs` — is
//! [`tos_api::aes::acc`]; the menu half is `tos_api::aes::menu`.
//!
//! ## What runs when
//!
//! EmuTOS separates **loading** from **running**, and so does this module:
//!
//! * [`load_accs`] runs at `run_accs_and_desktop`'s `load_accs(num_accs)`
//!   (`geminit.c:687`) — before `sh_tographic`. Each `.NAC` file is loaded
//!   into a process slot and the slot is put in the cooperative run-set
//!   (EmuTOS `pstart(gotopgm, …)`, `:255`). **No accessory code runs yet.**
//! * [`wait_for_accs_mesag`] runs at `dsptch(); wait_for_accs(AP_MESAG);`
//!   (`:736-737`) — after `process_inf2`, immediately before `sh_main`. This
//!   is where the accessories first execute: each runs its `appl_init` +
//!   `menu_register` and blocks in its message wait, at which point the CPU
//!   comes back here. That ordering is EmuTOS's and it matters: an accessory
//!   that opens a VDI workstation does so while the AES owns the screen, not
//!   while some program does.
//! * [`free_accs`] runs after `sh_main` returns (`:741`).
//!
//! ## Two search locations — a NewTOS rule, not EmuTOS's
//!
//! EmuTOS scans exactly one place, `\*.ACC` in the boot drive's root
//! (`geminit.c:272`), so it has no precedence question to answer. NewTOS
//! searches `C:\ACC\*.NAC` **first**, then `C:\*.NAC`, and skips a file name
//! in the root that the folder already provided. Dedupe is by **file name**:
//! the registered menu title only exists once the accessory has run, far too
//! late to decide at scan time (`docs/api/aes/accessories.md` §7.3).
//!
//! The [`NUM_ACCS`] budget is **shared** across both places — `sysconf.h:37-40`
//! is explicit that the load count and the registration count are one pool —
//! and the scan stops at six silently, exactly as `count_accs`'s loop bound
//! does (`geminit.c:275-281`: no break, no error, the 7th match is simply
//! never looked at).

use core::cell::UnsafeCell;

use tos_api::aes::acc::NUM_ACCS;
use tos_api::bdos::fs::find::{list_dir_ex, DirInfo};
use tos_api::bdos::process::{self, set_current, slot, State};

use crate::kprintln;

/// `LEN_ZFNAME` (`include/sysconf.h:53`) — the `NAME.EXT\0` field
/// `count_accs` copies out of the DTA (`geminit.c:280`).
const LEN_ZFNAME: usize = 13;

/// One discovered accessory — EmuTOS's `ACC { LONG addr; char name[]; }`
/// (`geminit.c:104-107`). `addr` is EmuTOS's load address, which it needs for
/// `free_accs`'s `dos_free`; ours is the process slot, which is what
/// [`free_accs`] releases.
#[derive(Clone, Copy)]
struct Acc {
    /// `NAME.EXT\0`, as the directory scan reported it.
    name: [u8; LEN_ZFNAME],
    /// Full path the file was found at (`C:\ACC\NAME.NAC` or `C:\NAME.NAC`),
    /// NUL-terminated.
    path: [u8; 32],
    /// The process slot the image was loaded into, or `usize::MAX` when the
    /// load failed (EmuTOS's `addr = -1L`, `geminit.c:246`).
    slot: usize,
}

impl Acc {
    const EMPTY: Acc = Acc { name: [0; LEN_ZFNAME], path: [0; 32], slot: usize::MAX };
}

/// `static ACC acc[NUM_ACCS]` (`geminit.c:109`) plus `num_accs`
/// (`geminit.c:121`) — the discovered accessories and how many of the entries
/// are filled. A fixed array, as EmuTOS's is: it *is* the enforcement of the
/// budget. Single boot/shutdown path, like the rest of the desktop's state.
struct AccTable(UnsafeCell<([Acc; NUM_ACCS], usize)>);
// SAFETY: touched only from the single-CPU boot / AES-session path.
unsafe impl Sync for AccTable {}
static ACC: AccTable = AccTable(UnsafeCell::new(([Acc::EMPTY; NUM_ACCS], 0)));

/// Borrow the accessory table.
///
/// # Safety
/// Single boot / AES-session path (no re-entrancy).
#[allow(clippy::mut_from_ref)]
unsafe fn table() -> &'static mut ([Acc; NUM_ACCS], usize) {
    // SAFETY: see `AccTable`'s Sync impl.
    unsafe { &mut *ACC.0.get() }
}

/// The two search locations, in precedence order (see the module note).
/// `(directory prefix, wildcard)`; the prefix is what a found name is
/// appended to.
///
/// **The extension lives in the wildcard**, exactly as EmuTOS's does:
/// `count_accs` searches the literal string `"\\*.ACC"` (`geminit.c:272`) and
/// never classifies a name afterwards. The pattern is `*.NAC` instead of
/// `*.ACC` for the reason the function doc gives (ADR-018), and it is the
/// *only* extension test on this path — the boot code does not consult the
/// desktop for it.
const SEARCH: [(&str, &str); 2] = [("C:\\ACC\\", "C:\\ACC\\*.NAC"), ("C:\\", "C:\\*.NAC")];

/// `count_accs()` (`aes/geminit.c:264-284`) — record up to [`NUM_ACCS`]
/// accessory file names, without loading anything.
///
/// ```c
/// if (bootflags & BOOTFLAG_SKIP_AUTO_ACC) return 0;
/// strcpy(D.g_work, "\\*.ACC");
/// for (i = 0; i < NUM_ACCS; i++) {
///     rc = (i == 0) ? dos_sfirst(D.g_work, FA_RO) : dos_snext();
///     if (rc < 0) break;
///     strlcpy(acc[i].name, D.g_dta.d_fname, LEN_ZFNAME);
/// }
/// return i;
/// ```
///
/// Two differences, both named: the pattern is `*.NAC` (ADR-018 — `.ACC` stays
/// reserved for the m68k emulator, and a classic `.ACC`'s *contents* are 68000
/// code this loader could not run whatever it was called,
/// `docs/api/aes/accessories.md` §7.4), and the scan runs over two
/// directories, `C:\ACC\` before the root, sharing the one budget.
///
/// Sub-directories are skipped explicitly: EmuTOS's `FA_RO` search attribute
/// already excludes them (`match()`'s attribute test, `bdos/fsdir.c:1840-1844`:
/// `FA_RO & FA_SUBDIR == 0`), while [`list_dir_ex`] reports them, so the
/// `FA_SUBDIR` test below is this function's stand-in for that attribute —
/// not an extra rule.
fn count_accs(skip: bool) -> usize {
    // `if (bootflags & BOOTFLAG_SKIP_AUTO_ACC) return 0;` (`:268-270`)
    if skip {
        kprintln!("[acc] skipped (Control held on the boot screen)");
        return 0;
    }

    // SAFETY: single boot path, no re-entrancy.
    let (acc, _) = unsafe { table() };
    let mut found = 0usize;
    for (prefix, pattern) in SEARCH {
        if found >= NUM_ACCS {
            break; // the shared budget, silently (`:275`'s loop bound)
        }
        let mut ents = [DirInfo::default(); 32];
        let n = list_dir_ex(pattern, &mut ents);
        for e in &ents[..n] {
            if found >= NUM_ACCS {
                break;
            }
            if e.attrib & 0x10 != 0 {
                continue; // FA_SUBDIR — never an accessory
            }
            let end = e.name.iter().position(|&b| b == 0).unwrap_or(LEN_ZFNAME);
            let name = &e.name[..end];
            // Dedupe by file name across the two locations: a name the
            // `\ACC\` folder already provided is skipped in the root.
            if acc[..found].iter().any(|a| same_name(&a.name, name)) {
                continue;
            }
            let mut e = Acc::EMPTY;
            let keep = end.min(LEN_ZFNAME - 1);
            e.name[..keep].copy_from_slice(&name[..keep]);
            let plen = prefix.len();
            e.path[..plen].copy_from_slice(prefix.as_bytes());
            let take = end.min(e.path.len() - plen - 1);
            e.path[plen..plen + take].copy_from_slice(&name[..take]);
            acc[found] = e;
            found += 1;
        }
    }
    found
}

/// Case-insensitive comparison of two `NAME.EXT\0` fields.
fn same_name(a: &[u8; LEN_ZFNAME], b: &[u8]) -> bool {
    let end = a.iter().position(|&c| c == 0).unwrap_or(LEN_ZFNAME);
    a[..end].eq_ignore_ascii_case(b)
}

/// `load_one_acc(acc)` (`aes/geminit.c:236-257`) — load the file and create a
/// process for it:
///
/// ```c
/// acc->addr = -1L;
/// ret = dos_open(D.s_cmd, ROPEN);
/// if (ret >= 0L) {
///     err_ret = pgmld(handle, D.s_cmd, (LONG **)&acc->addr);
///     dos_close(handle);
///     if (err_ret != -1) pstart(gotopgm, acc->name, acc->addr);
/// }
/// ```
///
/// `pgmld` is the same GEMDOS program loader every `.PRG` uses; ours is
/// `pexec::load` (the native-ELF equivalent, ADR-018). `pstart` creates the
/// process and makes it schedulable without running it — here
/// [`tos_api::aes::acc::register_process`], which records the accessory's AES
/// identity and admits its slot to the cooperative run-set.
///
/// A failed load is EmuTOS's `addr = -1L` case: no process, no diagnostic
/// beyond the serial line, and the boot carries on.
fn load_one_acc(acc: &mut Acc) {
    // SAFETY: single boot path; the BDOS is not concurrently locked and the
    // returned `LoadedChild` borrows nothing past this call. No command tail,
    // and `env == NULL` as EmuTOS's `pgmld` passes it (`Pexec(PE_LOAD, pname,
    // "", NULL)`, `util/gemdos.c:34`): a copy of `run->p_env`, `run` being the
    // accdesk PD (`bdos/proc.c:419-420`; `load_accs` runs in the accdesk
    // process, `aes/geminit.c:687`) — here the accdesk stand-in's environment,
    // a copy of the AES PD's (TASK/PPARENT, corrected by TASK/PROC03). `p_parent`
    // stays the loader's 0: an accessory is never
    // `proc_go`ed (`bdos/proc.c:370,541`), which is how mintlib tells one.
    let child = match unsafe {
        process::pexec::load(&acc.path, &[], process::loader::EnvSpec::Inherit, false, true)
    } {
        Ok(c) => c,
        Err(e) => {
            kprintln!("[acc] {} load failed: {:?}", as_str(&acc.path), e);
            return;
        }
    };
    // `pgmld`'s second half (`util/gemdos.c:40-44`): the loader gave the
    // accessory the largest free block as its TPA (EmuTOS `alloc_tpa`), so
    // shrink it to "code+data+bss lengths plus basepage length" before the
    // next accessory — or anything else — is loaded. In the NewTOS layout the
    // image starts at `PD + 0x1000`, so that length is the image end
    // (`LoadedChild::image_len`), not the sum. A failed shrink is `pgmld`'s
    // `return -1`: the accessory is not started.
    // SAFETY: single boot path, BDOS not concurrently locked; the TPA block
    // starts at the PD pointer the loader returned and is the loader's.
    let rc = unsafe {
        tos_api::bdos::mem::traps::mshrink(
            0,
            child.basepage_va as u32 as tos_types::Long,
            child.image_len as tos_types::Long,
        )
    };
    if rc < 0 {
        kprintln!("[acc] {} Mshrink(PD, {:#x}) failed: {}", as_str(&acc.path), child.image_len, rc);
        // SAFETY: the slot was never started; nothing references its TPA or
        // its environment, both owned by the child's PD (`owned_by_child`).
        unsafe {
            tos_api::bdos::fs::curdir::release_pd(child.handle as *mut tos_types::BasePage);
            tos_api::bdos::mem::free_all_owned::free_all_owned(child.handle as *mut core::ffi::c_void);
            process::pterm::release_slot(child.slot);
        }
        return;
    }
    acc.slot = child.slot;
    let Some(s) = slot(child.slot) else {
        return;
    };
    s.set_state(State::Loaded);
    // `p_nameit(rlr, …)`/`p_setappdir` — the accessory's own directory, so a
    // future `rsrc_load` of an `.RSC` beside the `.NAC` resolves, and its AES
    // descriptor carries its name for `appl_find` (`aes/gempd.c:110-131`).
    tos_api::aes::resource::set_app_dir(&acc.path);
    tos_api::aes::register_process(child.slot, &acc.path);
    // `pstart(gotopgm, acc->name, acc->addr)` (`:255`) — schedulable, not
    // running: the first `dsptch()` ([`wait_for_accs_mesag`]) starts it.
    tos_api::aes::acc::register_process(child.slot);
    kprintln!(
        "[acc] loaded {} into slot {} (entry 0x{:x})",
        as_str(&acc.path),
        child.slot,
        child.entry_va
    );
}

/// `load_accs(n)` (`aes/geminit.c:309-315`) preceded by `count_accs()`
/// (`geminit.c:812`, whose result `gem_main` keeps in `num_accs`): discover up
/// to [`NUM_ACCS`] `.NAC` files and load each into a process slot.
///
/// `skip` is `BOOTFLAG_SKIP_AUTO_ACC` — Control held on the boot screen
/// (`bios/bios.c:819-820`), which suppresses the *discovery*, so nothing
/// downstream has anything to act on (`docs/api/aes/accessories.md` §2).
///
/// Returns the number of accessory processes created.
pub fn load_accs(skip: bool) -> usize {
    let found = count_accs(skip);
    // SAFETY: single boot path, no re-entrancy.
    let (acc, num_found) = unsafe { table() };
    *num_found = found;
    for e in acc[..found].iter_mut() {
        load_one_acc(e);
    }
    let started = tos_api::aes::acc::count();
    if found > 0 {
        kprintln!("[acc] {} file(s) found, {} process(es) started", found, started);
    }
    started
}

/// `dsptch(); wait_for_accs(AP_MESAG);` (`aes/geminit.c:736-737`) — let every
/// loaded accessory run until it has issued its first message wait.
///
/// EmuTOS's `wait_for_accs` calls `all_run()` and lets the dispatcher run the
/// accessories; the barrier is the `AP_MESAG` flag each sets on entering
/// `evnt_mesag`/`evnt_multi` (`gemsuper.c:137,148`). Here the dispatch is a
/// **`Pwait` from the kernel context**: the trap handler parks this EL1 frame,
/// runs the head of the ready list, and returns here when the round robin
/// comes back to the kernel client (ACC03, A115 — it is one entry of that list,
/// EmuTOS's pid 0). With nothing else ready `Pwait` is the no-op it always
/// was, so the loop then degenerates to EmuTOS's bounded spin and gives up
/// after `WAIT_TIMEOUT` passes exactly as EmuTOS does.
///
/// Why an accessory's *first* execution is here and not at load time: this is
/// where EmuTOS puts it, and the reason is stated in `wait_for_accs`'s own
/// comment (`geminit.c:579-583`) — memory an accessory allocates during its
/// initialisation must be owned by the (long-lived) AES, not by whatever
/// program `sh_main` starts next.
pub fn wait_for_accs_mesag() {
    if tos_api::aes::acc::count() == 0 {
        return;
    }
    for _ in 0..tos_api::aes::acc::WAIT_TIMEOUT {
        if tos_api::aes::acc::all_have(tos_api::aes::acc::AP_MESAG) {
            kprintln!("[acc] all accessories have issued their message wait");
            return;
        }
        pwait();
    }
    kprintln!("[acc] wait_for_accs(AP_MESAG): timed out");
}

/// `Pwait` (GEMDOS 0x4D) from the kernel context — EmuTOS's `all_run();
/// dsptch();` for a caller that is not a process. The trap handler parks this
/// frame and gives the CPU to the head of the ready list; it returns when the
/// kernel client's turn comes round again.
fn pwait() {
    // SAFETY: the GEMDOS trap; the handler restores every register except
    // `x0`, which it sets to 0 on the resumed path.
    unsafe {
        core::arch::asm!(
            "svc #1",
            inlateout("x0") 0u64 => _,
            in("x1") 0u64,
            in("x2") 0u64,
            in("x8") 0x4Du64,
            options(nostack),
        );
    }
}

/// `free_accs(n)` (`aes/geminit.c:296-303`) — release every accessory's
/// memory when the AES session ends:
///
/// ```c
/// for (i = 0; i < n; i++)
///     if (acc[i].addr >= 0L) dos_free((void *)acc[i].addr);
/// ```
///
/// followed, in EmuTOS, by the end of the `run_accs_and_desktop` process
/// itself, whose `Pterm` frees everything that process owned
/// (`ixterm` → `free_all_owned(r, &pmd)` / `&pmdalt`, `bdos/proc.c:133-139`):
/// the accessories' environments (and TPAs — `Pexec(PE_LOAD)` makes `run`,
/// the AES process, their owner, `bdos/proc.c:305-307`) and whatever the accessories allocated while the AES was
/// `run` — their start-up stacks and everything else of their initialisation
/// (`wait_for_accs`, `:579-583`). EmuTOS's own comment (`:288-295`) is
/// explicit that `free_accs` is therefore not required for correctness, but
/// that at least one real accessory (Chameleon) needs the explicit free.
///
/// NewTOS has no such process to fall back on: an accessory is a process of
/// its own (its slot, its PD), it owns its TPA and its environment (loaded
/// `owned_by_child`), and its `Malloc`s are its own. So the net effect of
/// EmuTOS's two steps is reproduced per accessory with the memory half of a
/// termination (`pterm::terminate_inner`): `dos_free(acc->addr)` is the TPA,
/// and [`free_all_owned`] of the accessory's PD frees it together with the
/// environment and every block the accessory allocated — what EmuTOS's
/// process end frees. Programs it loaded with `PE_LOAD` and never started go
/// with it ([`release_unstarted_of`]), its AES resource entries too
/// ([`on_process_exit`]); then the slot is released
/// ([`process::pterm::release_slot`]: the AES descriptor, the window/menu
/// property, the page tables). A block another process also holds (ARCH03
/// 2d links) stays allocated for that owner, as on every `Pterm`.
///
/// [`free_all_owned`]: tos_api::bdos::mem::free_all_owned::free_all_owned
/// [`release_unstarted_of`]: process::pexec::release_unstarted_of
/// [`on_process_exit`]: tos_api::aes::resource::on_process_exit
pub fn free_accs() {
    // SAFETY: single restart/shutdown path, no re-entrancy.
    let (acc, num_found) = unsafe { table() };
    for e in acc[..*num_found].iter_mut() {
        if e.slot == usize::MAX {
            continue;
        }
        kprintln!("[acc] freeing {} (slot {})", as_str(&e.path), e.slot);
        tos_api::aes::acc::forget_process(e.slot);
        let pd = process::pd_of(e.slot);
        tos_api::aes::resource::on_process_exit(e.slot);
        // SAFETY: the accessory is not running (we are on the kernel path with
        // no EL0 context live), the BDOS is not concurrently locked, and
        // nothing holds a reference into its memory; `pd` is its PD pointer
        // (null if the slot has no image, which frees nothing).
        let freed = unsafe {
            // The accessory PD's `dirtbl` uses, copied from accdesk when it was
            // loaded (`init_pd_files`), go back before its memory does. EmuTOS
            // never `ixterm`s an accessory and leaks them (an accident, not
            // reproduced: `_run-ownership-curdir.md` §4.5, D-10).
            tos_api::bdos::fs::curdir::release_pd(pd);
            let freed = tos_api::bdos::mem::free_all_owned::free_all_owned(pd as *mut core::ffi::c_void);
            process::pexec::release_unstarted_of(pd);
            process::pterm::release_slot(e.slot);
            freed
        };
        kprintln!("[acc] {} block(s) of {} freed", freed, as_str(&e.path));
        e.slot = usize::MAX;
    }
    *acc = [Acc::EMPTY; NUM_ACCS];
    *num_found = 0;
    set_current(core::ptr::null_mut());
}

/// A NUL-terminated byte string as `&str` for logging (`"?"` if not UTF-8).
fn as_str(s: &[u8]) -> &str {
    let end = s.iter().position(|&b| b == 0).unwrap_or(s.len());
    core::str::from_utf8(&s[..end]).unwrap_or("?")
}
