//! A40 — the char→pixel coordinate fix (`rs_obfix`) runs at **load time**,
//! with the live `gl_wchar`/`gl_hchar`/`gl_width`, not in the conversion tool.
//!
//! EmuTOS stores every object's `ob_x`/`ob_y`/`ob_width`/`ob_height` in the
//! resource file as a packed char-grid word — character count in the low
//! byte, signed pixel offset in the high byte — and converts them when the
//! file is loaded (`references/emutos/aes/gemrslib.c`, pinned tree
//! `1a7484aba1bfba23e2c0899e243b6d8177fe41df`):
//!
//! ```c
//! /* fix_chpos, :53-83 */
//! coffset = HIBYTE(cpos);  cpos = LOBYTE(cpos);
//! case 0: cpos *= gl_wchar;                                    /* ob_x      */
//! case 1: cpos *= gl_hchar;                                    /* ob_y      */
//! case 2: if (cpos == 80) cpos = gl_width; else cpos *= gl_wchar; /* ob_width */
//! case 3: cpos *= gl_hchar;                                    /* ob_height */
//! cpos += ( coffset > 128 ) ? (coffset - 256) : coffset;
//! /* rs_obfix, :89-99: the four words of one object */
//! /* fix_objects, :606-637: rs_obfix(obj, 0) for every rsh_nobs object   */
//! /* rs_fixit, :816-820: called by rs_load (:846) after rs_readit         */
//! ```
//!
//! `gl_wchar`/`gl_hchar` are the AES workstation's font cell (`gemgraf.c:405`),
//! `gl_width` its pixel width (`:393`) — the values of the machine the file
//! is loaded on. `rsrc_obfix` (114) is the very same `rs_obfix`
//! (`gemsuper.c:375-377`).
//!
//! NewTOS's converter (`tools/mkrsc.py`) did that arithmetic **at conversion
//! time** with `WCHAR, HCHAR, SCREEN_W = 8, 16, 640` baked in, and the loader
//! converted nothing — so a resource carried the pixels of a 640-wide 8×16
//! screen onto every machine (audit B10,
//! `docs/analysis/audit-applinit-rsrc-load-2026-09-11.md`). From NRSC v9
//! (LP64) / v10 (ILP32) on, the blob carries the packed words unchanged and
//! the loader runs `rs_obfix` over every object at load.
//!
//! **Rig.** A metadata-only raster backend whose width is switchable (the rig
//! of `aes_wind_get_desktop_geometry.rs`), so `gl_width` can be the TT's 640
//! or the Pi 4's 1920. The font cell comes from `graf_handle`'s `int_out[1..2]`
//! — the AES's own reading of `gl_wchar`/`gl_hchar` — so the expectation
//! follows whatever the VDI's default face is. The blob is hand-built (a v9
//! header, packed words in the four coordinate fields) and loaded through
//! `resource::relocate_and_register`, the in-place relocation + registration
//! `rsrc_load`'s in-kernel branch runs (see `aes_rsrc_per_process.rs` for why
//! the EL0 `load_into_user` path is out of a host test's reach); the objects
//! are read back through `rsrc_gaddr(R_OBJECT, i)`, the real dispatcher.
//!
//! Ticket: `TASK/A40.md`.

#![allow(clippy::too_many_arguments, unused_variables)]

use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::{Mutex, MutexGuard, Once};

use tos_api::bdos::process::Abi;
use tos_api::host::Host;
use tos_api::vdi::backend::{ClipRect, MonoSrc, PatternFill, Pel, RasterBackend, Rect, WriteMode};

// ── the switchable metadata-only backend ─────────────────────────────────

/// The geometry the reference dumps were measured at (Atari TT, 640×480).
const TT: (u16, u16) = (640, 480);
/// The current hardware profile (Pi 4, 1920×1080) — a test profile, not a
/// new video mode: the backend below only *reports* it.
const PI4: (u16, u16) = (1920, 1080);

struct Profile {
    width: AtomicU16,
    height: AtomicU16,
}

static PROFILE: Profile = Profile {
    width: AtomicU16::new(TT.0),
    height: AtomicU16::new(TT.1),
};

/// Set while the fixture runs `gsx_init`: its `v_opnwk` clears the VT52
/// console (`esc_init`, `vdi_control.c:564`, CON04), the one drawing the
/// fixture itself does — screen words through the row interface and their
/// flush. Every other moment still panics on any drawing.
static CONSOLE_SETUP: AtomicBool = AtomicBool::new(false);

