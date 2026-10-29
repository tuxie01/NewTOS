//! Host abstraction.
//!
//! TOS-API code does not call kernel internals directly. Instead the
//! kernel installs a `&'static dyn Host` once at boot via
//! [`install_host`]; every TOS function then routes through this
//! interface. This keeps `tos-api` independent of the concrete UART /
//! GIC / framebuffer drivers and makes the surface unit-testable.

use core::sync::atomic::{AtomicPtr, Ordering};

/// Kernel-supplied host environment.
///
/// All methods may be called from inside an exception handler with
/// interrupts disabled — implementations must therefore be lock-free
/// or use IRQ-safe locks. Phase 02 only requires console I/O.
pub trait Host: Sync {
    /// Write a single byte to the console output device.
    fn console_write_byte(&self, byte: u8);

    /// Write each byte of `s` to the console output device. Default
    /// implementation loops over [`Host::console_write_byte`].
    fn console_write_bytes(&self, s: &[u8]) {
        for &b in s {
            self.console_write_byte(b);
        }
    }

    /// Non-blocking read from the console input device. Returns
    /// `Some(byte)` if a character is available, `None` otherwise.
    fn console_try_read(&self) -> Option<u8> {
        None
    }

    /// Non-blocking read of one whole **key word** (`scancode << 8 | ascii`)
    /// for the AES event manager, so it can deliver scancode-only keys
    /// (arrows, function keys). The default has no scancode source and
    /// widens the ASCII byte (scancode 0); the kernel host overrides this
    /// with the IKBD key ring.
    fn console_try_read_key(&self) -> Option<u16> {
        self.console_try_read().map(|b| b as u16)
    }

    /// Non-blocking, **non-destructive** peek of the next whole key word
    /// (`scancode << 8 | ascii`) at the front of the console input queue —
    /// the value the next [`Host::console_try_read_key`] would return —
    /// without consuming it. (Until TASK/CON01 the GEMDOS output-side break
    /// check peeked here; EmuTOS' `conbrk` uses `Bconstat`/`Bconin` and the
    /// BDOS typeahead instead, so the GEMDOS layer no longer calls this.)
    /// The default has no peekable source (`None`); the kernel host overrides
    /// it with the IKBD key ring.
    fn console_peek_key(&self) -> Option<u16> {
        None
    }

    /// Returns `true` if the console has at least one byte ready.
    fn console_has_input(&self) -> bool {
        false
    }

    /// **Blocking** read of one byte from the console input device —
    /// waits until input is available. Used by the blocking GEMDOS
    /// input functions (`Cconin`, `Cnecin`, `Crawcin`, `Cconrs`).
    ///
    /// The default busy-polls [`Host::console_try_read`]; the kernel
    /// host overrides it to enable IRQs and `wfi` while it waits (the
    /// PL011 RX interrupt fills the ringbuffer, which is otherwise
    /// masked inside the synchronous SVC handler).
    fn console_read_byte_blocking(&self) -> u8 {
        loop {
            if let Some(b) = self.console_try_read() {
                return b;
            }
            core::hint::spin_loop();
        }
    }

    /// **Blocking** read of one whole **key word** (`scancode << 8 | ascii`)
    /// from the console input device — waits until a key is available. Unlike
    /// [`Host::console_read_byte_blocking`] this does **not** skip
    /// scancode-only keys (ASCII 0, e.g. arrows / function keys): those must
    /// be delivered so `Bconin(2)`/`Cconin`/`Cnecin`/`Crawcin` can return the
    /// packed console LONG (scancode in bits 16-23, ASCII in bits 0-7 — see
    /// `docs/api/bios/02-bconin.md`).
    ///
    /// The default busy-polls [`Host::console_try_read_key`]; the kernel host
    /// overrides it to enable IRQs and spin/`wfi` while it waits (mirroring
    /// `console_read_byte_blocking`).
    fn console_read_key_blocking(&self) -> u16 {
        loop {
            if let Some(w) = self.console_try_read_key() {
                return w;
            }
            core::hint::spin_loop();
        }
    }

