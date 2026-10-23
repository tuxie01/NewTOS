//! **SYSINFO01** — `Ssystem` 2003–2019 at the GEMDOS trap boundary: the
//! effective system settings a system-information tool (the SYSACC desk
//! accessory) shows; **KEYOWNER** added 2020, the input pump mode.
//!
//! Spec: `docs/api/gemdos/154-ssystem.md` §"System-information modes
//! (2003–2020)". Pinned here, above the [`Host`] boundary and on the
//! `tos-api` modules that hold the rest of the state:
//!
//! 1. every scalar mode answers in the call's own LONG (`arg1 == 0`) and
//!    through the caller's LONG (`arg1 != 0` → `E_OK`);
//! 2. a fact the host does not know is `ERROR`, and nothing is written;
//! 3. `arg2 != 0` is `EINVAL` for every scalar mode, before anything is read;
//! 4. the kernel-module states (memory protection, ASIDs, block cache, pools)
//!    are reported as switched, including the "strict only with protection"
//!    and "ASID test mode" rules;
//! 5. 2019 copies with FreeMiNT's `strncpy_f` rule (`len - 1` characters, NUL
//!    always, `len <= 0` writes nothing);
//! 6. 2020 reports the input pump (0 timer pump, 1 `input=aes`);
//! 7. 2023 is not a mode (`EINVFN`); 2021 is the RST01 reset entry (`bios_reset.rs`),
//!    2022 the USB snapshot (`gemdos_ssystem_usbsnap.rs`) — on this host, which
//!    keeps the `Host::usb_snapshot` default, it answers `ERROR` (no controller).

use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU8, Ordering};
use std::sync::{Mutex, MutexGuard, Once};

use tos_api::bdos::process::{memprot, mmu};
use tos_api::host::{CpuClockPolicy, Host, InputMode, KeyboardLayout, ScreenInfo, SdCardMode};
use tos_api::trap::{TrapArgs, TrapClass};

const SSYSTEM: u16 = 0x154;
const S_NEWTOS_CPUMEAS: u64 = 2003;
const S_NEWTOS_CPUPOLICY: u64 = 2004;
const S_NEWTOS_MEMPROT: u64 = 2005;
const S_NEWTOS_ASID: u64 = 2006;
const S_NEWTOS_SDMODE: u64 = 2007;
const S_NEWTOS_SDBUS: u64 = 2008;
const S_NEWTOS_BCACHE: u64 = 2009;
const S_NEWTOS_BCACHEPCT: u64 = 2010;
const S_NEWTOS_STRAM: u64 = 2011;
const S_NEWTOS_TTRAM: u64 = 2012;
const S_NEWTOS_SCRWIDTH: u64 = 2013;
const S_NEWTOS_SCRHEIGHT: u64 = 2014;
const S_NEWTOS_SCRDEPTH: u64 = 2015;
const S_NEWTOS_GPUMEM: u64 = 2016;
const S_NEWTOS_KEYBOARD: u64 = 2017;
const S_NEWTOS_SERIAL: u64 = 2018;
const S_NEWTOS_KVERSION: u64 = 2019;
const S_NEWTOS_INPUTMODE: u64 = 2020;

const E_OK: i32 = 0;
const ERROR: i32 = -1;
const EINVAL: i32 = -25;
const EINVFN: i32 = -32;

/// The scalar modes whose answer comes from the host.
const HOST_MODES: [u64; 12] = [
    S_NEWTOS_CPUMEAS,
    S_NEWTOS_CPUPOLICY,
    S_NEWTOS_SDMODE,
    S_NEWTOS_SDBUS,
    S_NEWTOS_SCRWIDTH,
    S_NEWTOS_SCRHEIGHT,
    S_NEWTOS_SCRDEPTH,
    S_NEWTOS_GPUMEM,
    S_NEWTOS_KEYBOARD,
    S_NEWTOS_SERIAL,
    S_NEWTOS_KVERSION,
    S_NEWTOS_INPUTMODE,
];

/// Does the stand-in host know its facts? One switch for all of them, so the
/// present and the absent case run in one process.
static KNOWS: AtomicBool = AtomicBool::new(true);
/// The measured clock it reports, Hz.
static MEASURED_HZ: AtomicI64 = AtomicI64::new(1_499_998_000);
/// The input pump it reports, `InputMode as u8`.
static PUMP: AtomicU8 = AtomicU8::new(InputMode::TimerPump as u8);

