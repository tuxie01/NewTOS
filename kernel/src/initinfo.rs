//! NewTOS boot information screen — the EmuTOS-style "initinfo" screen,
//! rebuilt with our own VDI primitives.
//!
//! Like EmuTOS, it shows a centred banner, a separator, a block of
//! `label : value` system-info pairs (version, CPU, machine, RAM), a
//! second separator and a couple of message lines — but every pixel is
//! drawn **through the VDI** (`svc #2`), exercising the Phase-12 text,
//! fill and line primitives. This is the screen shown after boot.
//!
//! [`run`] also owns the **hold** — EmuTOS' `initinfo` does not just draw the
//! screen, it waits on it (`references/emutos/bios/initinfo.c:376-442`) and
//! returns the modifier keys the user was holding, which `biosmain` turns into
//! boot flags (`bios/bios.c:816-820`). See [`wait_for_user`].

use alloc::format;

use crate::bios::machine::frames::{stats, Pool};
use crate::vdi_client::Vdi;

// VDI palette indices (see vdi_backend::PALETTE).
const WHITE: i16 = 0;
const BLACK: i16 = 1;
const RED: i16 = 2;
const BLUE: i16 = 4;

/// EmuTOS `INITINFO_DURATION` — how long the welcome screen is held, in
/// **seconds** (`references/emutos/include/config.h:1392-1405`):
///
/// > By default, the EmuTOS welcome screen (initinfo) is displayed for 3
/// > seconds. On emulators, this is enough to read the text, and optionally
/// > to press Shift to keep the screen displayed. But on real hardware, it
/// > can take several seconds for the monitor to recover from stand-by mode,
/// > so the welcome screen may never be seen. In such cases, it is wise to
/// > increase the welcome screen duration.
///
/// EmuTOS makes it a **per-target build-time constant** and raises it for
/// machines whose display is slow to come up: the FireBee target overrides it
/// to 8 with the comment *"Longer time for LCD monitors startup"*
/// (`include/config.h:173-175`). NewTOS' build-time targets are the boards, so
/// the same per-target override lives here — the Pi-4's HDMI monitor spends
/// several seconds locking the mode (its own sync banner covers the first part
/// of our boot screen), so `board-rpi4` gets 15 s; qemu-virt keeps EmuTOS'
/// default 3, which is exactly the "on emulators, this is enough" case.
///
/// A long duration is not a long boot: this is a *wait*, not a delay. Any key
/// — or Shift/Control/Alternate — ends it immediately ([`wait_for_user`]), so
/// the cost of the 15 s on a machine that is being rebooted repeatedly is one
/// keypress.
///
/// EmuTOS' `INITINFO_DURATION == 0` case (`bios/bios.c:801-802`: do not show
/// the welcome screen at all) has no NewTOS counterpart — neither board sets 0.
#[cfg(feature = "board-rpi4")]
const INITINFO_DURATION: u64 = 15;
/// See the `board-rpi4` arm above.
#[cfg(not(feature = "board-rpi4"))]
const INITINFO_DURATION: u64 = 3;

/// `MODE_RSHIFT|MODE_LSHIFT` = `MODE_SHIFT` (`references/emutos/bios/ikbd.h:26`,
/// `include/biosdefs.h:90-91`) — a Shift key is down, which *holds* the screen.
const MODE_SHIFT: u8 = 0x03;

/// `MODE_SCA` (`references/emutos/include/biosdefs.h:94`) — Shift, Control or
/// Alternate is down; any of them ends the timed wait.
const MODE_SCA: u8 = 0x0F;

/// `DEL_ASCII` (`references/emutos/bios/initinfo.c:48`) — the spurious Delete
/// keypress left over from a Ctrl+Alt+Shift+Delete reboot, which the wait loop
/// swallows instead of treating as "user pressed a key".
const DEL_ASCII: u8 = 0x7F;

/// `INFO_LENGTH` (`references/emutos/bios/initinfo.c:50`) — the width of the
/// info lines in characters; also the width of the inverse-video bar.
const INFO_LENGTH: i16 = 40;

// ── The two strings the desktop's About box also shows ──────────────────────
//
// **They are duplicated there, since DESK43 U4 (2026-09-26), and that is the
// intended arrangement.** The older note here claimed the desktop reads these
// very constants because "EmuTOS keeps the same single source". EmuTOS does —
// `desk_xlate_fix` re-points `DEVERSN->ob_spec` at the BIOS `version[]` symbol
// (`desk/deskmain.c:1880`) — but it can, because its desktop is linked into
// the same ROM image as its BIOS. NewTOS's will not be: `programs/desktop` is
// to have neither `kernel` nor `tos-api` in its dependency graph
// (`docs/analysis/desktop-userspace-plan-v2.md` §6.4, measure 1), so the shared
// symbol is exactly the kind of link that move removes. The desktop's copies
// live at `programs/desktop/src/inf.rs`, next to the About box's other literals,
// and name this comment. Editing one means editing both (the `NTOS` cookie is
// a number, `tos-api/src/cookie.rs:142`; `Sversion` is the GEMDOS version).
// Programs get `VERSION_STRING` + the build stamp through
// `Ssystem(S_NEWTOS_KVERSION)` (2019, SYSINFO01, via `Host::os_version`).
// The former authorship line ("Adapted by ...") is gone from both readers:
// the boot screen and the About box dropped it on 2026-10-09 (user decision).

/// The NewTOS version string, as the boot screen's `Version` line prints it.
/// The desktop's About box shows the same text from its own constant — see the
/// block comment above.
pub(crate) const VERSION_STRING: &str = "NewTOS 0.6.1";

/// The project's independence notice, as the boot screen's footer carries it.
/// This is the NewTOS counterpart of the GPL/attribution block EmuTOS puts in
/// `ADDINFO` (`desk/desk_rsc.c:810-868`); the desktop's About box shows the
/// same text from its own constant.
pub(crate) const INDEPENDENCE_NOTICE: &str =
    "NewTOS is an independent project - not original Atari software.";

/// The build stamp, `YYYY-MM-DD (rev)`, re-exported from [`tos_api::build`].
///
/// It lives in `tos-api` because the OSHEADER's `os_date`/`os_dosdate` need the
/// same value and a build script's environment reaches only its own crate —
/// that is EmuTOS's arrangement as well, where `tools/mkheader.awk` generates
/// one `obj/header.h` and the header fields read it.
///
/// **An own addition on this screen.** EmuTOS's boot screen prints
/// `EmuTOS Version %s` and no date (`bios/initinfo.c:321`); its build date goes
/// into the OS header alone. NewTOS prints it here because this screen is the
/// project's own and a stamp is what tells two SD cards apart.
pub(crate) const BUILD_STAMP: &str = tos_api::build::BUILD_STAMP;

