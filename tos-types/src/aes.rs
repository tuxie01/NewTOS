//! GEM AES on-wire types and the one public set of AES/object constants.
//!
//! Spec: [`docs/api/structures/object.md`](../../../docs/api/structures/object.md),
//! [`tedinfo.md`](../../../docs/api/structures/tedinfo.md),
//! [`iconblk.md`](../../../docs/api/structures/iconblk.md),
//! [`docs/api/aes/`](../../../docs/api/aes/). Source of truth: EmuTOS
//! `include/obdefs.h` (object model) and `include/aesdefs.h` (AES call
//! constants); every value below carries its `file:line` there.
//!
//! **One home (DESK43 U5b).** These names used to be private copies in every
//! module that needed them — `G_STRING = 28` stood eleven times in the tree.
//! The AES server (`tos-api`), the kernel self-tests, the desktop and the
//! native Rust program runtime (`programs/newtos-gem`) all take them from
//! here; none of them declares its own. This crate has no dependencies, so
//! it builds for the kernel (EL1) and for a position-independent EL0 program
//! alike.
//!
//! **Native layout.** NewTOS is LP64: `OBJECT.ob_spec` and the three leading
//! pointers of `TEDINFO`/`ICONBLK` are 64-bit (COMPATIBILITY "Native vs m68k
//! struct variants"). The ILP32 reading of an app's tree (24-byte `OBJECT`)
//! stays in the AES (`tos-api` `aes::object::ObjTree`); the structs here are
//! the LP64 records, pinned by the `const` assertions at the end of this file.
//!
//! Widths follow the field each constant goes into: object types, flags and
//! states are `u16` (`ob_type`/`ob_flags`/`ob_state`), the window `kind` bits
//! `u16`, the `evnt_multi` `MU_*` mask `u16`; message ids, `wind_get`/`wind_set`
//! fields and every other `intin[]` selector are `i16` (a `WORD`).

// ── OBJECT tree model (`obdefs.h:141-201`) ─────────────────────────────

/// A GEM object tree node (`OBJECT`, `obdefs.h:141-154`), native NewTOS
/// layout: 32 bytes, `ob_spec` a 64-bit value at offset 16.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Object {
    /// next sibling, or `-1` (`NIL`).
    pub ob_next: i16,
    /// first child, or `-1`.
    pub ob_head: i16,
    /// last child, or `-1`.
    pub ob_tail: i16,
    /// `G_*` graphic type.
    pub ob_type: u16,
    /// `*` flag bits.
    pub ob_flags: u16,
    /// `*` state bits.
    pub ob_state: u16,
    /// type-specific payload: a native pointer (string/TEDINFO) or, for
    /// box types, `(thickness << 16) | colour-word`.
    pub ob_spec: u64,
    /// x relative to the parent.
    pub ob_x: i16,
    /// y relative to the parent.
    pub ob_y: i16,
    /// width in pixels.
    pub ob_width: i16,
    /// height in pixels.
    pub ob_height: i16,
}

/// `TEDINFO` — editable/formatted text for the `G_TEXT`/`G_BOXTEXT`
/// object types (`obdefs.h:170-183`). Pointers are native 64-bit.
#[repr(C)]
pub struct TedInfo {
    /// pointer to the text string.
    pub te_ptext: u64,
    /// pointer to the template string.
    pub te_ptmplt: u64,
    /// pointer to the validation string.
    pub te_pvalid: u64,
    /// font id.
    pub te_font: i16,
    /// reserved.
    pub te_junk1: i16,
    /// justification (0 = left, 1 = right, 2 = centre).
    pub te_just: i16,
    /// colour word (`gr_crack` encoding).
    pub te_color: i16,
    /// reserved.
    pub te_junk2: i16,
    /// border thickness.
    pub te_thickness: i16,
    /// text length.
    pub te_txtlen: i16,
    /// template length.
    pub te_tmplen: i16,
}

