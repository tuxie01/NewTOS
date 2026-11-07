//! AES boot-time initialisation — the parts of EmuTOS `aes/geminit.c` that
//! set up AES state an application can observe **before** it ever calls a
//! manager: the AES's own built-in `DESKTOP` object tree and the desktop
//! background byte read from the `.INF` file.
//!
//! Spec: `docs/api/aes/shell-loop.md` §1 (`gem_main`), §2 (`process_inf1`),
//! §11 (the `ap_private` chain).
//!
//! Three EmuTOS steps live here, in EmuTOS's order:
//!
//! 1. **`process_inf1`** (`geminit.c:343-392`, called from `:776`) scans the
//!    first `INF_SIZE` bytes of the `.INF` file for a `#Q` line and stores
//!    the three desktop background bytes in `bg[]`, setting `bgfound`.
//! 2. **`gem_rsc_fixit`** (`gem_rsc.c:636-641`) converts the built-in
//!    resource's char-grid coordinates to pixels once the VDI workstation is
//!    open, and `run_accs_and_desktop` then forces the `DESKTOP` root to the
//!    full screen height (`geminit.c:715-717`).
//! 3. **`set_aes_background`** (`geminit.c:635-641`), called from
//!    `geminit.c:719-728` when `#Q` was found, ORs the plane-appropriate
//!    background byte into the low byte of that root's `ob_spec`.
//!
//! The result is the value every `appl_init` reports as `ap_private`
//! (`gemsuper.c:94-96`) — `0x1143` (= 4419) with no `#Q` line, `0x1100 |
//! patcol` with one. It is **not** derived from any desktop-supplied tree:
//! `appl_init` indexes the AES's own compiled-in `gem.rsc`
//! (`gem_rsc.c:574-578`, index `DESKTOP` = 2 per `gem_rsc.h:48`), and the
//! tree a desktop registers with `wind_set(WF_NEWDESK)` lands in a different
//! variable entirely (`gemwmlib.c:1687`).

use core::cell::UnsafeCell;

use super::object::{ObjTree, Object, TedInfo};
use tos_types::aes::{G_BOX, G_TEXT, LASTOB, ROOT, TE_CNTR};

/// `DTNAME` — object 2 of the built-in `DESKTOP` tree, the `G_TEXT` that
/// `sh_draw` fills with the name of the program being launched
/// (`gem_rsc.h:49`, `gemshlib.c:284-286`).
const DTNAME: usize = 2;

/// `IBM` font id / `TE_CNTR` justification (`obdefs.h:62,255`).
const IBM: i16 = 3;

/// Number of bytes of the `.INF` file `process_inf1` looks at — EmuTOS
/// `INF_SIZE` (`geminit.c:99`, used at `:769`). Anything past it is invisible
/// to the AES; the desktop writes `#R`/`#Z`/`#E`/`#Q` first, so the `#Q` line
/// is always well inside it.
const INF_SIZE: usize = 300;

/// The `.INF` file the AES reads at boot. EmuTOS uses `EMUDESK.INF`
/// (`INF_FILE_NAME`); NewTOS uses the Atari-classic name, the same file the
/// desktop reads for its own `#Q` copy (`programs/desktop/src/inf.rs:51`).
const INF_PATH: &str = "C:\\NEWDESK.INF";

/// `ob_spec` of the built-in `DESKTOP` tree's ROOT — the literal `4419L` of
/// `gem_rsc.c:551-556`. `4419 == 0x1143`; a real TT running EmuTOS reports
/// exactly this in `appl_init`'s `ap_private`. `set_aes_background` only ever
/// rewrites its low byte, so `0x1100` is a compiled-in constant.
const DESKTOP_ROOT_OBSPEC: u64 = 4419;

/// `ob_spec` of object 1, the background box (`gem_rsc.c:558-563`).
const DESKTOP_BOX_OBSPEC: u64 = 16_716_032;

