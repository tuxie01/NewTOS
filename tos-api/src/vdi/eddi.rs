//! EdDI `vq_scrninfo` — VDI opcode 102, sub-opcode 1, `INTIN[0] == 2`.
//!
//! Spec: [`docs/api/vdi/eddi-vq_scrninfo.md`] (approved 2026-10-02, E1–E9 as
//! recommended in its §7, **E3 = `0x02`**) at 16 planes, and
//! `docs/api/vdi/eddi-vq_scrninfo-32.md` at the 32-plane setting (ADR-038
//! point 8, TASK/V66). EmuTOS has **no** EdDI (spec §0): a deliberate NewTOS
//! extension beyond EmuTOS, recorded in `docs/COMPATIBILITY.md` (E8). The
//! `EdDI` cookie that announces it is seeded in [`crate::cookie::seed_defaults`].
//!
//! **What is answered.** At 16 planes the device word is the Falcon HiColor
//! word `RRRRRGGG GG0BBBBB` — 5/5/5 bits, bit 5 the `OVERLAY_BIT` the VDI never
//! sets (spec §6.2; EmuTOS `vdi_col.c:599-613`, `vdi_fill.c:26`;
//! [`super::vwk::pack_color16`]), bit for bit the Falcon HiColor example of the
//! EdDI documentation (spec §3.3/§6.3), so NewTOS answers what a Falcon
//! answers, bit order `0x02`. At 32 planes the device pixel is `0xXXRRGGBB`, 8
//! bits a gun, X unused (ADR-038 point 4): 2^24 colours, bit order `0x01`.
//!
//! The bit numbers describe the pixel **as the CPU reads it** (E3): a NewTOS
//! program and `vro_cpyfm` read the same native `u16` (at 32 planes `u32`) out
//! of a device-format memory MFDB (`raster.rs`, `Mfdb::get_mem`/`put_mem`), so
//! there is no byte-order difference to report and bit 7 (`0x80`, "swapped
//! bytes") stays clear at both depths.
//!
//! Measured counterpart at 16 planes: the VDI Enhancer 1.00 (a real EdDI) on
//! EmuTOS Falcon truecolor reports the same 273 words except `[14]`
//! (`measurements/measurement-2026-10-02-scrninfo/hatari-falcon-tc-enhancer/
//! SCRNINFO.TXT`). Every word that differs is named at [`scrninfo_table`].

use super::backend::{RasterBackend, VDI_PLANES_WIDE};
use super::Vdipb;

/// `CONTRL[5]` sub-opcode that turns opcode 102 into `vq_scrninfo`
/// (spec §3.1; gemlib `n_vq_scrninfo.c`, `VDI_TRAP_ESC(...,102,1,0,1)`).
pub(super) const SCRNINFO_SUBOP: i16 = 1;

/// `INTIN[0]` value of `vq_scrninfo` — "output extended information"
/// (spec §3.1, NV250 §2).
pub(super) const SCRNINFO_MODE: i16 = 2;

/// Number of `work_out` words written: `INTOUT[0..=272]`. The documentation
/// gives `CONTRL[4] = 272` but describes 273 words, and the NVDI 2.5x
/// Enhancer copies 273 (spec §3.1); the measured Enhancer 1.00 writes
/// `INTOUT[272]` (= 0) and nothing beyond.
pub const SCRNINFO_WORDS: usize = 273;

/// `CONTRL[4]` as the documentation states it (spec §3.1: "`CONTRL[4]` |
/// 272 (documentation)" — "the driver fills it in").
pub const SCRNINFO_NINTOUT: i16 = 272;

/// The default device word: 16 planes (`kernel/src/vdi_backend.rs` `PLANES`,
/// ADR-024); the other depth is [`VDI_PLANES_WIDE`] (ADR-038 point 1). See
/// [`is_scrninfo`] for any other plane count.
const DEVICE_PLANES: u16 = 16;

// The 16-plane device word's layout, as `pack_color16` packs it (`(r << 11) |
// (g << 6) | b`, `vwk.rs`) and `OVERLAY_BIT` (`0x0020`, EmuTOS
// `vdi_fill.c:26`) — the inputs of the 16-plane bit-number tables.
const GUN_BITS: i16 = 5;
const RED_LSB: i16 = 11;
const GREEN_LSB: i16 = 6;
const BLUE_LSB: i16 = 0;
const GENLOCK_BIT: i16 = 5;

/// `work_out[14]` bit order: `2` = Falcon order `rrrrrggg-ggxbbbbb`, one
/// overlay bit in the middle (spec §3.3; gemlib `n_vq_scrninfo.c:144`).
/// Approved as E3 (spec status header).
const ORDER_FALCON: i16 = 0x02;