    /// [`Host::console_read_key_blocking`] that **gives way** to the running
    /// process's `kbdvec` hook (KEYS02, `docs/api/xbios/22-kbdvbase.md` B4):
    /// `None` when bytes for that hook are queued
    /// (`xbios::kbdvecs::pending_for_running`) — they can only be delivered on
    /// the way back to EL0, and a hook that chains to the default `kbdvec` is
    /// what puts the key this read waits for into the queue. The caller then
    /// asks for a restart (`xbios::kbdvecs::request_restart`) and returns
    /// without side effects; the trap handler rewinds the `svc`, runs the hook
    /// and re-executes the read.
    ///
    /// Used by the single-key reads (`Cconin`, `Cnecin`, `Crawcin`,
    /// `Bconin(2)`), which have no side effect before their key arrives. The
    /// default never gives way (a hostless test has no hook to wait for).
    fn console_read_key_restartable(&self) -> Option<u16> {
        Some(self.console_read_key_blocking())
    }

    /// [`Host::console_read_key_blocking`] returning the whole IKBD ring
    /// **entry** `shifty << 24 | scancode << 16 | ascii`, the shift state
    /// taken when the key was pushed (XBIOS01 E2, EmuTOS `bios/ikbd.c:900`) —
    /// what `Bconin(2)` returns before `conterm` bit 3 is applied
    /// (`bios/ikbd.c:279-280`). The default widens the key word with shift
    /// state 0; the kernel host reads the IOREC ring
    /// (`crate::xbios::iorec`).
    fn console_read_entry_blocking(&self) -> u32 {
        crate::xbios::iorec::entry_from_word(self.console_read_key_blocking(), 0)
    }

    /// [`Host::console_read_key_restartable`] returning the ring entry, as
    /// [`Host::console_read_entry_blocking`].
    fn console_read_entry_restartable(&self) -> Option<u32> {
        self.console_read_key_restartable()
            .map(|w| crate::xbios::iorec::entry_from_word(w, 0))
    }

    /// Run the default `kbdvec` — EmuTOS `kbd_int(scancode)`
    /// (`bios/ikbd.c:725-910`), entered through `_kbdvec`
    /// (`bios/aciavecs.S:372-376`) — for one IKBD key byte (bit 7 = break):
    /// modifier state, key word into the console queue, key repeat. Reached
    /// from `Ssystem(S_NEWTOS_KBDVEC)`, the call the per-process
    /// `kbdvec_default` stub makes (KEYS02, D6). Returns `false` when this host
    /// has no keyboard decoder (a hostless test), so the call can say so.
    fn kbd_int(&self, _scancode: u8) -> bool {
        false
    }

    /// Restart the machine — EmuTOS `warm_reset`/`cold_reset`
    /// (`bios/panicasm.S:102-140`), reached from `Ssystem(S_NEWTOS_RESET)`,
    /// the call the OSHEADER reset stub makes (TASK/RST01,
    /// [`crate::bios::reset`]). Does not return on a host that can restart;
    /// **returns** only where there is no restart mechanism (the default, a
    /// hostless test, a board without one), and the caller reports `ERROR`.
    fn system_restart(&self, _kind: crate::bios::reset::ResetKind) {}

    /// Advance the polled input sources once (keyboard/mouse bytes from
    /// the UART(s)), with interrupts briefly enabled. The AES event
    /// manager calls this while it polls for a wait condition: an
    /// `evnt_*` runs in the synchronous SVC handler with IRQs masked, so
    /// without this the RX sources never advance and the wait would
    /// deadlock. Default: no-op (a hostless test has no hardware to pump).
    fn pump_input(&self) {}

