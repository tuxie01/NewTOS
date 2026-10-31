//! BCM2711 VideoCore framebuffer (allocated via the mailbox property
//! interface).
//!
//! Reference: [`docs/hardware/mailbox-framebuffer.md`]. Unlike QEMU
//! `ramfb` (where the guest allocates the pixel store and points the
//! device at it), here the **VideoCore allocates** the framebuffer: we
//! send one property message setting the geometry/depth/order/alpha mode
//! and an allocate tag, and the VC returns the buffer's bus address, size
//! and pitch.
//!
//! This is the RPi4 counterpart of `qemu_virt::ramfb`, behind the
//! `board-rpi4` feature, implementing the same [`Framebuffer`] trait.
//! First test vehicle is QEMU `raspi4b`; the GDB-observed mailbox
//! transaction is the primary success signal (see the hardware ref §5).

use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use crate::arch::aarch64::mmu::{va_to_pa, VA_OFFSET};
use crate::drivers::bcm2711::mailbox;
use crate::hal::framebuffer::Framebuffer;

/// Fallback display geometry (32-bit XRGB), used **only** when the VideoCore
/// get-physical query ([`query_physical_size`]) returns nothing. On real HW
/// the firmware/`config.txt` (`hdmi_group`/`hdmi_mode`) already drives the
/// HDMI mode, so [`init`] queries the *current* physical (display) size and
/// renders natively at it; these constants are the fallback when the query
/// yields 0/none (e.g. the emulated raspi4b VideoCore reports no configured
/// display). Shared with the QEMU `ramfb` default via
/// [`crate::drivers::BOOT_WIDTH`]/[`BOOT_HEIGHT`] (ADR-017 rev 2,
/// config.txt-driven). The detected-or-fallback geometry feeds the
/// set-physical (`0x00048003`) and set-virtual (`0x00048004`) tags below.
const FALLBACK_WIDTH: u32 = crate::drivers::BOOT_WIDTH;
const FALLBACK_HEIGHT: u32 = crate::drivers::BOOT_HEIGHT;
const DEPTH_BPP: u32 = 32;
/// VC pixel order. `0 = BGR, 1 = RGB` (firmware wiki). Our pixel store is
/// XRGB8888 packed by [`hal::framebuffer::rgb`] as `0xXX_RR_GG_BB`, i.e.
/// little-endian memory bytes `[B, G, R, X]` — blue at the lowest
/// address. The VC's **BGR** order reads the lowest byte as blue, which
/// matches; selecting RGB here swaps the red/blue channels (verified by
/// QMP screendump on raspi4b). So BGR is correct for our layout. The X
/// byte is the VDI overlay bit (`hal::framebuffer` module doc), not shown.
const PIXEL_ORDER_BGR: u32 = 0;
/// VC alpha mode, property tag `0x0004_8007` **set alpha mode** (`u32 state`,
/// 4 bytes, echoed in the response). The firmware wiki (*Mailbox property
/// interface*, doc source #4) defines the states:
///
/// > `0x0`: Alpha channel enabled (0 = fully opaque)
/// > `0x1`: Alpha channel reversed (0 = fully transparent)
/// > `0x2`: Alpha channel ignored
///
/// NewTOS asks for **ignored**: at depth 32 the top byte of every pixel is
/// the VDI overlay bit (`hal::framebuffer` module doc) — `0x00` on drawn
/// pixels, `0xFF` on cleared/complemented ones — and the display must show
/// the same colour either way. A firmware that treated the byte as alpha
/// would blend or blank one of the two. What the VideoCore does with the
/// byte *without* this tag is documented nowhere in the Track-B sources;
/// sending the documented mode removes the question instead of guessing it
/// (`docs/hardware/mailbox-framebuffer.md` §3c). `0` is the same value the
/// wiki lists for "enabled" — hence a named constant.
const ALPHA_MODE_IGNORED: u32 = 2;

/// Mask turning a VideoCore bus address into an ARM physical address:
/// the top 2 bits select the cache alias (hardware ref §4).
const BUS_TO_PHYS: u32 = 0x3FFF_FFFF;

