//! NewTOS kernel (library crate root).
//!
//! The kernel is split into `src/lib.rs` (this file: modules, library API)
//! and `src/main.rs` (the boot binary that pulls the lib in and exposes
//! the `kernel_main` symbol referenced by the boot stub).

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]
#![allow(internal_features)]

extern crate alloc;

#[global_allocator]
static GLOBAL_ALLOCATOR: bios::machine::heap::KernelAllocator =
    bios::machine::heap::KernelAllocator;

// Compile-time enforcement: exactly one board feature must be selected.
#[cfg(all(feature = "board-rpi4", feature = "board-qemu-virt"))]
compile_error!(
    "kernel: features `board-rpi4` and `board-qemu-virt` are mutually exclusive."
);

#[cfg(not(any(feature = "board-rpi4", feature = "board-qemu-virt")))]
compile_error!(
    "kernel: exactly one of `board-rpi4` or `board-qemu-virt` must be enabled."
);

pub mod arch;
pub mod bios;
pub mod bootargs;
pub mod console;
pub mod drivers;
pub mod hal;
pub mod host_env;
pub mod panic;
pub mod selftest;
pub mod selftest_d3;
pub mod selftest_d6;
pub mod selftest_d7;
pub mod selftest_ec4;
pub mod selftest_time;
pub mod selftest_pexec;
pub mod selftest_pexec_d4d5;
pub mod bench;
pub mod fbcon;
pub mod ikbd;
pub mod input_gate;
pub mod key_repeat;
pub mod keymap;
pub mod reset;
pub mod shell;
pub mod selftest_aes;
pub mod selftest_bios;
pub mod selftest_console;
pub mod selftest_cookie;
pub mod selftest_objc;
pub mod selftest_sysvars;
pub mod selftest_upcall;
pub mod selftest_sleepwake;
pub mod selftest_sleepwake_switch;
pub mod selftest_timer_restart;
pub mod selftest_etv_term;
pub mod accs;
pub mod aes_demo;
pub mod embedded;
pub mod launch;
pub mod shell_loop;
pub mod selftest_gemtrap;
pub mod selftest_ikbd;
pub mod selftest_vdi;
pub mod selftest_raster;
pub mod selftest_fbcon;
pub mod selftest_xbios;
pub mod sync;
pub mod initinfo;
/// Boot-log capture to `C:\USBDIAG.LOG` (feature `usb-diag` only).
#[cfg(feature = "usb-diag")]
pub mod bootlog;
#[cfg(feature = "board-rpi4")]
pub mod selftest_emmc;
pub mod vdi_backend;
pub mod vdi_client;

// TEMPORARY debug tooling — NOT for product history. Gated behind
// `dbg_qedtrace::SVC_TRACE` (QED mode / `svc-seq-trace` feature); remove the
// module + its exception hooks to drop it entirely.
pub mod dbg_qedtrace;
/// BIOS/XBIOS argument tracer to the serial console (TOOL08; feature
/// `bios-arg-trace` only).
#[cfg(feature = "bios-arg-trace")]
pub mod dbg_biostrace;

// ---------------------------------------------------------------------------
// kprintf! / kprintln! (Phase 03 D7) — formatter macros into the boot
// console. Safe to call from any context (panic, IRQ, pre/post MMU) as
// long as `console::CONSOLE` has been initialized at least once. The
// PL011 driver writes byte-by-byte through MMIO with no internal
// locking, so the macros are non-blocking and recursion-safe; bytes
// from concurrent callers may interleave but neither caller will hang.
// ---------------------------------------------------------------------------

/// Formatted write to the kernel console without a trailing newline.
///
/// Equivalent to `core::write!(UartWriter(&CONSOLE), …)`. The
/// formatter result is discarded — `UartWriter::write_str` cannot
/// fail.
#[macro_export]
macro_rules! kprint {
    ($($arg:tt)*) => {{
        use ::core::fmt::Write as _;
        let _ = ::core::write!(
            $crate::hal::uart::UartWriter(&$crate::console::CONSOLE),
            $($arg)*
        );
    }};
}

/// Formatted write to the kernel console with a trailing newline.
#[macro_export]
macro_rules! kprintln {
    () => {{ $crate::kprint!("\n"); }};
    ($($arg:tt)*) => {{
        use ::core::fmt::Write as _;
        let _ = ::core::writeln!(
            $crate::hal::uart::UartWriter(&$crate::console::CONSOLE),
            $($arg)*
        );
    }};
}

/// Friendly board name for boot output.
#[cfg(feature = "board-rpi4")]
pub const BOARD_NAME: &str = "rpi4";
/// Friendly board name for boot output.
#[cfg(feature = "board-qemu-virt")]
pub const BOARD_NAME: &str = "qemu-virt";

/// Rust kernel entry. Called by the AArch64 boot stub once the stack is
/// set and BSS is zeroed.
///
/// This is a thin naked shim that performs the SPSel = 0 switch
/// required by Phase 03 D6 (stack guard) before any Rust function
/// runs:
///
/// 1. While SPSel is still 1 (boot-stub default), repoint SP_EL1 at
///    the dedicated emergency exception stack via `mov sp, x_emerg`.
///    The boot stub's pre-MMU stack is now abandoned — no Rust code
///    has executed on it past the BSS-zero loop.
/// 2. Program SP_EL0 with the in-image main kernel stack top using
///    the system-register write (allowed at EL1 regardless of SPSel).
/// 3. `MSR SPSel, #0` (register form) — flip the active stack to
///    SP_EL0. From here on, every exception taken from EL1 enters via
///    the "Current EL with SP0" vector group and auto-promotes SPSel
///    to 1, so handlers run on SP_EL1 = emergency stack.
/// 4. Tail-call [`kernel_main_rust`].
///
/// The shim is `#[unsafe(naked)]` so that no compiler-generated
/// prologue interferes with the precise sequencing.
///
/// `x0` carries the device-tree blob pointer from the boot stub
/// (Phase 03 D8), `x1` the exception level the stub was entered at
/// (SER01). The shim deliberately preserves both so they reach
/// `kernel_main_rust` as the AAPCS64 first and second argument.
///
/// Note on `MSR SP_EL1, Xt`: that encoding is `UNDEFINED` at EL1
/// (ARM ARM C5.2.20 — SP_EL1 is writable as a system register only
/// from EL2 or higher). The only legal way to set SP_EL1 from EL1 is
/// `mov sp, …` while SPSel = 1, which is what step 1 does.
///
/// # Safety
///
/// Must be called exactly once, from the AArch64 boot stub
/// (`arch::aarch64::boot`), as the very first Rust function on the
/// boot CPU. Pre-conditions enforced by the boot stub, not by this
/// function:
///
/// - SPSel = 1 (active SP is SP_EL1) on entry, so the `mov sp, x_emerg`
///   in step 1 actually programs SP_EL1 (the only legal SP_EL1 write
///   path at EL1).
/// - `.bss` has been zeroed.
/// - `x0` holds the device-tree-blob physical address (or `0`).
///
/// The function never returns and is sound only as the kernel
/// entry point; calling it from anywhere else would clobber the
/// active stack and switch SP banks unexpectedly.
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kernel_main() -> ! {
    core::arch::naked_asm!(
        // x0 = DTB phys (from boot stub) — preserved end-to-end.
        // Step 1: SP_EL1 := emergency exception stack top.
        "ldr   x11, ={emerg_top}",
        "mov   sp, x11",
        // Step 2: SP_EL0 := main kernel stack top.
        "ldr   x10, ={stack_top}",
        "msr   sp_el0, x10",
        // Step 3: switch the active SP to SP_EL0.
        "mov   x12, #0",
        "msr   spsel, x12",
        // Step 4: enter Rust. x0 still carries the DTB pointer, x1 the
        // entry EL.
        "b     {rust}",
        stack_top = sym __stack_top_exclusive,
        emerg_top = sym __emerg_stack_top_exclusive,
        rust      = sym kernel_main_rust,
    )
}

unsafe extern "C" {
    static __stack_top_exclusive: u8;
    static __emerg_stack_top_exclusive: u8;
}