static RIG: Mutex<()> = Mutex::new(());

/// A board whose kernel knows every fact (`KNOWS`), or none of them.
struct InfoHost;

impl Host for InfoHost {
    fn console_write_byte(&self, _byte: u8) {}

    fn screen(&self) -> Option<ScreenInfo> {
        KNOWS.load(Ordering::Relaxed).then_some(ScreenInfo {
            base: 0x10_0000,
            phys_base: 0x10_0000,
            mem_base: 0x10_0000,
            mem_bytes: 1920 * 1080 * 4,
            width: 1920,
            height: 1080,
            bpp: 32,
            stride_bytes: 1920 * 4,
            size_bytes: 1920 * 1080 * 4,
        })
    }

    fn cpu_clock_measured_hz(&self) -> Option<u32> {
        KNOWS.load(Ordering::Relaxed).then(|| MEASURED_HZ.load(Ordering::Relaxed) as u32)
    }

    fn cpu_clock_policy(&self) -> Option<CpuClockPolicy> {
        KNOWS.load(Ordering::Relaxed).then_some(CpuClockPolicy::MaxRefused)
    }

    fn sd_card_mode(&self) -> Option<SdCardMode> {
        KNOWS.load(Ordering::Relaxed).then_some(SdCardMode { compat: false, bus_width: 4 })
    }

    fn gpu_memory_bytes(&self) -> Option<u64> {
        KNOWS.load(Ordering::Relaxed).then_some(76 * 1024 * 1024)
    }

    fn keyboard_layout(&self) -> Option<KeyboardLayout> {
        KNOWS.load(Ordering::Relaxed).then_some(KeyboardLayout::BuiltinGerman)
    }

    fn serial_console(&self) -> Option<bool> {
        KNOWS.load(Ordering::Relaxed).then_some(false)
    }

    fn os_version(&self) -> Option<&'static str> {
        KNOWS.load(Ordering::Relaxed).then_some("NewTOS 0.6.1")
    }

    fn input_mode(&self) -> Option<InputMode> {
        KNOWS.load(Ordering::Relaxed).then(|| {
            if PUMP.load(Ordering::Relaxed) == InputMode::Aes as u8 {
                InputMode::Aes
            } else {
                InputMode::TimerPump
            }
        })
    }
}

static HOST: Once = Once::new();