/// The `TEDINFO` object 2 (`DTNAME`) points at — `rs_tedinfo[12]`
/// (`gem_rsc.c:88-91` in the ROM table: `{0L, "", "", IBM, 1, TE_CNTR, 4352,
/// 0, 1, 2, 1}`). `te_ptext` is NULL until `sh_draw` points it at the command
/// name (`gemshlib.c:284-285`); the two string fields are the empty string.
static DTNAME_TEXT: [u8; 1] = [0];

/// Wrapper giving the built-in `DESKTOP` tree interior mutability. EmuTOS
/// mutates `rs_trees[DESKTOP]` in place (`gem_rsc_fixit`, `set_aes_background`,
/// the `ob_height` fixup); so do we.
struct DesktopTree(UnsafeCell<[Object; 3]>);

// SAFETY: single boot CPU and a cooperative (non-preemptive) trap path — the
// same discipline as every other mutable AES/BIOS singleton in this crate
// (e.g. `xbios::kbdvecs::KERNEL_TABLE`). Writers are the boot-time [`init`] and
// [`set_aes_background`]; readers are `appl_init` and the drawing path.
unsafe impl Sync for DesktopTree {}

/// `rs_obj_rom`'s `DESKTOP` entries (`aes/gem_rsc.c:550-570`) — the **ROM
/// master** of the AES's own built-in `DESKTOP` tree, three objects, in the
/// resource's packed char-grid form.
///
/// EmuTOS keeps this copy because `gem_rsc_fixit`'s coordinate conversion
/// (`rs_obfix`, char cells → pixels) is **not** idempotent: every AES session
/// starts with `gem_rsc_init`'s `memcpy(rs_obj, rs_obj_rom, …)`
/// (`gem_rsc.c:624-627`) so the conversion always runs on unconverted data.
/// [`gem_rsc`] does the same, which is what makes an AES restart (A106) land
/// on the same geometry a cold boot does instead of scaling the tree twice.
const DESKTOP_ROM: [Object; 3] = [
    // [0] ROOT: { -1, 1, 2, G_BOX, NONE, NORMAL, 4419L, 0,0,80,25 }
    Object {
        ob_next: -1,
        ob_head: 1,
        ob_tail: 2,
        ob_type: G_BOX,
        ob_flags: 0,
        ob_state: 0,
        ob_spec: DESKTOP_ROOT_OBSPEC,
        ob_x: 0,
        ob_y: 0,
        ob_width: 80,
        ob_height: 25,
    },
    // [1]: { 2, -1, -1, G_BOX, NONE, NORMAL, 16716032L, 0,0,80,513 }
    Object {
        ob_next: 2,
        ob_head: -1,
        ob_tail: -1,
        ob_type: G_BOX,
        ob_flags: 0,
        ob_state: 0,
        ob_spec: DESKTOP_BOX_OBSPEC,
        ob_x: 0,
        ob_y: 0,
        ob_width: 80,
        ob_height: 513,
    },
    // [2] DTNAME: { 0, -1, -1, G_TEXT, LASTOB, NORMAL, &rs_tedinfo[12],
    //               0,0,80,769 } — `ob_spec` is patched in `init` (a static
    //               cannot take the address of another static here).
    Object {
        ob_next: 0,
        ob_head: -1,
        ob_tail: -1,
        ob_type: G_TEXT,
        ob_flags: LASTOB,
        ob_state: 0,
        ob_spec: 0,
        ob_x: 0,
        ob_y: 0,
        ob_width: 80,
        ob_height: 769,
    },
];

/// `rs_trees[DESKTOP]` — the live (RAM) copy of the tree above, the one
/// `gem_rsc_fixit`, `set_aes_background` and the `ob_height` fixup mutate
/// (`gem_rsc.c:573-578`, `rs_trees[]` pointing into `rs_obj`). Re-seeded from
/// [`DESKTOP_ROM`] at the start of every AES session by [`gem_rsc`].
static DESKTOP: DesktopTree = DesktopTree(UnsafeCell::new(DESKTOP_ROM));