/// `ICONBLK` — a monochrome icon (mask + data + optional char + text)
/// for a `G_ICON` object (`obdefs.h:185-201`). Pointers are native 64-bit.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct IconBlk {
    /// pointer to the mask raster (16-bit words).
    pub ib_pmask: u64,
    /// pointer to the data raster (16-bit words).
    pub ib_pdata: u64,
    /// pointer to the icon's text string.
    pub ib_ptext: u64,
    /// colour/char word: `[15:12]` fg, `[11:8]` bg, `[7:0]` character.
    pub ib_char: i16,
    /// character x/y.
    pub ib_xchar: i16,
    /// character y.
    pub ib_ychar: i16,
    /// icon image rect x/y/w/h (relative to the object).
    pub ib_xicon: i16,
    /// icon y.
    pub ib_yicon: i16,
    /// icon width.
    pub ib_wicon: i16,
    /// icon height.
    pub ib_hicon: i16,
    /// text rect x/y/w/h (relative to the object).
    pub ib_xtext: i16,
    /// text y.
    pub ib_ytext: i16,
    /// text width.
    pub ib_wtext: i16,
    /// text height.
    pub ib_htext: i16,
}

/// A GEM rectangle (`GRECT`, `obdefs.h:156-162`): x/y/w/h in device pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Grect {
    /// Left edge.
    pub x: i16,
    /// Top edge.
    pub y: i16,
    /// Width in pixels.
    pub w: i16,
    /// Height in pixels.
    pub h: i16,
}

impl Grect {
    /// The intersection of `self` and `o`, or `None` when they do not
    /// overlap (EmuTOS `rc_intersect`, `aes/gemrslib.c`/`optimize.c`).
    pub fn intersect(&self, o: &Grect) -> Option<Grect> {
        let x0 = self.x.max(o.x);
        let y0 = self.y.max(o.y);
        let x1 = (self.x + self.w).min(o.x + o.w);
        let y1 = (self.y + self.h).min(o.y + o.h);
        if x1 > x0 && y1 > y0 {
            Some(Grect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 })
        } else {
            None
        }
    }
}

// ── tree indices (`obdefs.h:20-21`) ──────────────────────────────────────

/// `ROOT` — the root object of a tree (`obdefs.h:20`); also `form_do`'s "no
/// editable field preselected" start value.
pub const ROOT: i16 = 0;
/// `NIL` — "no object" link / "no window" (`obdefs.h:21`).
pub const NIL: i16 = -1;

// ── AES inside-pattern indices (`obdefs.h:32-39`) ───────────────────────

/// `IP_HOLLOW` — hollow fill (`obdefs.h:32`).
pub const IP_HOLLOW: u16 = 0;
/// `IP_4PATT` — the 50 % dither; `just_draw` lays it over a `DISABLED`
/// object (`obdefs.h:36`, `gemoblib.c:928-929`).
pub const IP_4PATT: u16 = 4;
/// `IP_SOLID` — solid fill (`obdefs.h:39`).
pub const IP_SOLID: u16 = 7;

// ── ob_type graphic types (`obdefs.h:67-80`) ────────────────────────────