/// Is this opcode-102 call a `vq_scrninfo`?
///
/// E7: `CONTRL[5] == 1` **and** `INTIN[0] == 2`; every other combination
/// stays `vq_extnd` exactly as before (spec §7 E7). This is the one place
/// NewTOS reads `CONTRL[5]` for opcode 102, which EmuTOS does not
/// (`vdi_main.c:77`) — the deviation E7/E8 names.
///
/// The answer is defined for the two depths of ADR-038 point 1 only: 16
/// planes ([`DEVICE_PLANES`], spec §6.3) and the 32-plane setting
/// ([`VDI_PLANES_WIDE`], `eddi-vq_scrninfo-32.md` §2). A backend with any
/// other plane count — only host test doubles have one — keeps the
/// pre-EdDI `vq_extnd` answer rather than a table no spec derived.
pub(super) fn is_scrninfo(pb: &Vdipb, backend: &dyn RasterBackend) -> bool {
    pb.contrl(5) == SCRNINFO_SUBOP
        && pb.intin(0) == SCRNINFO_MODE
        && layout(backend.planes()).is_some()
}

/// The 273 `work_out` words NewTOS reports at 16 planes, the default setting
/// (spec §3.2 layout, §6.3 values); at 32 planes see [`scrninfo_table_for`].
///
/// Word by word against the measured Enhancer 1.00 on EmuTOS Falcon
/// truecolor (`.../hatari-falcon-tc-enhancer/SCRNINFO.TXT`, §3b):
///
/// | idx | NewTOS | Enhancer | why |
/// |---|---|---|---|
/// | 0 | 2 | 2 | packed pixels |
/// | 1 | 2 | 2 | software CLUT (`INQ_TAB[5] = 0`) |
/// | 2 | 16 | 16 | `v_planes` |
/// | 3/4 | 0 / `0x8000` | 0 / `0x8000` | 32768 colours, high word first (E4) |
/// | 5 | 0 | 0 | no accessible line width (E5) |
/// | 6/7 | 0 / 0 | 0 / 0 | no accessible bitmap address (E5) |
/// | 8/9/10 | 5/5/5 | 5/5/5 | bits per gun |
/// | 11 | 0 | 0 | no alpha |
/// | 12 | 1 | 1 | one genlock bit (`OVERLAY_BIT`); programs leave it 0 (E9) |
/// | 13 | 0 | 0 | no unused bits |
/// | **14** | **2** | **0** | **differs**: the Enhancer is EdDI 1.00, which has no `[14]` (it arrives with 1.10, spec §2) and leaves it 0; NewTOS fills it with the Falcon order (E3) |
/// | 15 | 0 | 0 | undocumented |
/// | 16..20 | 11..15 | 11..15 | red bit numbers, LSB first |
/// | 32..36 | 6..10 | 6..10 | green |
/// | 48..52 | 0..4 | 0..4 | blue |
/// | 80 | 5 | 5 | genlock bit |
/// | rest of 16..127 | -1 | -1 | unused table entries |
/// | 128..272 | 0 | 0 | reserved |
pub fn scrninfo_table() -> [i16; SCRNINFO_WORDS] {
    build(&LAYOUT_16)
}

/// The 273 words for the layout `l` (spec §3.2 layout; the values per depth).
fn build(l: &Layout) -> [i16; SCRNINFO_WORDS] {
    let mut w = [0i16; SCRNINFO_WORDS];
    w[0] = 2; // format: packed pixels
    w[1] = 2; // CLUT: software ("pseudo") CLUT, HiColor/TrueColor
    w[2] = l.planes as i16; // bits per pixel
    // Colour count as a LONG, high word first (spec §3.2 [3/4]): 32768 at 16,
    // not 65536 — the overlay bit carries no colour (E4); 2^24 at 32.
    let colours: u32 = 1 << (3 * l.gun_bits);
    w[3] = (colours >> 16) as i16;
    w[4] = colours as u16 as i16;
    // [5] line width, [6/7] bitmap address: 0 (E5; ADR-038 point 8).
    w[8..11].fill(l.gun_bits); // bits of red, green, blue
    w[11] = 0; // alpha
    w[12] = l.genlock_bits;
    w[13] = l.unused_bits;
    w[14] = l.order;
    // [15]: undocumented, 0.
    // Bit-number tables 16..127: -1 = "not used", then the bits present.
    w[16..128].fill(-1);
    for i in 0..l.gun_bits {
        w[16 + i as usize] = l.red_lsb + i;
        w[32 + i as usize] = l.green_lsb + i;
        w[48 + i as usize] = l.blue_lsb + i;
    }
    for i in 0..l.genlock_bits {
        w[80 + i as usize] = l.genlock_lsb + i;
    }
    for i in 0..l.unused_bits {
        w[96 + i as usize] = l.unused_lsb + i;
    }
    // [128..=272]: reserved, 0.
    w
}

