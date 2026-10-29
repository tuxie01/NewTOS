//! Kernel-side implementation of [`tos_api::Host`].
//!
//! The host wires TOS-API console operations to the board's PL011
//! console driver. Phase 02 only needs output; the read paths are
//! stubbed out (returning `None` / `false`) until the IRQ-driven
//! ringbuffer in PHASE-02 D5 lands.

#[cfg(feature = "board-rpi4")]
use core::sync::atomic::Ordering;

use tos_api::Host;

use crate::console::CONSOLE;
use crate::hal::uart::Uart;

/// Singleton host installed into `tos-api` during boot.
pub struct KernelHost;

impl KernelHost {
    /// Give the **polled** console input source a turn before the key queue is
    /// observed, so a non-blocking TOS status/read call reports input that
    /// arrived while its caller was looping.
    ///
    /// EmuTOS fills the IKBD queue from the keyboard *interrupt*
    /// (`bios/ikbd.c:157-173`, `ikbdiorec` fed by the ACIA ISR), so
    /// `bconstat2` — and with it `Bconstat(2)`, `Cconis` and the non-blocking
    /// `Crawio(0xFF)` — is a true status read: the classic TOS idiom
    /// `while (!Bconstat(2)) ;` terminates because the queue keeps filling
    /// underneath the loop.
    ///
    /// NewTOS' console input on `board-rpi4` is the USB-HID keyboard, which
    /// has no interrupt path: a report only reaches the queue when somebody
    /// runs [`crate::drivers::usb::poll`], and its only callers are
    /// [`Host::pump_input`] (AES event manager, cooperative idle pass), the
    /// two blocking reads and — since INPUT01, unless `input=aes` — the timer
    /// interrupt while a program runs at EL0 ([`irq_input_pump`]). The trap
    /// that runs this check is none of those (IRQs are masked inside it, and
    /// a program spinning on `Bconstat(2)` spends most of its time in the
    /// trap). Without pumping here, a program that polls
    /// `Bconstat(2)` and calls nothing else spins forever — nothing can make
    /// the queue non-empty. TeraDesk's "wait after a TOS program" is exactly
    /// that loop (`sdk/apps/teradesk/startprg.c:320-327`:
    /// `while (Bconstat(2)) Bconin(2); while (!Bconstat(2) && …) ;`).
    /// Pumping here is the polled stand-in for EmuTOS' interrupt.
    ///
    /// Only pumps while the queue is empty, so a key that is already waiting
    /// is reported without touching the hardware and queue order is kept.
    ///
    /// Since TASK/CON01 the GEMDOS output-side `conbrk` is EmuTOS' — it asks
    /// `Bconstat` before every byte `Cconout`/`Cconws`/`Fwrite` to CON writes
    /// — so this pump also runs once per output byte while the queue is
    /// empty: Ctrl-C / Ctrl-S typed during a long print reach `board-rpi4`, as
    /// the keyboard interrupt delivers them on EmuTOS. (`console_peek_key`
    /// is no longer used by the GEMDOS layer and stays unpumped.)
    #[inline]
    fn fill_from_device(&self) {
        if crate::console::rx_has_input() {
            return;
        }
        self.pump_input();
    }
}

impl Host for KernelHost {
    fn console_write_byte(&self, byte: u8) {
        CONSOLE.write_byte(byte);
        crate::fbcon::write_byte(byte);
    }

    fn console_write_bytes(&self, s: &[u8]) {
        for &b in s {
            CONSOLE.write_byte(b);
            crate::fbcon::write_byte(b);
        }
    }

    fn console_try_read(&self) -> Option<u8> {
        self.fill_from_device();
        crate::console::rx_pop()
    }

    fn console_try_read_key(&self) -> Option<u16> {
        self.fill_from_device();
        crate::console::rx_pop_key()
    }

    fn console_peek_key(&self) -> Option<u16> {
        crate::console::rx_peek_key()
    }