/// `G_BOX` (`obdefs.h:67`).
pub const G_BOX: u16 = 20;
/// `G_TEXT` (`obdefs.h:68`).
pub const G_TEXT: u16 = 21;
/// `G_BOXTEXT` (`obdefs.h:69`).
pub const G_BOXTEXT: u16 = 22;
/// `G_IMAGE` (`obdefs.h:70`).
pub const G_IMAGE: u16 = 23;
/// `G_USERDEF` — application-drawn object, `USERBLK` + `ub_code` callback (`obdefs.h:71`).
pub const G_USERDEF: u16 = 24;
/// `G_IBOX` — invisible box (`obdefs.h:72`). Draws nothing; its job is to be
/// the parent of a radio-button group, because `fm_button` deselects the
/// siblings of the hit button by walking the parent's child chain
/// (`aes/gemfmlib.c` `select_radio`).
pub const G_IBOX: u16 = 25;
/// `G_BUTTON` (`obdefs.h:73`).
pub const G_BUTTON: u16 = 26;
/// `G_BOXCHAR` (`obdefs.h:74`).
pub const G_BOXCHAR: u16 = 27;
/// `G_STRING` (`obdefs.h:75`) — the low byte of `ob_type`; a submenu can only
/// be attached to a `G_STRING` menu item (`gemmnext.c:539-541`).
pub const G_STRING: u16 = 28;
/// `G_FTEXT` — formatted text: a `TEDINFO` drawn through its template,
/// without a box (`obdefs.h:76`).
pub const G_FTEXT: u16 = 29;
/// `G_FBOXTEXT` — the same as [`G_FTEXT`], boxed (`obdefs.h:77`).
pub const G_FBOXTEXT: u16 = 30;
/// `G_ICON` (`obdefs.h:78`).
pub const G_ICON: u16 = 31;
/// `G_TITLE` (`obdefs.h:79`).
pub const G_TITLE: u16 = 32;
/// `G_CICON` — colour icon: a `CICONBLK` that begins with a mono `ICONBLK` (`obdefs.h:80`).
pub const G_CICON: u16 = 33;

// ── ob_flags (`obdefs.h:83-98`) ─────────────────────────────────────────

/// `SELECTABLE` (`obdefs.h:83`).
pub const SELECTABLE: u16 = 0x0001;
/// `DEFAULT` — the default (heavy-frame) object (`obdefs.h:84`).
pub const DEFAULT: u16 = 0x0002;
/// `EXIT` — an exit button; thickens the border by 1 (`obdefs.h:85`).
pub const EXIT: u16 = 0x0004;
/// `EDITABLE` (`obdefs.h:86`).
pub const EDITABLE: u16 = 0x0008;
/// `RBUTTON` — radio button: selecting it deselects every other `RBUTTON`
/// child of the same parent (`obdefs.h:87`).
pub const RBUTTON: u16 = 0x0010;
/// `LASTOB` — marks the final object of a tree (`obdefs.h:88`).
pub const LASTOB: u16 = 0x0020;
/// `TOUCHEXIT` — the object ends `form_do` on the button *down*, without
/// being selected (`obdefs.h:89`).
pub const TOUCHEXIT: u16 = 0x0040;
/// `HIDETREE` (`obdefs.h:90`).
pub const HIDETREE: u16 = 0x0080;
/// `INDIRECT` — `ob_spec` points at a cell holding the real spec (`obdefs.h:91`).
pub const INDIRECT: u16 = 0x0100;
/// `FL3DOBJ` — the "indicator OR activator" bit: set on IND/ACT, clear on BAK (`obdefs.h:92`).
pub const FL3DOBJ: u16 = 0x0200;
/// `FL3DMASK` (`obdefs.h:93`).
pub const FL3DMASK: u16 = 0x0600;
/// `FL3DIND` — 3D indicator: grown by [`ADJ3DSTD`], bevelled, changes colour
/// when pressed (`obdefs.h:95`).
pub const FL3DIND: u16 = 0x0200;
/// `FL3DACT` — 3D activator: grown and bevelled like an indicator, but keeps
/// its colour and nudges its text when pressed (`obdefs.h:96`; EmuTOS
/// resources spell it `FL3DBAK | FL3DIND`, the same bit pattern).
pub const FL3DACT: u16 = 0x0600;
/// `FL3DBAK` — 3D background panel: not grown and not bevelled (no
/// [`FL3DOBJ`] bit), but a hollow-white interior takes the `objc_sysvar`
/// `BACKGRCOL` face (`obdefs.h:97`).
pub const FL3DBAK: u16 = 0x0400;
/// `SUBMENU` — the menu item has a submenu attached; `menu_attach` sets it,
/// detaching clears it (`obdefs.h:98`).
pub const SUBMENU: u16 = 0x0800;

// ── ob_state (`obdefs.h:101-107`) ───────────────────────────────────────

