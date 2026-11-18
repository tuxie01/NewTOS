//! The `AESGLOBAL` byte layout `appl_init` writes — the TOS structure, byte
//! for byte, on the native AArch64 ABI.
//!
//! `AESGLOBAL` is EmuTOS's `struct aesglobal` (`aes/gemrslib.h:15-27`): ten
//! fields, **30 bytes / 15 WORDs**, every `LONG`/pointer field **4 bytes**.
//! Unlike `OBJECT`, this struct is addressed two ways at once — by byte offset
//! *and* by WORD index (`AP_VERSION`..`AP_4RESV`, `include/aesdefs.h:147-157`;
//! every GEM client declares it as `short aes_global[]`). The two are only
//! simultaneously satisfiable with 4-byte pointer fields, so the layout does
//! **not** widen with the caller's pointer width (decision of 2026-09-13;
//! `docs/api/aes/application.md` §`AESGLOBAL`).
//!
//! What `appl_init` may write is the `APPL_INIT` arm's six assignments
//! (`aes/gemsuper.c:92-98`); the other four fields keep whatever the caller
//! had. This test therefore poisons the whole array, runs opcode 10, and
//! checks **every byte**: the six fields against their EmuTOS values, the
//! four others against the poison, and two canary WORDs outside the 15 for
//! an overrun.
//!
//! It also models gemlib's accessors at the widths the NewTOS overlay gives
//! them (`sdk/gemlib/overlay/gem.h`): `_AESappglobal` / `_AESrscfile` /
//! `_AESrscmem` are 4-byte reads at bytes 6 / 10 / 14, `_AESrsclen` and
//! `_AESmaxchar`/`_AESminchar` are the WORDs at 18 and 26 / 28. Read at the
//! pristine gemlib widths instead — eight bytes each, no `__LP64__`
//! conditional — `_AESrscmem` spans bytes 14..21 and so returns
//! `ap_planes << 48`, a wild pointer manufactured by the *reader*. That read
//! is what this test pins as gone.
#![allow(clippy::too_many_arguments, unused_variables)]

use tos_api::bdos::process::Abi;
use tos_api::host::Host;
use tos_api::vdi::backend::{ClipRect, MonoSrc, PatternFill, Pel, RasterBackend, Rect, WriteMode};

/// A metadata-only backend: `appl_init` reads `planes()` for `ap_planes`
/// (`gemsuper.c:97`) and must not draw.
struct Profile;
const PLANES: u16 = 16;
impl RasterBackend for Profile {
    fn width(&self) -> u16 {
        1920
    }
    fn height(&self) -> u16 {
        1080
    }
    fn planes(&self) -> u16 {
        PLANES
    }
    fn num_colors(&self) -> u16 {
        256
    }
    fn set_color(&self, i: u16, r: u16, g: u16, b: u16) {
        panic!("unexpected palette write")
    }
    fn query_color(&self, i: u16) -> (u16, u16, u16) {
        panic!("unexpected palette read")
    }
    fn set_active_palette(&self, p: &[u32; 256]) {
        panic!("unexpected palette switch")
    }
    fn pel_to_device16(&self, p: Pel) -> u16 {
        panic!("unexpected pixel conversion")
    }
    fn device_to_rgb565(&self, p: u32) -> u16 {
        panic!("unexpected pixel conversion")
    }
    fn rgb565_to_device(&self, p: u16) -> u32 {
        panic!("unexpected pixel conversion")
    }
    fn read_pixel(&self, x: i16, y: i16) -> Pel {
        panic!("unexpected pixel read")
    }
    fn fill_rect(&self, x: i16, y: i16, w: i16, h: i16, p: Pel, c: ClipRect) {
        panic!("unexpected drawing")
    }
    fn abline(
        &self,
        x: i16,
        y: i16,
        xx: i16,
        yy: i16,
        p: Pel,
        m: WriteMode,
        c: ClipRect,
        mask: u16,
        last: bool,
    ) -> u16 {
        panic!("unexpected drawing")
    }
    fn fill_pattern(&self, x: i16, y: i16, w: i16, h: i16, p: PatternFill, c: ClipRect) {
        panic!("unexpected drawing")
    }
    fn draw_glyph_sized(
        &self,
        x: i16,
        y: i16,
        g: &[u8],
        w: i16,
        h: i16,
        p: Pel,
        m: WriteMode,
        c: ClipRect,
    ) {
        panic!("unexpected drawing")
    }
    fn blit_mono(&self, d: Rect, s: MonoSrc, f: Pel, b: Pel, m: WriteMode, c: ClipRect) {
        panic!("unexpected drawing")
    }
    fn copy_rect(&self, s: Rect, x: i16, y: i16, op: u8, c: ClipRect) {
        panic!("unexpected drawing")
    }
    fn get_device_pixel(&self, x: i16, y: i16) -> u32 {
        panic!("unexpected pixel read")
    }
    fn put_device_pixel(&self, x: i16, y: i16, p: u32) {
        panic!("unexpected drawing")
    }
    fn flush_rect(&self, x: i16, y: i16, xx: i16, yy: i16) {
        panic!("unexpected flush")
    }
}
struct TestHost;
impl Host for TestHost {
    fn console_write_byte(&self, b: u8) {
        panic!("unexpected console output")
    }
    fn raster(&self) -> Option<&dyn RasterBackend> {
        Some(&Profile)
    }
}
static HOST: TestHost = TestHost;
static HOST_REF: &dyn Host = &HOST;