    fn pump_input(&self) {
        // Briefly unmask IRQs so pending interrupts (timer tick, the
        // qemu-virt PL011 receive interrupt that fills `Iorec(0)`) are taken
        // — they are masked inside the synchronous SVC handler. Mirrors the
        // loop body of `console_read_byte_blocking`, but for a single pass so
        // the AES event loop stays responsive to its other (timer)
        // conditions. The console is the USB-HID keyboard on both boards,
        // drained below via `usb::poll()`.
        let daif: u64;
        // SAFETY: reading DAIF and toggling the IRQ mask are permitted at
        // EL1; we restore the caller's DAIF before returning.
        unsafe {
            core::arch::asm!(
                "mrs {0}, daif",
                out(reg) daif,
                options(nomem, nostack, preserves_flags),
            );
            core::arch::asm!(
                "msr daifclr, #2",
                options(nomem, nostack, preserves_flags),
            );
        }

        // Drain USB-HID interrupt transfers (keyboard/mouse) → ikbd::feed,
        // then the key repeat ([`pump_devices`]). The sole console input
        // source on both boards (qemu-virt: `qemu-xhci` + `usb-kbd`; serial
        // bytes go to AUX there, XBIOS15); no-op until a HID device is
        // enumerated.
        pump_devices();
        core::hint::spin_loop();

        // SAFETY: restore the caller's original DAIF mask state.
        unsafe {
            core::arch::asm!(
                "msr daif, {0}",
                in(reg) daif,
                options(nomem, nostack, preserves_flags),
            );
        }
    }

    fn console_has_input(&self) -> bool {
        self.fill_from_device();
        crate::console::rx_has_input()
    }

    fn console_read_byte_blocking(&self) -> u8 {
        // This runs inside the synchronous SVC handler, where the CPU
        // took the exception with IRQs masked — so no interrupt (the
        // timer tick that wakes the `wfi` below) could fire and a plain
        // wait would deadlock. Save DAIF, unmask IRQ (`daifclr #2` clears
        // the `I` bit), poll the keyboard until a byte lands in the
        // ringbuffer, then restore the original mask.
        let daif: u64;
        // SAFETY: reading DAIF and clearing the IRQ mask are permitted
        // at EL1; we restore DAIF before returning.
        unsafe {
            core::arch::asm!(
                "mrs {0}, daif",
                out(reg) daif,
                options(nomem, nostack, preserves_flags),
            );
            core::arch::asm!(
                "msr daifclr, #2",
                options(nomem, nostack, preserves_flags),
            );
        }

        let byte = loop {
            // Drain USB-HID interrupt transfers, so a *blocking* console
            // read (Cconin/Bconin — e.g. the EmuCON shell) sees the USB
            // keyboard.
            // `pump_input` already does this for the desktop / AES event
            // loops; without it here the USB keyboard reaches the desktop
            // but never a blocking console read. No-op until a HID device
            // is enumerated (on both boards the only console input source).
            // Key auto-repeat after the input ([`pump_devices`]).
            pump_devices();

            if let Some(b) = crate::console::rx_pop() {
                break b;
            }
            // On board-rpi4 the USB-HID keyboard is polled, so spin lightly
            // rather than sleeping (only the UART0 RX IRQ would wake a
            // `wfi`, and CON no longer reads the UART). On other boards
            // (qemu-virt) `wfi` sleeps until the next interrupt — the 200 Hz
            // tick — and the keyboard is polled again.
            #[cfg(feature = "board-rpi4")]
            core::hint::spin_loop();
            #[cfg(not(feature = "board-rpi4"))]
            // SAFETY: wait for the next interrupt (the timer tick wakes us).
            unsafe {
                core::arch::asm!("wfi", options(nomem, nostack, preserves_flags));
            }
        };

        // SAFETY: restore the caller's original DAIF mask state.
        unsafe {
            core::arch::asm!(
                "msr daif, {0}",
                in(reg) daif,
                options(nomem, nostack, preserves_flags),
            );
        }
        byte
    }

    fn console_read_key_blocking(&self) -> u16 {
        tos_api::xbios::iorec::word_from_entry(self.console_read_entry_blocking())
    }

    fn console_read_key_restartable(&self) -> Option<u16> {
        self.console_read_entry_restartable().map(tos_api::xbios::iorec::word_from_entry)
    }

    fn console_read_entry_blocking(&self) -> u32 {
        // Never gives way: `read_key_loop(false)` only returns with a key.
        read_key_loop(false).unwrap_or(0)
    }