/// Friendly machine name for the current board, as the boot screen's
/// `Machine` line prints it.
///
/// **The desktop's About box shows the same texts from its own per-board build
/// constant** (`MACHINE_NAME` in `programs/desktop/src/inf.rs`, X03, user decision
/// 2026-10-01) and no longer calls this function — the last `crate::initinfo`
/// reference DESK43 U4 left standing. Same reason as the block comment above:
/// the desktop will not have `kernel` in its dependency graph, and no syscall
/// carries the answer (`_MCH` is a single fixed NewTOS marker, `0x0005_0000`
/// on both boards, `tos-api/src/cookie.rs:143`). Editing a text here means
/// editing it there too. Private to this module since X03, so no second
/// reader can attach to it.
///
/// The `other => other` arm prints the raw board identifier for a board that
/// has no friendly name yet; the desktop's copy cannot see that identifier and
/// fails the build instead (see its doc comment).
fn machine_name() -> &'static str {
    match crate::BOARD_NAME {
        "rpi4" => "Raspberry Pi 4 (BCM2711)",
        "qemu-virt" => "QEMU aarch64-virt",
        other => other,
    }
}

/// Build a list of mounted drive letters (e.g. `"C: D:"`) from the BIOS
/// drive bitmap, as filled by `Drvmap` / the partition mount.
fn drives_str() -> alloc::string::String {
    let bits = tos_api::bios::drive::drvbits();
    if bits == 0 {
        return alloc::string::String::from("none");
    }
    let mut s = alloc::string::String::new();
    for d in 0..16u32 {
        if bits & (1 << d) != 0 {
            if !s.is_empty() {
                s.push(' ');
            }
            s.push((b'A' + d as u8) as char);
            s.push(':');
        }
    }
    s
}

/// Format a byte count the way EmuTOS' `initinfo` does — its
/// `cprintf_bytesize` (`references/emutos/bios/initinfo.c:243-260`):
///
/// ```c
/// if (bytes % (1024UL * 1024UL) == 0) { unit = _("MB"); value = bytes / (1024UL*1024UL); }
/// else                                { unit = _("KB"); value = bytes / 1024UL; }
/// ```
///
/// i.e. **MB only for an exact multiple of a MiB, KB otherwise** — never a
/// truncated MB. That distinction matters here: our ST-RAM figure is rarely
/// a round MiB (it starts at the end of the kernel image), and the previous
/// `bytes >> 20` silently threw away up to 1 MiB, so a real 2-MiB change in
/// the pool showed up as "5 MiB → 3 MiB" while the true values were 5.92 and
/// 3.92 MiB. EmuTOS' rule reports both exactly, as `6064 KB` / `4016 KB`.
fn size_str(bytes: u64) -> alloc::string::String {
    const MIB: u64 = 1024 * 1024;
    if bytes.is_multiple_of(MIB) {
        format!("{} MB", bytes / MIB)
    } else {
        format!("{} KB", bytes / 1024)
    }
}

/// The "NEWTOS" logo as a 5×35 cell grid. `#` = a filled (coloured) cell,
/// space = background — rendered as coloured blocks (not text), exactly the
/// way EmuTOS' `print_art` paints its logo from a colour grid. "NEW" is
/// drawn blue, "TOS" red (columns < 18 vs ≥ 18).
const LOGO: [&[u8]; 5] = [
    b"#   # ##### #   # #####  ###   ####",
    b"##  # #     #   #   #   #   # #    ",
    b"# # # ####  # # #   #   #   #  ### ",
    b"#  ## #     ## ##   #   #   #     #",
    b"#   # ##### #   #   #    ###  #### ",
];

/// Format the GEMDOS clock as `YYYY-MM-DD HH:MM:SS` for the boot-time line.
fn boot_time_str() -> alloc::string::String {
    let (time, date) = tos_api::gemdos::time::dos_now();
    let day = date & 0x1F;
    let month = (date >> 5) & 0x0F;
    let year = 1980u16.wrapping_add(date >> 9);
    let sec = (time & 0x1F) * 2;
    let min = (time >> 5) & 0x3F;
    let hour = time >> 11;
    format!("{year:04}-{month:02}-{day:02} {hour:02}:{min:02}:{sec:02}")
}

/// Draw the boot information screen, hold it, and return the keyboard
/// modifier state the user was holding when it ended — EmuTOS
/// `initinfo(&shiftbits)` (`references/emutos/bios/initinfo.c:267-457`).
///
/// The returned byte is EmuTOS' `*pshiftbits` in `Kbshift` / `MODE_*`
/// encoding; `biosmain` turns it into boot flags (`bios/bios.c:816-820`) —
/// see [`crate::launch::set_bootflags_from_shiftbits`]. EmuTOS also returns
/// the selected boot device; NewTOS does not (see [`wait_for_user`]).
///
/// **Without a screen** there is nothing to hold, so we take EmuTOS' own
/// no-welcome-screen path, `*pshiftbits = kbshift(-1)` with no wait
/// (`initinfo.c:463-469`, the `#else FULL_INITINFO` variant; the same
/// assignment `biosmain` makes when the screen is suppressed,
/// `bios/bios.c:812`).
pub fn run() -> u8 {
    #[cfg(feature = "initinfo-diag")]
    diag::ENTERED.store(true, core::sync::atomic::Ordering::Relaxed);
    // Every input the wait loop needs — `kbshift`, `bconstat2`, `bconin2` and
    // the 200 Hz tick — reaches us through the installed host, exactly as
    // EmuTOS' `initinfo` calls those BIOS functions directly. No host means no
    // keyboard and no clock, so there is nothing to wait *for*.
    let Some(host) = tos_api::host::host() else {
        return 0;
    };
    #[cfg(feature = "initinfo-diag")]
    diag::HOST_OK.store(true, core::sync::atomic::Ordering::Relaxed);
    let (sw, sh) = crate::vdi_backend::dims();
    #[cfg(feature = "initinfo-diag")]
    diag::set(&diag::DIMS, ((sw as u64) << 32) | sh as u64);
    if sw == 0 || sh == 0 {
        return host.keyboard_shift_state();
    }
    let (sw, sh) = (sw as i16, sh as i16);

    let mut v = Vdi::new();
    if !v.open() {
        return host.keyboard_shift_state();
    }
    #[cfg(feature = "initinfo-diag")]
    diag::VWK_OK.store(true, core::sync::atomic::Ordering::Relaxed);
    v.clear(); // white background

    // Everything the rows show is gathered once, so the measuring pass and
    // the drawing pass below lay out exactly the same rows.
    let content = Content::gather();

    // ── geometry: a box centred both ways ──
    //
    // EmuTOS draws no frame round its boot screen; it centres the text block
    // instead — vertically by `top_margin = (screen_height - initinfo_height)
    // / 2` (`references/emutos/bios/initinfo.c:301-306`), horizontally by
    // `left_margin = (SCREEN_WIDTH-LOGO_LENGTH) / 2` for the logo and
    // `(SCREEN_WIDTH-INFO_LENGTH) / 2` for the info lines (:308-316). EmuTOS
    // has to keep `initinfo_height` in step with the rows by hand (:270,
    // :287-299); here a measuring pass of the same `lay_out` supplies both the
    // height and the width the rows need. The box is the middle third of the
    // screen, widened to the content and narrowed to the screen if need be;
    // a block taller than the screen starts at the top edge.
    let mut probe = Sheet::measure();
    lay_out(&mut probe, &content);
    let bw = (sw / 3).max(probe.need_w).min(sw - 32);
    let bh = probe.y;
    let fx0 = (sw - bw) / 2;
    let fy0 = ((sh - bh) / 2).max(0);
    let (fx1, fy1) = (fx0 + bw - 1, (fy0 + bh - 1).min(sh - 1));

    // ── outer frame ──
    v.set_line(BLACK);
    rect(&mut v, fx0, fy0, fx1, fy1);

    let mut sheet = Sheet::draw(&mut v, fx0, fx1, fy0);
    lay_out(&mut sheet, &content);

    v.close();

    // EmuTOS' `initinfo` ends in the wait loop, not in the last `cprintf`.
    wait_for_user(host)
}

