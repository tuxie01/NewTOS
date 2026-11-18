//! V67 — the VT52 console at the 32-plane setting **before any workstation
//! is open** (the boot console, the `\AUTO\` programs):
//! `docs/api/bios/_vt52-32plane.md` rule C3 (TASK/V67 reading 4). The
//! registers resolve through EmuTOS' `falcon_default_palette`
//! (`bios/conout.c:31-34,169-170`), each 16-bit word expanded to a pixel by
//! gun replication with X = 0, so the boot console is the same picture at
//! both settings. This binary never opens a workstation.
//! Ticket: `TASK/V67.md`.

#![cfg(all(target_arch = "aarch64", any(target_os = "macos", target_os = "linux")))]

mod vdi_planes32_common;

use vdi_planes32_common::*;

/// EmuTOS `falcon_default_palette` (`bios/conout.c:31-34`).
const FALCON: [u16; 16] = [
    0xffdf, 0xf800, 0x07c0, 0xffc0, 0x001f, 0xf81f, 0x07df, 0xbdd7, //
    0x8c51, 0xa800, 0x0540, 0xad40, 0x0015, 0xa815, 0x0555, 0x0000,
];

fn out(bytes: &[u8]) {
    for &b in bytes {
        tos_api::vdi::alpha_conout(screen(), b);
    }
}

/// The background of cell `(col, 0)`: all 64 raw words of the 8×8 cell.
fn cell_words(col: i16) -> Vec<u32> {
    (0..8).flat_map(|y| (0..8).map(move |x| screen().raw(col * 8 + x, y))).collect()
}

/// One cell per register in that register's background colour.
fn paint() -> Vec<Vec<u32>> {
    tos_api::vdi::alpha_init(screen());
    for r in 0..16u8 {
        out(&[0x1b, b'c', b'0' + r, b' ']);
    }
    (0..16).map(cell_words).collect()
}

#[test]
fn without_a_workstation_the_registers_are_the_default_words_expanded_at_32() {
    let _g = setting(32);
    let cells = paint();
    for (r, &word) in FALCON.iter().enumerate() {
        assert_eq!(word & OVERLAY_BIT, 0, "register {r}: no overlay bit, so X = 0");
        let want = word_to_device(word);
        assert!(cells[r].iter().all(|&w| w == want), "register {r}: {want:08x}, got {:08x}", cells[r][0]);
    }
}

#[test]
fn without_a_workstation_the_boot_console_is_the_same_picture_at_16_and_32() {
    let a = {
        let _g = setting(16);
        paint()
    };
    let b = {
        let _g = setting(32);
        paint()
    };
    assert_eq!(a, b, "bit for bit");
}
