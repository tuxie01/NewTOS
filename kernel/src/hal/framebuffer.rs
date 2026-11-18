//! Framebuffer HAL trait.
//!
//! A linear 32-bit (XRGB8888) framebuffer, board-independent. Concrete
//! implementations:
//!   * `drivers::qemu_virt::ramfb` — QEMU `ramfb` via fw_cfg (the dev
//!     vehicle; verified headless with QMP `screendump`).
//!   * `drivers::bcm2711` — VideoCore mailbox framebuffer (RPi4; HW
//!     bring-up later).
//!
//! The TOS graphics stack (VDI / Line-A, later phases) and a future
//! on-screen VT52 console sit on top of this; for now it carries a
//! boot test pattern.
//!
//! # Pixel format: `0xXX_RR_GG_BB`
//!
//! Bits 0..23 are the colour the display shows (little-endian memory bytes
//! `[B, G, R, X]`; the VideoCore is told pixel order BGR for exactly that
//! reason, `drivers/bcm2711/framebuffer.rs`). The top byte **X is not a
//! colour and the display does not show it** — `ramfb` is programmed with
//! the DRM `XRGB8888` fourcc, whose X is "don't care", and the VideoCore is
//! given depth 32 **and alpha mode "ignored"** (set-alpha-mode tag
//! `0x0004_8007` = 2 in the allocate message, `docs/hardware/mailbox-
//! framebuffer.md` §3c; since 2026-09-21 — before that no alpha-mode tag was
//! sent and the byte's treatment rested on an undocumented default). The
//! byte is owned by the VDI backend (`crate::vdi_backend`). At 16 planes it
//! keeps the one bit of the 16-bit Falcon screen word that carries no
//! colour: `OVERLAY_BIT` (bit 5, EmuTOS `vdi/vdi_fill.c:26`), replicated into
//! all eight X bits — `0xFF` set, `0x00` clear (V11/V12, 2026-09-21). At
//! 32 planes the store word is the device pixel and X is unused: pens write
//! 0, every VDI read masks it off (ADR-038 point 4, TASK/V63). Everything
//! outside the VDI packs its colours with [`rgb`], i.e. with X = 0.

/// Pack a plain RGB colour into the framebuffer pixel format: X = 0x00,
/// the VDI overlay bit clear (see the module doc).
#[inline]
pub const fn rgb(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

/// A linear XRGB8888 framebuffer.
///
/// # Safety
///
/// Implementations return a raw mutable pointer to the pixel store;
/// callers must respect [`Framebuffer::width`] / [`Framebuffer::height`]
/// / [`Framebuffer::stride_px`] bounds.
pub trait Framebuffer: Sync {
    /// Visible width in pixels.
    fn width(&self) -> u32;
    /// Visible height in pixels.
    fn height(&self) -> u32;
    /// Row stride in **pixels** (`stride_bytes / 4`).
    fn stride_px(&self) -> u32;
    /// Base pointer to the pixel store (XRGB8888, one `u32` per pixel),
    /// addressed through the kernel's normal (high-half) mapping.
    ///
    /// This is the **logical** screen — where drawing goes, EmuTOS'
    /// `v_bas_ad` (`bios/screen.c:1071`). With more than one
    /// [`Framebuffer::pages`] it need not be the page the display engine
    /// currently scans out ([`Framebuffer::scanout_base`]).
    fn pixels(&self) -> *mut u32;

    // ── Screen memory + page flipping ────────────────────────────────────
    //
    // A display whose allocated screen memory holds more than one full
    // screen can show one page while the CPU draws into another — the
    // hardware half of XBIOS `Setscreen` (0x05). The default implementations
    // describe a single-page display that cannot move its scan-out: an
    // honest "this board has one screen and it is the one you see", not a
    // stub — `set_scanout_base` refusing is the same answer EmuTOS' `setphys`
    // gives on a machine built without `CONF_WITH_ATARI_VIDEO`, where the
    // physical base simply cannot be programmed (`bios/screen.c:1058-1068`).

    /// Base pointer of the whole screen memory — page 0, the lowest address
    /// of the allocated buffer. [`Framebuffer::pixels`] and
    /// [`Framebuffer::scanout_base`] always lie inside
    /// `[mem_base, mem_base + pages * height * stride)`.
    fn mem_base(&self) -> *mut u32 {
        self.pixels()
    }

    /// Number of full screens (`height * stride_px` pixels each) the
    /// allocated screen memory holds. `1` means no page flipping.
    fn pages(&self) -> u32 {
        1
    }

    /// Point drawing at `va` (a high-half VA inside the screen memory) —
    /// the `v_bas_ad` half of `Setscreen`. Returns `false` if `va` is not the
    /// base of a whole page inside the screen memory.
    ///
    /// The default accepts only the page drawing is already on, which is the
    /// complete truth for a single-page display: `Setscreen(Logbase(), …)`
    /// succeeds (it asks for no move), any other address fails.
    fn set_draw_base(&self, va: u64) -> bool {
        va == self.pixels() as u64
    }

    /// High-half VA of the screen the display engine is currently scanning
    /// out — EmuTOS' `physbase()`, which likewise reads the *hardware*
    /// back rather than a shadow (`bios/screen.c:939-952,1030`).
    fn scanout_base(&self) -> u64 {
        self.mem_base() as u64
    }

    /// Point the display engine at `va` (a high-half VA inside the screen
    /// memory) — the `setphys` half of `Setscreen`. Returns `false` when the
    /// hardware cannot represent that address, in which case the scan-out is
    /// left exactly where it was and [`Framebuffer::scanout_base`] keeps
    /// reporting the truth.
    ///
    /// The default accepts only the screen already displayed — a display that
    /// cannot move its scan-out can still honour a request not to move it.
    fn set_scanout_base(&self, va: u64) -> bool {
        va == self.scanout_base()
    }

    /// Set one pixel. Out-of-bounds coordinates are ignored.
    fn put_pixel(&self, x: u32, y: u32, color: u32) {
        if x >= self.width() || y >= self.height() {
            return;
        }
        let off = (y * self.stride_px() + x) as usize;
        // SAFETY: bounds checked above; `pixels()` is valid for
        // `height * stride_px` u32 writes.
        unsafe { self.pixels().add(off).write_volatile(color) };
    }

    /// Fill a rectangle (clipped to the visible area).
    fn fill_rect(&self, x: u32, y: u32, w: u32, h: u32, color: u32) {
        let x1 = (x + w).min(self.width());
        let y1 = (y + h).min(self.height());
        let stride = self.stride_px();
        let base = self.pixels();
        for yy in y..y1 {
            let row = (yy * stride) as usize;
            for xx in x..x1 {
                // SAFETY: xx<width, yy<height ⇒ in-bounds.
                unsafe { base.add(row + xx as usize).write_volatile(color) };
            }
        }
    }

    /// Fill the whole framebuffer with one colour.
    fn clear(&self, color: u32) {
        self.fill_rect(0, 0, self.width(), self.height(), color);
    }
}