/// Space between the frame's top/bottom edge and the first/last row.
const PAD_Y: i16 = 16;
/// Space a centred row keeps from the frame's left and right edges.
const PAD_X: i16 = 24;
/// Inset of the separators from the frame's left and right edges.
const SEP_INSET: i16 = 16;
/// Left edge of the USB tree, measured from the frame's left edge.
const USB_INSET: i16 = 40;
/// Size of one logo cell, in pixels.
const LOGO_CELL: (i16, i16) = (12, 13);

/// The values the boot screen shows, read once so that the measuring and the
/// drawing pass of [`lay_out`] see the same rows.
struct Content {
    /// The `label : value` system-info pairs, top to bottom.
    pairs: [(&'static [u8], alloc::string::String); 8],
    /// The xHCI port count and one line per HID device, or `None` without a
    /// USB host controller.
    usb: Option<(u8, alloc::vec::Vec<alloc::string::String>)>,
}

impl Content {
    fn gather() -> Self {
        use alloc::string::String;

        let st = stats(Pool::Stram);
        let tt = stats(Pool::Ttram);
        let pairs = [
            (&b"Version"[..], String::from(VERSION_STRING)),
            (&b"Built"[..], String::from(BUILD_STAMP)),
            (&b"CPU type"[..], String::from("ARM Cortex-A72 (AArch64)")),
            (&b"Machine"[..], String::from(machine_name())),
            (&b"ST-RAM"[..], size_str(st.total_bytes())),
            (&b"TT-RAM"[..], size_str(tt.total_bytes())),
            (&b"GEMDOS drives"[..], drives_str()),
            (&b"Boot time"[..], boot_time_str()),
        ];

        // USB devices are enumerated *before* this screen.
        let mut devs = [crate::drivers::usb::DeviceInfo::default(); 4];
        let (ports, n) = crate::drivers::usb::tree(&mut devs);
        #[cfg(feature = "initinfo-diag")]
        diag::set(&diag::USB, ((ports as u64) << 32) | n as u64);
        let usb = (ports > 0).then(|| {
            let lines = devs[..n]
                .iter()
                .enumerate()
                .map(|(i, d)| {
                    let kind = match d.protocol {
                        1 => "Keyboard",
                        2 => "Mouse",
                        _ => "HID device",
                    };
                    let lead = if i + 1 == n { "\\-" } else { "|-" };
                    format!("{} Port {}: {}  {:04x}:{:04x}", lead, d.port, kind, d.vid, d.pid)
                })
                .collect();
            (ports, lines)
        });
        Content { pairs, usb }
    }
}

/// One pass over the boot screen's rows: either **measuring** (no
/// workstation; it records how wide the widest row is and, through `y`, how
/// tall the block is) or **drawing** into the frame `x0..=x1`.
struct Sheet<'a> {
    /// `None` while measuring.
    v: Option<&'a mut Vdi>,
    x0: i16,
    x1: i16,
    /// Top of the next row; after a measuring pass, the block's height.
    y: i16,
    /// Frame width the rows need (measuring pass).
    need_w: i16,
}

impl<'a> Sheet<'a> {
    fn measure() -> Self {
        Sheet { v: None, x0: 0, x1: 0, y: 0, need_w: 0 }
    }

    fn draw(v: &'a mut Vdi, x0: i16, x1: i16, y0: i16) -> Self {
        Sheet { v: Some(v), x0, x1, y: y0, need_w: 0 }
    }

    fn cx(&self) -> i16 {
        (self.x0 + self.x1 + 1) / 2
    }

    fn gap(&mut self, dy: i16) {
        self.y += dy;
    }

    /// A row of `w` pixels centred in the frame.
    fn need_centred(&mut self, w: i16) {
        self.need_w = self.need_w.max(w + 2 * PAD_X);
    }

    /// The logo grid, centred; advances past it.
    fn logo(&mut self) {
        let (cw, chh) = LOGO_CELL;
        let cols = LOGO[0].len() as i16;
        self.need_centred(cols * cw);
        let (logo_x, logo_y) = (self.cx() - cols * cw / 2, self.y);
        if let Some(v) = self.v.as_deref_mut() {
            for (r, row) in LOGO.iter().enumerate() {
                let ry = logo_y + r as i16 * chh;
                for (c, &cell) in row.iter().enumerate() {
                    if cell == b'#' {
                        let col = c as i16;
                        v.set_fill(if col < 18 { BLUE } else { RED });
                        let bx = logo_x + col * cw;
                        v.fill_rect(bx, ry, bx + cw - 1, ry + chh - 1);
                    }
                }
            }
        }
        self.y += LOGO.len() as i16 * chh;
    }

    /// A text line centred in the frame, in `colour`.
    fn centred(&mut self, text: &[u8], colour: i16) {
        use tos_api::vdi::font::CELL_W;
        self.need_centred(text.len() as i16 * CELL_W);
        let (cx, y) = (self.cx(), self.y);
        if let Some(v) = self.v.as_deref_mut() {
            v.set_text_align(1, 5); // centre, top
            v.set_text_color(colour);
            v.gtext(cx, y, text);
        }
    }

    /// A horizontal separator across the frame.
    fn separator(&mut self) {
        let (x0, x1, y) = (self.x0 + SEP_INSET, self.x1 - SEP_INSET, self.y);
        if let Some(v) = self.v.as_deref_mut() {
            v.set_line(BLACK);
            v.line(x0, y, x1, y);
        }
    }