/// Opcode 102 sub-opcode 1, `INTIN[0] == 2` — `vq_scrninfo`.
///
/// Writes `INTOUT[0..=272]`, the table of the backend's depth; PTSOUT is not
/// touched (spec §3.1; the measured Enhancer writes none). Every valid screen
/// handle, physical or virtual, gets the same answer (E6); an invalid one never
/// gets here (the dispatcher's no-op, as for `vq_extnd`). Output counts at both
/// depths: `CONTRL[4] = 272` as documented, `CONTRL[2] = 0` (no PTSOUT); the
/// dispatcher rewrites the counts on every call (as EmuTOS' `screen()` does).
pub(super) fn vq_scrninfo(pb: &Vdipb, backend: &dyn RasterBackend) {
    // `is_scrninfo` lets only a depth with a layout through.
    let Some(l) = layout(backend.planes()) else {
        return;
    };
    for (i, &v) in build(l).iter().enumerate() {
        pb.set_intout(i, v);
    }
    pb.set_contrl(4, SCRNINFO_NINTOUT);
    pb.set_contrl(2, 0);
}

/// The 273 words at the depth `planes`: the 16-plane table
/// ([`scrninfo_table`]) or the 32-plane one (`eddi-vq_scrninfo-32.md` §2);
/// `None` for any other plane count, which has no table ([`is_scrninfo`]).
pub fn scrninfo_table_for(planes: u16) -> Option<[i16; SCRNINFO_WORDS]> {
    layout(planes).map(build)
}

/// The pixel layout one depth's answer describes: the inputs of [`build`].
struct Layout {
    planes: u16,
    gun_bits: i16,
    red_lsb: i16,
    green_lsb: i16,
    blue_lsb: i16,
    genlock_bits: i16,
    genlock_lsb: i16,
    unused_bits: i16,
    unused_lsb: i16,
    order: i16,
}

/// The layout of the depth `planes`, or `None` for a depth without a table.
fn layout(planes: u16) -> Option<&'static Layout> {
    match planes {
        DEVICE_PLANES => Some(&LAYOUT_16),
        VDI_PLANES_WIDE => Some(&LAYOUT_32),
        _ => None,
    }
}

/// 16 planes: 5/5/5 bits, red 11..15, green 6..10, blue 0..4, one genlock bit
/// (bit 5), no unused bits, Falcon order (E3).
const LAYOUT_16: Layout = Layout {
    planes: DEVICE_PLANES,
    gun_bits: GUN_BITS,
    red_lsb: RED_LSB,
    green_lsb: GREEN_LSB,
    blue_lsb: BLUE_LSB,
    genlock_bits: 1,
    genlock_lsb: GENLOCK_BIT,
    unused_bits: 0,
    unused_lsb: 0,
    order: ORDER_FALCON,
};

// The 32-plane device pixel `0xXXRRGGBB` (ADR-038 point 4; the pen packing
// `(r << 16) | (g << 8) | b` of `vwk::pack_color32`): 8 bits a gun, the X
// byte unused, no genlock bit (V61 O1 option A) — the inputs of the 32-plane
// bit-number tables (ADR-038 point 8).
const GUN_BITS_32: i16 = 8;
const RED_LSB_32: i16 = 16;
const GREEN_LSB_32: i16 = 8;
const BLUE_LSB_32: i16 = 0;
const UNUSED_BITS_32: i16 = 8;
const UNUSED_LSB_32: i16 = 24;

/// `work_out[14]` bit order: `1` = usual order `xxxxxxxx-rrrrrrrr-gggggggg-
/// bbbbbbbb` (gemlib `n_vq_scrninfo.c:70-80`), the bits counted on the native
/// `u32` as E3 counts the `u16`, so not `0x81` (`eddi-vq_scrninfo-32.md` §3).
const ORDER_USUAL: i16 = 0x01;

/// 32 planes: 8/8/8 bits, red 16..23, green 8..15, blue 0..7, no genlock bit,
/// the X byte 24..31 unused, usual order (ADR-038 point 8).
const LAYOUT_32: Layout = Layout {
    planes: VDI_PLANES_WIDE,
    gun_bits: GUN_BITS_32,
    red_lsb: RED_LSB_32,
    green_lsb: GREEN_LSB_32,
    blue_lsb: BLUE_LSB_32,
    genlock_bits: 0,
    genlock_lsb: 0,
    unused_bits: UNUSED_BITS_32,
    unused_lsb: UNUSED_LSB_32,
    order: ORDER_USUAL,
};
