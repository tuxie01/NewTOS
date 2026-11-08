//! The frame tick — the part of EmuTOS' VBL interrupt NewTOS has (TASK/V55).
//!
//! # What EmuTOS does
//!
//! The VBL interrupt (`references/emutos/bios/vectors.S:329-496`, `_int_vbl`)
//! runs once per displayed frame and, among other things, calls the routines
//! of the `vblqueue` (`:436-464`). The VDI puts its pointer redraw into the
//! first slot, `vblqueue[0] = vb_draw` (`vdi/vdi_mouse.c:602`, again at `:631`).
//! The mouse interrupt only records where the pointer is to go (`mov_cur`,
//! `vdi/vdi_asm.S:496-532`); `vb_draw` (`vdi/vdi_mouse.c:668-686`) draws it
//! there in the next frame. A program that reads the screen between the
//! packet and the frame sees the pointer at the old place while `vq_mouse`
//! already reports the new one (wkprobe M1, M2; audit VDI-WK-004).
//!
//! # What NewTOS does
//!
//! There is no frame interrupt. The frame is the ~60 Hz frame clock `Vsync`
//! waits on (`Host::frame_clock`, `CNTPCT * 60 / CNTFRQ`, a counter read
//! that is callable from any context); [`run_if_due`] calls
//! [`tos_api::vdi::vb_draw`] once per frame of it, at the first of two kinds
//! of moments after the frame began:
//!
//! * [`irq_tick`] — the 200 Hz timer interrupt, when it interrupted a program
//!   at EL0 (and the input pump is not confined to the trap path, boot
//!   argument `input=aes`, [`crate::input_gate::InputMode`]). A program that
//!   computes, or returns from a `Vsync`, gets its pointer redrawn here.
//! * [`pump_tick`] — the start of every trap-side input pump
//!   (`crate::host_env` `pump_devices`: the AES wait loops, the blocking
//!   console reads, `Bconstat` on an empty queue). A program parked in an AES
//!   or console wait gets its pointer redrawn here.
//!
//! The pointer therefore follows a packet within one frame plus at most one
//! 5 ms tick, and not inside a long trap — where the packet is not decoded
//! either (`crate::input_gate`, "What is deliberately not done"). Only
//! `vb_draw` runs: NewTOS has no program-visible `vblqueue` (ADR-031); the
//! VT52 cursor blink, EmuTOS' other VBL client the console uses, keeps its own
//! per-tick call (`crate::fbcon::blink_tick`).
//!
//! # Why the two moments are safe
//!
//! `vb_draw` touches the pointer's save-under and draws through the screen
//! backend. Neither moment interrupts a cursor operation or a backend call:
//!
//! * An interrupt taken from EL0 interrupts no kernel code — every trap runs
//!   with IRQs masked, and the kernel never returns to EL0 in the middle of a
//!   cursor operation (`crate::input_gate`, first condition). An interrupt
//!   taken at EL1 — inside an input pump's open-IRQ window, a blocking read,
//!   the boot path or the shell — never runs `vb_draw`.
//! * The pump moment is the trap path itself, between two complete
//!   operations: no cursor function calls the pump.
//!
//! The VDI side keeps EmuTOS' guards on top (`tos-api/src/vdi/mouse.rs`):
//! `vb_draw` does nothing while the hide count or `mouse_flag` is set, the
//! hide paths raise the count before they erase, and the form change holds
//! `mouse_flag`.
//!
//! # FP/SIMD context (ADR-036 point 3)
//!
//! `vb_draw`'s save-under and sprite run through the NEON kernels
//! (`crate::raster_copy` `read_rect`/`write_rect`, `crate::raster_expand`
//! `sprite`). From the interrupt the V registers hold the interrupted EL0
//! program's live values — the case INPUT01's pointer motion has run since
//! TASK/V59 (`crate::raster_copy`, the note before `copy_rows`): each kernel
//! stores every V register it uses to a save area on its own stack frame and
//! loads it back before its `asm!` block ends, uses integer/bitwise SIMD only
//! (`FPCR`/`FPSR` untouched), and neither calls, waits nor switches context
//! inside; `CPACR_EL1.FPEN` is `0b11` from boot (`kernel/src/arch/aarch64/
//! boot.rs:337@cpacr_el1`), so nothing traps; the EL0 FP file is saved and
//! loaded only on the synchronous `svc` paths (`save_fp_context`,
//! `kernel/src/arch/aarch64/exceptions.rs:999@save_fp_context`), which an
//! interrupt never runs. From the pump the case is the one of every VDI trap.
//! A nested kernel cannot occur on either path: the interrupt one runs only
//! over EL0, and an interrupt over the pump one does not draw (the cursor
//! blink it may run draws through the scalar row methods).

use core::sync::atomic::{AtomicU64, Ordering};

use crate::input_gate::{self, InputMode};

/// The frame [`run_if_due`] last ran for.
static LAST_FRAME: AtomicU64 = AtomicU64::new(0);

/// The frame tick of the 200 Hz timer interrupt; `spsr` is the interrupted
/// context's `SPSR_EL1`. Runs [`run_if_due`] only when the interrupt came
/// from EL0 and the input pump may run from the interrupt at all
/// ([`InputMode::Irq`], the default; `input=aes` keeps all pointer work on
/// the trap path, as before INPUT01).
pub fn irq_tick(spsr: u64) {
    if input_gate::spsr_is_el0(spsr) && input_gate::mode() == InputMode::Irq {
        run_if_due();
    }
}

/// The frame tick of the trap-side input pump, called at its start.
pub fn pump_tick() {
    run_if_due();
}

/// EmuTOS' `vblqueue[0]` call: `vb_draw` once per frame — if a frame began
/// since the last run, and a screen is up.
fn run_if_due() {
    let Some(host) = tos_api::host::host() else {
        return;
    };
    let frame = host.frame_clock();
    if LAST_FRAME.swap(frame, Ordering::Relaxed) == frame {
        return;
    }
    if !crate::vdi_backend::available() {
        return;
    }
    tos_api::vdi::vb_draw(&crate::vdi_backend::VDI_SCREEN);
}
