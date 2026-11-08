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
//!   at EL0 with no kernel operation suspended underneath it — no upcall in
//!   flight, the AES fiber not suspended — and the input pump is not confined
//!   to the trap path (boot argument `input=aes`): the context of INPUT01's
//!   pump, [`crate::input_gate::frame_tick_allowed`]. A program that
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
//! * The interrupt runs `vb_draw` only over a program at EL0 with nothing
//!   suspended underneath: every trap runs with IRQs masked, so an interrupt
//!   from EL0 interrupts no running kernel code; and an EL0 upcall (a
//!   `kbdvec` hook, a `G_USERDEF` callout in the middle of an AES draw) or a
//!   suspended AES fiber — where a kernel operation *is* suspended under the
//!   program — is excluded by the same conditions as INPUT01's pump
//!   (`crate::input_gate`). An interrupt taken at EL1 — inside an input
//!   pump's open-IRQ window, a blocking read, the boot path or the shell —
//!   never runs `vb_draw`.
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
//! over a quiet EL0 program, and an interrupt over the pump one does not draw (the cursor
//! blink it may run draws through the scalar row methods).

use core::sync::atomic::{AtomicU64, Ordering};

use crate::input_gate;

/// The frame [`run_if_due`] last ran for.
static LAST_FRAME: AtomicU64 = AtomicU64::new(0);

/// The frame tick of the 200 Hz timer interrupt; `spsr` is the interrupted
/// context's `SPSR_EL1`. Runs [`run_if_due`] only in the context the INPUT01
/// input pump needs ([`input_gate::frame_tick_allowed`] =
/// [`input_gate::irq_pump_allowed`]): the interrupt came from a program at
/// EL0, with no upcall in flight and the AES fiber not suspended, and the
/// pump may run from the interrupt at all ([`input_gate::InputMode::Irq`], the default;
/// `input=aes` keeps all pointer work on the trap path, as before INPUT01).
/// The context is read as `crate::host_env::irq_input_pump` reads it.
pub fn irq_tick(spsr: u64) {
    use tos_api::bdos::process::{current, slot_for, PROCESS_TABLE};

    let from_el0 = input_gate::spsr_is_el0(spsr);
    if !from_el0 {
        return;
    }
    let slot = slot_for(current());
    let ctx = input_gate::Interrupted {
        from_el0,
        process: slot.is_some(),
        upcall_depth: slot.map_or(0, |s| PROCESS_TABLE[s].upcall_depth.load(Ordering::Acquire)),
        aes_suspended: tos_api::aes::thread::is_suspended(),
    };
    if input_gate::frame_tick_allowed(input_gate::mode(), ctx) {
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
