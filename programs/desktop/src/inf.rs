//! `desktop::inf` — `NEWDESK.INF` read/save and the `deskinf.c` Show-Info /
//! Set-background dialogs.
//!
//! Reimplements two EmuTOS pieces that share the `NEWDESK.INF`/`EMUDESK.INF`
//! feature area, per [`docs/api/desktop/deskinf.md`] (the file-format appendix
//! and the Show-Info dialogs) and `deskapp.md` (`app_start`'s INF-position
//! apply loop, `:979-987`):
//!
//! * the **file format** — `desk/deskapp.c`'s `app_parse`/`app_start` (load) and
//!   `app_save` (save): the `#R`/`#Z`/`#E`/`#Q`/`#W` inline lines plus the
//!   `#M`/`#T`/`#O`/`#D`/`#G`/`#Y`/`#F`/`#P`/`#I`/`#N`/`#X`/`#V` `ANODE` lines,
//!   with the grid-unit ↔ pixel lifecycle for desktop-icon spots;
//! * the **dialogs** — `deskinf.c`'s `inf_file_folder` (Show Info + rename) and
//!   `inf_backgrounds` (Set Background).
//!
//! ## What is applied at boot
//!
//! Per the lead decision (EmuTOS-faithful): INF desktop-icon positions are
//! **grid-rounded via [`DeskState::snap_icon_grid`] with no collision check**
//! (`app_start` does exactly this — colliding INF positions overlap silently,
//! `deskins.md`). Collision avoidance is the install path's job (#20), not here.
//! View/sort ([`View`]/[`Sort`]), the background (`g_patcol`), the desktop-icon
//! grid positions, and the saved window rectangles are applied; a missing file
//! is not an error (the desktop keeps its built-in defaults).

use alloc::string::String;
use alloc::vec::Vec;
use core::cell::RefCell;

use tos_types::aes::{
    Grect, Object, TedInfo, ADJ3DSTD, DEFAULT, DESKWH, DISABLED, EDITABLE, EXIT, FL3DACT, FL3DBAK,
    FL3DIND, G_BOX, G_BUTTON, G_FBOXTEXT, G_FTEXT, G_IBOX, G_STRING, HIDETREE, LASTOB, MU_MESAG, MU_TIMER,
    NIL, OUTLINED, RBUTTON, ROOT, SELECTABLE, SELECTED, TE_LEFT, TE_RIGHT, TOUCHEXIT,
};
use tos_types::error::EFILNF;
use tos_types::Long;

use newtos_gem::gemdos::{dos_close, dos_create, dos_gdrv, dos_open, dos_read, dos_sdrv, dos_write};
use super::global::{
    self, gl_hchar, gl_wchar, Anode, DeskStatic, WSave, AF_ISEXEC, BORDER_TEXT_COLOURS, DROOT, NUM_WNODES,
    WOBS_START,
};
use super::icons;
use super::model::{AType, DateFmt, DateOrder, Sort, TimeFmt, View};

// ── GEMDOS file access (through the trap, [`newtos_gem::gemdos`]) ────────

/// The desktop configuration file, on the boot drive's root (EmuTOS keeps its
/// own `EMUDESK.INF` here; NewTOS uses the Atari-classic `NEWDESK.INF` name the
/// user's SD card carries).
const INF_PATH: &str = "C:\\NEWDESK.INF";

/// Upper bound on the file we will read (a full 64-`ANODE` file with strings is
/// a few KiB; this guards a runaway read).
const MAX_INF_BYTES: usize = 64 * 1024;

/// `SIZE_SHELBUF` (`include/sysconf.h:74`, *"size of shell buffer — same as TOS
/// 1.04→"*) — the capacity of the AES shell buffer [`super::shel_get`] /
/// [`super::shel_put`] transfer. Matches `tos_api::aes::shell`'s own
/// `SHELLBUF`, which is the buffer this number describes.
const SIZE_SHELBUF: usize = 4192;

/// `CPDATA_LEN` (`desk/deskapp.c:136`) — *"the number of bytes reserved for
/// control panel use at the start of the shell buffer. these bytes are not
/// modified by EmuDesk, and are (currently) not saved to the EMUDESK.INF
/// file."* (`:131-135`).
///
/// This is a **desktop-side** convention layered on an opaque AES buffer, not
/// an AES contract — the server has no knowledge of it
/// (`docs/api/aes/shell-loop.md` §10). The `.INF` text therefore starts at
/// offset 128 in the shell buffer, `app_save` round-trips the first 128 bytes
/// unmodified (`:1123`, `:1293`), and the disk file never contains them
/// (`:1297`).
const CPDATA_LEN: usize = 128;

/// `SIZE_SHELBUF-CPDATA_LEN-1` (`deskapp.c:688`) — the most `read_inf_file`
/// will load, because it loads **into the shell buffer**, behind the
/// control-panel prefix and leaving room for the terminating NUL.
const MAX_INF_TEXT: usize = SIZE_SHELBUF - CPDATA_LEN - 1;

/// `read_inf_file` (`deskapp.c:680-692`) — load `NEWDESK.INF` into the `.INF`
/// half of the shell buffer, or [`None`] if it cannot be opened (a missing file
/// is **not** an error — the caller falls through to `build_inf`).
///
/// EmuTOS reads at most [`MAX_INF_TEXT`] bytes (`dos_load_file(…,
/// SIZE_SHELBUF-CPDATA_LEN-1, infbuf)`) and NUL-terminates what it got; the
/// same bound holds here now that the buffer it reads into exists. A longer
/// file is truncated exactly where EmuTOS truncates it — and `app_save`'s own
/// overflow check (`:1276-1282`) keeps everything *we* write below it.
fn read_inf_file() -> Option<Vec<u8>> {
    let mut bytes = read_file(INF_PATH).ok()?;
    bytes.truncate(MAX_INF_TEXT);
    Some(bytes)
}

/// `dos_load_file` (`util/gemdos.c:148-162`) for an arbitrary path: `Fopen`,
/// read to EOF, `Fclose`. On a failed open the GEMDOS error is handed back
/// unchanged, because that is the one thing [`app_read_inf`] has to tell apart
/// — `EFILNF` is its "file not found" alert, every other failure its "illegal
/// .INF file" one (`deskapp.c:1536-1542`).
fn read_file(path: &str) -> Result<Vec<u8>, Long> {
    let rc = dos_open(path, 0);
    if rc < 0 {
        return Err(rc);
    }
    let handle = rc as i16;
    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 512];
    loop {
        let n = dos_read(handle, chunk.len() as i32, chunk.as_mut_ptr());
        if n <= 0 {
            break;
        }
        let got = n as usize;
        buf.extend_from_slice(&chunk[..got.min(chunk.len())]);
        if got < chunk.len() || buf.len() >= MAX_INF_BYTES {
            break;
        }
    }
    dos_close(handle);
    Ok(buf)
}

/// Create/truncate `NEWDESK.INF` and write `bytes`. Returns whether the full
/// buffer was written.
fn write_inf_file(bytes: &[u8]) -> bool {
    let rc = dos_create(INF_PATH, 0);
    if rc < 0 {
        return false;
    }
    let handle = rc as i16;
    let mut off = 0usize;
    let mut ok = true;
    while off < bytes.len() {
        let n = dos_write(handle, (bytes.len() - off) as i32, bytes[off..].as_ptr());
        if n <= 0 {
            ok = false;
            break;
        }
        off += n as usize;
    }
    dos_close(handle);
    ok
}

// ── `#E` environment flag bits (`deskinf.md` appendix, `deskapp.c`) ──────────

const INF_E1_VIEWTEXT: u8 = 0x80;
const INF_E1_SORTMASK: u8 = 0x60;
const INF_E1_SORTSHIFT: u8 = 5;
/// `INF_E1_CONFDEL` (`deskapp.c:89`) — 1 ⇒ confirm deletes.
const INF_E1_CONFDEL: u8 = 0x10;
/// `INF_E1_CONFCPY` (`deskapp.c:90`) — 1 ⇒ confirm copies.
const INF_E1_CONFCPY: u8 = 0x08;
/// `INF_E2_ALLOWOVW` (`deskapp.c:98`) — 1 ⇒ **allow** overwrites, i.e. do
/// **not** confirm them. "yes, it's backwards", says the EmuTOS comment; the
/// decode is `cs_confovwr = ((envr & INF_E2_ALLOWOVW) == 0)` (`:925`).
const INF_E2_ALLOWOVW: u8 = 0x10;
/// `INF_E1_DCMASK` (`deskapp.c:91`) — double-click speed index, 0..4 valid.
const INF_E1_DCMASK: u8 = 0x07;
/// `INF_E2_IDTDATE` (`deskapp.c:96`) — 1 ⇒ take the date format from the `_IDT`
/// cookie and ignore [`INF_E2_DAYMONTH`].
const INF_E2_IDTDATE: u8 = 0x40;
/// `INF_E2_IDTTIME` (`deskapp.c:97`) — 1 ⇒ take the time format from `_IDT` and
/// ignore [`INF_E2_24HCLOCK`].
const INF_E2_IDTTIME: u8 = 0x20;
/// `INF_E2_DAYMONTH` (`deskapp.c:100`) — 1 ⇒ day before month.
const INF_E2_DAYMONTH: u8 = 0x04;
/// `INF_E2_24HCLOCK` (`deskapp.c:101`) — 1 ⇒ 24-hour clock.
const INF_E2_24HCLOCK: u8 = 0x02;
const INF_E5_NOSORT: u8 = 0x80;
/// `INF_E5_APPDIR` (`deskapp.c:110`) — 1 ⇒ launch a **non-installed** program
/// with the current directory set to the program's own directory, 0 ⇒ to the
/// top window's. The *Default dir* group of `inf_conf` edits it and
/// `do_aopen` reads it (`desksupp.c:901`).
const INF_E5_APPDIR: u8 = 0x40;
/// `INF_E5_ISFULL` (`deskapp.c:111`) — 1 ⇒ pass the **full path** of a file
/// dropped on a program icon as its argument, 0 ⇒ only the file name. The
/// *Parameter* group of `inf_conf` edits it and `build_selected_path` reads it
/// (`deskfun.c:916`).
const INF_E5_ISFULL: u8 = 0x20;
const INF_E5_NOSIZE: u8 = 0x10;

/// `INF_E1_DEFAULT` = `CONFDEL|CONFCPY|dclick(2)` (`deskapp.c` `desk_inf_data1`).
const INF_E1_DEFAULT: u8 = 0x1A;
/// `INF_E2_DEFAULT` = `BLITTER|IDTDATE|IDTTIME`.
const INF_E2_DEFAULT: u8 = 0xE0;
/// `INF_E5_DEFAULT` = `APPDIR|ISFULL` (size-to-fit enabled, sorting on).
const INF_E5_DEFAULT: u8 = 0x60;

/// Number of `#K` menu-shortcut bytes (`NUM_SHORTCUTS`, `deskmain.h:18`).
/// The array itself lives in [`super::menu`], which owns the menu strings the
/// letters are rendered into; this module only moves it in and out of the file
/// (`deskapp.c:964-975` / `:1184-1192`).
use super::menu::NUM_SHORTCUTS;

// ── Installed-application extra-block bits + icon indices ────────────────────
/// `INF_AT_APPDIR` (`deskapp.c:126`) — 1 ⇒ set the current directory to the
/// application's directory (else to the top window's).
const INF_AT_APPDIR: u8 = 0x01;
/// `INF_AT_ISFULL` (`deskapp.c:127`) — 1 ⇒ pass the document's full path in the
/// command tail (else just the file name).
const INF_AT_ISFULL: u8 = 0x02;

use super::icons::{IG_APPL, IG_DOCU};

// ── The serializable INF model ──────────────────────────────────────────────

/// One `ANODE` line (`#M`/`#T`/`#O`/`#D`/`#G`/`#Y`/`#F`/`#P`/`#I`/`#N`/`#X`/
/// `#V`). Desktop icons (`is_desk`) carry a grid spot + drive letter; file
/// types (`has_extra`) carry a function-key/flags/args block.
#[derive(Clone, PartialEq, Eq, Default)]
struct InfAnode {
    /// One-character tag byte (`b'M'`, `b'T'`, …).
    tag: u8,
    /// `AF_ISDESK` implied by the tag (has `xx`/`yy` + drive letter).
    is_desk: bool,
    /// `AT_ISFILE && !AF_ISDESK` implied by the tag (has the extra block).
    has_extra: bool,
    /// Desktop grid column (`a_xspot`, grid units on disk).
    xspot: u8,
    /// Desktop grid row (`a_yspot`, grid units on disk).
    yspot: u8,
    /// `a_aicon` index (`FF` → `-1`).
    aicon: i16,
    /// `a_dicon` index (`FF` → `-1`).
    dicon: i16,
    /// Drive letter (`0` = none).
    letter: u8,
    /// `a_pappl` — application path / wildcard / label.
    pappl: String,
    /// `a_pdata` — data path / wildcard.
    pdata: String,
    /// Extra-block flags nibble (`INF_AT_APPDIR|INF_AT_ISFULL`, 0-3).
    flags: u8,
    /// Extra-block function-key number.
    funkey: u8,
    /// `a_pargs` — command-line arguments.
    pargs: String,
}

/// One `#W` saved-window-geometry line (character-cell units on disk).
#[derive(Clone, PartialEq, Eq, Default)]
struct InfWindow {
    /// Horizontal slider position (opaque slider units).
    hsl: u8,
    /// Vertical slider position.
    vsl: u8,
    /// X position in character cells.
    xc: u8,
    /// Y position in character cells.
    yc: u8,
    /// Width in character cells.
    wc: u8,
    /// Height in character cells.
    hc: u8,
    /// Saved directory path (`@`-terminated on disk; empty for a default slot).
    path: String,
}

/// The whole parsed / serialisable `NEWDESK.INF` state.
#[derive(Clone, PartialEq, Eq)]
struct InfData {
    /// `#E` env byte 1 (view/sort/confirm/dclick).
    e1: u8,
    /// `#E` env byte 2 (blitter/idt/overwrite/date-time format).
    e2: u8,
    /// `#E` env byte 5 (nosort/appdir/isfull/nosize/nocache).
    e5: u8,
    /// `#Q` background pattern/colour: three `(desktop, window)` pairs, one per
    /// plane-count bucket.
    patcol: [(u8, u8); 3],
    /// `#K` menu-shortcut letters, one per [`NUM_SHORTCUTS`] slot (`0` = none).
    /// EmuTOS lets **every** build read the line and only
    /// `CONF_WITH_DESKTOP_CONFIG` builds write it (`deskapp.c:961-965`,
    /// `:1183-1193`); ours has the dialog, so it does both.
    shortcuts: [u8; NUM_SHORTCUTS],
    /// `#Z` autorun program path (empty = none).
    autorun: String,
    /// `#Z` flag byte — EmuTOS writes `pa->a_flags & AF_ISCRYS`
    /// (`deskapp.c:1133`), i.e. `01` for a GEM application and `00` for a
    /// character-mode one. The AES reads it back as `isgem` in `process_inf2`
    /// (`geminit.c:443`) and it decides whether the autorun program starts in
    /// graphics or alpha mode, so it must survive a load/save round trip.
    autorun_gem: u8,
    /// `#W` saved window geometries, in file order.
    windows: Vec<InfWindow>,
    /// Every `ANODE` line, in file order.
    anodes: Vec<InfAnode>,
}

impl Default for InfData {
    fn default() -> Self {
        Self {
            e1: INF_E1_DEFAULT,
            e2: INF_E2_DEFAULT,
            e5: INF_E5_DEFAULT,
            patcol: [(0x41, 0x40), (0x43, 0x40), (0x43, 0x40)],
            // `memcpy(menu_shortcuts, default_shortcuts, NUM_SHORTCUTS)`
            // before `app_start` (`deskmain.c:2013`): a file with no `#K` line
            // leaves the built-in letters in place.
            shortcuts: super::menu::DEFAULT_SHORTCUTS,
            autorun: String::new(),
            autorun_gem: 0,
            windows: Vec::new(),
            anodes: Vec::new(),
        }
    }
}

/// The session INF: the last-parsed file, kept so a Save preserves the
/// file-type `ANODE`s and confirm/format preferences NewTOS does not yet edit
/// (mirroring EmuTOS keeping the whole `G.g_ahead` list live for the session).
static SESSION: DeskStatic<RefCell<Option<InfData>>> = DeskStatic::new(RefCell::new(None));

// ── Field-scan primitives (`util/optimize.c` `scan_2`, `deskapp.c` `scan_str`)

/// A forward byte cursor over the INF buffer.
struct Cursor<'a> {
    b: &'a [u8],
    i: usize,
}

impl Cursor<'_> {
    /// The byte at the cursor (`0` past the end).
    fn peek(&self) -> u8 {
        self.b.get(self.i).copied().unwrap_or(0)
    }

    /// Consume and return the byte at the cursor.
    fn bump(&mut self) -> u8 {
        let c = self.peek();
        if self.i < self.b.len() {
            self.i += 1;
        }
        c
    }

    /// Skip a run of literal spaces (`scan_2`/`scan_str` both do this first).
    fn skip_spaces(&mut self) {
        while self.peek() == b' ' {
            self.i += 1;
        }
    }

    /// Whether the cursor sits at a line end (`\r`/`\n`/NUL) — the extra-block
    /// present-or-absent test.
    fn at_line_end(&self) -> bool {
        matches!(self.peek(), b'\r' | b'\n' | 0)
    }
}

/// One hex digit's value (`0` for a non-hex byte).
fn hexval(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'A'..=b'F' => c - b'A' + 10,
        b'a'..=b'f' => c - b'a' + 10,
        _ => 0,
    }
}

/// `scan_2` — a 2-hex-digit field: skip leading spaces; a `\r` at the field
/// start yields `0` with nothing consumed; otherwise read exactly two hex
/// digits (`util/optimize.c:202-219`).
fn scan_2(c: &mut Cursor) -> u8 {
    c.skip_spaces();
    if c.peek() == b'\r' {
        return 0;
    }
    let hi = hexval(c.bump());
    let lo = hexval(c.bump());
    (hi << 4) | lo
}

/// A `scan_2` icon field: `0xFF` → `-1` (the "no icon override" sentinel,
/// `app_parse:359-364`).
fn scan_icon(c: &mut Cursor) -> i16 {
    let v = scan_2(c);
    if v == 0xFF {
        -1
    } else {
        v as i16
    }
}

/// `app_parse`'s out-of-range guard (`deskapp.c:359-365`): an icon number that
/// names no built-in picture becomes `dflt` — `IG_APPL` for `a_aicon`,
/// `IG_DOCU` for `a_dicon`. Values below the bound (including `-1` = "none")
/// pass through.
fn clamp_icon(v: i16, dflt: i16) -> i16 {
    if v >= icons::NUM_GEM_IBLKS {
        dflt
    } else {
        v
    }
}

/// `scan_str` — an `@`-terminated string: skip leading spaces, copy until `@`
/// (consumed) or `\r`/`\n`/NUL (left in place, so the caller can detect the
/// line end), `deskapp.c:258-289`.
fn scan_str(c: &mut Cursor) -> String {
    c.skip_spaces();
    let mut s = String::new();
    loop {
        match c.peek() {
            b'@' => {
                c.i += 1;
                break;
            }
            b'\r' | b'\n' | 0 => break,
            ch => {
                s.push(ch as char);
                c.i += 1;
            }
        }
    }
    s
}

// ── Parser (`app_start`'s line loop + `app_parse`) ──────────────────────────

/// Parse a whole `NEWDESK.INF` buffer (`app_start`, `deskapp.c:851-976`): scan
/// forward for each `#`, read the one-character tag, dispatch. Bytes between a
/// field's end and the next `#` are skipped, matching EmuTOS's `if (*pcurr++ !=
/// '#') continue;`.
fn parse_inf(bytes: &[u8]) -> InfData {
    let mut d = InfData::default();
    // `inf_rev_level` (`deskapp.c:129,855`) — the revision of the file being
    // read. `#R` is the first line every writer emits, so a shortcut line
    // always sees the right value; a file with no `#R` at all is EmuTOS's
    // `inf_rev_level = 0` initial value (`deskapp.c:150`).
    let mut rev: u8 = 0;
    let mut c = Cursor { b: bytes, i: 0 };
    while c.i < c.b.len() {
        if c.bump() != b'#' {
            continue;
        }
        let tag = c.bump();
        match tag {
            b'R' => {
                // `inf_rev_level` (`deskapp.c:855`). Written back as `02`
                // unconditionally ([`InfData::serialize`]), read here only for
                // the one thing EmuTOS still uses it for: the `a_pappl`/
                // `a_pdata` swap on a pre-rev-2 desktop-shortcut line
                // (`:404-415`).
                rev = scan_2(&mut c);
            }
            b'Z' => {
                c.skip_spaces();
                // `AF_ISCRYS` (`deskapp.h:28`) — kept so `serialize` writes the
                // same byte back; the AES reads it as `isgem` (`geminit.c:443`).
                d.autorun_gem = scan_2(&mut c);
                d.autorun = scan_str(&mut c);
            }
            b'E' => {
                d.e1 = scan_2(&mut c);
                d.e2 = scan_2(&mut c);
                let _vh = scan_2(&mut c); // video mode high — read but discarded
                let _vl = scan_2(&mut c); // video mode low  — read but discarded
                d.e5 = scan_2(&mut c);
            }
            b'Q' => {
                for pair in d.patcol.iter_mut() {
                    pair.0 = scan_2(&mut c);
                    pair.1 = scan_2(&mut c);
                }
            }
            // `case 'K'` (`deskapp.c:961-975`) — all 33 slots, overwriting the
            // defaults. EmuTOS notes that *reading* the line is unconditional
            // even in a build that cannot save it (`:962-965`).
            b'K' => {
                for slot in d.shortcuts.iter_mut() {
                    *slot = scan_2(&mut c);
                }
            }
            b'W' => {
                d.windows.push(parse_window(&mut c));
            }
            b'T' | b'M' | b'O' | b'X' | b'V' | b'D' | b'G' | b'Y' | b'F' | b'P' | b'I' | b'N' => {
                d.anodes.push(parse_anode(&mut c, tag, rev));
            }
            _ => {}
        }
    }
    d
}

/// Parse one `#W` line: `<hsl> <vsl> <xc> <yc> <wc> <hc> <00> <path>@`
/// (`app_start:889-913`). The 7th field is the unused placeholder.
fn parse_window(c: &mut Cursor) -> InfWindow {
    let hsl = scan_2(c);
    let vsl = scan_2(c);
    let xc = scan_2(c);
    let yc = scan_2(c);
    let wc = scan_2(c);
    let hc = scan_2(c);
    let _unused = scan_2(c);
    let path = scan_str(c);
    InfWindow { hsl, vsl, xc, yc, wc, hc, path }
}

/// Parse one `ANODE` line after its tag (`app_parse`, `deskapp.c:295-436`).
/// Desktop icons (`T`/`M`/`O`/`X`/`V`) carry `xx yy … L`; file types
/// (`G`/`Y`/`F`/`P`/`I`/`N`) carry the trailing `f kk pargs@` block when present.
fn parse_anode(c: &mut Cursor, tag: u8, rev: u8) -> InfAnode {
    let is_desk = matches!(tag, b'T' | b'M' | b'O' | b'X' | b'V');
    let has_extra = matches!(tag, b'G' | b'Y' | b'F' | b'P' | b'I' | b'N');

    let (xspot, yspot) = if is_desk { (scan_2(c), scan_2(c)) } else { (0, 0) };
    // `if (pa->a_aicon >= G.g_numiblks) pa->a_aicon = IG_APPL;` and the same
    // for `a_dicon` with `IG_DOCU` (`deskapp.c:359-365`). `scan_2` has already
    // turned `FF` into `-1`, which is *below* the bound and stays: `-1` is
    // "this node has no icon of that kind", not an out-of-range index.
    let aicon = clamp_icon(scan_icon(c), IG_APPL);
    let dicon = clamp_icon(scan_icon(c), IG_DOCU);

    // The `<L>` field is a single literal character (space = no letter): consume
    // exactly one separator space, then one char — do NOT skip a run of spaces,
    // or a blank `L` would swallow the first char of `pappl`.
    let letter = if is_desk {
        if c.peek() == b' ' {
            c.i += 1;
        }
        let l = c.bump();
        if l == b' ' || l == b'\r' || l == b'\n' || l == 0 {
            0
        } else {
            l
        }
    } else {
        0
    };

    let mut pappl = scan_str(c);
    let mut pdata = scan_str(c);
    // "fix up old format of desktop icon entries if necessary"
    // (`deskapp.c:404-415`): before revision 2 a desktop shortcut carried the
    // file *name* in `a_pappl` and the full path in `a_pdata`; from revision 2
    // on it is the other way round. Only `#X`/`#V` are affected — no other tag
    // is both `AF_ISDESK` and `AT_ISFILE`/`AT_ISFOLD`.
    if rev < 2 && matches!(tag, b'X' | b'V') {
        core::mem::swap(&mut pappl, &mut pdata);
    }

    // The extra `f kk pargs@` block is present only when a non-space, non-EOL
    // byte follows `pdata` (the `desk_inf_data2` type defaults omit it).
    let (flags, funkey, pargs) = if has_extra {
        c.skip_spaces();
        if c.at_line_end() {
            (0, 0, String::new())
        } else {
            let flags = hexval(c.bump());
            let funkey = scan_2(c);
            let pargs = scan_str(c);
            (flags, funkey, pargs)
        }
    } else {
        (0, 0, String::new())
    };

    InfAnode { tag, is_desk, has_extra, xspot, yspot, aicon, dicon, letter, pappl, pdata, flags, funkey, pargs }
}

// ── Serialiser (`app_save`, `deskapp.c:1106-1300`) ──────────────────────────

/// Append two upper-case hex digits of `v` to `out`.
fn push_hex2(out: &mut Vec<u8>, v: u8) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    out.push(HEX[(v >> 4) as usize]);
    out.push(HEX[(v & 0x0F) as usize]);
}

/// Append the hex byte for an icon index (`-1` → `FF`).
fn push_icon(out: &mut Vec<u8>, v: i16) {
    push_hex2(out, if v < 0 { 0xFF } else { (v & 0xFF) as u8 });
}

impl InfData {
    /// Serialise to the `NEWDESK.INF` byte grammar in `app_save`'s line order:
    /// `#R`, `#Z`, `#E`, `#Q`, all `#W`, then every `ANODE` line.
    ///
    /// Appends to `out`, which is EmuTOS's `outbuf` — the buffer that already
    /// holds the [`CPDATA_LEN`]-byte control-panel prefix when [`app_save`]
    /// calls this (`deskapp.c:1121-1124`). `out.len()` is therefore
    /// `pcurr - outbuf`, which is the origin the `ANODE` loop's overflow check
    /// measures from (`:1276`).
    fn serialize_into(&self, out: &mut Vec<u8>) {
        // #R — always written at the current revision level (2).
        out.extend_from_slice(b"#R 02\r\n");

        // #Z — only when an autorun program is set. The flag byte is
        // `pa->a_flags & AF_ISCRYS` (`deskapp.c:1133`), round-tripped from the
        // file so a Save does not silently downgrade a GEM autorun program to
        // character mode.
        if !self.autorun.is_empty() {
            out.extend_from_slice(b"#Z ");
            push_hex2(out, self.autorun_gem);
            out.push(b' ');
            out.extend_from_slice(self.autorun.as_bytes());
            out.extend_from_slice(b"@\r\n");
        }

        // #E — the video-mode bytes are informational only; written as 00 00.
        out.extend_from_slice(b"#E ");
        push_hex2(out, self.e1);
        out.push(b' ');
        push_hex2(out, self.e2);
        out.extend_from_slice(b" 00 00 ");
        push_hex2(out, self.e5);
        out.extend_from_slice(b"\r\n");

        // #Q — three (desktop, window) pattern/colour pairs.
        out.extend_from_slice(b"#Q ");
        for (i, pair) in self.patcol.iter().enumerate() {
            if i > 0 {
                out.push(b' ');
            }
            push_hex2(out, pair.0);
            out.push(b' ');
            push_hex2(out, pair.1);
        }
        out.extend_from_slice(b"\r\n");

        // #K — the menu-shortcut letters, one two-digit byte per slot, written
        // between `#Q` and the `#W` lines exactly as `app_save` orders them
        // (`deskapp.c:1176-1193`). The whole array is always written, including
        // the `00` slots, because the reader takes it positionally.
        out.extend_from_slice(b"#K");
        for k in self.shortcuts {
            out.push(b' ');
            push_hex2(out, k);
        }
        out.extend_from_slice(b"\r\n");

        // #W — every saved window geometry.
        for w in &self.windows {
            out.extend_from_slice(b"#W ");
            for v in [w.hsl, w.vsl, w.xc, w.yc, w.wc, w.hc] {
                push_hex2(out, v);
                out.push(b' ');
            }
            out.extend_from_slice(b"00 ");
            out.extend_from_slice(w.path.as_bytes());
            out.extend_from_slice(b"@\r\n");
        }

        // ANODE lines.
        for a in &self.anodes {
            let line_start = out.len();
            out.push(b'#');
            out.push(a.tag);
            out.push(b' ');
            if a.is_desk {
                push_hex2(out, a.xspot);
                out.push(b' ');
                push_hex2(out, a.yspot);
                out.push(b' ');
            }
            push_icon(out, a.aicon);
            out.push(b' ');
            push_icon(out, a.dicon);
            out.push(b' ');
            if a.is_desk {
                out.push(if a.letter == 0 { b' ' } else { a.letter });
                out.push(b' ');
            }
            out.extend_from_slice(a.pappl.as_bytes());
            out.extend_from_slice(b"@ ");
            out.extend_from_slice(a.pdata.as_bytes());
            out.push(b'@');
            // Extra `<f><kk> <pargs>@` block only when it carries data (matches
            // the compact `desk_inf_data2` defaults, which omit it entirely):
            // one flags nibble, then the 2-digit funkey, then the args string.
            if a.has_extra && (a.flags != 0 || a.funkey != 0 || !a.pargs.is_empty()) {
                const HEX: &[u8; 16] = b"0123456789ABCDEF";
                out.push(b' ');
                out.push(HEX[(a.flags & 0x0F) as usize]);
                push_hex2(out, a.funkey);
                out.push(b' ');
                out.extend_from_slice(a.pargs.as_bytes());
                out.push(b'@');
            }
            out.extend_from_slice(b"\r\n");

            // `if (pcurr-outbuf >= SIZE_SHELBUF) { … break; }`
            // (`deskapp.c:1276-1282`) — the shell buffer is the hard limit on
            // what a save can carry, and the `ANODE` list is the only
            // unbounded part of the file. EmuTOS drops the line that crossed
            // the boundary **whole** (it rewinds past the `\r\n` it just wrote
            // to the end of the previous line) and stops; the disk copy is
            // truncated with it, because `save_to_disk` gets the same buffer.
            if out.len() >= SIZE_SHELBUF {
                out.truncate(line_start);
                break;
            }
        }
    }

    /// The built-in file-type default `ANODE`s (`desk_inf_data2`,
    /// `deskapp.c:181-189`) — generic per-extension entries. Used when no file
    /// has been read, so a fresh Save still writes a complete, valid file.
    ///
    /// **Order: EmuTOS's live list, not its file.** `desk_inf_data2` lists the
    /// three `*.*` catch-alls *first* and the per-extension program entries
    /// after them, but `app_alloc` links every parsed `ANODE` at the **head**
    /// (`deskapp.c:204-219`), so by the time `app_afind_by_name` walks
    /// `G.g_ahead` the order is reversed: `*.TOS` … `*.GTP` first, the three
    /// `*.*` entries last. `app_save` restores the file order with a pair of
    /// `app_revit` calls around the write (`deskapp.c:1213`, `:1285`). NewTOS
    /// has no head-linked list to reverse — [`InfData::anodes`] *is* the live
    /// list and is serialised in that order — so the table below is written in
    /// the **search** order, which is what makes a `FOO.PRG` resolve to
    /// `#G 06 FF *.PRG@ @` (application icon) instead of being swallowed by
    /// `#F FF 07 @ *.*@` (document icon). The on-disk line order of the eight
    /// generic lines is therefore the reverse of EmuTOS's; every other property
    /// of the file, and the resolution result, is identical.
    ///
    /// **The three `*.N?P` entries are NewTOS's (ADR-018).** `desk_inf_data2`
    /// is exactly where EmuTOS declares which extensions name a *program*, and
    /// NewTOS's native AArch64 family is `.NAP`/`.NCP`/`.NTP`
    /// ([`super::ftype`]) rather than `.APP`/`.PRG`/`.TOS`/`.TTP`/`.GTP`. The
    /// classic five stay in the table unchanged — they are still the m68k
    /// family's names — and the native three are added with the same
    /// `a_aicon = IG_APPL` the classic ones carry, so a native program gets the
    /// application icon through the `ANODE` list rather than only through
    /// `win_bldview`'s extension fall-back.
    fn default_type_anodes() -> Vec<InfAnode> {
        // (tag, aicon, dicon, pappl, pdata)
        let table: [(u8, i16, i16, &str, &str); 11] = [
            (b'G', 0x06, -1, "*.NAP", ""),
            (b'F', 0x06, -1, "*.NCP", ""),
            (b'P', 0x06, -1, "*.NTP", ""),
            (b'F', 0x06, -1, "*.TOS", ""),
            (b'P', 0x06, -1, "*.TTP", ""),
            (b'G', 0x06, -1, "*.PRG", ""),
            (b'G', 0x06, -1, "*.APP", ""),
            (b'Y', 0x06, -1, "*.GTP", ""),
            (b'D', -1, 0x02, "", "*.*"),
            (b'N', -1, 0x07, "", "*.*"),
            (b'F', -1, 0x07, "", "*.*"),
        ];
        table
            .iter()
            .map(|&(tag, aicon, dicon, pappl, pdata)| InfAnode {
                tag,
                is_desk: false,
                has_extra: matches!(tag, b'G' | b'Y' | b'F' | b'P' | b'I' | b'N'),
                xspot: 0,
                yspot: 0,
                aicon,
                dicon,
                letter: 0,
                pappl: String::from(pappl),
                pdata: String::from(pdata),
                flags: 0,
                funkey: 0,
                pargs: String::new(),
            })
            .collect()
    }
}