fn console_setup() -> bool {
    CONSOLE_SETUP.load(Ordering::Relaxed)
}

impl RasterBackend for Profile {
    fn width(&self) -> u16 {
        self.width.load(Ordering::Relaxed)
    }
    fn height(&self) -> u16 {
        self.height.load(Ordering::Relaxed)
    }
    fn planes(&self) -> u16 {
        4
    }
    fn num_colors(&self) -> u16 {
        16
    }
    fn set_color(&self, i: u16, r: u16, g: u16, b: u16) {
        panic!("unexpected palette write")
    }
    fn query_color(&self, i: u16) -> (u16, u16, u16) {
        panic!("unexpected palette read")
    }
    fn set_active_palette(&self, _p: &[u32; 256]) {
        // Every VDI call on an open workstation selects its pseudo-palette
        // (`tos-api/src/vdi/mod.rs`); the AES's own `gsx_init` set-up calls
        // (`gemgraf.c:405-422`) are such calls. Nothing is drawn with it.
    }
    fn pel_to_device16(&self, p: Pel) -> u16 {
        panic!("unexpected pixel conversion")
    }
    fn device_to_rgb565(&self, p: u32) -> u16 {
        if console_setup() {
            return p as u16;
        }
        panic!("unexpected pixel conversion")
    }
    fn rgb565_to_device(&self, p: u16) -> u32 {
        if console_setup() {
            return p as u32;
        }
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
        if console_setup() {
            return 0;
        }
        panic!("unexpected pixel read")
    }
    fn put_device_pixel(&self, x: i16, y: i16, p: u32) {
        if console_setup() {
            return;
        }
        panic!("unexpected drawing")
    }
    fn flush_rect(&self, x: i16, y: i16, xx: i16, yy: i16) {
        if console_setup() {
            return;
        }
        panic!("unexpected flush")
    }
}

struct TestHost;

impl Host for TestHost {
    fn console_write_byte(&self, b: u8) {
        panic!("unexpected console output")
    }
    fn raster(&self) -> Option<&dyn RasterBackend> {
        Some(&PROFILE)
    }
}

static HOST: TestHost = TestHost;
static HOST_REF: &dyn Host = &HOST;

/// The installed host, the raster profile and the AES resource table are
/// process-wide; the tests of this binary run one at a time.
static LOCK: Mutex<()> = Mutex::new(());
static INSTALL: Once = Once::new();

fn lock() -> MutexGuard<'static, ()> {
    INSTALL.call_once(|| {
        tos_api::host::install_host(&HOST_REF);
        // The AES session's workstation open (`gsx_init`, `geminit.c:675`),
        // whose handle `graf_handle` reports.
        CONSOLE_SETUP.store(true, Ordering::Relaxed);
        tos_api::aes::gsxif::gsx_init();
        CONSOLE_SETUP.store(false, Ordering::Relaxed);
    });
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn screen(w: u16, h: u16) {
    PROFILE.width.store(w, Ordering::Relaxed);
    PROFILE.height.store(h, Ordering::Relaxed);
}

// ── AES calls through the real dispatcher ─────────────────────────────────

const OP_GRAF_HANDLE: i16 = 77;
const OP_RSRC_FREE: i16 = 111;
const OP_RSRC_GADDR: i16 = 112;
const OP_RSRC_OBFIX: i16 = 114;
const R_OBJECT: i16 = 1;

fn call(op: i16, ni: i16, no: i16, nai: i16, intin: [i16; 4], addrin: [u64; 2]) -> ([i16; 8], u64) {
    let mut control = [op, ni, no, nai, 0];
    let mut global = [0i16; 16];
    let mut intin = intin;
    let mut intout = [0i16; 8];
    let mut addrin = addrin;
    let mut addrout = [0u64; 2];
    let raw = [
        control.as_mut_ptr() as u64,
        global.as_mut_ptr() as u64,
        intin.as_mut_ptr() as u64,
        intout.as_mut_ptr() as u64,
        addrin.as_mut_ptr() as u64,
        addrout.as_mut_ptr() as u64,
    ];
    // SAFETY: a well-formed LP64 parameter block, alive for the call.
    unsafe { tos_api::aes::dispatch(raw.as_ptr() as u64, Abi::Lp64) };
    (intout, addrout[0])
}

