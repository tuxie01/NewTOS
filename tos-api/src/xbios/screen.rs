//! XBIOS screen group (Phase 10): `Physbase` / `Logbase` / `Getrez` /
//! `Setscreen` / `Setpalette` / `Setcolor`. Mirrors the API surface of
//! EmuTOS `bios/screen.c` on top of the host framebuffer
//! ([`crate::host::ScreenInfo`]).
//!
//! Spec: `docs/api/xbios/{02-physbase,03-logbase,04-getrez,05-setscreen,
//! 06-setpalette,07-setcolor}.md`.
//!
//! The logical and physical screen bases are **not** software shadows: they
//! live where EmuTOS keeps them — the logical one in the display's own
//! `v_bas_ad` (our framebuffer's draw pointer, which the VDI backend
//! actually draws through) and the physical one in the hardware, read back
//! from it (`bios/screen.c:939-952,1030,1058-1071`). `Setscreen` moves them
//! for real: the VideoCore's screen memory holds two whole screens and the
//! set-virtual-offset mailbox tag selects which one is scanned out
//! (`docs/hardware/mailbox-framebuffer.md` §3b), so a program can draw into
//! the page it cannot see and then show it.
//!
//! Resolution is **not** a shadow: per ADR-017 rev 2 (config.txt-driven,
//! no in-system switch), `Getrez` reports a fixed honest code and
//! `Setscreen`'s `rez` is accepted-but-ignored — there is no in-system
//! resolution change (the firmware `config.txt` owns the HDMI mode).
//!
//! The palette (`Setpalette` / `Setcolor`) is the 16-entry ST(e) palette
//! shadow that stands in for EmuTOS' ST(e) palette registers, seeded with
//! EmuTOS' boot palette (`dflt_palette & 0x0FFF`). It does **not** reach the
//! VDI colour table: NewTOS's VDI is the truecolor one, and on EmuTOS's
//! Falcon/truecolor profile the XBIOS ST palette is not the VDI palette
//! either (owner decision 2026-11-12, `TASK/XBIOS21.md`; spec
//! `docs/api/xbios/07-setcolor.md` "Boot palette" and Pi binding).

use crate::bdos::process::{self, usermem};
use crate::host;
use crate::trap::TrapArgs;
use tos_types::{Long, ProtMode};

/// Atari resolution code `TT_MEDIUM` (`bios/screen.h`): 640×480, 16
/// colours — the classic code whose colour depth matches NewTOS's native
/// 16-colour framebuffer. `Getrez` reports this as an honest "not
/// monochrome, 16 colours" indicator; the actual geometry is
/// config.txt-driven and read via VDI `DEV_TAB` / `Physbase`, not this
/// coarse code (ADR-017 rev 2).
const TT_MEDIUM: i64 = 4;

/// High-half kernel VA offset — the whole of DRAM is mapped at
/// `phys + VA_OFFSET` for EL1. Mirrored here the way
/// [`crate::bdos::process::arena`], `loader` and `usermem` mirror it.
const VA_OFFSET: u64 = 0xFFFF_0000_0000_0000;

