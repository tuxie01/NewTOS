//! TASK/FS22 — the kernel's own file-system paths meet a media change and
//! restart after the re-login, as the GEMDOS dispatcher does since FS20.
//!
//! Spec: `docs/api/gemdos/_state.md` §4a. EmuTOS (`references/emutos`, pin
//! `1a7484ab`): the AES, the desktop and the program loader read files through
//! `trap #1` (`dos_open`, `dos_sfirst`, `Pexec`, `include/gemdos.h:55-100`,
//! `util/miscasm.S:21-27,50-66`), so every one of those reads runs inside
//! `osif` and is restarted after a media change (`bdos/bdosmain.c:449-486`);
//! `Pexec` frees what its load holds before the restart (`bdos/proc.c:319-332`).
//! NewTOS's kernel reads beside the dispatcher (`bdos::fs::read_file`, the
//! directory listings, `Pexec`'s load), so before FS22 each failed on a
//! pending change until the next GEMDOS call on the drive logged it in.
//!
//! The change is armed as in the probe `tosdump/probes/gcprobe.c` (A03/A04):
//! logical sector 0 read and written back unchanged with `RW_NOMEDIACH`. The
//! image is left as it was by every test.

mod fatvol;

use std::sync::Once;

use fatvol::*;
use tos_api::bdos::process::loader::EnvSpec;
use tos_api::bdos::process::pexec::{self, PexecError, PE_LOAD};
use tos_api::bdos::process::{self, standin};
use tos_api::trap::{TrapArgs, TrapClass};

const C: u64 = 2;
const DFREE: u16 = 0x36;
const E_CHNG: i32 = -14;
const EPLFMT: i32 = -66;

const RW_READ_NOMEDIACH: u64 = 2;
const RW_WRITE_NOMEDIACH: u64 = 3;

fn bios(func: u64, regs: [u64; 8]) -> i64 {
    tos_api::bios::dispatch(&TrapArgs {
        regs,
        func,
        svc_imm: TrapClass::Bios as u16,
        space: tos_api::uaccess::Space::Kernel,
    })
}

fn mediach(drive: u64) -> i64 {
    bios(0x09, [drive, 0, 0, 0, 0, 0, 0, 0])
}

fn getbpb(drive: u64) -> i64 {
    bios(0x07, [drive, 0, 0, 0, 0, 0, 0, 0])
}

/// `Rwabs(flag, buf, 1, 0, C:, 0)` — logical sector 0.
fn rwabs_sector0(flag: u64, buf: &mut [u8; SECTOR_BYTES]) -> i64 {
    bios(0x04, [flag, buf.as_mut_ptr() as u64, 1, 0, C, 0, 0, 0])
}

/// Read logical sector 0 and write it back unchanged, both with
/// `RW_NOMEDIACH`: the write arms `forcechange`, `Mediach` answers 2.
fn arm_forcechange() {
    let mut boot = [0u8; SECTOR_BYTES];
    assert_eq!(rwabs_sector0(RW_READ_NOMEDIACH, &mut boot), 0, "read sector 0");
    assert_eq!(rwabs_sector0(RW_WRITE_NOMEDIACH, &mut boot), 0, "write it back");
    assert_eq!(mediach(C), 2, "Mediach after the sector-0 write");
}

fn dfree() -> i32 {
    let mut buf = [0u32; 4];
    gemdos2(DFREE, buf.as_mut_ptr() as u64, 3)
}

static SETUP: Once = Once::new();

/// The file-system lock, C: logged in and settled (no pending change), and a
/// stand-in `run` PD.
fn settled() -> (std::sync::MutexGuard<'static, ()>, &'static DiskHost) {
    let (g, h) = fs();
    SETUP.call_once(|| {
        // SAFETY: once per binary, before any program; boot drive C: (2).
        assert!(unsafe { standin::init(2) }.is_some());
    });
    process::set_current(core::ptr::null_mut());
    assert_ne!(getbpb(C), 0, "Getbpb(C:)");
    assert_eq!(mediach(C), 0, "C: settled");
    assert_eq!(dfree(), E_OK, "Dfree(C:) logs C: in");
    (g, h)
}

// ── bdos::fs::read_file: the AES resources, shel_find, NEWDESK.INF, KEYBOARD.KBD ──

#[test]
fn read_file_meets_the_change_and_restarts_after_the_relogin() {
    let (_g, _h) = settled();
    let mut out = [0u8; 16];
    assert_eq!(tos_api::bdos::fs::read_file("C:\\HELLO.TXT", &mut out), Some(5));
    arm_forcechange();
    // Before FS22: None — the directory buffer's media test met the change
    // and nobody logged C: in again.
    let mut out = [0u8; 16];
    assert_eq!(
        tos_api::bdos::fs::read_file("C:\\HELLO.TXT", &mut out),
        Some(5),
        "read_file after the arming"
    );
    assert_eq!(&out[..5], b"hello");
    assert_eq!(mediach(C), 0, "the re-login's Getbpb cleared the change");
}