/// Rust-level kernel entry. Tail-called by the [`kernel_main`] naked
/// shim with `x0 = device-tree-blob phys addr` (or `0` if the firmware
/// did not provide one) and `x1 = entry exception level` (1..3).
///
/// Phase 03 D8: the DTB pointer feeds `bios::machine::layout::discover` so the
/// memory map reflects the actual installed DRAM rather than the
/// compile-time board default.
// The `initinfo-diag` build ends the boot inside `initinfo::diag::show()`
// (a `-> !`), which makes the rest of this function unreachable — that is the
// point of that build, not a defect.
#[cfg_attr(feature = "initinfo-diag", allow(unreachable_code))]
extern "C" fn kernel_main_rust(dtb_phys: usize, entry_el: u64) -> ! {
    use crate::hal::uart::Uart;

    console::CONSOLE.init();
    // SER01: on board-rpi4 the serial output stays off unless the boot
    // arguments carry `serial=on`; decide before the first byte is printed.
    #[cfg(feature = "board-rpi4")]
    let serial_switched_on = console::configure_serial_tx(entry_el);
    #[cfg(not(feature = "board-rpi4"))]
    let _ = entry_el;
    kprintln!("NewTOS booting on {}", BOARD_NAME);
    #[cfg(feature = "board-rpi4")]
    if serial_switched_on {
        kprintln!("[serial] output on: boot arguments contain `serial=on`");
    }
    kprintln!("Phase 02 reached.");

    let dtb = if dtb_phys == 0 { None } else { Some(dtb_phys) };
    if let Some(p) = dtb {
        kprintln!("[mm] DTB at 0x{:016x}", p);
    } else {
        kprintln!("[mm] no DTB — using compiled-in DRAM size");
    }

    // Ask the board firmware how much DRAM it actually gives us. Runs
    // before the memory map exists because the answer *is* the map's
    // upper bound; the Stage-1 boot tables already cover DRAM and MMIO,
    // so the mailbox is reachable here.
    let fw_dram = firmware_dram();
    report_dtb_memory(dtb);
    // ARCH03 3b: memory protection on unless the boot arguments say
    // `memprot=off`; decided now, while the boot arguments are reachable
    // with the boot tables, and applied before the BDOS pools are seeded.
    let memprot = memprot_switch(dtb);
    // ARCH03 stage 4: per-process ASIDs (`asid=on`), off by default.
    let asid = asid_switch(dtb);
    // FS08 part 3: raise the ARM core from the firmware's boot rate to its
    // maximum (`cpuclk=firmware` keeps the boot rate). Early, so the rest of
    // the boot already runs at the final clock.
    #[cfg(feature = "board-rpi4")]
    raise_arm_clock(dtb);
    // FS09: the size of the block read cache (`bcache=` / `bcachepct=`);
    // its memory is carved off the top of DRAM with the other OS buffers.
    let bcache = bcache_switch(dtb);
    // MEM05: the per-process limit of allocated blocks (`mdlimit=<n>`,
    // 0 = off); applied with the BDOS pools.
    let mdlimit = boot_switch(dtb, bootargs::Setting::Default, |args: &[u8]| {
        bootargs::numeric_setting(args, bootargs::MDLIMIT, u32::MAX)
    });
    // INPUT01: where the keyboard/mouse pump runs. Default: also from the
    // timer interrupt while a program runs at EL0 (pointer motion only);
    // `input=aes` keeps it in the trap-side pump sites only (the behaviour
    // before INPUT01). Decided now, while the boot arguments are reachable,
    // long before IRQs open.
    let input_aes = boot_switch(dtb, false, |args: &[u8]| {
        bootargs::has_token(args, bootargs::INPUT_AES)
    });
    input_gate::set_mode(if input_aes {
        input_gate::InputMode::Aes
    } else {
        input_gate::InputMode::Irq
    });
    // Discover the physical memory map and dump it for visibility.
    // Pure function of the board constants, the DTB and the firmware
    // answer above (no MMIO, no allocation of its own).
    let map = bios::machine::layout::discover(dtb, fw_dram, u64::from(bcache.kib) * 1024);
    print_memory_map(&map);

    // Replace the Stage-1 boot tables (1-GiB blocks, both TTBRs
    // identical, set up by `arch::aarch64::boot::_start`) with the
    // full Stage-2 mapping and clear TTBR0_EL1 so any low-half VA
    // access faults from this point on (Phase 05 D3-bis / D4).
    // SAFETY: boot CPU only, single-threaded; the static page
    // tables are mutated and TTBR0/TTBR1 swapped inside.
    unsafe { arch::aarch64::mmu::reinstall(&map) };
    kprintln!("[mm] MMU reinstalled (high-half via TTBR1, TTBR0 deferred to Phase 06).");

    // ARCH03 stage 4: with `asid=on` size the ASID (TCR_EL1.AS from
    // ID_AA64MMFR0_EL1.ASIDBits) and switch the user tables to nG leaves and
    // ASID-tagged roots — before any user table entry exists. Off: nothing
    // changes (TCR_EL1 as booted, ASID 0, TLBI on every switch).
    if let Some(tiny) = asid {
        // SAFETY: boot CPU; TTBR0 holds the empty root (reinstall), no
        // process root has been built or installed.
        let (bits, field) = unsafe { arch::aarch64::mmu::size_asids() };
        let limit = if tiny { tos_api::bdos::process::mmu::ASID_TEST_LIMIT } else { 1u32 << bits };
        // SAFETY: boot CPU, before `map_pools` and any root install.
        unsafe {
            tos_api::bdos::process::mmu::configure_asids(limit, arch::aarch64::mmu::empty_l0_pa())
        };
        kprintln!(
            "[mm] ASIDs ON: {}-bit (ID_AA64MMFR0_EL1.ASIDBits=0x{:x}), {} per generation{}",
            bits,
            field,
            limit - 1,
            if tiny { " (asid=tiny: rollover test)" } else { "" },
        );
    }

    // Initialise the physical frame allocator (Phase 03 D4).
    bios::machine::frames::init(&map);
    print_frame_stats();
    smoke_test_frame_allocator();

    // Bring up the kernel heap on top of the frame allocator
    // (Phase 03 D5). After this point `alloc::{Box, Vec, String}` is
    // legal kernel-side.
    bios::machine::heap::init(&map);
    print_heap_stats("after init");
    smoke_test_heap();
    print_heap_stats("after smoke");

    // Bring up the framebuffer (if any). Done before the BDOS pools
    // claim the remaining frames so the framebuffer's contiguous run is
    // reserved up front. The screen is drawn later via the VDI.
    init_framebuffer(&map);

    // Bring up the BDOS memory pools (Phase 04 D6). The two MPBs
    // are seeded with whatever frames remain free in each pool
    // after the kernel heap reservation. From this point on the
    // frame allocator must not hand out frames that overlap these
    // ranges — `claim_remaining_pool` enforces this by allocating
    // the entire remaining contiguous run before publishing it to
    // BDOS.
    bring_up_bdos_pools(memprot, mdlimit);
    print_bdos_pool_stats("after init");

    // Install exception vectors and the TOS-API host before any SVC.
    arch::aarch64::exceptions::init();
    host_env::install();

    // Seed the NewTOS cookie jar (system-feature registry, read via the
    // Ssystem trap). `NTOS` = 0x0000_0100, set at NewTOS 0.1.0 and left as is
    // when the product version became 0.5 (2026-10-09): a cookie value is
    // program-visible (`docs/api/bios/_cookie-jar.md`), so it changes only on
    // its own decision, not with the version string.
    tos_api::cookie::seed_defaults(0x0000_0100);

    // qemu-virt: AUX (BIOS 1) reads the serial receive ring of `Iorec(0)`
    // (XBIOS15) — in the device table before the receive interrupt is on.
    #[cfg(feature = "board-qemu-virt")]
    console::install_aux();

    // Bring up the GIC, enable the PL011 RX interrupt, and unmask IRQs.
    drivers::gic::init();
    console::CONSOLE.enable_rx_interrupt();

    // Start the 200 Hz system tick — EmuTOS' `init_system_timer()`
    // (`bios/mfp.c:193`, called from `bios_init` right before interrupts are
    // enabled, `bios/bios.c:384`). It drives `_hz_200` (`$4BA`), the counter
    // programs read for elapsed time.
    if !drivers::timer::init() {
        kprintln!("[timer] no usable CNTFRQ_EL0 — 200 Hz system tick is OFF");
    }

    kprintln!(
        "[input] keyboard/mouse pump: {}",
        if input_aes {
            "AES waits and console reads only (input=aes)"
        } else {
            "AES waits and console reads; pointer motion also from the 200 Hz tick while a program runs (input=aes switches that off)"
        }
    );
    drivers::gic::enable_irqs();

    // Bring up the microSD card and mount its partitions before the boot
    // screen, so the drive map is populated when initinfo renders it.
    // board-rpi4 / QEMU raspi4b only (no block device on aarch64-virt).
    // FS08: `emmc=compat` in the boot arguments keeps the pre-FS08 SD path
    // (1-bit bus, one CMD17/CMD24 per sector); default is multi-block + 4-bit.
    #[cfg(feature = "board-rpi4")]
    drivers::bcm2711::emmc::set_compat(boot_switch(dtb, false, |args: &[u8]| {
        bootargs::has_token(args, bootargs::EMMC_COMPAT)
    }));
    // FS09: the block read cache under every access to the card, installed
    // before the first sector is read (`bios::mount`).
    install_block_cache(&map, &bcache);
    #[cfg(feature = "board-rpi4")]
    // SAFETY: boot CPU; VideoCore firmware has released the card.
    if unsafe { drivers::bcm2711::emmc::init() } {
        tos_api::bios::mount();
        // Select the keyboard layout from the card (KEYBOARD.KBD override or
        // the #K line in NEWDESK.INF), before the console/desktop reads keys.
        keymap::init_from_disk();
        // Seed the wall clock from the running kernel image's FAT timestamp
        // (no battery RTC yet). Must run after `mount()` so `C:` is readable
        // and before `initinfo::run()` so the boot screen shows the date.
        seed_clock_from_kernel_file();

        // Self-test build only (`--features selftest-emmc`): the Phase-09
        // microSD/FAT/GEMDOS-file checks, with hard assertions on the
        // Fattrib contract (`fmeta_test`). Runs here, after the normal
        // bring-up, because `emmc::init` and `bios::mount` are idempotent
        // and the test re-enters both; the boot then continues. It writes
        // to C:, so it is a test-image feature only (see `Cargo.toml`).
        #[cfg(feature = "selftest-emmc")]
        selftest_emmc::run();
    }

    // AES boot initialisation (EmuTOS `gem_main`/`run_accs_and_desktop`
    // prologue): read the `.INF` file's `#Q` desktop-background line
    // (`process_inf1`, geminit.c:343-392), fix up the AES's own built-in
    // DESKTOP resource for the open workstation (`gem_rsc_fixit` + the
    // full-screen `ob_height`, geminit.c:715-717) and apply the background
    // byte (`set_aes_background`, geminit.c:635-641). Must run after the
    // card is mounted (the `.INF` file lives on C:) and before anything can
    // call `appl_init`, whose `ap_private` reads that resource
    // (`gemsuper.c:95-96`).
    tos_api::aes::init::init();

    // Bring up the USB host stack (keyboard/mouse) — PHASE-16. On QEMU-virt
    // this finds the xHCI on the PCIe bus and reports its ports; on real
    // Pi-4 it is a no-op until the VL805 firmware path (D7).
    drivers::usb::init(dtb_phys);

    // Diagnostic build only: the enumeration is over, so put the whole boot
    // log on the card while the machine is still healthy. This is the flush
    // that matters — everything the hub chain did is above this line, and a
    // later hang cannot take it away (`flush` rewrites the file whole, so the
    // second call below simply extends what this one wrote).
    #[cfg(feature = "usb-diag")]
    bootlog::flush("after usb::init");

    // `initinfo(&shiftbits)` (EmuTOS `biosmain`, `bios/bios.c:809`; the
    // `bootdev` half of its return value has no NewTOS counterpart, see
    // `initinfo::wait_for_user`): show the welcome screen, hold it, and come
    // back with the modifier keys the user was holding while it was up.
    // The hold itself — its duration, the any-key skip
    // and the Shift pause — lives in `initinfo::wait_for_user`
    // (`bios/initinfo.c:376-442`); it replaced a fixed 2 s `delay_ms`, which
    // was too short for an HDMI monitor still locking its mode and could not
    // be skipped or extended by the user.
    let shiftbits = initinfo::run();

    // Diagnostic build only: a second, complete copy — this one also carries
    // whatever the boot screen and the wait loop printed.
    #[cfg(feature = "usb-diag")]
    bootlog::flush("after initinfo");

    // Diagnostic build only: report what the wait loop actually saw and stop
    // the boot there, so nothing can overwrite the report (this machine has no
    // serial line). See `initinfo::diag`.
    #[cfg(feature = "initinfo-diag")]
    initinfo::diag::show(shiftbits);

    // Development self-tests are no longer invoked during the normal
    // boot — the system goes straight to the console. They remain in the
    // `selftest*` modules and can be re-enabled here when debugging
    // (e.g. `selftest_vdi::run_phase12_vdi_demo()` draws the Phase-12
    // drawing-surface demo verified by QMP screendump,
    // `selftest_aes::run_phase13_aes_test()` for the AES lifecycle, or
    // `selftest_sysvars::run()` to prove the B1 per-process low-memory
    // sysvars/OSHEADER/$5A0 area — docs/analysis/bios-audit/
    // b1-lowmem-feasibility.md, Option 1 — is populated in a loaded EL0 TPA:
    // checks _sysbase/os_version/_p_cookies + the Setexc($5A0/4,-1) idiom).

    // "if (shiftbits & MODE_CTRL) bootflags |= BOOTFLAG_SKIP_AUTO_ACC;"
    // (EmuTOS `biosmain`, `bios/bios.c:816-820`) — the modifiers held on the
    // welcome screen become boot flags, read by `run_auto_folder` below.
    launch::set_bootflags_from_shiftbits(shiftbits);

    // `vt52_init` (EmuTOS `bios_init`, `bios/bios.c:341`): reset the VT52
    // console and clear it, so the boot / AUTO-program console output
    // (Cconws) starts at the top left with EmuTOS' initial console state —
    // text cursor hidden, no line wrap (`docs/api/bios/_vt52.md` §1).
    // EmuTOS shows its AUTO programs on the console before the GUI.
    fbcon::init();

    // The PDs EmuTOS's `run` is while kernel code is on the stack
    // (TASK/PPARENT, TASK/PROC03, `tos_api::bdos::process::standin`):
    // `initial_basepage` (`bdos/bdosmain.c:337-347`, with `Dsetdrv(bootdev)`
    // at `bios/bios.c:827`) and the AES PD with its copy of `default_env`
    // (`init_default_environment`, `bios/bios.c:626-636,828`; the PD itself
    // `:862-865`); the accdesk and EmuCON PDs live in the same page and are
    // built when they start (`enter_accdesk`, `enter_emucon`). Built here, where
    // the boot drive is known and no program has run yet, and published
    // read-only to every process, so the AUTO programs below already find
    // `initial_basepage` as their `p_parent` and its empty environment.
    // SAFETY: boot CPU, before the first program; pools and user tables exist.
    match unsafe { tos_api::bdos::process::standin::init(tos_api::bdos::fs::drive::boot_drive()) } {
        Some(va) => kprintln!(
            "[proc] stand-in PDs: initial_basepage 0x{:08x}, AES PD 0x{:08x}, accdesk 0x{:08x}, \
             EmuCON 0x{:08x} (read-only to all)",
            va,
            tos_api::bdos::process::standin::aes_pd() as u64,
            tos_api::bdos::process::standin::accdesk_pd() as u64,
            tos_api::bdos::process::standin::emucon_pd() as u64,
        ),
        None => kprintln!(
            "[proc] stand-in PDs NOT published: programs cannot read their p_parent chain"
        ),
    }

    // Run the AUTO folder (\AUTO\*.PRG) before the GUI — EmuTOS' autoexec.
    // No-op until the boot drive has an \AUTO\ folder with programs; an
    // invalid/empty .PRG fails the load gracefully (no fault).
    launch::run_auto_folder();

    // The programs the kernel image carries (DESK43 D3, `crate::embedded`):
    // into the program registry before anything can `Pexec` them, as
    // EmuTOS' ROM desktop is there before the AES starts (`NEWDESK`, U9).
    embedded::register();

    // The AES starts — EmuTOS `exec_os` after `autoexec` (`bios/bios.c:
    // 855-866`): from here on the kernel's `run` is the AES PD while the AES
    // initialises (`aes/geminit.c:753-894`); each session below runs as the
    // accdesk PD (TASK/PPARENT, TASK/PROC03).
    tos_api::bdos::process::standin::enter_aes();

    // ── The AES session loop — EmuTOS `gemstart.S:86-124` `aes_restart` ──
    //
    // One pass through this body is one whole AES session: the tail of
    // `run_accs_and_desktop` (`geminit.c:693-731`) with `sh_main` at the end
    // of it. EmuTOS leaves the session by returning out of `gem_main` into
    // assembly, which re-zeroes the global AES area and jumps back to
    // `gem_main` when `gl_changerez` is still set (`:122-124`); here the same
    // decision is `sh_main`'s return value, and the re-zeroing is
    // `tos_api::aes::reinit()` (A106, ticket `TASK/A106.md`, design
    // `docs/analysis/a48-shw-reschng-design-2026-09-21.md` §5 tier 2). The
    // desktop's own state is a process's and ends with it (DESK43 U10).
    //
    // Only `ShExit::Shutdown` leaves the loop. On a board whose `can_shutdown()`
    // equivalent is FALSE — every NewTOS profile, as on the Falcon this build
    // targets (`bios/bios.c:754-770`) — nothing can set `sh_doexec` to
    // `SHW_SHUTDOWN`, so the `halt()` below is as unreachable as EmuTOS'
    // `Pterm0` after `ui_start`, and just as deliberately present.
    loop {
        // `aes_run_rom_program(accdesk_start)` (EmuTOS `gem_main`,
        // `geminit.c:894`; `gemshlib.c:570-581`) — the session runs in its own
        // process, the accdesk PD (`p_parent` = the AES PD, a copy of its
        // environment): `run` for everything `run_accs_and_desktop` does
        // (`geminit.c:654-750`), so the accessories are loaded under it and
        // the desktop, the `#Z` program and every program the desktop asks
        // for get it as `p_parent` (`gemshlib.c:600-611,645`). A fresh PD per
        // session (`gemstart.S:121-124`). TASK/PROC03 stage 1,
        // `docs/api/gemdos/process/_parent-and-environment.md` §2.3.
        tos_api::bdos::process::standin::enter_accdesk();

        // `gsx_init()` (EmuTOS `run_accs_and_desktop`, `geminit.c:675`) — the
        // AES opens the physical VDI workstation (`v_opnwk` → `gl_handle`,
        // `aes/gemgsxif.c:169-186`) and sets it up (`gsx_start`,
        // `aes/gemgraf.c:383-428`). `graf_handle` reports this handle, so a
        // program's VDI calls on it are answered, and its first `v_opnvwk`
        // gets handle 2. Before the accessories load, as in EmuTOS.
        tos_api::aes::gsxif::gsx_init();

        // `load_accs(num_accs)` (EmuTOS `run_accs_and_desktop`,
        // `geminit.c:687`) — discover up to `NUM_ACCS` `.NAC` desk accessories
        // in `C:\ACC\` and the boot drive's root and load each into a process
        // slot. Before `sh_tographic`, as EmuTOS loads before it; nothing of
        // the accessories runs yet (`crate::accs`, ACC02).
        accs::load_accs(launch::skip_auto_acc());

        // `sh_tographic()` (EmuTOS `run_accs_and_desktop`, `geminit.c:700`) —
        // go into graphic mode now that the character-screen part of the boot
        // (the initinfo screen and the \AUTO\ programs) is done, and before
        // the shell loop starts anything. On a board without a raster backend
        // this is a no-op and the loop's first `sh_chgrf` falls back to alpha
        // mode. On a restart it is the way back out of the alpha screen
        // `sh_main`'s closing `sh_toalpha` left behind (`gemshlib.c:733`).
        shell_loop::go_graphic();

        // `process_inf2` (EmuTOS `geminit.c:734`) — the second `.INF` pass,
        // run after the AES's own init and immediately before the shell loop:
        // it sets the double-click speed from the `#E` line and reads the `#Z`
        // autorun line into the SHELL's `sh_desk`/`sh_cdir` pair, telling us
        // whether a program is configured to run instead of the embedded
        // desktop. Re-run per session, as EmuTOS runs it, so a restarted
        // session starts the same program a cold boot would.
        let (isauto, isgem) = tos_api::aes::init::process_inf2();

        // The AES shell loop (EmuTOS `sh_main`, `geminit.c:739`): it decides
        // which program runs — the `#Z` program, the embedded desktop
        // `NEWDESK`, or the console — starts it, and starts the next one when it terminates.
        // This is what replaced the hardcoded `desk::run()` call: the desktop
        // is now a configuration choice, not a compiled-in one.
        // `dsptch(); wait_for_accs(AP_MESAG);` (`geminit.c:736-737`) — this is
        // where the loaded accessories first execute: each runs its
        // `appl_init` + `menu_register` and parks in its message wait, and the
        // CPU comes back here (ACC03's kernel park). EmuTOS puts it exactly
        // here, after `process_inf2` and before `sh_main`, so that whatever an
        // accessory allocates while initialising belongs to the AES and not to
        // the program `sh_main` starts next (`geminit.c:579-583`).
        accs::wait_for_accs_mesag();

        let exit = shell_loop::sh_main(isauto, isgem);

        // `free_accs(num_accs)` (`geminit.c:741`) — the session is over; give
        // every accessory's memory back. EmuTOS can treat this as optional
        // (its `run_accs_and_desktop` process dies and takes the memory with
        // it); NewTOS has no such process, so this frees each accessory's
        // memory and releases its slot.
        accs::free_accs();

        // `gsx_wsclose()` (`geminit.c:749`) — `v_clswk` on `gl_handle`: every
        // virtual workstation of the session is closed, the physical one
        // stays (`vdi/vdi_control.c:573-590`). On the shutdown path too, as
        // EmuTOS closes before `run_accs_and_desktop` returns.
        tos_api::aes::gsxif::gsx_wsclose();

        // `run_accs_and_desktop` returns into `accdesk_start`, which ends with
        // `Pterm0` (`gemstart.S:160-163`): `run` is the AES PD again
        // (`bdos/proc.c:609`) for `gem_main`'s restart or shutdown.
        tos_api::bdos::process::standin::leave_accdesk();

        if exit == tos_api::aes::shell::ShExit::Shutdown {
            break;
        }

        // A `shel_write(SHW_RESCHNG, …)` was pending when the running program
        // ended: close this AES session and open a fresh one. The physical
        // mode switch `gem_main` would do here (`geminit.c:778-793`) is the
        // one hardware-gated step and stays absent on a single-mode board
        // (ADR-017 rev 2, D-101); every AES structure is rebuilt from
        // scratch. The desktop's own state needs nothing here: it belongs to
        // the `NEWDESK` process, which has ended (DESK43 U10).
        kprintln!("[shell] AES restart (resolution change requested)");
        tos_api::aes::reinit();
        // `dos_sdrv(bootdev)` (`geminit.c:796-803`): "resolution change always
        // resets the default drive to the boot device". NewTOS has no boot
        // device variable; the boot drive is the first hard disk in the drive
        // map, the same derivation the loader writes into `bootdev` ($446)
        // for every process (`bdos/process/loader.rs`). `run` is the AES PD
        // here (accdesk has ended), so its `p_curdrv` changes and the next
        // session's accdesk inherits it (TASK/PROC04 point 2).
        tos_api::bdos::fs::drive::set_boot_drive();
    }

    // Reached only through `shel_write(SHW_SHUTDOWN, …)` — see the loop note.
    kprintln!("[shell] shutdown — halting");
    arch::aarch64::halt();
}

