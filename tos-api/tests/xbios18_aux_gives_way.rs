//! TASK/XBIOS18 — the AUX wait gives way to a `kbdvec` hook.
//!
//! On qemu-virt AUX (BIOS device 1) is a driver that waits for a serial byte
//! (`kernel/src/console.rs` `AuxSerial`, XBIOS15); since XBIOS18 its trap
//! entry `bconin_restartable` returns `None` while the running process has
//! bytes queued for its `kbdvec` hook, as the CON wait does (KEYS02,
//! `docs/api/xbios/22-kbdvbase.md` C3 "O1"; `docs/api/bios/02-bconin.md`,
//! XBIOS18 note). The kernel driver cannot run on the host; this pins the
//! part above it: `Bconin(1)` and `Cauxin` (EmuTOS `xauxin` = `Bconin(1)`,
//! `bdos/console.c:432-435`) call the device entry's `bconin_restartable`,
//! and when it gives way they ask the trap handler for a restart
//! (`xbios::kbdvecs::request_restart`) and return a placeholder; the
//! re-executed call returns the byte. Measured end to end with
//! `tosdump/probes/axprobe.c`.

use std::sync::atomic::{AtomicU32, Ordering};

use tos_api::bios::chardev::{install_device, CharDevice, DEV_AUX};
use tos_api::host::{install_host, Host};
use tos_api::trap::{TrapArgs, TrapClass};
use tos_api::xbios::kbdvecs::take_restart;

/// The GEMDOS dispatcher needs a host (no host = `EUNDEV`); `Cauxin` uses
/// nothing of it but its presence.
struct NullHost;

impl Host for NullHost {
    fn console_write_byte(&self, _byte: u8) {}
}

static HOST: NullHost = NullHost;
static HOST_REF: &dyn Host = &HOST;

/// A waiting AUX device: its trap entry gives way `give_way` times, then
/// returns `byte`; the non-restartable read returns `byte` at once.
struct WaitingAux {
    give_way: AtomicU32,
    byte: AtomicU32,
}

impl CharDevice for WaitingAux {
    fn bconstat(&self) -> i32 {
        0
    }
    fn bconin(&self) -> i32 {
        self.byte.load(Ordering::SeqCst) as i32
    }
    fn bconin_restartable(&self) -> Option<i32> {
        if self.give_way.load(Ordering::SeqCst) > 0 {
            self.give_way.fetch_sub(1, Ordering::SeqCst);
            return None;
        }
        Some(self.bconin())
    }
    fn bconout(&self, _byte: u8) -> i32 {
        1
    }
    fn bcostat(&self) -> i32 {
        -1
    }
}

static AUX: WaitingAux = WaitingAux {
    give_way: AtomicU32::new(0),
    byte: AtomicU32::new(0),
};
static AUX_REF: &dyn CharDevice = &AUX;

const BCONIN: u16 = 0x02;
const CAUXIN: u16 = 0x03;

fn trap(class: TrapClass, func: u16, a0: u64) -> i64 {
    let args = TrapArgs {
        regs: [a0, 0, 0, 0, 0, 0, 0, 0],
        func: func as u64,
        svc_imm: class as u16,
        space: tos_api::uaccess::Space::Kernel,
    };
    match class {
        TrapClass::Bios => tos_api::bios::dispatch(&args),
        TrapClass::Gemdos => i64::from(tos_api::gemdos::dispatch(&args)),
        _ => unreachable!("only BIOS and GEMDOS calls here"),
    }
}

/// `Bconin(1)` and `Cauxin` give way once (restart requested, placeholder
/// result), then return the byte on the re-executed call without a further
/// restart request.
#[test]
fn bconin1_and_cauxin_give_way_then_return_the_byte() {
    install_host(&HOST_REF);
    let prev = install_device(DEV_AUX, &AUX_REF).expect("DEV_AUX is a device table slot");
    let _ = take_restart();

    for (class, func, what) in [
        (TrapClass::Bios, BCONIN, "Bconin(1)"),
        (TrapClass::Gemdos, CAUXIN, "Cauxin"),
    ] {
        AUX.byte.store(0x51, Ordering::SeqCst);
        AUX.give_way.store(1, Ordering::SeqCst);
        let r = trap(class, func, 1);
        assert!(
            take_restart(),
            "{what}: a read that gave way requests the restart"
        );
        assert_eq!(r, 0, "{what}: the placeholder the trap handler discards");
        let r = trap(class, func, 1);
        assert!(!take_restart(), "{what}: the re-executed read completes");
        assert_eq!(r, 0x51, "{what}: the byte, zero-extended");
    }

    install_device(DEV_AUX, prev);
}