    /// Whole seconds elapsed since boot, from a monotonic hardware
    /// counter. The GEMDOS time functions add this to a settable base
    /// epoch to produce the wall clock. The kernel host reads the ARM
    /// generic timer (`CNTPCT_EL0 / CNTFRQ_EL0`); the default `0` makes
    /// the clock stand still (acceptable fallback for a hostless test).
    fn monotonic_seconds(&self) -> u64 {
        0
    }

    /// Milliseconds elapsed since boot, from the same monotonic hardware
    /// counter as [`Host::monotonic_seconds`] but at millisecond
    /// resolution (`CNTPCT * 1000 / CNTFRQ`). Backs the AES event timer
    /// (`evnt_timer` / `evnt_multi`'s `MU_TIMER`). The default `0` makes
    /// a timed wait fire immediately (acceptable for a hostless test).
    fn monotonic_millis(&self) -> u64 {
        0
    }

    /// A monotonic **frame counter** at ~60 Hz, derived from the hardware
    /// timer (`CNTPCT * 60 / CNTFRQ`). Backs XBIOS `Vsync` (spin until it
    /// ticks) and seeds `Random`. The default `0` makes `Vsync` a no-op
    /// (acceptable for a hostless test).
    fn frame_clock(&self) -> u64 {
        0
    }

    /// The VDI rasterizer backend (the screen device), or `None` if the
    /// board has no framebuffer. This is the swappable hardware boundary
    /// for the VDI: the bring-up host returns a software rasterizer over
    /// the framebuffer; a later accelerated (NEON / DMA / GPU) backend
    /// drops in here without the VDI layer changing. Default `None`
    /// keeps hostless / display-less configurations building.
    fn raster(&self) -> Option<&dyn crate::vdi::backend::RasterBackend> {
        None
    }

    /// The block device backing BIOS `Rwabs` (the SD card), or `None` if
    /// the board has no storage. Same swappable-boundary pattern as
    /// [`Host::raster`].
    fn block(&self) -> Option<&dyn crate::bios::block::BlockDev> {
        None
    }

    /// The current hardware framebuffer (base address + geometry), or
    /// `None` if no display is up. Backs the XBIOS screen group
    /// (`Physbase` / `Logbase` / `Getrez`). Same swappable boundary as
    /// [`Host::raster`].
    fn screen(&self) -> Option<ScreenInfo> {
        None
    }

    /// Move the **logical** screen — where drawing goes, EmuTOS `v_bas_ad`
    /// (`bios/screen.c:1106-1107`) — to the screen page whose CPU-visible
    /// base is `base`. Returns `false` when `base` is not the base of a page
    /// of this display's screen memory, in which case nothing moved.
    ///
    /// The default `false` is the honest answer of a host with no display:
    /// there is no screen to move. It is *not* a "silently accepted" no-op —
    /// the caller learns the move did not happen and
    /// [`Host::screen`]`().base` keeps reporting where drawing really goes.
    fn set_screen_logical(&self, _base: u64) -> bool {
        false
    }

    /// Move the **physical** screen — what the display engine scans out,
    /// EmuTOS `setphys()` (`bios/screen.c:1110-1111`) — to the screen page
    /// whose CPU-visible base is `base`. Returns `false` when the hardware
    /// cannot show that address; the scan-out is then left exactly where it
    /// was and [`Host::screen`]`().phys_base` keeps reporting the truth, the
    /// same way EmuTOS' `physbase()` reads the video-base register back
    /// rather than a shadow.
    fn set_screen_physical(&self, _base: u64) -> bool {
        false
    }

    /// The current keyboard shift byte, EmuTOS `shifty` (`MODE_*` bits:
    /// `0x01` right-shift, `0x02` left-shift, `0x04` ctrl, `0x08` alt, `0x10`
    /// caps, `0x20` Clr/Home and `0x40` Insert pressed with Alt, bit 7 as a
    /// program stored it), as BIOS `Kbshift` reports it; VDI `vq_key_s` masks
    /// it to the low four bits. The kernel host reads the IKBD decoder's byte
    /// ([`crate::bios::shifty::Shifty`]); the default `0` (no modifiers) suits
    /// a hostless / keyboard-less configuration.
    fn keyboard_shift_state(&self) -> u8 {
        0
    }