/// Ask the board firmware how much DRAM it gives the ARM, and log the
/// raw answer.
///
/// RPi4: VideoCore mailbox property tags `0x0001_0005` ("Get ARM
/// memory") and `0x0001_0006` ("Get VC memory") — see
/// `docs/hardware/mailbox-framebuffer.md` §3a. The ARM tag reports the
/// part of the fitted DRAM the firmware hands to the CPU, i.e. the
/// installed RAM minus the `gpu_mem` split, which is exactly the extent
/// we may allocate from. The documented response is one `(base, size)`
/// pair with the reservation that a future firmware may answer with
/// several, so contiguous further banks extend the extent and
/// non-contiguous ones are logged and left alone (managing a second,
/// detached bank needs an `xmaddalt`-style second region — not part of
/// this change).
///
/// Returns `None` when the firmware does not answer; the caller then
/// falls back to the device tree (`layout::discover`).
#[cfg(feature = "board-rpi4")]
fn firmware_dram() -> Option<bios::machine::layout::DramReport> {
    let report = match drivers::bcm2711::mailbox::get_memory() {
        Some(r) => r,
        None => {
            kprintln!("[mm] firmware: VideoCore did not answer the memory tag");
            return None;
        }
    };

    for (i, r) in report.arm.iter().take(report.arm_len).enumerate() {
        kprintln!(
            "[mm] firmware: ARM memory bank {}: 0x{:016x} + {} bytes ({} MiB)",
            i, r.base, r.size, r.size / (1024 * 1024),
        );
    }
    match report.vc {
        Some(r) => {
            kprintln!(
                "[mm] firmware: VC memory: 0x{:016x} + {} bytes ({} MiB)",
                r.base, r.size, r.size / (1024 * 1024),
            );
            // SYSINFO01: `Ssystem(S_NEWTOS_GPUMEM)` reports this split.
            host_env::record_gpu_memory(r.size);
        }
        None => kprintln!("[mm] firmware: VC memory: not reported"),
    }

    let first = report.arm[0];
    let mut end = first.base + first.size;
    for r in report.arm.iter().take(report.arm_len).skip(1) {
        if r.size == 0 {
            continue;
        }
        if r.base == end {
            end = r.base + r.size;
        } else {
            kprintln!(
                "[mm] firmware: ARM bank at 0x{:016x} is not contiguous — not managed",
                r.base,
            );
        }
    }
    Some(bios::machine::layout::DramReport { start: first.base, size: end - first.base })
}