/// The whole screen memory, expressed **in the calling process's address
/// space**, plus the caller-space address of `kva` inside it.
///
/// The framebuffer is mapped for EL1 at `phys + VA_OFFSET`, high-half and
/// EL1-only, so returning that pointer to an EL0 program satisfies the letter
/// of `Physbase`'s contract ("a pointer to the screen") and violates its
/// substance: the program faults on the first store. ADR-023 settles the
/// class — what the kernel hands out, the caller must be able to reach — so
/// an EL0 caller gets the framebuffer published into the shared user address
/// space at its own address ([`usermem::expose_attr`], VA = PA since ARCH03
/// stage 1) and that **user address** back. An EL1 caller (the
/// desktop, the boot self-tests, the VDI) gets the kernel VA, which is what
/// it can reach.
///
/// **The whole screen memory is published, not just the visible screen.**
/// The VideoCore buffer holds [`host::ScreenInfo::mem_bytes`] bytes — two
/// stacked screens — and a program that wants to double-buffer has to reach
/// both: it computes the hidden page as `Logbase() + size_bytes` and hands
/// that back to `Setscreen`. Publishing the pair as **one** mapping is what
/// makes that arithmetic valid in the process's own address space.
///
/// Two decisions are attached to that mapping and neither is the program's
/// business:
///
/// * **`PROT_G`, not `PROT_PR`.** `Physbase`/`Logbase` exist so a program can
///   *write* pixels; a read-only screen would be a pointer that still cannot
///   be used for its purpose. `PROT_G` is FreeMiNT's "global — any access
///   OK" mode (`references/freemint/sys/mint/mem.h:130-134`).
/// * **Normal Non-Cacheable** ([`usermem::MemAttr::Uncached`]). The pixels
///   are read out by the VideoCore display engine, a DMA master that scans
///   DRAM from outside the ARM caches
///   (`docs/hardware/mailbox-framebuffer.md` §6). The kernel's own alias is
///   Write-Back and the VDI cleans every drawn rectangle to the point of
///   coherency itself; an EL0 program has no such hook, and requiring one
///   would mean inventing a cache-maintenance call the TOS API does not
///   have. Uncached pages make a plain store land in DRAM — the program
///   writes pixels and they appear, exactly as on the flat machine the API
///   was written for.
///
/// Returns `0` when the publication cannot be made — the same value the call
/// already produces on a display-less machine, and never an address the
/// caller would fault on.
fn caller_view(info: &host::ScreenInfo, kva: u64) -> u64 {
    match process::slot_for(process::current()) {
        // EL1 caller: the kernel high-half VA *is* the reachable address.
        None => kva,
        // SAFETY: BDOS lock held (single-CPU SVC context). The framebuffer
        // is a permanently-mapped high-half region that outlives every
        // process, and `mem_bytes` is the extent of its pixel store.
        Some(slot) => match unsafe {
            usermem::expose_attr(
                slot,
                info.mem_base as *const u8,
                info.mem_bytes as usize,
                ProtMode::G,
                usermem::MemAttr::Uncached,
            )
        } {
            Some(window) => window + (kva - info.mem_base),
            None => 0,
        },
    }
}

/// `Physbase` (0x02) — physical (displayed) screen base.
///
/// Reads the **hardware** back, as EmuTOS' `physbase()` does
/// (`bios/screen.c:1030` → `atari_physbase()`, which reconstructs the address
/// from the video-base registers rather than from a variable), so the value
/// a program gets is where the display engine is really scanning from — not
/// where it was last asked to scan from.
pub fn physbase() -> i64 {
    let Some(info) = host::host().and_then(|h| h.screen()) else {
        return 0;
    };
    caller_view(&info, info.phys_base) as i64
}

/// `Logbase` (0x03) — logical screen base (where drawing goes).
///
/// EmuTOS returns `v_bas_ad` (`bios/screen.c:1071`). Ours lives in the
/// display itself — it is the pointer the VDI raster backend draws through —
/// so this reports the address that is genuinely the drawing target.
pub fn logbase() -> i64 {
    let Some(info) = host::host().and_then(|h| h.screen()) else {
        return 0;
    };
    caller_view(&info, info.base) as i64
}

/// Is a process currently holding a direct, uncached mapping of the
/// framebuffer handed out by [`physbase`] / [`logbase`]?
///
/// The kernel's drawing backend asks once per primitive: while the answer is
/// `true`, the same physical pixels are reachable through two aliases with
/// **mismatched cacheability** — the VDI's Write-Back one and the program's
/// Non-Cacheable one — and the architecture only guarantees coherency if
/// software keeps the region out of the caches. The backend therefore cleans
/// *and invalidates* each drawn rectangle instead of merely cleaning it, so
/// no line of the framebuffer survives a primitive to be written back later
/// over the program's own stores (`kernel/src/vdi_backend.rs`).
pub fn direct_screen_active() -> bool {
    usermem::uncached_shared_active()
}

/// `Getrez` (0x04) — current resolution code.
///
/// NewTOS is a native 16-colour framebuffer, so `Getrez` reports the
/// classic 16-colour code `TT_MEDIUM` (4) as an honest colour-depth
/// indicator (the one thing this coarse code conveys that VDI `DEV_TAB` /
/// `Physbase` don't: colour depth / *not monochrome*). It is **not** a
/// geometry claim — the live geometry is config.txt-driven and read from
/// VDI `DEV_TAB` / `Physbase`, not from this value (ADR-017 rev 2). Revisit
/// at the truecolor VDI step, when a wider-depth code may be more honest.
pub fn getrez() -> i64 {
    TT_MEDIUM
}

