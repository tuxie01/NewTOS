//! A126 — the keyboard state the AES reports carries only bits 0-3.
//!
//! EmuTOS' AES has one source for it: `gsx_kstate` (`aes/gemgsxif.c:432-436`)
//! is the VDI's `vq_key_s` (`vdi/vdi_input.c:110-113`) = `gshift_s()` =
//! `Kbshift(-1) & 0x000f` (`:183-187`); `chkkbd` keeps it in `kstate`
//! (`aes/gemdisp.c:165,175`, `aes/geminput.c:288-293`), and `ev_rets`
//! (`aes/gemevlib.c:56`: `evnt_button`/`evnt_mouse`/`evnt_multi` `EV_KS`) and
//! `gr_mkstate` (`aes/gemgrlib.c:463`: `graf_mkstate`) report that global.
//! CapsLock (0x10), `MODE_HOME`/`MODE_INSERT` (0x20/0x40) and a program-set
//! bit 7 (0x80), which BIOS `Kbshift` keeps since BIOS10, never reach a
//! program through the AES. The host here holds the BIOS shift byte as the
//! kernel's IKBD decoder does (`kernel/src/host_env.rs`).
//!
//! Spec: `docs/api/aes/event.md` (`EV_KS`), `docs/api/aes/window-graphics.md`
//! (`graf_mkstate`). Measured against the oracle by `tosdump/probes/akprobe.c`
//! (`measurements/measurement-2026-10-30-a126/RUNINFO.TXT`).

use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

use tos_api::bdos::process::Abi;
use tos_api::host::Host;

/// The BIOS shift byte (`shifty`), whole.
static SHIFTY: AtomicU8 = AtomicU8::new(0);

struct KbdHost {
    clock: AtomicU64,
}

impl Host for KbdHost {
    fn console_write_byte(&self, _byte: u8) {}
    fn console_try_read_key(&self) -> Option<u16> {
        None
    }
    fn keyboard_shift_state(&self) -> u8 {
        SHIFTY.load(Ordering::Relaxed)
    }
    fn set_keyboard_shift_state(&self, mask: u8) {
        SHIFTY.store(mask, Ordering::Relaxed);
    }
    fn monotonic_millis(&self) -> u64 {
        self.clock.load(Ordering::Relaxed)
    }
    fn pump_input(&self) {
        self.clock.fetch_add(1, Ordering::Relaxed);
    }
}

static HOST: OnceLock<()> = OnceLock::new();
static LOCK: Mutex<()> = Mutex::new(());