// TASK/MEM11 (user decision 2026-10-31): `Physbase`/`Logbase` publish the
// screen memory at this physical address, so it must stay a positive `LONG`.
// The mask keeps the base below 1 GiB, and the store lies in the VideoCore's
// share of SDRAM, which ends at `0x4000_0000` (BCM2711 ARM Peripherals
// §1.2.3; `bios::machine::layout`, `board-rpi4`).
const _: () = assert!(
    (BUS_TO_PHYS as u64) < tos_api::bdos::process::usermem::USER_PTR_END,
    "MEM11: a masked VideoCore address can reach 0x8000_0000"
);

/// Property message buffer, 16-byte aligned (mailbox requirement). Sized
/// generously (36 words) to hold the seven-tag allocate sequence (34 words
/// incl. the end tag) plus 16-byte padding.
#[repr(C, align(16))]
struct MboxBuf {
    words: [u32; 36],
}

static mut MBOX_BUF: MboxBuf = MboxBuf { words: [0; 36] };

/// Indices into [`MBOX_BUF`] for the response fields we read back.
const I_PHYS_W: usize = 5; // set-physical width (echoed/adjusted)
const I_PHYS_H: usize = 6; // set-physical height
const I_VIRT_H: usize = 11; // set-virtual height (echoed — may be clamped)
const I_ALPHA_CODE: usize = 22; // set-alpha-mode → tag response code (b31 = answered)
const I_ALPHA: usize = 23; // set-alpha-mode → state (echoed — may differ)
const I_FB_BASE: usize = 27; // allocate → base (bus address)
const I_FB_SIZE: usize = 28; // allocate → size
const I_PITCH: usize = 32; // get-pitch → bytes per line

/// Indices into [`MBOX_BUF`] for the standalone get-physical query message
/// (see [`query_physical_size`]): the get-physical tag and the width/height
/// the VideoCore writes into its 8-byte value buffer.
const Q_TAG: usize = 2; // get-physical tag id
const Q_W: usize = 5; // response: physical width
const Q_H: usize = 6; // response: physical height

/// The installed framebuffer.
pub struct VcFb {
    /// High-half VA of page 0 of the allocated screen memory.
    mem: AtomicU64,
    /// High-half VA drawing goes to (EmuTOS `v_bas_ad`). Equals [`Self::mem`]
    /// until `Setscreen` moves it.
    draw: AtomicU64,
    /// High-half VA the VideoCore is currently scanning out. Equals
    /// [`Self::mem`] until `Setscreen` flips a page.
    scanout: AtomicU64,
    width: AtomicU32,
    height: AtomicU32,
    stride_px: AtomicU32,
    /// Number of full screens the allocated buffer holds (virtual height /
    /// physical height, as the VideoCore actually granted it).
    pages: AtomicU32,
}

/// Board-selected framebuffer instance.
pub static FRAMEBUFFER: VcFb = VcFb {
    mem: AtomicU64::new(0),
    draw: AtomicU64::new(0),
    scanout: AtomicU64::new(0),
    width: AtomicU32::new(0),
    height: AtomicU32::new(0),
    stride_px: AtomicU32::new(0),
    pages: AtomicU32::new(1),
};

impl VcFb {
    /// Bytes one full screen occupies (`stride_bytes * height`).
    fn page_bytes(&self) -> u64 {
        (self.stride_px.load(Ordering::Acquire) as u64)
            * 4
            * (self.height.load(Ordering::Acquire) as u64)
    }

    /// Offset of `va` from page 0, if `va` starts a whole screen page inside
    /// the allocated screen memory. `None` for anything else — an address
    /// outside the buffer or not on a page boundary is not a screen this
    /// display can show or draw into.
    fn page_offset(&self, va: u64) -> Option<u64> {
        let mem = self.mem.load(Ordering::Acquire);
        let page = self.page_bytes();
        if mem == 0 || page == 0 || va < mem {
            return None;
        }
        let off = va - mem;
        if !off.is_multiple_of(page) || off / page >= self.pages.load(Ordering::Acquire) as u64 {
            return None;
        }
        Some(off)
    }
}