// ── Sort ↔ `#E`-bit mapping ─────────────────────────────────────────────────

/// Decode the `#E` env bytes to `(view, sort, size_to_fit, appdir, fullpath)`.
fn env_to_state(e1: u8, e5: u8) -> (View, Sort, bool, bool, bool) {
    let view = if e1 & INF_E1_VIEWTEXT != 0 { View::Text } else { View::Icon };
    let sort = if e5 & INF_E5_NOSORT != 0 {
        Sort::NoSort
    } else {
        match (e1 & INF_E1_SORTMASK) >> INF_E1_SORTSHIFT {
            1 => Sort::Type,
            2 => Sort::Size,
            3 => Sort::Date,
            _ => Sort::Name,
        }
    };
    let size_to_fit = e5 & INF_E5_NOSIZE == 0;
    // `cnxsave->cs_appdir = ((envr & INF_E5_APPDIR) != 0);` (`deskapp.c:942`).
    let appdir = e5 & INF_E5_APPDIR != 0;
    // `cnxsave->cs_fullpath = ((envr & INF_E5_ISFULL) != 0);` (`deskapp.c:943`).
    let fullpath = e5 & INF_E5_ISFULL != 0;
    (view, sort, size_to_fit, appdir, fullpath)
}

/// Fold `(view, sort, size_to_fit, appdir, fullpath)` into the `#E` `e1`/`e5`
/// bytes, keeping every other bit of the base bytes (confirm/dclick/format
/// prefs) unchanged.
fn state_to_env(
    view: View,
    sort: Sort,
    size_to_fit: bool,
    appdir: bool,
    fullpath: bool,
    base_e1: u8,
    base_e5: u8,
) -> (u8, u8) {
    let mut e1 = base_e1 & !(INF_E1_VIEWTEXT | INF_E1_SORTMASK);
    if view == View::Text {
        e1 |= INF_E1_VIEWTEXT;
    }
    let mut e5 = base_e5 & !(INF_E5_NOSORT | INF_E5_NOSIZE | INF_E5_APPDIR | INF_E5_ISFULL);
    match sort {
        Sort::Name => {}
        Sort::Type => e1 |= 1 << INF_E1_SORTSHIFT,
        Sort::Size => e1 |= 2 << INF_E1_SORTSHIFT,
        Sort::Date => e1 |= 3 << INF_E1_SORTSHIFT,
        Sort::NoSort => e5 |= INF_E5_NOSORT,
    }
    if !size_to_fit {
        e5 |= INF_E5_NOSIZE;
    }
    // `env5 |= cnxsave->cs_appdir ? INF_E5_APPDIR : 0;` (`deskapp.c:1164`).
    if appdir {
        e5 |= INF_E5_APPDIR;
    }
    // `env5 |= cnxsave->cs_fullpath ? INF_E5_ISFULL : 0;` (`deskapp.c:1165`).
    if fullpath {
        e5 |= INF_E5_ISFULL;
    }
    (e1, e5)
}

/// The Preferences-dialog half of the `#E` line, decoded from env bytes 1 and 2
/// — EmuTOS `app_start`'s `'E'` arm (`deskapp.c:917-934`):
///
/// ```c
/// cnxsave->cs_confdel  = ((envr & INF_E1_CONFDEL) != 0);
/// cnxsave->cs_confcpy  = ((envr & INF_E1_CONFCPY) != 0);
/// cnxsave->cs_dblclick =  (envr & INF_E1_DCMASK);
/// …
/// cnxsave->cs_confovwr = ((envr & INF_E2_ALLOWOVW) == 0);
/// cnxsave->cs_datefmt = (envr & INF_E2_IDTDATE) ? DATEFORM_IDT
///                     : (envr & INF_E2_DAYMONTH) ? DATEFORM_DMY : DATEFORM_MDY;
/// cnxsave->cs_timefmt = (envr & INF_E2_IDTTIME) ? TIMEFORM_IDT
///                     : (envr & INF_E2_24HCLOCK) ? TIMEFORM_24H : TIMEFORM_12H;
/// ```
///
/// Returns `(confirm_deletes, confirm_copies, confirm_overwrites,
/// dclick_index, time_format, date_format)`.
#[allow(clippy::type_complexity)]
fn env_to_prefs(e1: u8, e2: u8) -> (bool, bool, bool, u8, TimeFmt, DateFmt) {
    let confdel = e1 & INF_E1_CONFDEL != 0;
    let confcpy = e1 & INF_E1_CONFCPY != 0;
    let confovwr = e2 & INF_E2_ALLOWOVW == 0;
    let dclick = e1 & INF_E1_DCMASK;
    let timefmt = if e2 & INF_E2_IDTTIME != 0 {
        TimeFmt::Idt
    } else if e2 & INF_E2_24HCLOCK != 0 {
        TimeFmt::H24
    } else {
        TimeFmt::H12
    };
    let datefmt = if e2 & INF_E2_IDTDATE != 0 {
        DateFmt::Idt
    } else if e2 & INF_E2_DAYMONTH != 0 {
        DateFmt::Dmy
    } else {
        DateFmt::Mdy
    };
    (confdel, confcpy, confovwr, dclick, timefmt, datefmt)
}

/// The inverse of [`env_to_prefs`] — EmuTOS `app_save`'s env assembly
/// (`deskapp.c:1138-1160`). Every bit outside this dialog's own is taken from
/// `base_e1`/`base_e2` unchanged, so the blitter / cache bits NewTOS does not
/// edit survive a load/save round trip.
#[allow(clippy::too_many_arguments)]
fn prefs_to_env(
    confdel: bool,
    confcpy: bool,
    confovwr: bool,
    dclick: u8,
    timefmt: TimeFmt,
    datefmt: DateFmt,
    base_e1: u8,
    base_e2: u8,
) -> (u8, u8) {
    let mut e1 = base_e1 & !(INF_E1_CONFDEL | INF_E1_CONFCPY | INF_E1_DCMASK);
    if confdel {
        e1 |= INF_E1_CONFDEL;
    }
    if confcpy {
        e1 |= INF_E1_CONFCPY;
    }
    e1 |= dclick & INF_E1_DCMASK;

    let mut e2 =
        base_e2 & !(INF_E2_IDTDATE | INF_E2_IDTTIME | INF_E2_ALLOWOVW | INF_E2_DAYMONTH | INF_E2_24HCLOCK);
    // `env2 |= (cnxsave->cs_confovwr) ? 0x00 : INF_E2_ALLOWOVW;` (`:1142`).
    if !confovwr {
        e2 |= INF_E2_ALLOWOVW;
    }
    match timefmt {
        TimeFmt::Idt => e2 |= INF_E2_IDTTIME,
        TimeFmt::H24 => e2 |= INF_E2_24HCLOCK,
        TimeFmt::H12 => {}
    }
    match datefmt {
        DateFmt::Idt => e2 |= INF_E2_IDTDATE,
        DateFmt::Dmy => e2 |= INF_E2_DAYMONTH,
        DateFmt::Mdy => {}
    }
    (e1, e2)
}

// ── Boot / runtime load + apply ─────────────────────────────────────────────

/// Forget the parsed `.INF` of the run that has ended — the `inf` half of
/// [`super::reset`]. [`load_and_apply`] fills it again at the next
/// `deskmain` run; dropping it here means a file that has since disappeared
/// cannot be re-saved from a stale in-memory copy.
pub(super) fn reset() {
    *SESSION.borrow_mut() = None;
    BG_WINDOW_SELECTED.store(false, core::sync::atomic::Ordering::Relaxed);
}

/// The bytes of a shell-buffer image up to its terminating NUL — EmuTOS's
/// `infbuf` is a C string and every reader of it (`app_start`'s
/// `while (*pcurr)`, `deskapp.c:845`) stops there, while the rest of the
/// 4192-byte buffer still holds whatever the previous `shel_put` left.
fn inf_text(buf: &[u8]) -> &[u8] {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    &buf[..end]
}

/// Get the `.INF` image and apply it to the live desktop state — the
/// acquisition half of `app_start` (`deskapp.c:808-841`) plus the env/
/// background/INF-position apply loop (`:845-987`).
///
/// **The three-level fallback, in EmuTOS's order** (`:832-840`,
/// `docs/api/aes/shell-loop.md` §10):
///
/// ```c
/// shel_get(buf, SIZE_SHELBUF);                 /* :832 */
/// inf_data = buf + CPDATA_LEN;                 /* :833 */
/// if (!(bootflags & BOOTFLAG_SKIP_AUTO_ACC)
///  && (inf_data[0] != '#'))                    /* :834-835 */
///     read_inf_file(inf_data);                 /* :836 */
/// if (inf_data[0] != '#')                      /* :839 */
///     build_inf(inf_data, xcnt, ycnt);         /* :840 */
/// ```
///
/// **shell-buffer memory → disk file → built-in defaults.** The memory image is
/// what the dying desktop's `app_save(FALSE)` parked there ([`app_save`],
/// `:1293`), so a desktop that is restarted inside one AES session comes back
/// with the state it had, without touching the disk. The 128 bytes in front of
/// it ([`CPDATA_LEN`]) are not part of the `.INF` text and are never inspected
/// here.
///
/// Applies: the `#E` view/sort/size-to-fit, the `#Q` background (`g_patcol`),
/// the `#M`/`#T`/`#O` desktop-icon grid positions (cached for `app_blddesk`),
/// and the `#W` window rectangles (cached for `main::win_default_rect`). The parsed
/// image is stashed in [`SESSION`] so a later Save preserves the file-type
/// `ANODE`s and the confirm/format prefs.
///
/// Must run **after** `app_start` (so `g_icw`/`g_ich` exist for the grid
/// rounding) and **before** `win_view`/`app_blddesk` (so the view and icon
/// positions take effect). Returns `false` for "still no `.INF` data", i.e.
/// EmuTOS's `:839` test — the caller then runs [`build_inf`] (`:840`).
///
/// **One divergence, unchanged by this unit and named rather than rationalised:**
/// the `!(bootflags & BOOTFLAG_SKIP_AUTO_ACC)` term of `:834` is missing, so a
/// Ctrl-held boot still reads the file. The flag is a BIOS global EmuTOS's
/// desktop shares an address space with; NewTOS has it in
/// `crate::launch::skip_auto_acc()`, which is a kernel symbol the desktop, an
/// EL0 program, cannot name (DESK43 §6.4), and there is no trap that carries
/// it. Closing it needs that carrier first.
pub(super) fn load_and_apply() -> bool {
    // `buf = must_alloc_buf(SIZE_SHELBUF)` (`:808`) — the whole shell buffer,
    // prefix included, because `shel_get` transfers from offset 0.
    let mut buf = alloc::vec![0u8; SIZE_SHELBUF];
    super::shel_get(&mut buf); // `shel_get(buf, SIZE_SHELBUF)` (`:832`)
    // `inf_data = buf + CPDATA_LEN` (`:833`).
    let mut inf_data: Vec<u8> = Vec::from(inf_text(&buf[CPDATA_LEN..]));

    // `if (inf_data[0] != '#') read_inf_file(inf_data);` (`:834-836`).
    //
    // EmuTOS decides "is there INF data?" on the **signature**, not on whether
    // a file exists — `shel_get` hands it a buffer either way, and a buffer
    // that does not start with `#` is not an INF. Our test image ships a
    // 21-byte `newdesk placeholder\r\n` for exactly that case, so the check is
    // not theoretical: without it the desktop comes up with no icons at all.
    //
    // This is deliberately the **weaker** of EmuTOS's two signature rules: the
    // boot path tests `'#'` alone (`:833`), while *Read .INF file* demands `#R`
    // or `#E` ([`is_inf_signature`], `deskapp.c:1523-1525`). Both are EmuTOS's,
    // and they are not interchangeable — at boot the buffer may be a `shel_put`
    // image whose first line is any tag, whereas the Read path is validating a
    // file the user just picked out of a selector.
    if inf_data.first() != Some(&b'#') {
        if let Some(bytes) = read_inf_file() {
            inf_data = bytes;
        }
    }

    // `if (inf_data[0] != '#') build_inf(…);` (`:839-840`) — the caller's job.
    if inf_data.first() != Some(&b'#') {
        return false;
    }
    apply_inf_bytes(&inf_data);
    true
}

/// Parse an INF image, apply it to the live desktop and stash it in [`SESSION`]
/// — the tail both [`load_and_apply`] (boot) and [`app_read_inf`] (the
/// *Options* menu) run once they have accepted the bytes as INF data.
fn apply_inf_bytes(bytes: &[u8]) {
    let data = parse_inf(bytes);
    apply(&data);
    *SESSION.borrow_mut() = Some(data);
}

/// `*p == '#' && (*(p+1) == 'R' || *(p+1) == 'E')` (`deskapp.c:1523-1525`) —
/// the signature *Read .INF file* requires: the first line must be the revision
/// level or the environment, which is what `app_save` always writes first
/// (`deskapp.c:1215-1230`). Anything else is "not an .INF file" and gets the
/// [`STINVINF`] alert; see [`load_and_apply`] for why the boot path is allowed
/// to be laxer.
fn is_inf_signature(bytes: &[u8]) -> bool {
    bytes.first() == Some(&b'#') && matches!(bytes.get(1), Some(b'R') | Some(b'E'))
}

// ── `Options → Read .INF file` (`app_read_inf`, `deskapp.c:1491-1546`) ──────

/// `STRDINF` (`desk_rsc.c:2488`) — the file selector's caption.
const STRDINF: &[u8] = b"Select .INF file\0";

/// `INF_FILE_WILD` (`include/sysconf.h:45` = `"A:\\*.INF"`) — the path the
/// Read-.INF selector opens with.
///
/// EmuTOS patches the drive letter in at run time (`*path += G.g_stdrv`,
/// `deskapp.c:1502`) because its `INF_FILE_NAME` is patched the same way
/// (`:686`, `:1050`); the invariant is "the selector opens on the drive the
/// desktop keeps its INF file on". NewTOS pins that file to the boot drive in
/// [`INF_PATH`], so the wildcard is pinned to the same drive and the invariant
/// holds without a `g_stdrv`.
const INF_FILE_WILD: &str = "C:\\*.INF";

/// Size of the `pipath` buffer handed to `fsel_exinput`. EmuTOS uses
/// `MAXPATHLEN` (81, `sysconf.h:55`); ours has to cover what *our* AES writes
/// back, which is its own 96-byte `FSDIRECT` text plus the NUL
/// (`tos-api/src/aes/fsel.rs:135`). Rounded up to 128.
pub(super) const FSEL_PATH_LEN: usize = 128;

/// Size of the `pisel` buffer. EmuTOS uses `LEN_ZFNAME` (13); the AES writes
/// back its 16-byte `FSSELECT` text plus the NUL
/// (`tos-api/src/aes/fsel.rs:136`). Rounded up to 32.
pub(super) const FSEL_NAME_LEN: usize = 32;

/// The NUL-terminated text of a file-selector buffer.
fn cbuf_str(buf: &[u8]) -> &str {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    core::str::from_utf8(&buf[..end]).unwrap_or("")
}

/// `strcpy(buf, s)` into a fixed buffer, NUL-terminated and NUL-padded.
fn set_cbuf(buf: &mut [u8], s: &str) {
    let n = s.len().min(buf.len() - 1);
    buf[..n].copy_from_slice(&s.as_bytes()[..n]);
    buf[n..].fill(0);
}

/// `filename_start` (`util/optimize.c`) — everything after the last `\`.
fn filename_start(path: &str) -> &str {
    match path.rfind('\\') {
        Some(i) => &path[i + 1..],
        None => path,
    }
}

/// `app_read_inf` (`deskapp.c:1492-1546`) — the *Options → Read .INF file*
/// worker: prompt for a file with [`fsel_exinput`](super::fsel_exinput), load
/// it, validate its signature, **merge it into the AES shell buffer**, and
/// report the two failures by alert. Returns `TRUE` iff a file was read
/// (EmuTOS's own contract).
///
/// ```c
/// shel_get(buf, CPDATA_LEN);          /* :1532 */
/// shel_put(buf, CPDATA_LEN+rc+1);     /* :1533 — also save terminating nul */
/// ret = TRUE;
/// ```
///
/// The 128-byte control-panel prefix is read back out of the buffer and put in
/// front of the file text unchanged, exactly as [`app_save`] does; the file
/// never reaches the live desktop from here. The caller
/// ([`super::main::do_read_inf`]) ends the desktop with `restart_desktop`, and
/// the restarted one picks the image up through [`load_and_apply`]'s
/// `shel_get` (`deskapp.c:832`) — which is why this is the only write.
/// **DESK43 U3 retired the DESK14 §4 "apply in place" deviation that stood
/// here while there was no restart path.**
pub(super) fn app_read_inf() -> bool {
    // "prompt for filename" (`deskapp.c:1499-1508`): the wildcard on the INF
    // drive as the path, the desktop's own INF file name as the selection.
    let mut path = [0u8; FSEL_PATH_LEN];
    let mut fname = [0u8; FSEL_NAME_LEN];
    set_cbuf(&mut path, INF_FILE_WILD);
    set_cbuf(&mut fname, filename_start(INF_PATH));
    let (rc, button) = super::fsel_exinput(&mut path, &mut fname, STRDINF);
    if rc == 0 || button == 0 {
        return false;
    }

    // `strcpy(filename_start(path), fname)` (`:1509`) — the directory the
    // selector ended in, plus the name the user chose.
    let chosen = cbuf_str(&path);
    let leaf = String::from(cbuf_str(&fname));
    let mut full = match chosen.rfind('\\') {
        Some(i) => String::from(&chosen[..=i]),
        None => String::new(),
    };
    full.push_str(&leaf);

    // "load in file & validate contents" (`:1517-1526`):
    // `dos_load_file(path, SIZE_SHELBUF-CPDATA_LEN-1, p)` — at most
    // [`MAX_INF_TEXT`] bytes, because the destination *is* the shell buffer
    // behind its copy-data header (`sysconf.h:74`, `deskapp.c:136`).
    let loaded = match read_file(&full) {
        Ok(mut bytes) if is_inf_signature(&bytes) => {
            bytes.truncate(MAX_INF_TEXT);
            Some(bytes)
        }
        // A file that opened but is not an INF is EmuTOS's `rc = 0L`, which
        // falls through to the STINVINF arm below — as does an empty file.
        Ok(_) => None,
        Err(EFILNF) => {
            // "issue alert for file not found" (`:1536-1538`), `STFILENF` =
            // `[1][%s not found.][  OK  ]` (`desk_rsc.c:2548`) with the name
            // the user picked merged in (`fun_alert_merge`, `deskfun.c:88`).
            let mut msg = leaf;
            msg.push_str(" not found.");
            super::dir::fun_alert(1, msg.as_bytes());
            return false;
        }
        Err(_) => None,
    };
    let Some(bytes) = loaded else {
        // "issue alert for invalid inf file" (`:1540-1542`), `STINVINF` =
        // `[1][Illegal .INF file format.][  OK  ]` (`desk_rsc.c:2560`).
        super::dir::fun_alert(1, b"Illegal .INF file format.");
        return false;
    };

    // "still OK: merge contents into shell buffer" (`:1530-1534`).
    // `shel_get(buf, CPDATA_LEN)` keeps whatever a control panel parked in the
    // prefix; `shel_put(buf, CPDATA_LEN+rc+1)` stores prefix + text + the
    // terminating NUL, which is what `app_start`'s `inf_data[0] != '#'` test
    // then reads (`:834`).
    let mut buf = alloc::vec![0u8; CPDATA_LEN];
    super::shel_get(&mut buf);
    buf.extend_from_slice(&bytes);
    buf.push(0);
    super::shel_put(&buf);
    true
}

/// `build_inf` (`deskapp.c:696-752`) — synthesise the default `EMUDESK.INF`
/// and apply it, for the case [`load_and_apply`] found no INF data.
///
/// EmuTOS assembles three pieces of *text* and lets its own parser read them
/// back: `desk_inf_data1`'s `#R`/`#E`/`#Q`/`#W` lines (`deskapp.c:171-180`),
/// the drive map's `#M` lines, and `desk_inf_data2`'s file-type lines, then the
/// `#T` (and `#O`) fixtures. Here the first piece **is** [`InfData::default`]
/// — the same `#E 1A E0 00 00 60` and `#Q 41 40 43 40 43 40` byte for byte —
/// the third is [`InfData::default_type_anodes`], and only the icon half has to
/// be built, which [`build_desk_icons`] does.
///
/// The synthesised file becomes the [`SESSION`] model, exactly as EmuTOS's
/// `app_start` *parses* its `infbuf` into the live `G.g_ahead`
/// (`deskapp.c:745-752`, `:838-841`) rather than leaving the list empty. That
/// matters beyond a later Save: `app_afind_by_name` is what gives a window item
/// its icon (`win_icalc`) and what `install_window_icon` resolves a name
/// against, and both would see an empty list on a machine whose `NEWDESK.INF`
/// is missing or unreadable — EmuTOS's "can't happen" null-`ANODE` case
/// (`deskins.c:1157-1161`).
pub(super) fn build_inf() {
    let data = InfData { anodes: InfData::default_type_anodes(), ..InfData::default() };
    apply(&data);
    *SESSION.borrow_mut() = Some(data);
    build_desk_icons();
}

/// Apply a parsed [`InfData`] — EmuTOS `app_start`'s dispatch loop
/// (`deskapp.c:845-976`).
///
/// EmuTOS splits this deliberately: the `#M`/`#T`/`#O` icon spots and the `#Q`
/// pattern/colours go straight into `GLOBES` (they belong to the desktop's own
/// object tree), while the `#E` preferences and the `#W` window geometries go
/// into the **context save area** `G.g_cnxsave` and reach the live desktop only
/// later, through `cnx_get` (`deskmain.c:2055`). The same split is kept here.
fn apply(data: &InfData) {
    // #E → cnxsave->cs_view / cs_sort / cs_sizefit (`deskapp.c:994-1013`) and
    // the Preferences half (`deskapp.c:917-934`).
    let (view, sort, size_to_fit, appdir, fullpath) = env_to_state(data.e1, data.e5);
    let (confdel, confcpy, confovwr, dclick, timefmt, datefmt) = env_to_prefs(data.e1, data.e2);

    // #M/#T/#O → `G.g_ahead` (`app_parse`'s tag switch, `deskapp.c:300-346`).
    // The spot is still in grid units here; `app_start` scales it and snaps it
    // once the whole file is parsed (`:979-987`), which is what the closure
    // below does — it is the first point at which `g_icw`/`g_ich` are in hand.
    let parsed: Vec<InfAnode> = data.anodes.iter().filter(|a| a.is_desk).cloned().collect();

    // #W → cnxsave->cs_wnode[] (character cells → pixels, `deskapp.c:889-913`:
    // `pws->x_save *= gl_wchar` and so on for each of the four).
    global::with(|g| {
        g.g_cnxsave.cs_view = view;
        g.g_cnxsave.cs_sort = sort;
        g.g_cnxsave.cs_sizefit = size_to_fit;
        g.g_cnxsave.cs_confdel = confdel;
        g.g_cnxsave.cs_confcpy = confcpy;
        g.g_cnxsave.cs_confovwr = confovwr;
        g.g_cnxsave.cs_dblclick = dclick;
        g.g_cnxsave.cs_timefmt = timefmt;
        g.g_cnxsave.cs_datefmt = datefmt;
        g.g_cnxsave.cs_appdir = appdir;
        g.g_cnxsave.cs_fullpath = fullpath;
        g.g_cnxsave.cs_wnode = Default::default();
        for (slot, w) in data.windows.iter().take(NUM_WNODES).enumerate() {
            g.g_cnxsave.cs_wnode[slot] = WSave {
                x_save: w.xc as i16 * gl_wchar(),
                y_save: w.yc as i16 * gl_hchar(),
                w_save: w.wc as i16 * gl_wchar(),
                h_save: w.hc as i16 * gl_hchar(),
                hsl_save: w.hsl as i16,
                vsl_save: w.vsl as i16,
                pth_save: w.path.clone(),
            };
        }
        g.g_ahead = parsed
            .iter()
            .map(|a| {
                // `app_parse`'s tag switch for the `AF_ISDESK` tags
                // (`deskapp.c:300-346`): `#X` is a file shortcut, `#V` a
                // folder shortcut (`CONF_WITH_DESKTOP_SHORTCUTS`, `:333-343`).
                let a_type = match a.tag {
                    b'T' => AType::Trash,
                    b'O' => AType::Printer,
                    b'X' => AType::File,
                    b'V' => AType::Folder,
                    _ => AType::Disk,
                };
                // `x = pa->a_xspot * G.g_icw; y = pa->a_yspot * G.g_ich + …;
                //  snap_icon(x, y, &pa->a_xspot, &pa->a_yspot, 0, 0);`
                // (`deskapp.c:981-984`) — grid units on disk, pixels in memory.
                let (x, y) = g.snap_icon(a.xspot as i16 * g.g_icw, a.yspot as i16 * g.g_ich);
                // "mark desktop shortcuts as executable based on file
                // extension" (`deskapp.c:422-426`). NewTOS's executable
                // extensions are the native `.N?P` family plus the classic
                // m68k five ([`super::ftype`]), which is EmuTOS's `exec_ext`
                // (`deskins.c:984`) widened by ADR-018 — the same decision
                // `win_bldview` already makes for a window item.
                let a_flags = if a_type == AType::File
                    && super::ins::is_executable(a.pappl.as_bytes())
                {
                    AF_ISEXEC
                } else {
                    0
                };
                Anode {
                    a_type,
                    a_letter: a.letter,
                    a_aicon: a.aicon,
                    a_dicon: a.dicon,
                    a_flags,
                    a_pappl: a.pappl.clone(),
                    a_pdata: a.pdata.clone(),
                    a_xspot: x,
                    a_yspot: y,
                    a_obid: NIL,
                }
            })
            .collect();
    });
    // #K → `menu_shortcuts[]` (`deskapp.c:967-974`). The array is the desktop's
    // live shortcut table; the *rendering* of it into the menu strings happens
    // later, in `install_shortcuts` — EmuTOS runs that once `app_start` has
    // returned (`deskmain.c:2023`), which for NewTOS is
    // `super::menu::install_menu_bar`.
    super::menu::set_shortcuts(data.shortcuts);

    // #Q → g_patcol (the >2-plane bucket drives the live desktop background).
    let patcol = [
        data.patcol[0].0,
        data.patcol[0].1,
        data.patcol[1].0,
        data.patcol[1].1,
        data.patcol[2].0,
        data.patcol[2].1,
    ];
    global::set_patcol_all(patcol);
}

/// The desktop-icon half of `build_inf` (`deskapp.c:696-752`) — synthesise
/// `G.g_ahead` from the live drive map plus the Trash and Printer fixtures.
///
/// EmuTOS reaches this when neither `shel_get` nor the disk produced an
/// `EMUDESK.INF`: it *writes the `#M`/`#T`/`#O` text* into a buffer and lets its
/// own parser read it back. NewTOS goes straight to the nodes — the text
/// detour exists only because EmuTOS has one parser and wants one code path,
/// and a round trip through a string buffer is not observable. The **layout
/// rules are its**: drives fill the grid left-to-right, `icon_index % xcnt`
/// columns and `icon_index / xcnt` rows (`:719-721`); the Trash goes to the
/// bottom-left, moved to the bottom-**right** if the last drive icon reached
/// that row (`:738-744`); the Printer takes the bottom-right only if the Trash
/// did not (`:747-754`).
///
/// A no-op when `G.g_ahead` is already populated, so the two callers — the
/// tail of [`apply`] and the no-file arm of [`load_and_apply`] — are the same
/// call.
fn build_desk_icons() {
    if global::with(|g| !g.g_ahead.is_empty()).unwrap_or(true) {
        return;
    }
    // `drivemask = dos_sdrv(dos_gdrv());` (`deskapp.c:714`). Read outside the
    // state lock: it is a GEMDOS trap, not desktop state.
    let drivemask = dos_sdrv(dos_gdrv());

    global::with(|g| {
        let icw = g.g_icw.max(1);
        let ich = g.g_ich.max(1);
        // `build_inf`'s grid counts are `app_start`'s locals, derived from the
        // cell size and the inter-icon gap — **not** from `g_icw`/`g_ich`
        // (`deskapp.c:813-817`, `xcnt = G.g_desk.g_w / (G.g_wicon+MIN_WINT)`).
        // The two agree whenever the division is exact and can differ by one
        // column when it is not, so the source formula is the one used here.
        let screen_h = g.g_desk.y + g.g_desk.h;
        let xcnt = (g.g_desk.w / (g.g_wicon + global::min_wint(screen_h))).max(1);
        let ycnt = (g.g_desk.h / (g.g_hicon + global::MIN_HINT)).max(1);

        let mut icon_index = 0i16;
        let mut drive_y = 0i16;
        for drive in 0..26u32 {
            if drivemask & (1 << drive) == 0 {
                continue;
            }
            let drive_x = icon_index % xcnt;
            drive_y = icon_index / xcnt;
            let letter = b'A' + drive as u8;
            // `icon_type = (i > 1) ? IG_HARD : IG_FLOPPY;` (`deskapp.c:721`).
            let aicon = if drive > 1 { icons::IG_HARD } else { icons::IG_FLOPPY };
            g.g_ahead.push(Anode {
                a_type: AType::Disk,
                a_letter: letter,
                a_aicon: aicon,
                a_dicon: NIL,
                a_flags: 0,
                a_pappl: disk_label(letter),
                a_pdata: String::new(),
                a_xspot: drive_x * icw,
                a_yspot: drive_y * ich,
                a_obid: NIL,
            });
            icon_index += 1;
        }

        // Trash: bottom-left, or bottom-right when the drives already reached
        // the bottom row (`deskapp.c:738-744`).
        let icon_y = ycnt - 1;
        let trash_x = if drive_y >= icon_y { xcnt - 1 } else { 0 };
        g.g_ahead.push(Anode {
            a_type: AType::Trash,
            a_letter: 0,
            a_aicon: icons::IG_TRASH,
            a_dicon: NIL,
            a_flags: 0,
            a_pappl: String::from(TRASH_LABEL),
            a_pdata: String::new(),
            a_xspot: trash_x * icw,
            a_yspot: icon_y * ich,
            a_obid: NIL,
        });

        // Printer: only "if room", i.e. only when the Trash stayed on the left
        // (`deskapp.c:747-754`, `CONF_WITH_PRINTER_ICON`).
        if trash_x == 0 {
            g.g_ahead.push(Anode {
                a_type: AType::Printer,
                a_letter: 0,
                a_aicon: icons::IG_PRINT,
                a_dicon: NIL,
                a_flags: 0,
                a_pappl: String::from(PRINTER_LABEL),
                a_pdata: String::new(),
                a_xspot: (xcnt - 1) * icw,
                a_yspot: icon_y * ich,
                a_obid: NIL,
            });
        }
    });
}

// ── The built-in desktop-icon labels ────────────────────────────
//
// EmuTOS's wording, `desk_rsc.c:2481-2483` (`STDISK`/`STTRASH`/`STPRINT`). Both
// a hard disk and a floppy get `STDISK` plus the drive letter — `build_inf`
// prints `"%s %c"` (`deskapp.c:723`) and `install_drive` builds the same string
// (`deskins.c:247`), so an icon added by *Install devices* is labelled exactly
// like one the desktop built at boot. The icon *picture* is what distinguishes
// the two (`IG_HARD` vs `IG_FLOPPY`), not the text.