/// `infbuf` (`geminit.c:110`) — the first [`INF_SIZE`] bytes of the `.INF`
/// file, read once per AES session in `gem_main` (`:766-773`) and used by both
/// [`process_inf1_q`] (`:776`) and [`process_inf2`] (`:734`).
struct InfBuf(UnsafeCell<[u8; INF_SIZE + 1]>);
// SAFETY: as [`DesktopTree`] — single boot CPU, cooperative trap path.
unsafe impl Sync for InfBuf {}
static INFBUF: InfBuf = InfBuf(UnsafeCell::new([0; INF_SIZE + 1]));

/// `rs_tedinfo[12]`, the `TEDINFO` `DTNAME` points at (`gem_rsc.c:88-91`).
struct DtNameTed(UnsafeCell<TedInfo>);
// SAFETY: as [`DesktopTree`] — single CPU, cooperative trap path.
unsafe impl Sync for DtNameTed {}

static DTNAME_TED: DtNameTed = DtNameTed(UnsafeCell::new(TedInfo {
    te_ptext: 0, // `0L` — `sh_draw` points it at the command name
    te_ptmplt: 0,
    te_pvalid: 0,
    te_font: IBM,
    te_junk1: 1,
    te_just: TE_CNTR,
    te_color: 4352,
    te_junk2: 0,
    te_thickness: 1,
    te_txtlen: 2,
    te_tmplen: 1,
}));

/// `bg[]` + `bgfound` (`geminit.c:112-113`) — the three desktop background
/// bytes of the `.INF` `#Q` line, `None` while no `#Q` line has been seen
/// (`bgfound == FALSE`).
///
/// Both are plain AES globals, **outside** the `theglo` area `aes_restart`
/// re-zeroes (`gemstart.S:101-110`), and `gem_main` re-runs `process_inf1`
/// only when the restart is *not* a resolution change (`geminit.c:775`). So
/// across an A106 restart the previous session's `#Q` result is what
/// `run_accs_and_desktop` re-applies (`:719-728`) — kept here for the same
/// reason.
struct BgCell(UnsafeCell<Option<[i16; 3]>>);
// SAFETY: as [`DesktopTree`] — single boot CPU, cooperative trap path.
unsafe impl Sync for BgCell {}
static BG: BgCell = BgCell(UnsafeCell::new(None));

/// Base pointer of the built-in `DESKTOP` tree — EmuTOS `rs_trees[DESKTOP]`.
#[inline]
pub(crate) fn desktop_tree() -> *mut Object {
    DESKTOP.0.get() as *mut Object
}

/// Point `DTNAME`'s `TEDINFO.te_ptext` at `p` — `sh_draw(lcmd, FALSE)`'s
/// `ted->te_ptext = (char *)lcmd` (`gemshlib.c:284-285`), the program name
/// shown while a program starts.
pub(crate) fn set_dtname_text(p: u64) {
    // SAFETY: single-CPU AES; `DTNAME_TED` is an always-initialised static.
    unsafe { (*DTNAME_TED.0.get()).te_ptext = p };
}

/// `rs_trees[DESKTOP][ROOT].ob_spec` — the word `appl_init` hands back as
/// `ap_private` (`gemsuper.c:95-96`).
pub(crate) fn desktop_root_obspec() -> u64 {
    // SAFETY: the static tree is always initialised and three objects long;
    // single-CPU cooperative access (see [`DesktopTree`]).
    unsafe { (*desktop_tree().add(ROOT as usize)).ob_spec }
}

/// `set_aes_background` (`geminit.c:635-641`) — replace the low byte of the
/// `DESKTOP` root's `ob_spec` with the desktop background pattern/colour
/// byte. The upper 24 bits of the resource constant are preserved verbatim,
/// which is what keeps `ap_private`'s high half at `0x1100`.
pub(crate) fn set_aes_background(patcol: u8) {
    // SAFETY: as [`desktop_root_obspec`].
    unsafe {
        let root = &mut *desktop_tree().add(ROOT as usize);
        root.ob_spec &= 0xffff_ff00;
        root.ob_spec |= patcol as u64;
    }
}