fn rig(knows: bool) -> MutexGuard<'static, ()> {
    let g = RIG.lock().unwrap_or_else(|e| e.into_inner());
    HOST.call_once(|| {
        let slot: &'static &'static dyn Host = Box::leak(Box::new(&InfoHost as &'static dyn Host));
        tos_api::host::install_host(slot);
    });
    KNOWS.store(knows, Ordering::Relaxed);
    g
}

fn gemdos(a0: u64, a1: u64, a2: u64) -> i32 {
    let args = TrapArgs {
        regs: [a0, a1, a2, 0, 0, 0, 0, 0],
        func: SSYSTEM as u64,
        svc_imm: TrapClass::Gemdos as u16,
        space: tos_api::uaccess::Space::Kernel,
    };
    tos_api::gemdos::dispatch(&args)
}

/// One scalar mode, both forms; they must agree. Returns the value.
fn both_forms(mode: u64) -> i32 {
    let direct = gemdos(mode, 0, 0);
    let mut out: i32 = 0x5A5A_5A5A;
    assert_eq!(gemdos(mode, &mut out as *mut i32 as u64, 0), E_OK, "mode {mode}: pointer form");
    assert_eq!(out, direct, "mode {mode}: pointer form and returned LONG agree");
    direct
}

#[test]
fn host_facts_come_back_in_both_forms() {
    let _g = rig(true);
    assert_eq!(both_forms(S_NEWTOS_CPUMEAS), 1_499_998_000, "measured clock, Hz");
    assert_eq!(both_forms(S_NEWTOS_CPUPOLICY), 2, "cpuclk=max refused by the firmware");
    assert_eq!(both_forms(S_NEWTOS_SDMODE), 0, "fast mode");
    assert_eq!(both_forms(S_NEWTOS_SDBUS), 4, "4-bit bus negotiated");
    assert_eq!(both_forms(S_NEWTOS_SCRWIDTH), 1920);
    assert_eq!(both_forms(S_NEWTOS_SCRHEIGHT), 1080);
    assert_eq!(both_forms(S_NEWTOS_SCRDEPTH), 32);
    assert_eq!(both_forms(S_NEWTOS_GPUMEM), 76 * 1024, "VideoCore memory in KiB");
    assert_eq!(both_forms(S_NEWTOS_KEYBOARD), 1, "built-in German");
    assert_eq!(both_forms(S_NEWTOS_SERIAL), 0, "serial console off");
}

#[test]
fn unknown_host_facts_are_error_and_write_nothing() {
    let _g = rig(false);
    for mode in HOST_MODES {
        if mode == S_NEWTOS_KVERSION {
            continue; // string form, below
        }
        assert_eq!(gemdos(mode, 0, 0), ERROR, "mode {mode}: unknown -> ERROR, no invented number");
        let mut out: i32 = 0x1234_5678;
        assert_eq!(gemdos(mode, &mut out as *mut i32 as u64, 0), ERROR, "mode {mode}");
        assert_eq!(out, 0x1234_5678, "mode {mode}: nothing written");
    }
    let mut buf = [0xEEu8; 8];
    assert_eq!(gemdos(S_NEWTOS_KVERSION, buf.as_mut_ptr() as u64, 8), ERROR);
    assert_eq!(buf, [0xEE; 8], "no version, nothing written");
}

#[test]
fn arg2_is_refused_for_every_scalar_mode_before_anything_is_read() {
    let _g = rig(true);
    for mode in (2003..=2018u64).chain([S_NEWTOS_INPUTMODE]) {
        let mut out: i32 = 0x0BAD_F00D;
        assert_eq!(gemdos(mode, &mut out as *mut i32 as u64, 1), EINVAL, "mode {mode}: arg2 != 0");
        assert_eq!(out, 0x0BAD_F00D, "mode {mode}: nothing written");
    }
}

#[test]
fn memory_protection_and_asids_report_the_switch_in_effect() {
    let _g = rig(true);
    // SAFETY: this binary seeds no pools under these switches; the switch is
    // only read back.
    unsafe { memprot::configure(false, false) };
    assert_eq!(both_forms(S_NEWTOS_MEMPROT), 0, "memprot=off");
    unsafe { memprot::configure(true, false) };
    assert_eq!(both_forms(S_NEWTOS_MEMPROT), 1, "on (default)");
    unsafe { memprot::configure(true, true) };
    assert_eq!(both_forms(S_NEWTOS_MEMPROT), 2, "memprot=strict");
    unsafe { memprot::configure(false, true) };
    assert_eq!(both_forms(S_NEWTOS_MEMPROT), 0, "strict without protection is off");

    assert_eq!(both_forms(S_NEWTOS_ASID), 0, "ASIDs off until configured");
    // SAFETY: no user table exists in this binary.
    unsafe { mmu::configure_asids(mmu::ASID_TEST_LIMIT, 0x7000) };
    assert_eq!(both_forms(S_NEWTOS_ASID), 2, "asid=tiny: test mode");
    unsafe { mmu::configure_asids(1 << 16, 0x7000) };
    assert_eq!(both_forms(S_NEWTOS_ASID), 1, "asid=on, 16-bit ASIDs");
}

#[test]
fn block_cache_reports_its_arena_and_share_or_off() {
    let _g = rig(true);
    tos_api::bios::bcache::uninstall();
    assert_eq!(both_forms(S_NEWTOS_BCACHE), 0, "no cache: 0 KiB = off");
    assert_eq!(gemdos(S_NEWTOS_BCACHEPCT, 0, 0), ERROR, "no cache, no share");

    const BYTES: usize = 2 << 20;
    let arena: &'static mut [u64] = Box::leak(vec![0u64; BYTES / 8].into_boxed_slice());
    // SAFETY: leaked, the cache's for the life of the binary.
    unsafe { tos_api::bios::bcache::install(arena.as_mut_ptr().cast(), BYTES, 7) }.expect("installs");
    assert_eq!(both_forms(S_NEWTOS_BCACHE), 2048, "the arena, KiB (not the slot data)");
    assert_eq!(both_forms(S_NEWTOS_BCACHEPCT), 7);

    tos_api::bios::bcache::uninstall();
    assert_eq!(both_forms(S_NEWTOS_BCACHE), 0);
    assert_eq!(gemdos(S_NEWTOS_BCACHEPCT, 0, 0), ERROR);
}

#[test]
fn pool_sizes_are_error_before_init_then_kib() {
    let _g = rig(true);
    if !tos_api::bdos::mem::init::is_initialised() {
        assert_eq!(gemdos(S_NEWTOS_STRAM, 0, 0), ERROR, "no GEMDOS memory yet");
        assert_eq!(gemdos(S_NEWTOS_TTRAM, 0, 0), ERROR);
        // SAFETY: once per binary; the host test never touches the bytes.
        unsafe { tos_api::bdos::mem::init::init(0x0040_0000, 10 << 20, 0x0100_0000, 64 << 20) }
            .expect("pools");
    }
    assert_eq!(both_forms(S_NEWTOS_STRAM), 10 * 1024);
    assert_eq!(both_forms(S_NEWTOS_TTRAM), 64 * 1024);
}

#[test]
fn version_string_follows_strncpy_f() {
    let _g = rig(true);
    let full = format!("NewTOS 0.6.1 {}", tos_api::build::BUILD_STAMP);
    assert!(full.len() < 64, "documented: shorter than 64 bytes");

    let mut buf = [0xEEu8; 64];
    assert_eq!(gemdos(S_NEWTOS_KVERSION, buf.as_mut_ptr() as u64, 64), E_OK);
    let n = buf.iter().position(|&b| b == 0).expect("NUL-terminated");
    assert_eq!(&buf[..n], full.as_bytes());

    let mut small = [0xEEu8; 8];
    assert_eq!(gemdos(S_NEWTOS_KVERSION, small.as_mut_ptr() as u64, 7), E_OK);
    assert_eq!(&small[..7], b"NewTOS\0", "len - 1 characters and a NUL");
    assert_eq!(small[7], 0xEE, "nothing past len");

    let mut none = [0xEEu8; 4];
    assert_eq!(gemdos(S_NEWTOS_KVERSION, none.as_mut_ptr() as u64, 0), E_OK, "len 0");
    assert_eq!(gemdos(S_NEWTOS_KVERSION, none.as_mut_ptr() as u64, (-5i64) as u64), E_OK);
    assert_eq!(none, [0xEE; 4], "len <= 0 writes nothing");
    assert_eq!(gemdos(S_NEWTOS_KVERSION, 0, 16), EINVAL, "null buffer");
}

#[test]
fn input_mode_reports_the_pump_in_effect() {
    let _g = rig(true);
    PUMP.store(InputMode::TimerPump as u8, Ordering::Relaxed);
    assert_eq!(both_forms(S_NEWTOS_INPUTMODE), 0, "timer pump: the pointer moves while busy");
    PUMP.store(InputMode::Aes as u8, Ordering::Relaxed);
    assert_eq!(both_forms(S_NEWTOS_INPUTMODE), 1, "input=aes");
    PUMP.store(InputMode::TimerPump as u8, Ordering::Relaxed);
}

/// The system-information block ends at 2020; 2021 is the RST01 reset entry
/// (`S_NEWTOS_RESET`, tested in `bios_reset.rs`), 2022 the USB snapshot
/// (`docs/api/usb/usb-snapshot.md`, tested in `gemdos_ssystem_usbsnap.rs`), so
/// 2023 is the first free number. A host that keeps the `usb_snapshot`
/// default — this one — has no USB controller: a well-formed 2022 request is
/// `ERROR` and leaves the buffer as it was.
#[test]
fn the_block_ends_at_2020_and_2023_is_the_first_free_number() {
    let _g = rig(true);
    assert_eq!(gemdos(2023, 0, 0), EINVFN);
    let mut req = [0u8; 64];
    req[0..4].copy_from_slice(b"USBS");
    req[4] = 2; // layout version 2 (usb-snapshot.md §10)
    req[8] = 1; // op LIST
    let before = req;
    assert_eq!(gemdos(2022, req.as_mut_ptr() as u64, 64), ERROR);
    assert_eq!(req, before);
}