/// `graf_handle` → `(gl_wchar, gl_hchar)` from `int_out[1..2]`
/// (`gemsuper.c:288-289`): the AES's own reading of the system-font cell.
fn font_cell() -> (i16, i16) {
    let (out, _) = call(OP_GRAF_HANDLE, 0, 5, 0, [0; 4], [0; 2]);
    assert!(out[0] > 0, "graf_handle returned no handle ({})", out[0]);
    (out[1], out[2])
}

/// `rsrc_gaddr(R_OBJECT, i)` → the object's address.
fn object_addr(i: i16) -> u64 {
    let (out, addr) = call(OP_RSRC_GADDR, 2, 1, 0, [R_OBJECT, i, 0, 0], [0; 2]);
    assert_eq!(out[0], 1, "rsrc_gaddr(R_OBJECT, {i}) must resolve");
    addr
}

/// `rsrc_free()` — so one test's loaded resource is not another's.
fn rsrc_free() {
    let (out, _) = call(OP_RSRC_FREE, 0, 1, 0, [0; 4], [0; 2]);
    assert_eq!(out[0], 1, "rsrc_free must find the load");
}

/// `rsrc_obfix(tree, obj)` (114) — in place, on the caller's own tree
/// (`int_in[0]` = object, `addr_in[0]` = tree, `AIN_LEN` 1; `gemsuper.c:375-377`).
fn rsrc_obfix(tree: u64, obj: i16) {
    let (out, _) = call(OP_RSRC_OBFIX, 1, 1, 1, [obj, 0, 0, 0], [tree, 0]);
    assert_eq!(
        out[0], 1,
        "rsrc_obfix reports 1 (rs_obfix is void, ret stays TRUE)"
    );
}

// ── the rule, restated from gemrslib.c:53-83 ──────────────────────────────

/// `fix_chpos` for one packed word: `LOBYTE × cell` (or `gl_width` for the
/// 80-character width), plus the signed `HIBYTE` (`> 128 ⇒ − 256`).
fn fix_chpos(word: u16, cell: i16, is_width: bool, gl_width: i16) -> i16 {
    let coffset = (word >> 8) as i32;
    let chars = (word & 0xff) as i32;
    let base = if is_width && chars == 80 {
        gl_width as i32
    } else {
        chars * cell as i32
    };
    (base
        + if coffset > 128 {
            coffset - 256
        } else {
            coffset
        }) as i16
}

/// The four pixel values `rs_obfix` gives an object whose packed words are
/// `raw`, at cell `(wchar, hchar)` and screen width `gl_width`.
fn expect(raw: [u16; 4], wchar: i16, hchar: i16, gl_width: i16) -> [i16; 4] {
    [
        fix_chpos(raw[0], wchar, false, gl_width),
        fix_chpos(raw[1], hchar, false, gl_width),
        fix_chpos(raw[2], wchar, true, gl_width),
        fix_chpos(raw[3], hchar, false, gl_width),
    ]
}

// ── a hand-built NRSC blob with char-grid coordinates ─────────────────────

/// The three objects' packed words, chosen so every clause of `fix_chpos`
/// is exercised at least once:
///
/// | obj | word | meaning |
/// |---|---|---|
/// | 0 | `x 0x0000, y 0x0000, w 0x0050, h 0x0019` | a full-width root: width char **80** → `gl_width`; 25 rows |
/// | 1 | `x 0x0405, y 0xFF02, w 0x0208, h 0x0001` | 5 chars + 4 px; 2 rows − 1 px; 8 chars + 2 px; 1 row |
/// | 2 | `x 0x8001, y 0x8101, w 0x0010, h 0x0102` | offset **128** stays +128 (`> 128` is false); 129 → −127; 16 chars (not 80); 2 rows + 1 px |
const RAW: [[u16; 4]; 3] = [
    [0x0000, 0x0000, 0x0050, 0x0019],
    [0x0405, 0xFF02, 0x0208, 0x0001],
    [0x8001, 0x8101, 0x0010, 0x0102],
];

/// First NRSC version whose OBJECT coordinates are packed char-grid words.
const NRSC_VER_CHARGRID: u32 = 9;

const HEADER_SIZE: usize = 68;
const OBJECT_SIZE: usize = 32;
const S_TRINDEX: usize = HEADER_SIZE; // one 8-byte slot, padded to 80
const S_OBJECTS: usize = 80;
const S_N_OBJECTS: usize = 3;
const S_STRINGS: usize = S_OBJECTS + S_N_OBJECTS * OBJECT_SIZE; // 176
const S_TOTAL: usize = 192;