/// `STDISK` — the word a drive icon's label starts with; [`disk_label`] appends
/// the drive letter.
pub(super) const DISK_LABEL: &str = "DISK";
/// Label of the Trash icon, `STTRASH`.
pub(super) const TRASH_LABEL: &str = "TRASH";
/// Label of the Printer icon, `STPRINT`.
pub(super) const PRINTER_LABEL: &str = "PRINTER";

/// `sprintf(G.g_work, "%s %c", desktop_str_addr(STDISK), pa->a_letter)`
/// (`deskins.c:247`, and the same text from `build_inf`, `deskapp.c:723`).
pub(super) fn disk_label(letter: u8) -> String {
    let mut s = String::from(DISK_LABEL);
    s.push(' ');
    s.push(letter as char);
    s
}

/// The pixel rectangle to open the window in slot `slot` at, taken from that
/// slot's [`WSave`] — EmuTOS `win_alloc`, which reads the new window's default
/// geometry straight out of `G.g_cnxsave->cs_wnode[G.g_wcnt]`
/// (`deskwin.c:165-177`). [`None`] when the slot holds no saved window, and the
/// caller then uses its own staggered fallback.
///
/// **Named deviation.** EmuTOS takes that rectangle *unconditionally*, because
/// its built-in `EMUDESK.INF` always supplies four sensible default `#W` lines
/// (`desk_inf_data1`, `deskapp.c:177-180`: `02 06 26 0C`, `02 08 26 0C`, …)
/// with empty paths — geometry without a window. NewTOS ships no such defaults,
/// and `cnx_get`'s validation turns an all-zero slot into the **whole desktop**
/// (`w_save <= 0` → `g_desk.g_w`, `deskmain.c:1476-1483`), so taking it
/// unconditionally would open every fresh window full-screen. Until the built-in
/// `#W` defaults are adopted — a visible change to the boot picture, and a
/// decision of its own — an unused slot supplies no rectangle.
pub(super) fn window_rect(slot: usize) -> Option<Grect> {
    global::with(|g| {
        let ws = g.g_cnxsave.cs_wnode.get(slot)?;
        (!ws.pth_save.is_empty() && ws.w_save > 0 && ws.h_save > 0)
            .then_some(Grect { x: ws.x_save, y: ws.y_save, w: ws.w_save, h: ws.h_save })
    })
    .flatten()
}

// ── Save (`app_save`, `deskapp.c:1106-1290`) ────────────────────────────────

/// Build an [`InfData`] snapshot — EmuTOS `app_save`.
///
/// `app_save` reads the **context save area**, not the live desktop: the
/// `#E` preference bits come from `cnxsave->cs_view`/`cs_sort`/`cs_sizefit`
/// (`deskapp.c:1136-1170`) and every `#W` line from `cnxsave->cs_wnode[]`
/// (`:1193-1203`). `G.g_cnxsave` is brought up to date by the `cnx_put()` its
/// callers run first (`deskmain.c:891,2154`). The desktop-icon spots and
/// `g_patcol` are read from the live `GLOBES`, exactly as EmuTOS does.
fn build_from_state() -> InfData {
    let session = SESSION.borrow().clone();

    // Env + desktop-icon anodes, under one lock.
    #[allow(clippy::type_complexity)]
    type Prefs = (bool, bool, bool, u8, TimeFmt, DateFmt);
    #[allow(clippy::type_complexity)]
    let (view, sort, size_to_fit, appdir, fullpath, prefs, desk_anodes): (View, Sort, bool, bool, bool, Prefs, Vec<InfAnode>) = global::with(|g| {
        // The `ANODE` half of `app_save` (`deskapp.c:1216-1262`), for the
        // `AF_ISDESK` nodes: one line per entry of `G.g_ahead`, in list order,
        // with the spot converted back to grid units
        // (`pa->a_xspot/G.g_icw`, `:1258-1259`). Reading the **list** and not
        // the object tree is what EmuTOS does, and it is why a node whose icon
        // could not be allocated (`a_obid == NIL`) still survives a save.
        let icw = g.g_icw.max(1);
        let ich = g.g_ich.max(1);
        let anodes: Vec<InfAnode> = g
            .g_ahead
            .iter()
            .map(|pa| InfAnode {
                // `app_save`'s type switch (`deskapp.c:1216-1256`): an
                // `AF_ISDESK` node of type `AT_ISFILE` is written `#X` and one
                // of type `AT_ISFOLD` `#V` (`:1224-1227`, `:1239-1243`).
                tag: match pa.a_type {
                    AType::Trash => b'T',
                    AType::Printer => b'O',
                    AType::File => b'X',
                    AType::Folder => b'V',
                    _ => b'M',
                },
                is_desk: true,
                has_extra: false,
                xspot: (pa.a_xspot / icw).clamp(0, 255) as u8,
                yspot: (pa.a_yspot / ich).clamp(0, 255) as u8,
                aicon: pa.a_aicon,
                // A drive/trash/printer icon has no data icon: EmuTOS's
                // `install_drive` and `install_desktop_icon` both set
                // `a_dicon = NIL` (`deskins.c:250`, `:854`), which `app_save`
                // writes as `FF`. A shortcut carries the number it copied from
                // the `ANODE` that governs the file.
                dicon: pa.a_dicon,
                letter: pa.a_letter,
                pappl: pa.a_pappl.clone(),
                pdata: pa.a_pdata.clone(),
                flags: 0,
                funkey: 0,
                pargs: String::new(),
            })
            .collect();
        (
            g.g_cnxsave.cs_view,
            g.g_cnxsave.cs_sort,
            g.g_cnxsave.cs_sizefit,
            g.g_cnxsave.cs_appdir,
            g.g_cnxsave.cs_fullpath,
            (
                g.g_cnxsave.cs_confdel,
                g.g_cnxsave.cs_confcpy,
                g.g_cnxsave.cs_confovwr,
                g.g_cnxsave.cs_dblclick,
                g.g_cnxsave.cs_timefmt,
                g.g_cnxsave.cs_datefmt,
            ),
            anodes,
        )
    })
    .unwrap_or((
        View::Icon,
        Sort::Name,
        true,
        true,
        true,
        (true, true, true, global::DEF_DCLICK, TimeFmt::H24, DateFmt::Mdy),
        Vec::new(),
    ));

    // Base env / patcol / file-type anodes: preserved from the session, or the
    // built-in defaults on a first-ever save.
    let base = session.unwrap_or_default();
    let (e1, e5) = state_to_env(view, sort, size_to_fit, appdir, fullpath, base.e1, base.e5);
    // The Preferences bits go into the *same* `e1` byte, in a disjoint bit
    // range (`CONFDEL|DCMASK` vs `VIEWTEXT|SORTMASK`), so they are folded in on
    // top of the result rather than from `base.e1` again.
    let (confdel, confcpy, confovwr, dclick, timefmt, datefmt) = prefs;
    let (e1, e2) = prefs_to_env(confdel, confcpy, confovwr, dclick, timefmt, datefmt, e1, base.e2);

    let mut anodes = desk_anodes;
    let type_anodes: Vec<InfAnode> = if base.anodes.iter().any(|a| !a.is_desk) {
        base.anodes.iter().filter(|a| !a.is_desk).cloned().collect()
    } else {
        InfData::default_type_anodes()
    };
    anodes.extend(type_anodes);

    InfData {
        e1,
        e2,
        e5,
        patcol: {
            let p = global::patcol_all();
            [(p[0], p[1]), (p[2], p[3]), (p[4], p[5])]
        },
        // The live array, not the session copy: the Desktop-configuration
        // dialog edits it in place, and `app_save` writes `menu_shortcuts[]`
        // (`deskapp.c:1188-1189`).
        shortcuts: super::menu::shortcuts(),
        autorun: base.autorun,
        autorun_gem: base.autorun_gem,
        windows: collect_open_windows(),
        anodes,
    }
}

/// The `#W` lines of `app_save` (`deskapp.c:1192-1203`): **every** slot of the
/// context save area, in slot order, converted from pixels to character cells.
/// EmuTOS writes all `NUM_WNODES` lines unconditionally — an unused slot is a
/// line with an empty path — because the rectangles of the unused slots are the
/// default geometry `win_alloc` hands the next window that opens.
fn collect_open_windows() -> Vec<InfWindow> {
    global::with(|g| {
        g.g_cnxsave
            .cs_wnode
            .iter()
            .map(|ws| InfWindow {
                hsl: ws.hsl_save.clamp(0, 255) as u8,
                vsl: ws.vsl_save.clamp(0, 255) as u8,
                xc: (ws.x_save / gl_wchar()).clamp(0, 255) as u8,
                yc: (ws.y_save / gl_hchar()).clamp(0, 255) as u8,
                wc: (ws.w_save / gl_wchar()).clamp(0, 255) as u8,
                hc: (ws.h_save / gl_hchar()).clamp(0, 255) as u8,
                path: ws.pth_save.clone(),
            })
            .collect()
    })
    .unwrap_or_default()
}

/// `app_save(todisk)` (`deskapp.c:1106-1299`) — *"save the current state of all
/// the desktop options/icons/windows etc **to memory** and, optionally, to a
/// file called EMUDESK.INF"*. Both callers run `cnx_put()` first
/// (`deskmain.c:891` *Save desktop*, `:2154` the desktop epilog).
///
/// ```c
/// outbuf = must_alloc_buf(SIZE_SHELBUF+MAX_SIZE_INF_LINE);
/// shel_get(outbuf, CPDATA_LEN);            /* :1123 */
/// inf_data = outbuf + CPDATA_LEN;          /* :1124 */
/// …                                        /* :1128-1284, the INF text */
/// *pcurr = 0x00;                           /* :1285 */
/// len = pcurr - outbuf;                    /* :1290 */
/// shel_put(outbuf, len+1);                 /* :1293 */
/// if (todisk) save_to_disk(inf_data, len-CPDATA_LEN);   /* :1296-1297 */
/// ```
///
/// The 128-byte control-panel prefix is pulled **out** of the shared buffer
/// first and put back in front of the fresh text unchanged, so whatever a
/// control panel left there survives every save; the disk file never contains
/// it. `MAX_SIZE_INF_LINE` is EmuTOS's "one line of slack so the overflow check
/// only has to run at end of line" — a `Vec` needs no such headroom, and the
/// check itself is in [`InfData::serialize_into`].
///
/// Returns whether the disk write succeeded — NewTOS's addition for
/// `SAVEITEM`'s alert; `app_save(FALSE)`, which writes no file, is always
/// `true`.
pub(super) fn app_save(todisk: bool) -> bool {
    let data = build_from_state();

    // `shel_get(outbuf, CPDATA_LEN)` (`:1123`) — round-trip **only** the
    // prefix, then `inf_data = outbuf + CPDATA_LEN` (`:1124`).
    let mut outbuf = alloc::vec![0u8; CPDATA_LEN];
    super::shel_get(&mut outbuf);
    data.serialize_into(&mut outbuf);
    // `len = pcurr - outbuf` (`:1290`) is measured before the NUL; `shel_put`
    // then stores `len+1` bytes, "also save terminating NUL" (`:1293`).
    let len = outbuf.len();
    outbuf.push(0);
    super::shel_put(&outbuf);

    // `if (todisk) save_to_disk(inf_data, len-CPDATA_LEN);` (`:1296-1297`).
    if !todisk {
        return true;
    }
    let ok = write_inf_file(&outbuf[CPDATA_LEN..len]);
    if ok {
        *SESSION.borrow_mut() = Some(data);
    }
    ok
}

// ── Install Application: the `ANODE` application-association model ────────────
//
// EmuTOS `deskins.c` `ins_app_dialog` creates/edits an `ANODE` whose `a_pappl`
// is the full path of an application and whose `a_pdata` is a document wildcard
// (`*.EXT`), so that later double-clicking a matching document launches the
// application with the document as its command tail (`desksupp.c` `do_aopen`'s
// `installed_datafile` branch). NewTOS keeps that association in the same
// `NEWDESK.INF` `#G`/`#Y`/`#P`/`#F` line an `ANODE` serialises to (see
// [`InfData::serialize`] and EmuTOS `deskapp.c:1216-1273` `app_save`), so it
// survives Save Desktop + reboot exactly like the built-in type defaults.

/// The application flavour chosen in the Install-Application dialog — selects
/// the `NEWDESK.INF` line tag exactly as EmuTOS `app_save` (`deskapp.c:1232-
/// 1235`) derives it from `AF_ISCRYS`/`AF_ISPARM`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum AppFlavor {
    /// GEM application, no parameters (`AF_ISCRYS`) → `#G`.
    Gem,
    /// Plain TOS application, no parameters → `#F`.
    Tos,
    /// GEM application that takes parameters (`AF_ISCRYS|AF_ISPARM`) → `#Y`.
    Gtp,
    /// TOS application that takes parameters (`AF_ISPARM`) → `#P`.
    Ttp,
}

impl AppFlavor {
    /// The `NEWDESK.INF` `ANODE` tag byte for this flavour (`app_save`).
    fn tag(self) -> u8 {
        match self {
            AppFlavor::Gem => b'G',
            AppFlavor::Tos => b'F',
            AppFlavor::Gtp => b'Y',
            AppFlavor::Ttp => b'P',
        }
    }

    /// Recover the flavour from a serialised `ANODE` tag byte (the load path).
    fn from_tag(tag: u8) -> AppFlavor {
        match tag {
            b'G' => AppFlavor::Gem,
            b'Y' => AppFlavor::Gtp,
            b'P' => AppFlavor::Ttp,
            _ => AppFlavor::Tos, // 'F' and anything else
        }
    }
}

/// A resolved document→application association, returned by
/// [`lookup_association`] for the desktop's document-open path.
#[derive(Clone)]
pub(super) struct Association {
    /// Full `C:` path of the associated application (`a_pappl`).
    pub app_path: String,
    /// `AF_ISFULL`: pass the document's full path in the tail (else file name).
    pub full_path: bool,
    /// `AF_APPDIR` (`INF_AT_APPDIR`): launch with the current directory set to
    /// the **application's** directory; `false` ⇒ to the top window's when
    /// there is one (`do_aopen`, `desksupp.c:896-902`).
    pub app_dir: bool,
    /// The stored fixed argument prefix (`a_pargs`), prepended to the tail.
    pub args: String,
}

/// Turn a document-type field (the extension the user types in the dialog, e.g.
/// `TXT`) into the stored `a_pdata` wildcard, mirroring EmuTOS
/// `ins_app_dialog` (`deskins.c:559-562`): prefix `*.`, upper-case, and collapse
/// a leading `*` to the all-files `*.*` spec.
fn doc_mask_from_ext(ext: &str) -> String {
    let e = ext.trim().trim_start_matches("*.").to_ascii_uppercase();
    if e.starts_with('*') || e.is_empty() {
        String::from("*.*")
    } else {
        let mut s = String::from("*.");
        s.push_str(&e);
        s
    }
}

/// The fields of an installed application's `ANODE` the Install-Application
/// dialog pre-fills itself from — EmuTOS reads them straight off the `ANODE *`
/// it already holds (`deskins.c:470-503`).
pub(super) struct Installed {
    /// `a_pdata` minus its `*.` prefix — the document-type field.
    pub doc_ext: String,
    /// The `#`-line tag, i.e. the GEM/TOS/GTP/TTP flavour radio.
    pub flavor: AppFlavor,
    /// `AF_ISFULL` ([`INF_AT_ISFULL`]) — the *Parameter* radio.
    pub full_path: bool,
    /// `a_funkey` — the function-key field (0 = none).
    pub funkey: u8,
}

/// The association record currently held for `app_path`, for pre-filling the
/// dialog when an application is already installed. [`None`] if it is not
/// installed.
pub(super) fn installed_doctype(app_path: &str) -> Option<Installed> {
    let guard = SESSION.borrow();
    let data = guard.as_ref()?;
    let a = data
        .anodes
        .iter()
        .find(|a| !a.is_desk && is_real_path(&a.pappl) && a.pappl.eq_ignore_ascii_case(app_path))?;
    let ext = a.pdata.strip_prefix("*.").unwrap_or(&a.pdata);
    Some(Installed {
        doc_ext: String::from(ext),
        flavor: AppFlavor::from_tag(a.tag),
        full_path: a.flags & INF_AT_ISFULL != 0,
        funkey: a.funkey,
    })
}

/// `get_funkey`'s duplicate check (`deskins.c:414-427`), in two halves because
/// the question the user is asked sits between them and a trap must not run
/// while [`SESSION`] is borrowed:
///
/// ```c
/// for (an = G.g_ahead; an; an = an->a_next) {
///     if (an == pa) continue;
///     if (an->a_funkey == funkey) {
///         if (fun_alert(1,STDUPKEY) != 1) return -1;
///         an->a_funkey = 0;
///         break;
///     }
/// }
/// ```
///
/// This is the `if` — does an application **other** than `except_app` hold
/// `funkey`? `except_app` is EmuTOS's `if (an == pa) continue;`: for a
/// re-install it is the node being edited, and for a first install it names no
/// node at all (EmuTOS's `pa` is freshly allocated with `a_funkey == 0`, so its
/// own node can never be the hit either way).
pub(super) fn funkey_owner(funkey: u8, except_app: &str) -> bool {
    let guard = SESSION.borrow();
    let Some(data) = guard.as_ref() else {
        return false;
    };
    data.anodes
        .iter()
        .any(|a| !a.is_desk && a.funkey == funkey && !a.pappl.eq_ignore_ascii_case(except_app))
}

/// The `an->a_funkey = 0; break;` of that walk (`deskins.c:424-425`) — take the
/// key away from the **first** other application that holds it, once the user
/// has confirmed. Returns whether one was found.
pub(super) fn release_funkey(funkey: u8, except_app: &str) -> bool {
    let mut guard = SESSION.borrow_mut();
    let Some(data) = guard.as_mut() else {
        return false;
    };
    for a in data.anodes.iter_mut() {
        if !a.is_desk && a.funkey == funkey && !a.pappl.eq_ignore_ascii_case(except_app) {
            a.funkey = 0;
            return true;
        }
    }
    false
}

/// What [`app_by_funkey`] hands back: the two `ANODE` fields `do_aopen` reads
/// for an **installed** application launched from a function key.
pub(super) struct FunkeyApp {
    /// `pa->a_pappl` — the application's full path.
    pub app_path: String,
    /// `pa->a_flags & AF_APPDIR` ([`INF_AT_APPDIR`]) — "set the current
    /// directory to the application's own", the choice `do_aopen` makes for an
    /// installed application (`desksupp.c:884-890`).
    pub app_dir: bool,
}

/// `process_funkey`'s `ANODE` search (EmuTOS `deskmain.c:1092-1094`):
///
/// ```c
/// for (pa = G.g_ahead; pa; pa = pa->a_next)
///     if (pa->a_funkey == funkey)
///         break;
/// ```
///
/// EmuTOS walks the single `G.g_ahead` list, which holds the desktop icons and
/// the installed applications together; NewTOS splits the two ([`Anode`]'s
/// doc-comment), and only the association half this module owns can carry
/// `a_funkey` at all — a desktop node is created with `a_funkey = 0`
/// (`deskins.c:243,825,1037,1385`) and nothing ever sets it, so walking the
/// association list alone finds exactly the same node.
pub(super) fn app_by_funkey(funkey: u8) -> Option<FunkeyApp> {
    if funkey == 0 {
        return None; // 0 is "no key", never a match
    }
    let guard = SESSION.borrow();
    let data = guard.as_ref()?;
    data.anodes
        .iter()
        .find(|a| !a.is_desk && a.funkey == funkey)
        .map(|a| FunkeyApp { app_path: a.pappl.clone(), app_dir: a.flags & INF_AT_APPDIR != 0 })
}

/// Whether `pappl` is a concrete file path rather than a wildcard/empty
/// placeholder — EmuTOS `is_installed` (`deskins.c:71-80`).
fn is_real_path(pappl: &str) -> bool {
    !matches!(pappl.as_bytes().first(), None | Some(b'*') | Some(b'?'))
}

/// Ensure [`SESSION`] exists and carries the built-in file-type default
/// `ANODE`s, so a subsequently-added association is preserved by
/// [`build_from_state`]'s "any non-desk anode" branch on the next Save.
fn session_with_type_defaults(guard: &mut Option<InfData>) -> &mut InfData {
    let data = guard.get_or_insert_with(InfData::default);
    if !data.anodes.iter().any(|a| !a.is_desk) {
        data.anodes.extend(InfData::default_type_anodes());
    }
    data
}

/// Install (or re-install) an application association into the in-memory INF
/// model — EmuTOS `ins_app_dialog`'s `APINSTAL` branch (`deskins.c:511-582`).
/// `app_path` is the application's full `C:` path, `doc_ext` the document-type
/// extension the user entered, `flavor` the GEM/TOS/(G)TP flavour, `full_path`
/// the `AF_ISFULL` choice, `appdir` the `AF_APPDIR` choice, `funkey` the
/// validated function key (`pa->a_funkey = funkey;`, `deskins.c:531`; 0 = none)
/// and `args` the fixed argument prefix. Re-installing the same application
/// updates its existing record in place (one association record per
/// application, as EmuTOS `app_afind_by_name` resolves). The change is
/// persisted by Save Desktop.
#[allow(clippy::too_many_arguments)]
pub(super) fn install_application(
    app_path: &str,
    doc_ext: &str,
    flavor: AppFlavor,
    full_path: bool,
    appdir: bool,
    funkey: u8,
    args: &str,
) {
    let mut guard = SESSION.borrow_mut();
    let data = session_with_type_defaults(&mut guard);

    let tag = flavor.tag();
    let pdata = doc_mask_from_ext(doc_ext);
    let flags = (if appdir { INF_AT_APPDIR } else { 0 }) | (if full_path { INF_AT_ISFULL } else { 0 });

    if let Some(a) = data
        .anodes
        .iter_mut()
        .find(|a| !a.is_desk && is_real_path(&a.pappl) && a.pappl.eq_ignore_ascii_case(app_path))
    {
        a.tag = tag;
        a.has_extra = true;
        a.aicon = IG_APPL;
        a.dicon = IG_DOCU;
        a.pdata = pdata;
        a.flags = flags;
        a.funkey = funkey;
        a.pargs = String::from(args);
    } else {
        // New application: insert ahead of the generic type defaults so a
        // document matches the specific installed app first (EmuTOS `app_alloc`
        // links new ANODEs at the head; `app_save` reverses so generics trail).
        data.anodes.insert(
            0,
            InfAnode {
                tag,
                is_desk: false,
                has_extra: true,
                xspot: 0,
                yspot: 0,
                aicon: IG_APPL,
                dicon: IG_DOCU,
                letter: 0,
                pappl: String::from(app_path),
                pdata,
                flags,
                funkey,
                pargs: String::from(args),
            },
        );
    }
}

/// Remove an installed application association — EmuTOS `ins_app_dialog`'s
/// `APREMOVE` branch (`deskins.c:584-589`). Returns whether a record was
/// removed. The change is persisted by Save Desktop.
pub(super) fn remove_application(app_path: &str) -> bool {
    let mut guard = SESSION.borrow_mut();
    let Some(data) = guard.as_mut() else {
        return false;
    };
    let before = data.anodes.len();
    data.anodes
        .retain(|a| !(!a.is_desk && is_real_path(&a.pappl) && a.pappl.eq_ignore_ascii_case(app_path)));
    data.anodes.len() != before
}

/// Resolve a document `name` (a bare 8.3 file name, e.g. `README.TXT`) to its
/// installed application, if any — the lookup EmuTOS performs implicitly when it
/// binds an `FNODE` to its matching `ANODE` during a directory scan and then
/// `do_aopen` sees `is_installed(pa) && !isapp`. Walks the session `ANODE`s for
/// the first *installed* file record whose `a_pdata` wildcard matches `name`.
pub(super) fn lookup_association(name: &[u8]) -> Option<Association> {
    let guard = SESSION.borrow();
    let data = guard.as_ref()?;
    for a in &data.anodes {
        if a.is_desk || !is_real_path(&a.pappl) || a.pdata.is_empty() {
            continue;
        }
        if super::model::wildcmp(a.pdata.as_bytes(), name) {
            return Some(Association {
                app_path: a.pappl.clone(),
                full_path: a.flags & INF_AT_ISFULL != 0,
                app_dir: a.flags & INF_AT_APPDIR != 0,
                args: a.pargs.clone(),
            });
        }
    }
    None
}

// ── Window icons: the `ANODE`-by-name layer (`CONF_WITH_WINDOW_ICONS`) ──────
//
// EmuTOS keeps *one* `ANODE` list for the desktop icons, the installed
// applications, the generic file types **and** the per-name window icons
// (`#I`/`#N`/`#D` lines whose `a_pdata` is a literal file name). NewTOS splits
// the list in two — the desktop half is `DeskState::g_ahead`, this module's
// [`SESSION`] model is everything else — so the four primitives EmuTOS's
// `install_window_icon` calls on that list live here, next to the store they
// edit: [`app_afind_by_name`] (`deskapp.c:1437-1466`),
// [`allocate_window_anode`] (`deskins.c:1028-1052`), [`set_anode_data`]
// (`scan_str(new_name, &pa->a_pdata)`, `deskins.c:1175`) and [`set_icon`]
// (`deskins.c:1012-1022`), plus [`app_free`] (`deskapp.c:227-247`).
//
// `CONF_WITH_WINDOW_ICONS` defaults to 1 (`include/config.h:1866-1868`) and is
// forced off only inside `#ifdef TARGET_192` (`:210-336`), which our
// Falcon030/TOS4 `make 512` profile does not define — so the feature is part of
// the reference build, not an optional extra.

/// One hit from [`app_afind_by_name`] — EmuTOS returns the `ANODE *` and writes
/// the match kind through `pisapp`; this carries the same two answers plus the
/// fields the two callers read off the node.
pub(super) struct Afind {
    /// `a_type` of the node that matched. `app_afind_by_name` only ever
    /// returns a node whose `a_type` equals the queried one
    /// (`deskapp.c:1450`), so this is the caller's own `atype` handed back —
    /// carried because `i_find`'s consumers test it (`gr_drgplns`'s
    /// "don't highlight plain files", `deskact.c:340`).
    pub atype: AType,
    /// Index of the node in the session list, for [`set_icon`]/[`app_free`].
    /// Valid only until the list is next mutated; both callers are modal
    /// dialogs that hold it across nothing but their own edits, exactly as
    /// EmuTOS holds the raw `ANODE *`.
    pub idx: usize,
    /// `*pisapp`: `true` when the name matched `a_pappl` (application name),
    /// `false` when it matched the `a_pdata` wildcard (data name).
    pub isapp: bool,
    /// `a_aicon` — the icon shown when [`Afind::isapp`].
    pub aicon: i16,
    /// `a_dicon` — the icon shown otherwise.
    pub dicon: i16,
    /// `a_pdata`, for `install_window_icon`'s `identical` test
    /// (`deskins.c:1162`).
    pub pdata: String,
    /// `a_pappl` — the application path, for `is_installed(pa)`
    /// (`deskins.c:71-80`) and for `do_aopen`'s directory choice
    /// (`desksupp.c:886-890`), both of which `fun_win2win`'s
    /// drop-on-an-application branch reaches.
    pub pappl: String,
    /// `pa->a_flags & AF_APPDIR` ([`INF_AT_APPDIR`]) — the same `do_aopen`
    /// step, for the case the node **is** an installed application.
    pub appdir: bool,
}

impl Afind {
    /// `is_installed(pa)` (`deskins.c:71-80`) — the node names a concrete
    /// application path rather than a wildcard placeholder or nothing at all.
    pub fn is_installed(&self) -> bool {
        is_real_path(&self.pappl)
    }
}

/// The `a_type` an `ANODE` line tag stands for (`app_parse`'s switch,
/// `deskapp.c:300-346`). The session list never holds `AF_ISDESK` nodes, but
/// the tag mapping is the full one so it stays correct if it ever does.
fn anode_type(tag: u8) -> AType {
    match tag {
        b'T' => AType::Trash,
        b'M' => AType::Disk,
        b'O' => AType::Printer,
        b'D' | b'V' => AType::Folder,
        _ => AType::File, // G, Y, F, P, I, N, X
    }
}

/// `app_afind_by_name` (`deskapp.c:1437-1466`) — the first `ANODE` of type
/// `atype` whose name matches `pname`, searching the session list in EmuTOS's
/// live-list order (see [`InfData::default_type_anodes`]).
///
/// `pspec` is the window's `p_spec`; EmuTOS builds `pathname` from it by
/// replacing the file-name part with `pname`, because an *installed*
/// application is matched on its fully-qualified `a_pappl` and not on a
/// wildcard. Pass `""` for the "no particular directory" query
/// `install_window_icon` makes when it has no `FNODE` (`deskins.c:1153`).
///
/// **`ignore`.** EmuTOS's callers pass `AF_ISDESK|AF_VIEWER` (`deskwin.c:382`)
/// or `AF_ISDESK` (`deskins.c:1153`). NewTOS's desktop half is a different list
/// entirely, so `AF_ISDESK` is the [`InfAnode::is_desk`] skip below; `AF_VIEWER`
/// is `CONF_WITH_VIEWER_SUPPORT`, which this desktop does not implement, so no
/// node can carry it and the two `ignore` masks coincide.
///
/// The `a_pappl` compare is EmuTOS's `strcmp`, i.e. case-**sensitive**, not the
/// case-blind compare [`installed_doctype`] uses on a path the user typed.
pub(super) fn app_afind_by_name(atype: AType, pspec: &str, pname: &str) -> Option<Afind> {
    // `strcpy(pathname,pspec); strcpy(filename_start(pathname),pname);`
    let dir_len = pspec.len() - filename_start(pspec).len();
    let mut pathname = String::from(&pspec[..dir_len]);
    pathname.push_str(pname);

    let guard = SESSION.borrow();
    let data = guard.as_ref()?;
    for (idx, a) in data.anodes.iter().enumerate() {
        if a.is_desk {
            continue; // `if (pa->a_flags & ignore) continue;`
        }
        if anode_type(a.tag) != atype {
            continue;
        }
        let hit = |isapp: bool| {
            Some(Afind {
                atype,
                idx,
                isapp,
                aicon: a.aicon,
                dicon: a.dicon,
                pdata: a.pdata.clone(),
                pappl: a.pappl.clone(),
                appdir: a.flags & INF_AT_APPDIR != 0,
            })
        };
        if super::model::wildcmp(a.pdata.as_bytes(), pname.as_bytes()) {
            return hit(false);
        }
        let matched = match a.pappl.as_bytes().first() {
            Some(b'*') | Some(b'?') => super::model::wildcmp(a.pappl.as_bytes(), pname.as_bytes()),
            _ => a.pappl == pathname,
        };
        if matched {
            return hit(true);
        }
    }
    None
}

/// `allocate_window_anode` (`deskins.c:1028-1052`) — a fresh `AF_WINDOW` node
/// for a file or folder name, linked at the head of the list so a per-name
/// icon is found before the generic `*.*` entries (EmuTOS's `app_alloc` links
/// at the head, `deskapp.c:204-219`). Returns its index.
///
/// **No allocation failure.** EmuTOS's pool is the fixed `NUM_ANODES` (64)
/// array `G.g_alist` and `app_alloc` returns `NULL` when it is full, which is
/// what `install_window_icon`'s `if (!pa) { change = -1; … }` arm
/// (`deskins.c:1170-1174`) exists for. NewTOS's list is a `Vec` with no fixed
/// pool — a ROM-scarcity constraint whose absence is unobservable except as
/// "the alert never appears" — so that arm has no counterpart.
pub(super) fn allocate_window_anode(atype: AType) -> usize {
    let mut guard = SESSION.borrow_mut();
    let data = session_with_type_defaults(&mut guard);
    // `pa->a_dicon = (type==AT_ISFOLD) ? IG_FOLDER : IG_DOCU;` (`:1049`), and
    // the save tag `app_save` gives that node before `set_icon` runs
    // (`deskapp.c:1223-1244`): a folder is `#D`, a not-yet-executable file `#N`.
    let (tag, dicon) = match atype {
        AType::Folder => (b'D', icons::IG_FOLDER),
        _ => (b'N', IG_DOCU),
    };
    data.anodes.insert(
        0,
        InfAnode {
            tag,
            is_desk: false,
            has_extra: tag != b'D',
            xspot: 0,
            yspot: 0,
            aicon: NIL,
            dicon,
            letter: 0,
            pappl: String::new(),
            pdata: String::new(),
            flags: 0,
            funkey: 0,
            pargs: String::new(),
        },
    );
    0
}

/// `scan_str(new_name, &pa->a_pdata)` (`deskins.c:1175`) — give the node at
/// `idx` its data name.
pub(super) fn set_anode_data(idx: usize, name: &str) {
    let mut guard = SESSION.borrow_mut();
    if let Some(a) = guard.as_mut().and_then(|d| d.anodes.get_mut(idx)) {
        a.pdata.clear();
        a.pdata.push_str(name);
    }
}

