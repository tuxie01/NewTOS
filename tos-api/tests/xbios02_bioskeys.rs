//! XBIOS02 — `Bioskeys` (XBIOS 0x18) undoes `Keytbl` (XBIOS 0x10).
//!
//! Spec: `docs/api/xbios/18-bioskeys.md`, `docs/api/xbios/10-keytbl.md`.
//! EmuTOS: `bios/ikbd.c:122-134` (`keytbl`), `bios/ikbd.c:136-140`
//! (`bioskeys`: `current_keytbl = *get_keytbl();`).
//!
//! The host is the kernel's keyboard translator in miniature
//! (`kernel/src/keymap.rs`): a power-up layout chosen at boot, an active
//! layout that `set_key_table` changes and `reset_key_tables` copies the
//! power-up layout back into. With no process running, `Keytbl` returns the
//! kernel-static `struct keytbl` (LP64: six 8-byte pointers, `dead`,
//! `features`), which the test reads directly. The EL0 path (the struct in
//! the process's page 0) and the real translator are measured in QEMU
//! (`measurements/measurement-2026-10-29-xbios02/`).

use std::sync::Mutex;

use tos_api::host::{Host, KeyTable, KeyTables, ALT_TABLE_LEN};
use tos_api::trap::{TrapArgs, TrapClass};

/// One layout: the three primary tables and the three AltGr pair arrays.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Layout {
    norm: [u8; 128],
    shft: [u8; 128],
    caps: [u8; 128],
    alt: [[u8; ALT_TABLE_LEN]; 3],
}

/// The power-up layout: distinct bytes per table, one AltGr pair each.
fn power_up() -> Layout {
    let mut l = Layout { norm: [0; 128], shft: [0; 128], caps: [0; 128], alt: [[0; ALT_TABLE_LEN]; 3] };
    for i in 0..128 {
        l.norm[i] = i as u8;
        l.shft[i] = (i as u8) ^ 0x80;
        l.caps[i] = (i as u8).wrapping_add(1);
    }
    l.alt[0][..2].copy_from_slice(&[0x1A, b'@']);
    l.alt[1][..2].copy_from_slice(&[0x1A, b'\\']);
    l.alt[2][..2].copy_from_slice(&[0x27, b'[']);
    l
}

struct Translator {
    default: Layout,
    active: Mutex<Layout>,
}

impl Host for Translator {
    fn console_write_byte(&self, _byte: u8) {}
    fn console_try_read(&self) -> Option<u8> {
        None
    }
    fn key_tables(&self) -> Option<KeyTables> {
        let a = *self.active.lock().unwrap();
        Some(KeyTables {
            norm: a.norm,
            shft: a.shft,
            caps: a.caps,
            altnorm: a.alt[0],
            altshft: a.alt[1],
            altcaps: a.alt[2],
        })
    }
    fn set_key_table(&self, which: KeyTable, table: &[u8; 128]) {
        let mut a = self.active.lock().unwrap();
        match which {
            KeyTable::Norm => a.norm = *table,
            KeyTable::Shft => a.shft = *table,
            KeyTable::Caps => a.caps = *table,
        }
    }
    fn reset_key_tables(&self) {
        *self.active.lock().unwrap() = self.default;
    }
}

fn xbios(func: u16, a: [u64; 3]) -> i64 {
    tos_api::xbios::dispatch(&TrapArgs {
        regs: [a[0], a[1], a[2], 0, 0, 0, 0, 0],
        func: func as u64,
        svc_imm: TrapClass::Xbios as u16,
        space: tos_api::uaccess::Space::Kernel,
    })
}

const KEYTBL: u16 = 0x10;
const BIOSKEYS: u16 = 0x18;
const KEEP: u64 = u64::MAX;

/// Field `i` of the LP64 `struct keytbl` at `p` (`bios/ikbd.h:57-86`).
fn field(p: i64, i: usize) -> u64 {
    // SAFETY: `p` is the kernel-static keytbl area `Keytbl` just wrote; the
    // seven pointer fields are 8 bytes each.
    unsafe { ((p as u64 + 8 * i as u64) as *const u64).read_unaligned() }
}