const G_BOX: i16 = 20;
const G_BUTTON: i16 = 26;
const LASTOB: i16 = 0x0020;

/// A v9 (LP64, char-grid) blob — or, with `version` 7, the same bytes as a
/// pixel-coordinate blob of the previous generation.
fn blob(version: u32) -> Vec<u8> {
    let mut b = vec![0u8; S_TOTAL];
    let put32 =
        |b: &mut Vec<u8>, off: usize, v: u32| b[off..off + 4].copy_from_slice(&v.to_le_bytes());
    let put64 =
        |b: &mut Vec<u8>, off: usize, v: u64| b[off..off + 8].copy_from_slice(&v.to_le_bytes());
    let put16 =
        |b: &mut Vec<u8>, off: usize, v: u16| b[off..off + 2].copy_from_slice(&v.to_le_bytes());

    b[0..4].copy_from_slice(b"NRSC");
    put32(&mut b, 4, version);
    put32(&mut b, 8, 1); // n_trees
    put32(&mut b, 12, S_N_OBJECTS as u32);
    put32(&mut b, 16, S_TRINDEX as u32);
    put32(&mut b, 20, S_OBJECTS as u32);
    put32(&mut b, 24, S_TOTAL as u32);
    for (off, v) in [
        (28, S_STRINGS),
        (36, S_STRINGS),
        (44, S_STRINGS),
        (52, S_STRINGS),
        (60, S_STRINGS),
    ] {
        put32(&mut b, off, v as u32); // off_frstr / off_frimg / off_ted / off_ib / off_bb
    }
    // n_frstr (32), n_frimg (40), n_ted (48), n_ib (56), n_bb (64) stay 0.

    // Tree index (v7+): one pointer-wide slot holding the root's byte offset.
    put64(&mut b, S_TRINDEX, S_OBJECTS as u64);

    // The one string, for the button.
    b[S_STRINGS..S_STRINGS + 3].copy_from_slice(b"OK\0");

    let obj = |b: &mut Vec<u8>,
               i: usize,
               next: i16,
               head: i16,
               tail: i16,
               ty: i16,
               flags: i16,
               spec: u64| {
        let o = S_OBJECTS + i * OBJECT_SIZE;
        put16(b, o, next as u16);
        put16(b, o + 2, head as u16);
        put16(b, o + 4, tail as u16);
        put16(b, o + 6, ty as u16);
        put16(b, o + 8, flags as u16);
        put16(b, o + 10, 0);
        put64(b, o + 16, spec);
        for (k, w) in RAW[i].iter().enumerate() {
            put16(b, o + 24 + 2 * k, *w);
        }
    };
    obj(&mut b, 0, -1, 1, 2, G_BOX, 0, 0x0001_1100);
    obj(&mut b, 1, 2, -1, -1, G_BUTTON, 0, S_STRINGS as u64);
    obj(&mut b, 2, 0, -1, -1, G_BOX, LASTOB, 0x0000_1100);
    b
}

/// An 8-byte-aligned, writable copy of `src`, kept alive by the caller.
fn aligned_copy(src: &[u8]) -> (Vec<u64>, *mut u8) {
    let mut v = vec![0u64; src.len().div_ceil(8) + 1];
    let p = v.as_mut_ptr() as *mut u8;
    // SAFETY: `v` holds at least `src.len()` bytes.
    unsafe { std::ptr::copy_nonoverlapping(src.as_ptr(), p, src.len()) };
    (v, p)
}

/// Load `src` through the in-kernel `rsrc_load` branch.
fn load(src: &[u8]) -> (Vec<u64>, *mut u8) {
    let (v, p) = aligned_copy(src);
    // SAFETY: `p` addresses `src.len()` writable bytes that live in `v`.
    let ok = unsafe { tos_api::aes::resource::relocate_and_register(p, src.len()) };
    assert!(ok, "the blob must relocate and register");
    (v, p)
}

/// `(ob_x, ob_y, ob_width, ob_height)` of the LP64 object at `addr`.
fn coords(addr: u64) -> [i16; 4] {
    let mut out = [0i16; 4];
    for (k, o) in out.iter_mut().enumerate() {
        // SAFETY: `addr` is an object inside a live blob.
        *o = unsafe { ((addr + 24 + 2 * k as u64) as *const i16).read_unaligned() };
    }
    out
}