/// Poison WORD: both halves distinct from any value `appl_init` writes, and
/// distinct from each other so a one-byte shift cannot pass unnoticed.
const POISON: i16 = 0x5a3cu16 as i16;

/// The poison byte expected at byte `off` of a WORD-filled array
/// (little-endian: the low half of `POISON` sits at the even offset).
fn poison_at(off: usize) -> u8 {
    POISON.to_le_bytes()[off % 2]
}

/// A 4-byte poison field at an even byte offset.
const POISON_U32: u32 = 0x5a3c_5a3c;
/// A 2-byte poison field at an even byte offset.
const POISON_U16: u16 = 0x5a3c;

/// The AES's own built-in `DESKTOP` root `ob_spec` — `ap_private`
/// (`gemsuper.c:95-96`, `gem_rsc.c:550-570`), `4419L` = `0x1143` with no
/// `.INF` loaded.
const DESKTOP_OBSPEC: u32 = 0x1143;

/// `appl_init` through the LP64 dispatcher into a poisoned 16-WORD caller
/// array with a canary WORD on each side. Returns (pid, the 16 WORDs, the
/// two canaries).
fn appl_init() -> (i16, [i16; 16], (i16, i16)) {
    // Both tests share one process; the host may only be installed once.
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| tos_api::host::install_host(&HOST_REF));
    // [0] = leading canary, [1..17] = the caller's aes_global[16],
    // [17] = trailing canary.
    let mut arena = [POISON; 18];
    let mut control = [10i16, 0, 1, 0, 0];
    let mut intin = [POISON; 16];
    let mut intout = [POISON; 8];
    let mut addrin = [0u64; 2];
    let mut addrout = [0u64; 2];
    let raw = [
        control.as_mut_ptr() as u64,
        arena[1..].as_mut_ptr() as u64,
        intin.as_mut_ptr() as u64,
        intout.as_mut_ptr() as u64,
        addrin.as_mut_ptr() as u64,
        addrout.as_mut_ptr() as u64,
    ];
    // SAFETY: a well-formed LP64 parameter block, alive for the call.
    unsafe { tos_api::aes::dispatch(raw.as_ptr() as u64, Abi::Lp64) };
    let mut global = [0i16; 16];
    global.copy_from_slice(&arena[1..17]);
    (intout[0], global, (arena[0], arena[17]))
}

/// The 30 bytes of the `AESGLOBAL` (plus the two gemlib declares beyond it).
fn bytes(global: &[i16; 16]) -> [u8; 32] {
    let mut out = [0u8; 32];
    for (i, w) in global.iter().enumerate() {
        out[2 * i..2 * i + 2].copy_from_slice(&w.to_le_bytes());
    }
    out
}

/// A 4-byte field at byte `off` — how the NewTOS gemlib overlay reads
/// `_AESappglobal` (6), `_AESrscfile` (10) and `_AESrscmem` (14).
fn u32_at(b: &[u8; 32], off: usize) -> u32 {
    u32::from_le_bytes(b[off..off + 4].try_into().unwrap())
}