/// `SELECTED` — selected / highlighted; the object renderer inverts it
/// (`obdefs.h:101`).
pub const SELECTED: u16 = 0x0001;
/// `CROSSED` — a diagonal cross over the object (`obdefs.h:102`).
pub const CROSSED: u16 = 0x0002;
/// `CHECKED` (`obdefs.h:103`).
pub const CHECKED: u16 = 0x0004;
/// `DISABLED` — greyed, not selectable (`obdefs.h:104`).
pub const DISABLED: u16 = 0x0008;
/// `OUTLINED` (`obdefs.h:105`). On an [`FL3DBAK`] panel the AES draws
/// `draw_3d_outline` instead of the flat double frame (`aes/gemoblib.c:869-887`);
/// `ob_center` grows the rectangle it reports by 3 px per side for it
/// (`aes/gemobed.c:81-93`).
pub const OUTLINED: u16 = 0x0010;
/// `SHADOWED` — solid drop shadow to the lower right (`obdefs.h:106`).
pub const SHADOWED: u16 = 0x0020;
/// `WHITEBAK` — skip a white icon background, `gr_gicon` (`obdefs.h:107`).
pub const WHITEBAK: u16 = 0x0040;

// ── objc_sysvar selectors and 3D adjustments (`obdefs.h:129-137`) ──────

/// `objc_sysvar` `LK3DIND` (`obdefs.h:129`).
pub const LK3DIND: i16 = 1;
/// `objc_sysvar` `LK3DACT` (`obdefs.h:130`).
pub const LK3DACT: i16 = 2;
/// `objc_sysvar` `INDBUTCOL` (`obdefs.h:131`).
pub const INDBUTCOL: i16 = 3;
/// `objc_sysvar` `ACTBUTCOL` (`obdefs.h:132`).
pub const ACTBUTCOL: i16 = 4;
/// `objc_sysvar` `BACKGRCOL` (`obdefs.h:133`).
pub const BACKGRCOL: i16 = 5;
/// `objc_sysvar` `AD3DVALUE` (`obdefs.h:134`).
pub const AD3DVALUE: i16 = 6;
/// `ADJ3DSTD` — standard pixel adjustment of a 3D object (`obdefs.h:136`): a
/// 3D object is drawn this much larger on every side than it measures
/// (`just_draw`, `gemoblib.c:504-509`; `CONF_WITH_3D_OBJECTS`, `config.h:1259`).
pub const ADJ3DSTD: i16 = 2;
/// `ADJ3DOUT` — extra extent of an `OUTLINED` 3D object (`obdefs.h:137`).
pub const ADJ3DOUT: i16 = 3;
/// `ADJ3DSHA` — per-pixel-of-border extent of a `SHADOWED` 3D object's drop
/// shadow (`obdefs.h:138`), applied by `ob_actxywh` (`gemoblib.c:331-336`).
pub const ADJ3DSHA: i16 = 2;

// ── objc_edit kinds (`obdefs.h:248-251`, EmuTOS spells them `EDSTART`…) ─

/// `objc_edit` start (EmuTOS `EDSTART`, `obdefs.h:248`).
pub const ED_START: i16 = 0;
/// `objc_edit` init (EmuTOS `EDINIT`, `obdefs.h:249`).
pub const ED_INIT: i16 = 1;
/// `objc_edit` char (EmuTOS `EDCHAR`, `obdefs.h:250`).
pub const ED_CHAR: i16 = 2;
/// `objc_edit` end (EmuTOS `EDEND`, `obdefs.h:251`).
pub const ED_END: i16 = 3;

// ── TEDINFO.te_just (`obdefs.h:253-255`) ────────────────────────────────
//
// The justification a `TEDINFO`'s text is merged into its template with
// (`ob_format`) and placed in its object rectangle with (`gr_just`,
// `aes/gemgraf.c:567-589`).

/// `TE_LEFT` (`obdefs.h:253`).
pub const TE_LEFT: i16 = 0;
/// `TE_RIGHT` (`obdefs.h:254`).
pub const TE_RIGHT: i16 = 1;
/// `TE_CNTR` (`obdefs.h:255`).
pub const TE_CNTR: i16 = 2;