    fn console_read_entry_restartable(&self) -> Option<u32> {
        read_key_loop(true)
    }

    fn kbd_int(&self, scancode: u8) -> bool {
        crate::ikbd::kbd_int(scancode);
        true
    }

    fn monotonic_seconds(&self) -> u64 {
        // ARM generic timer: physical count / frequency (Hz) = seconds
        // since the counter started (boot). Both are EL0-readable
        // system registers (ARM ARM D17.6 "Generic Timer").
        let cnt: u64;
        let frq: u64;
        // SAFETY: read-only system-register reads, always permitted.
        unsafe {
            core::arch::asm!(
                "mrs {cnt}, cntpct_el0",
                "mrs {frq}, cntfrq_el0",
                cnt = out(reg) cnt,
                frq = out(reg) frq,
                options(nomem, nostack, preserves_flags),
            );
        }
        cnt.checked_div(frq).unwrap_or(0)
    }

    fn monotonic_millis(&self) -> u64 {
        // Same generic-timer source as monotonic_seconds, scaled to ms.
        // Multiply before dividing to keep millisecond precision; use a
        // u128 intermediate so the *1000 cannot overflow on a fast or
        // long-running counter.
        let cnt: u64;
        let frq: u64;
        // SAFETY: read-only system-register reads, always permitted.
        unsafe {
            core::arch::asm!(
                "mrs {cnt}, cntpct_el0",
                "mrs {frq}, cntfrq_el0",
                cnt = out(reg) cnt,
                frq = out(reg) frq,
                options(nomem, nostack, preserves_flags),
            );
        }
        if frq == 0 {
            return 0;
        }
        ((cnt as u128 * 1000) / frq as u128) as u64
    }

    fn frame_clock(&self) -> u64 {
        // CNTPCT * 60 / CNTFRQ = the current ~60 Hz frame number.
        let cnt: u64;
        let frq: u64;
        // SAFETY: read-only system-register reads, always permitted.
        unsafe {
            core::arch::asm!(
                "mrs {cnt}, cntpct_el0",
                "mrs {frq}, cntfrq_el0",
                cnt = out(reg) cnt,
                frq = out(reg) frq,
                options(nomem, nostack, preserves_flags),
            );
        }
        cnt.checked_mul(60).unwrap_or(cnt).checked_div(frq).unwrap_or(0)
    }

    fn raster(&self) -> Option<&dyn tos_api::vdi::backend::RasterBackend> {
        // Hand the VDI the software framebuffer backend, but only once a
        // framebuffer is actually up (geometry published).
        if crate::vdi_backend::available() {
            Some(&crate::vdi_backend::VDI_SCREEN)
        } else {
            None
        }
    }

    fn screen(&self) -> Option<tos_api::host::ScreenInfo> {
        if !crate::vdi_backend::available() {
            return None;
        }
        let (width, height) = crate::vdi_backend::dims();
        Some(tos_api::host::ScreenInfo {
            base: crate::vdi_backend::base_addr(),
            phys_base: crate::vdi_backend::scanout_addr(),
            mem_base: crate::vdi_backend::mem_base_addr(),
            mem_bytes: crate::vdi_backend::mem_bytes(),
            width,
            height,
            bpp: 32,
            stride_bytes: crate::vdi_backend::stride_bytes(),
            size_bytes: crate::vdi_backend::size_bytes(),
        })
    }

    fn set_screen_logical(&self, base: u64) -> bool {
        crate::vdi_backend::set_draw_base(base)
    }

    fn set_screen_physical(&self, base: u64) -> bool {
        crate::vdi_backend::set_scanout_base(base)
    }

    /// RST01: the OSHEADER reset entry's restart ([`crate::reset::restart`]).
    /// Returns only on a board without a restart mechanism.
    fn system_restart(&self, kind: tos_api::bios::reset::ResetKind) {
        crate::reset::restart(kind, "OSHEADER reset entry");
    }

    fn keyboard_shift_state(&self) -> u8 {
        crate::ikbd::shift_state()
    }

    fn set_keyboard_shift_state(&self, mask: u8) {
        crate::ikbd::set_shift_state(mask);
    }

    fn key_tables(&self) -> Option<tos_api::host::KeyTables> {
        Some(crate::keymap::snapshot())
    }