    /// Overwrite the keyboard shift byte, all eight bits (same `MODE_*` bits
    /// as [`Host::keyboard_shift_state`]) — the set half of BIOS `Kbshift`
    /// (`flag >= 0`, EmuTOS `ikbd.c:150` stores `flag` into the `shifty`
    /// byte). The kernel host writes the IKBD decoder's byte so a
    /// subsequent [`Host::keyboard_shift_state`] reads the value back;
    /// the default is a no-op (a hostless / keyboard-less configuration
    /// has no modifier state to set). Not a stub — a `Kbshift` set with no
    /// backing keyboard simply has nothing to store, exactly as EmuTOS'
    /// `shifty` write is inert with no ACIA present.
    fn set_keyboard_shift_state(&self, _mask: u8) {}

    /// The three primary scancode→ASCII translation tables plus the AltGr
    /// override arrays of the **active** keyboard layout, in the shape XBIOS
    /// `Keytbl` publishes them (EmuTOS `struct keytbl`, `bios/ikbd.h:57-86`).
    /// `None` when the board has no keyboard translator, in which case
    /// `Keytbl` has nothing to point at. Same swappable boundary as
    /// [`Host::raster`].
    fn key_tables(&self) -> Option<KeyTables> {
        None
    }

    /// Install a caller-supplied 128-byte scancode→ASCII table as one of the
    /// three primary tables — the set half of `Keytbl` (EmuTOS
    /// `bios/ikbd.c:124-132` assigns the pointer; NewTOS's translator owns a
    /// RAM layout, so the bytes are taken over instead of the pointer). The
    /// default is a no-op: a board with no keyboard translator has no table
    /// to replace.
    fn set_key_table(&self, _which: KeyTable, _table: &[u8; 128]) {}

    /// Put the power-up keyboard layout back into force — the translator half
    /// of XBIOS `Bioskeys` (EmuTOS `current_keytbl = *get_keytbl();`,
    /// `bios/ikbd.c:136-140`): all three primary tables and the AltGr arrays
    /// return to the layout the machine booted with, undoing every
    /// [`Host::set_key_table`]. The default is a no-op: a board with no
    /// keyboard translator has no layout to restore.
    fn reset_key_tables(&self) {}

    /// The CPU core clock the machine is running at, in **Hz**, or `None`
    /// when this board cannot be asked. Backs `Ssystem(S_NEWTOS_CPUCLOCK)`
    /// (`docs/api/gemdos/154-ssystem.md`). Same swappable hardware boundary
    /// as [`Host::raster`]: on `board-rpi4` the kernel host asks the
    /// VideoCore firmware over the mailbox property channel; a board without
    /// that channel (`board-qemu-virt`, a hostless test) keeps the default
    /// and answers honestly with `None`, which the caller turns into
    /// `ERROR`. It never invents a number — a clock frequency that was not
    /// measured is not a clock frequency.
    fn cpu_clock_hz(&self) -> Option<u32> {
        None
    }

    /// The SoC's current die temperature in **thousandths of a degree
    /// Celsius** (47.2 °C → `47200`), or `None` when this board has no
    /// temperature sensor to ask. Backs `Ssystem(S_NEWTOS_CPUTEMP)`
    /// (`docs/api/gemdos/154-ssystem.md`); same boundary and the same
    /// honest-`None` rule as [`Host::cpu_clock_hz`].
    ///
    /// Signed because a temperature is: the firmware reports the value as an
    /// unsigned word, but a board started below freezing is a physical state,
    /// not an error, and the TOS-side LONG has to be able to carry it.
    fn cpu_temperature_mdeg(&self) -> Option<i32> {
        None
    }