/// QEMU `virt` has no VideoCore and therefore no firmware to ask — the
/// device tree QEMU hands us in `x0` is the authority on this board.
#[cfg(feature = "board-qemu-virt")]
fn firmware_dram() -> Option<bios::machine::layout::DramReport> {
    kprintln!("[mm] firmware: no VideoCore mailbox on this board — device tree decides");
    None
}

/// Log what the device tree says about memory, whether or not it ends up
/// being the source used. On a Raspberry Pi this is the second opinion
/// against the firmware answer (the `.dtb` on the card carries a zero-
/// sized `/memory@0`; the boot firmware patches the real banks in).
fn report_dtb_memory(dtb_phys: Option<usize>) {
    let Some(phys) = dtb_phys else { return };
    // SAFETY: the firmware/boot loader supplied this pointer; the parser
    // validates the header before any further read.
    match unsafe { arch::aarch64::dtb::parse_memory(phys) } {
        Some(m) => kprintln!(
            "[mm] device tree: /memory first bank: 0x{:016x} + {} bytes ({} MiB)",
            m.start, m.size, m.size / (1024 * 1024),
        ),
        None => kprintln!("[mm] device tree: no usable /memory node (blob invalid or absent)"),
    }
}

/// Pretty-print the physical memory map on the boot console.
fn print_memory_map(map: &bios::machine::layout::MemoryMap) {
    kprintln!("[mm] memory map:");
    kprintln!(
        "[mm]   DRAM  : 0x{:016x}..0x{:016x} ({} bytes) — source: {}",
        map.dram.start,
        map.dram.end,
        map.dram.len(),
        map.dram_source.label(),
    );
    if map.dram_unmanaged != 0 {
        kprintln!(
            "[mm]   {} bytes above 0x{:016x} NOT managed \
             (limits: {} bytes per pool bitmap, TOS descriptor end < 0x{:x})",
            map.dram_unmanaged,
            map.dram.end,
            bios::machine::layout::POOL_MAX_BYTES,
            bios::machine::layout::TOS_MD_MAX_END,
        );
    }
    print_range("kernel", &map.kernel);
    print_range("ST-RAM", &map.stram);
    print_range("TT-RAM", &map.ttram);
    // Kernel-owned buffers, off the top of DRAM and outside both pools —
    // EmuTOS' `balloc_stram(size, TRUE)` / lowered `memtop`.
    print_range("OSrsvd", &map.os_reserve);
    if !map.block_cache.is_empty() {
        print_range("  bcach", &map.block_cache);
    }
    print_range("  heap", &map.kernel_heap);
    if !map.framebuffer.is_empty() {
        print_range("  fb", &map.framebuffer);
    }
    for (i, r) in map.mmio.iter().enumerate() {
        kprintln!(
            "[mm]   MMIO {:02}: 0x{:016x}..0x{:016x}",
            i, r.start, r.end,
        );
    }
}