    fn set_key_table(&self, which: tos_api::host::KeyTable, table: &[u8; 128]) {
        crate::keymap::set_table(which, table);
    }

    #[cfg(feature = "board-rpi4")]
    fn block(&self) -> Option<&dyn tos_api::bios::block::BlockDev> {
        use crate::hal::block::BlockDevice;
        if crate::drivers::bcm2711::emmc::SDCARD.block_count() > 0 {
            Some(&KERNEL_BLOCK)
        } else {
            None
        }
    }

    /// `board-rpi4`: ask the VideoCore firmware for the ARM core clock over
    /// the mailbox property channel. The query runs inside the GEMDOS trap
    /// (`Ssystem(S_NEWTOS_CPUCLOCK)`), which is exactly where it belongs —
    /// the value is live, not a boot-time snapshot: the firmware scales the
    /// ARM clock with load and temperature, so a cached number would be a
    /// different machine's answer. `board-qemu-virt` has no VideoCore, keeps
    /// the trait default (`None`), and the trap reports `ERROR`.
    #[cfg(feature = "board-rpi4")]
    fn cpu_clock_hz(&self) -> Option<u32> {
        use crate::drivers::bcm2711::mailbox;
        mailbox::get_clock_rate(mailbox::CLOCK_ID_ARM)
    }

    /// `board-rpi4`: the SoC die temperature from the same property channel,
    /// in thousandths of a degree Celsius. Live for the same reason as
    /// [`Host::cpu_clock_hz`] — a temperature that is not read now is not a
    /// temperature. The firmware reports it as an unsigned word; widened to
    /// the signed TOS LONG unchanged.
    #[cfg(feature = "board-rpi4")]
    fn cpu_temperature_mdeg(&self) -> Option<i32> {
        use crate::drivers::bcm2711::mailbox;
        mailbox::get_temperature(mailbox::TEMPERATURE_ID).map(|mdeg| mdeg as i32)
    }

    /// `board-rpi4`: "Get clock rate measured" for the ARM clock, live per
    /// call for the same reason as [`Host::cpu_clock_hz`].
    #[cfg(feature = "board-rpi4")]
    fn cpu_clock_measured_hz(&self) -> Option<u32> {
        use crate::drivers::bcm2711::mailbox;
        mailbox::get_clock_rate_measured(mailbox::CLOCK_ID_ARM)
    }

    /// `board-rpi4`: what `raise_arm_clock` did at boot ([`record_cpu_clock_policy`]).
    #[cfg(feature = "board-rpi4")]
    fn cpu_clock_policy(&self) -> Option<tos_api::host::CpuClockPolicy> {
        use tos_api::host::CpuClockPolicy;
        match CPU_CLOCK_POLICY.load(Ordering::Relaxed) {
            0 => Some(CpuClockPolicy::Firmware),
            1 => Some(CpuClockPolicy::FixedMax),
            2 => Some(CpuClockPolicy::MaxRefused),
            _ => None,
        }
    }

    /// `board-rpi4`: the SD driver's transfer setup, `None` without a card.
    #[cfg(feature = "board-rpi4")]
    fn sd_card_mode(&self) -> Option<tos_api::host::SdCardMode> {
        crate::drivers::bcm2711::emmc::transfer_mode()
            .map(|(compat, bus_width)| tos_api::host::SdCardMode { compat, bus_width })
    }

    /// `board-rpi4`: the VideoCore memory the firmware reported at boot
    /// ([`record_gpu_memory`]); the `gpu_mem` split is fixed while the
    /// machine runs.
    #[cfg(feature = "board-rpi4")]
    fn gpu_memory_bytes(&self) -> Option<u64> {
        match GPU_MEMORY_BYTES.load(Ordering::Relaxed) {
            u64::MAX => None,
            b => Some(b),
        }
    }

    fn keyboard_layout(&self) -> Option<tos_api::host::KeyboardLayout> {
        Some(crate::keymap::source())
    }

    fn serial_console(&self) -> Option<bool> {
        Some(crate::console::serial_tx_enabled())
    }

    fn input_mode(&self) -> Option<tos_api::host::InputMode> {
        Some(match crate::input_gate::mode() {
            crate::input_gate::InputMode::Irq => tos_api::host::InputMode::TimerPump,
            crate::input_gate::InputMode::Aes => tos_api::host::InputMode::Aes,
        })
    }