    // ── SYSINFO01: the effective system settings (`Ssystem` 2003–2020) ──
    //
    // The facts below live in the kernel (drivers, boot decisions) and not in
    // `tos-api`; each is the state the kernel actually ended up in, never the
    // text of a boot argument. A host that does not know one keeps the
    // default `None`, which `Ssystem` turns into `ERROR`
    // (`docs/api/gemdos/154-ssystem.md` §"System-information modes").

    /// The ARM core clock as the firmware **measured** it, in Hz — mailbox
    /// "Get clock rate measured" (`0x0003_0047`), where
    /// [`Host::cpu_clock_hz`] answers the last *requested* rate. Live per
    /// call; `None` without a VideoCore or without an answer.
    fn cpu_clock_measured_hz(&self) -> Option<u32> {
        None
    }

    /// What the boot code did with the ARM clock (`cpuclk=`, FS08 part 3), or
    /// `None` on a board without clock control.
    fn cpu_clock_policy(&self) -> Option<CpuClockPolicy> {
        None
    }

    /// The SD card's transfer mode and negotiated bus width, or `None` when
    /// no card was initialised (or the board has no SD driver).
    fn sd_card_mode(&self) -> Option<SdCardMode> {
        None
    }

    /// The memory the firmware gives the VideoCore (the `gpu_mem` split), in
    /// bytes, or `None` when the firmware did not report it.
    fn gpu_memory_bytes(&self) -> Option<u64> {
        None
    }

    /// Where the active keyboard layout came from, or `None` without a
    /// keyboard translator.
    fn keyboard_layout(&self) -> Option<KeyboardLayout> {
        None
    }

    /// `Some(true)` while console output goes to the serial port,
    /// `Some(false)` while it is switched off, `None` without a serial
    /// console.
    fn serial_console(&self) -> Option<bool> {
        None
    }

    /// Where the keyboard/mouse pump runs (INPUT01, boot argument
    /// `input=aes`) — `Ssystem(S_NEWTOS_INPUTMODE)`, KEYOWNER. `None`
    /// without a kernel input pump (hostless).
    fn input_mode(&self) -> Option<InputMode> {
        None
    }

    /// The OS name and version as the boot screen prints it
    /// (`"NewTOS 0.6.1"`), or `None` (hostless). `Ssystem(S_NEWTOS_KVERSION)`
    /// appends the build stamp ([`crate::build::BUILD_STAMP`]).
    fn os_version(&self) -> Option<&'static str> {
        None
    }

    /// Answer one USB snapshot request — `Ssystem(S_NEWTOS_USBSNAP)` (2022,
    /// `docs/api/usb/usb-snapshot.md` §2). The trap has checked the header
    /// (§2.3 #1–#4); the host decides #5–#9 and hands the answer **body**
    /// (everything after the 32-byte header) to `sink` in order, in chunks —
    /// the trap copies each into the caller's buffer, so no large
    /// intermediate buffer exists. `sink` returns `false` when the copy
    /// faulted; the host then stops and answers [`UsbSnapAnswer::Faulted`].
    ///
    /// Default: [`UsbSnapAnswer::NoController`] — a host without a USB stack
    /// (a hostless test, a board whose controller was not found) answers the
    /// same as a board without USB, which the trap turns into `ERROR`.
    fn usb_snapshot(
        &self,
        _req: &UsbSnapRequest,
        _sink: &mut dyn FnMut(&[u8]) -> bool,
    ) -> UsbSnapAnswer {
        UsbSnapAnswer::NoController
    }
}