fn print_range(label: &str, r: &bios::machine::layout::PhysRange) {
    kprintln!(
        "[mm]   {:<6}: 0x{:016x}..0x{:016x} ({} bytes)",
        label, r.start, r.end, r.len(),
    );
}

/// Bring up the board framebuffer (if present). The screen content is
/// drawn later through the VDI (`initinfo`), not here — this just gets a
/// linear buffer published so [`hal::framebuffer`] / the VDI backend can
/// use it.
#[cfg(feature = "board-qemu-virt")]
fn init_framebuffer(map: &bios::machine::layout::MemoryMap) {
    use crate::hal::framebuffer::Framebuffer;
    // SAFETY: boot CPU, single-threaded; the map's framebuffer reserve is
    // outside both pools, so nothing else can own those bytes.
    let up = unsafe { drivers::qemu_virt::ramfb::init(map) };
    if !up {
        kprintln!("[fb] ramfb unavailable (pass `-device ramfb`) — no graphics");
        return;
    }
    let fb = &drivers::qemu_virt::ramfb::FRAMEBUFFER;
    kprintln!("[fb] ramfb up: {}x{}", fb.width(), fb.height());
}

/// RPi4 framebuffer from the VideoCore mailbox property interface (see
/// docs/hardware/mailbox-framebuffer.md). On real hardware the later VDI
/// drawing is the first life sign (no serial).
#[cfg(feature = "board-rpi4")]
fn init_framebuffer(map: &bios::machine::layout::MemoryMap) {
    // The VideoCore owns the framebuffer on this board, so the memory map
    // reserves nothing for it (BCM2711 §1.2.3: the VC's SDRAM is mapped in
    // from 0x0_4000_0000 downwards, below which `dram` already ends).
    debug_assert!(map.framebuffer.is_empty());
    use crate::hal::framebuffer::Framebuffer;
    // SAFETY: boot CPU, single-threaded, MMU + heap are up.
    let up = unsafe { drivers::bcm2711::framebuffer::init() };
    if !up {
        kprintln!("[fb] VideoCore mailbox framebuffer unavailable — no graphics");
        return;
    }
    let fb = &drivers::bcm2711::framebuffer::FRAMEBUFFER;
    kprintln!(
        "[fb] VideoCore fb up: {}x{} stride {}px",
        fb.width(),
        fb.height(),
        fb.stride_px(),
    );
}