    /// A `label : value` pair around the frame's centre (see [`pair`]).
    fn pair(&mut self, label: &[u8], value: &[u8]) {
        use tos_api::vdi::font::CELL_W;
        // `pair` writes at most 22 label bytes plus " :", 8 px left of centre,
        // and the value 8 px right of it: the wider half sets the width.
        let left = (label.len().min(22) as i16 + 2) * CELL_W + 8;
        let right = value.len() as i16 * CELL_W + 8;
        self.need_centred(2 * left.max(right));
        let (cx, y) = (self.cx(), self.y);
        if let Some(v) = self.v.as_deref_mut() {
            pair(v, cx, y, label, value);
        }
    }

    /// A left-aligned line of the USB tree, `indent` px into the tree.
    fn usb_line(&mut self, indent: i16, text: &[u8]) {
        use tos_api::vdi::font::CELL_W;
        let dx = USB_INSET + indent;
        self.need_w = self.need_w.max(dx + text.len() as i16 * CELL_W + PAD_X);
        let (x, y) = (self.x0 + dx, self.y);
        if let Some(v) = self.v.as_deref_mut() {
            v.set_text_align(0, 5); // left, top
            v.set_text_color(BLACK);
            v.gtext(x, y, text);
        }
    }

    /// The inverse-video message bar, centred (see [`display_inverse`]).
    fn inverse(&mut self, text: &[u8]) {
        use tos_api::vdi::font::CELL_W;
        self.need_centred(INFO_LENGTH * CELL_W);
        let (cx, y) = (self.cx(), self.y);
        if let Some(v) = self.v.as_deref_mut() {
            display_inverse(v, cx, y, text);
        }
    }
}

/// The boot screen's rows, top to bottom — the single description both the
/// measuring and the drawing pass of [`run`] walk. Starts at the frame's top
/// edge and ends at its bottom edge, so a measuring pass leaves the frame
/// height in `s.y`.
fn lay_out(s: &mut Sheet, c: &Content) {
    use tos_api::vdi::font::CELL_H;

    // ── NewTOS logo: a grid of coloured blocks (EmuTOS' print_art idea —
    //    each cell is a colour, not an ASCII char). "New" blue, "TOS" red. ──
    s.gap(PAD_Y);
    s.logo();

    // ── tagline, centred ──
    s.gap(12);
    s.centred(b"This operating system isnt Original", BLACK);

    // separator under the tagline
    s.gap(24);
    s.separator();

    // ── system-info pairs (label right-aligned, value left of centre) ──
    s.gap(18);
    for (i, (label, value)) in c.pairs.iter().enumerate() {
        if i > 0 {
            s.gap(22);
        }
        s.pair(label, value.as_bytes());
    }

    // separator
    s.gap(22);
    s.separator();

    // ── USB device tree ──
    s.gap(16);
    if let Some((ports, lines)) = &c.usb {
        s.usb_line(0, format!("USB: xHCI host controller, {ports} ports").as_bytes());
        s.gap(18);
        if lines.is_empty() {
            s.usb_line(16, b"(no devices connected)");
            s.gap(18);
        }
        for line in lines {
            s.usb_line(12, line.as_bytes());
            s.gap(18);
        }
    }

    // ── the key hints EmuTOS prints below the second separator
    //    (`bios/initinfo.c:353-364`) ──
    //
    // EmuTOS prints five; NewTOS prints the two whose effect this build
    // actually has. The other three are honestly absent, because the machinery
    // behind them is:
    //
    // * `"Hold <Alternate> to skip HDD boot"` (:355) sets
    //   `BOOTFLAG_SKIP_HDD_BOOT`, which suppresses *executing the hard disk's
    //   boot sector* in `blkdev_boot` (`bios/blkdev.c:231`). NewTOS never
    //   executes a boot sector (there is no m68k to run it on), so there is
    //   nothing for the flag to skip.
    // * `"Press key 'X' to boot from X:"` (:357) selects the boot device, which
    //   EmuTOS then feeds to `blkdev_boot`, `Dsetdrv(bootdev)` and `autoexec`
    //   (`bios/bios.c:825-846`). NewTOS has no boot-device variable: the AUTO
    //   scan is hardwired to `C:\AUTO\` (`launch::run_auto_folder`). Wiring a
    //   real `bootdev` through those callers is a separate change; faking the
    //   key without it would only pretend to select a drive.
    // * `"Press <Esc> to run an early console"` (:359) is `#if WITH_CLI` and
    //   sets `BOOTFLAG_EARLY_CLI`, which `biosmain` uses to `Pexec` EmuCON
    //   *before* `autoexec` (`bios/bios.c:834-844`). NewTOS' console shell is
    //   reachable as the shell loop's `CONSOLE_APP`, but there is no
    //   run-a-shell-then-continue-booting entry, so the flag has no consumer.
    s.gap(10);
    // `display_message(_("Hold <Control> to skip AUTO/ACC"))` (:353)
    s.centred(b"Hold <Control> to skip AUTO/ACC", BLACK);
    s.gap(22);
    // `display_inverse(_("Hold <Shift> to pause this screen"),0)` (:364)
    s.inverse(b"Hold <Shift> to pause this screen");

    // ── footer ──
    s.gap(26);
    s.centred(INDEPENDENCE_NOTICE.as_bytes(), BLACK);
    s.gap(CELL_H + PAD_Y);
}

/// `display_inverse(s, FALSE)` (`references/emutos/bios/initinfo.c:134-150`) —
/// the message centred in an inverse-video bar.
///
/// EmuTOS pads the string with spaces to [`INFO_LENGTH`] characters and
/// brackets the whole run with the VT52 inverse-video pair `\033p` … `\033q`
/// (`:142-146`). Our screen is VDI-drawn rather than a character grid, so the
/// same appearance is a filled `INFO_LENGTH`-cell bar in the foreground colour
/// with the text drawn on top in the background colour. The text goes down in
/// **transparent** writing mode: the VDI's default replace mode paints a
/// glyph's clear pixels with pen 0 (`vdi_backend::draw_glyph_sized`, EmuTOS
/// `vdi_textblit.c` `WM_REPLACE`), which would repaint the bar white under
/// every character cell.
fn display_inverse(v: &mut Vdi, cx: i16, y: i16, s: &[u8]) {
    use tos_api::vdi::font::{CELL_H, CELL_W};

    let half = INFO_LENGTH * CELL_W / 2;
    v.set_fill(BLACK);
    v.fill_rect(cx - half, y, cx + half - 1, y + CELL_H - 1);
    v.set_text_align(1, 5); // centre, top
    v.set_text_color(WHITE);
    v.set_write_mode(2); // transparent
    v.gtext(cx, y, s);
    v.set_write_mode(1); // back to replace, the workstation default
}

/// Hold the boot screen — EmuTOS `initinfo`'s wait loop
/// (`references/emutos/bios/initinfo.c:376-442`). Returns the keyboard
/// modifier state (`kbshift(-1)`) as it was when the wait ended.
///
/// ```c
/// while (1) {
///     long end = hz_200 + INITINFO_DURATION * 200UL;
///     olddev = dev;
///     do {
///         shiftbits = kbshift(-1);
///         if ((shiftbits & MODE_SCA) || bconstat2()) break;
///     } while (hz_200 < end);
///     while ((shiftbits & MODE_SHIFT) && !bconstat2()) shiftbits = kbshift(-1);
///     if (bconstat2()) {
///         int c = toupper(LOBYTE(bconin2()));
///         if (c == DEL_ASCII) continue;      /* eat spurious Delete */
///         ...                                /* Esc / drive letter */
///     }
///     if (dev == olddev) break;
///     cprint_devices(dev);
/// }
/// ```
///
/// So the screen stays up for [`INITINFO_DURATION`] seconds, **but**:
///
/// * pressing any key ends it at once — and the key is *consumed* here
///   (`bconin2`), so it does not leak into the AUTO programs or the desktop;
/// * holding Shift, Control or Alternate ends the timed part at once, and
///   holding **Shift** then keeps the screen up for as long as it is held
///   (`:407-413`) — this is the "Hold `<Shift>` to pause this screen" affordance;
/// * a spurious `Delete` (the tail of a Ctrl+Alt+Shift+Delete reboot) is eaten
///   and the whole wait starts over (`:420-422`).
///
/// **Two things from the C are deliberately not here.**
///
/// 1. The boot-device selection (`olddev`/`dev`, `:390`, `:429-441`), which is
///    what makes the outer `while (1)` loop more than one pass: EmuTOS
///    redisplays the drive line and waits again when the user picks another
///    drive. NewTOS has no boot-device variable to select (see the hint-line
///    comment in [`run`]), so the loop always leaves after one pass — except
///    for the `Delete` `continue`, which is kept.
/// 2. `stop_until_interrupt()` (`:400-402`, `#if USE_STOP_INSN_TO_FREE_HOST_CPU`)
///    — an m68k `stop` that idles the CPU until the next interrupt. On this
///    hardware that would be a *hang*, not an idle: the real Pi-4 keyboard is a
///    polled USB-HID device (`drivers::usb::poll`, the PHASE-16 stack), so
///    nothing would ever wake a `wfi` and the wait could not be skipped at all.
///    The loop therefore drives [`tos_api::Host::pump_input`] — the same hook
///    the AES event loop and the blocking console reads use — on every pass.
///    That call *is* the keyboard on real hardware; on qemu-virt it is
///    redundant (the PL011 RX IRQ fills the same ring) but harmless.
///
/// The 200 Hz `hz_200` tick becomes the host's monotonic millisecond clock —
/// the same source the process loader snapshots into the `_hz_200` sysvar
/// (`tos_api::bdos::process::loader`, `$4BA`).
fn wait_for_user(host: &dyn tos_api::Host) -> u8 {
    #[cfg(feature = "initinfo-diag")]
    {
        use core::sync::atomic::Ordering::Relaxed;
        let (cnt, frq) = diag::timer();
        let (head, tail) = crate::console::rx_indices();
        diag::WAITED.store(true, Relaxed);
        diag::set(&diag::FRQ, frq);
        diag::set(&diag::CNT_IN, cnt);
        diag::set(&diag::MS_IN, host.monotonic_millis());
        diag::set(&diag::ST_IN, diag::systimer());
        diag::set(&diag::UART_GIC, diag::uart_gic());
        diag::set(&diag::RING_IN, ((head as u64) << 32) | (tail as u64 & 0xFFFF_FFFF));
    }
    let mut shiftbits;
    loop {
        #[cfg(feature = "initinfo-diag")]
        diag::OUTER.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        // `long end = hz_200 + INITINFO_DURATION * 200UL;` (:388)
        let end = host.monotonic_millis() + INITINFO_DURATION * 1000;
        #[cfg(feature = "initinfo-diag")]
        diag::set(&diag::MS_END, end);

        // `do { … } while (hz_200 < end);` (:392-404)
        loop {
            #[cfg(feature = "initinfo-diag")]
            diag::ITERS.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
            host.pump_input();
            shiftbits = host.keyboard_shift_state(); // `kbshift(-1)` (:394)
            #[cfg(feature = "initinfo-diag")]
            diag::SHIFT_SEEN.fetch_or(shiftbits, core::sync::atomic::Ordering::Relaxed);
            // "If Shift, Control, Alt or normal key is pressed, stop waiting"
            // (:396-398)
            if shiftbits & MODE_SCA != 0 || host.console_has_input() {
                #[cfg(feature = "initinfo-diag")]
                diag::REASON.store(
                    if shiftbits & MODE_SCA != 0 { 1 } else { 2 },
                    core::sync::atomic::Ordering::Relaxed,
                );
                break;
            }
            if host.monotonic_millis() >= end {
                #[cfg(feature = "initinfo-diag")]
                diag::REASON.store(3, core::sync::atomic::Ordering::Relaxed);
                break;
            }
        }

        // "Wait while Shift is pressed, and normal key is not pressed" (:406-413)
        while shiftbits & MODE_SHIFT != 0 && !host.console_has_input() {
            host.pump_input();
            shiftbits = host.keyboard_shift_state();
        }

        // "if a non-modifier key was pressed, examine it" (:415-435). The
        // `bconstat2()` test and the `bconin2()` read collapse into one
        // non-blocking pop; only the ASCII byte matters (`LOBYTE`, :417).
        if let Some(key) = host.console_try_read_key() {
            #[cfg(feature = "initinfo-diag")]
            diag::set(&diag::KEY, key as u64 + 1);
            if (key as u8).to_ascii_uppercase() == DEL_ASCII {
                continue; // "eat spurious Delete key press" (:420-422)
            }
        }

        break; // `if (dev == olddev) break;` (:436-437)
    }
    #[cfg(feature = "initinfo-diag")]
    {
        use core::sync::atomic::Ordering::Relaxed;
        let (cnt, _) = diag::timer();
        let (head, tail) = crate::console::rx_indices();
        diag::set(&diag::CNT_OUT, cnt);
        diag::set(&diag::MS_OUT, host.monotonic_millis());
        diag::set(&diag::ST_OUT, diag::systimer());
        diag::set(&diag::RING_OUT, ((head as u64) << 32) | (tail as u64 & 0xFFFF_FFFF));
        diag::SHIFT_OUT.store(shiftbits, Relaxed);
    }
    shiftbits
}

/// Draw a label/value pair: `label` right-aligned ending near the
/// centre, `value` left-aligned just past it.
fn pair(v: &mut Vdi, cx: i16, y: i16, label: &[u8], value: &[u8]) {
    // label + " :" right-aligned, ending 8 px left of centre
    v.set_text_align(2, 5); // right, top
    v.set_text_color(BLACK);
    let mut lbuf = [0u8; 24];
    let n = label.len().min(lbuf.len() - 2);
    lbuf[..n].copy_from_slice(&label[..n]);
    lbuf[n] = b' ';
    lbuf[n + 1] = b':';
    v.gtext(cx - 8, y, &lbuf[..n + 2]);
    // value left-aligned, 8 px right of centre
    v.set_text_align(0, 5); // left, top
    v.set_text_color(BLUE);
    v.gtext(cx + 8, y, value);
}

/// Draw a rectangle outline (four `v_pline` segments).
fn rect(v: &mut Vdi, x0: i16, y0: i16, x1: i16, y1: i16) {
    v.line(x0, y0, x1, y0);
    v.line(x1, y0, x1, y1);
    v.line(x1, y1, x0, y1);
    v.line(x0, y1, x0, y0);
}

/// Boot-screen diagnostic instrument (`initinfo-diag` feature only).
///
/// On real Pi-4 hardware the boot screen is reported as not being held at all,
/// while the identical `board-rpi4` binary holds it for the full
/// [`INITINFO_DURATION`] under QEMU. Every input EmuTOS' wait loop consults
/// ([`wait_for_user`]) is a host call, so this module records what each of
/// them actually returned on the machine, plus the raw generic-timer state
/// behind [`tos_api::Host::monotonic_millis`], and paints the result on the
/// screen instead of continuing the boot — the machine under test has no
/// serial line, so the screen is the only output channel.
///
/// Nothing here is compiled into a normal build; the recorder is a handful of
/// relaxed atomic stores and the reporter runs after `run()` returns
/// (`kernel_main_rust`).
#[cfg(feature = "initinfo-diag")]
pub mod diag {
    use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, Ordering::Relaxed};