impl Framebuffer for VcFb {
    fn width(&self) -> u32 {
        self.width.load(Ordering::Acquire)
    }
    fn height(&self) -> u32 {
        self.height.load(Ordering::Acquire)
    }
    fn stride_px(&self) -> u32 {
        self.stride_px.load(Ordering::Acquire)
    }
    fn pixels(&self) -> *mut u32 {
        self.draw.load(Ordering::Acquire) as *mut u32
    }
    fn mem_base(&self) -> *mut u32 {
        self.mem.load(Ordering::Acquire) as *mut u32
    }
    fn pages(&self) -> u32 {
        self.pages.load(Ordering::Acquire)
    }
    fn set_draw_base(&self, va: u64) -> bool {
        if self.page_offset(va).is_none() {
            return false;
        }
        self.draw.store(va, Ordering::Release);
        true
    }
    fn scanout_base(&self) -> u64 {
        self.scanout.load(Ordering::Acquire)
    }
    fn set_scanout_base(&self, va: u64) -> bool {
        let Some(off) = self.page_offset(va) else {
            return false;
        };
        if va == self.scanout.load(Ordering::Acquire) {
            return true; // already there — no mailbox round trip
        }
        let stride = self.stride_px.load(Ordering::Acquire) as u64 * 4;
        let y = (off / stride) as u32;
        // SAFETY: single-CPU; the boot-time framebuffer bring-up has long
        // finished, so `MBOX_BUF` is free (see `set_virtual_offset`).
        if !unsafe { set_virtual_offset(0, y) } {
            return false;
        }
        self.scanout.store(va, Ordering::Release);
        true
    }
}

/// Fill the property buffer with the framebuffer-allocate tag sequence for
/// the given physical geometry `w`×`h` (the detected-or-fallback size, see
/// [`init`]) and `pages` stacked screens of virtual height.
///
/// `pages = 1` is the classic single, non-scrolled framebuffer. `pages = 2`
/// asks the VideoCore for a buffer of **double virtual height** — two whole
/// screens, one shown while the other is drawn into — which is what makes the
/// set-virtual-offset tag ([`set_virtual_offset`]) a page flip and therefore
/// what makes XBIOS `Setscreen` able to move the physical base at all.
/// Reference: `docs/hardware/mailbox-framebuffer.md` §3/§3b.
///
/// # Safety
/// Single-CPU boot; `MBOX_BUF` is a BSS static.
#[allow(clippy::deref_addrof)] // see `ramfb::fwcfg_dma` for the static-mut idiom.
unsafe fn build_request(w: u32, h: u32, pages: u32) {
    // SAFETY: single-CPU boot context, exclusive access to the static.
    let b = unsafe { &mut (*(&raw mut MBOX_BUF)).words };

    b[0] = 0; // total size in bytes, patched below
    b[1] = 0x0000_0000; // request

    // set physical (display) width/height — tag 0x00048003, value 8B.
    b[2] = 0x0004_8003;
    b[3] = 8;
    b[4] = 0;
    b[5] = w;
    b[6] = h;

    // set virtual (buffer) width/height — tag 0x00048004, value 8B.
    // `pages` screens stacked vertically: the visible window stays `h` tall
    // and the extra rows are the off-screen page(s).
    b[7] = 0x0004_8004;
    b[8] = 8;
    b[9] = 0;
    b[10] = w;
    b[11] = h * pages;

    // set depth — tag 0x00048005, value 4B.
    b[12] = 0x0004_8005;
    b[13] = 4;
    b[14] = 0;
    b[15] = DEPTH_BPP;

    // set pixel order — tag 0x00048006, value 4B.
    b[16] = 0x0004_8006;
    b[17] = 4;
    b[18] = 0;
    b[19] = PIXEL_ORDER_BGR;

    // set alpha mode — tag 0x00048007, value 4B (`u32 state`, echoed). Sent
    // before `allocate` like the other mode tags so the buffer is created
    // with it. A firmware that does not know the tag ignores it and leaves
    // the tag's response bit clear (wiki: "Unknown tags are ignored (the
    // response bit will not be set)"); the allocation is not affected —
    // `init` reports what came back and carries on.
    b[20] = 0x0004_8007;
    b[21] = 4;
    b[22] = 0;
    b[23] = ALPHA_MODE_IGNORED;

    // allocate buffer — tag 0x00040001, value 8B (req: alignment; resp:
    // base + size).
    b[24] = 0x0004_0001;
    b[25] = 8;
    b[26] = 0;
    b[27] = 16; // requested alignment in bytes
    b[28] = 0;

    // get pitch — tag 0x00040008, value 4B (resp: bytes per line).
    b[29] = 0x0004_0008;
    b[30] = 4;
    b[31] = 0;
    b[32] = 0;

    // end tag.
    b[33] = 0x0000_0000;

    // Total size = whole buffer (incl. padding to a 16-byte multiple).
    b[0] = (b.len() * 4) as u32;
}