/// Dump per-pool frame counts.
fn print_frame_stats() {
    use crate::bios::machine::frames::{stats, Pool};
    for (label, pool) in [("ST-RAM", Pool::Stram), ("TT-RAM", Pool::Ttram)] {
        let s = stats(pool);
        kprintln!(
            "[mm] frames {}: total={} used={} free={} ({} B free)",
            label, s.total, s.used, s.free(), s.free_bytes(),
        );
    }
}

/// Allocate one frame from each pool, verify the address, free it.
fn smoke_test_frame_allocator() {
    use crate::bios::machine::frames::{alloc_frame, free_frame, stats, Pool};
    for (label, pool) in [("ST-RAM", Pool::Stram), ("TT-RAM", Pool::Ttram)] {
        let before = stats(pool).used;
        let f = alloc_frame(pool).expect("frame alloc failed");
        kprintln!("[mm] alloc {} frame -> 0x{:016x}", label, f.pa());
        free_frame(f);
        let after = stats(pool).used;
        debug_assert!(before == after, "frame leak in smoke test");
    }
}

/// Print kernel-heap accounting with a short label.
fn print_heap_stats(label: &str) {
    let s = bios::machine::heap::stats();
    kprintln!(
        "[mm] heap {}: total={} used={} free={} largest={} blocks={}",
        label, s.total, s.used, s.free(), s.largest_free, s.free_blocks,
    );
}

/// Exercise the kernel heap: a `Box`, a growing `Vec`, and a 1000-cycle
/// alloc/free loop. Verifies leak-free operation (used must return to
/// the post-init baseline) and that the allocator survives churn.
fn smoke_test_heap() {
    use alloc::boxed::Box;
    use alloc::vec::Vec;

    let baseline_used = bios::machine::heap::stats().used;

    {
        let b = Box::new(0xDEAD_BEEF_u64);
        debug_assert_eq!(*b, 0xDEAD_BEEF_u64);
    }
    {
        let mut v: Vec<u32> = Vec::new();
        for i in 0..1024u32 {
            v.push(i);
        }
        debug_assert_eq!(v.len(), 1024);
        debug_assert_eq!(v[1023], 1023);
    }
    for i in 0..1000u32 {
        let v: Vec<u8> = Vec::with_capacity(((i * 7) & 0x3FF) as usize + 1);
        core::hint::black_box(v);
    }

    let after_used = bios::machine::heap::stats().used;
    debug_assert!(
        after_used == baseline_used,
        "heap leak in smoke test"
    );
    kprintln!("[mm] heap smoke OK");
}

/// Phase 04 D6 — claim every still-free frame in each pool as one
/// contiguous run and hand the resulting byte ranges to the BDOS
/// memory subsystem. From this point on, BDOS owns the bytes; the
/// kernel frame allocator only tracks them as "permanently used".
///
/// Heap-pre-allocation (Phase 03 D5) takes its 2 MiB from TT-RAM
/// before this runs, so ST-RAM is contiguous-from-start and TT-RAM
/// is contiguous-from-`heap_end`. Both runs satisfy
/// `alloc_frames_contig`.
fn bring_up_bdos_pools((protect, strict, verify): (bool, bool, bool), mdlimit: bootargs::Setting) {
    use bios::machine::frames::{alloc_frames_contig, stats, Pool, FRAME_SIZE};

    // MEM05: the per-process limit of allocated blocks, before any block
    // exists (design §3.3, decisions E3/E4). An invalid value keeps the
    // default, as for `bcache=`.
    {
        use tos_api::bdos::mem::owner_limit::{set_limit, DEFAULT_OWNER_MD_LIMIT};
        let (n, src) = match mdlimit {
            bootargs::Setting::Given(n) => (n, "boot argument"),
            bootargs::Setting::Default => (DEFAULT_OWNER_MD_LIMIT, "default"),
            bootargs::Setting::Invalid => (DEFAULT_OWNER_MD_LIMIT, "INVALID boot argument ignored, default"),
        };
        set_limit(n);
        if n == 0 {
            kprintln!("[mm] MEM05 per-process block limit: OFF (mdlimit=0, {})", src);
        } else {
            kprintln!("[mm] MEM05 per-process block limit: {} allocated blocks ({})", n, src);
        }
        // The MD supply has no fixed size: it grows page by page from the
        // kernel heap, MDs and the per-block tables together within one
        // page budget (design §2.2/§2.5, decision B).
        use tos_api::bdos::mem::{mdpool, pages};
        kprintln!(
            "[mm] MEM05 MD supply: {} MDs static, grows by {} MDs per {}-byte page from the kernel heap; page budget {} pages ({} KiB) for MDs + block records + snapshot + F_KEEP flags",
            mdpool::MDS_PER_PAGE,
            mdpool::MDS_PER_PAGE,
            pages::PAGE_BYTES,
            pages::MD_PAGE_BUDGET,
            pages::MD_PAGE_BUDGET * pages::PAGE_BYTES / 1024,
        );
    }

    // ARCH03 3b: the protection switch, before the pools exist (their grain
    // and their mapping depend on it), and with protection on the table
    // arena the per-process rights are built from: one L3 per 2-MiB region
    // of the pools (shared, non-owner rights) plus private copies for the
    // regions where a process's rights differ (plan Frage 14). Taken from the
    // frames before the pools claim the rest.
    // SAFETY: boot CPU, before any pool exists.
    unsafe { tos_api::bdos::process::memprot::configure(protect, strict) };
    // MEMPROT-FAST: `memprot=verify` checks every sync against a complete
    // recomputation and reports differences on the serial line (off by
    // default; a debugging and test aid, slow).
    tos_api::bdos::process::memprot::set_verify(protect && verify, false);
    if protect {
        let mib2 = 2 * 1024 * 1024u64;
        let pool_bytes: u64 = [Pool::Stram, Pool::Ttram]
            .iter()
            .map(|&p| stats(p).free() as u64 * FRAME_SIZE)
            .sum();
        let regions = pool_bytes.div_ceil(mib2) as usize;
        let shared = regions + 2 * 2 + 16;
        // The TPA is the largest free block (E9, EmuTOS `alloc_tpa`) until its
        // program shrinks it at start: its owner — and a second slot with the
        // same rights (a `Pexec` 3 caller and its mode-4 child, or a
        // non-strict child that inherits the block) — need a private copy
        // for every region it spans. Two such sets, plus the per-process
        // budget for everything else.
        let count = (shared + 2 * regions + PRIVATE_L3_BUDGET)
            .min(tos_api::bdos::process::mmu::ARENA_MAX_TABLES);
        let frame = alloc_frames_contig(Pool::Ttram, count)
            .or_else(|| alloc_frames_contig(Pool::Stram, count))
            .expect("memprot: no room for the page-table arena");
        let kva = frame.pa() + arch::aarch64::mmu::VA_OFFSET;
        // SAFETY: `count` fresh frames, mapped RW in the high half.
        unsafe { core::ptr::write_bytes(kva as *mut u8, 0, count * FRAME_SIZE as usize) };
        // SAFETY: boot CPU; the frames are ours for good.
        unsafe { tos_api::bdos::process::mmu::set_table_arena(kva, count) };
        kprintln!(
            "[mm] memory protection ON{}{}: table arena 0x{:08x}+{} tables",
            if strict { " (strict)" } else { "" },
            if verify { " (verify: every sync checked against a full recomputation)" } else { "" },
            frame.pa(),
            count,
        );
    } else {
        kprintln!("[mm] memory protection OFF (boot argument `memprot=off`)");
    }

    let claim = |pool: Pool| -> Option<(u64, u64)> {
        let s = stats(pool);
        let count = s.free();
        if count == 0 {
            return None;
        }
        let start = alloc_frames_contig(pool, count)
            .expect("frames: remaining-pool allocation must succeed (contiguous run)");
        Some((start.pa(), (count as u64) * FRAME_SIZE))
    };

    let (stram_start, stram_len) = claim(Pool::Stram).unwrap_or((0, 0));
    let (ttram_start, ttram_len) = claim(Pool::Ttram).unwrap_or((0, 0));

    // SAFETY: this is the boot CPU; no traps have run yet, so no
    // other code can be inside the BDOS layer. The two ranges come
    // from the frame allocator and are disjoint by construction.
    unsafe {
        tos_api::bdos::mem::init::init(stram_start, stram_len, ttram_start, ttram_len)
            .expect("bdos::mem::init failed (likely AddressOutOfRange — Phase 05 needed)");
    }

    kprintln!(
        "[mm] BDOS pools: ST-RAM 0x{:08x}+{} TT-RAM 0x{:08x}+{}",
        stram_start, stram_len, ttram_start, ttram_len,
    );

    // ARCH03 stage 1: enter both pools into the shared user address space at
    // VA = PA, once, so every process sees every pool address — the TPAs,
    // `Malloc` blocks, environments and resource blobs alike (plan Frage 2,
    // decision E18: boot-time mapping; spike 0b and the Pi run found no
    // aliasing wall).
    // SAFETY: boot CPU, no trap has run; the pools were just seeded.
    match unsafe { tos_api::bdos::process::mmu::map_pools() } {
        Ok(()) => kprintln!(
            "[mm] user address space: pools mapped VA = PA ({} shared L3 tables{})",
            tos_api::bdos::process::mmu::shared_l3_used(),
            if protect { ", 4-KiB pages, rights per process" } else { "" },
        ),
        Err(e) => panic!("[mm] mapping the BDOS pools into the user tables failed: {e:?}"),
    }
}