// ── messages (`aesdefs.h:227-243`) ──────────────────────────────────────

/// `MN_SELECTED` (`aesdefs.h:227`).
pub const MN_SELECTED: i16 = 10;
/// `WM_REDRAW` (`aesdefs.h:229`).
pub const WM_REDRAW: i16 = 20;
/// `WM_TOPPED` (`aesdefs.h:230`).
pub const WM_TOPPED: i16 = 21;
/// `WM_CLOSED` (`aesdefs.h:231`).
pub const WM_CLOSED: i16 = 22;
/// `WM_FULLED` (`aesdefs.h:232`).
pub const WM_FULLED: i16 = 23;
/// `WM_ARROWED` (`aesdefs.h:233`).
pub const WM_ARROWED: i16 = 24;
/// `WM_HSLID` (`aesdefs.h:234`).
pub const WM_HSLID: i16 = 25;
/// `WM_VSLID` (`aesdefs.h:235`).
pub const WM_VSLID: i16 = 26;
/// `WM_SIZED` (`aesdefs.h:236`).
pub const WM_SIZED: i16 = 27;
/// `WM_MOVED` (`aesdefs.h:237`).
pub const WM_MOVED: i16 = 28;
/// `WM_UNTOPPED` (`aesdefs.h:239`).
pub const WM_UNTOPPED: i16 = 30;
/// `AC_OPEN` (`aesdefs.h:242`).
pub const AC_OPEN: i16 = 40;
/// `AC_CLOSE` (`aesdefs.h:243`) — the one message type the Event Manager
/// reacts to itself, by setting `AP_ACCLOSE` on the receiving descriptor
/// (`gemsuper.c:139-140,160-161`).
pub const AC_CLOSE: i16 = 41;

// ── WM_ARROWED actions (`aesdefs.h:246-253`) ────────────────────────────

/// `WA_UPPAGE` (`aesdefs.h:246`).
pub const WA_UPPAGE: i16 = 0;
/// `WA_DNPAGE` (`aesdefs.h:247`).
pub const WA_DNPAGE: i16 = 1;
/// `WA_UPLINE` (`aesdefs.h:248`).
pub const WA_UPLINE: i16 = 2;
/// `WA_DNLINE` (`aesdefs.h:249`).
pub const WA_DNLINE: i16 = 3;
/// `WA_LFPAGE` (`aesdefs.h:250`).
pub const WA_LFPAGE: i16 = 4;
/// `WA_RTPAGE` (`aesdefs.h:251`).
pub const WA_RTPAGE: i16 = 5;
/// `WA_LFLINE` (`aesdefs.h:252`).
pub const WA_LFLINE: i16 = 6;
/// `WA_RTLINE` (`aesdefs.h:253`).
pub const WA_RTLINE: i16 = 7;

// ── evnt_multi event mask (`aesdefs.h:256-262`) ─────────────────────────

/// `MU_KEYBD` (`aesdefs.h:256`).
pub const MU_KEYBD: u16 = 0x0001;
/// `MU_BUTTON` (`aesdefs.h:257`).
pub const MU_BUTTON: u16 = 0x0002;
/// `MU_M1` — the first mouse-rectangle event (`aesdefs.h:258`).
pub const MU_M1: u16 = 0x0004;
/// `MU_M2` (`aesdefs.h:259`).
pub const MU_M2: u16 = 0x0008;
/// `MU_MESAG` (`aesdefs.h:260`).
pub const MU_MESAG: u16 = 0x0010;
/// `MU_TIMER` (`aesdefs.h:261`).
pub const MU_TIMER: u16 = 0x0020;
/// `MU_TOSVALID` — the bits valid for TOS compatibility (`aesdefs.h:262`).
pub const MU_TOSVALID: u16 = 0x003F;

// ── form_dial modes (`aesdefs.h:339-342`) ───────────────────────────────