    /// `run()` was entered at all.
    pub static ENTERED: AtomicBool = AtomicBool::new(false);
    /// A host was installed (`tos_api::host::host()` was `Some`).
    pub static HOST_OK: AtomicBool = AtomicBool::new(false);
    /// Screen geometry seen by `run()`: `(w << 32) | h`.
    pub static DIMS: AtomicU64 = AtomicU64::new(0);
    /// `v_opnvwk` succeeded (a non-zero workstation handle came back).
    pub static VWK_OK: AtomicBool = AtomicBool::new(false);
    /// The wait loop was entered.
    pub static WAITED: AtomicBool = AtomicBool::new(false);
    /// `CNTFRQ_EL0` as the machine reports it.
    pub static FRQ: AtomicU64 = AtomicU64::new(0);
    /// `CNTPCT_EL0` on entering / leaving the wait.
    pub static CNT_IN: AtomicU64 = AtomicU64::new(0);
    /// See [`CNT_IN`].
    pub static CNT_OUT: AtomicU64 = AtomicU64::new(0);
    /// `monotonic_millis()` on entering the wait, the computed deadline, and
    /// the value on leaving.
    pub static MS_IN: AtomicU64 = AtomicU64::new(0);
    /// See [`MS_IN`].
    pub static MS_END: AtomicU64 = AtomicU64::new(0);
    /// See [`MS_IN`].
    pub static MS_OUT: AtomicU64 = AtomicU64::new(0);
    /// Inner-loop passes and outer (`Delete`-restart) passes.
    pub static ITERS: AtomicU64 = AtomicU64::new(0);
    /// See [`ITERS`].
    pub static OUTER: AtomicU64 = AtomicU64::new(0);
    /// Which condition ended the timed wait: 1 = a modifier (`MODE_SCA`),
    /// 2 = `console_has_input`, 3 = the deadline, 0 = never ran.
    pub static REASON: AtomicU8 = AtomicU8::new(0);
    /// `keyboard_shift_state()` when the wait ended, and the OR of every value
    /// it ever returned during the wait.
    pub static SHIFT_OUT: AtomicU8 = AtomicU8::new(0);
    /// See [`SHIFT_OUT`].
    pub static SHIFT_SEEN: AtomicU8 = AtomicU8::new(0);
    /// The key word `console_try_read_key()` popped, `+1` (0 = none popped).
    pub static KEY: AtomicU64 = AtomicU64::new(0);
    /// RX-ring `(head << 32) | tail` on entering / leaving the wait.
    pub static RING_IN: AtomicU64 = AtomicU64::new(0);
    /// See [`RING_IN`].
    pub static RING_OUT: AtomicU64 = AtomicU64::new(0);
    /// USB: `(ports << 32) | devices` as `initinfo` itself read them.
    pub static USB: AtomicU64 = AtomicU64::new(0);
    /// PL011 `FR`/`RIS`/`MIS` and the GIC's view of the UART interrupt when
    /// the wait began, packed as `FR<<48 | RIS<<32 | MIS<<16 | gic`
    /// (`gic` = bit 0 `IGROUPR` bit for the UART INTID, bit 1 `ISPENDR`).
    /// See [`uart_gic`].
    pub static UART_GIC: AtomicU64 = AtomicU64::new(0);
    /// BCM2711 System Timer on entering / leaving the wait — a *second*,
    /// independent clock (see [`systimer`]), so the report can say whether the
    /// ARM generic timer's idea of 15 seconds is real time.
    pub static ST_IN: AtomicU64 = AtomicU64::new(0);
    /// See [`ST_IN`].
    pub static ST_OUT: AtomicU64 = AtomicU64::new(0);