/// Private L3 tables the arena holds besides the shared ones and the copies
/// for two un-shrunk largest-block TPAs (ARCH03 3b, E9): copies for the
/// 2-MiB regions where a process's rights differ from everybody else's once
/// it has shrunk its TPA — about ten per process (TPA, heap blocks,
/// inherited parent blocks; plan Frage 14), 64 per slot here. 4 MiB.
const PRIVATE_L3_BUDGET: usize = 64 * tos_api::bdos::process::MAX_PROCESSES;

/// The memory-protection switch from the boot arguments (ARCH03 3b):
/// `(on, strict, verify)`. On unless `memprot=off`; strict with
/// `memprot=strict`; the verification mode (MEMPROT-FAST) with
/// `memprot=verify`
/// (`bootargs::MEMPROT_OFF` / `MEMPROT_STRICT`).
fn memprot_switch(dtb: Option<usize>) -> (bool, bool, bool) {
    boot_switch(dtb, (true, false, false), |args: &[u8]| {
        (
            !bootargs::has_token(args, bootargs::MEMPROT_OFF),
            bootargs::has_token(args, bootargs::MEMPROT_STRICT),
            bootargs::has_token(args, bootargs::MEMPROT_VERIFY),
        )
    })
}

/// FS09 block read cache settings, from the boot arguments.
struct BcacheCfg {
    /// Size asked for, KiB (0 = off).
    kib: u32,
    /// Linear-read share, percent.
    pct: u32,
    /// What `bcache=` said.
    kib_arg: bootargs::Setting,
    /// What `bcachepct=` said.
    pct_arg: bootargs::Setting,
}

/// The block read cache settings (FS09, `docs/analysis/fs09-block-cache-
/// 2026-10-07.md` §2.4): `bcache=<KiB>` (0 = off, default
/// [`tos_api::bios::bcache::DEFAULT_KIB`]) and `bcachepct=<0..100>` (default
/// [`tos_api::bios::bcache::DEFAULT_PCT`], FreeMiNT's 5 %). An invalid value
/// keeps the default. `board-qemu-virt` has no block device: no cache.
fn bcache_switch(dtb: Option<usize>) -> BcacheCfg {
    use bootargs::{numeric_setting, Setting, BCACHE, BCACHE_PCT};
    use tos_api::bios::bcache::{DEFAULT_KIB, DEFAULT_PCT, MAX_KIB};
    if !cfg!(feature = "board-rpi4") {
        return BcacheCfg { kib: 0, pct: DEFAULT_PCT, kib_arg: Setting::Default, pct_arg: Setting::Default };
    }
    let (kib_arg, pct_arg) = boot_switch(dtb, (Setting::Default, Setting::Default), |args: &[u8]| {
        (numeric_setting(args, BCACHE, MAX_KIB), numeric_setting(args, BCACHE_PCT, 100))
    });
    let kib = match kib_arg {
        Setting::Given(v) => v,
        Setting::Default | Setting::Invalid => DEFAULT_KIB,
    };
    let pct = match pct_arg {
        Setting::Given(v) => v,
        Setting::Default | Setting::Invalid => DEFAULT_PCT,
    };
    BcacheCfg { kib, pct, kib_arg, pct_arg }
}

/// Hand the reserved arena to the block cache and log one `[bcache]` line.
fn install_block_cache(map: &bios::machine::layout::MemoryMap, cfg: &BcacheCfg) {
    use bootargs::Setting;
    let src = |s: Setting| match s {
        Setting::Default => "default",
        Setting::Given(_) => "boot argument",
        Setting::Invalid => "INVALID boot argument ignored, default",
    };
    if !cfg!(feature = "board-rpi4") {
        kprintln!("[bcache] FS09 block read cache: none (no block device on this board)");
        return;
    }
    let r = map.block_cache;
    if r.is_empty() {
        kprintln!("[bcache] FS09 block read cache: OFF (bcache=0, {})", src(cfg.kib_arg));
        return;
    }
    let va = (r.start + arch::aarch64::mmu::VA_OFFSET) as *mut u8;
    // SAFETY: `discover` reserved `r` for the cache alone, outside both pools
    // and the other OS buffers; the high-half alias is EL1-only Normal memory
    // (the reserve stays inside `map.dram`). Nothing else ever touches it.
    match unsafe { tos_api::bios::bcache::install(va, r.len() as usize, cfg.pct) } {
        Some(s) => kprintln!(
            "[bcache] FS09 block read cache: {} KiB at 0x{:x} (asked {} KiB, {}), {} sectors of 512 B, linear-read share {} % ({}), write-through",
            r.len() / 1024,
            r.start,
            cfg.kib,
            src(cfg.kib_arg),
            s.slots,
            cfg.pct,
            src(cfg.pct_arg),
        ),
        None => kprintln!("[bcache] FS09 block read cache: reserve of {} bytes unusable, cache OFF", r.len()),
    }
}

/// Per-process ASIDs from the boot arguments (ARCH03 stage 4): `None` = off
/// (the default, the TLBI-on-every-switch path); `Some(false)` = on
/// (`asid=on`); `Some(true)` = on with four numbers per generation
/// (`asid=tiny`, the rollover test hook).
fn asid_switch(dtb: Option<usize>) -> Option<bool> {
    boot_switch(dtb, None, |args: &[u8]| {
        if bootargs::has_token(args, bootargs::ASID_TINY) {
            Some(true)
        } else if bootargs::has_token(args, bootargs::ASID_ON) {
            Some(false)
        } else {
            None
        }
    })
}