/// `FMD_START` — reserve the dialog screen area (`aesdefs.h:339`).
pub const FMD_START: i16 = 0;
/// `FMD_GROW` (`aesdefs.h:340`).
pub const FMD_GROW: i16 = 1;
/// `FMD_SHRINK` (`aesdefs.h:341`).
pub const FMD_SHRINK: i16 = 2;
/// `FMD_FINISH` — release the dialog screen area and restore what was under
/// it (`aesdefs.h:342`).
pub const FMD_FINISH: i16 = 3;

// ── graf_mouse numbers (`aesdefs.h:424-438`) ────────────────────────────

/// `ARROW` — the first preset form (`aesdefs.h:424`).
pub const ARROW: i16 = 0;
/// `OUTLN_CROSS` — the last preset form (`aesdefs.h:431`).
pub const OUTLN_CROSS: i16 = 7;
/// `USER_DEF` — install the caller's `MFORM` (`aesdefs.h:432`).
pub const USER_DEF: i16 = 255;
/// `M_OFF` (`aesdefs.h:433`).
pub const M_OFF: i16 = 256;
/// `M_ON` (`aesdefs.h:434`).
pub const M_ON: i16 = 257;
/// `M_SAVE` — save the current form in the AESPD (`aesdefs.h:436`).
pub const M_SAVE: i16 = 258;
/// `M_RESTORE` — restore the saved form from the AESPD (`aesdefs.h:437`).
pub const M_RESTORE: i16 = 259;
/// `M_PREVIOUS` — restore the previously used global form (`aesdefs.h:438`).
pub const M_PREVIOUS: i16 = 260;

// ── window kind bits (`aesdefs.h:519-531`) ──────────────────────────────

/// `NAME` (`aesdefs.h:519`).
pub const NAME: u16 = 0x0001;
/// `CLOSER` (`aesdefs.h:520`).
pub const CLOSER: u16 = 0x0002;
/// `FULLER` (`aesdefs.h:521`).
pub const FULLER: u16 = 0x0004;
/// `MOVER` (`aesdefs.h:522`).
pub const MOVER: u16 = 0x0008;
/// `INFO` (`aesdefs.h:523`).
pub const INFO: u16 = 0x0010;
/// `SIZER` (`aesdefs.h:524`).
pub const SIZER: u16 = 0x0020;
/// `UPARROW` (`aesdefs.h:525`).
pub const UPARROW: u16 = 0x0040;
/// `DNARROW` (`aesdefs.h:526`).
pub const DNARROW: u16 = 0x0080;
/// `VSLIDE` (`aesdefs.h:527`).
pub const VSLIDE: u16 = 0x0100;
/// `LFARROW` (`aesdefs.h:528`).
pub const LFARROW: u16 = 0x0200;
/// `RTARROW` (`aesdefs.h:529`).
pub const RTARROW: u16 = 0x0400;
/// `HSLIDE` (`aesdefs.h:530`).
pub const HSLIDE: u16 = 0x0800;
/// `HOTCLOSE` — PC-GEM feature (`aesdefs.h:531`, `CONF_WITH_PCGEM`): the closer
/// fires `WM_CLOSED` on the press, without `gr_watchbox`'s wait for the
/// release (`gemctrl.c:208-214`). Needs [`CLOSER`] for a closer to exist.
pub const HOTCLOSE: u16 = 0x1000;

// ── wind_get/wind_set fields (`aesdefs.h:535-552`) ──────────────────────

