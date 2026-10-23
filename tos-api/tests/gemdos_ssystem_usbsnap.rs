//! **USB01 step 2 (K9, test T3)** — `Ssystem(S_NEWTOS_USBSNAP)` (2022) at the
//! GEMDOS trap boundary.
//!
//! Spec: `docs/api/usb/usb-snapshot.md` §2.2 (header), §2.3 (return values
//! and the order of the checks). Pinned here:
//!
//! 1. `buf == 0`, `len < 32`, a wrong magic → `EINVAL`, nothing written;
//! 2. a wrong version or an unknown op → `EINVAL` with a header that says
//!    `version = 1`, `used_len = 32`, even without a controller (the header
//!    checks come first);
//! 3. no controller (the hostless default and a host whose store has none)
//!    → `ERROR`, nothing written;
//! 4. an index out of range, a MUTE longer than 60 s or with `index != 0` →
//!    `EINVAL` with the header;
//! 5. too small a buffer → `E_OK`, header only, `TRUNCATED`, `total_len` =
//!    the size needed; the repeated call with that size gets everything;
//! 6. a full answer → `E_OK`, header echoing the request, `used_len` ≤ `len`,
//!    `now_hz200` = the tick; REPORTS sets `MORE` when entries are left over;
//! 7. nothing is ever written past `len`.
//!
//! The host here is a stand-in, but its answers come from the kernel's real
//! encoder (`kernel/src/drivers/usb/snapshot.rs`, compiled here by path — it
//! is pure), so the bytes are the ones the kernel produces. The fault side (a
//! bad pointer ends the calling program) needs the MMU; it is driven in QEMU.

use std::sync::{Mutex, MutexGuard, Once};

use tos_api::host::{Host, UsbSnapAnswer, UsbSnapRequest};
use tos_api::trap::{TrapArgs, TrapClass};

#[path = "../../kernel/src/drivers/usb/snapshot.rs"]
#[allow(dead_code)]
mod snapshot;

const SSYSTEM: u16 = 0x154;
const S_NEWTOS_USBSNAP: u64 = 2022;
const E_OK: i32 = 0;
const ERROR: i32 = -1;
const EINVAL: i32 = -25;

const TRUNCATED: u16 = 1;
const MORE: u16 = 2;

/// The kernel's store behind the stand-in host; `None` = a host without a
/// USB stack at all (the trait default).
static STORE: Mutex<Option<Box<snapshot::Store>>> = Mutex::new(None);
static RIG: Mutex<()> = Mutex::new(());
static HOST: Once = Once::new();

struct UsbHost;

impl Host for UsbHost {
    fn console_write_byte(&self, _byte: u8) {}

    fn usb_snapshot(&self, req: &UsbSnapRequest, sink: &mut dyn FnMut(&[u8]) -> bool) -> UsbSnapAnswer {
        let mut g = STORE.lock().unwrap();
        let Some(store) = g.as_mut() else {
            return UsbSnapAnswer::NoController;
        };
        let r = snapshot::Request {
            op: req.op,
            index: req.index,
            arg: req.arg,
            now: req.now_hz200,
            room: req.room,
        };
        match store.answer(r, sink) {
            snapshot::Answer::NoController => UsbSnapAnswer::NoController,
            snapshot::Answer::Invalid => UsbSnapAnswer::Invalid,
            snapshot::Answer::Faulted => UsbSnapAnswer::Faulted,
            snapshot::Answer::Done { total, written, truncated, more } => {
                UsbSnapAnswer::Done { total, written, truncated, more }
            }
        }
    }
}

