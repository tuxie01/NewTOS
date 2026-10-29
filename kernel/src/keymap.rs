//! Keyboard layout tables (scancode → ASCII), TOS-conform.
//!
//! Mirrors the EmuTOS `struct keytbl` (`bios/ikbd.h`): three 128-byte
//! tables (`norm`/`shft`/`caps`) indexed by Atari scancode, plus AltGr
//! `(scancode, ascii)` override pairs for the `@ [ ] \ { }` keys. The
//! selection logic (shift/caps/alt) follows EmuTOS `ascii_cnv`
//! (`bios/ikbd.c`).
//!
//! **Data source:** the built-in US and German tables are extracted
//! verbatim from EmuTOS `bios/keyb_us.h` / `bios/keyb_de.h`.
//!
//! The active layout lives in a mutable RAM copy so it can be selected at
//! boot from the built-ins (`#K` line in `NEWDESK.INF`) or overridden by a
//! loadable `C:\KEYBOARD.KBD` file ([`load_kbd`]). Single boot CPU only, so
//! a hand-rolled `Sync` over an `UnsafeCell` (like the console ring).

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicU8, Ordering};

use tos_api::host::{KeyTable, KeyTables, KeyboardLayout, ALT_TABLE_LEN};

/// Maximum AltGr `(scancode, ascii)` override pairs per shift level.
const ALTGR_MAX: usize = 16;

/// One keyboard layout: the three scancode→ASCII tables plus the AltGr
/// override pairs. `Copy` so the boot code can install a built-in into the
/// active RAM slot with a plain assignment.
#[derive(Clone, Copy)]
pub struct Layout {
    norm: [u8; 128],
    shft: [u8; 128],
    caps: [u8; 128],
    altnorm: [(u8, u8); ALTGR_MAX],
    altshft: [(u8, u8); ALTGR_MAX],
    altcaps: [(u8, u8); ALTGR_MAX],
    altnorm_n: u8,
    altshft_n: u8,
    altcaps_n: u8,
}

/// Build an AltGr pair array from a slice, zero-padded to `ALTGR_MAX`.
const fn altgr(pairs: &[(u8, u8)]) -> ([(u8, u8); ALTGR_MAX], u8) {
    let mut out = [(0u8, 0u8); ALTGR_MAX];
    let mut i = 0;
    while i < pairs.len() {
        out[i] = pairs[i];
        i += 1;
    }
    (out, pairs.len() as u8)
}

// ── US (QWERTY) — EmuTOS bios/keyb_us.h ──────────────────────────────────
const US_NORM: [u8; 128] = [
    0x00, 0x1B, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36,
    0x37, 0x38, 0x39, 0x30, 0x2D, 0x3D, 0x08, 0x09,
    0x71, 0x77, 0x65, 0x72, 0x74, 0x79, 0x75, 0x69,
    0x6F, 0x70, 0x5B, 0x5D, 0x0D, 0x00, 0x61, 0x73,
    0x64, 0x66, 0x67, 0x68, 0x6A, 0x6B, 0x6C, 0x3B,
    0x27, 0x60, 0x00, 0x5C, 0x7A, 0x78, 0x63, 0x76,
    0x62, 0x6E, 0x6D, 0x2C, 0x2E, 0x2F, 0x00, 0x00,
    0x00, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x2D, 0x00, 0x00, 0x00, 0x2B, 0x00,
    0x00, 0x00, 0x00, 0x7F, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x28, 0x29, 0x2F, 0x2A, 0x37,
    0x38, 0x39, 0x34, 0x35, 0x36, 0x31, 0x32, 0x33,
    0x30, 0x2E, 0x0D, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];
const US_SHFT: [u8; 128] = [
    0x00, 0x1B, 0x21, 0x40, 0x23, 0x24, 0x25, 0x5E,
    0x26, 0x2A, 0x28, 0x29, 0x5F, 0x2B, 0x08, 0x09,
    0x51, 0x57, 0x45, 0x52, 0x54, 0x59, 0x55, 0x49,
    0x4F, 0x50, 0x7B, 0x7D, 0x0D, 0x00, 0x41, 0x53,
    0x44, 0x46, 0x47, 0x48, 0x4A, 0x4B, 0x4C, 0x3A,
    0x22, 0x7E, 0x00, 0x7C, 0x5A, 0x58, 0x43, 0x56,
    0x42, 0x4E, 0x4D, 0x3C, 0x3E, 0x3F, 0x00, 0x00,
    0x00, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x37,
    0x38, 0x00, 0x2D, 0x34, 0x00, 0x36, 0x2B, 0x00,
    0x32, 0x00, 0x30, 0x7F, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x28, 0x29, 0x2F, 0x2A, 0x37,
    0x38, 0x39, 0x34, 0x35, 0x36, 0x31, 0x32, 0x33,
    0x30, 0x2E, 0x0D, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];