fn table(p: u64, len: usize) -> Vec<u8> {
    // SAFETY: `p` points at a table of at least `len` bytes (the area's own
    // copy or one of this test's arrays).
    unsafe { std::slice::from_raw_parts(p as *const u8, len).to_vec() }
}

/// One test: the `Keytbl` override pointers are process-global state.
#[test]
fn bioskeys_restores_all_three_tables_after_keytbl() {
    let t: &'static Translator =
        Box::leak(Box::new(Translator { default: power_up(), active: Mutex::new(power_up()) }));
    let slot: &'static &'static dyn Host = Box::leak(Box::new(t as &'static dyn Host));
    tos_api::host::install_host(slot);
    tos_api::bdos::process::set_current(core::ptr::null_mut());

    let def = power_up();

    // Power-up: the struct points at the area's own copies of the defaults.
    let p = xbios(KEYTBL, [KEEP, KEEP, KEEP]);
    assert_ne!(p, 0);
    let (n0, s0, c0) = (field(p, 0), field(p, 1), field(p, 2));
    assert_eq!(table(n0, 128), def.norm);
    assert_eq!(table(s0, 128), def.shft);
    assert_eq!(table(c0, 128), def.caps);

    // A program installs its own three tables (the probe's `i ^ 0x55` etc.).
    let my_n: &'static [u8; 128] = Box::leak(Box::new(core::array::from_fn(|i| (i as u8) ^ 0x55)));
    let my_s: &'static [u8; 128] = Box::leak(Box::new(core::array::from_fn(|i| (i as u8) ^ 0x2A)));
    let my_c: &'static [u8; 128] = Box::leak(Box::new(core::array::from_fn(|i| (i as u8) ^ 0x7F)));
    let (mn, ms, mc) = (my_n.as_ptr() as u64, my_s.as_ptr() as u64, my_c.as_ptr() as u64);
    assert_eq!(xbios(KEYTBL, [mn, KEEP, KEEP]), p);
    assert_eq!(xbios(KEYTBL, [KEEP, ms, mc]), p);
    assert_eq!((field(p, 0), field(p, 1), field(p, 2)), (mn, ms, mc));
    {
        let a = t.active.lock().unwrap();
        assert_eq!((&a.norm, &a.shft, &a.caps), (my_n, my_s, my_c));
    }

    // Bioskeys: all three pointers are the power-up ones again and the
    // translator uses the power-up layout again (`bios/ikbd.c:139`). The
    // return is the address of the struct now holding the defaults.
    assert_eq!(xbios(BIOSKEYS, [0, 0, 0]), p);
    assert_eq!(*t.active.lock().unwrap(), def);
    let q = xbios(KEYTBL, [KEEP, KEEP, KEEP]);
    assert_eq!(q, p);
    assert_eq!((field(q, 0), field(q, 1), field(q, 2)), (n0, s0, c0));
    assert_eq!(table(field(q, 0), 128), def.norm);
    assert_eq!(table(field(q, 1), 128), def.shft);
    assert_eq!(table(field(q, 2), 128), def.caps);
    // The AltGr arrays, `dead` (NULL) and `features` (0) are the defaults too.
    for i in 0..3 {
        assert_eq!(table(field(q, 3 + i), ALT_TABLE_LEN), def.alt[i]);
    }
    assert_eq!(field(q, 6), 0);
    // SAFETY: `features` is the WORD after the seven pointers.
    assert_eq!(unsafe { ((q as u64 + 56) as *const u16).read_unaligned() }, 0);

    // A second Bioskeys changes nothing; a later Keytbl still works.
    assert_eq!(xbios(BIOSKEYS, [0, 0, 0]), p);
    assert_eq!((field(p, 0), field(p, 1), field(p, 2)), (n0, s0, c0));
    assert_eq!(xbios(KEYTBL, [mn, KEEP, KEEP]), p);
    assert_eq!(field(p, 0), mn);
    assert_eq!(t.active.lock().unwrap().norm, *my_n);
    assert_eq!(xbios(BIOSKEYS, [0, 0, 0]), p);
    assert_eq!(field(p, 0), n0);
    assert_eq!(*t.active.lock().unwrap(), def);
}