/// Translate a screen base as the **caller** wrote it into the kernel
/// high-half VA the display speaks.
///
/// An EL0 program can only have got a screen address from `Physbase` /
/// `Logbase` — the published screen memory (ADR-023), possibly plus a whole
/// number of screens — so the publication is the authority
/// ([`usermem::resolve_kva`]). An EL1 caller holds kernel VAs; a value below
/// [`VA_OFFSET`] from EL1 is read as a physical address, which is the other
/// spelling of the same DRAM.
///
/// `None` means "the caller named memory this process has no mapping for",
/// which is neither a screen page nor anything the kernel may dereference.
fn caller_base_to_kva(addr: u64) -> Option<u64> {
    match process::slot_for(process::current()) {
        None if addr >= VA_OFFSET => Some(addr),
        None => Some(addr.wrapping_add(VA_OFFSET)),
        Some(slot) => usermem::resolve_kva(slot, addr),
    }
}

/// `Setscreen` (0x05) — set logical/physical base; `rez` is
/// accepted-but-ignored.
/// `x0` = logLoc, `x1` = physLoc (each `> 0` to apply), `x2` = rez,
/// `x3` = videlmode (ignored — no Falcon).
///
/// Each base is applied only when strictly **positive** — `0` and any
/// negative value mean "leave it alone", the convention EmuTOS spells as
/// `if ((LONG)logLoc > 0)` / `if ((LONG)physLoc > 0)`
/// (`bios/screen.c:1106,1110`).
///
/// **Both moves are real.** `logLoc` becomes the drawing target (EmuTOS
/// `v_bas_ad = logLoc`): the VDI raster backend, the VT52 console and the
/// desktop all draw through the display's draw pointer, so after this call
/// they draw into the named screen. `physLoc` goes to the display engine
/// (EmuTOS `setphys(physLoc)`): on the BCM2711 that is the mailbox
/// set-virtual-offset tag selecting which of the stacked screens the
/// VideoCore scans out (`docs/hardware/mailbox-framebuffer.md` §3b).
///
/// A base the display cannot use — outside its screen memory, or not on a
/// screen-page boundary — leaves that base where it was. The hardware simply
/// has no way to represent it, exactly as a plain ST's video-base register
/// cannot represent the low byte of an address and silently drops it
/// (`atari_setphys`, `bios/screen.c:939-947`); and, as there, the following
/// `Physbase`/`Logbase` reports what the hardware really does, so a program
/// that checks is told the truth rather than its own request echoed back.
///
/// There is no in-system resolution switch (ADR-017 rev 2: the firmware
/// `config.txt` owns the HDMI mode). The `rez` parameter is therefore
/// accepted for *any* value and ignored — no `REZ` shadow, no out-of-range
/// rejection — mirroring EmuTOS silently ignoring a hardware-unsupported
/// mode (`bios/screen.c:990-1013`). Always returns `0` (E_OK).
///
/// # KNOWN DEFECT — the physical base is applied at once, never latched
///
/// **A page flip through this call is not atomic, and no application can
/// work around it.** `physLoc` takes effect the moment it is asked for:
/// `set_screen_physical` reaches `vdi_backend::set_scanout_base`, which
/// reaches `drivers::bcm2711::framebuffer::set_scanout_base` and issues a
/// *synchronous* VideoCore mailbox property write (`set_virtual_offset`,
/// tag `0x0004_8009`). Nothing anywhere holds the new base back until the
/// next vertical blank — there is no pending-base register and no retrace
/// interrupt to install one.
///
/// The consequence is structural rather than intermittent. `Vsync` only
/// reports that the blank has *started*; what remains of it is short, and
/// the mailbox round trip is not bounded inside it. So the scan-out address
/// changes part-way down the visible picture and the frame is shown half
/// from one page and half from the other. Note that this does not depend on
/// the mailbox being slow: **even a free write applied immediately would
/// tear**, because there is no latch at all. The latency only decides how
/// far down the seam falls.
///
/// Observed on real hardware (Raspberry Pi 4, 2026-09-09) with the
/// bouncing-ball demo's `F2` diagnostic build, which holds the ball still
/// and turns only its texture: the changed region is then a fixed rectangle,
/// so the seam stands still and is unmistakable. With the ball travelling
/// the seam moves with the dirty rectangle every frame, which is why it went
/// unremarked. The demo's platform half documents the same finding from the
/// caller's side (the demo left this tree on 2026-09-09; see its
/// `plat-newtos.c`, `plat_present`), and its log
/// no longer claims a whole flip.
///
/// # What a repair needs, and what must be settled first
///
/// The shape is clear: remember the requested base and let the retrace
/// install it, so `Setscreen` queues rather than applies. That is a
/// kernel/XBIOS change — a pending-base slot, a vertical-blank interrupt
/// that writes it, and a decision about what `Physbase` reports between the
/// request and the blank.
///
/// Two things must come from the reference rather than from this symptom,
/// and are deliberately **not** decided here:
///
/// 1. The exact timing contract TOS offers. On Atari hardware the video base
///    registers are read by the shifter once per frame, which is what makes
///    ST double buffering whole; the precise latching rule, and what
///    `Physbase` is required to return while a flip is pending, must be
///    taken from EmuTOS (`bios/screen.c`, `atari_setphys`) and the spec
///    under `docs/api/xbios/05-setscreen.md`, not inferred from this bug.
/// 2. Whether the BCM2711 can latch a scan-out change at the blank at all
///    through the mailbox interface, or whether it needs the VideoCore's own
///    vsync tag. That is a `docs/hardware/mailbox-framebuffer.md` question.
///
/// Nothing about the behaviour below is changed by this note; it records
/// what the call does so the next person starts from the defect rather than
/// from the symptom.
pub fn setscreen(args: &TrapArgs) -> i64 {
    let log = args.arg_u64(0);
    let phys = args.arg_u64(1);
    let Some(host) = host::host() else {
        return 0;
    };

    if (log as i64) > 0 {
        if let Some(kva) = caller_base_to_kva(log) {
            host.set_screen_logical(kva);
        }
    }
    if (phys as i64) > 0 {
        if let Some(kva) = caller_base_to_kva(phys) {
            host.set_screen_physical(kva);
        }
    }
    0
}