/// `set_icon` (`deskins.c:1012-1022`) — put the chosen icon number into the
/// node at `idx`: `a_dicon` always, and `a_aicon` too when the data name is an
/// executable, "this marks it as an executable file".
///
/// EmuTOS records that in the `AF_ISEXEC` flag and lets `app_save` turn it into
/// the `#I` (executable) or `#N` (non-executable) tag (`deskapp.c:1230-1231`);
/// [`InfAnode`] carries no flag word, so the tag *is* the flag and is set here.
/// A folder node keeps its `#D` tag, which is what `app_save` writes for any
/// `AT_ISFOLD` node that is not a desktop shortcut (`:1238-1244`).
pub(super) fn set_icon(idx: usize, icon: i16) {
    let mut guard = SESSION.borrow_mut();
    let Some(a) = guard.as_mut().and_then(|d| d.anodes.get_mut(idx)) else {
        return;
    };
    a.aicon = NIL;
    a.dicon = icon;
    if anode_type(a.tag) == AType::File {
        a.tag = if super::ins::is_executable(a.pdata.as_bytes()) {
            a.aicon = icon;
            b'I'
        } else {
            b'N'
        };
        a.has_extra = true;
    }
}

/// `app_free` (`deskapp.c:227-247`) — unlink the node at `idx`.
pub(super) fn app_free(idx: usize) {
    let mut guard = SESSION.borrow_mut();
    if let Some(data) = guard.as_mut() {
        if idx < data.anodes.len() {
            data.anodes.remove(idx);
        }
    }
}

// ── Shared dialog open/close plumbing (`deskinf.c:345-402`) ─────────────────
//
// Every desktop dialog goes through the same three AES calls, and they are the
// reason a dialog neither lands under the menu bar nor has to repaint the whole
// screen when it goes away:
//
//   form_center(tree, &x,&y,&w,&h);            centre the ROOT, report the
//                                              outer rect (outline + shadow)
//   form_dial(FMD_START, 0,0,0,0, x,y,w,h);    reserve that rect
//   objc_draw(tree, ROOT, MAX_DEPTH, x,y,w,h); paint the dialog into it
//   ... form_do(tree, start) ...
//   form_dial(FMD_FINISH, 0,0,0,0, x,y,w,h);   release + repaint that rect
//   drain the redraw messages FMD_FINISH queued
//
// EmuTOS' `fm_dial` does **not** photograph the screen on FMD_START
// (`aes/gemfmlib.c:365-369` is a bare `break`): the restore is the FMD_FINISH
// repaint — `w_drawdesk(pt)` plus a `WM_REDRAW` to every window the rectangle
// touches (`:375-379`). Those messages are why `end_dialog` has to drain the
// pipe here instead of leaving them for the next `evnt_multi`: the desktop
// answers its own messages directly, so an undrained redraw would arrive out of
// sequence and show up as a partial refresh (`deskinf.c:371-378`, verbatim).

/// `start_dialog` (EmuTOS `deskinf.c:349-357`) — centre `tree`, constrain it to
/// the desktop rectangle (`rc_intersect(&gl_rfull, &t)`), reserve that area with
/// `form_dial(FMD_START)` and paint the tree into it. Returns the rectangle, to
/// be handed back to [`end_dialog`]-equivalent callers that keep it.
pub(super) fn start_dialog(tree: u64) -> Grect {
    let mut t = super::form_center(tree);
    // `rc_intersect(&gl_rfull, &t)` — `gl_rfull` is the screen minus the menu
    // row, i.e. exactly `G.g_desk` (`aes/gemgraf.c:423-424`, `deskmain.c:1959`).
    if let Some(clipped) = super::redraw::rc_intersect(super::g_desk(), t) {
        t = clipped;
    }
    super::form_dial_start(t);
    super::objc_draw_dialog(tree, t);
    t
}

/// `end_dialog` (EmuTOS `deskinf.c:363-390`) — release the dialog's rectangle
/// with `form_dial(FMD_FINISH)` (which repaints the desktop under it and fans a
/// `WM_REDRAW` out to every window it covered), then drain the messages that
/// just queued through the desktop's own `hndl_msg`, exactly as EmuTOS does and
/// for the reason its comment gives.
pub(super) fn end_dialog(tree: u64) {
    let t = super::form_center(tree);
    super::form_dial_finish(t);

    // `while (evnt_multi(MU_TIMER|MU_MESAG, …) & MU_MESAG) hndl_msg();`
    // The MU_TIMER with a zero timer is EmuTOS' "quick out": the call returns
    // immediately once no message is left.
    let mut msg = [0i16; 8];
    loop {
        let ev = super::evnt_multi_msg(MU_TIMER | MU_MESAG, &mut msg);
        if ev.which & MU_MESAG == 0 {
            break;
        }
        super::main::hndl_msg(&msg);
    }
}

/// `inf_show` (EmuTOS `deskinf.c:396-402`) — put a dialog up, run it, take it
/// down. Returns `form_do`'s exit object.
pub(super) fn inf_show(tree: u64, start: i16) -> i16 {
    start_dialog(tree);
    let exitobj = super::form_do(tree, start);
    end_dialog(tree);
    exitobj
}

// ── Dialog-tree primitives, shared by every `desk/` dialog built here ───────

// The `obdefs.h` object types, `ob_flags` and `ob_state` bits used below
// (`G_*`, `EXIT`, `RBUTTON`, `TOUCHEXIT`, `OUTLINED`, `FL3D*`, …) are
// `tos_types::aes` (DESK43 U5b).

// ── The EmuTOS 3D flag set (`obdefs.h:92-97`) ───────────────────────────────
//
// `desk_rsc.c` uses exactly three roles, and uses them the same way in every
// one of its twenty trees (the whole file was extracted object by object; the
// table is in `docs/api/desktop/deskinf.md`):
//
// | object                                         | ob_flags     |
// |------------------------------------------------|--------------|
// | ROOT `G_BOX` (with `OUTLINED` state)           | `FL3DBAK`    |
// | `G_FTEXT`/`G_FBOXTEXT` field                   | `FL3DBAK`    |
// | `G_BUTTON` with `RBUTTON` (± `EXIT`)           | `FL3DIND`    |
// | `G_BUTTON` with `EXIT` (OK/Cancel/Install/…)   | `FL3DACT`    |
// | `G_BUTTON` with `TOUCHEXIT` (arrow/stepper)    | `FL3DACT`    |
// | `G_STRING`, `G_TITLE`, `G_IBOX`, `G_IMAGE`     | none         |
//
// Two rows are worth naming because they are not what a guess would produce:
// a radio button that *also* carries `EXIT` is still an indicator
// (`ADBKGND`'s `BGDESK`/`BGWIN`, `desk_rsc.c:2254,2260`), and a stepper gadget
// is an activator (`ADINSDSK`'s `ID_UP`/`ID_DOWN`, `:990,996`).
//
// (`desk_rsc.c` spells `FL3DACT` as `FL3DBAK | FL3DIND`, which is the same
// 0x0600 bit pattern — `obdefs.h:93-97`.)

/// The dialog-box `ob_spec` of `desk_rsc.c`: a 1-pixel **inner** border over the
/// colour word `0x1100` — black border, black text, `IP_HOLLOW` pattern,
/// white interior (`gr_crack`). Used by every `desk_rsc.c` dialog ROOT bar
/// `ADDINFO`, which is two pixels thick (`desk_rsc.c:622,709,877,922,1027,
/// 1198,1237,1282,1315,1369,1570,1603,2230` all `69888L` = `(1<<16)|0x1100`).
///
/// **Hollow is load-bearing.** The AES only substitutes the 3D face colour when
/// the box is `IP_HOLLOW` *and* white (EmuTOS `just_draw`, `gemoblib.c:619-636`;
/// `tos-api/src/aes/object.rs` `takes_3d_face`). The previous NewTOS value
/// `(1<<16)|0x1070` was pattern 7 = `IP_SOLID`, so an [`FL3DBAK`] flag on it
/// would have been silently ignored and the box would have stayed flat white.
/// The sign is unchanged: a dialog box's border is *inner* (positive), unlike
/// the desktop's drop-down boxes, which A100 measured as outer (`0xFF1100`).
pub(super) const BOX_SPEC: u64 = (1 << 16) | 0x1100;

/// `TEDINFO.te_color` of every editable field in `desk_rsc.c`
/// (`desk_rs_tedinfo[0,18,24,25,…]`, all `4352` = `0x1100`): black border,
/// black text, hollow white interior — the same colour word as [`BOX_SPEC`],
/// so an [`FL3DBAK`] field takes the `backgrcol` face too
/// (`tos-api/src/aes/object.rs`, TEDINFO arm). The previous NewTOS value
/// `1 << 8` painted the frame in pen 0 (WHITE), which under `FL3DBAK` would
/// have drawn a white ring on the grey field.
pub(super) const TED_COLOR: i16 = 0x1100;

/// Build one dialog `OBJECT`.
#[allow(clippy::too_many_arguments)]
pub(super) fn mk(ty: u16, fl: u16, spec: u64, x: i16, y: i16, w: i16, h: i16, next: i16, head: i16, tail: i16) -> Object {
    Object {
        ob_next: next,
        ob_head: head,
        ob_tail: tail,
        ob_type: ty,
        ob_flags: fl,
        ob_state: 0,
        ob_spec: spec,
        ob_x: x,
        ob_y: y,
        ob_width: w,
        ob_height: h,
    }
}

/// Build the ROOT box of a desktop dialog exactly as `desk_rsc.c` declares one:
/// a `G_BOX` at the origin (`form_center` writes the centred position), with
/// [`FL3DBAK`] flags, [`OUTLINED`] state and a `(thickness << 16) | 0x1100`
/// `ob_spec` — the three pieces that together give EmuTOS's raised grey panel.
/// `spec` is normally [`BOX_SPEC`]; `ADDINFO` is the one tree with a 2-pixel
/// border (`desk_rsc.c:774-777`).
pub(super) fn mk_root(spec: u64, w: i16, h: i16, tail: i16) -> Object {
    let mut root = mk(G_BOX, FL3DBAK, spec, 0, 0, w, h, -1, 1, tail);
    root.ob_state = OUTLINED;
    root
}

/// One NUL-terminated `G_STRING` line held on the dialog's own stack frame.
///
/// A `G_STRING`'s `ob_spec` is a C string pointer, and the AES draws it
/// **left-aligned at `ob_x`**, vertically centred in `ob_h`
/// (`aes/gemoblib.c:817-845`, reproduced in `tos-api/src/aes/object.rs`'s
/// `G_STRING` arm) — there is no centring attribute to set, so a line that must
/// appear centred has to be *placed* centred, which is what [`Line::width`] is
/// for. EmuTOS's resource strings are `.rsc` literals with the NUL already in
/// them; a Rust `&str` has none, hence the copy.
struct Line {
    buf: [u8; LINE_MAX + 1],
    len: usize,
}

/// Longest dialog line, in characters (72 · 8 px = 576, still inside a 640-px
/// screen with margins).
const LINE_MAX: usize = 72;

impl Line {
    /// Copy `s` (truncated to [`LINE_MAX`]) into a NUL-terminated buffer.
    fn new(s: &str) -> Self {
        let mut buf = [0u8; LINE_MAX + 1];
        let src = s.as_bytes();
        let len = src.len().min(LINE_MAX);
        buf[..len].copy_from_slice(&src[..len]);
        Self { buf, len }
    }
    /// The `ob_spec` value for a `G_STRING` showing this line.
    fn spec(&self) -> u64 {
        self.buf.as_ptr() as u64
    }
    /// The drawn width in pixels (the AES normal font is fixed-pitch).
    fn width(&self) -> i16 {
        self.len as i16 * gl_wchar()
    }
}

// ── Desktop-info dialog (`do_deskmenu`'s `ABOUITEM`, `deskmain.c:612-646`) ──

/// `do_aboutdesk` — the Desk menu's *Desktop info...* box.
///
/// EmuTOS shows the `ADDINFO` resource tree (`desk/desk_rsc.c:772-873`): an
/// `OUTLINED` root box 40 characters wide, a `- EmuTOS -` title flanked by two
/// `G_IMAGE` logos, a `Version` label with the value `desk_xlate_fix` patched in
/// from the BIOS `version[]` string (`deskmain.c:1880`), the copyright block,
/// the project URL and the GPL notice, and a single `DEOK` button
/// (`desk_rsc.c:869-873`). The **structure** is reproduced here — centred box,
/// title line, a `Version`/`Machine` pair block, the credit paragraphs, one
/// default OK button.
///
/// ## The content, and where each line comes from
///
/// | row | line | source |
/// |-----|------|--------|
/// | 0 | `- NewTOS -` | `ADDINFO`'s `- EmuTOS -` title (`desk_rsc.c:785-789`) |
/// | 2 | `Version  …` | [`VERSION_STRING`] (`DELABEL`/`DEVERSN`) |
/// | 3 | `Machine  …` | [`MACHINE_NAME`] — a per-board build constant (X03) |
/// | 5-6 | the independence notice | [`INDEPENDENCE_NOTICE`] |
/// | 8-9 | the EmuTOS credit | [`CREDIT_REIMPL`] / [`CREDIT_EMUTOS`] |
/// | 11-14 | the `'GPLed'` chain | [`CREDIT_GPL`] / [`CREDIT_DRI`] / [`CREDIT_CALDERA`] / [`CREDIT_LINEO`] |
///
/// The blank rows 1/4/7/10 are `ADDINFO`'s own: it leaves char rows 3, 6, 8
/// and 13 empty between its blocks (`desk_rsc.c:790-866`); row 15 is the gap
/// above the OK button (`ADDINFO`'s char row 16, `:869-873`).
///
/// Until 2026-10-09 a row 16 carried a NewTOS authorship line with no
/// `ADDINFO` counterpart; it was dropped (user decision), which brings the
/// content closer to `ADDINFO`'s.
///
/// **Two readers, two copies — on purpose, since DESK43 U4 (the machine line
/// since X03).** The version, the machine line and the independence notice
/// are also printed by the boot screen. They *were* read from `kernel::initinfo`, on the argument that
/// EmuTOS shares one source (`desk_xlate_fix` points `DEVERSN->ob_spec` at the
/// BIOS `version[]` the boot screen prints, `deskmain.c:1880`). That argument
/// holds only while the desktop is linked with the BIOS, which is EmuTOS's
/// situation and stops being ours: the desktop crate will not have `kernel` in
/// its dependency graph. The strings are therefore the desktop's own
/// (see the block at [`VERSION_STRING`]), and `initinfo`'s copies say the same
/// at their definition. The six credit lines were always local, and for a
/// reason that is unchanged — EmuTOS keeps its `'GPLed'` chain in
/// `desk_rsc.c` alone and `bios/initinfo.c` prints none of it.
///
/// **Three named deviations.**
///
/// * EmuTOS runs `start_dialog` → `form_do` loop → `end_dialog` rather than
///   [`inf_show`], solely because `DEICON` is a `TOUCHEXIT` that plays the
///   Easter-egg tune and loops instead of closing (`deskmain.c:626-641`,
///   `CONF_WITH_EASTER_EGG`). With no such object the loop would run exactly
///   once, so this is `inf_show(tree, ROOT)` — the same three calls.
/// * The box is sized to its text instead of the resource's fixed 40 columns:
///   the machine line differs per board build ([`MACHINE_NAME`]), so there is
///   no single width to bake into the tree. EmuTOS solves the same problem for its
///   menus with the run-time `adjust_menu` (`deskmain.c:1608-1635`).
/// * `ADDINFO`'s copyright year is patched in at start-up from
///   `COPYRIGHT_YEAR` (`deskmain.c:1892`, `obj/header.h:19`, generated by
///   `tools/mkheader.awk`). The span in [`CREDIT_EMUTOS`] stays a literal even
///   though NewTOS now has a build year (`tos_api::build::BUILD_YEAR`):
///   EmuTOS's year comes from *its* build, and ours is pinned source, so
///   stamping our build year onto their line would extend their copyright by
///   the calendar.
///
/// **On the build date: this box shows none, and neither does EmuTOS's.**
/// `ADDINFO` carries `DECOPYRT`, patched from `COPYRIGHT_YEAR` — a *year*, not
/// a build date. EmuTOS's build date goes into the OS header alone
/// (`tools/mkheader.awk` generates `obj/header.h`'s `OS_DATE`/`OS_DOSDATE`),
/// and its boot screen prints only `EmuTOS Version %s` (`bios/initinfo.c:321`).
/// NewTOS now has that stamp too (`tos_api::build`, DESK12): it feeds the
/// OSHEADER the same way, and the boot screen prints it as an own addition.
/// It is not repeated here, because it has no EmuTOS counterpart. (The box is
/// 396 px tall since the authorship row went on 2026-10-09; it was 436 px,
/// with 10 px to spare at 640x480, before.)
///
pub(super) fn do_aboutdesk() {
    // Column layout: a `label   value` pair block, as `ADDINFO` pairs DELABEL
    // with DEVERSN (`desk_rsc.c:803-813`).
    const LABEL_COLS: i16 = 9;
    const MARGIN: i16 = 24;
    let row_pitch = gl_hchar() + 4;

    let title = Line::new("- NewTOS -");
    let ver_lbl = Line::new("Version");
    let ver_val = Line::new(VERSION_STRING);
    let mac_lbl = Line::new("Machine");
    // The per-board build constant (X03); see `MACHINE_NAME` for why it is
    // not a syscall and not a kernel symbol.
    let mac_val = Line::new(MACHINE_NAME);
    // The independence notice is one sentence; split it at its own dash so the
    // box stays close to the resource's proportions instead of growing to the
    // full 63-character line.
    let notice = INDEPENDENCE_NOTICE;
    let (head, tail) = match notice.find("- ") {
        Some(i) => (&notice[..i + 1], &notice[i + 2..]),
        None => (notice, ""),
    };
    let note1 = Line::new(head);
    let note2 = Line::new(tail);
    let reimpl = Line::new(CREDIT_REIMPL);
    let emuteam = Line::new(CREDIT_EMUTOS);
    let gpl = Line::new(CREDIT_GPL);
    let dri = Line::new(CREDIT_DRI);
    let caldera = Line::new(CREDIT_CALDERA);
    let lineo = Line::new(CREDIT_LINEO);

    static OK: &[u8] = b" OK \0";
    const OK_W: i16 = 90;
    const OK_H: i16 = 28;

    // Pair block: label column + the wider of the two values.
    let pair_w = LABEL_COLS * gl_wchar() + ver_val.width().max(mac_val.width());
    let text_w = pair_w
        .max(title.width())
        .max(note1.width())
        .max(note2.width())
        .max(reimpl.width())
        .max(emuteam.width())
        .max(gpl.width())
        .max(dri.width())
        .max(caldera.width())
        .max(lineo.width());
    let dw = text_w + 2 * MARGIN;
    let pair_x = (dw - pair_w) / 2;
    let val_x = pair_x + LABEL_COLS * gl_wchar();
    let ctr = |l: &Line| (dw - l.width()) / 2;

    // Row grid, with `ADDINFO`'s blank rows at 1, 4, 7 and 10; row 15 stays
    // empty above the button.
    let y0 = MARGIN;
    let row = |n: i16| y0 + n * row_pitch;
    let y_ok = row(16);
    let dh = y_ok + OK_H + MARGIN;

    /// Object index of the OK button — `ADDINFO`'s `DEOK` (`desk_rsc.h:91`);
    /// ours is 14, not 16, because this tree has no `G_IMAGE` logos.
    const DEOK: i16 = 14;
    let str_obj = |l: &Line, x: i16, n: i16, next: i16| mk(G_STRING, 0, l.spec(), x, row(n), l.width(), gl_hchar(), next, -1, -1);
    let mut tree = [
        mk_root(ADDINFO_BOX_SPEC, dw, dh, DEOK),
        str_obj(&title, ctr(&title), 0, 2),
        str_obj(&ver_lbl, pair_x, 2, 3),
        str_obj(&ver_val, val_x, 2, 4),
        str_obj(&mac_lbl, pair_x, 3, 5),
        str_obj(&mac_val, val_x, 3, 6),
        str_obj(&note1, ctr(&note1), 5, 7),
        str_obj(&note2, ctr(&note2), 6, 8),
        str_obj(&reimpl, ctr(&reimpl), 8, 9),
        str_obj(&emuteam, ctr(&emuteam), 9, 10),
        str_obj(&gpl, ctr(&gpl), 11, 11),
        str_obj(&dri, ctr(&dri), 12, 12),
        str_obj(&caldera, ctr(&caldera), 13, 13),
        str_obj(&lineo, ctr(&lineo), 14, DEOK),
        // `DEOK`: `SELECTABLE | DEFAULT | EXIT | LASTOB | FL3DBAK | FL3DIND`
        // (`desk_rsc.c:869-873`) — the last two bits are `FL3DACT`.
        mk(
            G_BUTTON,
            SELECTABLE | DEFAULT | EXIT | LASTOB | FL3DACT,
            OK.as_ptr() as u64,
            (dw - OK_W) / 2,
            y_ok,
            OK_W,
            OK_H,
            0,
            -1,
            -1,
        ),
    ];

    // `inf_show(tree, ROOT)` (`deskinf.c:396-402`). `as_mut_ptr`, not `as_ptr`:
    // the AES writes `ob_state` through this pointer while the dialog runs.
    let clicked = inf_show(tree.as_mut_ptr() as u64, ROOT);
    // `obj = tree + DEOK; obj->ob_state = NORMAL;` (`deskmain.c:643-644`) — the
    // resource tree is reused on the next open, so the pressed button must not
    // stay inverted. Ours is rebuilt each time; done anyway, for the same reason
    // `set_background` does it.
    if let Some(o) = tree.get_mut(clicked.max(0) as usize) {
        o.ob_state &= !SELECTED;
    }
}

/// `ADDINFO`'s ROOT `ob_spec` — `135424L` = `(2 << 16) | 0x1100`
/// (`desk_rsc.c:774-777`): the same colour word as [`BOX_SPEC`], but a
/// **two**-pixel inner border. `ADDINFO` is the only `desk_rsc.c` tree with it.
const ADDINFO_BOX_SPEC: u64 = (2 << 16) | 0x1100;

// ── The About box's text ────────────────────────────────────────────────────
//
// **All of it is the desktop's own** (DESK43 U4, 2026-09-26). EmuTOS's
// `ADDINFO` carries every line as a literal in `desk_rsc.c:772-873` except
// `DEVERSN`, whose `ob_spec` `desk_xlate_fix` re-points at the BIOS `version[]`
// symbol (`deskmain.c:1880`) — a shared *symbol*, which costs EmuTOS nothing
// because its desktop is linked into the same ROM as its BIOS. Ours will not
// be: `programs/desktop` has neither `kernel` nor `tos-api` in its dependency
// graph (`docs/analysis/desktop-userspace-plan-v2.md` §6.4, measure 1), so a
// `kernel::initinfo::…` here is a link the move has to break, and breaking it
// means the string lives on this side. `kernel::initinfo`'s own copies stay for
// the boot screen and say so at their definition.

/// The NewTOS version, as the About box's `Version` row prints it —
/// `ADDINFO`'s `DEVERSN` (`desk_rsc.c:808-813`, patched at `deskmain.c:1880`).
///
/// **Deliberately the same text as `kernel::initinfo::VERSION_STRING`, and
/// deliberately a second copy of it.** See the block comment above for why the
/// shared symbol cannot survive the move. The `NTOS` cookie carries a
/// *number* (`0x0000_0100`, `tos-api/src/cookie.rs:142`) with no defined
/// rendering, and `Sversion` reports the GEMDOS version, not the OS name.
/// Since SYSINFO01, `Ssystem(S_NEWTOS_KVERSION)` (2019,
/// `docs/api/gemdos/154-ssystem.md`) hands programs this string followed by
/// the build stamp; this desktop does not ask for it (it is compiled with the
/// kernel), a separately built one could.
const VERSION_STRING: &str = "NewTOS 0.6.1";

/// The project's independence notice — the NewTOS counterpart of the
/// GPL/attribution block `ADDINFO` puts below its copyright
/// (`desk_rsc.c:810-868`). Same text as
/// `kernel::initinfo::INDEPENDENCE_NOTICE`, same reason as above.
const INDEPENDENCE_NOTICE: &str =
    "NewTOS is an independent project - not original Atari software.";

/// The machine the desktop was built for, as the About box's `Machine` row
/// prints it — a NewTOS addition; `ADDINFO` has no Machine line
/// (`desk_rsc.c:772-873`).
///
/// **A per-board build constant** (X03, user decision 2026-10-01): the text is
/// fixed when the desktop is compiled for a board feature (`cargo xtask
/// build-programs --board`, which the kernel build runs for its own board,
/// DESK43 U6b/U9), not asked for at run time. Same texts as the boot screen's `kernel::initinfo::machine_name`,
/// deliberately a second copy, for the reason in the block comment above.
/// There is no syscall source for it: `_MCH` is one fixed value,
/// `0xFFFF_FFFF` (EmuTOS `MCH_NOHARD`, "no Atari hardware", ADR-012
/// amendment 2026-10-28) on **both** boards (`tos_api::cookie::MCH_NOHARD`,
/// `docs/api/bios/_cookie-jar.md`), so a cookie read cannot tell the Pi
/// from QEMU; a board cookie or an `Ssystem` subfunction was considered and
/// declined (`TASK/X03.md`).
///
/// **Limit, accepted in X03:** this holds because the program is built per
/// board and embedded in that board's kernel image (DESK43 D3). A `NEWDESK`
/// started from a card built for the other board (`#Z`) would show that
/// board's line; the cookie-or-`Ssystem` question returns if that matters.
///
/// **A board without a line here is a compile error, not a fallback.** The
/// boot screen's `other => other` arm prints the raw board identifier, which
/// the desktop cannot see (it has no kernel symbol to read). Any text chosen
/// here instead would be the "one line that claims the wrong machine" the
/// ticket rules out, so a new board feature must add its own line.
#[cfg(feature = "board-rpi4")]
const MACHINE_NAME: &str = "Raspberry Pi 4 (BCM2711)";
#[cfg(feature = "board-qemu-virt")]
const MACHINE_NAME: &str = "QEMU aarch64-virt";
#[cfg(not(any(feature = "board-rpi4", feature = "board-qemu-virt")))]
compile_error!(
    "desktop::inf: no About-box machine line for this board feature; add a \
     `MACHINE_NAME` for it next to the existing ones (TASK/X03)."
);

// The credit lines are local to this dialog for a second, older reason:
// EmuTOS keeps the same split — `bios/initinfo.c` prints none of the `'GPLed'`
// chain, only `desk_rsc.c`'s `ADDINFO` carries it.

/// What NewTOS is, in one line. EmuTOS's `ADDINFO` has no counterpart claim —
/// it *is* the code it credits; NewTOS is a reimplementation of it, which is
/// the fact this line states.
const CREDIT_REIMPL: &str = "Reimplemented from the EmuTOS sources";
/// EmuTOS's own copyright line, as `ADDINFO` assembles it from
/// `"Copyright \275 2001-"` + `COPYRIGHT_YEAR` + `"The EmuTOS development team"`
/// (`desk_rsc.c:806-828`, `deskmain.c:1892`). `\275` is the Atari ST charset's
/// `(c)` glyph, written out here. The year span stays a literal even though
/// NewTOS now has a build year (`tos_api::build::BUILD_YEAR`): EmuTOS's year
/// comes from *its* build, ours is a pinned source tree, and stamping our build
/// year onto their line would extend their copyright by the calendar.
const CREDIT_EMUTOS: &str = "(c) 2001-2026 The EmuTOS development team";
/// `ADDINFO`'s `"Based on 'GPLed' sources"` heading, verbatim
/// (`desk_rsc.c:834-838`).
const CREDIT_GPL: &str = "Based on 'GPLed' sources";
/// `ADDINFO`'s `"\275 1987 Digital Research, Inc."` (`desk_rsc.c:840-844`).
const CREDIT_DRI: &str = "(c) 1987 Digital Research, Inc.";
/// `ADDINFO`'s `"\275 1999 Caldera Thin Clients, Inc."` (`desk_rsc.c:846-850`).
const CREDIT_CALDERA: &str = "(c) 1999 Caldera Thin Clients, Inc.";
/// `ADDINFO`'s `"\275 2001 Lineo, Inc."` (`desk_rsc.c:852-856`).
const CREDIT_LINEO: &str = "(c) 2001 Lineo, Inc.";

/// Append a decimal rendering of `v` to `buf` at `*p` (bounded by `buf.len()`).
fn push_dec(buf: &mut [u8], p: &mut usize, mut v: u64) {
    if v == 0 {
        if *p < buf.len() {
            buf[*p] = b'0';
            *p += 1;
        }
        return;
    }
    let mut tmp = [0u8; 20];
    let mut i = 20;
    while v > 0 {
        i -= 1;
        tmp[i] = b'0' + (v % 10) as u8;
        v /= 10;
    }
    for &d in &tmp[i..] {
        if *p < buf.len() {
            buf[*p] = d;
            *p += 1;
        }
    }
}

/// Append the bytes of `s` to `buf` at `*p`.
fn push_str(buf: &mut [u8], p: &mut usize, s: &[u8]) {
    for &b in s {
        if *p < buf.len() {
            buf[*p] = b;
            *p += 1;
        }
    }
}

/// Append `v` as two zero-padded decimal digits to `buf`.
fn push_dec2(buf: &mut [u8], p: &mut usize, v: u32) {
    if *p + 1 < buf.len() {
        buf[*p] = b'0' + (v / 10 % 10) as u8;
        buf[*p + 1] = b'0' + (v % 10) as u8;
        *p += 2;
    }
}

/// Append `v` with C's `%02d` semantics — zero-padded to **at least** two
/// digits, but never truncated. `fmt_date` formats all three of its fields with
/// `%02d` (`deskinf.c:177`), which is why a four-digit year prints in full even
/// though the field is nominally two wide.
fn push_dec02(buf: &mut [u8], p: &mut usize, v: u64) {
    if v < 10 {
        push_str(buf, p, b"0");
    }
    push_dec(buf, p, v);
}

// ── Date / time formatting (`fmt_date`/`fmt_time`, `deskinf.c:74-180`) ──────
//
// Both readers of the date/time preference live here, as they do in EmuTOS:
// the text-view line builder (`format_fnode`, `deskinf.c:220-307`, ours in
// `super::win`) and the Show-Info box (`inf_dttm`, `deskinf.c:460-468`).
// Neither takes the desktop lock — the caller passes the format *and* the
// current `_IDT` word in, because `format_fnode` already runs inside
// `global::with`.
//
// The separator is **not** a parameter (it was, until DESK13): EmuTOS starts
// from `DATEFORM_SEP` ('/') and only the `DATEFORM_IDT` arm overrides it, from
// the cookie (`deskinf.c:129,141-143`). NewTOS's text view printed `MM-DD-YY`
// with a dash while the Show-Info box printed `MM/DD/YYYY` — one rule, two
// answers. `DateFmt::rendered` is now that rule's single home.

/// `fmt_date` (`deskinf.c:124-180`) — a DOS-packed date word written into `out`
/// as `xx<sep>xx<sep>xx`, with a four-digit year when `fourdigit`. Field order
/// and separator both come from [`DateFmt::rendered`], resolved against the
/// current `_IDT` word `idt` ([`super::global::DeskState::g_idt`]). Returns the
/// number of bytes written.
pub(super) fn fmt_date(date: u16, fourdigit: bool, datefmt: DateFmt, idt: u32, out: &mut [u8]) -> usize {
    let mut year = 1980u32 + ((date >> 9) & 0x7F) as u32;
    if !fourdigit {
        year %= 100;
    }
    let month = ((date >> 5) & 0x0F) as u32;
    let day = (date & 0x1F) as u32;

    let (order, sep) = datefmt.rendered(idt);
    // `switch(format)` (`deskinf.c:154-176`) — which of the three fields goes
    // where. The year is a whole field, so a year-first order prints it first.
    let (v1, v2, v3) = match order {
        DateOrder::Mdy => (month, day, year),
        DateOrder::Ymd => (year, month, day),
        DateOrder::Ydm => (year, day, month),
        DateOrder::Dmy => (day, month, year),
    };

    // `sprintf(pdate, "%02d%c%02d%c%02d", var1, separator, var2, separator,
    //  var3)` (`:177`).
    let mut p = 0usize;
    push_dec02(out, &mut p, v1 as u64);
    push_str(out, &mut p, &[sep]);
    push_dec02(out, &mut p, v2 as u64);
    push_str(out, &mut p, &[sep]);
    push_dec02(out, &mut p, v3 as u64);
    p
}