    // ── who fills the IKBD queue ──────────────────────────────────────
    //
    // The 2026-09-04 measurement found one key word already queued before
    // the boot screen was drawn, value `0x0000`. Then two functions could
    // advance the ring — `console::serial_rx_byte` (PL011 RX IRQ) and
    // `console::push_rx` (the IKBD decoder); since XBIOS15 only `push_rx`
    // does (serial bytes go to `Iorec(0)` on qemu-virt, are dropped on
    // rpi4). Tagging every push still names the producer directly instead
    // of leaving it to be inferred.

    /// [`note_push`] source tag: the IKBD decoder (USB-HID keyboard).
    pub const PUSH_SRC_IKBD: u8 = 2;
    /// The first four words pushed into the IKBD queue, packed
    /// `src << 16 | word` (`src` = `PUSH_SRC_*`). `0` = slot unused.
    pub static PUSH_LOG: [AtomicU32; 4] = [const { AtomicU32::new(0) }; 4];
    /// Total number of pushes into the IKBD queue (may exceed [`PUSH_LOG`]).
    pub static PUSH_N: AtomicU32 = AtomicU32::new(0);
    /// Serial bytes the board's console profile discarded instead of queueing
    /// them as keys (real Pi-4: UART0 is debug-only, CON is the USB-HID
    /// keyboard). Counts every spurious character the floating RXD0 line
    /// clocks in during a whole boot.
    pub static SERIAL_DROPPED: AtomicU32 = AtomicU32::new(0);
    /// The first discarded serial byte, `+1` (0 = none). See
    /// [`SERIAL_DROPPED`].
    pub static SERIAL_FIRST: AtomicU32 = AtomicU32::new(0);
    /// Bytes the UART init flush found already in the RX FIFO — i.e. left
    /// there by the firmware or clocked in during the GPIO ALT0 switch,
    /// *before* the kernel owned the UART.
    pub static UART_FLUSHED: AtomicU32 = AtomicU32::new(0);
    /// The first byte the init flush discarded, `+1` (0 = none). See
    /// [`UART_FLUSHED`].
    pub static UART_FLUSH_FIRST: AtomicU32 = AtomicU32::new(0);