/// `Setpalette` (0x06) — load all 16 entries of the ST(e) palette from the
/// caller's 16 packed words. `x0` = palette ptr; a null pointer is ignored.
///
/// The words go into the palette shadow (`& 0x0FFF`, STe class), which
/// `Setcolor(n, -1)` reads back — the store EmuTOS shares between the two
/// calls (the ST(e) registers, `bios/vectors.S:358-383`). EmuTOS defers the
/// load to the next VBL; NewTOS has no ST(e) palette to keep tear-free and
/// applies it at once. Like EmuTOS on its truecolor VDI, the write does not
/// touch the VDI colour table (owner decision 2026-11-12, XBIOS21). Spec:
/// `docs/api/xbios/06-setpalette.md` (Pi/BCM2711 binding).
pub fn setpalette(args: &TrapArgs) -> i64 {
    let ptr = args.arg_u64(0);
    if ptr == 0 {
        return 0;
    }
    // The caller's 16-WORD palette, read in the caller's space (ARCH03 3a,
    // `crate::uaccess`) before anything changes: a table the program cannot
    // read ends the call (and the program) with the palette untouched.
    let mut words = [0u16; 16];
    for (i, w) in words.iter_mut().enumerate() {
        match args.space.get_u16(ptr + 2 * i as u64) {
            Ok(v) => *w = v,
            Err(_) => return i64::from(crate::uaccess::FAULTED),
        }
    }
    for (slot, &w) in palette().iter_mut().zip(words.iter()) {
        *slot = w & PALETTE_MASK;
    }
    0
}