/// Install the host once; set the store for this test.
fn rig(store: Option<Box<snapshot::Store>>) -> MutexGuard<'static, ()> {
    let g = RIG.lock().unwrap_or_else(|e| e.into_inner());
    HOST.call_once(|| {
        let slot: &'static &'static dyn Host = Box::leak(Box::new(&UsbHost as &'static dyn Host));
        tos_api::host::install_host(slot);
    });
    *STORE.lock().unwrap() = store;
    g
}

/// A controller with a boot keyboard (with a report ring) and a hub.
fn two_devices() -> Box<snapshot::Store> {
    let mut s = Box::new(snapshot::Store::new());
    s.present = true;
    s.root_ports = 4;
    s.generation = 1;
    {
        let d = s.claim(0).unwrap();
        d.slot = 1;
        d.root_port = 1;
        d.vid = 0x0627;
        d.pid = 0x0001;
        d.role = snapshot::role::KEYBOARD;
        d.ring_iface = 0;
        d.ring_ep = 0x81;
        d.device_desc_state = snapshot::Fetch::Ok;
        d.device_desc[0] = 18;
        d.device_desc[1] = 1;
    }
    {
        let d = s.claim(1).unwrap();
        d.slot = 2;
        d.root_port = 2;
        d.role = snapshot::role::HUB;
    }
    for i in 0..5u8 {
        s.record(0, &[0, 0, 4 + i, 0, 0, 0, 0, 0], 1, 100 + i as u32, false);
    }
    s
}

fn call(buf: u64, len: i64) -> i32 {
    let args = TrapArgs {
        regs: [S_NEWTOS_USBSNAP, buf, len as u64, 0, 0, 0, 0, 0],
        func: SSYSTEM as u64,
        svc_imm: TrapClass::Gemdos as u16,
        space: tos_api::uaccess::Space::Kernel,
    };
    tos_api::gemdos::dispatch(&args)
}

/// A request header in a buffer of `len` bytes plus 16 guard bytes.
fn request(len: usize, version: u16, op: u16, index: u32, arg: u32) -> Vec<u8> {
    let mut b = vec![0xEEu8; len + 16];
    b[0..4].copy_from_slice(b"USBS");
    b[4..6].copy_from_slice(&version.to_le_bytes());
    b[6..8].copy_from_slice(&0u16.to_le_bytes());
    b[8..10].copy_from_slice(&op.to_le_bytes());
    b[10..12].copy_from_slice(&0u16.to_le_bytes());
    b[12..16].copy_from_slice(&index.to_le_bytes());
    b[16..20].copy_from_slice(&arg.to_le_bytes());
    b
}

fn ssys(b: &mut [u8], len: usize) -> i32 {
    call(b.as_mut_ptr() as u64, len as i64)
}

struct Hdr {
    version: u16,
    hdr_len: u16,
    op: u16,
    flags: u16,
    index: u32,
    arg: u32,
    total: u32,
    used: u32,
    now: u32,
}

fn hdr(b: &[u8]) -> Hdr {
    let u16le = |o: usize| u16::from_le_bytes([b[o], b[o + 1]]);
    let u32le = |o: usize| u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
    assert_eq!(&b[0..4], b"USBS");
    Hdr {
        version: u16le(4),
        hdr_len: u16le(6),
        op: u16le(8),
        flags: u16le(10),
        index: u32le(12),
        arg: u32le(16),
        total: u32le(20),
        used: u32le(24),
        now: u32le(28),
    }
}

fn guard_intact(b: &[u8], len: usize) {
    assert!(b[len..].iter().all(|&x| x == 0xEE), "nothing written past len");
}

#[test]
fn null_buffer_short_buffer_and_bad_magic_write_nothing() {
    let _g = rig(Some(two_devices()));
    assert_eq!(call(0, 4096), EINVAL, "buf == 0");
    let mut b = request(31, 1, 1, 0, 0);
    let before = b.clone();
    assert_eq!(ssys(&mut b, 31), EINVAL, "len < 32");
    assert_eq!(b, before);
    assert_eq!(call(b.as_mut_ptr() as u64, -1), EINVAL, "negative len");

    let mut b = request(64, 1, 1, 0, 0);
    b[3] = b'X';
    let before = b.clone();
    assert_eq!(ssys(&mut b, 64), EINVAL, "magic");
    assert_eq!(b, before, "nothing written");
}

#[test]
fn wrong_version_and_unknown_op_are_einval_with_header_even_without_controller() {
    for store in [Some(two_devices()), None] {
        let _g = rig(store);
        for (version, op) in [(2u16, 1u16), (0, 1), (1, 0), (1, 5)] {
            let mut b = request(256, version, op, 7, 9);
            assert_eq!(ssys(&mut b, 256), EINVAL, "version {version} op {op}");
            let h = hdr(&b);
            assert_eq!((h.version, h.hdr_len, h.op, h.index, h.arg), (1, 32, op, 7, 9));
            assert_eq!((h.used, h.total, h.flags), (32, 32, 0));
            assert!(b[32..256].iter().all(|&x| x == 0xEE), "only the header");
        }
    }
}

#[test]
fn no_controller_is_error_and_writes_nothing() {
    // A host without a USB stack (the trait default answers NoController).
    let _g = rig(None);
    let mut b = request(256, 1, 1, 0, 0);
    let before = b.clone();
    assert_eq!(ssys(&mut b, 256), ERROR);
    assert_eq!(b, before);
    drop(_g);

    // A host whose controller was not found (raspi4b).
    let _g = rig(Some(Box::new(snapshot::Store::new())));
    for op in 1..=4u16 {
        let mut b = request(256, 1, op, 0, 0);
        let before = b.clone();
        assert_eq!(ssys(&mut b, 256), ERROR, "op {op}");
        assert_eq!(b, before);
    }
}

#[test]
fn bad_index_and_mute_argument_are_einval_with_header() {
    let _g = rig(Some(two_devices()));
    for (op, index, arg) in [(2u16, 2u32, 0u32), (3, 9, 0), (4, 0, 12_001), (4, 1, 100)] {
        let mut b = request(512, 1, op, index, arg);
        assert_eq!(ssys(&mut b, 512), EINVAL, "op {op} index {index} arg {arg}");
        let h = hdr(&b);
        assert_eq!((h.version, h.used, h.op, h.index, h.arg), (1, 32, op, index, arg));
        assert!(b[32..512].iter().all(|&x| x == 0xEE));
    }
}

#[test]
fn list_full_answer_and_too_small_buffer() {
    let _g = rig(Some(two_devices()));
    tos_api::bios::tick::advance(3);
    let now = tos_api::bios::tick::hz_200();
    assert_ne!(now, 0);

    // 32 + 16 + 2 × 32 = 112 bytes needed; offer 100.
    let mut b = request(100, 1, 1, 0, 0);
    assert_eq!(ssys(&mut b, 100), E_OK);
    let h = hdr(&b);
    assert_eq!((h.flags, h.total, h.used, h.now), (TRUNCATED, 112, 32, now));
    assert!(b[32..100].iter().all(|&x| x == 0xEE), "header only");
    guard_intact(&b, 100);

    let mut b = request(h.total as usize, 1, 1, 0, 0);
    assert_eq!(ssys(&mut b, 112), E_OK);
    let h = hdr(&b);
    assert_eq!((h.version, h.hdr_len, h.op, h.flags, h.total, h.used), (1, 32, 1, 0, 112, 112));
    let body = &b[32..112];
    assert_eq!(&body[..16], &[1, 0, 0, 0, 2, 0, 32, 0, 4, 1, 0, 0, 0, 0, 0, 0]);
    assert_eq!(&body[16 + 8..16 + 12], &[0x27, 0x06, 1, 0], "entry 0 VID/PID");
    assert_eq!((body[16 + 14], body[48 + 14]), (1, 4), "roles keyboard, hub");
    guard_intact(&b, 112);
}

#[test]
fn device_answer_matches_the_encoder() {
    let _g = rig(Some(two_devices()));
    let mut b = request(1024, 1, 2, 0, 0);
    assert_eq!(ssys(&mut b, 1024), E_OK);
    let h = hdr(&b);
    // Entry 32 + n_sections 4 + device descriptor section 8 + 18 + 2 pad.
    assert_eq!((h.flags, h.total, h.used, h.index), (0, 32 + 64, 32 + 64, 0));
    assert_eq!(&b[32 + 36..32 + 40], &[1, 0, 0, 0], "section type 1");
    assert_eq!(&b[32 + 44..32 + 46], &[18, 1]);
    guard_intact(&b, 1024);
}

#[test]
fn reports_more_and_mute() {
    let _g = rig(Some(two_devices()));
    // Room for the body head and two of the five entries.
    let len = 32 + 16 + 2 * 80 + 40;
    let mut b = request(len, 1, 3, 0, 1);
    assert_eq!(ssys(&mut b, len), E_OK);
    let h = hdr(&b);
    assert_eq!((h.flags, h.total, h.used, h.arg), (MORE, 32 + 16 + 4 * 80, 32 + 16 + 160, 1));
    let u32le = |o: usize| u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
    assert_eq!((u32le(32), u32le(36), u32le(40)), (0, 5, 0), "oldest, next, lost");
    assert_eq!((u32le(48), u32le(128)), (1, 2), "entries from seq 1");
    guard_intact(&b, len);

    let mut b = request(40, 1, 4, 0, 6000);
    assert_eq!(ssys(&mut b, 40), E_OK);
    let h = hdr(&b);
    assert_eq!((h.flags, h.used), (0, 40));
    let until = u32::from_le_bytes([b[32], b[33], b[34], b[35]]);
    assert_eq!(until, h.now.wrapping_add(6000));
    assert!(STORE.lock().unwrap().as_mut().unwrap().muted(h.now));
    let mut b = request(40, 1, 4, 0, 0);
    assert_eq!(ssys(&mut b, 40), E_OK);
    assert_eq!(&b[32..40], &[0; 8], "mute off");
}