    /// Record one push into the IKBD queue with its producer tag.
    pub fn note_push(src: u8, word: u16) {
        let n = PUSH_N.fetch_add(1, Relaxed) as usize;
        if let Some(slot) = PUSH_LOG.get(n) {
            slot.store((src as u32) << 16 | word as u32, Relaxed);
        }
    }

    /// Record one serial byte dropped by the board's console profile.
    pub fn note_serial_drop(b: u8) {
        if SERIAL_DROPPED.fetch_add(1, Relaxed) == 0 {
            SERIAL_FIRST.store(b as u32 + 1, Relaxed);
        }
    }

    /// Record one byte discarded by the UART init RX-FIFO flush.
    pub fn note_uart_flush(b: u8) {
        if UART_FLUSHED.fetch_add(1, Relaxed) == 0 {
            UART_FLUSH_FIRST.store(b as u32 + 1, Relaxed);
        }
    }

    /// Read the generic timer the way [`tos_api::Host::monotonic_millis`]
    /// does, so the report shows its raw inputs rather than a derived value.
    pub fn timer() -> (u64, u64) {
        let (cnt, frq): (u64, u64);
        // SAFETY: read-only system-register reads, always permitted at EL1.
        unsafe {
            core::arch::asm!(
                "mrs {cnt}, cntpct_el0",
                "mrs {frq}, cntfrq_el0",
                cnt = out(reg) cnt,
                frq = out(reg) frq,
                options(nomem, nostack, preserves_flags),
            );
        }
        (cnt, frq)
    }

    /// The BCM2711 **System Timer**'s free-running 64-bit counter — a clock
    /// that is *not* the ARM generic timer, so it can falsify it.
    ///
    /// BCM2711 ARM Peripherals §10 (p. 142-143): the System Timer sits at bus
    /// address `0x7e003000` — visible to the ARM at `0xFE003000` in Low
    /// Peripheral mode (§1.2, "visible to the ARM at `0x0_FEnn_nnnn` if Low
    /// Peripheral mode is enabled", the same mapping this kernel already uses
    /// for the PL011 at `0xFE201000`) — with `CLO` at `+0x04` and `CHI` at
    /// `+0x08` (§10.2, Table 166), both read-only. `CHI` is read either side of
    /// `CLO` so a carry between the two halves is detected and retried.
    ///
    /// NewTOS does not use this counter for timekeeping (`docs/HARDWARE-PROFILE.md`
    /// pins the clock to `CNTPCT_EL0`); it is here only as the diagnostic's
    /// independent reference.
    #[cfg(feature = "board-rpi4")]
    pub fn systimer() -> u64 {
        let base = crate::drivers::mmio_va(0xFE00_3000);
        loop {
            // SAFETY: read-only MMIO in the mapped peripheral window.
            unsafe {
                let hi1 = ((base + 0x08) as *const u32).read_volatile();
                let lo = ((base + 0x04) as *const u32).read_volatile();
                let hi2 = ((base + 0x08) as *const u32).read_volatile();
                if hi1 == hi2 {
                    return ((hi1 as u64) << 32) | lo as u64;
                }
            }
        }
    }

    /// No BCM2711 System Timer off the Pi-4 board — the report says `0`.
    #[cfg(not(feature = "board-rpi4"))]
    pub fn systimer() -> u64 {
        0
    }

    /// The PL011's receive state and the GIC's view of its interrupt, packed
    /// as `FR<<48 | RIS<<32 | MIS<<16 | (ISPENDR<<1 | IGROUPR)`.
    ///
    /// This settles the one input the wait loop has that is not the USB
    /// keyboard. With no serial adapter plugged in, GPIO 15 (RXD0) is an
    /// input with **no pull** (`bcm2711::pl011::configure_uart0_pins` clears
    /// `GPIO_PUP_PDN_CNTRL_REG0` bits 31:30), so the receiver may see a
    /// permanent break and keep filling its FIFO. Whether any of that reaches
    /// the console ring depends on the GIC delivering INTID 153 — the very
    /// thing `docs/MEMORY.md` QUIRK-002 says this firmware does not do.
    ///
    /// * `FR` (`+0x18`), `RIS` (`+0x3c`), `MIS` (`+0x40`): BCM2711 ARM
    ///   Peripherals §11.5, Register View. `FR` bit 4 = RXFE (RX FIFO empty),
    ///   bit 6 = RXFF; `RIS` bit 4 = RXRIS, bit 9 = BERIS (break), bit 10 =
    ///   OERIS (overrun).
    /// * `GICD_IGROUPR` (`+0x080`) and `GICD_ISPENDR` (`+0x200`), one bit per
    ///   INTID: ARM IHI 0048B (GICv2), the registers QUIRK-002 names.
    #[cfg(feature = "board-rpi4")]
    pub fn uart_gic() -> u64 {
        const UART: usize = 0xFE20_1000;
        const GICD: usize = 0xFF84_1000;
        let uart = crate::drivers::mmio_va(UART);
        let gicd = crate::drivers::mmio_va(GICD);
        let intid = crate::drivers::gic::BOARD_PL011_INTID as usize;
        // SAFETY: read-only MMIO in the mapped peripheral window.
        unsafe {
            let fr = ((uart + 0x18) as *const u32).read_volatile() as u64;
            let ris = ((uart + 0x3c) as *const u32).read_volatile() as u64;
            let mis = ((uart + 0x40) as *const u32).read_volatile() as u64;
            let grp = ((gicd + 0x080 + (intid / 32) * 4) as *const u32).read_volatile();
            let pend = ((gicd + 0x200 + (intid / 32) * 4) as *const u32).read_volatile();
            let bit = 1u32 << (intid % 32);
            let gic = u64::from(grp & bit != 0) | (u64::from(pend & bit != 0) << 1);
            (fr << 48) | (ris << 32) | (mis << 16) | gic
        }
    }

    /// No BCM2711 PL011/GIC-400 pair off the Pi-4 board.
    #[cfg(not(feature = "board-rpi4"))]
    pub fn uart_gic() -> u64 {
        0
    }

    /// Store a value under [`Relaxed`] — single writer, read once at the end.
    pub fn set(cell: &AtomicU64, v: u64) {
        cell.store(v, Relaxed);
    }