/// `fmt_time` (`deskinf.c:74-105`) — a DOS-packed time word written into `out`
/// as `hh<sep>mm` plus a **two-character suffix**: `am`/`pm` for
/// [`TimeFmt::H12`], two blanks for [`TimeFmt::H24`] (EmuTOS's
/// `char *suffix = "  ";`, `deskinf.c:77`). The suffix keeps the field width
/// constant whichever format is chosen, which is why EmuTOS can lay the text
/// view out in fixed columns. Returns the number of bytes written.
///
/// `sep` is the whole of what EmuTOS varies between its call sites through the
/// `sprintf` format string it passes in: the text view uses `"%02d:%02d%s"`
/// (`:293`) and `inf_dttm` `"%02d%02d%s"` (`:467`), because `ADFFINFO`'s
/// `Time:   __:__ __  ` template already carries the colon.
///
/// [`TimeFmt::Idt`] is resolved against `idt` by [`TimeFmt::rendered`].
pub(super) fn fmt_time(time: u16, timefmt: TimeFmt, idt: u32, sep: &[u8], out: &mut [u8]) -> usize {
    let mut hh = ((time >> 11) & 0x1F) as u32;
    let mm = ((time >> 5) & 0x3F) as u32;
    let suffix: &[u8] = match timefmt.rendered(idt) {
        TimeFmt::H12 => {
            let pm = hh >= 12;
            if pm {
                hh -= 12;
            }
            if hh == 0 {
                hh = 12; // `if (hh == 0) hh = 12;` (`deskinf.c:99-100`)
            }
            if pm {
                b"pm"
            } else {
                b"am"
            }
        }
        _ => b"  ",
    };
    let mut p = 0usize;
    push_dec2(out, &mut p, hh);
    push_str(out, &mut p, sep);
    push_dec2(out, &mut p, mm);
    push_str(out, &mut p, suffix);
    p
}

/// FAT attribute bits (`bdosdefs.h`).
const FA_RO: u8 = 0x01;
const FA_SUBDIR: u8 = 0x10;

/// `LEN_ZFNAME` (`include/sysconf.h:52`) — a NUL-terminated 8.3 name, and the
/// buffer size both [`fmt_str`] and [`unfmt_str`] write into (`fmt_str` emits
/// at most 11 bytes, `unfmt_str` at most 12).
const LEN_ZFNAME: usize = 13;

// ── `util/optimize.c` name/number helpers of the two Show-Info boxes ────────

/// `fmt_str` (`util/optimize.c:43-70`) — a normal file name laid out for an
/// `FFFFFFFF.FFF` TEDINFO template: up to eight bytes before the **first** dot,
/// then, if anything follows it, blanks out to column 8 and up to three more
/// bytes. `SAMPLE.PRG` → `SAMPLE  PRG`, `TESTPROG.C` → `TESTPROGC`, `TEST` →
/// `TEST`. Excess bytes are eaten, which is how the source copes with names a
/// GEMDOS emulation hands it that are not 8.3 at all.
///
/// Returns the number of bytes written; `out` is always NUL-terminated.
fn fmt_str(instr: &[u8], out: &mut [u8; LEN_ZFNAME]) -> usize {
    out.fill(0);
    let mut q = 0usize;
    let mut i = 0usize;
    while i < instr.len() {
        let c = instr[i];
        i += 1;
        if c == b'.' {
            break;
        }
        if q < 8 {
            out[q] = c;
            q += 1;
        }
    }
    // `if (*p)` — there is something after the dot.
    if i < instr.len() {
        while q < 8 {
            out[q] = b' ';
            q += 1;
        }
        while q < 11 && i < instr.len() {
            out[q] = instr[i];
            q += 1;
            i += 1;
        }
    }
    q
}

/// `unfmt_str` (`util/optimize.c:77-93`) — the reverse of [`fmt_str`]:
/// `SAMPLE  PRG` → `SAMPLE.PRG`. The blanks inside the first eight columns are
/// dropped and a dot is re-inserted before whatever follows them.
///
/// Returns the number of bytes written; `out` is always NUL-terminated.
fn unfmt_str(instr: &[u8], out: &mut [u8; LEN_ZFNAME]) -> usize {
    out.fill(0);
    let mut o = 0usize;
    let mut i = 0usize;
    while i < instr.len() && i < 8 {
        let c = instr[i];
        i += 1;
        if c != b' ' {
            out[o] = c;
            o += 1;
        }
    }
    if i < instr.len() {
        out[o] = b'.';
        o += 1;
        while i < instr.len() && o < LEN_ZFNAME - 1 {
            out[o] = instr[i];
            o += 1;
            i += 1;
        }
    }
    o
}

/// `set_tedinfo_name` (`deskinf.c:424-430`) — `fmt_str` into a scratch buffer,
/// then [`inf_sset`] into the field's text.
fn set_tedinfo_name(buf: &mut [u8], name: &[u8]) {
    let mut temp = [0u8; LEN_ZFNAME];
    let n = fmt_str(name, &mut temp);
    inf_sset(buf, &temp[..n]);
}

/// `inf_numset` (`deskinf.c:409-419`) — `sprintf(te_ptext, "%*lu",
/// te_txtlen-1, value)`: the number right-aligned in the field's capacity,
/// blank-padded on the left. `buf.len()` **is** `te_txtlen`.
///
/// A value with more digits than the field holds is written by its **last**
/// `te_txtlen - 1` digits. EmuTOS's `sprintf` writes the whole number and
/// overruns the field into the next one in `desk_rs_te_text[]`; what the user
/// then *sees* is the same, because a `TE_RIGHT` `ob_format` fills the
/// template's underscores from the right end of the text. Unreachable for
/// either caller on a FAT16 volume (10 digits cover 9 999 999 999 bytes, five
/// cover 99 999 files).
fn inf_numset(buf: &mut [u8], value: u64) {
    let len = buf.len() - 1; // `te_txtlen - 1`
    let mut digits = [0u8; 20];
    let mut n = 0usize;
    push_dec(&mut digits, &mut n, value);
    buf.fill(0);
    if n >= len {
        buf[..len].copy_from_slice(&digits[n - len..n]);
    } else {
        buf[..len - n].fill(b' ');
        buf[len - n..len].copy_from_slice(&digits[..n]);
    }
}

/// `inf_fifosz` (`deskinf.c:437-456`) — put a walk's file, folder and byte
/// counts into three fields.
///
/// **`G.g_ndirs--`** (`:448`, "reproduce TOS's folder count in this
/// situation"): [`super::dir::count_ffs`] counts the directory it was started
/// on as well, and neither the drive root nor the folder whose box this is, is
/// a folder the user put *inside* it.
///
/// The source also clears `DISABLED` on all three objects; here that is
/// implicit, because both trees are built fresh with `NORMAL` states and only
/// [`inf_file_folder`]'s plain-file branch ever sets the bit.
fn inf_fifosz(count: &super::dir::DirCount, b_fi: &mut [u8], b_fo: &mut [u8], b_sz: &mut [u8]) {
    inf_numset(b_fi, count.files as u64);
    inf_numset(b_fo, count.dirs.saturating_sub(1) as u64);
    inf_numset(b_sz, count.size);
}

/// Build one of the two Show-Info boxes' `TEDINFO`s.
///
/// Every field of `ADFFINFO` and `ADDISKIN` shares `te_font = IBM`,
/// `te_color = 4352` ([`TED_COLOR`]) and `te_thickness = 0`
/// (`desk_rs_tedinfo[0..11]`, `desk_rsc.c:34-93`), so only the text buffer, the
/// template, the validation string and the justification vary. `te_thickness`
/// of `0` is why these fields carry no frame of their own — the underscores of
/// the template are the whole of the "field" the user sees.
///
/// `tmpl` carries its own NUL, which makes `tmpl.len()` exactly `te_tmplen`;
/// `text.len()` is exactly `te_txtlen`.
fn rsc_ted(text: &mut [u8], tmpl: &[u8], valid: &[u8], just: i16) -> TedInfo {
    TedInfo {
        te_ptext: text.as_mut_ptr() as u64,
        te_ptmplt: tmpl.as_ptr() as u64,
        te_pvalid: valid.as_ptr() as u64,
        te_font: 3, // IBM (`obdefs.h:62`)
        te_junk1: 0,
        te_just: just,
        te_color: TED_COLOR,
        te_junk2: 0,
        te_thickness: 0,
        te_txtlen: text.len() as i16,
        te_tmplen: tmpl.len() as i16,
    }
}

/// `align_title` (`deskmain.c:1731-1747`) — centre object 1 on the box when it
/// is the tree's title: a `G_STRING` exactly one character row down. EmuTOS
/// runs this over every tree at start-up (`:1995`) and again whenever a title
/// changes, which is why [`inf_file_folder`] calls it after picking between
/// `STFIINFO` and `STFOINFO`.
///
/// `title_chars` is `strlen(title->ob_spec)`; it is passed in because our
/// `ob_spec` is a raw pointer. Note that EmuTOS moves `ob_x` and leaves
/// `ob_width` alone — a `G_STRING` is drawn from `ob_x` with no clamping
/// (`gemoblib.c:817-845`), so a one-cell-wide title object still shows in full.
///
/// This is the **classic-TOS arm**. `CONF_WITH_ALT_DESKTOP_GRAPHICS` (1 in the
/// reference build) left-aligns the title and underlines it instead; every
/// NewTOS dialog centres, and closing that gap for all of them at once is
/// `TASK/GUI05.md`.
fn align_title(tree: &mut [Object], title_chars: i16) {
    let root_w = tree[0].ob_width;
    let title = &mut tree[1];
    if title.ob_type == G_STRING && title.ob_y == gl_hchar() {
        let len = (title_chars * gl_wchar()).min(root_w);
        title.ob_x = (root_w - len) / 2;
    }
}

/// `inf_disk` (EmuTOS `deskinf.c:650-693`) — the `ADDISKIN` box: how much of a
/// drive is in use, in folders, files and bytes.
///
/// ```c
/// if (!valid_drive(dr_id)) return 1;
/// desk_busy_on();
/// srcpth = "X:\\*.*"; more = count_ffs(srcpth);
/// if (!more) { desk_busy_off(); return 1; }
/// dos_space(dr_id - 'A' + 1, &total, &avail);
/// desk_busy_off();
/// if (!dos_label(dr_id - 'A' + 1, label)) label[0] = '\0';
/// inf_sset(tree, DIDRIVE, drive);
/// set_tedinfo_name(tree, DIVOLUME, label);
/// inf_fifosz(tree, DINFILES, DINFOLDS, DIUSED);
/// inf_numset(tree, DIAVAIL, avail);
/// inf_show(tree, ROOT);
/// return inf_what(tree, DIOK);
/// ```
///
/// Returns EmuTOS's `1` (continue) / `0` (cancel), which is what
/// [`super::main`]'s `SHOWITEM` loop tests: *Cancel* stops it walking the rest
/// of the selection.
///
/// Three details of the source that are easy to lose:
///
/// * **`total` is read and not shown.** `dos_space` yields both figures, the
///   dialog has a field only for `avail`, and the used figure comes from
///   `inf_fifosz`'s `G.g_size` — the bytes the *walk* added up, not
///   `total - avail`. A drive whose clusters are partly wasted on slack
///   therefore shows `used + available < total`, and that is TOS's arithmetic,
///   not a rounding error here.
/// * **`inf_fifosz` decrements the folder count** ([`inf_fifosz`]).
/// * **A missing drive is not silent.** `valid_drive` puts up `STNODRIV`
///   (`desksupp.c:1822`) and `inf_disk` then returns `1`, so the loop carries
///   on to the next icon.
///
/// **The tree is `ADDISKIN` object for object** (`desk_rsc.c:709-768`,
/// `desk_rsc.h:75-84`) since DESK42: ten objects, its numbering, its labels,
/// its six `G_FBOXTEXT` template fields, its measurements resolved through
/// `rsrc_obfix` at 8 x 16. It used to be seven hand-laid `G_STRING` lines in a
/// 340 x 230 box, which is what the user saw as "nothing in common with
/// EmuTOS's".
pub(super) fn inf_disk(dr_id: u8) -> i16 {
    // ── object numbers (`desk_rsc.h:75-84`) ──────────────────────────────
    //   0 ROOT · 1 DITITLE · 2 DIDRIVE · 3 DIVOLUME · 4 DINFOLDS
    //   5 DINFILES · 6 DIUSED · 7 DIAVAIL · 8 DIOK · 9 DICNCL
    const DIDRIVE: i16 = 2;
    const DIVOLUME: i16 = 3;
    const DINFOLDS: i16 = 4;
    const DINFILES: i16 = 5;
    const DIUSED: i16 = 6;
    const DIAVAIL: i16 = 7;
    const DIOK: usize = 8;
    const DICNCL: i16 = 9;

    if !super::main::valid_drive(dr_id) {
        return 1;
    }

    super::pro::desk_busy_on();
    // `srcpth = "X:\\*.*"` — NewTOS's walker takes the directory and appends
    // its own wildcard.
    let mut root = String::new();
    root.push(dr_id as char);
    root.push_str(":\\");
    let count = super::dir::count_ffs(&root);
    let Some(count) = count else {
        super::pro::desk_busy_off();
        return 1; // `if (!more) { desk_busy_off(); return 1; }`
    };
    let drv = (dr_id - b'A' + 1) as i16; // `dos_space`/`dos_label` are 1-based
    let (_total, avail) = newtos_gem::gemdos::dos_space(drv);
    super::pro::desk_busy_off();
    let label = newtos_gem::gemdos::dos_label(drv).unwrap_or_default();

    // ── strings: `ADDISKIN`'s own (`desk_rsc.c:711-768`) and the templates of
    //    `desk_rs_tedinfo[6..11]` (`:76-93`), character for character.
    static TITLE: &[u8] = b"DISK INFORMATION\0";
    static OK: &[u8] = b"OK\0";
    static CANCEL: &[u8] = b"Cancel\0";
    static T_DRIVE: &[u8] = b"Drive identifier:  _           \0";
    static T_VOLUME: &[u8] = b"Disk label:  ________.___\0";
    static T_FOLDS: &[u8] = b"Number of folders:         _____\0";
    static T_FILES: &[u8] = b"Number of files:         _____\0";
    static T_USED: &[u8] = b"Bytes used:    __________\0";
    static T_AVAIL: &[u8] = b"Bytes available:    __________\0";
    static V_UPPER: &[u8] = b"A\0";
    static V_FNAME: &[u8] = b"f\0";
    static V_DIGIT: &[u8] = b"9\0";

    // The six text buffers, each exactly its field's `te_txtlen`.
    let mut b_drive = [0u8; 2];
    let mut b_volume = [0u8; 12];
    let mut b_folds = [0u8; 6];
    let mut b_files = [0u8; 6];
    let mut b_used = [0u8; 11];
    let mut b_avail = [0u8; 11];

    // `drive[0] = dr_id; drive[1] = '\0'; inf_sset(tree, DIDRIVE, drive);`
    inf_sset(&mut b_drive, &[dr_id]);
    set_tedinfo_name(&mut b_volume, label.as_bytes());
    inf_fifosz(&count, &mut b_files, &mut b_folds, &mut b_used);
    inf_numset(&mut b_avail, avail);

    let mut ted_drive = rsc_ted(&mut b_drive, T_DRIVE, V_UPPER, TE_RIGHT);
    let mut ted_volume = rsc_ted(&mut b_volume, T_VOLUME, V_FNAME, TE_LEFT);
    let mut ted_folds = rsc_ted(&mut b_folds, T_FOLDS, V_DIGIT, TE_RIGHT);
    let mut ted_files = rsc_ted(&mut b_files, T_FILES, V_DIGIT, TE_RIGHT);
    let mut ted_used = rsc_ted(&mut b_used, T_USED, V_DIGIT, TE_RIGHT);
    let mut ted_avail = rsc_ted(&mut b_avail, T_AVAIL, V_DIGIT, TE_RIGHT);

    // ── geometry: `ADDISKIN`'s own, cell for cell, resolved the way
    //    `rsrc_obfix` resolves it (`fix_chpos`, `aes/gemrslib.c:53-83`:
    //    `LOBYTE * cell + (signed)HIBYTE`). The ROOT's packed height `2060` is
    //    `(8 << 8) | 12` = 12 rows + 8 px; the buttons' packed `2058` is row 10
    //    + 8 px. At 8 x 16 that is a 304 x 200 box.
    let dw = 38 * gl_wchar(); // 304
    let dh = 12 * gl_hchar() + 8; // 200
    let y_btn = 10 * gl_hchar() + 8; // 168
    let col = 3 * gl_wchar(); // char column 3
    let field_w = 32 * gl_wchar();

    // 3D roles follow `ADDISKIN` object for object and the table in DESK01b:
    // the ROOT is `FL3DBAK` + `OUTLINED`, the six fields are `FL3DBAK`,
    // `DIOK`/`DICNCL` are `FL3DBAK | FL3DIND` = [`FL3DACT`], `DITITLE` carries
    // none. A 3D activator is grown by `ADJ3DSTD` (2 px per side) before it is
    // drawn, which the resource's own measurements already leave room for:
    // the buttons sit at y 168..184 in a 200 px box.
    #[rustfmt::skip]
    let mut tree = [
        // 0 ROOT — `ob_spec` 69888 = `(1 << 16) | 0x1100` ([`BOX_SPEC`]).
        mk_root(BOX_SPEC, dw, dh, DICNCL),
        // 1 DITITLE — a `G_STRING` one row down, so `align_title` centres it.
        mk(G_STRING, 0, TITLE.as_ptr() as u64, col, gl_hchar(), 16 * gl_wchar(), gl_hchar(), DIDRIVE, -1, -1),
        // 2..7 the six value fields, char rows 3..8.
        mk(G_FBOXTEXT, FL3DBAK, &mut ted_drive as *mut _ as u64, col, 3 * gl_hchar(), field_w, gl_hchar(), DIVOLUME, -1, -1),
        mk(G_FBOXTEXT, FL3DBAK, &mut ted_volume as *mut _ as u64, col, 4 * gl_hchar(), field_w, gl_hchar(), DINFOLDS, -1, -1),
        mk(G_FBOXTEXT, FL3DBAK, &mut ted_folds as *mut _ as u64, col, 5 * gl_hchar(), field_w, gl_hchar(), DINFILES, -1, -1),
        mk(G_FBOXTEXT, FL3DBAK, &mut ted_files as *mut _ as u64, col, 6 * gl_hchar(), field_w, gl_hchar(), DIUSED, -1, -1),
        mk(G_FBOXTEXT, FL3DBAK, &mut ted_used as *mut _ as u64, col, 7 * gl_hchar(), field_w, gl_hchar(), DIAVAIL, -1, -1),
        mk(G_FBOXTEXT, FL3DBAK, &mut ted_avail as *mut _ as u64, col, 8 * gl_hchar(), field_w, gl_hchar(), DIOK as i16, -1, -1),
        // 8/9 DIOK / DICNCL, nine cells wide at char columns 7 and 22.
        mk(G_BUTTON, SELECTABLE | DEFAULT | EXIT | FL3DACT, OK.as_ptr() as u64, 7 * gl_wchar(), y_btn, 9 * gl_wchar(), gl_hchar(), DICNCL, -1, -1),
        mk(G_BUTTON, SELECTABLE | EXIT | LASTOB | FL3DACT, CANCEL.as_ptr() as u64, 22 * gl_wchar(), y_btn, 9 * gl_wchar(), gl_hchar(), 0, -1, -1),
    ];
    align_title(&mut tree, TITLE.len() as i16 - 1);

    // `inf_show(tree, ROOT); return inf_what(tree, DIOK);` — `inf_show`
    // discards `form_do`'s exit object and `inf_what` reads the buttons'
    // SELECTED state instead, so the tree has to outlive the call.
    inf_show(tree.as_mut_ptr() as u64, ROOT);
    inf_what(&mut tree, DIOK)
}

/// `inf_file_folder` (EmuTOS `deskinf.c:493-639`) — the `ADFFINFO` box for one
/// file or folder: name (editable), size, date, time, the contents count for a
/// folder, and the read-only switch.
///
/// Returns EmuTOS's three answers: `0` cancel (which stops [`super::main`]'s
/// `SHOWITEM` loop), `1` continue, `-1` continue and the window showing the
/// item needs rebuilding.
///
/// `ppath` is the item's directory, ending in `\` — EmuTOS's `ppath` is the
/// window's `p_spec` (or the shortcut's path with the file name deleted) and it
/// splices the name in at `filename_start`, which is the same position.
/// `pf` is the [`super::dir::FileInfo`] `do_info` built, i.e. the `FNODE` of
/// `desksupp.c:1293`; a rename or an attribute change is written back into it
/// the way EmuTOS writes into its `FNODE`.
///
/// ## The loop
///
/// `while(1)` (`:551-635`) exists so that a **failed rename** can be retried
/// with the box still up: `STRENAME` offers *Retry* / *Cancel*, and *Retry*
/// re-shows the dialog with the attempted name still in the field. `OK`
/// commits an attribute change first (`dos_chmod`, whose result is deliberately
/// not checked — "like Atari TOS, we don't check the return code", `:592-593`)
/// and then the rename; an unchanged name simply leaves the loop.
///
/// ## The tree
///
/// `ADFFINFO` object for object (`desk_rsc.c:622-703`, `desk_rsc.h:61-72`)
/// since DESK42: fourteen objects, its numbering, its labels, its six
/// `G_FBOXTEXT` template fields and its `Read/Write` · `Read-Only` radio pair,
/// laid out on its own 40 x 16-cell grid. Before DESK42 it was a 320 x 200 box
/// with a plain name field, three prose `G_STRING` lines and two buttons.
pub(super) fn inf_file_folder(ppath: &str, pf: &mut super::dir::FileInfo) -> i16 {
    // ── object numbers (`desk_rsc.h:61-72`) ──────────────────────────────
    //   0 ROOT · 1 FFTITLE · 2 FFNAME · 3 FFSIZE · 4 FFDATE · 5 FFTIME
    //   6 FFNUMFOL · 7 FFNUMFIL · 8 the pair's `G_IBOX` · 9 FFRWRITE
    //   10 FFRONLY · 11 FFOK · 12 FFSKIP · 13 FFCNCL
    const FFNAME: i16 = 2;
    const FFSIZE: i16 = 3;
    const FFDATE: i16 = 4;
    const FFTIME: i16 = 5;
    const FFNUMFOL: usize = 6;
    const FFNUMFIL: usize = 7;
    /// The unnamed `G_IBOX` (`ADFFINFO` object 8) the two attribute buttons
    /// hang under: the AES deselects the siblings of the button that was hit
    /// through **this** object (see [`G_IBOX`]).
    const FFATTR: i16 = 8;
    const FFRWRITE: usize = 9;
    const FFRONLY: usize = 10;
    const FFOK: usize = 11;
    const FFSKIP: usize = 12;
    const FFCNCL: i16 = 13;

    let is_dir = pf.attr & FA_SUBDIR != 0;

    // ── strings: `ADFFINFO`'s own and `desk_rs_tedinfo[0..5]`'s templates ──
    /// `STFIINFO` / `STFOINFO` (`desk_rsc.c:2470-2471`) — the title, picked by
    /// the item's type (`deskinf.c:505-508`).
    static STFIINFO: &[u8] = b"FILE INFORMATION\0";
    static STFOINFO: &[u8] = b"FOLDER INFORMATION\0";
    static RWRITE: &[u8] = b"Read/Write\0";
    static RONLY: &[u8] = b"Read-Only\0";
    static OK: &[u8] = b"OK\0";
    static SKIP: &[u8] = b"Skip\0";
    static CANCEL: &[u8] = b"Cancel\0";
    /// `STRENAME` (`desk_rsc.c:2557`). Two buttons, so it goes to `form_alert`
    /// verbatim rather than through [`super::dir::fun_alert`]'s one-OK wrapper.
    static STRENAME: &[u8] = b"[1][Rename error!][ Retry | Cancel ]\0";
    static T_NAME: &[u8] = b"Name: ________.___\0";
    static T_SIZE: &[u8] = b"Size:   __________\0";
    static T_DATE: &[u8] = b"Date:   __________\0";
    static T_TIME: &[u8] = b"Time:   __:__ __  \0";
    static T_NUMFOL: &[u8] = b"Number of folders:         _____\0";
    static T_NUMFIL: &[u8] = b"Number of files:         _____\0";
    static V_FNAME: &[u8] = b"f\0";
    static V_DIGIT: &[u8] = b"9\0";
    static V_ANY: &[u8] = b"X\0";
    static V_TIME: &[u8] = b"9999a\0";

    // The six text buffers, each exactly its field's `te_txtlen`.
    let mut b_name = [0u8; 12];
    let mut b_size = [0u8; 11];
    let mut b_date = [0u8; 11];
    let mut b_time = [0u8; 7];
    let mut b_numfol = [0u8; 6];
    let mut b_numfil = [0u8; 6];

    // `if (pf->f_attr & FA_SUBDIR) { count the contents } else { blank the two
    // counters and show the file's own size }` (`deskinf.c:519-546`).
    if is_dir {
        super::pro::desk_busy_on();
        let mut sub = String::from(ppath);
        sub.push_str(&pf.name);
        sub.push('\\'); // EmuTOS's `strcat(srcpth, "\\*.*")`
        let more = super::dir::count_ffs(&sub);
        super::pro::desk_busy_off();
        let Some(count) = more else {
            return 1; // `if (!more) return 1;`
        };
        inf_fifosz(&count, &mut b_numfil, &mut b_numfol, &mut b_size);
    } else {
        // `inf_sset(tree, FFNUMFIL, "")` / `FFNUMFOL` — both buffers are
        // already empty; the `DISABLED` state goes on below, once the tree is
        // built.
        inf_numset(&mut b_size, pf.size as u64);
    }

    // `fmt_str(pf->f_name, poname); inf_sset(tree, FFNAME, poname);` — the
    // edit field shows the 8.3-split form, and `poname` is kept because the
    // loop unformats it back into the source path on every pass.
    let mut poname = [0u8; LEN_ZFNAME];
    let pon = fmt_str(pf.name.as_bytes(), &mut poname);
    inf_sset(&mut b_name, &poname[..pon]);

    // `inf_dttm(tree, pf, FFDATE, FFTIME)` (`deskinf.c:457-468`): `fmt_date`
    // with a four-digit year always, and `fmt_time` through `"%02d%02d%s"` —
    // **no** colon, because `T_TIME`'s template supplies it.
    let (datefmt, timefmt, idt) = global::with(|g| (g.g_cdateform, g.g_ctimeform, g.g_idt))
        .unwrap_or((DateFmt::Mdy, TimeFmt::H24, 0));
    let mut work = [0u8; 16];
    let n = fmt_date(pf.date, true, datefmt, idt, &mut work);
    inf_sset(&mut b_date, &work[..n]);
    let n = fmt_time(pf.time, timefmt, idt, b"", &mut work);
    inf_sset(&mut b_time, &work[..n]);

    let mut ted_name = rsc_ted(&mut b_name, T_NAME, V_FNAME, TE_LEFT);
    let mut ted_size = rsc_ted(&mut b_size, T_SIZE, V_DIGIT, TE_RIGHT);
    let mut ted_date = rsc_ted(&mut b_date, T_DATE, V_ANY, TE_RIGHT);
    let mut ted_time = rsc_ted(&mut b_time, T_TIME, V_TIME, TE_RIGHT);
    let mut ted_numfol = rsc_ted(&mut b_numfol, T_NUMFOL, V_DIGIT, TE_RIGHT);
    let mut ted_numfil = rsc_ted(&mut b_numfil, T_NUMFIL, V_DIGIT, TE_RIGHT);

    // ── geometry: `ADFFINFO`'s own 40 x 16 cells = 320 x 256 at 8 x 16, with
    //    its blank rows at 2, 7, 10, 12 and 13. The three exit buttons are ten
    //    cells wide at columns 2, 15 and 28; the attribute pair sits inside a
    //    36-cell `G_IBOX` on row 11.
    let dw = 40 * gl_wchar(); // 320
    let dh = 16 * gl_hchar(); // 256
    let col = 4 * gl_wchar(); // char column 4
    let y_btn = 14 * gl_hchar();
    let title: &[u8] = if is_dir { STFOINFO } else { STFIINFO };

    // 3D roles follow `ADFFINFO` object for object and the DESK01b table:
    // `FL3DBAK` + `OUTLINED` ROOT, `FL3DBAK` on the six fields, `FL3DIND` on
    // the two `RBUTTON`s, `FL3DACT` (= `FL3DBAK | FL3DIND`) on the three exit
    // buttons, nothing on the title or the `G_IBOX`.
    #[rustfmt::skip]
    let mut tree = [
        // 0 ROOT
        mk_root(BOX_SPEC, dw, dh, FFCNCL),
        // 1 FFTITLE — one row down, so `align_title` centres it.
        mk(G_STRING, 0, title.as_ptr() as u64, 11 * gl_wchar(), gl_hchar(), gl_wchar(), gl_hchar(), FFNAME, -1, -1),
        // 2 FFNAME — the one `EDITABLE` field of the box.
        mk(G_FBOXTEXT, EDITABLE | FL3DBAK, &mut ted_name as *mut _ as u64, col, 3 * gl_hchar(), 26 * gl_wchar(), gl_hchar(), FFSIZE, -1, -1),
        // 3..5 FFSIZE / FFDATE / FFTIME, char rows 4..6.
        mk(G_FBOXTEXT, FL3DBAK, &mut ted_size as *mut _ as u64, col, 4 * gl_hchar(), 26 * gl_wchar(), gl_hchar(), FFDATE, -1, -1),
        mk(G_FBOXTEXT, FL3DBAK, &mut ted_date as *mut _ as u64, col, 5 * gl_hchar(), 26 * gl_wchar(), gl_hchar(), FFTIME, -1, -1),
        mk(G_FBOXTEXT, FL3DBAK, &mut ted_time as *mut _ as u64, col, 6 * gl_hchar(), 26 * gl_wchar(), gl_hchar(), FFNUMFOL as i16, -1, -1),
        // 6/7 FFNUMFOL / FFNUMFIL, char rows 8 and 9, 32 cells wide.
        mk(G_FBOXTEXT, FL3DBAK, &mut ted_numfol as *mut _ as u64, col, 8 * gl_hchar(), 32 * gl_wchar(), gl_hchar(), FFNUMFIL as i16, -1, -1),
        mk(G_FBOXTEXT, FL3DBAK, &mut ted_numfil as *mut _ as u64, col, 9 * gl_hchar(), 32 * gl_wchar(), gl_hchar(), FFATTR, -1, -1),
        // 8 the attribute pair's `G_IBOX` (`ob_spec` 0, so nothing is drawn).
        mk(G_IBOX, 0, 0, 2 * gl_wchar(), 11 * gl_hchar(), 36 * gl_wchar(), gl_hchar(), FFOK as i16, FFRWRITE as i16, FFRONLY as i16),
        // 9/10 FFRWRITE / FFRONLY, positioned inside the `G_IBOX`.
        mk(G_BUTTON, SELECTABLE | RBUTTON | FL3DIND, RWRITE.as_ptr() as u64, 0, 0, 18 * gl_wchar(), gl_hchar(), FFRONLY as i16, -1, -1),
        mk(G_BUTTON, SELECTABLE | RBUTTON | FL3DIND, RONLY.as_ptr() as u64, 19 * gl_wchar(), 0, 17 * gl_wchar(), gl_hchar(), FFATTR, -1, -1),
        // 11/12/13 FFOK / FFSKIP / FFCNCL.
        mk(G_BUTTON, SELECTABLE | DEFAULT | EXIT | FL3DACT, OK.as_ptr() as u64, 2 * gl_wchar(), y_btn, 10 * gl_wchar(), gl_hchar(), FFSKIP as i16, -1, -1),
        mk(G_BUTTON, SELECTABLE | EXIT | FL3DACT, SKIP.as_ptr() as u64, 15 * gl_wchar(), y_btn, 10 * gl_wchar(), gl_hchar(), FFCNCL, -1, -1),
        mk(G_BUTTON, SELECTABLE | EXIT | LASTOB | FL3DACT, CANCEL.as_ptr() as u64, 28 * gl_wchar(), y_btn, 10 * gl_wchar(), gl_hchar(), 0, -1, -1),
    ];
    align_title(&mut tree, title.len() as i16 - 1);
    if !is_dir {
        // `obj->ob_state |= DISABLED` on both counters (`deskinf.c:557-564`).
        tree[FFNUMFOL].ob_state |= DISABLED;
        tree[FFNUMFIL].ob_state |= DISABLED;
    }

    // ── the retry loop (`deskinf.c:551-635`) ────────────────────────────
    let mut changed = false;
    loop {
        // The attribute pair, reset on every pass (`:556-572`): a folder has
        // neither button, a file has exactly one selected.
        tree[FFRONLY].ob_state = if is_dir {
            DISABLED
        } else if pf.attr & FA_RO != 0 {
            SELECTED
        } else {
            0
        };
        tree[FFRWRITE].ob_state = if is_dir {
            DISABLED
        } else if pf.attr & FA_RO == 0 {
            SELECTED
        } else {
            0
        };
        // `obj = tree + FFOK; obj->ob_state &= ~SELECTED;` (`:574-575`) — the
        // tree is re-shown, so last pass's pressed OK must not stay inverted.
        tree[FFOK].ob_state &= !SELECTED;

        inf_show(tree.as_mut_ptr() as u64, ROOT);
        // `ret = inf_what(tree, FFSKIP); if (ret >= 0) return ret;` — Skip is
        // `1` (continue with the next item), Cancel `0` (end the series).
        let ret = inf_what(&mut tree, FFSKIP);
        if ret >= 0 {
            return ret;
        }

        // OK: rename and/or change attributes (`:580-634`).
        super::pro::desk_busy_on();

        // `inf_sget(tree, FFNAME, pnname)` plus the two `unfmt_str` calls that
        // turn the template form back into a file name.
        let mut oldname = [0u8; LEN_ZFNAME];
        let on = unfmt_str(&poname[..pon], &mut oldname);
        let bn = b_name.iter().position(|&c| c == 0).unwrap_or(b_name.len());
        let mut newname = [0u8; LEN_ZFNAME];
        let nn = unfmt_str(&b_name[..bn], &mut newname);
        // A name the AES's validated field cannot produce (`te_pvalid` "f"
        // admits no byte above 0x7E) — treated as "unchanged", which is the
        // one answer that cannot damage the file.
        let (Ok(old_s), Ok(new_s)) = (
            core::str::from_utf8(&oldname[..on]),
            core::str::from_utf8(&newname[..nn]),
        ) else {
            break;
        };

        // `if (!(pf->f_attr & FA_SUBDIR)) { … dos_chmod … }` (`:585-605`).
        if !is_dir {
            let mut attr = pf.attr;
            if tree[FFRONLY].ob_state & SELECTED != 0 {
                attr |= FA_RO;
            } else {
                attr &= !FA_RO;
            }
            if attr != pf.attr {
                let mut src = String::from(ppath);
                src.push_str(old_s);
                newtos_gem::gemdos::dos_chmod(&src, newtos_gem::gemdos::F_SETMOD, attr);
                pf.attr = attr;
                changed = true;
            }
        }

        // `if (strcmp(srcpth+nmidx, dstpth+nmidx) == 0) break;` (`:610-614`).
        if old_s == new_s {
            break;
        }

        let mut src = String::from(ppath);
        src.push_str(old_s);
        let mut dst = String::from(ppath);
        dst.push_str(new_s);
        if super::dir::rename(&src, &dst) {
            // `strcpy(pf->f_name, dstpth+nmidx);` (`:623`).
            pf.name = String::from(new_s);
            changed = true;
            break;
        }

        // `if (fun_alert(1, STRENAME) == 2) break;` (`:632-633`) — *Retry*
        // re-shows the box with the attempted name still in the field.
        if super::form_alert(1, STRENAME.as_ptr()) == 2 {
            break;
        }
    }

    super::pro::desk_busy_off();
    // `return changed ? -1 : 1;`
    if changed {
        -1
    } else {
        1
    }
}