const US_CAPS: [u8; 128] = [
    0x00, 0x1B, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36,
    0x37, 0x38, 0x39, 0x30, 0x2D, 0x3D, 0x08, 0x09,
    0x51, 0x57, 0x45, 0x52, 0x54, 0x59, 0x55, 0x49,
    0x4F, 0x50, 0x5B, 0x5D, 0x0D, 0x00, 0x41, 0x53,
    0x44, 0x46, 0x47, 0x48, 0x4A, 0x4B, 0x4C, 0x3B,
    0x27, 0x60, 0x00, 0x5C, 0x5A, 0x58, 0x43, 0x56,
    0x42, 0x4E, 0x4D, 0x2C, 0x2E, 0x2F, 0x00, 0x00,
    0x00, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x2D, 0x00, 0x00, 0x00, 0x2B, 0x00,
    0x00, 0x00, 0x00, 0x7F, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x28, 0x29, 0x2F, 0x2A, 0x37,
    0x38, 0x39, 0x34, 0x35, 0x36, 0x31, 0x32, 0x33,
    0x30, 0x2E, 0x0D, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// Built-in US (QWERTY) layout.
pub const US: Layout = {
    let (an, ann) = altgr(&[]);
    let (as_, asn) = altgr(&[]);
    let (ac, acn) = altgr(&[]);
    Layout {
        norm: US_NORM, shft: US_SHFT, caps: US_CAPS,
        altnorm: an, altshft: as_, altcaps: ac,
        altnorm_n: ann, altshft_n: asn, altcaps_n: acn,
    }
};

// ── DE (QWERTZ) — EmuTOS bios/keyb_de.h ──────────────────────────────────
const DE_NORM: [u8; 128] = [
    0x00, 0x1B, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36,
    0x37, 0x38, 0x39, 0x30, 0x9E, 0x27, 0x08, 0x09,
    0x71, 0x77, 0x65, 0x72, 0x74, 0x7A, 0x75, 0x69,
    0x6F, 0x70, 0x81, 0x2B, 0x0D, 0x00, 0x61, 0x73,
    0x64, 0x66, 0x67, 0x68, 0x6A, 0x6B, 0x6C, 0x94,
    0x84, 0x23, 0x00, 0x7E, 0x79, 0x78, 0x63, 0x76,
    0x62, 0x6E, 0x6D, 0x2C, 0x2E, 0x2D, 0x00, 0x00,
    0x00, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x2D, 0x00, 0x00, 0x00, 0x2B, 0x00,
    0x00, 0x00, 0x00, 0x7F, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x3C, 0x00, 0x00, 0x28, 0x29, 0x2F, 0x2A, 0x37,
    0x38, 0x39, 0x34, 0x35, 0x36, 0x31, 0x32, 0x33,
    0x30, 0x2E, 0x0D, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];
const DE_SHFT: [u8; 128] = [
    0x00, 0x1B, 0x21, 0x22, 0xDD, 0x24, 0x25, 0x26,
    0x2F, 0x28, 0x29, 0x3D, 0x3F, 0x60, 0x08, 0x09,
    0x51, 0x57, 0x45, 0x52, 0x54, 0x5A, 0x55, 0x49,
    0x4F, 0x50, 0x9A, 0x2A, 0x0D, 0x00, 0x41, 0x53,
    0x44, 0x46, 0x47, 0x48, 0x4A, 0x4B, 0x4C, 0x99,
    0x8E, 0x5E, 0x00, 0x7C, 0x59, 0x58, 0x43, 0x56,
    0x42, 0x4E, 0x4D, 0x3B, 0x3A, 0x5F, 0x00, 0x00,
    0x00, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x37,
    0x38, 0x00, 0x2D, 0x34, 0x00, 0x36, 0x2B, 0x00,
    0x32, 0x00, 0x30, 0x7F, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x3E, 0x00, 0x00, 0x28, 0x29, 0x2F, 0x2A, 0x37,
    0x38, 0x39, 0x34, 0x35, 0x36, 0x31, 0x32, 0x33,
    0x30, 0x2E, 0x0D, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];
const DE_CAPS: [u8; 128] = [
    0x00, 0x1B, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36,
    0x37, 0x38, 0x39, 0x30, 0x9E, 0x27, 0x08, 0x09,
    0x51, 0x57, 0x45, 0x52, 0x54, 0x5A, 0x55, 0x49,
    0x4F, 0x50, 0x9A, 0x2B, 0x0D, 0x00, 0x41, 0x53,
    0x44, 0x46, 0x47, 0x48, 0x4A, 0x4B, 0x4C, 0x99,
    0x8E, 0x23, 0x00, 0x7E, 0x59, 0x58, 0x43, 0x56,
    0x42, 0x4E, 0x4D, 0x2C, 0x2E, 0x2D, 0x00, 0x00,
    0x00, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x2D, 0x00, 0x00, 0x00, 0x2B, 0x00,
    0x00, 0x00, 0x00, 0x7F, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x3C, 0x00, 0x00, 0x28, 0x29, 0x2F, 0x2A, 0x37,
    0x38, 0x39, 0x34, 0x35, 0x36, 0x31, 0x32, 0x33,
    0x30, 0x2E, 0x0D, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// Built-in German (QWERTZ) layout, with AltGr `@ [ ] \ { }`.
pub const DE: Layout = {
    let (an, ann) = altgr(&[(0x1A, b'@'), (0x27, b'['), (0x28, b']')]);
    let (as_, asn) = altgr(&[(0x1A, b'\\'), (0x27, b'{'), (0x28, b'}')]);
    let (ac, acn) = altgr(&[(0x1A, b'@'), (0x27, b'['), (0x28, b']')]);
    Layout {
        norm: DE_NORM, shft: DE_SHFT, caps: DE_CAPS,
        altnorm: an, altshft: as_, altcaps: ac,
        altnorm_n: ann, altshft_n: asn, altcaps_n: acn,
    }
};

// ── active layout (mutable, single boot CPU) ─────────────────────────────

struct Active {
    layout: UnsafeCell<Layout>,
}
// SAFETY: only the single boot CPU (the IKBD decode path + boot init)
// touches the active layout.
unsafe impl Sync for Active {}
static ACTIVE: Active = Active { layout: UnsafeCell::new(US) };

/// Where the active layout came from (`KeyboardLayout as u8`), for
/// `Ssystem(S_NEWTOS_KEYBOARD)` (SYSINFO01). Starts as the compiled-in US
/// default, like [`ACTIVE`].
static SOURCE: AtomicU8 = AtomicU8::new(KeyboardLayout::BuiltinUs as u8);

/// The power-up layout — NewTOS' counterpart of the table EmuTOS'
/// `get_keytbl` returns for the current country
/// (`references/emutos/bios/country.c:195-202`). EmuTOS picks it from the
/// ROM's country and the NVRAM keyboard byte (`detect_akp`, `:140-163`);
/// NewTOS from the boot selection ([`init_from_disk`]: `C:\KEYBOARD.KBD`,
/// else `#K` in `C:\NEWDESK.INF`, else US). Only the boot selection writes
/// it; XBIOS `Bioskeys` copies it back into [`ACTIVE`] ([`restore_default`]).
static DEFAULT: Active = Active { layout: UnsafeCell::new(US) };

/// Where [`DEFAULT`] came from (`KeyboardLayout as u8`), so that `Bioskeys`
/// also restores what `Ssystem(S_NEWTOS_KEYBOARD)` reports.
static DEFAULT_SOURCE: AtomicU8 = AtomicU8::new(KeyboardLayout::BuiltinUs as u8);

/// Where the active layout came from: a built-in, `C:\KEYBOARD.KBD`, or a
/// program's `Keytbl` since.
pub fn source() -> KeyboardLayout {
    match SOURCE.load(Ordering::Relaxed) {
        0 => KeyboardLayout::BuiltinUs,
        1 => KeyboardLayout::BuiltinGerman,
        2 => KeyboardLayout::File,
        _ => KeyboardLayout::ChangedByProgram,
    }
}

#[inline]
fn active() -> &'static Layout {
    // SAFETY: single-CPU; the layout is only replaced during boot init,
    // never concurrently with a decode.
    unsafe { &*ACTIVE.layout.get() }
}

/// Install a built-in layout by name (`"us"` / `"de"`) as the power-up
/// layout; returns whether the name was recognised.
pub fn select_builtin(name: &[u8]) -> bool {
    let (lay, src) = match name {
        b"us" | b"US" => (US, KeyboardLayout::BuiltinUs),
        b"de" | b"DE" => (DE, KeyboardLayout::BuiltinGerman),
        _ => return false,
    };
    install_power_up(lay, src);
    true
}

/// Make `lay` both the active and the power-up layout ([`DEFAULT`]) — the
/// boot selection, which is what `Bioskeys` later restores.
fn install_power_up(lay: Layout, src: KeyboardLayout) {
    // SAFETY: single-CPU boot context; nothing decodes a key while the
    // layout is selected.
    unsafe {
        *ACTIVE.layout.get() = lay;
        *DEFAULT.layout.get() = lay;
    }
    SOURCE.store(src as u8, Ordering::Relaxed);
    DEFAULT_SOURCE.store(src as u8, Ordering::Relaxed);
}

/// Put the power-up layout back — the kernel half of XBIOS `Bioskeys`,
/// EmuTOS `current_keytbl = *get_keytbl();`
/// (`references/emutos/bios/ikbd.c:136-140`). Every table a program installed
/// through `Keytbl` is replaced, the three primary tables as well as the AltGr
/// overrides, and the reported source goes back to the boot selection.
pub fn restore_default() {
    // SAFETY: single-CPU cooperative path (the XBIOS trap); the active layout
    // is not read concurrently with this copy, and `DEFAULT` is only written
    // by the boot selection.
    unsafe { *ACTIVE.layout.get() = *DEFAULT.layout.get() };
    SOURCE.store(DEFAULT_SOURCE.load(Ordering::Relaxed), Ordering::Relaxed);
}

/// Snapshot the active layout in the XBIOS `Keytbl` shape (EmuTOS
/// `struct keytbl`, `bios/ikbd.h:57-86`): the three 128-byte scancode→ASCII
/// tables plus the three AltGr override arrays, each rendered in EmuTOS'
/// non-`DUAL_KEYBOARD` wire format — `(scancode, ascii)` pairs terminated by a
/// zero byte (`keyb_de.h`'s `keytbl_de_altcaps` is the model).
pub fn snapshot() -> KeyTables {
    let lay = active();
    let pack = |pairs: &[(u8, u8); ALTGR_MAX], n: u8| -> [u8; ALT_TABLE_LEN] {
        let mut out = [0u8; ALT_TABLE_LEN];
        for (i, p) in pairs[..n as usize].iter().enumerate() {
            out[i * 2] = p.0;
            out[i * 2 + 1] = p.1;
        }
        // The terminating zero byte is already there (the array starts zeroed
        // and `2 * ALTGR_MAX < ALT_TABLE_LEN`).
        out
    };
    KeyTables {
        norm: lay.norm,
        shft: lay.shft,
        caps: lay.caps,
        altnorm: pack(&lay.altnorm, lay.altnorm_n),
        altshft: pack(&lay.altshft, lay.altshft_n),
        altcaps: pack(&lay.altcaps, lay.altcaps_n),
    }
}

/// Replace one of the three primary scancode→ASCII tables of the active
/// layout — the set half of XBIOS `Keytbl` (`bios/ikbd.c:124-132`). The AltGr
/// overrides and the other two tables are untouched, exactly as `Keytbl`
/// leaves them.
pub fn set_table(which: KeyTable, table: &[u8; 128]) {
    // SAFETY: single-CPU cooperative path; the active layout is only mutated
    // here and during boot init, never concurrently with a decode.
    let lay = unsafe { &mut *ACTIVE.layout.get() };
    let slot = match which {
        KeyTable::Norm => &mut lay.norm,
        KeyTable::Shft => &mut lay.shft,
        KeyTable::Caps => &mut lay.caps,
    };
    // A program that hands back the table it read changes nothing; only
    // different contents make the layout "changed by a program".
    if slot != table {
        *slot = *table;
        SOURCE.store(KeyboardLayout::ChangedByProgram as u8, Ordering::Relaxed);
    }
}

/// Translate a scancode to ASCII against the **active** layout given the
/// modifier state (EmuTOS `ascii_cnv`, non-DUAL_KEYBOARD path). Returns 0
/// when the key has no character (cursor / function keys, unmapped codes).
/// Ctrl folding is applied by the caller.
pub fn to_ascii(scancode: u8, shift: bool, caps: bool, alt: bool) -> u8 {
    if scancode >= 128 {
        return 0;
    }
    let lay = active();
    let idx = scancode as usize;
    let base = if shift {
        &lay.shft
    } else if caps {
        &lay.caps
    } else {
        &lay.norm
    };
    let mut ascii = base[idx];

    if alt {
        // AltGr override pairs for this shift level.
        let (pairs, n) = if shift {
            (&lay.altshft, lay.altshft_n)
        } else if caps {
            (&lay.altcaps, lay.altcaps_n)
        } else {
            (&lay.altnorm, lay.altnorm_n)
        };
        for p in &pairs[..n as usize] {
            if p.0 == scancode {
                ascii = p.1;
                break;
            }
        }
        // Alt + a letter yields no character (EmuTOS).
        if ascii.is_ascii_alphabetic() {
            ascii = 0;
        }
    }
    ascii
}

/// Select the keyboard layout at boot from the SD card (called once the
/// filesystem is mounted, before the console/desktop): a `C:\KEYBOARD.KBD`
/// file overrides everything; otherwise a `#K <layout>` line in
/// `C:\NEWDESK.INF` selects a built-in (`us`/`de`). Absent both → the
/// compiled-in default (US).
pub fn init_from_disk() {
    // 1) A loadable KEYBOARD.KBD takes precedence.
    let mut buf = [0u8; 1024];
    if let Some(n) = tos_api::bdos::fs::read_file("C:\\KEYBOARD.KBD", &mut buf) {
        if load_kbd(&buf[..n]) {
            return;
        }
    }
    // 2) Else the `#K <layout>` line in NEWDESK.INF picks a built-in.
    let mut inf = [0u8; 4096];
    if let Some(n) = tos_api::bdos::fs::read_file("C:\\NEWDESK.INF", &mut inf) {
        if let Some(name) = find_k_line(&inf[..n]) {
            select_builtin(name);
        }
    }
}

/// Find the `#K <layout>` line in a NEWDESK.INF blob and return the layout
/// token (e.g. `b"de"`), or `None`.
fn find_k_line(data: &[u8]) -> Option<&[u8]> {
    let mut i = 0;
    while i < data.len() {
        // Start of a line?
        if data[i] == b'#' && i + 1 < data.len() && data[i + 1] == b'K' {
            let mut j = i + 2;
            while j < data.len() && (data[j] == b' ' || data[j] == b'\t') {
                j += 1;
            }
            let start = j;
            while j < data.len() && !data[j].is_ascii_whitespace() {
                j += 1;
            }
            if j > start {
                return Some(&data[start..j]);
            }
        }
        // Advance to the next line.
        while i < data.len() && data[i] != b'\n' {
            i += 1;
        }
        i += 1;
    }
    None
}

// ── loadable `.KBD` file ─────────────────────────────────────────────────

/// Native keyboard-file magic (`"KBD1"`).
const KBD_MAGIC: [u8; 4] = *b"KBD1";

/// Load a keyboard layout from a native `.KBD` blob and make it active.
/// Layout of the blob (little-endian, all bytes):
/// ```text
///   magic "KBD1"          (4)
///   norm[128] shft[128] caps[128]
///   altnorm_n (1), altnorm_n × (scancode, ascii)
///   altshft_n (1), altshft_n × (scancode, ascii)
///   altcaps_n (1), altcaps_n × (scancode, ascii)
/// ```
/// Returns whether the blob was valid and installed.
pub fn load_kbd(blob: &[u8]) -> bool {
    if blob.len() < 4 + 3 * 128 + 3 || blob[..4] != KBD_MAGIC {
        return false;
    }
    let mut lay = US; // start from a valid base (fields all present)
    let mut o = 4;
    let read128 = |dst: &mut [u8; 128], src: &[u8]| dst.copy_from_slice(&src[..128]);
    read128(&mut lay.norm, &blob[o..]);
    o += 128;
    read128(&mut lay.shft, &blob[o..]);
    o += 128;
    read128(&mut lay.caps, &blob[o..]);
    o += 128;

    // Three AltGr pair groups.
    for group in 0..3 {
        if o >= blob.len() {
            return false;
        }
        let n = blob[o] as usize;
        o += 1;
        if n > ALTGR_MAX || o + n * 2 > blob.len() {
            return false;
        }
        let mut pairs = [(0u8, 0u8); ALTGR_MAX];
        for p in pairs.iter_mut().take(n) {
            *p = (blob[o], blob[o + 1]);
            o += 2;
        }
        match group {
            0 => {
                lay.altnorm = pairs;
                lay.altnorm_n = n as u8;
            }
            1 => {
                lay.altshft = pairs;
                lay.altshft_n = n as u8;
            }
            _ => {
                lay.altcaps = pairs;
                lay.altcaps_n = n as u8;
            }
        }
    }
    install_power_up(lay, KeyboardLayout::File);
    true
}