/// One `Ssystem(S_NEWTOS_USBSNAP)` request as the trap passes it to
/// [`Host::usb_snapshot`] (`docs/api/usb/usb-snapshot.md` §2.2).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UsbSnapRequest {
    /// `op`: 1 LIST, 2 DEVICE, 3 REPORTS, 4 MUTE (already range-checked).
    pub op: u16,
    /// `index`: device index (ops 2, 3); 0 otherwise.
    pub index: u32,
    /// `arg`: op 3 first wanted sequence number, op 4 ticks; 0 otherwise.
    pub arg: u32,
    /// `_hz_200` at the call (the header's `now_hz200`).
    pub now_hz200: u32,
    /// Bytes the caller's buffer holds behind the 32-byte header.
    pub room: u32,
}

/// How [`Host::usb_snapshot`] answered (`docs/api/usb/usb-snapshot.md` §2.3).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UsbSnapAnswer {
    /// No usable USB controller (§2.3 #5): `ERROR`, nothing written.
    NoController,
    /// `index` out of range or a MUTE argument refused (§2.3 #6/#7):
    /// `EINVAL`, the header is written.
    Invalid,
    /// Answered (§2.3 #8/#9): `E_OK`.
    Done {
        /// Size of the complete body (`total_len` − 32).
        total: u32,
        /// Body bytes handed to the sink (`used_len` − 32).
        written: u32,
        /// Only the header was written: the answer did not fit.
        truncated: bool,
        /// Op 3 returned fewer entries than are available.
        more: bool,
    },
    /// The sink reported a fault in the caller's buffer.
    Faulted,
}

/// What the boot code did with the ARM core clock (`cpuclk=`, FS08 part 3) —
/// `Ssystem(S_NEWTOS_CPUPOLICY)`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CpuClockPolicy {
    /// The firmware controls the clock (the default, or `cpuclk=firmware`).
    Firmware = 0,
    /// Pinned at the firmware's maximum rate (`cpuclk=max`, granted).
    FixedMax = 1,
    /// `cpuclk=max` was asked, but the firmware reported no maximum or did
    /// not answer "Set clock rate": the clock stayed with the firmware.
    MaxRefused = 2,
}

/// The SD card's effective transfer setup — `Ssystem(S_NEWTOS_SDMODE)` and
/// `Ssystem(S_NEWTOS_SDBUS)`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SdCardMode {
    /// `emmc=compat`: 1-bit bus, one single-block command per sector.
    pub compat: bool,
    /// Data-bus width the driver and the card use, in bits (1 or 4).
    pub bus_width: u8,
}

/// Where the keyboard/mouse pump runs — `Ssystem(S_NEWTOS_INPUTMODE)`
/// (`docs/api/gemdos/154-ssystem.md`, 2020).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputMode {
    /// Also from the 200 Hz timer interrupt while a program runs: the pointer
    /// moves while a program is busy (INPUT01, the default).
    TimerPump = 0,
    /// Only in the AES waits and the console reads (`input=aes`).
    Aes = 1,
}

/// Where the active keyboard layout came from — `Ssystem(S_NEWTOS_KEYBOARD)`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyboardLayout {
    /// The built-in US table (the default, or `#K us`).
    BuiltinUs = 0,
    /// The built-in German table (`#K de` in `NEWDESK.INF`).
    BuiltinGerman = 1,
    /// Loaded from `C:\KEYBOARD.KBD`.
    File = 2,
    /// A program has since replaced a table (XBIOS `Keytbl`) with different
    /// contents.
    ChangedByProgram = 3,
}

/// Bytes reserved for one AltGr override array. EmuTOS's non-`DUAL_KEYBOARD`
/// format is `(scancode, ascii)` pairs terminated by a zero byte
/// (`bios/ikbd.h:66-71`); NewTOS's translator holds up to 16 pairs per shift
/// level, so 33 bytes always suffice.
pub const ALT_TABLE_LEN: usize = 33;

/// Which of the three primary tables [`Host::set_key_table`] replaces.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyTable {
    /// Unshifted (`norm`).
    Norm,
    /// Shifted (`shft`).
    Shft,
    /// Caps-locked (`caps`).
    Caps,
}