// ── Preferences dialog (`inf_pref`, `deskinf.c:721-818`) ────────────────────

/// `inf_which` (`deskinf.c:705-716`) — the index, **counted from the end**, of
/// the first `SELECTED` object in `tree[baseobj .. baseobj+numobj]`, or `-1`
/// when none is selected. For a two-button Yes/No pair that is `1` for Yes and
/// `0` for No; for the three-button format groups it is EmuTOS's
/// `TIMEFORM_*`/`DATEFORM_*` value read straight off the left-to-right button
/// order (`deskinf.c:793-805`, whose comment says exactly that).
fn inf_which(tree: &[Object], baseobj: usize, numobj: usize) -> i16 {
    let mut i = numobj as i16 - 1;
    for obj in tree.iter().skip(baseobj).take(numobj) {
        if obj.ob_state & SELECTED != 0 {
            break;
        }
        i -= 1;
    }
    i
}

/// `inf_gindex` (`util/optimize.c:134-147`) — the index, **counted from the
/// start**, of the first `SELECTED` object in `tree[baseobj .. baseobj+numobj]`,
/// or `-1` when none is selected. The reverse of [`inf_which`].
pub(super) fn inf_gindex(tree: &[Object], baseobj: usize, numobj: usize) -> i16 {
    for (n, obj) in tree.iter().skip(baseobj).take(numobj).enumerate() {
        if obj.ob_state & SELECTED != 0 {
            return n as i16;
        }
    }
    -1
}

/// `inf_what` (`util/optimize.c:157-170`) — `1` if `ok` was the selected
/// button, `0` if `ok + 1` (its Cancel neighbour) was, `-1` otherwise; the
/// selected button is deselected on the way out.
fn inf_what(tree: &mut [Object], ok: usize) -> i16 {
    let field = inf_gindex(tree, ok, 2);
    if field == -1 {
        return -1;
    }
    tree[ok + field as usize].ob_state = 0; // `objptr->ob_state = NORMAL;`
    i16::from(field == 0)
}

/// Options → Set preferences (`inf_pref`, `deskinf.c:721-818`).
///
/// Pre-selects every radio group from the current preference values, runs the
/// dialog through [`inf_show`], and on OK reads each group back with
/// [`inf_which`] / [`inf_gindex`] and commits it. Returns EmuTOS's narrow
/// rebuild signal: `true` **only** when the view is `V_TEXT` *and* the time or
/// date format actually changed (`deskinf.c:807-814`) — nothing else in this
/// dialog alters anything already on screen.
///
/// ## The field set
///
/// All six groups of `ADSETPRE` (`desk/desk_rsc.c:1369-1567`), in its order and
/// under its object numbers (`desk_rsc.h:164-184`): confirm deletes, confirm
/// copies, confirm overwrites, double-click speed, time format, date format.
///
/// DESK02 shipped three of them, because the other three had nothing behind
/// them: NewTOS's copy engine asked neither a copy nor an overwrite question,
/// and `Default` means "take the format from the `_IDT` cookie", which was out
/// of the desktop's reach. DESK13 supplied all three — [`super::dir::dir_op`]'s
/// confirmation box, `output_fname`'s name-conflict dialog, and the cookie read
/// through GEMDOS `Ssystem` ([`newtos_gem::gemdos::dos_getcookie`]) — so the fields
/// are back, and no control in this box is dead.
///
/// Note what is **not** needed any more: DESK02 had to pre-select through
/// `TimeFmt::rendered` / `DateFmt::rendered`, because a stored `Idt` had no
/// button and the box would otherwise have claimed a format the text view did
/// not print. With `Default` on screen the raw value is shown and stored, which
/// is EmuTOS's own code (`deskinf.c:754-778`, `:797-805`).
pub(super) fn inf_pref() -> bool {
    // ── current values ──────────────────────────────────────────────────
    let (confdel, confcpy, confovwr, timefmt, datefmt, view) = global::with(|g| {
        (g.g_cdelepref, g.g_ccopypref, g.g_covwrpref, g.g_ctimeform, g.g_cdateform, g.view)
    })
    .unwrap_or((true, true, true, TimeFmt::H24, DateFmt::Mdy, View::Icon));
    // `G.g_cdclkpref = evnt_dclick(0, FALSE);` (`deskinf.c:751`) — the rate is
    // the AES's, so it is asked for rather than remembered.
    let dclick = super::evnt_dclick(0, false).clamp(0, 4);

    static TITLE: &[u8] = b" SET PREFERENCES \0";
    static CONFDEL_LBL: &[u8] = b"Confirm deletes:\0";
    static CONFCPY_LBL: &[u8] = b"Confirm copies:\0";
    static CONFOVW_LBL: &[u8] = b"Confirm overwrites:\0";
    static DCLICK_LBL: &[u8] = b"Double-click speed:\0";
    static TIME_LBL: &[u8] = b"Time format:\0";
    static DATE_LBL: &[u8] = b"Date format:\0";
    static YES: &[u8] = b" Yes \0";
    static NO: &[u8] = b" No \0";
    static DC: [&[u8]; 5] = [b"1\0", b"2\0", b"3\0", b"4\0", b"5\0"];
    static DEFAULT_LBL: &[u8] = b" Default \0";
    static H12: &[u8] = b" 12H \0";
    static H24: &[u8] = b" 24H \0";
    static MDY: &[u8] = b" MDY \0";
    static DMY: &[u8] = b" DMY \0";
    static OK: &[u8] = b" OK \0";
    static CANCEL: &[u8] = b" Cancel \0";

    // ── geometry ────────────────────────────────────────────────────────
    //
    // The pitch and the gap are `ADSETPRE`'s own, because a 3D indicator is
    // **grown** before it is drawn: the AES adds `ADJ3DSTD` (2 px) per side and
    // then lays the `th = -1` border outside that (`aes/gemoblib.c:504`,
    // `tos-api/src/aes/object.rs`), so every `FL3DIND` button claims 3 px more
    // in each direction than its `ob_*` rectangle says. `ADSETPRE` spaces its
    // rows two character rows apart (`desk_rsc.c:1386,1406,1426,1450` — char
    // rows 3, 5, 7, 9 → 32 px) and its speed buttons 23 px apart for a 15 px
    // button (`:1462-1493`, an 8 px gap).
    //
    // `ADSETPRE` puts the two format groups on a row of their **own**, below
    // their label and spanning the whole box (`:1503,1533`: an `IBOX` at char
    // column 1, 38 wide, one row under the `G_STRING`); NewTOS keeps every
    // group on its label's row, which is the layout the other four rows already
    // had and which `docs/api/desktop/deskinf.md` marks as free (the visual
    // layout is fidelity, not contract). The box therefore grows sideways to
    // fit the widest group instead of downwards.
    const MARGIN: i16 = 24;
    // `ADSETPRE`'s two-character row pitch (`desk_rsc.c:1386` ff.).
    let row_h = 2 * gl_hchar();
    const BTN_H: i16 = 22;
    const YESNO_W: i16 = 52;
    const DC_W: i16 = 30;
    const FMT_W: i16 = 56;
    // " Default " — the widest label in either format group.
    let def_w = 9 * gl_wchar();
    /// `ADSETPRE`'s inter-button gap (`desk_rsc.c:1462-1493`: pitch 23 px,
    /// width 15 px).
    const GAP: i16 = 8;
    let lbl_w = 20 * gl_wchar(); // "Confirm overwrites:" + a blank
    /// The five speed buttons.
    const SPEED_W: i16 = 5 * DC_W + 4 * GAP;
    // `Default` + two format buttons — the widest group.
    let fmtgrp_w = def_w + GAP + 2 * (FMT_W + GAP) - GAP;
    let group_w = if SPEED_W > fmtgrp_w { SPEED_W } else { fmtgrp_w };
    let dw = MARGIN + lbl_w + group_w + MARGIN;
    let gx = MARGIN + lbl_w;

    let y_del = MARGIN + row_h;
    let y_ccpy = y_del + row_h;
    let y_covw = y_ccpy + row_h;
    let y_dc = y_covw + row_h;
    let y_tf = y_dc + row_h;
    let y_df = y_tf + row_h;
    let y_btn = y_df + row_h + GAP;
    let dh = y_btn + 28 + MARGIN;
    // Vertical offset of a `BTN_H` button inside its `gl_hchar()` label row.
    let boff = (gl_hchar() - BTN_H) / 2;

    // ── object indices — `desk_rsc.h:164-184`, unchanged ────────────────
    /// `SPCDYES` — the *Confirm deletes* `Yes` button; `SPCDNO` follows it.
    const SPCDYES: usize = 4;
    /// `SPCCYES` — the *Confirm copies* `Yes` button (`:168`).
    const SPCCYES: usize = 8;
    /// `SPCOWYES` — the *Confirm overwrites* `Yes` button (`:170`).
    const SPCOWYES: usize = 12;
    /// `SPDC1` — the first of the five double-click speed buttons (`:172`).
    const SPDC1: usize = 16;
    /// `SPTF_DEF` — the first time-format button (`:177`); `SPTF12HR` and
    /// `SPTF24HR` follow, in that left-to-right order.
    const SPTF_DEF: usize = 23;
    /// `SPDF_DEF` — the first date-format button (`:180`); `SPDFMMDD` and
    /// `SPDFDDMM` follow.
    const SPDF_DEF: usize = 28;
    /// `SPOK` — the OK button; `SPCNCL` follows it, as [`inf_what`] requires.
    const SPOK: usize = 31;

    // Every `RBUTTON` of `ADSETPRE` is `SELECTABLE | RBUTTON | FL3DIND`
    // (`desk_rsc.c:1396` ff.) — the 3D indicator role: grown, beveled, and it
    // changes colour when selected.
    let rb = |spec: &'static [u8], x: i16, w: i16, next: i16| {
        mk(G_BUTTON, SELECTABLE | RBUTTON | FL3DIND, spec.as_ptr() as u64, x, 0, w, BTN_H, next, -1, -1)
    };
    let yesno = |y: i16, next: i16, head: i16| {
        mk(G_IBOX, 0, 0, gx, y + boff, 2 * YESNO_W + GAP, BTN_H, next, head, head + 1)
    };

    // Each radio group hangs under its own `G_IBOX`, because that is what makes
    // the AES deselect the siblings of the button that was hit (see [`G_IBOX`]).
    // `ADSETPRE`'s `G_IBOX`es carry no 3D flag and `ob_spec = 0` (`:1390` ff.).
    #[rustfmt::skip]
    let mut tree = [
        // 0 ROOT
        mk_root(BOX_SPEC, dw, dh, SPOK as i16 + 1),
        // 1 title
        mk(G_STRING, 0, TITLE.as_ptr() as u64, MARGIN, MARGIN, dw - 2 * MARGIN, gl_hchar(), 2, -1, -1),
        // 2 "Confirm deletes:"          3 its G_IBOX      4/5 Yes/No
        mk(G_STRING, 0, CONFDEL_LBL.as_ptr() as u64, MARGIN, y_del, lbl_w, gl_hchar(), 3, -1, -1),
        yesno(y_del, 6, SPCDYES as i16),
        rb(YES, 0, YESNO_W, SPCDYES as i16 + 1),
        rb(NO, YESNO_W + GAP, YESNO_W, 3),
        // 6 "Confirm copies:"           7 its G_IBOX      8/9 Yes/No
        mk(G_STRING, 0, CONFCPY_LBL.as_ptr() as u64, MARGIN, y_ccpy, lbl_w, gl_hchar(), 7, -1, -1),
        yesno(y_ccpy, 10, SPCCYES as i16),
        rb(YES, 0, YESNO_W, SPCCYES as i16 + 1),
        rb(NO, YESNO_W + GAP, YESNO_W, 7),
        // 10 "Confirm overwrites:"      11 its G_IBOX     12/13 Yes/No
        mk(G_STRING, 0, CONFOVW_LBL.as_ptr() as u64, MARGIN, y_covw, lbl_w, gl_hchar(), 11, -1, -1),
        yesno(y_covw, 14, SPCOWYES as i16),
        rb(YES, 0, YESNO_W, SPCOWYES as i16 + 1),
        rb(NO, YESNO_W + GAP, YESNO_W, 11),
        // 14 "Double-click speed:"      15 its G_IBOX     16..20 the five speeds
        mk(G_STRING, 0, DCLICK_LBL.as_ptr() as u64, MARGIN, y_dc, lbl_w, gl_hchar(), 15, -1, -1),
        mk(G_IBOX, 0, 0, gx, y_dc + boff, SPEED_W, BTN_H, 21, SPDC1 as i16, SPDC1 as i16 + 4),
        rb(DC[0], 0, DC_W, SPDC1 as i16 + 1),
        rb(DC[1], DC_W + GAP, DC_W, SPDC1 as i16 + 2),
        rb(DC[2], 2 * (DC_W + GAP), DC_W, SPDC1 as i16 + 3),
        rb(DC[3], 3 * (DC_W + GAP), DC_W, SPDC1 as i16 + 4),
        rb(DC[4], 4 * (DC_W + GAP), DC_W, 15),
        // 21 "Time format:"             22 its G_IBOX     23/24/25 Default/12H/24H
        mk(G_STRING, 0, TIME_LBL.as_ptr() as u64, MARGIN, y_tf, lbl_w, gl_hchar(), 22, -1, -1),
        mk(G_IBOX, 0, 0, gx, y_tf + boff, fmtgrp_w, BTN_H, 26, SPTF_DEF as i16, SPTF_DEF as i16 + 2),
        rb(DEFAULT_LBL, 0, def_w, SPTF_DEF as i16 + 1),
        rb(H12, def_w + GAP, FMT_W, SPTF_DEF as i16 + 2),
        rb(H24, def_w + GAP + FMT_W + GAP, FMT_W, 22),
        // 26 "Date format:"             27 its G_IBOX     28/29/30 Default/MDY/DMY
        mk(G_STRING, 0, DATE_LBL.as_ptr() as u64, MARGIN, y_df, lbl_w, gl_hchar(), 27, -1, -1),
        mk(G_IBOX, 0, 0, gx, y_df + boff, fmtgrp_w, BTN_H, SPOK as i16, SPDF_DEF as i16, SPDF_DEF as i16 + 2),
        rb(DEFAULT_LBL, 0, def_w, SPDF_DEF as i16 + 1),
        rb(MDY, def_w + GAP, FMT_W, SPDF_DEF as i16 + 2),
        rb(DMY, def_w + GAP + FMT_W + GAP, FMT_W, 27),
        // 31 OK                         32 Cancel  (`SPOK`/`SPCNCL`: FL3DACT)
        mk(G_BUTTON, SELECTABLE | DEFAULT | EXIT | FL3DACT, OK.as_ptr() as u64, dw / 2 - 100, y_btn, 90, 28, SPOK as i16 + 1, -1, -1),
        mk(G_BUTTON, SELECTABLE | EXIT | LASTOB | FL3DACT, CANCEL.as_ptr() as u64, dw / 2 + 10, y_btn, 90, 28, 0, -1, -1),
    ];

    // ── pre-select the current state (`deskinf.c:731-777`) ──────────────
    // `deselect_all(tree1)` is implicit: `mk` leaves every `ob_state` at 0.
    tree[SPCDYES + usize::from(!confdel)].ob_state |= SELECTED;
    tree[SPCCYES + usize::from(!confcpy)].ob_state |= SELECTED;
    tree[SPCOWYES + usize::from(!confovwr)].ob_state |= SELECTED;
    tree[SPDC1 + dclick as usize].ob_state |= SELECTED;
    // `switch(G.g_ctimeform)` / `switch(G.g_cdateform)` (`:754-778`), with the
    // `default:` arm — the `Idt` value — landing on the `Default` button.
    tree[match timefmt {
        TimeFmt::H12 => SPTF_DEF + 1,
        TimeFmt::H24 => SPTF_DEF + 2,
        TimeFmt::Idt => SPTF_DEF,
    }]
    .ob_state |= SELECTED;
    tree[match datefmt {
        DateFmt::Mdy => SPDF_DEF + 1,
        DateFmt::Dmy => SPDF_DEF + 2,
        DateFmt::Idt => SPDF_DEF,
    }]
    .ob_state |= SELECTED;

    // ── run it ──────────────────────────────────────────────────────────
    inf_show(tree.as_mut_ptr() as u64, ROOT);
    if inf_what(&mut tree, SPOK) != 1 {
        return false;
    }

    // ── commit (`deskinf.c:784-814`) ────────────────────────────────────
    let new_confdel = inf_which(&tree, SPCDYES, 2) == 1;
    let new_confcpy = inf_which(&tree, SPCCYES, 2) == 1;
    let new_confovwr = inf_which(&tree, SPCOWYES, 2) == 1;
    // `G.g_cdclkpref = evnt_dclick(inf_gindex(tree1, SPDC1, 5), TRUE);`
    let picked = inf_gindex(&tree, SPDC1, 5).max(0);
    let new_dclick = super::evnt_dclick(picked, true).clamp(0, 4) as u8;
    // "the following assumes that the buttons are in the left-to-right order:
    // default, 12hour, 24hour" (`:793-796`) — `inf_which` counts from the end,
    // so the three buttons yield 2/1/0, which *is* `TIMEFORM_IDT` (2),
    // `TIMEFORM_12H` (1), `TIMEFORM_24H` (0) (`deskbind.h:68-70`). Same for
    // default/MDY/DMY against `DATEFORM_IDT`/`_MDY`/`_DMY` (`:65-67`).
    let new_timefmt = match inf_which(&tree, SPTF_DEF, 3) {
        2 => TimeFmt::Idt,
        1 => TimeFmt::H12,
        // 0, and `inf_which`'s "nothing selected" -1, which cannot happen: a
        // radio group always has exactly one member selected.
        _ => TimeFmt::H24,
    };
    let new_datefmt = match inf_which(&tree, SPDF_DEF, 3) {
        2 => DateFmt::Idt,
        1 => DateFmt::Mdy,
        _ => DateFmt::Dmy,
    };

    global::with(|g| {
        g.g_cdelepref = new_confdel;
        g.g_ccopypref = new_confcpy;
        g.g_covwrpref = new_confovwr;
        g.g_cdclkpref = new_dclick;
        g.g_ctimeform = new_timefmt;
        g.g_cdateform = new_datefmt;
    });

    // "if the current view is as text, and the date or time format has changed,
    // we need to tell the caller so he can rebuild the windows if necessary"
    // (`deskinf.c:807-814`) — the raw values, as EmuTOS compares them.
    view == View::Text && (new_timefmt != timefmt || new_datefmt != datefmt)
}

// ── Set-background dialog (`inf_backgrounds`, `deskinf.c:825-921`) ───────────

/// `SAMPLE_BORDER_FLAGS` (`deskinf.c:53-57`) — "Border flags for `BGSAMPLE`
/// object in 'Set background' dialog: outside border, thickness=1, border
/// colour black, text black."
const SAMPLE_BORDER_FLAGS: u64 = 0x00ff_1100;
/// `FILLPAT_MASK` (`include/obdefs.h:126`) — the fill-pattern bits of an
/// `ob_spec` colour word.
const FILLPAT_MASK: u64 = 0x0000_0070;
/// `FILLCOL_MASK` (`include/obdefs.h:127`) — the fill-colour bits.
const FILLCOL_MASK: u64 = 0x0000_000f;

/// `tree[BGWIN].ob_state & SELECTED` of `ADBKGND` between two calls of
/// [`inf_backgrounds`]. EmuTOS's tree lives in RAM for the whole `deskmain`
/// run (`desk_rs_init` copies it once per run, `deskmain.c:1980`,
/// `desk_rsc.c:2568-2577`), so the *Desktop*/*Window* radio a user left the
/// dialog with is the one it opens with next time. This tree is built on the
/// stack per call, so that one bit of it is kept here and cleared with the
/// rest of the run's state ([`reset`]).
static BG_WINDOW_SELECTED: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

/// `inf_backgrounds` (EmuTOS `deskinf.c:825-921`, `CONF_WITH_BACKGROUNDS`) —
/// View → Set background…: the `ADBKGND` box with its live sample.
///
/// The `BGSAMPLE` box is redrawn with `draw_fld` before every `form_do`; a click
/// on one of the eight pattern swatches (`BGPAT0..7`) copies that swatch's
/// `FILLPAT_MASK` bits into the sample, a click on one of the sixteen colour
/// swatches (`BGCOL0..15`) its `FILLCOL_MASK` bits, and the *Desktop*/*Window*
/// radios switch which of the two candidates (`curdesk`/`curwin`) the sample
/// shows and edits. On OK a changed desktop background is applied and
/// repainted here (`:900-908`); a changed window background is put into every
/// window box and reported with `TRUE` (`:910-916`), on which `do_viewmenu`
/// answers `BACKGROUND_HAS_CHANGED` and the windows are repainted.
///
/// Source quirks kept as they are: the sample starts with
/// `SAMPLE_BORDER_FLAGS`, but a click on *Desktop*/*Window* puts the plain
/// candidate (no border) back into it; and because `curdesk`/`curwin` are
/// taken from the sample after every turn, a confirmed swatch click carries
/// the sample's border flags into `G.g_screen[…].ob_spec` (the next
/// `set_background`, `deskapp.c:1306-1331`, writes the plain word again).
///
/// **Not reproduced: `set_aes_background(curdesk & 0xff)` (`:904`).** That is
/// an AES-internal function (`aes/geminit.c:635-641`) the ROM desktop can call
/// because it is linked into the AES; it sets the background the AES paints
/// when a program is launched. The `NEWDESK` program has no call for it; the
/// AES takes the byte from the `.INF` `#Q` line at its start
/// (`tos-api/src/aes/init.rs`, `geminit.c:719-728`).
///
/// **Named deviation (DESK64, to be generalised by DESK65): the change is
/// saved at once.** EmuTOS leaves writing the `.INF` to the user's next *Save
/// desktop* (`inf_backgrounds` ends at `:919`); NewTOS calls `cnx_put();
/// app_save(TRUE)` whenever OK changed either background — user decision
/// 2026-10-17. `cnx_put()` first, as before every EmuTOS `app_save`
/// (`deskmain.c:891,2154`), so the `#W` lines describe the windows on screen.
pub(super) fn inf_backgrounds() -> bool {
    let window_selected = BG_WINDOW_SELECTED.load(core::sync::atomic::Ordering::Relaxed);
    let mut tree = adbkgnd_tree(gl_wchar(), gl_hchar(), global::gl_nplanes(), window_selected);

    // "set the initially-displayed background pattern" (`:854-858`).
    let Some((mut curdesk, mut curwin)) = global::with(|g| {
        (g.g_screen[DROOT as usize].ob_spec, g.g_screen[(DROOT + 1) as usize].ob_spec)
    }) else {
        return false;
    };
    tree[BGSAMPLE as usize].ob_spec =
        (if tree[BGDESK as usize].ob_state & SELECTED != 0 { curdesk } else { curwin }) | SAMPLE_BORDER_FLAGS;

    // `start_dialog(tree); while(1) { draw_fld(tree, BGSAMPLE); ret =
    // form_do(tree, ROOT); … } end_dialog(tree);` (`:860-896`).
    // The tree's address is taken afresh for every AES call, so no raw
    // pointer outlives a direct write to `tree` between two calls.
    start_dialog(tree.as_mut_ptr() as u64);
    let ret = loop {
        super::dir::draw_fld(tree.as_mut_ptr() as u64, BGSAMPLE);
        let ret = super::form_do(tree.as_mut_ptr() as u64, ROOT);
        if ret == BGOK || ret == BGCANCEL {
            tree[ret as usize].ob_state &= !SELECTED;
            break ret;
        }
        bg_turn(&mut tree, ret, &mut curdesk, &mut curwin);
    };
    end_dialog(tree.as_mut_ptr() as u64);
    BG_WINDOW_SELECTED.store(tree[BGWIN as usize].ob_state & SELECTED != 0, core::sync::atomic::Ordering::Relaxed);

    // "handle dialog exit" (`:898-918`).
    if ret != BGOK {
        return false;
    }
    let mut desk_changed = false;
    let mut win_changed = false;
    // "check for desktop background change" (`:900-908`).
    if global::with(|g| g.g_screen[DROOT as usize].ob_spec) != Some(curdesk) {
        global::set_patcol_desktop((curdesk & 0xff) as u8);
        global::with(|g| g.g_screen[DROOT as usize].ob_spec = curdesk);
        let desk = global::with(|g| g.g_desk).unwrap_or_default();
        super::redraw::do_wredraw(DESKWH, desk);
        desk_changed = true;
    }
    // "check for window background change" (`:910-916`): every window box
    // `DROOT+1 .. WOBS_START-1`, open or not.
    if global::with(|g| g.g_screen[(DROOT + 1) as usize].ob_spec) != Some(curwin) {
        global::set_patcol_window((curwin & 0xff) as u8);
        global::with(|g| {
            for obj in &mut g.g_screen[(DROOT + 1) as usize..WOBS_START as usize] {
                obj.ob_spec = curwin;
            }
        });
        win_changed = true;
    }
    // NEWTOS: named deviation (DESK64, generalised by DESK65) — save the new
    // background at once instead of at the next *Save desktop*.
    if desk_changed || win_changed {
        super::main::cnx_put();
        app_save(true);
    }
    win_changed
}

// ── `ADBKGND` object numbers (`desk_rsc.h:251-260`) ──────────────────────────
//   0 ROOT · 1 title · 2 `Select:` · 3 radio box · 4 BGDESK · 5 BGWIN
//   6 `Style:` · 7 `Color:` · 8 pattern box · 9..16 BGPAT0..7 · 17 BGSAMPLE
//   18 colour box · 19..34 BGCOL0..15 · 35 BGOK · 36 BGCANCEL
const BGDESK: i16 = 4;
const BGWIN: i16 = 5;
const BGPAT0: i16 = 9;
const BGPAT7: i16 = 16;
const BGSAMPLE: i16 = 17;
const BGCOL0: i16 = 19;
const BGCOL15: i16 = 34;
const BGOK: i16 = 35;
const BGCANCEL: i16 = 36;
/// Number of objects in `ADBKGND` (`desk_rsc.c:2230-2452`, `TR19`).
const ADBKGND_NOBS: usize = 37;

/// The `ADBKGND` tree (`desk_rsc.c:2230-2452`) object for object, its
/// coordinates resolved for a `cw` x `ch` character cell and its colour
/// swatches hidden the way `inf_backgrounds` hides them for `nplanes`
/// (`deskinf.c:832-852`); `window_selected` picks the radio that starts
/// `SELECTED` (see [`BG_WINDOW_SELECTED`]).
fn adbkgnd_tree(cw: i16, ch: i16, nplanes: i16, window_selected: bool) -> [Object; ADBKGND_NOBS] {
    // `switch(gl_nplanes)` (`deskinf.c:832-847`): the first colour the screen
    // does not have. The pair index is [`global::patcol_index`], the same switch.
    let unused: i16 = match nplanes {
        1 => 2,
        2 => 4,
        _ => 16,
    };

    // ── strings: `ADBKGND`'s own (`desk_rsc.c:2230-2452`).
    static TITLE: &[u8] = b"SET BACKGROUND\0";
    static SELECT: &[u8] = b"Select:\0";
    static DESKTOP: &[u8] = b"Desktop\0";
    static WINDOW: &[u8] = b"Window\0";
    static STYLE: &[u8] = b"Style:\0";
    static COLOR: &[u8] = b"Color:\0";
    static OK: &[u8] = b"OK\0";
    static CANCEL: &[u8] = b"Cancel\0";

    // ── geometry: `ADBKGND`'s own, cell for cell, resolved the way
    //    `rsrc_obfix` resolves it (`fix_chpos`, `aes/gemrslib.c:53-83`:
    //    `LOBYTE * cell + (signed)HIBYTE`). The two labels' packed row `1030`
    //    is `(4 << 8) | 6` = row 6 + 4 px. At 8 x 16 the box is 304 x 256.
    let cell = |x: i16, y: i16, w: i16, h: i16| (x * cw, y * ch, w * cw, h * ch);
    let place = |o: &mut Object, (x, y, w, h): (i16, i16, i16, i16)| {
        o.ob_x = x;
        o.ob_y = y;
        o.ob_width = w;
        o.ob_height = h;
    };

    // 3D roles follow `ADBKGND` object for object: the ROOT is `FL3DBAK` +
    // `OUTLINED`, the radios `BGDESK`/`BGWIN` are `FL3DIND` (a radio that is
    // also `EXIT` stays an indicator), `BGOK`/`BGCANCEL` are `FL3DBAK |
    // FL3DIND` = [`FL3DACT`]; the three panel boxes, the swatches, the sample
    // and the `G_STRING`s carry none.
    let mut tree = [mk(0, 0, 0, 0, 0, 0, 0, -1, -1, -1); ADBKGND_NOBS];
    // 0 ROOT — `ob_spec` 69888 = `(1 << 16) | 0x1100` ([`BOX_SPEC`]), 38 x 16.
    tree[0] = mk_root(BOX_SPEC, 38 * cw, 16 * ch, BGCANCEL);
    // 1 title, 2 `Select:`.
    tree[1] = mk(G_STRING, 0, TITLE.as_ptr() as u64, 0, 0, 0, 0, 2, -1, -1);
    place(&mut tree[1], cell(12, 1, 14, 1));
    tree[2] = mk(G_STRING, 0, SELECT.as_ptr() as u64, 0, 0, 0, 0, 3, -1, -1);
    place(&mut tree[2], cell(2, 4, 7, 1));
    // 3 the radio box (`4352` = `0x1100`), children 4..5.
    tree[3] = mk(G_BOX, 0, BORDER_TEXT_COLOURS, 0, 0, 0, 0, 6, BGDESK, BGWIN);
    place(&mut tree[3], cell(15, 4, 21, 1));
    tree[4] = mk(G_BUTTON, SELECTABLE | EXIT | RBUTTON | FL3DIND, DESKTOP.as_ptr() as u64, 0, 0, 0, 0, BGWIN, -1, -1);
    place(&mut tree[4], cell(0, 0, 11, 1));
    tree[5] = mk(G_BUTTON, SELECTABLE | EXIT | RBUTTON | FL3DIND, WINDOW.as_ptr() as u64, 0, 0, 0, 0, 3, -1, -1);
    place(&mut tree[5], cell(12, 0, 9, 1));
    // 6 `Style:`, 7 `Color:` — row 6 + 4 px.
    tree[6] = mk(G_STRING, 0, STYLE.as_ptr() as u64, 0, 0, 0, 0, 7, -1, -1);
    place(&mut tree[6], cell(2, 6, 12, 1));
    tree[6].ob_y += 4;
    tree[7] = mk(G_STRING, 0, COLOR.as_ptr() as u64, 0, 0, 0, 0, 8, -1, -1);
    place(&mut tree[7], cell(24, 6, 12, 1));
    tree[7].ob_y += 4;
    // 8 the pattern box, children 9..16; 9..16 BGPAT0..7, `ob_spec`
    // `0xff1100 | (k << 4) | (k != 0)` (`16716032`, `16716049`, …,
    // `16716145`): pattern k, BGPAT0 in white (hollow), the rest in black.
    tree[8] = mk(G_BOX, 0, BORDER_TEXT_COLOURS, 0, 0, 0, 0, BGSAMPLE, BGPAT0, BGPAT7);
    place(&mut tree[8], cell(2, 8, 12, 2));
    for k in 0..8i16 {
        let i = (BGPAT0 + k) as usize;
        let spec = SAMPLE_BORDER_FLAGS | ((k as u64) << 4) | u64::from(k != 0);
        let next = if k == 7 { 8 } else { BGPAT0 + k + 1 };
        tree[i] = mk(G_BOX, TOUCHEXIT, spec, 0, 0, 0, 0, next, -1, -1);
        place(&mut tree[i], cell(3 * (k % 4), k / 4, 3, 1));
    }
    // 17 BGSAMPLE (`16716033` = `0xff1101`; `inf_backgrounds` overwrites it).
    tree[17] = mk(G_BOX, 0, 0x00ff_1101, 0, 0, 0, 0, 18, -1, -1);
    place(&mut tree[17], cell(17, 8, 4, 2));
    // 18 the colour box, children 19..34; 19..34 BGCOL0..15, `ob_spec`
    // `0xff1170 | k` (`16716144` … `16716159`): solid fill in colour k.
    tree[18] = mk(G_BOX, 0, BORDER_TEXT_COLOURS, 0, 0, 0, 0, BGOK, BGCOL0, BGCOL15);
    place(&mut tree[18], cell(24, 8, 12, 4));
    for k in 0..16i16 {
        let i = (BGCOL0 + k) as usize;
        let spec = SAMPLE_BORDER_FLAGS | 0x70 | k as u64;
        let next = if k == 15 { 18 } else { BGCOL0 + k + 1 };
        tree[i] = mk(G_BOX, TOUCHEXIT, spec, 0, 0, 0, 0, next, -1, -1);
        place(&mut tree[i], cell(3 * (k % 4), k / 4, 3, 1));
    }
    // 35 BGOK, 36 BGCANCEL.
    tree[35] = mk(G_BUTTON, SELECTABLE | DEFAULT | EXIT | FL3DACT, OK.as_ptr() as u64, 0, 0, 0, 0, BGCANCEL, -1, -1);
    place(&mut tree[35], cell(7, 14, 9, 1));
    tree[36] = mk(G_BUTTON, SELECTABLE | EXIT | LASTOB | FL3DACT, CANCEL.as_ptr() as u64, 0, 0, 0, 0, 0, -1, -1);
    place(&mut tree[36], cell(22, 14, 9, 1));
    align_title(&mut tree, TITLE.len() as i16 - 1);

    // The radio pair: `BGDESK` is `SELECTED` in the resource; a later call
    // starts with whichever the previous one left selected.
    tree[if window_selected { BGWIN } else { BGDESK } as usize].ob_state = SELECTED;

    // "hide colours that are not available in the current resolution"
    // (`deskinf.c:850-852`).
    for obj in &mut tree[(BGCOL0 + unused) as usize..=BGCOL15 as usize] {
        obj.ob_flags |= HIDETREE;
    }
    tree
}