/// An 8-byte field at byte `off` — how **pristine** gemlib reads the same
/// three macros on an LP64 target (`gem.h:41,49,57`, no `__LP64__` arm).
fn u64_at(b: &[u8; 32], off: usize) -> u64 {
    u64::from_le_bytes(b[off..off + 8].try_into().unwrap())
}

/// Every byte of the caller's `AESGLOBAL` after `appl_init`, against the six
/// fields `gemsuper.c:92-98` writes and the four it does not.
#[test]
fn appl_init_lays_the_tos_aesglobal_byte_for_byte() {
    let (pid, global, canaries) = appl_init();
    assert!(pid >= 0, "appl_init failed");
    let b = bytes(&global);

    // The six the APPL_INIT arm writes.
    assert_eq!(
        u16::from_le_bytes([b[0], b[1]]),
        0x0340,
        "ap_version @0 (gemsuper.c:92)"
    );
    assert_eq!(
        u16::from_le_bytes([b[2], b[3]]),
        1,
        "ap_count @2 (gemsuper.c:93)"
    );
    assert_eq!(
        i16::from_le_bytes([b[4], b[5]]),
        pid,
        "ap_id @4 (gemsuper.c:94)"
    );
    assert_eq!(
        u32_at(&b, 6),
        DESKTOP_OBSPEC,
        "ap_private @6, four bytes (gemsuper.c:95-96)"
    );
    assert_eq!(
        u16::from_le_bytes([b[20], b[21]]),
        PLANES,
        "ap_planes @20 (gemsuper.c:97)"
    );
    assert_ne!(u32_at(&b, 22), 0, "ap_3resv @22 (gemsuper.c:98)");

    // The four it does not: ap_ptree @10, ap_rscmem @14, ap_rsclen @18 and
    // ap_4resv @26 keep the caller's bytes. `rs_load`/`rs_free` own the
    // first three (`gemrslib.c:597,787-788`); nothing owns ap_4resv.
    for off in (10..20).chain(26..30) {
        assert_eq!(
            b[off],
            poison_at(off),
            "appl_init wrote byte {off}, which belongs to a field the \
             APPL_INIT arm does not name (gemsuper.c:92-98)"
        );
    }

    // Nothing past the 15-WORD struct, and nothing before it.
    assert_eq!(
        &b[30..32],
        &[poison_at(30), poison_at(31)],
        "write past the 30-byte AESGLOBAL"
    );
    assert_eq!(canaries, (POISON, POISON), "write outside the caller array");
}

/// The same result read through gemlib's accessors, at the widths the NewTOS
/// overlay gives them — and the value the pristine 8-byte readers produce,
/// which is what the overlay exists to remove.
#[test]
fn gemlib_accessors_see_the_tos_fields() {
    let (pid, global, _) = appl_init();
    assert!(pid >= 0, "appl_init failed");
    let b = bytes(&global);

    // Overlay widths: each field is its own four or two bytes.
    assert_eq!(u32_at(&b, 6), DESKTOP_OBSPEC, "_AESappglobal");
    assert_eq!(u32_at(&b, 10), POISON_U32, "_AESrscfile");
    assert_eq!(u32_at(&b, 14), POISON_U32, "_AESrscmem");
    assert_eq!(u16::from_le_bytes([b[18], b[19]]), POISON_U16, "_AESrsclen");
    assert_eq!(u16::from_le_bytes([b[26], b[27]]), POISON_U16, "_AESmaxchar");
    assert_eq!(u16::from_le_bytes([b[28], b[29]]), POISON_U16, "_AESminchar");

    // Pristine widths, for the record: `_AESrscmem` reaches bytes 14..21 and
    // therefore drags `ap_planes` in as bits 48..63. With a caller array that
    // starts out zeroed (the .bss case, `globals.c:40`) that read is exactly
    // `planes << 48` — nonzero, so `if (_AESrscmem)` takes the wrong branch.
    let wide = u64_at(&b, 14);
    assert_eq!(
        wide >> 48,
        PLANES as u64,
        "the pristine 8-byte _AESrscmem still spans ap_planes — that is the \
         read the NewTOS overlay narrows, not a value appl_init controls"
    );
}