// ── the directory listings: file selector, scrap, AUTO, accessories ──

#[test]
fn list_dir_ex_meets_the_change_and_restarts_after_the_relogin() {
    let (_g, _h) = settled();
    arm_forcechange();
    let mut ents = [tos_api::bdos::fs::find::DirInfo::default(); 8];
    let n = tos_api::bdos::fs::find::list_dir_ex("C:\\HELLO.TXT", &mut ents);
    assert_eq!(n, 1, "list_dir_ex after the arming");
    assert_eq!(&ents[0].name[..10], b"HELLO.TXT\0");
    assert_eq!(ents[0].size, 5);
    assert_eq!(mediach(C), 0);
}

#[test]
fn list_dir_meets_the_change_and_restarts_after_the_relogin() {
    let (_g, _h) = settled();
    arm_forcechange();
    let mut names = [[0u8; 16]; 8];
    let mut attrs = [0u8; 8];
    let n = tos_api::bdos::fs::find::list_dir("C:\\*.*", &mut names, &mut attrs);
    let mut got: Vec<String> = names[..n]
        .iter()
        .map(|b| {
            let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
            String::from_utf8_lossy(&b[..end]).into_owned()
        })
        .collect();
    got.sort();
    assert_eq!(got, ["HELLO.TXT", "SUB"], "list_dir after the arming");
    assert_eq!(mediach(C), 0);
}

// ── Pexec's load (xexec) ──

#[test]
fn pexec_meets_the_change_in_its_file_lookup_and_restarts() {
    let (_g, _h) = settled();
    arm_forcechange();
    // `C:\HELLO.TXT` is no program: once the file is found, the loader
    // refuses its header (EPLFMT) — the answer EmuTOS' restarted `xexec`
    // gives too. Before FS22 the lookup met the change and Pexec answered
    // EFILNF, the file "not found".
    let p = b"C:\\HELLO.TXT\0";
    // SAFETY: test binary, single file-system user under the lock.
    let e = unsafe { pexec::dispatch(PE_LOAD, p, 0, 0, &[], EnvSpec::None) }.unwrap_err();
    assert_eq!(e.errno(), EPLFMT, "Pexec after the arming: {e:?}");
    assert_eq!(mediach(C), 0, "the re-login cleared the change");
}

#[test]
fn pexec_reports_e_chng_when_the_relogin_refuses_the_drive() {
    let (_g, h) = settled();
    let boot_lba = part_lba(0);
    let orig = h.read_sector(boot_lba);

    // Cluster size 0: Getbpb refuses the boot sector (bios/blkdev.c:651).
    let mut boot = orig;
    assert_eq!(rwabs_sector0(RW_READ_NOMEDIACH, &mut boot), 0);
    boot[0x0D] = 0;
    assert_eq!(rwabs_sector0(RW_WRITE_NOMEDIACH, &mut boot), 0);
    assert_eq!(mediach(C), 2);

    // osif: Getbpb NULL -> E_CHNG to the caller (bdos/bdosmain.c:471-476).
    let p = b"C:\\HELLO.TXT\0";
    // SAFETY: as above.
    let e = unsafe { pexec::dispatch(PE_LOAD, p, 0, 0, &[], EnvSpec::None) }.unwrap_err();
    assert_eq!(e, PexecError::MediaChange);
    assert_eq!(e.errno(), E_CHNG);

    // Restore: the boot sector back by hand, then a log-in.
    h.write_sector(boot_lba, &orig);
    assert_ne!(getbpb(C), 0);
    assert_eq!(dfree(), E_OK);
}

// ── the bound on the restart ──

#[test]
fn the_restart_gives_up_with_e_chng_after_restart_limit_restarts() {
    let (_g, _h) = settled();
    // An operation that meets a fresh change on every run — the case EmuTOS
    // loops on for ever: each run re-arms forcechange and looks a file up on
    // C: (`FileImage::open`, the bare lookup without a restart of its own).
    let mut runs = 0u32;
    let r = tos_api::gemdos::restartable(|| {
        runs += 1;
        arm_forcechange();
        tos_api::bdos::fs::FileImage::open("C:\\HELLO.TXT").is_some()
    });
    assert_eq!(r, Err(E_CHNG), "the restart reports the change");
    assert_eq!(runs, tos_api::gemdos::RESTART_LIMIT + 1, "one run plus RESTART_LIMIT restarts");
    // The last change was still met and the drive logged in again before the
    // helper gave up: nothing is pending, the drive works.
    assert_eq!(mediach(C), 0);
    let mut out = [0u8; 16];
    assert_eq!(tos_api::bdos::fs::read_file("C:\\HELLO.TXT", &mut out), Some(5));
}

#[test]
fn a_run_without_a_change_is_not_restarted() {
    let (_g, _h) = settled();
    let mut runs = 0u32;
    let r = tos_api::gemdos::restartable(|| {
        runs += 1;
        7
    });
    assert_eq!(r, Ok(7));
    assert_eq!(runs, 1);
}