/// `scan_2` (`util/optimize.c:202-217`) — skip blanks, then convert the next
/// two hex digits to a value; a `\r` before them yields `0` and consumes
/// nothing. `0xff` decodes to `-1`. Returns `(value, bytes consumed)`.
fn scan_2(buf: &[u8]) -> (i16, usize) {
    let mut i = 0;
    while i < buf.len() && buf[i] == b' ' {
        i += 1;
    }
    if i >= buf.len() || buf[i] == b'\r' {
        return (0, i);
    }
    // `hex_dig` (`optimize.c:181-187`): `c >= 'A' ? c + 9 : c`, then `& 0x0f`.
    let hex_dig = |c: u8| -> i16 { (if c >= b'A' { c.wrapping_add(9) } else { c } & 0x0f) as i16 };
    let hi = hex_dig(buf[i]);
    i += 1;
    let lo = if i < buf.len() { hex_dig(buf[i]) } else { 0 };
    i += 1;
    let v = (hi << 4) | lo;
    (if v == 0x00ff { -1 } else { v }, i)
}

/// `process_inf1`'s `#Q` arm (`geminit.c:384-388`) — the AES's own read of the
/// desktop background line, `#Q <desktop> <window>` three times, one pair per
/// plane-count bucket. Only the *desktop* byte of each pair is taken: EmuTOS
/// advances `scan_2(...) + 3`, skipping the space and the two window-colour
/// digits. Returns `Some(bg)` when a `#Q` line was present (`bgfound`).
///
/// This is deliberately a **second** reader of the same line — the desktop
/// parses `#Q` again for its own `G.g_patcol` (`desk/deskapp.c:953-960`).
fn process_inf1_q(buf: &[u8]) -> Option<[i16; 3]> {
    let mut found = None;
    let mut i = 0;
    // `for (pcurr = infbuf; *pcurr; )` with the `'#'` + tag lookahead
    // (`geminit.c:356-361`).
    while i + 1 < buf.len() && buf[i] != 0 {
        if buf[i] != b'#' {
            i += 1;
            continue;
        }
        i += 1;
        let tag = buf[i];
        i += 1;
        if tag != b'Q' {
            continue;
        }
        let mut bg = [0i16; 3];
        for slot in bg.iter_mut() {
            let (v, used) = scan_2(&buf[i.min(buf.len())..]);
            *slot = v;
            i = (i + used + 3).min(buf.len());
        }
        found = Some(bg);
    }
    found
}

/// `gem_main`'s `.INF` read (`geminit.c:766-773`) — fill [`INFBUF`] with the
/// first [`INF_SIZE`] bytes of the file and NUL-terminate it. A
/// missing/unreadable file is EmuTOS's `n < 0 → n = 0` case (`:771-772`): an
/// empty buffer, so neither `#Q` nor `#Z` is found. Runs at the head of
/// **every** AES session, cold boot and restart alike; [`process_inf2`] reads
/// the same bytes later in the same session.
fn read_infbuf() {
    // SAFETY: single-CPU boot/restart path; exclusive access to the static.
    let infbuf = unsafe { &mut *INFBUF.0.get() };
    infbuf.fill(0);
    let n = crate::bdos::fs::read_file(INF_PATH, &mut infbuf[..INF_SIZE]).unwrap_or(0);
    infbuf[n] = 0; // `infbuf[n] = '\0'` (geminit.c:773)
}

/// `process_inf1()` (`geminit.c:343-392`, called from `:776`), reduced to its
/// `#Q` arm: record the desktop background bytes in [`BG`] (`bg`/`bgfound`).
///
/// `gem_main` guards the call with `if (!gl_changerez)` (`:775`) — "can't be
/// here because of rez change, so see if .inf says we need to change rez" —
/// so on an A106 restart it does **not** run and [`BG`] keeps the value the
/// previous session parsed. The `#E` resolution arm (`:363-380`) has no
/// NewTOS counterpart: `check_moderez` reports no change on a board with one
/// fixed mode (ADR-017 rev 2, D-101).
fn process_inf1() {
    // SAFETY: single-CPU boot path; the buffer was filled by `read_infbuf`.
    let buf = unsafe { &*INFBUF.0.get() };
    // SAFETY: as above — exclusive access to the cell on this path.
    unsafe { *BG.0.get() = process_inf1_q(buf) };
}