/// Query the VideoCore for the **current physical (display) width/height** —
/// the resolution the firmware/`config.txt` (`hdmi_group`/`hdmi_mode`) already
/// configured for the HDMI output — via property tag `0x0004_0003`, the GET
/// counterpart of the set-physical `0x0004_8003` tag used in [`build_request`]
/// (same 8-byte `u32 width, u32 height` value layout; the request value is
/// empty and the VC writes the geometry into it).
///
/// Reference: `docs/hardware/mailbox-framebuffer.md` §2 (buffer format) + §3
/// (the `0x0004_8003` set-physical layout this mirrors), sourced from the
/// firmware *Mailbox property interface* wiki (doc source #4). Per CLAUDE.md
/// Rule 6 the tag id/layout is sourced, not invented: the `0x0004_0xxx` ids
/// are the GET/action family (cf. `allocate 0x0004_0001`, `get pitch
/// 0x0004_0008` already in the doc), the `0x0004_8xxx` ids the SET family.
///
/// Returns `Some((w, h))` when the VC reports a valid non-zero geometry
/// (`config.txt` drove a real HDMI mode → render natively at it); `None` when
/// the transaction fails or the VC reports 0 (no configured display, e.g. the
/// emulated raspi4b path) → the caller falls back to
/// [`FALLBACK_WIDTH`]/[`FALLBACK_HEIGHT`].
///
/// # Safety
/// Boot CPU, single-threaded; exclusive use of the static buffer. Runs before
/// [`build_request`], which then overwrites the buffer for the allocate call.
#[allow(clippy::deref_addrof)]
unsafe fn query_physical_size() -> Option<(u32, u32)> {
    // SAFETY: single-CPU boot; exclusive use of the static buffer.
    unsafe {
        {
            let b = &mut (*(&raw mut MBOX_BUF)).words;
            b[1] = 0x0000_0000; // request

            // get physical (display) width/height — tag 0x00040003, value 8B.
            // Request value is empty; the VC writes width/height into b[5]/b[6].
            b[Q_TAG] = 0x0004_0003;
            b[Q_TAG + 1] = 8; // value buffer size = max(req 0, resp 8)
            b[Q_TAG + 2] = 0; // request code
            b[Q_W] = 0;
            b[Q_H] = 0;
            b[7] = 0x0000_0000; // end tag

            b[0] = 8 * 4; // total message size in bytes (header + 1 tag + end)
        }

        let buf_va = (&raw const MBOX_BUF) as usize;
        let buf_pa = va_to_pa(buf_va as u64) as u32;
        let len = core::mem::size_of::<MboxBuf>();

        if !mailbox::call_property(buf_va, buf_pa, len) {
            return None;
        }

        let b = &(*(&raw const MBOX_BUF)).words;
        let w = b[Q_W];
        let h = b[Q_H];
        if w != 0 && h != 0 {
            Some((w, h))
        } else {
            None
        }
    }
}