/// Load the v9 blob on a `w`×`h` screen and return every object's pixel
/// coordinates as `rsrc_gaddr` hands them out, plus the metrics the
/// expectation is built from.
fn loaded_at(w: u16, h: u16) -> (Vec<[i16; 4]>, (i16, i16)) {
    screen(w, h);
    let (wchar, hchar) = font_cell();
    let (_keep, _) = load(&blob(NRSC_VER_CHARGRID));
    let got: Vec<[i16; 4]> = (0..S_N_OBJECTS as i16)
        .map(|i| coords(object_addr(i)))
        .collect();
    rsrc_free();
    (got, (wchar, hchar))
}

// ── the tests ────────────────────────────────────────────────────────────

/// **The acceptance case of A40:** on a 1920-wide screen the 80-character
/// root is 1920 px wide, not 640 — and every other word is the live font
/// cell times its char count, plus its signed offset.
#[test]
fn a_char_grid_resource_is_fixed_with_the_screen_it_is_loaded_on() {
    let _g = lock();
    let (got, (wchar, hchar)) = loaded_at(PI4.0, PI4.1);
    for (i, raw) in RAW.iter().enumerate() {
        assert_eq!(
            got[i],
            expect(*raw, wchar, hchar, PI4.0 as i16),
            "object {i}: rs_obfix of {raw:04x?} at cell {wchar}x{hchar}, gl_width {}",
            PI4.0
        );
    }
    assert_eq!(
        got[0][2], PI4.0 as i16,
        "the 80-character width is the full screen width"
    );
    assert_ne!(got[0][2], 640, "… and not the converter's frozen 640");
}

/// The same file on the TT's 640-wide screen gives the pixels the converter
/// used to bake in — which is why nothing changes at 8×16/640: at that
/// profile the load-time result equals the old pre-converted one.
#[test]
fn at_640_the_load_time_fix_equals_the_old_pre_converted_pixels() {
    let _g = lock();
    let (got, (wchar, hchar)) = loaded_at(TT.0, TT.1);
    for (i, raw) in RAW.iter().enumerate() {
        assert_eq!(
            got[i],
            expect(*raw, wchar, hchar, TT.0 as i16),
            "object {i}"
        );
    }
    assert_eq!(got[0][2], 640);
    // The values the converter's WCHAR/HCHAR/SCREEN_W = 8/16/640 produced,
    // written out for the 8×16 face (the VDI's default; the assertion above
    // holds for any face, this one pins the numbers a reader expects).
    if (wchar, hchar) == (8, 16) {
        assert_eq!(got[1], [44, 31, 66, 16]);
        assert_eq!(got[2], [136, -111, 128, 33]);
    }
}

/// `rsrc_obfix` (114) is `rs_obfix` itself: applied by the application to
/// the raw words, it gives exactly what the loader gave.
#[test]
fn rsrc_obfix_computes_what_the_loader_computes() {
    let _g = lock();
    let (from_loader, _) = loaded_at(PI4.0, PI4.1);
    // A copy of the *unconverted* objects, fixed one by one through opcode
    // 114 — no load, no relocation, the tree is the caller's own.
    let (_keep, p) = aligned_copy(&blob(NRSC_VER_CHARGRID));
    let tree = p as u64 + S_OBJECTS as u64;
    for i in 0..S_N_OBJECTS as i16 {
        rsrc_obfix(tree, i);
        assert_eq!(
            coords(tree + i as u64 * OBJECT_SIZE as u64),
            from_loader[i as usize],
            "object {i}: rsrc_obfix and the loader must agree"
        );
    }
}

/// A blob of the previous generation (v7, pixel coordinates) is not touched:
/// the loader must not double-convert what the tool already converted.
#[test]
fn a_pixel_coordinate_blob_is_left_alone() {
    let _g = lock();
    screen(PI4.0, PI4.1);
    let (_keep, _) = load(&blob(7));
    for (i, raw) in RAW.iter().enumerate() {
        let got = coords(object_addr(i as i16));
        let as_pixels = [raw[0] as i16, raw[1] as i16, raw[2] as i16, raw[3] as i16];
        assert_eq!(
            got, as_pixels,
            "object {i}: a v7 word is a pixel value and stays one"
        );
    }
    rsrc_free();
}