/// The per-session bring-up of the AES's own built-in resource:
/// `gem_rsc_init` (`gem_rsc.c:624-630`), `gem_rsc_fixit` (`:636-641`,
/// `geminit.c:704`), the full-screen `ob_height` (`geminit.c:707-709`) and
/// `set_aes_background` (`:719-728` → `:635-641`).
///
/// It starts from [`DESKTOP_ROM`] because `rs_obfix` converts char cells to
/// pixels in place and would compound on a second pass — EmuTOS avoids that
/// with `gem_rsc_init`'s ROM→RAM `memcpy`, and so does this.
fn gem_rsc() {
    // SAFETY: single-CPU boot/restart path; the statics are initialised and
    // in scope for the whole program (see [`DesktopTree`]).
    unsafe {
        // `memcpy(rs_obj, rs_obj_rom, …)` + `bzero(rs_te_text, …)` and the
        // ROM `TEDINFO` copy (`gem_rsc.c:626-630`): `te_ptext` goes back to
        // the `0L` of `rs_tedinfo[12]`, dropping the program name a previous
        // session's `sh_draw` pointed it at (`gemshlib.c:284-285`).
        *DESKTOP.0.get() = DESKTOP_ROM;
        let ted = DTNAME_TED.0.get();
        (*ted).te_ptext = 0;
        (*ted).te_ptmplt = DTNAME_TEXT.as_ptr() as u64;
        (*ted).te_pvalid = DTNAME_TEXT.as_ptr() as u64;

        // `gem_rsc_fixit` (`gem_rsc.c:636-641`): the tree's coordinates are
        // packed char-grid words; convert all four of every object with
        // `rs_obfix` (`gemrslib.c:89-99`), the live `gl_wchar`/`gl_hchar`/
        // `gl_width` included.
        let tree = desktop_tree();
        let t = ObjTree::lp64(tree);
        for i in 0..3 {
            super::resource::rs_obfix(t, i);
        }
        (*tree.add(DTNAME)).ob_spec = ted as u64;

        // `tree[ROOT].ob_height = gl_rscreen.g_h` (`geminit.c:716-717`) — the
        // full screen height, overriding the resource's 25 character rows.
        if let Some(h) = crate::host::host().and_then(|h| h.raster()).map(|r| r.height() as i16) {
            (*tree.add(ROOT as usize)).ob_height = h;
        }
    }

    // `set_aes_background` (`geminit.c:719-728`), only if `#Q` was found.
    // `n = (gl_nplanes > 2) ? 2 : gl_nplanes - 1` picks the pair for this
    // screen depth; `gl_nplanes` is the VDI workstation's plane count.
    // SAFETY: single-CPU boot/restart path.
    if let Some(bg) = unsafe { *BG.0.get() } {
        let planes = crate::host::host()
            .and_then(|h| h.raster())
            .map(|r| r.planes() as i16)
            .unwrap_or(4);
        let idx = if planes > 2 { 2 } else { (planes - 1).max(0) } as usize;
        set_aes_background((bg[idx] & 0xff) as u8);
    }
}

/// AES boot initialisation — the cold-boot form of the session bring-up:
/// read the `.INF` file (`geminit.c:766-773`), parse its `#Q` line
/// (`process_inf1`, `:776`) and set the built-in `DESKTOP` resource up for
/// the open workstation ([`gem_rsc`]). Runs before any process can call
/// `appl_init`.
pub fn init() {
    read_infbuf();
    process_inf1();
    gem_rsc();
}

/// The same bring-up for the **second and later** AES sessions of a boot —
/// `gem_main`'s body on the restart path (A106, `gemstart.S:86-124` →
/// `geminit.c:766-776`): the `.INF` file is re-read, but `process_inf1` is
/// skipped because the restart *is* the resolution change it would look for
/// (`:775`), so the `#Q` background bytes of [`BG`] carry over unchanged.
///
/// Called only from [`super::reinit`].
pub(super) fn reinit() {
    read_infbuf();
    gem_rsc();
}