/// One turn of `inf_backgrounds`' loop after `form_do` returned `ret` (not
/// `BGOK`/`BGCANCEL`) — `deskinf.c:875-893`: a radio puts its candidate into
/// the sample, a pattern / colour swatch replaces the sample's
/// `FILLPAT_MASK` / `FILLCOL_MASK` bits, and the sample then becomes the
/// candidate of the selected radio. A `ret` that is none of these (a
/// double-click's `0x8000` bit, an unnamed object) changes nothing but that
/// last step, as in the source.
fn bg_turn(tree: &mut [Object; ADBKGND_NOBS], ret: i16, curdesk: &mut u64, curwin: &mut u64) {
    let s = BGSAMPLE as usize;
    if ret == BGDESK {
        tree[s].ob_spec = *curdesk;
    } else if ret == BGWIN {
        tree[s].ob_spec = *curwin;
    } else if (BGPAT0..=BGPAT7).contains(&ret) {
        tree[s].ob_spec &= !FILLPAT_MASK;
        tree[s].ob_spec |= tree[ret as usize].ob_spec & FILLPAT_MASK;
    } else if (BGCOL0..=BGCOL15).contains(&ret) {
        tree[s].ob_spec &= !FILLCOL_MASK;
        tree[s].ob_spec |= tree[ret as usize].ob_spec & FILLCOL_MASK;
    }

    if tree[BGDESK as usize].ob_state & SELECTED != 0 {
        *curdesk = tree[s].ob_spec;
    } else {
        *curwin = tree[s].ob_spec;
    }
}

// ── Desktop-configuration dialog (`inf_conf`, `deskinf.c:925-1246`) ─────────

/// `FIRST_FUNKEY` (`deskins.h:16`) — `a_funkey` numbering starts at 1.
const FIRST_FUNKEY: u8 = 1;
/// `NUM_FUNKEYS` (`deskins.h:18`) — F1…F20.
const NUM_FUNKEYS: usize = 20;

/// `inf_sset` (`util/optimize.c:103-112`) — `strlcpy(ted->te_ptext, pstr,
/// ted->te_txtlen)`: copy `src` into a `TEDINFO` text buffer, truncating to the
/// field's capacity and always NUL-terminating.
fn inf_sset(buf: &mut [u8], src: &[u8]) {
    let n = src.len().min(buf.len() - 1);
    buf[..n].copy_from_slice(&src[..n]);
    buf[n..].fill(0);
}

/// `inf_sget` (`util/optimize.c:119-126`) — `strcpy(pstr, ted->te_ptext)`: the
/// other direction, i.e. what the user left in a `TEDINFO` text buffer, up to
/// its NUL. The source copies into a caller buffer because it has only a
/// `char *`; here the buffer is a slice, so the same bytes are borrowed out of
/// it instead of copied.
fn inf_sget(buf: &[u8]) -> &[u8] {
    let n = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    &buf[..n]
}

/// One function key the desktop knows about: its number (1…20) and the
/// application path bound to it — EmuTOS's `pa[]` array of `ANODE` pointers
/// (`get_all_funkeys`, `deskinf.c:928-948`).
struct FunKey {
    num: u8,
    path: String,
}

/// `get_all_funkeys` (`deskinf.c:928-948`) — the installed applications that
/// carry a function key, indexed by `a_funkey - FIRST_FUNKEY`.
///
/// EmuTOS walks `G.g_ahead`, the one list that holds both desktop icons and
/// installed-application associations. NewTOS splits those two ([`Anode`]'s
/// doc-comment): the association half — the half that can carry `a_funkey` at
/// all — is this module's [`SESSION`] model, so that is what is walked here.
/// The `NEWDESK.INF` extra block round-trips the number ([`InfAnode::funkey`]),
/// so a file written by EmuTOS, or by a NewTOS that later learns to assign
/// them, shows up in this dialog.
fn get_all_funkeys() -> [Option<FunKey>; NUM_FUNKEYS] {
    const NONE: Option<FunKey> = None;
    let mut out = [NONE; NUM_FUNKEYS];
    let guard = SESSION.borrow();
    let Some(data) = guard.as_ref() else {
        return out;
    };
    for a in &data.anodes {
        if a.is_desk {
            continue;
        }
        let i = a.funkey.wrapping_sub(FIRST_FUNKEY) as usize;
        if a.funkey >= FIRST_FUNKEY && i < NUM_FUNKEYS {
            out[i] = Some(FunKey { num: a.funkey, path: a.pappl.clone() });
        }
    }
    out
}

/// Options → Desktop configuration (`inf_conf`, `deskinf.c:1096-1246`,
/// `CONF_WITH_DESKTOP_CONFIG` — on by default,
/// `include/config.h:1774-1776`, and therefore on for our Falcon030/TOS4
/// profile).
///
/// Two radio groups, two independent "step through a list" panels and a
/// free-RAM read-out, in `ADDESKCF`'s order:
///
/// * **Default dir** — out in DESK05 for want of a consumer, back since
///   DESK18. `DCDEFAPP`/`DCDEFWIN` select the current directory a
///   *non-installed* program is launched with, through `G.g_appdir`
///   ([`global::DeskState::g_appdir`]); the consumer is [`super::main`]'s
///   `set_launch_dir` → [`super::pro::set_default_path`], which is
///   `do_aopen`'s own step (`desksupp.c:893-916`). An *installed* application
///   ignores it and uses its own `INF_AT_APPDIR` flag, which is why the switch
///   says nothing about those.
/// * **Parameter** — out in DESK05 for want of a consumer, back since DESK19.
///   `DCPMFULL`/`DCPMFILE` select what a file dropped on a program icon is
///   passed as, through `G.g_fullpath`
///   ([`global::DeskState::g_fullpath`]); the consumer is [`super::dir`]'s
///   `build_selected_path` (`deskfun.c:903-928`), reached from `fun_win2win`'s
///   drop-on-an-application branch (`:958-970`). A **document** launch answers
///   the same question from the installed application's own `INF_AT_ISFULL`
///   flag instead, so the switch says nothing about those either.
/// * **Function keys** — a *viewer*. `DCFUNPRV`/`DCFUNNXT` walk the installed
///   applications that have a function key bound; `DCFUNLT`/`DCFUNRT` scroll
///   the path sideways inside the fixed-width field (`scroll_conf_funkeys`,
///   `:952-969`). Nothing here assigns a key — EmuTOS assigns them in the
///   Install-Application dialog (`deskins.c` `APFUNKEY`), and the display is
///   read-only there too. With no key bound the panel shows its empty template,
///   which is exactly what EmuTOS shows on a fresh `EMUDESK.INF`.
/// * **Menu shortcut keys** — the editor. `DCMNUPRV`/`DCMNUNXT` step through
///   the [`super::menu::NUM_SHORTCUTS`] shortcut slots, showing the menu item's
///   text and its current key; the key is typed into `DCMNUKEY` and committed
///   by `save_shortcut` when the slot is left or the dialog OK'd, with the
///   duplicate-key question of `:1027-1057`. `DCMNUCLR` clears all 33 at once,
///   with no confirmation (`:1218-1222`).
/// * **Free RAM** — `Mxalloc(-1)` per pool (`:1119-1124`).
///
/// On OK the shortcut array is rendered into the menu strings
/// (`install_shortcuts`); on any other exit it is rebuilt *from* them
/// (`recover_shortcuts`), which undoes the edits. Like EmuTOS, nothing here
/// writes `NEWDESK.INF` — the new shortcuts reach the file through
/// *Save desktop*, which now emits the `#K` line.
///
/// With *Parameter* back, every object of `ADDESKCF` (`desk_rsc.c:1603-1772`)
/// is in the tree and the numbering below is the resource's own.
pub(super) fn inf_conf() {
    use super::menu;

    // ── object indices (`desk_rsc.h:191-211`) ─────────────────────────────
    /// `DCDEFDIR` — the unnamed `G_IBOX` (`ADDESKCF` object 3) the two
    /// *Default dir* radio buttons hang under; the AES deselects the siblings
    /// of the button that was hit through **this** object (see [`G_IBOX`]).
    const DCDEFDIR: i16 = 3;
    /// `DCDEFAPP` — *Application*: the program's own directory. It is the
    /// first of the pair, so `inf_which(tree, DCDEFAPP, 2)` yields 1 for it and
    /// 0 for its neighbour — which is `G.g_appdir` (`deskinf.c:1226`).
    const DCDEFAPP: usize = 4;
    /// `DCPARM` — the unnamed `G_IBOX` (`ADDESKCF` object 7) of the *Parameter*
    /// pair. Its **own** `G_IBOX`, not `DCDEFDIR`'s: `RBUTTON` deselection is
    /// per parent (`tos-api/tests/aes_desk_prefs_radio.rs`), so two groups
    /// under one box would clear each other.
    const DCPARM: i16 = 7;
    /// `DCPMFULL` — *Full path*: the dropped file's whole path. First of the
    /// pair, so `inf_which(tree, DCPMFULL, 2)` is `G.g_fullpath`
    /// (`deskinf.c:1227`).
    const DCPMFULL: usize = 8;
    /// `DCFUNBOX` — the panel holding the function-key number, path and the two
    /// scroll arrows; the field `draw_fld` refreshes after a step (`:1163`).
    const DCFUNBOX: i16 = 11;
    /// `DCFUNNUM` — the `F__` number field.
    const DCFUNNUM: i16 = 12;
    /// `DCFUNLT` — scroll the path text left.
    const DCFUNLT: i16 = 13;
    /// `DCFUNPTH` — the scrollable path field.
    const DCFUNPTH: i16 = 14;
    /// `DCFUNRT` — scroll the path text right.
    const DCFUNRT: i16 = 15;
    /// `DCFUNPRV` — previous bound function key.
    const DCFUNPRV: i16 = 16;
    /// `DCFUNNXT` — next bound function key.
    const DCFUNNXT: i16 = 17;
    /// `DCMNUBOX` — the panel holding the menu-item text, its key and *Clear
    /// all*.
    const DCMNUBOX: i16 = 19;
    /// `DCMNUKEY` — the editable `Key: ^_` field.
    const DCMNUKEY: i16 = 20;
    /// `DCMNUCLR` — clear every shortcut.
    const DCMNUCLR: i16 = 21;
    /// `DCMNUTXT` — the menu item the current slot names.
    const DCMNUTXT: i16 = 22;
    /// `DCMNUPRV` — previous shortcut slot.
    const DCMNUPRV: i16 = 23;
    /// `DCMNUNXT` — next shortcut slot.
    const DCMNUNXT: i16 = 24;
    /// `DCFREMEM` — the free-RAM read-out.
    const DCFREMEM: i16 = 25;
    /// `DCOK` — the OK button; `DC_CNCL` follows it.
    const DCOK: i16 = 26;
    /// `DC_CNCL` — Cancel.
    const DC_CNCL: i16 = 27;

    // ── geometry ────────────────────────────────────────────────────────
    //
    // `ADDESKCF`'s own, character cell for character cell (`desk_rsc.c:1603-
    // 1772`, `rsrc_obfix`-resolved at 8x16): a 39x22-cell box whose rows are
    // title 1, *Default dir* 3, *Parameter* 5, *Function keys* 7, the function
    // panel 8+6px, *Menu shortcut keys* 11+8px, the shortcut panel 12+14px,
    // free RAM 17+8px, the buttons 20. With DESK19 every one of those rows is
    // in the tree, so the box is `ADDESKCF`'s full 39x22 and no row is shifted
    // — which also restores the blank character row the resource leaves
    // between *Parameter* and *Function keys*. That spacing matters: every 3D
    // gadget here is grown by `ADJ3DSTD` (2 px per side) and then laid out one
    // pixel of border wider still (`aes/gemoblib.c:504`, `sst_th` `G_BUTTON`
    // → `-1`), so the 8 px a blank cell is worth is nearly eaten by the growth.
    let dw = 39 * gl_wchar(); // 312
    let dh = 22 * gl_hchar(); // 352
    let y_defdir = 3 * gl_hchar(); // 48
    let y_parm = 5 * gl_hchar(); // 80
    let y_fun_lbl = 7 * gl_hchar(); // 112
    let y_fun_box = 8 * gl_hchar() + 6; // 134
    let y_mnu_lbl = 11 * gl_hchar() + 8; // 184
    let y_mnu_box = 12 * gl_hchar() + 14; // 206
    let y_freemem = 17 * gl_hchar() + 8; // 280
    let y_btn = 20 * gl_hchar(); // 320
    // `adjust_3d_positions` (`deskmain.c:1647-1656`): "avoid button overlap" —
    // the two `PRV` gadgets sit flush against their `NXT` neighbour in the
    // resource, so a 3D build moves them left by `3 * ADJ3DSTD`.
    let x_prv = 30 * gl_wchar() - 3 * ADJ3DSTD; // 234
    let x_nxt = 34 * gl_wchar(); // 272

    // ── strings ─────────────────────────────────────────────────────────
    static TITLE: &[u8] = b"DESKTOP CONFIGURATION\0";
    // `ADDESKCF`'s own labels, unpadded: a `G_BUTTON` centres its text in the
    // object rectangle, and the rectangles below are the resource's.
    static DEFDIR_LBL: &[u8] = b"Default dir:\0";
    static DEFAPP: &[u8] = b"Application\0";
    static DEFWIN: &[u8] = b"Window\0";
    static PARM_LBL: &[u8] = b"Parameter:\0";
    static PMFULL: &[u8] = b"Full path\0";
    static PMFILE: &[u8] = b"File name\0";
    static FUN_LBL: &[u8] = b"Function keys:\0";
    static MNU_LBL: &[u8] = b"Menu shortcut keys:\0";
    static CLEAR_ALL: &[u8] = b"Clear all\0";
    // Characters 1-4 of the system font are the up / down / right / left
    // arrows — `ADDESKCF` labels its gadgets `"\001"`, `"\002"`, `"\003"`,
    // `"\004"` (`desk_rsc.c:1682,1697,1704,1710`).
    static UP: &[u8] = b"\x01\0";
    static DOWN: &[u8] = b"\x02\0";
    static RIGHT: &[u8] = b"\x03\0";
    static LEFT: &[u8] = b"\x04\0";
    static OK: &[u8] = b" OK \0";
    static CANCEL: &[u8] = b" Cancel \0";
    /// `STDUPCUT` (`desk_rsc.c:2563-2564`) — the duplicate-shortcut question.
    /// Two buttons, so it goes to `form_alert` verbatim rather than through
    /// `dir::fun_alert`'s one-OK-button wrapper.
    static STDUPCUT: &[u8] =
        b"[2][Last key is already assigned.|Do you want to reassign it?][  OK  |Cancel]\0";

    // ── TEDINFO templates + buffers (`desk_rs_tedinfo[28..32]`) ─────────
    static T_FUNNUM: &[u8] = b"F__\0";
    static T_FUNPTH: &[u8] = b"______________________\0";
    static T_MNUKEY: &[u8] = b"Key: ^_\0";
    static T_MNUTXT: &[u8] = b"________________________________\0";
    static T_FREMEM: &[u8] = b"Free RAM: _____________________\0";
    static V_DIGIT: &[u8] = b"9\0";
    static V_ANY: &[u8] = b"X\0";
    static V_ALPHA: &[u8] = b"a\0";

    /// `te_txtlen` of `DCFUNNUM` (`desk_rs_tedinfo[28]`).
    const FUNNUM_LEN: usize = 3;
    /// `te_txtlen` of `DCFUNPTH` (`[29]`) — the scroll window's width + NUL.
    const FUNPTH_LEN: usize = 23;
    /// `te_txtlen` of `DCMNUKEY` (`[30]`) — one character + NUL.
    const MNUKEY_LEN: usize = 2;
    /// `te_txtlen` of `DCMNUTXT` (`[31]`).
    const MNUTXT_LEN: usize = 33;
    /// `te_txtlen` of `DCFREMEM` (`[32]`).
    const FREMEM_LEN: usize = 22;

    let mut b_funnum = [0u8; FUNNUM_LEN];
    let mut b_funpth = [0u8; FUNPTH_LEN];
    let mut b_mnukey = [0u8; MNUKEY_LEN];
    let mut b_mnutxt = [0u8; MNUTXT_LEN];
    let mut b_fremem = [0u8; FREMEM_LEN];

    // `te_color` and `te_thickness` are the resource's own, per field: `4480`
    // (`0x1180`) is REPLACE-mode text on the panel face, `4352` (`0x1100`) the
    // TRANSPARENT one ([`TED_COLOR`]); a `te_thickness` of `-1` frames the
    // field, `0` leaves it unframed (`gr_crack` / `sst_th`).
    const TED_OPAQUE: i16 = 0x1180;
    let ted = |text: &mut [u8], tmpl: &[u8], valid: &[u8], just: i16, color: i16, th: i16| TedInfo {
        te_ptext: text.as_mut_ptr() as u64,
        te_ptmplt: tmpl.as_ptr() as u64,
        te_pvalid: valid.as_ptr() as u64,
        te_font: 3, // IBM (`obdefs.h:62`)
        te_junk1: 0,
        te_just: just,
        te_color: color,
        te_junk2: 0,
        te_thickness: th,
        te_txtlen: text.len() as i16,
        te_tmplen: tmpl.len() as i16,
    };
    let mut ted_funnum = ted(&mut b_funnum, T_FUNNUM, V_DIGIT, 2 /* TE_CNTR */, TED_OPAQUE, -1);
    let mut ted_funpth = ted(&mut b_funpth, T_FUNPTH, V_ANY, 0, TED_COLOR, 0);
    let mut ted_mnukey = ted(&mut b_mnukey, T_MNUKEY, V_ALPHA, 0, TED_OPAQUE, -1);
    let mut ted_mnutxt = ted(&mut b_mnutxt, T_MNUTXT, V_ANY, 0, TED_COLOR, -1);
    let mut ted_fremem = ted(&mut b_fremem, T_FREMEM, V_ANY, 0, TED_OPAQUE, 0);

    // `DCFUNBOX`/`DCMNUBOX` `ob_spec` = `16716032L` = `0xFF1100`
    // (`desk_rsc.c:1673,1721`): thickness byte `0xFF` = `th = -1`, a one-pixel
    // border **outside** the rectangle, over the hollow-white colour word that
    // lets [`FL3DBAK`] substitute the panel face (DESK01b).
    const PANEL_SPEC: u64 = 0x00FF_1100;

    #[rustfmt::skip]
    let mut tree = [
        // 0 ROOT
        mk_root(BOX_SPEC, dw, dh, DC_CNCL),
        // 1 title — `align_title` centres it on the box (`deskmain.c:1739-1747`).
        mk(G_STRING, 0, TITLE.as_ptr() as u64, (dw - 21 * gl_wchar()) / 2, gl_hchar(), 21 * gl_wchar(), gl_hchar(), 2, -1, -1),
        // 2 "Default dir:"      3 DCDEFDIR (its `G_IBOX`)   4/5 the buttons
        //
        // `ADDESKCF` objects 2-5 verbatim (`desk_rsc.c:1615-1637`): the label
        // at char (1,3) 13x1, the `G_IBOX` at (16,3) 22x1 carrying `ob_spec`
        // `4352L` = `0x1100` and no 3D flag, and inside it *Application* at
        // (0,0) 12x1 and *Window* at (13,0) 9x1, both `SELECTABLE | RBUTTON |
        // FL3DIND`.
        mk(G_STRING, 0, DEFDIR_LBL.as_ptr() as u64, gl_wchar(), y_defdir, 13 * gl_wchar(), gl_hchar(), DCDEFDIR, -1, -1),
        mk(G_IBOX, 0, TED_COLOR as u64, 16 * gl_wchar(), y_defdir, 22 * gl_wchar(), gl_hchar(), 6, DCDEFAPP as i16, DCDEFAPP as i16 + 1),
        mk(G_BUTTON, SELECTABLE | RBUTTON | FL3DIND, DEFAPP.as_ptr() as u64, 0, 0, 12 * gl_wchar(), gl_hchar(), DCDEFAPP as i16 + 1, -1, -1),
        mk(G_BUTTON, SELECTABLE | RBUTTON | FL3DIND, DEFWIN.as_ptr() as u64, 13 * gl_wchar(), 0, 9 * gl_wchar(), gl_hchar(), DCDEFDIR, -1, -1),
        // 6 "Parameter:"        7 DCPARM (its own `G_IBOX`)  8/9 the buttons
        //
        // `ADDESKCF` objects 6-9 verbatim (`desk_rsc.c:1641-1663`): the label
        // at char (1,5) 13x1, the `G_IBOX` at (16,5) 22x1 with `ob_spec`
        // `4352L` = `0x1100` and no 3D flag, and inside it *Full path* at
        // (0,0) 11x1 and *File name* at (12,0) 10x1, both `SELECTABLE |
        // RBUTTON | FL3DIND`.
        mk(G_STRING, 0, PARM_LBL.as_ptr() as u64, gl_wchar(), y_parm, 13 * gl_wchar(), gl_hchar(), DCPARM, -1, -1),
        mk(G_IBOX, 0, TED_COLOR as u64, 16 * gl_wchar(), y_parm, 22 * gl_wchar(), gl_hchar(), 10, DCPMFULL as i16, DCPMFULL as i16 + 1),
        mk(G_BUTTON, SELECTABLE | RBUTTON | FL3DIND, PMFULL.as_ptr() as u64, 0, 0, 11 * gl_wchar(), gl_hchar(), DCPMFULL as i16 + 1, -1, -1),
        mk(G_BUTTON, SELECTABLE | RBUTTON | FL3DIND, PMFILE.as_ptr() as u64, 12 * gl_wchar(), 0, 10 * gl_wchar(), gl_hchar(), DCPARM, -1, -1),
        // 10 "Function keys:"
        mk(G_STRING, 0, FUN_LBL.as_ptr() as u64, gl_wchar(), y_fun_lbl, 14 * gl_wchar(), gl_hchar(), DCFUNBOX, -1, -1),
        // 11 DCFUNBOX           12..15 its children
        mk(G_BOX, FL3DBAK, PANEL_SPEC, gl_wchar(), y_fun_box, 37 * gl_wchar(), gl_hchar() + 8, DCFUNPRV, DCFUNNUM, DCFUNRT),
        mk(G_FTEXT, FL3DBAK, &mut ted_funnum as *mut _ as u64, 4, 4, 3 * gl_wchar(), gl_hchar(), DCFUNLT, -1, -1),
        mk(G_BUTTON, TOUCHEXIT | FL3DACT, LEFT.as_ptr() as u64, 5 * gl_wchar() + 4, 4, 3 * gl_wchar() + 4, gl_hchar(), DCFUNPTH, -1, -1),
        mk(G_FBOXTEXT, FL3DBAK, &mut ted_funpth as *mut _ as u64, 10 * gl_wchar(), 4, 22 * gl_wchar(), gl_hchar(), DCFUNRT, -1, -1),
        mk(G_BUTTON, TOUCHEXIT | FL3DACT, RIGHT.as_ptr() as u64, 33 * gl_wchar(), 4, 3 * gl_wchar() + 4, gl_hchar(), DCFUNBOX, -1, -1),
        // 16/17 DCFUNPRV / DCFUNNXT
        mk(G_BUTTON, TOUCHEXIT | FL3DACT, UP.as_ptr() as u64, x_prv, y_fun_lbl, 4 * gl_wchar(), gl_hchar(), DCFUNNXT, -1, -1),
        mk(G_BUTTON, TOUCHEXIT | FL3DACT, DOWN.as_ptr() as u64, x_nxt, y_fun_lbl, 4 * gl_wchar(), gl_hchar(), 18, -1, -1),
        // 18 "Menu shortcut keys:"
        mk(G_STRING, 0, MNU_LBL.as_ptr() as u64, gl_wchar(), y_mnu_lbl, 19 * gl_wchar(), gl_hchar(), DCMNUBOX, -1, -1),
        // 19 DCMNUBOX           20..22 its children
        mk(G_BOX, FL3DBAK, PANEL_SPEC, gl_wchar(), y_mnu_box, 37 * gl_wchar(), 3 * gl_hchar() + 4, DCMNUPRV, DCMNUKEY, DCMNUTXT),
        mk(G_FTEXT, EDITABLE | FL3DBAK, &mut ted_mnukey as *mut _ as u64, 4, 2 * gl_hchar(), 15 * gl_wchar(), gl_hchar(), DCMNUCLR, -1, -1),
        mk(G_BUTTON, TOUCHEXIT | FL3DACT, CLEAR_ALL.as_ptr() as u64, 17 * gl_wchar() + 4, 2 * gl_hchar(), 19 * gl_wchar(), gl_hchar(), DCMNUTXT, -1, -1),
        mk(G_FBOXTEXT, 0, &mut ted_mnutxt as *mut _ as u64, 4, 4, 36 * gl_wchar(), gl_hchar(), DCMNUBOX, -1, -1),
        // 23/24 DCMNUPRV / DCMNUNXT
        mk(G_BUTTON, TOUCHEXIT | FL3DACT, UP.as_ptr() as u64, x_prv, y_mnu_lbl, 4 * gl_wchar(), gl_hchar(), DCMNUNXT, -1, -1),
        mk(G_BUTTON, TOUCHEXIT | FL3DACT, DOWN.as_ptr() as u64, x_nxt, y_mnu_lbl, 4 * gl_wchar(), gl_hchar(), DCFREMEM, -1, -1),
        // 25 DCFREMEM
        mk(G_FTEXT, FL3DBAK, &mut ted_fremem as *mut _ as u64, gl_wchar(), y_freemem, 36 * gl_wchar(), gl_hchar(), DCOK, -1, -1),
        // 26/27 OK / Cancel (`DCOK` is `SELECTABLE|DEFAULT|EXIT|FL3DACT`,
        // `DC_CNCL` the same without `DEFAULT`, `desk_rsc.c:1759-1771`).
        mk(G_BUTTON, SELECTABLE | DEFAULT | EXIT | FL3DACT, OK.as_ptr() as u64, 7 * gl_wchar(), y_btn, 9 * gl_wchar(), gl_hchar(), DC_CNCL, -1, -1),
        mk(G_BUTTON, SELECTABLE | EXIT | LASTOB | FL3DACT, CANCEL.as_ptr() as u64, 23 * gl_wchar(), y_btn, 9 * gl_wchar(), gl_hchar(), 0, -1, -1),
    ];
    // `DCFUNBOX`/`DCMNUBOX` carry `OUTLINED` like the ROOT (`desk_rsc.c:1672,
    // 1720`), which on an `FL3DBAK` panel is EmuTOS's `draw_3d_outline`.
    tree[DCFUNBOX as usize].ob_state = OUTLINED;
    tree[DCMNUBOX as usize].ob_state = OUTLINED;

    // ── pre-select the current state (`deskinf.c:1105-1112`) ───────────
    //
    // `deselect_all(tree)` is implicit: `mk` leaves every `ob_state` at 0 (the
    // two `OUTLINED` panels above are not `SELECTED`). Then
    // `if (G.g_appdir) tree[DCDEFAPP] |= SELECTED; else tree[DCDEFWIN] |= …`
    // and the same pair for `G.g_fullpath` (`deskinf.c:1113-1116`).
    let (appdir, fullpath) = global::with(|g| (g.g_appdir, g.g_fullpath)).unwrap_or((true, true));
    tree[DCDEFAPP + usize::from(!appdir)].ob_state |= SELECTED;
    tree[DCPMFULL + usize::from(!fullpath)].ob_state |= SELECTED;

    // ── free-RAM line (`deskinf.c:1118-1125`) ───────────────────────────
    // `sprintf("%ld KB", dos_avail_stram()/1024)`, plus `" + %ld KB"` of
    // alt-RAM when the machine has any (`has_alt_ram`; here: a non-zero
    // `Mxalloc(-1, MX_TTRAM)`). The trailing blanks are EmuTOS's
    // `memset(G.g_work+n, ' ', 40)` — enough to fill the object's text field.
    {
        let mut work = [b' '; FREMEM_LEN - 1];
        let mut p = 0usize;
        push_dec(&mut work, &mut p, (newtos_gem::gemdos::dos_avail_stram() / 1024) as u64);
        push_str(&mut work, &mut p, b" KB");
        let alt = newtos_gem::gemdos::dos_avail_altram();
        if alt > 0 {
            push_str(&mut work, &mut p, b" + ");
            push_dec(&mut work, &mut p, (alt / 1024) as u64);
            push_str(&mut work, &mut p, b" KB");
        }
        work[p..].fill(b' ');
        inf_sset(&mut b_fremem, &work);
    }

    // ── function-key setup (`deskinf.c:1127-1143`) ──────────────────────
    let funkeys = get_all_funkeys();
    // `textpos`/`maxpos`, EmuTOS's two file-scope scroll variables (`:950`).
    let mut textpos: i16 = 0;
    let mut maxpos: i16 = 0;
    // `init_conf_funkeys` (`:974-991`): the number into `DCFUNNUM`, the path
    // into `DCFUNPTH` at scroll position 0.
    let init_funkeys = |fk: &FunKey, b_num: &mut [u8], b_pth: &mut [u8], textpos: &mut i16, maxpos: &mut i16| {
        let mut num = [b' '; FUNNUM_LEN - 1];
        let mut p = 0usize;
        push_dec(&mut num, &mut p, fk.num as u64);
        inf_sset(b_num, &num);
        *maxpos = (fk.path.len() as i16 - FUNPTH_LEN as i16 + 1).max(0);
        *textpos = 0;
        inf_sset(b_pth, fk.path.as_bytes());
    };
    // `scroll_conf_funkeys` (`:957-969`) — returns whether the text moved.
    let scroll_funkeys = |fk: &FunKey, b_pth: &mut [u8], textpos: &mut i16, maxpos: i16, amount: i16| -> bool {
        let newpos = *textpos + amount;
        if newpos < 0 || newpos > maxpos {
            return false;
        }
        *textpos = newpos;
        inf_sset(b_pth, &fk.path.as_bytes()[newpos as usize..]);
        true
    };

    // "find lowest function key" (`:1136-1142`).
    let mut current_funkey = funkeys.iter().position(Option::is_some).unwrap_or(usize::MAX);
    if let Some(fk) = funkeys.get(current_funkey).and_then(Option::as_ref) {
        init_funkeys(fk, &mut b_funnum, &mut b_funpth, &mut textpos, &mut maxpos);
    } else {
        current_funkey = 0; // "ensure legal value for array index" (`:1143`)
    }

    // ── menu-shortcut setup (`deskinf.c:1145-1154`) ─────────────────────
    // `init_conf_shortcuts` (`:997-1018`): the slot's key into `DCMNUKEY`, and
    // the menu item's text — minus its two leading blanks and minus the
    // `SHORTCUT_SIZE` reserve at its right edge — into `DCMNUTXT`.
    let init_shortcuts = |n: usize, b_key: &mut [u8], b_txt: &mut [u8]| {
        let key = menu::shortcut_key(n);
        inf_sset(b_key, &[if key == 0 { b' ' } else { key }]);
        let mut work = [b' '; MNUTXT_LEN - 1];
        menu::shortcut_item_text(n, &mut work);
        inf_sset(b_txt, &work);
    };
    // `save_shortcut` (`:1027-1057`): commit the edited key into the array,
    // asking before stealing it from another item.
    let save_shortcut = |n: usize, b_key: &[u8]| {
        let mut key = b_key[0];
        key.make_ascii_uppercase();
        if key.is_ascii_uppercase() {
            for i in 0..menu::NUM_SHORTCUTS {
                if i == n {
                    continue; // "ignore ourselves"
                }
                if menu::shortcut_key(i) == key {
                    if super::form_alert(1, STDUPCUT.as_ptr()) != 1 {
                        return; // user cancelled — leave both slots alone
                    }
                    menu::set_shortcut_key(i, 0);
                    break;
                }
            }
        } else {
            key = 0;
        }
        menu::set_shortcut_key(n, key);
    };

    // "find lowest available menu item" (`:1147-1154`).
    let mut current_shortcut = (0..menu::NUM_SHORTCUTS)
        .find(|&i| menu::shortcut_item(i).is_some())
        .unwrap_or(0);
    init_shortcuts(current_shortcut, &mut b_mnukey, &mut b_mnutxt);

    // ── interact (`deskinf.c:1156-1240`) ────────────────────────────────
    let tree_ptr = tree.as_mut_ptr() as u64;
    start_dialog(tree_ptr);
    let mut exitobj;
    loop {
        let mut redraw: i16 = -1; // "by default, no redraw"
        exitobj = super::form_do(tree_ptr, ROOT) & 0x7fff;
        let mut done = false;
        match exitobj {
            // "look for next lower" / "next higher" (`:1160-1186`).
            DCFUNPRV | DCFUNNXT => {
                let found = if exitobj == DCFUNPRV {
                    (0..current_funkey).rev().find_map(|i| funkeys[i].as_ref().map(|fk| (i, fk)))
                } else {
                    (current_funkey + 1..NUM_FUNKEYS).find_map(|i| funkeys[i].as_ref().map(|fk| (i, fk)))
                };
                if let Some((i, fk)) = found {
                    current_funkey = i;
                    init_funkeys(fk, &mut b_funnum, &mut b_funpth, &mut textpos, &mut maxpos);
                    redraw = DCFUNBOX;
                }
            }
            DCFUNLT | DCFUNRT => {
                let amount = if exitobj == DCFUNLT { -1 } else { 1 };
                if let Some(fk) = funkeys[current_funkey].as_ref() {
                    if scroll_funkeys(fk, &mut b_funpth, &mut textpos, maxpos, amount) {
                        redraw = DCFUNPTH;
                    }
                }
            }
            DCMNUPRV => {
                save_shortcut(current_shortcut, &b_mnukey);
                if let Some(i) = (0..current_shortcut).rev().find(|&i| menu::shortcut_item(i).is_some()) {
                    current_shortcut = i;
                    init_shortcuts(i, &mut b_mnukey, &mut b_mnutxt);
                    redraw = DCMNUBOX;
                }
            }
            DCMNUNXT => {
                save_shortcut(current_shortcut, &b_mnukey);
                if let Some(i) =
                    (current_shortcut + 1..menu::NUM_SHORTCUTS).find(|&i| menu::shortcut_item(i).is_some())
                {
                    current_shortcut = i;
                    init_shortcuts(i, &mut b_mnukey, &mut b_mnutxt);
                    redraw = DCMNUBOX;
                }
            }
            DCMNUCLR => {
                menu::set_shortcuts([0; menu::NUM_SHORTCUTS]);
                inf_sset(&mut b_mnukey, b"");
                redraw = DCMNUBOX;
            }
            DCOK => {
                save_shortcut(current_shortcut, &b_mnukey);
                // `G.g_appdir = inf_which(tree, DCDEFAPP, 2);` (`:1226`) —
                // 1 = *Application* (the first of the pair), 0 = *Window*.
                let chosen = inf_which(&tree, DCDEFAPP, 2) == 1;
                // `G.g_fullpath = inf_which(tree, DCPMFULL, 2);` (`:1227`) —
                // 1 = *Full path*, 0 = *File name*.
                let full = inf_which(&tree, DCPMFULL, 2) == 1;
                global::with(|g| {
                    g.g_appdir = chosen;
                    g.g_fullpath = full;
                });
                done = true;
            }
            DC_CNCL => done = true,
            _ => {}
        }
        // `tree[exitobj].ob_state &= ~SELECTED;` (`:1232`).
        if let Some(o) = tree.get_mut(exitobj as usize) {
            o.ob_state &= !SELECTED;
        }
        if done {
            break;
        }
        if redraw >= 0 {
            super::dir::draw_fld(tree_ptr, redraw);
        }
    }

    // `if (exitobj == DCOK) install_shortcuts(); else recover_shortcuts();`
    // (`:1236-1241`) — commit the array into the menu strings, or rebuild it
    // from them and so discard every edit made in this dialog.
    if exitobj == DCOK {
        menu::install_shortcuts();
    } else {
        menu::recover_shortcuts();
    }

    end_dialog(tree_ptr);
}