/// Indices into [`MBOX_BUF`] for the standalone get-alpha-mode query
/// (see [`query_alpha_mode`]).
const A_TAG: usize = 2; // get-alpha-mode tag id
const A_CODE: usize = 4; // tag request/response code
const A_STATE: usize = 5; // response: alpha mode state

/// Ask the VideoCore which alpha mode the framebuffer now has — property tag
/// `0x0004_0007` **get alpha mode**, request length 0, response `u32 state`
/// (4 bytes), the GET counterpart of the `0x0004_8007` set tag sent in
/// [`build_request`]. States as in [`ALPHA_MODE_IGNORED`]'s doc (firmware
/// wiki, *Mailbox property interface*, doc source #4; corroborated by
/// `references/circle/addon/linux/raspberrypi-firmware.h:81`
/// `RPI_FIRMWARE_FRAMEBUFFER_GET_ALPHA_MODE = 0x00040007`).
///
/// Returns `None` when the transaction fails or the firmware leaves the tag
/// unanswered (response bit clear — how a mailbox that does not implement
/// the tag presents itself, `docs/hardware/mailbox-framebuffer.md` §3a).
/// The boot log then says so instead of printing a number that was never
/// written.
///
/// # Safety
/// Boot CPU, single-threaded; exclusive use of the static buffer. Runs after
/// the allocate message has been consumed (its results copied out).
#[allow(clippy::deref_addrof)]
unsafe fn query_alpha_mode() -> Option<u32> {
    // SAFETY: single-CPU boot; exclusive use of the static buffer.
    unsafe {
        {
            let b = &mut (*(&raw mut MBOX_BUF)).words;
            b[1] = 0x0000_0000; // request
            b[A_TAG] = 0x0004_0007; // get alpha mode
            b[A_TAG + 1] = 4; // value buffer size = max(req 0, resp 4)
            b[A_CODE] = 0; // request code (request length 0)
            b[A_STATE] = 0;
            b[6] = 0x0000_0000; // end tag
            b[7] = 0; // pad to a 16-byte multiple
            b[0] = 8 * 4; // total message size in bytes
        }

        let buf_va = (&raw const MBOX_BUF) as usize;
        let buf_pa = va_to_pa(buf_va as u64) as u32;
        if !mailbox::call_property(buf_va, buf_pa, core::mem::size_of::<MboxBuf>()) {
            return None;
        }

        let b = &(*(&raw const MBOX_BUF)).words;
        if b[A_CODE] & mailbox::TAG_RESPONSE == 0 {
            return None;
        }
        Some(b[A_STATE])
    }
}

/// Property-buffer indices for the standalone set-virtual-offset message
/// (see [`set_virtual_offset`]).
const O_TAG: usize = 2; // set-virtual-offset tag id
const O_X: usize = 5; // request/response: X offset in pixels
const O_Y: usize = 6; // request/response: Y offset in pixels