    fn os_version(&self) -> Option<&'static str> {
        Some(crate::initinfo::VERSION_STRING)
    }

    /// `Ssystem(S_NEWTOS_USBSNAP)`: the USB snapshot the boot enumeration and
    /// the input pump keep (`drivers::usb::snapshot`,
    /// `docs/api/usb/usb-snapshot.md`). A board whose controller was not found
    /// (raspi4b in QEMU, qemu-virt without `qemu-xhci`) answers
    /// `NoController`.
    fn usb_snapshot(
        &self,
        req: &tos_api::host::UsbSnapRequest,
        sink: &mut dyn FnMut(&[u8]) -> bool,
    ) -> tos_api::host::UsbSnapAnswer {
        use crate::drivers::usb::snapshot::{Answer, Request};
        use tos_api::host::UsbSnapAnswer;
        let r = Request {
            op: req.op,
            index: req.index,
            arg: req.arg,
            now: req.now_hz200,
            room: req.room,
        };
        match crate::drivers::usb::snapshot_answer(r, sink) {
            Answer::NoController => UsbSnapAnswer::NoController,
            Answer::Invalid => UsbSnapAnswer::Invalid,
            Answer::Faulted => UsbSnapAnswer::Faulted,
            Answer::Done { total, written, truncated, more } => {
                UsbSnapAnswer::Done { total, written, truncated, more }
            }
        }
    }
}

/// `raise_arm_clock`'s outcome as `CpuClockPolicy as u8`; `u8::MAX` = not
/// decided (the boot step has not run).
#[cfg(feature = "board-rpi4")]
static CPU_CLOCK_POLICY: core::sync::atomic::AtomicU8 = core::sync::atomic::AtomicU8::new(u8::MAX);

/// Record what the boot step did with the ARM clock (`cpuclk=`, FS08 part 3)
/// for `Ssystem(S_NEWTOS_CPUPOLICY)`.
#[cfg(feature = "board-rpi4")]
pub fn record_cpu_clock_policy(p: tos_api::host::CpuClockPolicy) {
    CPU_CLOCK_POLICY.store(p as u8, Ordering::Relaxed);
}

/// The VideoCore memory from the firmware's "Get VC memory" answer at boot;
/// `u64::MAX` = not reported.
#[cfg(feature = "board-rpi4")]
static GPU_MEMORY_BYTES: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(u64::MAX);

/// Record the VideoCore memory size the firmware reported at boot, for
/// `Ssystem(S_NEWTOS_GPUMEM)`.
#[cfg(feature = "board-rpi4")]
pub fn record_gpu_memory(bytes: u64) {
    GPU_MEMORY_BYTES.store(bytes, Ordering::Relaxed);
}

/// The one-at-a-time guard of [`pump_devices`] (`crate::input_gate`).
static PUMP: crate::input_gate::PumpLock = crate::input_gate::PumpLock::new();

/// One pass of the trap-side input pump — the body behind
/// [`Host::pump_input`] and both blocking console reads: decode one report
/// (keyboard/mouse → [`crate::ikbd::feed`] → console ring, VDI cursor, AES
/// button/motion handlers), then run the key repeat — after the input, so a
/// break that arrived has already stopped it (`docs/api/xbios/_state.md`
/// §"Key-repeat handler").
///
/// The report is the one the timer-interrupt pump held back (INPUT01: a
/// button change, a wheel detent or a key — [`crate::ikbd::release_held`]),
/// or else the next one drained from the USB controller. One report per pass
/// either way, as before INPUT01.
///
/// Skipped if a pump is already running ([`crate::input_gate::PumpLock`]) —
/// which by construction never happens: the interrupt pumps only from EL0,
/// every other site runs in kernel code at EL1.
fn pump_devices() {
    let Some(_pump) = PUMP.try_enter() else {
        return;
    };
    if !crate::ikbd::release_held() {
        crate::drivers::usb::poll();
    }
    crate::ikbd::poll_repeat();
}