    /// Show the recorded run on screen and stop the boot there.
    ///
    /// Called from `kernel_main_rust` in place of the rest of the boot, so the
    /// report cannot be overwritten by the console, the AUTO folder or the
    /// desktop. After the report it runs a **visible ten-second countdown**:
    /// that is the one measurement the recorded numbers cannot supply
    /// themselves — an independent check, against the user's own stopwatch,
    /// of whether this machine's `delay_ms`/`monotonic_millis` second is a
    /// real second.
    pub fn show(shiftbits: u8) -> ! {
        use crate::vdi_client::Vdi;
        use alloc::format;

        let mut v = Vdi::new();
        if !v.open() {
            // No workstation, no report — say so on the serial line and stop.
            crate::kprintln!("[diag] v_opnvwk failed; no report possible");
            crate::arch::aarch64::halt();
        }
        v.clear();
        v.set_text_align(0, 5); // left, top
        v.set_text_color(super::BLACK);

        let (cnt_now, frq) = timer();
        let d = |c: &AtomicU64| c.load(Relaxed);
        let hi = |c: &AtomicU64| d(c) >> 32;
        let lo = |c: &AtomicU64| d(c) & 0xFFFF_FFFF;
        // Keep every line <= 64 characters: `Vdi::gtext` passes the string
        // through INTIN, which is 64 words wide, and truncates beyond that.
        // Who filled the IKBD queue, and what the serial line delivered —
        // the 2026-09-04 phantom-key measurement's open questions.
        let mut plog = alloc::string::String::new();
        for slot in PUSH_LOG.iter() {
            let v = slot.load(Relaxed);
            if v == 0 {
                continue;
            }
            let src = match (v >> 16) as u8 {
                PUSH_SRC_IKBD => "kbd",
                _ => "?",
            };
            plog.push_str(&format!("{src}:{:04x} ", v & 0xFFFF));
        }
        if plog.is_empty() {
            plog.push_str("none");
        }
        let opt_byte = |c: &AtomicU32| match c.load(Relaxed) {
            0 => alloc::string::String::from("-"),
            v => format!("0x{:02x}", v - 1),
        };
        let reason = match REASON.load(Relaxed) {
            1 => "1 = modifier held (kbshift & MODE_SCA)",
            2 => "2 = console_has_input (RX ring not empty)",
            3 => "3 = deadline reached (full duration)",
            _ => "0 = the wait loop never ran",
        };
        let lines = [
            format!("NEWTOS BOOT-SCREEN DIAGNOSTIC ({}, {} s)",
                crate::BOARD_NAME, super::INITINFO_DURATION),
            format!("initinfo::run() entered : {}", ENTERED.load(Relaxed)),
            format!("host installed          : {}", HOST_OK.load(Relaxed)),
            format!("screen geometry         : {} x {}", hi(&DIMS), lo(&DIMS)),
            format!("v_opnvwk ok             : {}", VWK_OK.load(Relaxed)),
            format!("wait loop entered       : {}", WAITED.load(Relaxed)),
            format!("CNTFRQ_EL0              : {}", FRQ.load(Relaxed)),
            format!("CNTPCT in / out         : {} / {}", d(&CNT_IN), d(&CNT_OUT)),
            format!("CNTPCT delta            : {}", d(&CNT_OUT).wrapping_sub(d(&CNT_IN))),
            format!("millis in / deadline    : {} / {}", d(&MS_IN), d(&MS_END)),
            format!("millis out              : {}", d(&MS_OUT)),
            format!("millis elapsed in wait  : {}", d(&MS_OUT).wrapping_sub(d(&MS_IN))),
            format!("BCM SysTimer in / out   : {} / {}", d(&ST_IN), d(&ST_OUT)),
            format!("SysTimer delta (us)     : {}", d(&ST_OUT).wrapping_sub(d(&ST_IN))),
            format!("wait exit reason        : {reason}"),
            format!("inner / outer passes    : {} / {}", d(&ITERS), d(&OUTER)),
            format!("kbshift out / ever seen : 0x{:02x} / 0x{:02x}",
                SHIFT_OUT.load(Relaxed), SHIFT_SEEN.load(Relaxed)),
            format!("shiftbits returned      : 0x{shiftbits:02x}"),
            format!("key popped              : {}", match d(&KEY) {
                0 => alloc::string::String::from("none"),
                k => format!("0x{:04x}", k - 1),
            }),
            format!("RX ring head/tail in    : {} / {}", hi(&RING_IN), lo(&RING_IN)),
            format!("RX ring head/tail out   : {} / {}", hi(&RING_OUT), lo(&RING_OUT)),
            format!("USB ports / devices     : {} / {}", hi(&USB), lo(&USB)),
            format!("PL011 FR/RIS/MIS at wait: {:04x} / {:04x} / {:04x}",
                (d(&UART_GIC) >> 48) & 0xFFFF,
                (d(&UART_GIC) >> 32) & 0xFFFF,
                (d(&UART_GIC) >> 16) & 0xFFFF),
            format!("UART irq group1/pending : {} / {}   (now {:04x}/{:04x})",
                d(&UART_GIC) & 1, (d(&UART_GIC) >> 1) & 1,
                (uart_gic() >> 32) & 0xFFFF, uart_gic() & 3),
            format!("IKBD queue pushes/log    : {} / {}", PUSH_N.load(Relaxed), plog),
            format!("serial RX dropped/first  : {} / {}",
                SERIAL_DROPPED.load(Relaxed), opt_byte(&SERIAL_FIRST)),
            format!("UART init flush cnt/1st  : {} / {}",
                UART_FLUSHED.load(Relaxed), opt_byte(&UART_FLUSH_FIRST)),
            format!("CNTPCT now / CNTFRQ now : {cnt_now} / {frq}"),
        ];
        let mut y = 8i16;
        for l in lines.iter() {
            v.gtext(16, y, l.as_bytes());
            y += 17;
        }

        // Independent clock check: ten `delay_ms(1000)` steps, each redrawing
        // the counter. Timed against a stopwatch this says whether a second on
        // this machine is a second — which no self-reported number can.
        v.gtext(16, y, b"STOPWATCH: the counter below steps once per second, 1..10");
        y += 17;
        let bar_y = y;
        for i in 1..=10u32 {
            crate::arch::aarch64::delay_ms(1000);
            v.set_fill(WHITE_BG);
            v.fill_rect(16, bar_y, 16 + 62 * 8, bar_y + 16);
            v.set_text_color(super::BLACK);
            v.gtext(16, bar_y, format!("   ... {i} of 10").as_bytes());
        }
        v.gtext(16, bar_y + 17, b"DONE - the boot stops here. Please photograph this screen.");
        v.close();
        crate::arch::aarch64::halt();
    }

    /// The boot screen's background pen — the diagnostic report reuses it to
    /// erase the countdown line between steps.
    const WHITE_BG: i16 = super::WHITE;
}