/// Move the visible window inside the (taller-than-visible) framebuffer to
/// pixel offset `(x, y)` — property tag `0x0004_8009`
/// **SET_VIRTUAL_OFFSET**, request and response value `u32 nOffsetX,
/// u32 nOffsetY` (8 bytes).
///
/// This is the BCM2711's page flip: with a virtual height of `pages × h`
/// (see [`build_request`]), `y = page × h` selects which of the stacked
/// screens the display engine scans out. It is the hardware underneath
/// XBIOS `Setscreen`'s `physLoc`, the VideoCore's stand-in for the Atari
/// video-base register EmuTOS' `atari_setphys` writes
/// (`references/emutos/bios/screen.c:939-947`).
///
/// Tag id and value layout: `docs/hardware/mailbox-framebuffer.md` §3b,
/// sourced from `references/circle/include/circle/bcmpropertytags.h:65`
/// (`PROPTAG_SET_VIRTUAL_OFFSET`) + `include/circle/bcmpropertytags.h:282`
/// (`TPropertyTagVirtualOffset { u32 nOffsetX; u32 nOffsetY; }`) and
/// corroborated by `references/circle/addon/linux/raspberrypi-firmware.h:103`.
/// Per CLAUDE.md Rule 6 nothing here is from memory.
///
/// Returns `true` only when the transaction succeeded **and** the firmware
/// echoed back the offsets we asked for — Circle applies the same check
/// (`references/circle/lib/bcmframebuffer.cpp:386-399`), because a firmware
/// that cannot honour the offset answers with the one it kept.
///
/// # Safety
/// Single-CPU, exclusive use of [`MBOX_BUF`]. Boot-time bring-up
/// ([`init`]) has finished by the time a trap can call this, and traps are
/// serialised by the BDOS giant lock, so the two never overlap.
#[allow(clippy::deref_addrof)]
unsafe fn set_virtual_offset(x: u32, y: u32) -> bool {
    // SAFETY: caller's contract — exclusive use of the static buffer.
    unsafe {
        {
            let b = &mut (*(&raw mut MBOX_BUF)).words;
            b[1] = 0x0000_0000; // request
            b[O_TAG] = 0x0004_8009; // PROPTAG_SET_VIRTUAL_OFFSET
            b[O_TAG + 1] = 8; // value buffer size
            b[O_TAG + 2] = 8; // request parameter size
            b[O_X] = x;
            b[O_Y] = y;
            b[7] = 0x0000_0000; // end tag
            b[0] = 8 * 4; // total message size (header + 1 tag + end)
        }

        let buf_va = (&raw const MBOX_BUF) as usize;
        let buf_pa = va_to_pa(buf_va as u64) as u32;
        if !mailbox::call_property(buf_va, buf_pa, core::mem::size_of::<MboxBuf>()) {
            return false;
        }
        let b = &(*(&raw const MBOX_BUF)).words;
        b[O_X] == x && b[O_Y] == y
    }
}

/// How many whole screens we ask the VideoCore to allocate: two, so XBIOS
/// `Setscreen` can show one while a program draws into the other. The
/// firmware may grant fewer (it echoes the virtual height it accepted), and
/// [`init`] falls back to a single page if the double-height allocation
/// fails outright — a display that boots is worth more than one that flips.
const WANTED_PAGES: u32 = 2;