/// `process_inf2` (`geminit.c:407-464`) — the **second** `.INF` pass, run
/// after the AES's own init and before the shell loop (`geminit.c:734`,
/// inside `run_accs_and_desktop`). It reads two lines of the buffer
/// [`init`] filled:
///
/// * **`#E`** (`:421-436`) — `ev_dclick(env & 0x07, TRUE)` sets the
///   double-click speed from the low three bits of the first byte
///   ([`super::event::ev_dclick`]). EmuTOS then reads further bytes on the
///   same line for the blitter (`Blitmode`, `#if CONF_WITH_BLITTER`) and the
///   68030 cache (`set_cache`, `#if CONF_WITH_CACHE_CONTROL`). Both are
///   **m68k hardware** with no BCM2711 equivalent — the hardware-layer
///   carve-out of `docs/COMPATIBILITY.md`. The bytes are still consumed so
///   the scan cursor advances exactly as EmuTOS's does; only the two
///   hardware actions are absent.
/// * **`#Z`** (`:438-460`), format `#Z <isgem-byte> <path>@` — the autorun
///   entry. `scan_2` reads the flag byte (`00` = character mode, else GEM),
///   the path up to `@` is split by `sh_name` into a directory and a bare
///   program name, and the pair goes to [`super::shell::sh_wdef`] with
///   `isauto = TRUE`.
///
/// Returns `(isauto, isgem)`; `isgem` is EmuTOS's return value
/// `(*isauto && !isgem) ? FALSE : TRUE` (`:463`), i.e. character mode only
/// when an autorun entry exists *and* its flag byte is `00`. Both are passed
/// straight into `sh_main(isauto, isgem)` (`geminit.c:739`).
///
/// **Only the first [`INF_SIZE`] (300) bytes are visible**, because that is
/// all `gem_main` reads (`geminit.c:769`). EmuTOS states the resulting
/// constraint on the file itself: "the `#Z` and `#E` lines must be completely
/// contained within the first INF_SIZE (currently 300) bytes of EMUDESK.INF"
/// (`desk/deskapp.c:161-162`), which is why the desktop's writer emits `#Z`
/// between `#R` and `#E` (`deskapp.c:1128-1133`). A `#Z` line placed after the
/// `#W`/ANODE bulk is invisible to the AES.
///
/// **`BOOTFLAG_SKIP_AUTO_ACC` (`geminit.c:452`) is not reproduced**: NewTOS
/// has no boot-time modifier-key scan (EmuTOS sets the flag when Control is
/// held on the initinfo screen, `bios/bios.c:819-820`), so there is no such
/// flag to test. Nothing is faked — the guard is simply absent, and the
/// autorun entry is always honoured.
pub fn process_inf2() -> (bool, bool) {
    // SAFETY: single-CPU boot path; the static was filled by `init`.
    let buf = unsafe { &*INFBUF.0.get() };
    let mut isauto = false;
    let mut isgem: i16 = 1; // `WORD env, isgem = TRUE;` (:409)

    let mut i = 0usize;
    // `for (pcurr = infbuf; *pcurr; )` with the `'#'` + tag lookahead (:415-419)
    while i < buf.len() && buf[i] != 0 {
        if buf[i] != b'#' {
            i += 1;
            continue;
        }
        i += 1;
        if i >= buf.len() {
            break;
        }
        let tag = buf[i];
        if tag == b'E' {
            // `pcurr += 2` — skip the tag and the following blank (:423).
            i += 2;
            let (env, used) = scan_2(&buf[i.min(buf.len())..]);
            i = (i + used).min(buf.len());
            super::event::ev_dclick(env & 0x07, true); // (:425)
            // The blitter byte (:426) and, with CONF_WITH_CACHE_CONTROL, the
            // two video bytes plus the cache byte (:431-434). Consumed to
            // keep the cursor in step; no RPi4 equivalent to act on.
            for _ in 0..4 {
                let (_v, used) = scan_2(&buf[i.min(buf.len())..]);
                i = (i + used).min(buf.len());
            }
        } else if tag == b'Z' {
            // `pcurr += 2; scan_2(pcurr, &isgem); pcurr += 3;` (:441-444) —
            // note `scan_2`'s return is discarded here, the cursor advances
            // by a fixed three bytes (two hex digits + one blank).
            i += 2;
            let (v, _used) = scan_2(&buf[i.min(buf.len())..]);
            isgem = v;
            i = (i + 3).min(buf.len());
            // `while (*pcurr && *pcurr != '@') ++pcurr; *pcurr = 0;` (:445-447)
            let start = i;
            while i < buf.len() && buf[i] != 0 && buf[i] != b'@' {
                i += 1;
            }
            let path = &buf[start..i];
            // `tmpptr2 = sh_name(tmpptr1); *(tmpptr2-1) = 0;` (:448-449) —
            // split the one string into (directory, program) by writing a NUL
            // over the byte before the filename. When the filename starts at
            // offset 0 there is no such byte inside the path, so the
            // directory is empty.
            let off = path.len() - super::shell::sh_name(path).len();
            let dir: &[u8] = if off > 0 { &path[..off - 1] } else { &[] };
            let prog = &path[off..];
            super::shell::sh_wdef(prog, dir); // (:455)
            isauto = true; // (:456)
            i += 1; // `++pcurr;` (:458)
        }
    }

    // `return (*isauto && !isgem) ? FALSE : TRUE;` (:463)
    (isauto, !(isauto && isgem == 0))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Where `appl_init`'s `ap_private` comes from, end to end bar the file
    /// read: the built-in `DESKTOP` root's `ob_spec` with the `.INF` `#Q`
    /// background byte ORed into its low byte
    /// (`gemsuper.c:95-96` ← `geminit.c:719-728` ← `:635-641` ← `:384-388`).
    ///
    /// This is what the two round-2 reference runs disagree about, and the
    /// disagreement is **data, not code**: the reference TT reported
    /// `ap_private = 4419` (`0x1143`) because EmuTOS's shipped
    /// `#Q 41 40 43 40 43 40` (`desk/deskapp.c:175`) puts `0x43` — the same
    /// byte the resource constant already carries — into the >2-plane bucket;
    /// the Pi's `NEWDESK.INF` had `0x7B` there (pattern 7, colour 11, a value
    /// the desktop's Set-Background dialog writes), giving `0x117B` = 4475.
    #[test]
    fn ap_private_is_the_desktop_root_obspec_with_the_inf_background_byte() {
        // The compiled-in resource constant, before any `#Q` is applied.
        assert_eq!(DESKTOP_ROOT_OBSPEC, 0x1143);

        // No `#Q` line at all → `bgfound` FALSE → `set_aes_background` is
        // never called (`geminit.c:724`).
        assert_eq!(process_inf1_q(b"#E 3A 11 FF 02\r\n#R 00\r\n\0"), None);
        assert_eq!(process_inf1_q(b"newdesk placeholder\r\n\0"), None);

        // EmuTOS's own default line: three (desktop, window) pairs, only the
        // desktop byte of each taken (`geminit.c:384-388`).
        assert_eq!(
            process_inf1_q(b"#Q 41 40 43 40 43 40\r\n\0"),
            Some([0x41, 0x43, 0x43])
        );
        // A desktop that saved a changed background writes the same line with
        // the >2-plane pair replaced.
        assert_eq!(
            process_inf1_q(b"#E 3A 11\r\n#Q 41 40 43 40 7B 40\r\n#W 00 00\r\n\0"),
            Some([0x41, 0x43, 0x7B])
        );

        // `n = (gl_nplanes > 2) ? 2 : gl_nplanes-1` picks bg[2] on both a
        // 4-plane TT and a 16-plane Pi, so the two runs differ only in the
        // byte their `.INF` file carried.
        let saved = desktop_root_obspec();
        set_aes_background(0x43);
        assert_eq!(desktop_root_obspec(), 4419);
        set_aes_background(0x7B);
        assert_eq!(desktop_root_obspec(), 4475);
        // Only the low byte is ever replaced (`geminit.c:639-640`).
        set_aes_background(0x00);
        assert_eq!(desktop_root_obspec() & 0xffff_ff00, 0x1100);
        set_aes_background((saved & 0xff) as u8);
    }
}