/// `WF_NAME` (`aesdefs.h:535`).
pub const WF_NAME: i16 = 2;
/// `WF_INFO` (`aesdefs.h:536`).
pub const WF_INFO: i16 = 3;
/// `WF_WXYWH` — work area (gemlib `WF_WORKXYWH`, `aesdefs.h:537`).
pub const WF_WXYWH: i16 = 4;
/// `WF_CXYWH` — current border rectangle (gemlib `WF_CURRXYWH`, `aesdefs.h:538`).
pub const WF_CXYWH: i16 = 5;
/// `WF_PXYWH` — previous border rectangle (`aesdefs.h:539`).
pub const WF_PXYWH: i16 = 6;
/// `WF_FXYWH` — full border rectangle (`aesdefs.h:540`).
pub const WF_FXYWH: i16 = 7;
/// `WF_HSLIDE` (`aesdefs.h:541`).
pub const WF_HSLIDE: i16 = 8;
/// `WF_VSLIDE` (`aesdefs.h:542`).
pub const WF_VSLIDE: i16 = 9;
/// `WF_TOP` (`aesdefs.h:543`).
pub const WF_TOP: i16 = 10;
/// `WF_FIRSTXYWH` (`aesdefs.h:544`).
pub const WF_FIRSTXYWH: i16 = 11;
/// `WF_NEXTXYWH` (`aesdefs.h:545`).
pub const WF_NEXTXYWH: i16 = 12;
/// `WF_NEWDESK` (`aesdefs.h:546`).
pub const WF_NEWDESK: i16 = 14;
/// `WF_HSLSIZ` (`aesdefs.h:547`).
pub const WF_HSLSIZ: i16 = 15;
/// `WF_VSLSIZ` (`aesdefs.h:548`).
pub const WF_VSLSIZ: i16 = 16;
/// `WF_SCREEN` (`aesdefs.h:549`).
pub const WF_SCREEN: i16 = 17;
/// `WF_COLOR` (`aesdefs.h:550`).
pub const WF_COLOR: i16 = 18;
/// `WF_DCOLOR` (`aesdefs.h:551`).
pub const WF_DCOLOR: i16 = 19;
/// `WF_OWNER` (`aesdefs.h:552`).
pub const WF_OWNER: i16 = 20;

// ── wind_calc types, wind_update modes (`aesdefs.h:555-562`) ────────────

/// `WC_BORDER` (`aesdefs.h:555`).
pub const WC_BORDER: i16 = 0;
/// `WC_WORK` (`aesdefs.h:556`).
pub const WC_WORK: i16 = 1;
/// `END_UPDATE` — release the window-tree lock (`aesdefs.h:559`).
pub const END_UPDATE: i16 = 0;
/// `BEG_UPDATE` — take the window-tree lock, so the AES holds its redraws
/// back while the caller rearranges windows (`aesdefs.h:560`).
pub const BEG_UPDATE: i16 = 1;
/// `END_MCTRL` (`aesdefs.h:561`).
pub const END_MCTRL: i16 = 2;
/// `BEG_MCTRL` (`aesdefs.h:562`).
pub const BEG_MCTRL: i16 = 3;

/// `DESKWH` — the desktop's window handle (`aes/gemwmlib.h:13`,
/// `desk/deskbind.h:30`): the bottom of the z-order and a real window —
/// `wm_init` runs `w_setup(ppd, DESKWH, NONE)` on it (`gemwmlib.c:1290`).
pub const DESKWH: i16 = 0;

// ── layout assertions (native LP64 records) ────────────────────────────
//
// The AES reads LP64 trees at exactly these offsets (`tos-api`
// `aes::object::ObjTree`, `aes::resource` `OB_SPEC_OFF_LP64 = 16`); a field
// reorder or width change here would silently misread every native tree.