/// One pass of the timer-interrupt input pump ([`irq_input_pump`]): drain
/// one USB report into the decoder's capture and decode it only if it is pure
/// pointer motion; anything else is held for the next [`pump_devices`]
/// (`crate::input_gate`, "Only the pointer moves from the interrupt"). While a
/// report is held nothing is drained. No key repeat here — it stays with the
/// trap-side pump, next to the key bytes it must follow.
fn pump_devices_irq() {
    let Some(_pump) = PUMP.try_enter() else {
        return;
    };
    if !crate::ikbd::capture_begin() {
        return;
    }
    crate::drivers::usb::poll();
    crate::ikbd::capture_end();
}

/// INPUT01: the input pump of the 200 Hz timer interrupt. Called by the IRQ
/// handler after the tick has been credited and EOIed, still with IRQs
/// masked; `spsr` is the interrupted context's `SPSR_EL1`.
///
/// Pumps only when [`crate::input_gate::irq_pump_allowed`] says the
/// interrupted context is a program at EL0 with no kernel operation
/// suspended underneath it — the reasons are in `crate::input_gate`. Then the
/// pointer moves on while the program computes or between its traps (a timer
/// interrupt that fell due during a trap is taken on the first EL0
/// instruction after it), as with EmuTOS' interrupt-driven IKBD path
/// (`bios/aciavecs.S:249`, `vdi/vdi_mouse.c:668-686`).
///
/// Only pure pointer motion is decoded here ([`pump_devices_irq`]); button
/// changes, the wheel and keys keep their pre-INPUT01 moment in the
/// trap-side pump — so no key byte reaches a `kbdvec` hook's ring from the
/// interrupt, and no upcall is ever delivered from it (the upcall does not
/// save the FP/SIMD registers of the code it interrupts,
/// `docs/api/xbios/22-kbdvbase.md` C3).
pub fn irq_input_pump(spsr: u64) {
    use crate::input_gate::{irq_pump_allowed, mode, spsr_is_el0, Interrupted};
    use core::sync::atomic::Ordering;
    use tos_api::bdos::process::{current, slot_for, PROCESS_TABLE};

    let from_el0 = spsr_is_el0(spsr);
    if !from_el0 {
        return;
    }
    let slot = slot_for(current());
    let ctx = Interrupted {
        from_el0,
        process: slot.is_some(),
        upcall_depth: slot.map_or(0, |s| PROCESS_TABLE[s].upcall_depth.load(Ordering::Acquire)),
        aes_suspended: tos_api::aes::thread::is_suspended(),
    };
    if irq_pump_allowed(mode(), ctx) {
        pump_devices_irq();
    }
}

/// The blocking key read shared by [`Host::console_read_entry_blocking`]
/// and [`Host::console_read_entry_restartable`]: pops the whole IOREC entry
/// (`shifty << 24 | scancode << 16 | ascii`) via `rx_pop_entry` — it does
/// *not* skip scancode-only keys (ASCII 0), so arrow / function keys reach
/// `Bconin(2)`/`Cconin` and friends. Runs inside the synchronous SVC handler
/// with IRQs masked, so unmask + `wfi` while waiting (qemu-virt, woken by
/// the 200 Hz tick), or spin (rpi4); both poll USB-HID.
///
/// With `give_way`, returns `None` as soon as the running process has bytes
/// queued for its `kbdvec` hook (KEYS02, `docs/api/xbios/22-kbdvbase.md` B4):
/// those are delivered on the way back to EL0, so the caller must let the trap
/// restart instead of waiting for a key the hook itself may have to produce.
fn read_key_loop(give_way: bool) -> Option<u32> {
    let daif: u64;
    // SAFETY: reading DAIF and clearing the IRQ mask are permitted
    // at EL1; we restore DAIF before returning.
    unsafe {
        core::arch::asm!(
            "mrs {0}, daif",
            out(reg) daif,
            options(nomem, nostack, preserves_flags),
        );
        core::arch::asm!(
            "msr daifclr, #2",
            options(nomem, nostack, preserves_flags),
        );
    }

    let word = loop {
        // Drain USB-HID interrupt transfers (keyboard) → ikbd::feed →
        // the key ring — the sole console input source on both boards;
        // no-op until a HID device is enumerated. Key auto-repeat after
        // the input ([`pump_devices`]).
        pump_devices();

        if let Some(e) = crate::console::rx_pop_entry() {
            break Some(e);
        }
        if give_way && tos_api::xbios::kbdvecs::pending_for_running() {
            break None;
        }
        // rpi4 polls the USB-HID keyboard, so spin lightly (no IRQ
        // wakes a `wfi` for CON input); qemu-virt `wfi`s until the next
        // interrupt (the 200 Hz tick) and polls again.
        #[cfg(feature = "board-rpi4")]
        core::hint::spin_loop();
        #[cfg(not(feature = "board-rpi4"))]
        // SAFETY: wait for the next interrupt (the timer tick wakes us).
        unsafe {
            core::arch::asm!("wfi", options(nomem, nostack, preserves_flags));
        }
    };

    // SAFETY: restore the caller's original DAIF mask state.
    unsafe {
        core::arch::asm!(
            "msr daif, {0}",
            in(reg) daif,
            options(nomem, nostack, preserves_flags),
        );
    }
    word
}