fn setup() -> MutexGuard<'static, ()> {
    let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    HOST.get_or_init(|| {
        let h: &'static KbdHost = Box::leak(Box::new(KbdHost { clock: AtomicU64::new(0) }));
        let slot: &'static &'static dyn Host = Box::leak(Box::new(h as &'static dyn Host));
        tos_api::host::install_host(slot);
    });
    SHIFTY.store(0, Ordering::Relaxed);
    g
}

/// A kernel-space LP64 AES parameter block, as the GEM-trap handler hands it
/// to the dispatcher.
struct Aespb {
    contrl: Box<[i16; 8]>,
    global: Box<[i16; 16]>,
    intin: Box<[i16; 16]>,
    intout: Box<[i16; 16]>,
    addrin: Box<[u64; 4]>,
    addrout: Box<[u64; 4]>,
    block: Box<[u64; 6]>,
}

impl Aespb {
    fn new() -> Aespb {
        let mut pb = Aespb {
            contrl: Box::new([0; 8]),
            global: Box::new([0; 16]),
            intin: Box::new([0; 16]),
            intout: Box::new([0; 16]),
            addrin: Box::new([0; 4]),
            addrout: Box::new([0; 4]),
            block: Box::new([0; 6]),
        };
        pb.block[0] = pb.contrl.as_ptr() as u64;
        pb.block[1] = pb.global.as_ptr() as u64;
        pb.block[2] = pb.intin.as_ptr() as u64;
        pb.block[3] = pb.intout.as_ptr() as u64;
        pb.block[4] = pb.addrin.as_ptr() as u64;
        pb.block[5] = pb.addrout.as_ptr() as u64;
        pb
    }

    fn call(&mut self, opcode: i16, in_len: i16, out_len: i16, ain_len: i16) {
        self.contrl[0] = opcode;
        self.contrl[1] = in_len;
        self.contrl[2] = out_len;
        self.contrl[3] = ain_len;
        self.contrl[4] = 0;
        self.intout.fill(0);
        // SAFETY: `block` holds six live pointers to arrays owned by `self`,
        // each larger than any opcode issued here declares.
        unsafe { tos_api::aes::dispatch(self.block.as_ptr() as u64, Abi::Lp64) };
    }

    /// `graf_mkstate` (79): `int_out[4]` = `kstate`.
    fn graf_mkstate(&mut self) -> i16 {
        self.call(79, 0, 5, 0);
        self.intout[4]
    }

    /// `evnt_button(1, 0x01, 0x00)` (21) with the button up: satisfied at
    /// once (`abutton` → `azombie(e, 1)`). `int_out[4]` = `EV_KS`.
    fn evnt_button_up(&mut self) -> (i16, i16) {
        self.intin.fill(0);
        self.intin[0] = 1;
        self.intin[1] = 0x01;
        self.intin[2] = 0x00;
        self.call(21, 3, 5, 0);
        (self.intout[0], self.intout[4])
    }

    /// `evnt_multi(flags, 1, 0x01, 0x00, …, timer 0)` (25): `RET_CODE` and
    /// `EV_KS`.
    fn evnt_multi(&mut self, flags: i16) -> (i16, i16) {
        let mut buf = Box::new([0i16; 8]);
        self.intin.fill(0);
        self.intin[0] = flags;
        self.intin[1] = 1;
        self.intin[2] = 0x01;
        self.intin[3] = 0x00;
        self.addrin[0] = buf.as_mut_ptr() as u64;
        self.call(25, 16, 7, 1);
        self.addrin[0] = 0;
        (self.intout[0], self.intout[4])
    }
}

const MU_BUTTON: i16 = 0x0002;
const MU_TIMER: i16 = 0x0020;

/// Every value of the BIOS shift byte: the AES reports `byte & 0x0f`.
const BYTES: [u8; 8] = [0x00, 0x08, 0x10, 0x18, 0x1f, 0x48, 0x80, 0xff];

#[test]
fn graf_mkstate_reports_only_the_four_modifier_bits() {
    let _g = setup();
    let mut pb = Aespb::new();
    pb.call(10, 0, 1, 0); // appl_init
    for b in BYTES {
        SHIFTY.store(b, Ordering::Relaxed);
        assert_eq!(
            pb.graf_mkstate(),
            (b & 0x0f) as i16,
            "graf_mkstate kstate for the BIOS shift byte {b:#04x} (gr_mkstate *pkstat = kstate, \
             aes/gemgrlib.c:463; kstate = Kbshift(-1) & 0x0f, vdi/vdi_input.c:183-187)"
        );
    }
}

#[test]
fn caps_lock_and_alt_give_k_alt_in_every_event_return() {
    let _g = setup();
    let mut pb = Aespb::new();
    pb.call(10, 0, 1, 0); // appl_init
    SHIFTY.store(0x18, Ordering::Relaxed); // CapsLock + Alt, the A126 case
    assert_eq!(pb.graf_mkstate(), 0x08, "graf_mkstate: K_ALT, not 0x18");
    let (ret, ks) = pb.evnt_button_up();
    assert_eq!((ret, ks), (1, 0x08), "evnt_button: one click, EV_KS = K_ALT (ev_rets, aes/gemevlib.c:56)");
    let (fired, ks) = pb.evnt_multi(MU_BUTTON | MU_TIMER);
    assert_ne!(fired & (MU_BUTTON | MU_TIMER), 0, "evnt_multi returned an event (fired {fired:#06x})");
    assert_eq!(ks, 0x08, "evnt_multi EV_KS = K_ALT (ev_rets, aes/gemevlib.c:56,268)");
    let (fired, ks) = pb.evnt_multi(MU_TIMER);
    assert_eq!(fired, MU_TIMER, "evnt_multi(MU_TIMER, 0)");
    assert_eq!(ks, 0x08, "evnt_multi MU_TIMER return: EV_KS = K_ALT");
}

#[test]
fn every_event_return_masks_the_upper_four_bits() {
    let _g = setup();
    let mut pb = Aespb::new();
    pb.call(10, 0, 1, 0); // appl_init
    for b in BYTES {
        SHIFTY.store(b, Ordering::Relaxed);
        let want = (b & 0x0f) as i16;
        assert_eq!(pb.evnt_button_up().1, want, "evnt_button EV_KS for {b:#04x}");
        assert_eq!(pb.evnt_multi(MU_TIMER).1, want, "evnt_multi EV_KS for {b:#04x}");
    }
}