/// `Setcolor` (0x07) — set or query one ST(e) palette entry (EmuTOS
/// `atari_setcolor`, `bios/screen.c:1015-1036`). `x0` = index, masked
/// `& 0x0F`; `x1` = packed colour word, or any negative value to query.
/// Returns the entry's previous word `& 0x0FFF`; a query writes nothing.
/// After boot the entries hold `DFLT_PALETTE`. The VDI colour table is
/// not touched (owner decision 2026-11-12, XBIOS21). Spec:
/// `docs/api/xbios/07-setcolor.md`.
pub fn setcolor(args: &TrapArgs) -> i64 {
    let index = (args.arg_u16(0) & 0x0F) as usize;
    let value = args.arg_i16(1);
    let pal = palette();
    let old = (pal[index] & PALETTE_MASK) as Long as i64;
    if value >= 0 {
        pal[index] = (value as u16) & PALETTE_MASK;
    }
    old
}

/// The STe-class palette word mask `0x0FFF` (4 bits per gun): EmuTOS uses it
/// on every profile with an STe shifter, TT shifter or Videl
/// (`bios/screen.c:437-440,1026-1029`); NewTOS has no plain-ST `0x0777`
/// profile (owner decision 2026-11-12, XBIOS21 question 3).
const PALETTE_MASK: u16 = 0x0FFF;

/// EmuTOS' boot palette of the 16 ST(e) registers: `dflt_palette[]`
/// (`bios/screen.c:54-59`, values `bios/screen.h:56-71`) `& 0x0FFF`, as
/// `initialise_ste_palette` loads it (`bios/screen.c:64-71`, called from
/// `initialise_palette_registers`, `:432-457`). Register order, not VDI pen
/// order: register 1 is red, register 15 black. Measured on the oracle:
/// `Setcolor(0,-1) = 0x0FFF`, `Setcolor(1,-1) = 0x0F00` (bios-r1 cases
/// 93/94, `measurements/measurement-2026-11-12-xbios-full/RUNINFO.TXT`).
const DFLT_PALETTE: [u16; 16] = [
    0x0FFF, // RGB_WHITE
    0x0F00, // RGB_RED
    0x00F0, // RGB_GREEN
    0x0FF0, // RGB_YELLOW
    0x000F, // RGB_BLUE
    0x0F0F, // RGB_MAGENTA
    0x00FF, // RGB_CYAN
    0x0555, // RGB_LTGRAY
    0x0333, // RGB_GRAY
    0x0F33, // RGB_LTRED
    0x03F3, // RGB_LTGREEN
    0x0FF3, // RGB_LTYELLOW
    0x033F, // RGB_LTBLUE
    0x0F3F, // RGB_LTMAGENTA
    0x03FF, // RGB_LTCYAN
    0x0000, // RGB_BLACK
];

// ── Cursconf (VT52 text-cursor config) ───────────────────────────────────

/// `Cursconf` (0x15) — configure the VT52 text cursor (EmuTOS `cursconf`,
/// `bios/vt52.c:893-914`): `function` 0 hide (`cursor_off`), 1 show
/// (`cursor_on`), 2 clear `M_CFLASH` (steady), 3 set `M_CFLASH` (blinking),
/// 4 set the blink period (`operand` low byte), 5 return the period; any
/// other value is a no-op. Only subfunction 5 returns a value, all others 0.
/// The state is the alpha console's (`crate::vdi::alpha_cursconf`), the
/// same one `ESC e`/`ESC f` drive; without a screen the state still
/// changes and nothing is drawn.
pub fn cursconf(args: &TrapArgs) -> i64 {
    let function = args.arg_i16(0);
    let operand = args.arg_i16(1);
    let backend = crate::host::host().and_then(|h| h.raster());
    i64::from(crate::vdi::alpha_cursconf(backend, function, operand))
}

/// The 16-entry ST(e) palette shadow (`0x0RGB`, 4 bits per gun), the
/// NewTOS stand-in for EmuTOS' ST(e) palette registers. Its boot state is
/// [`DFLT_PALETTE`] (the static's initial value, loaded with the kernel
/// image), as EmuTOS' `screen_init` loads the registers at boot.
fn palette() -> &'static mut [u16; 16] {
    use core::cell::UnsafeCell;
    struct Pal(UnsafeCell<[u16; 16]>);
    // SAFETY: single-CPU, IRQ-off trap path.
    unsafe impl Sync for Pal {}
    static PAL: Pal = Pal(UnsafeCell::new(DFLT_PALETTE));
    // SAFETY: single trap path; never concurrently borrowed.
    unsafe { &mut *PAL.0.get() }
}