/// Bring up the VideoCore framebuffer. Returns `false` if the mailbox
/// transaction fails or returns an implausible buffer — the caller then
/// skips graphics (never a stubbed success).
///
/// # Safety
/// Boot CPU, single-threaded, after the MMU/heap are up.
#[allow(clippy::deref_addrof)]
pub unsafe fn init() -> bool {
    // SAFETY: single-CPU boot; exclusive use of the static buffer.
    unsafe {
        // Ask the VideoCore for the display geometry the firmware/config.txt
        // configured, and render natively at it; fall back to the compile-time
        // default when the VC reports none (ADR-017 rev 2, config.txt-driven).
        let (w, h) = match query_physical_size() {
            Some((qw, qh)) => {
                crate::kprintln!("[fb] VideoCore display geometry detected: {}x{}", qw, qh);
                (qw, qh)
            }
            None => {
                crate::kprintln!(
                    "[fb] VideoCore reports no display; falling back to {}x{}",
                    FALLBACK_WIDTH,
                    FALLBACK_HEIGHT,
                );
                (FALLBACK_WIDTH, FALLBACK_HEIGHT)
            }
        };

        // Ask for `WANTED_PAGES` stacked screens so `Setscreen` can flip
        // between them; if that allocation fails, retry with the classic
        // single screen. A booting display beats a flippable one that isn't
        // there.
        let mut wanted = WANTED_PAGES;
        let (fb_bus, fb_size, pitch, w, h, virt_h, alpha_code, alpha_echo) = loop {
            build_request(w, h, wanted);

            let buf_va = (&raw const MBOX_BUF) as usize;
            let buf_pa = va_to_pa(buf_va as u64) as u32;
            let len = core::mem::size_of::<MboxBuf>();

            let ok = mailbox::call_property(buf_va, buf_pa, len);
            let b = &(*(&raw const MBOX_BUF)).words;
            let got = (
                b[I_FB_BASE],
                b[I_FB_SIZE],
                b[I_PITCH],
                b[I_PHYS_W],
                b[I_PHYS_H],
                b[I_VIRT_H],
                b[I_ALPHA_CODE],
                b[I_ALPHA],
            );
            // A zero base / pitch means the VC did not allocate (e.g. the
            // emulated path is incomplete) — bail out cleanly.
            if ok && got.0 != 0 && got.1 != 0 && got.2 != 0 {
                break got;
            }
            if wanted == 1 {
                return false;
            }
            crate::kprintln!(
                "[fb] double-height allocation refused; retrying single-page",
            );
            wanted = 1;
        };

        // How many whole screens did we actually get? Believe the two figures
        // the firmware returned — the echoed virtual height and the allocated
        // byte size — not the number we asked for.
        let page_bytes = (pitch as u64) * (h as u64);
        let pages = (virt_h / h.max(1))
            .min((fb_size as u64 / page_bytes.max(1)) as u32)
            .max(1);

        // Alpha mode: what the set tag echoed (the wiki says the response
        // "may not be the same as the request so it must be checked"), and
        // what the VideoCore reports the framebuffer has now. One line; the
        // qualifiers appear only when something deviates from the request.
        let set_answered = alpha_code & mailbox::TAG_RESPONSE != 0;
        match query_alpha_mode() {
            Some(mode) if set_answered && alpha_echo == ALPHA_MODE_IGNORED => {
                crate::kprintln!("[fb] alpha mode = {}", mode);
            }
            Some(mode) if set_answered => {
                crate::kprintln!(
                    "[fb] alpha mode = {} (set tag echoed {}, asked {})",
                    mode,
                    alpha_echo,
                    ALPHA_MODE_IGNORED,
                );
            }
            Some(mode) => {
                crate::kprintln!("[fb] alpha mode = {} (set tag unanswered)", mode);
            }
            None => {
                crate::kprintln!(
                    "[fb] alpha mode = not reported (get tag unanswered; set tag {})",
                    if set_answered { "answered" } else { "unanswered" },
                );
            }
        }

        let fb_phys = (fb_bus & BUS_TO_PHYS) as u64;
        let fb_va = fb_phys + VA_OFFSET; // draw through the high half

        // Map the framebuffer Normal Cacheable Write-Back Write-Allocate so
        // CPU pixel writes hit the cache (line-fill + write-allocate + prefetch)
        // — software blits/fills are catastrophically slow through the previous
        // Non-Cacheable mapping on real HW, where every access is one uncached
        // DRAM transaction. The display controller scans out DRAM beyond the ARM
        // caches, so with a cacheable FB every drawn region must be cleaned to
        // PoC: NewTOS routes all FB writes through the `VdiScreen` backend, which
        // cleans each primitive's rectangle row by row (`clean_dcache_rect`
        // — GUI06; before that, the enclosing address span), so no stale
        // pixels result. The VC never writes the FB, so no invalidate is needed.
        // QEMU has no cache model, so cacheable vs. NC render identically there;
        // this is a real-RPi4 speed fix (and the no-stale-pixels correctness is
        // HW-verified).
        // SAFETY: boot CPU, Stage-2 tables installed; the VC-allocated FB
        // holds no executing code or the active stack.
        crate::arch::aarch64::mmu::remap_normal_wb(fb_va, fb_size as u64);

        FRAMEBUFFER.mem.store(fb_va, Ordering::Release);
        FRAMEBUFFER.draw.store(fb_va, Ordering::Release);
        FRAMEBUFFER.scanout.store(fb_va, Ordering::Release);
        FRAMEBUFFER.width.store(w, Ordering::Release);
        FRAMEBUFFER.height.store(h, Ordering::Release);
        FRAMEBUFFER.stride_px.store(pitch / 4, Ordering::Release);
        FRAMEBUFFER.pages.store(pages, Ordering::Release);
        crate::kprintln!(
            "[fb] {}x{} pitch {} virtual height {} -> {} screen page(s), {} KiB",
            w,
            h,
            pitch,
            virt_h,
            pages,
            fb_size / 1024,
        );
        true
    }
}