/// Adapter exposing the EMMC2 SD card as a TOS-API [`BlockDev`].
#[cfg(feature = "board-rpi4")]
struct KernelBlock;

/// Translate a driver failure into the TOS error code the BIOS block
/// interface is defined in terms of — the same mapping EmuTOS' own SD
/// driver makes in `bios/sd.c` (table in `docs/hardware/emmc-sd.md` §5).
///
/// Before HW03 this function did not exist: the driver's `Result` was
/// flattened to a `bool` here, which is where every device error stopped
/// (audit finding 5). `write` selects between the write and the read
/// fault code exactly as `sd_rw` does with `rw & RW_RW` (`sd.c:188`).
#[cfg(feature = "board-rpi4")]
fn block_error_code(e: crate::hal::block::BlockError, write: bool) -> tos_types::Long {
    use crate::hal::block::BlockError;
    use tos_types::error::{EDRVNR, EREADF, ESECNF, EUNDEV, EWRITF};
    match e {
        // "error but not timeout" — the card refused the command, or the
        // transfer broke on the bus (`sd.c:187-190`). EmuTOS makes no
        // distinction between the two here.
        BlockError::Card | BlockError::Io => {
            if write {
                EWRITF
            } else {
                EREADF
            }
        }
        // A wait ran out of time, or there is no card: `sd.c:195` and
        // `sd.c:172-173`. Both are "drive not ready" for a card that
        // cannot report a media change.
        BlockError::Timeout | BlockError::NotReady => EDRVNR,
        // The block is not on this device: EmuTOS' own range check answers
        // a sector past the unit with `ESECNF` (`bios/blkdev.c:488-490`).
        BlockError::OutOfRange => ESECNF,
        // A request this device cannot serve at all — the code
        // `blkdev_rwabs` gives an unusable device (`bios/blkdev.c:440-441`).
        BlockError::Unsupported => EUNDEV,
    }
}

#[cfg(feature = "board-rpi4")]
impl tos_api::bios::block::BlockDev for KernelBlock {
    fn block_count(&self) -> u64 {
        use crate::hal::block::BlockDevice;
        crate::drivers::bcm2711::emmc::SDCARD.block_count()
    }
    fn read_blocks(&self, lba: u64, buf: &mut [u8]) -> Result<(), tos_types::Long> {
        use crate::hal::block::BlockDevice;
        crate::drivers::bcm2711::emmc::SDCARD
            .read_blocks(lba, buf)
            .map_err(|e| block_error_code(e, false))
    }
    fn write_blocks(&self, lba: u64, buf: &[u8]) -> Result<(), tos_types::Long> {
        use crate::hal::block::BlockDevice;
        crate::drivers::bcm2711::emmc::SDCARD
            .write_blocks(lba, buf)
            .map_err(|e| block_error_code(e, true))
    }
}

#[cfg(feature = "board-rpi4")]
static KERNEL_BLOCK: KernelBlock = KernelBlock;

/// Static reference handed to `tos_api::host::install_host`.
static KERNEL_HOST: KernelHost = KernelHost;
static KERNEL_HOST_REF: &'static dyn Host = &KERNEL_HOST;

/// Install the kernel host into `tos-api`. Called once from
/// `kernel_main`.
pub fn install() {
    tos_api::host::install_host(&KERNEL_HOST_REF);
}