// ── Open-application dialog (`opn_appl`, `deskinf.c:1250-1271`) ─────────────

/// `opn_appl` (EmuTOS `deskinf.c:1250-1271`) — *"Open application icon"*: the
/// `ADOPENAP` box that asks a parameter-taking program for its command tail
/// before `pro_run` starts it.
///
/// ```c
/// tree = desk_rs_trees[ADOPENAP];
///
/// set_tedinfo_name(tree, APPLNAME, papname);
/// inf_sset(tree, APPLPARM, "");
/// inf_show(tree, APPLPARM);
///
/// /* now find out what happened */
/// if (inf_what(tree, APPLOK))
/// {
///     inf_sget(tree, APPLPARM, ptail);
///     return TRUE;
/// }
///
/// return FALSE;
/// ```
///
/// `papname` is the program's bare file name — `do_aopen`'s `pname`
/// (`desksupp.c:984`) — and it is shown through [`set_tedinfo_name`], i.e.
/// split into the `________.___` template like every other name field of the
/// desktop. `ptail` receives the typed line; EmuTOS's caller points it at
/// `G.g_work + 1`, where [`super::pro::pro_run`] turns it into the Atari
/// command-tail image. Bytes rather than a `String` because the field's
/// `te_pvalid` is `X`, which admits every character the AES's `check` lets
/// through (`tos-api/src/aes/object.rs`), including the ones above 0x7E that
/// are not UTF-8 — the tail is handed on as the bytes the user typed.
///
/// Returns EmuTOS's `WORD`: TRUE → run the program with this tail, FALSE →
/// the user cancelled and nothing is started. **[`inf_what`]'s third answer
/// counts as TRUE**: `if (inf_what(tree, APPLOK))` is a plain non-zero test,
/// so the `-1` it gives when neither button ended up `SELECTED` launches too.
/// That is the source's own arithmetic, kept rather than tidied.
///
/// ## The tree
///
/// `ADOPENAP` object for object (`desk_rsc.c:877-919`, `desk_rsc.h:93-98`) —
/// seven objects, its numbering, its labels, its two `G_FBOXTEXT` template
/// fields, its measurements resolved the way `rsrc_obfix` resolves them
/// (`fix_chpos`, `aes/gemrslib.c:53-83`: `LOBYTE * cell + (signed)HIBYTE`).
/// The ROOT's packed height `2058` is `(8 << 8) | 10` = 10 rows + 8 px and the
/// buttons' packed `2056` is row 8 + 8 px, so at 8 x 16 the box is 320 x 168
/// with the buttons at y 136..152 — room for the 2 px per side a 3D activator
/// grows by before it is drawn.
pub(super) fn opn_appl(papname: &str, ptail: &mut Vec<u8>) -> bool {
    // ── object numbers (`desk_rsc.h:93-98`) ──────────────────────────────
    //   0 ROOT · 1 APPLOPEN · 2 APPLNAME · 3 (the `Parameters:` label, which
    //   the resource header does not name) · 4 APPLPARM · 5 APPLOK
    //   6 APPLCNCL
    const APPLNAME: i16 = 2;
    /// `ADOPENAP` object 3, the `Parameters:` label. `desk_rsc.h` gives it no
    /// name — it lists only `APPLOPEN`/`APPLNAME`/`APPLPARM`/`APPLOK`/
    /// `APPLCNCL` — because nothing in `deskinf.c` addresses it.
    const APPLPLBL: i16 = 3;
    const APPLPARM: i16 = 4;
    const APPLOK: usize = 5;
    const APPLCNCL: i16 = 6;

    // ── strings: `ADOPENAP`'s own (`desk_rsc.c:877-919`) and the templates of
    //    `desk_rs_tedinfo[12]`/`[13]` (`:62-70`), character for character.
    static TITLE: &[u8] = b"OPEN APPLICATION\0";
    static PARAMS: &[u8] = b"Parameters:\0";
    static OK: &[u8] = b"OK\0";
    static CANCEL: &[u8] = b"Cancel\0";
    /// `desk_rs_tedinfo[12]`'s template, `rs_str_Name______________`
    /// (`desk_rsc.c:18`) — `te_tmplen` 19, so 18 characters and the NUL.
    static T_NAME: &[u8] = b"Name: ________.___\0";
    /// `desk_rs_tedinfo[13]`'s template — 38 underscores, `te_tmplen` 39.
    static T_PARM: &[u8] = b"______________________________________\0";
    static V_FNAME: &[u8] = b"f\0";
    static V_ANY: &[u8] = b"X\0";

    // The two text buffers, each exactly its field's `te_txtlen`
    // (`desk_rs_tedinfo[12]` 12, `[13]` 39).
    let mut b_name = [0u8; 12];
    let mut b_parm = [0u8; 39];

    // `set_tedinfo_name(tree, APPLNAME, papname);` and
    // `inf_sset(tree, APPLPARM, "");`
    set_tedinfo_name(&mut b_name, papname.as_bytes());
    inf_sset(&mut b_parm, b"");

    let mut ted_name = rsc_ted(&mut b_name, T_NAME, V_FNAME, TE_LEFT);
    let mut ted_parm = rsc_ted(&mut b_parm, T_PARM, V_ANY, TE_LEFT);

    // ── geometry: `ADOPENAP`'s own, cell for cell. 40 x (10 cells + 8 px).
    let dw = 40 * gl_wchar(); // 320
    let dh = 10 * gl_hchar() + 8; // 168
    let col = gl_wchar(); // char column 1 — both fields and the label
    let y_btn = 8 * gl_hchar() + 8; // 136

    // 3D roles follow `ADOPENAP` object for object and the table in DESK01b:
    // the ROOT is `FL3DBAK` + `OUTLINED`, both fields are `FL3DBAK`,
    // `APPLOK`/`APPLCNCL` are `FL3DBAK | FL3DIND` = [`FL3DACT`], the two
    // `G_STRING`s carry none.
    #[rustfmt::skip]
    let mut tree = [
        // 0 ROOT — `ob_spec` 69888 = `(1 << 16) | 0x1100` ([`BOX_SPEC`]).
        mk_root(BOX_SPEC, dw, dh, APPLCNCL),
        // 1 APPLOPEN — a `G_STRING` one row down, so `align_title` centres it.
        mk(G_STRING, 0, TITLE.as_ptr() as u64, 11 * gl_wchar(), gl_hchar(), 16 * gl_wchar(), gl_hchar(), APPLNAME, -1, -1),
        // 2 APPLNAME — the program's name, 21 cells wide on char row 3.
        mk(G_FBOXTEXT, FL3DBAK, &mut ted_name as *mut _ as u64, col, 3 * gl_hchar(), 21 * gl_wchar(), gl_hchar(), APPLPLBL, -1, -1),
        // 3 the `Parameters:` label, char row 5.
        mk(G_STRING, 0, PARAMS.as_ptr() as u64, col, 5 * gl_hchar(), 11 * gl_wchar(), gl_hchar(), APPLPARM, -1, -1),
        // 4 APPLPARM — the one `EDITABLE` field of the box, 38 cells on row 6.
        mk(G_FBOXTEXT, EDITABLE | FL3DBAK, &mut ted_parm as *mut _ as u64, col, 6 * gl_hchar(), 38 * gl_wchar(), gl_hchar(), APPLOK as i16, -1, -1),
        // 5/6 APPLOK / APPLCNCL, nine cells wide at char columns 7 and 24.
        mk(G_BUTTON, SELECTABLE | DEFAULT | EXIT | FL3DACT, OK.as_ptr() as u64, 7 * gl_wchar(), y_btn, 9 * gl_wchar(), gl_hchar(), APPLCNCL, -1, -1),
        mk(G_BUTTON, SELECTABLE | EXIT | LASTOB | FL3DACT, CANCEL.as_ptr() as u64, 24 * gl_wchar(), y_btn, 9 * gl_wchar(), gl_hchar(), 0, -1, -1),
    ];
    align_title(&mut tree, TITLE.len() as i16 - 1);

    // `inf_show(tree, APPLPARM);` — the **start field**, not `ROOT`: the edit
    // cursor opens in the parameter line, which is the only editable object
    // of the box (`fm_inifld`, `aes/gemfmlib.c:153-160`).
    inf_show(tree.as_mut_ptr() as u64, APPLPARM);

    // `if (inf_what(tree, APPLOK)) { inf_sget(tree, APPLPARM, ptail); return TRUE; }`
    if inf_what(&mut tree, APPLOK) != 0 {
        ptail.clear();
        ptail.extend_from_slice(inf_sget(&b_parm));
        return true;
    }
    false
}

// ── Round-trip self-test (verification (b); not run at boot) ─────────────────

/// Save→read round-trip verification against the live SD image (mirrors the
/// other `selftest_*` modules): build a representative [`InfData`], round-trip
/// it (1) purely in memory and (2) through `NEWDESK.INF` on disk, and confirm
/// the parsed-back state matches. Prints one line per check (console,
/// [`super::selftest`]); run by [`super::selftest::run_once`] in a
/// `selftest-desk` build only. **Mutates the SD image** — run only against a
/// fresh `/tmp/sd.img`.
pub fn selftest() {
    use super::selftest::tprintln;

    tprintln!("[desk-inf] NEWDESK.INF round-trip self-test on C:\\ (mutates the SD image)");
    let mut pass = 0u32;
    let mut fail = 0u32;
    let mut check = |name: &str, cond: bool| {
        if cond {
            pass += 1;
            tprintln!("[desk-inf]  PASS {}", name);
        } else {
            fail += 1;
            tprintln!("[desk-inf]  FAIL {}", name);
        }
    };

    // A representative state: text view, sort by size, custom background, two
    // desktop-icon positions, one window, and the default file-type anodes.
    let mut original = InfData::default();
    // `appdir = false` is the non-default *Default dir* setting and
    // `fullpath = false` the non-default *Parameter* one (the `.INF` default
    // `INF_E5_DEFAULT` has both `INF_E5_APPDIR` and `INF_E5_ISFULL` set), so
    // the round trip exercises both bits in their cleared state (DESK18/DESK19).
    let (e1, e5) = state_to_env(View::Text, Sort::Size, false, false, false, original.e1, original.e5);
    // …and a non-default value for every Preferences field, so the `#E` bytes
    // the dialog writes are exercised too (DESK02/DESK13): confirm-deletes off,
    // confirm-copies on, confirm-overwrites off (i.e. `INF_E2_ALLOWOVW` set —
    // the one inverted bit), double-click 4, 12-hour clock, day-before-month.
    let (e1, e2) = prefs_to_env(false, true, false, 4, TimeFmt::H12, DateFmt::Dmy, e1, original.e2);
    original.e1 = e1;
    original.e2 = e2;
    original.e5 = e5;
    original.patcol = [(0x51, 0x40), (0x63, 0x41), (0x72, 0x0A)];
    original.windows.push(InfWindow { hsl: 0, vsl: 0, xc: 2, yc: 6, wc: 0x26, hc: 0x0C, path: String::from("C:\\") });
    original.anodes.push(InfAnode {
        tag: b'M',
        is_desk: true,
        has_extra: false,
        xspot: 3,
        yspot: 5,
        aicon: -1,
        dicon: -1,
        letter: b'C',
        pappl: String::from("Disk C"),
        pdata: String::new(),
        flags: 0,
        funkey: 0,
        pargs: String::new(),
    });
    original.anodes.push(InfAnode {
        tag: b'T',
        is_desk: true,
        has_extra: false,
        xspot: 7,
        yspot: 1,
        aicon: -1,
        dicon: -1,
        letter: 0,
        pappl: String::from("Trash"),
        pdata: String::new(),
        flags: 0,
        funkey: 0,
        pargs: String::new(),
    });
    original.anodes.extend(InfData::default_type_anodes());

    // (1) In-memory round-trip: serialize → parse → compare. The buffer starts
    // empty rather than behind the `CPDATA_LEN` prefix `app_save` reserves —
    // this is the file grammar under test, not the shell-buffer layout.
    let mut bytes = Vec::new();
    original.serialize_into(&mut bytes);
    let mem = parse_inf(&bytes);
    check("in-memory: #E view/sort/appdir/fullpath", env_to_state(mem.e1, mem.e5) == env_to_state(original.e1, original.e5));
    check(
        "in-memory: #E preferences",
        env_to_prefs(mem.e1, mem.e2) == (false, true, false, 4, TimeFmt::H12, DateFmt::Dmy),
    );
    check("in-memory: #Q background", mem.patcol == original.patcol);
    check("in-memory: #W geometry", mem.windows == original.windows);
    check("in-memory: ANODE positions", mem.anodes == original.anodes);
    check("in-memory: full equality", mem == original);

    // (2) Disk round-trip: write NEWDESK.INF → read it back → parse → compare.
    let wrote = write_inf_file(&bytes);
    check("disk: write NEWDESK.INF", wrote);
    match read_inf_file() {
        Some(back) => {
            let disk = parse_inf(&back);
            check("disk: #E view/sort/appdir/fullpath", env_to_state(disk.e1, disk.e5) == env_to_state(original.e1, original.e5));
            check(
                "disk: #E preferences",
                env_to_prefs(disk.e1, disk.e2) == (false, true, false, 4, TimeFmt::H12, DateFmt::Dmy),
            );
            check("disk: #Q background", disk.patcol == original.patcol);
            check("disk: #W geometry", disk.windows == original.windows);
            check("disk: ANODE positions", disk.anodes == original.anodes);
            check("disk: full equality", disk == original);
        }
        None => check("disk: read NEWDESK.INF back", false),
    }

    tprintln!("[desk-inf] round-trip self-test: {} passed, {} failed", pass, fail);
}

/// Install-Application association self-test (deskins #20): install an
/// application association, persist it to `NEWDESK.INF`, read it back off disk,
/// and confirm the document→application lookup resolves correctly. Prints the
/// written `NEWDESK.INF` and one line per check (console, [`super::selftest`];
/// run by [`super::selftest::run_once`] in a `selftest-desk` build only).
/// **Mutates the SD image** — run only against a fresh `/tmp/sd.img`.
/// Verifies the parts of the feature that do
/// not need a keyboard (the dialog text entry is real-hardware-only).
pub fn selftest_install() {
    use super::selftest::tprintln;

    tprintln!("[desk-ins] Install-Application association self-test on C:\\ (mutates the SD image)");
    let mut pass = 0u32;
    let mut fail = 0u32;
    let mut check = |name: &str, cond: bool| {
        if cond {
            pass += 1;
            tprintln!("[desk-ins]  PASS {}", name);
        } else {
            fail += 1;
            tprintln!("[desk-ins]  FAIL {}", name);
        }
    };

    // Start from a clean model, then install HELLO.NCP (a real native console
    // program on the test image) as the handler for *.TXT documents, passing the
    // document's full path (AF_ISFULL) so the persisted line carries the extra
    // flags block, and giving it function key 7 (DESK20). A .NCP is a
    // TOS-console flavour → the #F tag.
    *SESSION.borrow_mut() = None;
    install_application("C:\\HELLO.NCP", "TXT", AppFlavor::Tos, true, false, 7, "");

    // The in-memory model resolves the association immediately.
    let mem = lookup_association(b"README.TXT");
    check("mem: README.TXT resolves", mem.as_ref().map(|a| a.app_path.as_str()) == Some("C:\\HELLO.NCP"));
    check("mem: full-path flag set", mem.as_ref().map(|a| a.full_path) == Some(true));
    check("mem: HELLO.BIN has no association", lookup_association(b"HELLO.BIN").is_none());
    check("mem: funkey 7 stored", installed_doctype("C:\\HELLO.NCP").map(|a| a.funkey) == Some(7));
    // `get_funkey`'s duplicate walk: another application asking for 7 sees it
    // taken, the owner itself does not.
    check("mem: funkey 7 is taken by another app", funkey_owner(7, "C:\\OTHER.NCP"));
    check("mem: funkey 7 is not taken by its owner", !funkey_owner(7, "C:\\HELLO.NCP"));

    // Persist to disk, then dump the written NEWDESK.INF so the #F line is visible.
    let wrote = app_save(true);
    check("disk: write NEWDESK.INF", wrote);
    if let Some(bytes) = read_inf_file() {
        tprintln!("[desk-ins] ---- NEWDESK.INF ({} bytes) ----", bytes.len());
        for line in bytes.split(|&b| b == b'\n') {
            let end = line.iter().position(|&b| b == b'\r').unwrap_or(line.len());
            if end == 0 {
                continue;
            }
            tprintln!("[desk-ins] | {}", core::str::from_utf8(&line[..end]).unwrap_or("<non-utf8>"));
        }
        tprintln!("[desk-ins] ---- end NEWDESK.INF ----");

        // Reload purely from the disk bytes (fresh parse into the session), then
        // confirm the association survived the write→read round-trip.
        let reparsed = parse_inf(&bytes);
        *SESSION.borrow_mut() = Some(reparsed);
        let disk = lookup_association(b"README.TXT");
        check("disk: README.TXT resolves to HELLO.NCP", disk.as_ref().map(|a| a.app_path.as_str()) == Some("C:\\HELLO.NCP"));
        check("disk: full-path flag survived", disk.as_ref().map(|a| a.full_path) == Some(true));
        check("disk: contains the #F HELLO.NCP line", contains_subslice(&bytes, b"#F 06 07 C:\\HELLO.NCP@ *.TXT@ 207 @"));
        check("disk: funkey 7 survived", installed_doctype("C:\\HELLO.NCP").map(|a| a.funkey) == Some(7));
        check("disk: INSIDE.TXT (any dir) resolves", lookup_association(b"INSIDE.TXT").is_some());
        check("disk: HELLO.NCP is not its own document", lookup_association(b"HELLO.NCP").is_none());

        // Removal round-trip.
        check("model: remove HELLO.NCP association", remove_application("C:\\HELLO.NCP"));
        check("model: README.TXT no longer resolves", lookup_association(b"README.TXT").is_none());
    } else {
        check("disk: read NEWDESK.INF back", false);
    }

    tprintln!("[desk-ins] association self-test: {} passed, {} failed", pass, fail);
}

/// Whether `hay` contains the contiguous byte sequence `needle` (self-test aid).
fn contains_subslice(hay: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || needle.len() > hay.len() {
        return needle.is_empty();
    }
    hay.windows(needle.len()).any(|w| w == needle)
}

/// Set-background self-test (DESK64): the `ADBKGND` tree [`inf_backgrounds`]
/// builds, checked against `desk_rsc.c:2230-2452` at an 8 x 16 cell — the
/// sibling links, the `ob_spec` words of the swatches and the sample, the 3D
/// flags, the resolved geometry, the `HIDETREE` per plane count
/// (`deskinf.c:832-852`) — and the sample arithmetic of one loop turn
/// ([`bg_turn`], `deskinf.c:875-893`). Pure: no AES call, nothing on `C:`.
/// Run by [`super::selftest::run_once`] in a `selftest-desk` build only.
// NEWTOS: self-test (DESK64); EmuTOS's desktop has no self-tests.
pub fn selftest_backgrounds() {
    use super::selftest::tprintln;

    tprintln!("[desk-bkgnd] ADBKGND / inf_backgrounds self-test");
    let mut pass = 0u32;
    let mut fail = 0u32;
    let mut check = |name: &str, cond: bool| {
        if cond {
            pass += 1;
            tprintln!("[desk-bkgnd]  PASS {}", name);
        } else {
            fail += 1;
            tprintln!("[desk-bkgnd]  FAIL {}", name);
        }
    };

    let t = adbkgnd_tree(8, 16, 16, false);

    // Sibling chains: every parent's children run head → … → tail and the
    // tail's `ob_next` points back at the parent (`objc_add` shape).
    let chain_ok = |parent: usize, head: i16, tail: i16| {
        let mut o = head;
        let mut n = 0;
        while o != tail && n < ADBKGND_NOBS {
            o = t[o as usize].ob_next;
            n += 1;
        }
        o == tail && t[parent].ob_head == head && t[parent].ob_tail == tail && t[tail as usize].ob_next == parent as i16
    };
    check("links: ROOT 1..36", chain_ok(0, 1, BGCANCEL) && t[BGCANCEL as usize].ob_flags & LASTOB != 0);
    check("links: radio box 3 = 4..5", chain_ok(3, BGDESK, BGWIN));
    check("links: pattern box 8 = 9..16", chain_ok(8, BGPAT0, BGPAT7));
    check("links: colour box 18 = 19..34", chain_ok(18, BGCOL0, BGCOL15));

    // `ob_spec` words, as the resource writes them (decimal).
    check("ob_spec: ROOT 69888", t[0].ob_spec == 69888);
    check("ob_spec: panels 3/8/18 4352", [3, 8, 18].iter().all(|&i| t[i].ob_spec == 4352));
    check("ob_spec: BGPAT0 16716032, BGPAT1 16716049, BGPAT7 16716145",
        t[9].ob_spec == 16716032 && t[10].ob_spec == 16716049 && t[16].ob_spec == 16716145);
    check("ob_spec: BGSAMPLE 16716033", t[17].ob_spec == 16716033);
    check("ob_spec: BGCOL0 16716144, BGCOL15 16716159", t[19].ob_spec == 16716144 && t[34].ob_spec == 16716159);

    // Flags and states (`desk_rsc.c:2232-2452`).
    check("flags: ROOT FL3DBAK + OUTLINED", t[0].ob_flags == FL3DBAK && t[0].ob_state == OUTLINED);
    check("flags: BGDESK/BGWIN SELECTABLE|EXIT|RBUTTON|FL3DIND",
        [BGDESK, BGWIN].iter().all(|&i| t[i as usize].ob_flags == SELECTABLE | EXIT | RBUTTON | FL3DIND));
    check("state: BGDESK SELECTED, BGWIN NORMAL", t[4].ob_state == SELECTED && t[5].ob_state == 0);
    check("flags: swatches TOUCHEXIT only",
        (BGPAT0..=BGPAT7).chain(BGCOL0..=BGCOL15).all(|i| t[i as usize].ob_flags == TOUCHEXIT));
    check("flags: BGSAMPLE NONE", t[17].ob_flags == 0);
    check("flags: BGOK SELECTABLE|DEFAULT|EXIT|FL3DACT", t[35].ob_flags == SELECTABLE | DEFAULT | EXIT | FL3DACT);
    check("flags: BGCANCEL SELECTABLE|EXIT|LASTOB|FL3DACT", t[36].ob_flags == SELECTABLE | EXIT | LASTOB | FL3DACT);

    // Geometry at 8 x 16 (`rsrc_obfix`).
    let geo = |i: usize| (t[i].ob_x, t[i].ob_y, t[i].ob_width, t[i].ob_height);
    check("geometry: ROOT 304x256", geo(0) == (0, 0, 304, 256));
    check("geometry: title centred (304-14*8)/2 = 96, row 1", geo(1) == (96, 16, 112, 16));
    check("geometry: Style:/Color: at row 6 + 4 px", t[6].ob_y == 100 && t[7].ob_y == 100 && t[7].ob_x == 192);
    check("geometry: BGSAMPLE 136,128 32x32", geo(17) == (136, 128, 32, 32));
    check("geometry: BGPAT5 3,16 in its box", geo(14) == (24, 16, 24, 16));
    check("geometry: BGCOL15 72,48 in its box", geo(34) == (72, 48, 24, 16));
    check("geometry: BGOK 56,224 72x16, BGCANCEL 176,224", geo(35) == (56, 224, 72, 16) && geo(36) == (176, 224, 72, 16));

    // HIDETREE per plane count (`deskinf.c:832-852`).
    let hidden = |t: &[Object; ADBKGND_NOBS]| {
        (BGCOL0..=BGCOL15).filter(|&i| t[i as usize].ob_flags & HIDETREE != 0).count()
    };
    let t1 = adbkgnd_tree(8, 16, 1, false);
    let t2 = adbkgnd_tree(8, 16, 2, false);
    check("HIDETREE: 16 planes none, 1 plane BGCOL2..15, 2 planes BGCOL4..15",
        hidden(&t) == 0 && hidden(&t1) == 14 && t1[20].ob_flags & HIDETREE == 0 && t1[21].ob_flags & HIDETREE != 0
            && hidden(&t2) == 12 && t2[22].ob_flags & HIDETREE == 0 && t2[23].ob_flags & HIDETREE != 0);
    let tw = adbkgnd_tree(8, 16, 16, true);
    check("radio carried over: Window selected", tw[4].ob_state == 0 && tw[5].ob_state == SELECTED);

    // One loop turn at a time (`deskinf.c:875-893`).
    let mut t = t;
    let (mut curdesk, mut curwin) = (0x1143u64, 0x1140u64);
    t[17].ob_spec = curdesk | SAMPLE_BORDER_FLAGS;
    bg_turn(&mut t, BGPAT0 + 2, &mut curdesk, &mut curwin);
    check("turn: pattern 2 → sample/curdesk 0xff1123", t[17].ob_spec == 0xff_1123 && curdesk == 0xff_1123 && curwin == 0x1140);
    bg_turn(&mut t, BGCOL0 + 5, &mut curdesk, &mut curwin);
    check("turn: colour 5 → 0xff1125", t[17].ob_spec == 0xff_1125 && curdesk == 0xff_1125);
    bg_turn(&mut t, (0x8000u16 as i16) | BGPAT0, &mut curdesk, &mut curwin);
    check("turn: double-click bit changes nothing", t[17].ob_spec == 0xff_1125 && curdesk == 0xff_1125);
    // `form_do` moves the radio before it returns `BGWIN`.
    t[4].ob_state = 0;
    t[5].ob_state = SELECTED;
    bg_turn(&mut t, BGWIN, &mut curdesk, &mut curwin);
    check("turn: Window → sample = curwin, border flags gone", t[17].ob_spec == 0x1140 && curwin == 0x1140);
    bg_turn(&mut t, BGCOL0 + 3, &mut curdesk, &mut curwin);
    check("turn: colour 3 on Window → curwin 0x1143, curdesk kept",
        curwin == 0x1143 && curdesk == 0xff_1125);

    tprintln!("[desk-bkgnd] ADBKGND self-test: {} passed, {} failed", pass, fail);
}