/// A snapshot of the active keyboard layout in `struct keytbl` shape
/// (`bios/ikbd.h:57-86`). The three primary tables are 128-byte
/// scancode→ASCII arrays; the three AltGr arrays are zero-terminated
/// `(scancode, ascii)` pair lists padded to [`ALT_TABLE_LEN`].
#[derive(Clone, Copy)]
pub struct KeyTables {
    /// Unshifted table.
    pub norm: [u8; 128],
    /// Shifted table.
    pub shft: [u8; 128],
    /// Caps-locked table.
    pub caps: [u8; 128],
    /// AltGr overrides for the unshifted level.
    pub altnorm: [u8; ALT_TABLE_LEN],
    /// AltGr overrides for the shifted level.
    pub altshft: [u8; ALT_TABLE_LEN],
    /// AltGr overrides for the caps-locked level.
    pub altcaps: [u8; ALT_TABLE_LEN],
}

/// The hardware framebuffer as the XBIOS screen calls see it.
///
/// Three addresses, because a display whose screen memory holds more than one
/// screen has three (EmuTOS `bios/screen.c`):
///
/// * [`Self::mem_base`] / [`Self::mem_bytes`] — the whole allocated screen
///   memory, `mem_bytes / size_bytes` screen pages of it. This is the extent
///   a program that draws into the screen must be able to reach, and the
///   extent `Setscreen` may point the two bases into.
/// * [`Self::base`] — the **logical** screen: where drawing goes
///   (`v_bas_ad`).
/// * [`Self::phys_base`] — the **physical** screen: what the display engine
///   scans out (`physbase()`, read back from the hardware).
///
/// On a single-page display all three coincide.
#[derive(Clone, Copy)]
pub struct ScreenInfo {
    /// CPU-visible base address of the **logical** screen — the drawing
    /// target, EmuTOS `v_bas_ad`.
    pub base: u64,
    /// CPU-visible base address of the **physical** screen — the page the
    /// display engine is currently scanning out.
    pub phys_base: u64,
    /// CPU-visible base of the whole screen memory (page 0).
    pub mem_base: u64,
    /// Byte size of the whole screen memory, `size_bytes` × the number of
    /// screen pages it holds.
    pub mem_bytes: u32,
    /// Visible width in pixels.
    pub width: u16,
    /// Visible height in pixels.
    pub height: u16,
    /// Bits per pixel (32 for our XRGB8888 framebuffer).
    pub bpp: u16,
    /// Bytes per scan line (the hardware pitch). May exceed
    /// `width * bpp / 8` when the display controller pads lines.
    pub stride_bytes: u32,
    /// Byte size of **one** screen (`stride_bytes * height`) — the distance
    /// between two screen pages inside [`Self::mem_bytes`].
    pub size_bytes: u32,
}

// `&'static dyn Host` is `Sync + 'static`; we store it as a raw
// pointer because there is no `AtomicRef`. Access is read-mostly:
// installed exactly once at boot, never replaced.
static HOST: AtomicPtr<&'static dyn Host> = AtomicPtr::new(core::ptr::null_mut());

/// Install the kernel-supplied host. Must be called exactly once
/// before any trap is taken.
///
/// # Panics
///
/// Panics in debug builds if called more than once.
pub fn install_host(host: &'static &'static dyn Host) {
    let ptr = host as *const _ as *mut _;
    let prev = HOST.swap(ptr, Ordering::Release);
    debug_assert!(prev.is_null(), "tos_api::host already installed");
}

/// Access the installed host. Returns `None` before [`install_host`]
/// has run; the dispatcher treats that as a fatal misconfiguration.
#[inline]
pub fn host() -> Option<&'static dyn Host> {
    let ptr = HOST.load(Ordering::Acquire);
    if ptr.is_null() {
        None
    } else {
        // SAFETY: `install_host` stored a valid `&'static &'static dyn Host`.
        Some(unsafe { *ptr })
    }
}