/// FS08 part 3: set the ARM core clock to the highest rate the firmware
/// supports, and log one `[clock]` boot line.
///
/// Protocol (`docs/hardware/mailbox-framebuffer.md` §3f, firmware wiki
/// *Mailbox property interface*): "Get max clock rate" for `CLOCK_ID_ARM`
/// (*"Clocks should not be set higher than this"*), then "Set clock rate" to
/// exactly that rate with `skip setting turbo = 0`, so the firmware raises
/// the turbo settings that go with an ARM rate above default (*"e.g.
/// voltage"*) itself — the kernel sets no voltage. Overheat protection stays
/// the firmware's (`temp_limit`, `references/rpi4-boot/
/// config-txt-overclocking.adoc`). Read back: "Get clock rate" (the rate now
/// requested) and "Get clock rate measured" (the rate actually running).
///
/// Any unanswered step leaves the firmware rate as it is and says so in the
/// line. `cpuclk=firmware` in the boot arguments skips the change.
#[cfg(feature = "board-rpi4")]
fn raise_arm_clock(dtb: Option<usize>) {
    use drivers::bcm2711::clock_tags::{deci_celsius, mhz};
    use drivers::bcm2711::mailbox::{self, CLOCK_ID_ARM, TEMPERATURE_ID};

    /// A rate in MHz, `?` when the firmware gave none.
    struct Mhz(Option<u32>);
    impl core::fmt::Display for Mhz {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self.0 {
                Some(hz) => write!(f, "{}", mhz(hz)),
                None => f.write_str("?"),
            }
        }
    }
    /// The SoC temperature, `?` when the firmware gave none.
    struct Soc(Option<u32>);
    impl core::fmt::Display for Soc {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self.0 {
                Some(mdeg) => {
                    let (c, tenth) = deci_celsius(mdeg);
                    write!(f, "SoC {}.{} C", c, tenth)
                }
                None => f.write_str("SoC ? C"),
            }
        }
    }

    let firmware_only = boot_switch(dtb, false, |args: &[u8]| {
        bootargs::has_token(args, bootargs::CPUCLK_FIRMWARE)
    });
    let before = mailbox::get_clock_rate(CLOCK_ID_ARM);
    let temp = Soc(mailbox::get_temperature(TEMPERATURE_ID));
    use tos_api::host::CpuClockPolicy;
    if firmware_only {
        host_env::record_cpu_clock_policy(CpuClockPolicy::Firmware);
        kprintln!(
            "[clock] ARM {} MHz kept: boot arguments contain `cpuclk=firmware` ({})",
            Mhz(before),
            temp
        );
        return;
    }
    let Some(max) = mailbox::get_max_clock_rate(CLOCK_ID_ARM) else {
        host_env::record_cpu_clock_policy(CpuClockPolicy::MaxRefused);
        kprintln!("[clock] ARM {} MHz kept: firmware reports no max rate ({})", Mhz(before), temp);
        return;
    };
    if mailbox::set_clock_rate(CLOCK_ID_ARM, max, false).is_none() {
        host_env::record_cpu_clock_policy(CpuClockPolicy::MaxRefused);
        kprintln!(
            "[clock] ARM {} MHz kept: Set clock rate not answered (max {}, {})",
            Mhz(before),
            Mhz(Some(max)),
            temp
        );
        return;
    }
    host_env::record_cpu_clock_policy(CpuClockPolicy::FixedMax);
    let after = mailbox::get_clock_rate(CLOCK_ID_ARM);
    let measured = mailbox::get_clock_rate_measured(CLOCK_ID_ARM);
    kprintln!(
        "[clock] ARM {} -> {} MHz (max {}, measured {}, {})",
        Mhz(before),
        Mhz(after),
        Mhz(Some(max)),
        Mhz(measured),
        temp
    );
}

/// Read the boot arguments once and `decide` on them; `default` when there
/// are none. `board-rpi4` asks the firmware for its command line
/// (`cmdline.txt`, or QEMU's `-append`), QEMU `virt` reads `/chosen/bootargs`
/// of its device tree. Boot CPU, while the boot tables still reach the DTB.
fn boot_switch<R>(dtb: Option<usize>, default: R, decide: impl FnOnce(&[u8]) -> R) -> R {
    #[cfg(feature = "board-rpi4")]
    {
        let _ = dtb;
        drivers::bcm2711::mailbox::with_command_line(decide).unwrap_or(default)
    }
    #[cfg(not(feature = "board-rpi4"))]
    {
        let mut buf = [0u8; 256];
        // SAFETY: the boot loader supplied the DTB pointer; still reachable
        // through the boot tables at this point of the boot.
        match dtb.and_then(|d| unsafe { arch::aarch64::dtb::copy_bootargs(d, &mut buf) }) {
            Some(n) => decide(&buf[..n]),
            None => default,
        }
    }
}

/// Phase 04 D13 — dump BDOS pool accounting on the boot console.
///
/// Calls [`tos_api::bdos::mem::stats::pool_stats`] for both pools
/// and prints a one-line snapshot per pool. Cheap O(n) walk over
/// `mp_mfl` + `mp_mal`; safe to call any time after
/// `bring_up_bdos_pools` has run.
fn print_bdos_pool_stats(label: &str) {
    use tos_api::bdos::mem::stats::{pool_stats, PoolKind};
    for (name, kind) in [("ST-RAM", PoolKind::Stram), ("TT-RAM", PoolKind::Ttram)] {
        let s = pool_stats(kind);
        kprintln!(
            "[mm] BDOS {} {}: total={} free={} used={} largest_free={} (free_mds={} alloc_mds={})",
            name, label,
            s.total_bytes(), s.free_bytes, s.used_bytes, s.largest_free,
            s.free_blocks, s.allocated_blocks,
        );
    }
    print_mem05_stats(label);
}

/// MEM05: one line with the MD supply, the page budget and the kernel heap
/// (boot, and after every program the shell ran). Counters only; no list
/// walk.
pub(crate) fn print_mem05_stats(label: &str) {
    use tos_api::bdos::mem::{mdpool, pages};
    let md = mdpool::stats();
    let pg = pages::stats();
    let t = tos_api::bdos::process::memprot::tables();
    let h = bios::machine::heap::stats();
    kprintln!(
        "[mm] MEM05 {}: MDs in_use={} capacity={} (+{} pages); pages held={}/{} (md={} recs={} snap={} kept={}) peak={} refused budget={} heap={}; records={} snapshot_cap={} snapshot_short={}; heap used={} free={} largest={}",
        label,
        md.in_use, md.capacity, md.pages,
        pg.held, pg.budget, pg.md, pg.recs, pg.snap, pg.kept, pg.peak,
        pg.refused_budget, pg.refused_source,
        t.records, t.snapshot_capacity, t.snapshot_shortfalls,
        h.used, h.free(), h.largest_free,
    );
}

/// Seed the wall clock from `C:\KERNEL8.IMG`'s FAT directory timestamp at
/// boot — the HW-adapted boot clock-init for a machine with no RTC.
///
/// NewTOS has no battery-backed RTC. As documented in
/// `tos-api/src/gemdos/time.rs`, the wall clock is a settable `BASE_SECS`
/// epoch plus the ARM generic-timer monotonic delta; the default base of
/// `0` boots the system reading 1980-01-01, so every OS-created file
/// (logs) is stamped 1980-01-01 and builds are indistinguishable.
///
/// EmuTOS seeds its boot clock from the NVRAM / IKBD RTC. We have neither,
/// so — until an SPI RTC lands — we use the FAT date/time of the running
/// kernel image as a coarse stopgap source: it tells you *which kernel
/// build/flash produced a file*. The TOS API (`Tgetdate`/`Tgettime`/
/// `Settime`) is unchanged; only the clock *source* is HW-adapted, and it
/// will be swapped for the SPI RTC without touching the API.
///
/// The FAT directory date/time WORDs are bit-identical to the GEMDOS
/// packed encoding, so the pair feeds straight into `xbios_settime`
/// (`packed = (date << 16) | time`) with no conversion. A missing file or
/// a zero FAT date (FAT `0` = "no date") leaves the 1980-01-01 default
/// untouched. This never panics and never blocks boot.
#[cfg(feature = "board-rpi4")]
fn seed_clock_from_kernel_file() {
    use tos_api::bdos::fs::find::{list_dir_ex, DirInfo};

    // `list_dir_ex` upper-cases the mask into the 11-byte FCB pattern
    // `KERNEL8 IMG`, matching the FAT 8.3 short name of the boot image.
    let mut ents = [DirInfo::default(); 1];
    let n = list_dir_ex("C:\\KERNEL8.IMG", &mut ents);
    if n == 0 || ents[0].date == 0 {
        kprintln!("[clock] no kernel8.img FAT date on C: — clock stays at 1980-01-01");
        return;
    }

    let (date, time) = (ents[0].date, ents[0].time);
    let packed = ((date as u32) << 16) | time as u32;
    tos_api::gemdos::time::xbios_settime(packed);

    let year = 1980 + (date >> 9);
    let month = (date >> 5) & 0x0F;
    let day = date & 0x1F;
    let hour = time >> 11;
    let min = (time >> 5) & 0x3F;
    let sec = (time & 0x1F) * 2;
    kprintln!(
        "[clock] seeded from kernel8.img: {year:04}-{month:02}-{day:02} {hour:02}:{min:02}:{sec:02}"
    );
}