const _: () = {
    use core::mem::{align_of, offset_of, size_of};
    assert!(size_of::<Object>() == 32 && align_of::<Object>() == 8);
    assert!(offset_of!(Object, ob_next) == 0);
    assert!(offset_of!(Object, ob_head) == 2);
    assert!(offset_of!(Object, ob_tail) == 4);
    assert!(offset_of!(Object, ob_type) == 6);
    assert!(offset_of!(Object, ob_flags) == 8);
    assert!(offset_of!(Object, ob_state) == 10);
    assert!(offset_of!(Object, ob_spec) == 16);
    assert!(offset_of!(Object, ob_x) == 24);
    assert!(offset_of!(Object, ob_y) == 26);
    assert!(offset_of!(Object, ob_width) == 28);
    assert!(offset_of!(Object, ob_height) == 30);

    assert!(size_of::<TedInfo>() == 40 && align_of::<TedInfo>() == 8);
    assert!(offset_of!(TedInfo, te_ptext) == 0);
    assert!(offset_of!(TedInfo, te_ptmplt) == 8);
    assert!(offset_of!(TedInfo, te_pvalid) == 16);
    assert!(offset_of!(TedInfo, te_font) == 24);
    assert!(offset_of!(TedInfo, te_junk1) == 26);
    assert!(offset_of!(TedInfo, te_just) == 28);
    assert!(offset_of!(TedInfo, te_color) == 30);
    assert!(offset_of!(TedInfo, te_junk2) == 32);
    assert!(offset_of!(TedInfo, te_thickness) == 34);
    assert!(offset_of!(TedInfo, te_txtlen) == 36);
    assert!(offset_of!(TedInfo, te_tmplen) == 38);

    assert!(size_of::<IconBlk>() == 48 && align_of::<IconBlk>() == 8);
    assert!(offset_of!(IconBlk, ib_pmask) == 0);
    assert!(offset_of!(IconBlk, ib_pdata) == 8);
    assert!(offset_of!(IconBlk, ib_ptext) == 16);
    assert!(offset_of!(IconBlk, ib_char) == 24);
    assert!(offset_of!(IconBlk, ib_xchar) == 26);
    assert!(offset_of!(IconBlk, ib_ychar) == 28);
    assert!(offset_of!(IconBlk, ib_xicon) == 30);
    assert!(offset_of!(IconBlk, ib_yicon) == 32);
    assert!(offset_of!(IconBlk, ib_wicon) == 34);
    assert!(offset_of!(IconBlk, ib_hicon) == 36);
    assert!(offset_of!(IconBlk, ib_xtext) == 38);
    assert!(offset_of!(IconBlk, ib_ytext) == 40);
    assert!(offset_of!(IconBlk, ib_wtext) == 42);
    assert!(offset_of!(IconBlk, ib_htext) == 44);

    assert!(size_of::<Grect>() == 8);
    assert!(offset_of!(Grect, x) == 0);
    assert!(offset_of!(Grect, y) == 2);
    assert!(offset_of!(Grect, w) == 4);
    assert!(offset_of!(Grect, h) == 6);
};

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{offset_of, size_of};

    /// The native records as the AES' LP64 accessor reads them: `ob_spec` @16,
    /// coordinates @24..30, 32-byte stride; TEDINFO/ICONBLK WORDs after three
    /// 8-byte pointers (@24).
    #[test]
    fn native_record_layout() {
        assert_eq!(size_of::<Object>(), 32);
        assert_eq!(offset_of!(Object, ob_type), 6);
        assert_eq!(offset_of!(Object, ob_spec), 16);
        assert_eq!(offset_of!(Object, ob_x), 24);
        assert_eq!(offset_of!(Object, ob_height), 30);
        assert_eq!(size_of::<TedInfo>(), 40);
        assert_eq!(offset_of!(TedInfo, te_font), 24);
        assert_eq!(offset_of!(TedInfo, te_tmplen), 38);
        assert_eq!(size_of::<IconBlk>(), 48);
        assert_eq!(offset_of!(IconBlk, ib_char), 24);
        assert_eq!(offset_of!(IconBlk, ib_htext), 44);
        assert_eq!(size_of::<Grect>(), 8);
    }

    /// `Grect::intersect` (EmuTOS `rc_intersect`): overlap, touching edges
    /// (no overlap), containment.
    #[test]
    fn grect_intersect() {
        let a = Grect { x: 0, y: 0, w: 10, h: 10 };
        let b = Grect { x: 5, y: 5, w: 10, h: 10 };
        assert_eq!(a.intersect(&b), Some(Grect { x: 5, y: 5, w: 5, h: 5 }));
        let c = Grect { x: 10, y: 0, w: 5, h: 5 };
        assert_eq!(a.intersect(&c), None);
        let d = Grect { x: 2, y: 3, w: 4, h: 5 };
        assert_eq!(a.intersect(&d), Some(d));
    }
}
