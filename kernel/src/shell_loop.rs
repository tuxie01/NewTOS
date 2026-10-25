//! The AES shell loop — EmuTOS `sh_main` / `sh_ldapp` (`aes/gemshlib.c`).
//!
//! Spec: `docs/api/aes/shell-loop.md` §7 (`sh_main`), §8 (`sh_ldapp`), §9
//! (`sh_chdef`/`sh_chgrf`/`sh_tographic`/`sh_toalpha`).
//! Stage **S3** of `docs/analysis/desktop-userspace-plan.md`.
//!
//! This is what makes the machine boot into *whichever* program the
//! configuration names instead of a program compiled into the kernel: the
//! `.INF` file's `#Z` line is read by `process_inf2`
//! ([`tos_api::aes::init::process_inf2`], EmuTOS `geminit.c:407-464`), split
//! into a (program, directory) pair by `sh_wdef`, recombined into a
//! fully-qualified command by `sh_chdef`, and launched here.
//!
//! **Layering.** EmuTOS puts the state, the policy *and* the loop in one file
//! (`aes/gemshlib.c`) because there the AES **is** the kernel. In NewTOS the
//! state and the policy live in [`tos_api::aes::shell`] (they are pure AES
//! bookkeeping and GEMDOS calls) and only the loop plus the actual program
//! start live here, because dropping to EL0
//! ([`crate::launch::run_program_with_tail_rc`]) is architecture-specific
//! kernel code. No behaviour is split — every function below is one EmuTOS
//! function, cited.
//!
//! **The one deliberate deviation, and where it is.** EmuTOS decides at
//! *build* time which user interface the machine has: `endrom.c:28-35` sets
//! `exec_os` to the AES entry `#if WITH_AES`, to EmuCON `#elif WITH_CLI`, and
//! `#error You must provide a main UI` otherwise. On an Atari that is a
//! build-time fact because every Atari has a screen. On our boards it is a
//! *runtime* fact (a board may come up without a raster backend), so the same
//! choice is made at runtime, in exactly one place: [`sh_ldapp`]'s
//! `DESKTOP_APP` arm. That is the hardware-layer carve-out of
//! `docs/COMPATIBILITY.md`; everything else follows `gemshlib.c` literally.

use core::sync::atomic::{AtomicBool, AtomicI32, Ordering};

use tos_api::aes::shell::{
    self, ShExit, AUTORUN_APP, CONSOLE_APP, DESKTOP_APP, NORMAL_APP,
};
use tos_api::bdos::process::sched::running_slot;
use tos_api::bdos::process::{current_abi, Abi};
use tos_api::trap::TrapArgs;
use tos_api::uaccess::Space;

use crate::kprintln;

// ── "desktop up" — the kernel's observation of the desktop (DESK43 D4) ──────
//
// The desktop prints nothing (EmuTOS leaves `KDEBUG` compiled out of every
// `desk/*.c`, `deskact.c:19`, `deskapp.c:20`; user decision D4, 2026-10-14).
// What the harnesses need from it — "the desktop is ready for input" — is
// observed here instead, from outside, with the criterion the desktop's own
// line used to have: the desktop has finished `deskmain`'s prolog and issues
// its first `evnt_multi` (`deskmain.c:2077-2123`, the event loop). The test
// runs in the GEM-trap path, so it sees exactly the calls the AES sees.
//
// **Which caller is the desktop** ([`is_desktop_caller`], DESK43 U9-0c): the
// program the DESKTOP arm started, while [`DESKTOP_ACTIVE`]. The arm starts the
// EL0 program `NEWDESK` through the launcher like any program
// (`aes_run_rom_program`, `gemshlib.c:606`; DESK43 U9-4), so it is the shell's
// child ([`crate::launch::shell_child_slot`]), and its GEM traps are the ones
// from user space whose running slot (`RUN`, [`running_slot`]) is that child
// slot. Accessories, and programs the desktop itself `Pexec`s, have other
// slots; an EL1 (`Space::Kernel`) caller is never the desktop — since DESK43
// U10 there is no desktop inside the kernel.
//
// **Once per `deskmain` run**, like the old line: `appl_init` (the first call
// of every run, `deskmain.c:1953`) arms the report, the first `evnt_multi`
// fires and disarms it. A *Read .INF file* restart runs `deskmain` again and
// reports again; an `evnt_multi` re-executed after an S2 yield (ACC03) does
// not report twice.

/// Set while the DESKTOP arm of [`sh_ldapp`] runs the desktop program
/// `NEWDESK`.
static DESKTOP_ACTIVE: AtomicBool = AtomicBool::new(false);
/// Armed by the desktop's `appl_init`, consumed by its next `evnt_multi`.
static DESKTOP_UP_ARMED: AtomicBool = AtomicBool::new(false);

/// AES opcode `APPL_INIT` (`include/aesdefs.h:17`).
const AES_APPL_INIT: u16 = 10;
/// AES opcode `FORM_ERROR` (`include/aesdefs.h:62`).
const AES_FORM_ERROR: i16 = 53;
/// AES opcode `EVNT_MULTI` (`include/aesdefs.h:32`).
const AES_EVNT_MULTI: u16 = 25;
/// GEM-trap selector of the AES (`x0` = 200, `docs/ARCHITECTURE.md`).
const GEM_SEL_AES: u64 = 200;

/// `true` when the GEM trap `args` comes from the desktop the DESKTOP arm
/// started — the launched shell child (see the block comment above). Only
/// meaningful while [`DESKTOP_ACTIVE`].
fn is_desktop_caller(args: &TrapArgs) -> bool {
    crate::launch::shell_child_slot()
        .is_some_and(|child| args.space == Space::User && running_slot() == child)
}

/// Called by the `svc` handler for every GEM trap (`svc #2`), before the
/// dispatch: report `[desk] desktop up` when the desktop reaches its first
/// `evnt_multi` of a `deskmain` run (see the block comment above).
///
/// A no-op unless the desktop is running. It reads only `block[0]` and
/// `control[0]` of the desktop's own parameter block — words the AES
/// dispatcher reads a few instructions later (`Aespb::from_raw`: `block[0..6]`,
/// `control[0..5]`). They are read with the program's rights
/// (`tos_api::uaccess`): a bad pointer faults here and is recorded as the
/// call's fault, and the dispatcher's own read of the same words fails the
/// same way and ends the program as before — the observation cannot change
/// what happens to the program, only which of the two reads is reported.
pub fn observe_desktop_gem_call(args: &TrapArgs) {
    if !DESKTOP_ACTIVE.load(Ordering::Acquire)
        || args.regs[0] != GEM_SEL_AES
        || !is_desktop_caller(args)
    {
        return;
    }
    let block = args.regs[1];
    if block == 0 {
        return;
    }
    // `control` = `block[0]`, a pointer of the caller's ABI width.
    let control = match current_abi() {
        Abi::Lp64 => args.space.get_u64(block),
        Abi::Ilp32 => args.space.get_u32(block).map(u64::from),
    };
    let Ok(control) = control else { return };
    if control == 0 {
        return;
    }
    let Ok(op) = args.space.get_u16(control) else { return };
    match op {
        AES_APPL_INIT => DESKTOP_UP_ARMED.store(true, Ordering::Release),
        AES_EVNT_MULTI if DESKTOP_UP_ARMED.swap(false, Ordering::AcqRel) => {
            // The drive count the desktop's line carried
            // (`dos_sdrv(dos_gdrv())` = `Dsetdrv`'s return = the BIOS drive
            // map). The old "M window(s) from the context" has no source
            // outside the desktop and is gone; the windows are on the screen.
            kprintln!(
                "[desk] desktop up ({} drives)",
                tos_api::bios::drive::drvbits().count_ones()
            );
        }
        _ => {}
    }
}

// ── A desktop program that cannot be loaded (DESK43 D6-c/D6-d) ────────────
//
// EmuTOS has one desktop, started as a process (`aes_run_rom_program
// (deskstart)`, `gemshlib.c:606`); a ROM entry point cannot fail to load.
// NewTOS's is the program `NEWDESK`, embedded in the kernel image under that
// registry name (`crate::embedded`, D3) and started the same way — and a
// program load can fail. When it does, the DESKTOP arm does not try again
// this boot (D6-c: the same blob would fail the same way) and runs the
// console instead (D6-d, after DESK43 U10 removed the kernel's own copy of
// the desktop): EmuCON, the other main UI EmuTOS provides
// (`endrom.c:28-35`).

/// The negative load error of the desktop program, kept for the rest of the
/// boot once its start has failed; 0 while it has not.
static DESKTOP_LOAD_ERROR: AtomicI32 = AtomicI32::new(0);

/// `LEN_ZPATH + LEN_ZFNAME + 1` = `MAXPATHLEN` (`include/sysconf.h:51-55`) —
/// the size of `D.s_cmd`.
const MAXPATHLEN: usize = 128;

/// `fm_error(n)` for the shell loop (`gemshlib.c:722-723`), issued as the AES
/// call `form_error` through the GEM trap (`svc #2`, selector 200) from this
/// EL1 context.
///
/// `fm_error` reaches `fm_show` → `fm_alert`, whose `wm_update(BEG_UPDATE)`
/// (`aes/gemfmalt.c:361`) waits while another process — a desk accessory —
/// holds the window-tree lock (`tak_flag` → `ev_block(MU_MUTEX, …)`,
/// `aes/gemwmlib.c:1785-1786`). A direct call from here could not wait: the
/// shell loop is no process the scheduler can switch away from. Through the
/// trap it is the EL1 kernel client every wait already knows (ACC03): the
/// dispatcher's gate parks it (`window::update_block`), the trap handler
/// parks this frame with the `svc` rewound, the holder runs, and the call is
/// re-executed once the lock is free or handed over (A118). The return value
/// is `sh_main`'s to ignore, as there (`:723`).
fn fm_error(n: i16) {
    let mut control = [0i16; 5];
    control[0] = AES_FORM_ERROR;
    control[1] = 1; // IN_LEN
    control[2] = 1; // OUT_LEN
    let mut global = [0i16; 16];
    let mut intin = [0i16; 16];
    intin[0] = n;
    let mut intout = [0i16; 16];
    let mut addrin = [0u64; 4];
    let mut addrout = [0u64; 4];
    // The LP64 parameter block — the layout the trap handler reads for a
    // caller without a process (`current_abi()`; the program `sh_ldapp` ran
    // has ended, so there is none).
    let block: [u64; 6] = [
        control.as_mut_ptr() as u64,
        global.as_mut_ptr() as u64,
        intin.as_mut_ptr() as u64,
        intout.as_mut_ptr() as u64,
        addrin.as_mut_ptr() as u64,
        addrout.as_mut_ptr() as u64,
    ];
    debug_assert_eq!(current_abi(), Abi::Lp64, "the shell loop runs with no process");
    // SAFETY: the GEM trap from EL1 with selector 200 and an LP64 AES
    // parameter block whose six arrays live on this stack frame until the
    // call has returned; the handler restores every register except `x0`.
    unsafe {
        core::arch::asm!(
            "svc #2",
            inlateout("x0") GEM_SEL_AES => _,
            in("x1") block.as_ptr() as u64,
        );
    }
}

/// `sh_main(isauto, isgem)` (`gemshlib.c:692-731`) — the AES shell loop.
///
/// ```c
/// psh->sh_doexec  = SHW_EXEC;
/// psh->sh_nextapp = isauto ? AUTORUN_APP : DESKTOP_APP;
/// psh->sh_isgem   = isgem;
/// strcpy(sh_apdir, D.s_cdir);
/// gl_shgem = TRUE;
/// if (psh->sh_nextapp == DESKTOP_APP) set_default_desktop(psh);
/// do {
///     sh_chdef(psh);
///     sh_chgrf(psh);
///     if (gl_shgem) { wm_init(); ratinit(); sh_draw(D.s_cmd, TRUE); }
///     if (rc && !IS_BIOS_ERROR(rc)) fm_error(-rc-31);
///     rc = sh_ldapp(psh);
/// } while ((psh->sh_doexec != SHW_SHUTDOWN) && !gl_changerez);
/// if (gl_shgem) sh_toalpha();
/// ```
///
/// **Exit condition — both terms (A106).** `gl_changerez` has two producers
/// in EmuTOS. `process_inf1`'s boot-time `#E` parse (`geminit.c:343-392`)
/// goes through `check_moderez`, which reports no change on a board with
/// exactly one fixed mode (ADR-017 rev 2, D-101) — that producer is
/// legitimately FALSE here. `sh_write`'s `SHW_RESCHNG` arm
/// (`gemshlib.c:187-191`) is pure AES software state, set unconditionally by
/// `shel_write(SHW_RESCHNG, …)` (A48), and *that* is what ends this loop:
/// when the running program terminates with the request pending, the next
/// program is not started ([`tos_api::aes::shell::sh_main_continue`],
/// `gemshlib.c:727`) and the caller gets [`ShExit::RezChange`].
///
/// Returning instead of halting is the caller's job: the kernel's session
/// loop (`kernel/src/lib.rs`) is NewTOS's `gemstart.S:86-124` `aes_restart`
/// — it re-initialises the AES ([`tos_api::aes::reinit`]) and enters
/// `sh_main` again, and only [`ShExit::Shutdown`] falls through to `halt()`.
pub fn sh_main(isauto: bool, isgem: bool) -> ShExit {
    // `gemshlib.c:698-706` — seed the SHELL and, without an autorun entry,
    // make the desktop the first program.
    shell::sh_main_init(isauto, isgem);

    let mut rc: i32 = 0;
    loop {
        shell::sh_chdef(); // :712
        sh_chgrf(); // :713

        if shell::gl_shgem() {
            // :715-720
            tos_api::aes::window::wm_init();
            ratinit();
            shell::sh_draw_clear();
        }

        // Alert for the *previous* iteration's error (`:722-723`). GEMDOS
        // codes (`rc <= EINVFN`) become a positive DOS error number for
        // `fm_error`; BIOS-level codes are suppressed.
        if let Some(n) = shell::sh_main_alert(rc) {
            fm_error(n);
        }

        rc = sh_ldapp(); // :725

        // `while ((psh->sh_doexec != SHW_SHUTDOWN) && !gl_changerez)` (:727)
        if !shell::sh_main_continue() {
            break;
        }
    }

    if shell::gl_shgem() {
        sh_toalpha(); // :733
    }

    let exit = shell::sh_main_exit();
    kprintln!(
        "[shell] sh_main: session over — {}",
        match exit {
            ShExit::Shutdown => "shutdown",
            ShExit::RezChange =>
                "shel_write(SHW_RESCHNG) pending, restarting the AES",
        }
    );
    exit
}

/// `sh_chgrf` (`gemshlib.c:513-523`) — switch the screen between graphics
/// (GEM) and alpha (character) mode when the program about to run needs the
/// other one.
///
/// ```c
/// if (psh->sh_isgem != gl_shgem) {
///     gl_shgem = psh->sh_isgem;
///     if (gl_shgem) sh_tographic(); else sh_toalpha();
/// }
/// ```
///
/// **Hardware-layer clamp:** `sh_tographic` cannot succeed without a raster
/// backend, so the desired mode is `sh_isgem && have_graphics()`. On every
/// board that has a screen this is exactly `sh_isgem` and the function is
/// EmuTOS's verbatim.
fn sh_chgrf() {
    let want = shell::sh_isgem() && have_graphics();
    if want != shell::gl_shgem() {
        shell::set_gl_shgem(want);
        if want {
            sh_tographic();
        } else {
            sh_toalpha();
        }
    }
}

/// Whether this machine has a screen — the runtime form of EmuTOS's
/// build-time `WITH_AES` (`bios/endrom.c:28-35`). See the module note.
fn have_graphics() -> bool {
    tos_api::host::host().and_then(|h| h.raster()).is_some()
}

/// `sh_tographic` (`gemshlib.c:223-239`) — convert the screen to graphics
/// mode for a GEM application: retake the vectors a character application may
/// have stepped on, open the graphics workstation, set the initial clip
/// rectangle, allocate screen space, start the mouse and show the hourglass.
///
/// `gsx_graphic(TRUE)` (:234) switches the VT52 console out of the way —
/// `v_exit_cur`, text cursor off — when the screen was in alpha mode
/// ([`tos_api::aes::gsxif::gsx_graphic`]). `gsx_malloc()` (:236) has a
/// counterpart too: the AES menu/alert screen buffer
/// ([`tos_api::aes::gsxif`]), taken from the BDOS pool the first time this
/// runs — before the first GEM program, so the block is kernel-owned and
/// survives every `Pterm` — and kept: `sh_toalpha` does **not** free it
/// (EmuTOS `gsx_mfree`, `:258`), so its address stays stable across an alpha
/// interlude (unobservable superset; `gsxif` module docs,
/// `docs/COMPATIBILITY.md`). What else is observable — an active mouse and
/// the hourglass shape — is done here. `retake()` restores the critical-error
/// handler and the AES trap vector, neither of which a NewTOS program can
/// displace (the trap vector is the exception table, not a writable
/// low-memory long).
///
/// Going graphic does **not** silence the character console: EmuTOS'
/// `bconout2` writes into screen memory whichever mode the AES thinks it is
/// in, so a program that prints here paints over the GEM screen, on TOS as on
/// NewTOS. See [`crate::fbcon`].
fn sh_tographic() {
    tos_api::aes::gsxif::gsx_graphic(true); // `gsx_graphic(TRUE)` (:234)
    ratinit(); // `ratinit()` (:236)
    // `gsx_malloc()` (:236) — once; a later pass finds the buffer in place.
    let had_buffer = tos_api::aes::gsxif::gsx_mret().is_some();
    match tos_api::aes::gsxif::gsx_malloc() {
        Some(b) if !had_buffer => kprintln!(
            "[aes] gsx_malloc: screen buffer pa=0x{:08x} len={} (kernel-owned, WF_SCREEN)",
            b.pa, b.len,
        ),
        Some(_) => {}
        None => kprintln!("[aes] gsx_malloc: no screen buffer (pool short) -- WF_SCREEN answers FALSE"),
    }
    if let Some(b) = tos_api::host::host().and_then(|h| h.raster()) {
        // `set_mouse_to_hourglass()` (:237) — AES preset shape 2.
        tos_api::vdi::mouse::set_preset(2, b);
    }
}

/// `sh_toalpha` (`gemshlib.c:246-260`) — convert the screen back to alpha
/// mode for a character-mode program: arrow cursor, give back the error
/// handler, turn the mouse off, free screen space, close the workstation.
///
/// The observable half here is the arrow shape, the mouse going away
/// (`ratexit`) and the VT52 text console taking the screen: `gsx_graphic
/// (FALSE)` (:259) is the VDI `v_enter_cur` — clear, home, text cursor on.
fn sh_toalpha() {
    if let Some(b) = tos_api::host::host().and_then(|h| h.raster()) {
        // `set_mouse_to_arrow()` (:248) — AES preset shape 0.
        tos_api::vdi::mouse::set_preset(0, b);
        // `ratexit()` (:256) — `gsx_moff()`, hide the cursor.
        tos_api::vdi::mouse::hide(b);
    }
    tos_api::aes::gsxif::gsx_graphic(false); // `gsx_graphic(FALSE)` (:259)
}

/// `ratinit` (`aes/gemgsxif.c:205-209`) — `gsx_1code(SHOW_CUR, 0); gl_moff = 0;`
/// i.e. show the mouse cursor at its current position. The `0` is `v_show_c`'s
/// `reset` argument, which forces the cursor visible regardless of how many
/// `v_hide_c` calls preceded it — [`tos_api::vdi::cursor_show_reset`], not the
/// counted [`tos_api::vdi::cursor_show`]. It does **not** move the pointer; the
/// VDI centres it once when the workstation opens (`vdi/vdi_mouse.c:582`,
/// `GCURX = xres / 2`).
fn ratinit() {
    if let Some(b) = tos_api::host::host().and_then(|h| h.raster()) {
        tos_api::vdi::cursor_show_reset(b);
    }
}

/// `run_accs_and_desktop`'s `sh_tographic()` (`aes/geminit.c:700`, "go into
/// graphic mode") — the one-time switch out of the character screen the BIOS
/// and the `\AUTO\` programs used, done **before** `process_inf2` and
/// `sh_main`. `sh_main` then records the resulting state as `gl_shgem = TRUE`
/// (`gemshlib.c:703`).
///
/// A machine without a raster backend cannot make this switch; the loop's
/// first `sh_chgrf` notices (`gl_shgem` true, desired false) and drops back to
/// alpha mode, which routes the boot into the console arm — see the module
/// note.
pub fn go_graphic() {
    if have_graphics() {
        sh_tographic();
    }
}

/// `sh_ldapp(psh)` (`gemshlib.c:592-689`) — run the next program and block
/// until it terminates. Three top-level arms, four `sh_nextapp` values.
///
/// Returns the GEMDOS code `sh_main` may turn into an alert: `0` on success,
/// the child's negative `dos_exec` result, or `EFILNF` when `sh_find` could
/// not locate a [`NORMAL_APP`].
fn sh_ldapp() -> i32 {
    let mut cmd = [0u8; MAXPATHLEN];
    shell::s_cmd(&mut cmd);
    let nextapp = shell::sh_nextapp();

    // ── Arm 1 — DESKTOP_APP (`:600-612`) ─────────────────────────────────
    if nextapp == DESKTOP_APP {
        // EmuTOS's `KDEBUG` here prints `D.s_cmd` (`:597-598`), but the arm
        // does not start what `D.s_cmd` names: it starts *the* desktop
        // (`deskstart`, `:606`), and `D.s_cmd` only names its descriptor
        // (`p_nameit`, `:604`). The two differ after a `#Z` program that
        // called `shel_write(SHW_NOEXEC)` (TeraDesk does, after each program
        // it `Pexec`s, `startprg.c:386`): `sh_nextapp` is then `DESKTOP_APP`,
        // so the AUTORUN return (`:662-666`) skips `set_default_desktop` and
        // `sh_chdef` rebuilds `D.s_cmd` from the `#Z` program's `sh_desk`
        // (`:551-565`) — e.g. `DESKTOP.NAP` after TeraDesk; EmuTOS behaves
        // identically (DESK60). So the line names the desktop that runs and
        // shows `D.s_cmd` separately.
        kprintln!(
            "[shell] sh_ldapp: starting desktop {} (D.s_cmd {}), sh_nextapp={}, sh_isgem={}",
            desktop_label(),
            as_str(&cmd),
            nextapp,
            i32::from(shell::sh_isgem()),
        );
        if !shell::gl_shgem() {
            // The machine has no screen, so the desktop is not the main UI
            // this boot — `endrom.c:28-35`'s choice, made at runtime. Fall
            // through to the console arm, which is the other main UI EmuTOS
            // provides (`endrom.c:33`, `coma_start`).
            report_no_desktop();
            return run_console_arm();
        }
        let load_error = DESKTOP_LOAD_ERROR.load(Ordering::Acquire);
        if load_error != 0 {
            // NEWTOS: D6-c — the desktop program failed to load earlier this
            // boot; it is not loaded again (see the block comment above).
            kprintln!(
                "[shell] desktop {} not started: its load failed earlier this boot (rc={}) \
                 — the console (EmuCON) instead",
                as_str(shell::DEF_DESKTOP),
                load_error
            );
            return run_console_instead_of_desktop(load_error);
        }
        shell::sh_show(&[]); // `sh_show("")` — no name for the desktop (:603)
        p_setappdir(&cmd); // (:605)
        // `if (aes_run_rom_program(deskstart)) wm_new();` (:606-610) — the
        // desktop as a fresh process every time: the embedded `NEWDESK`,
        // started without a file and without the hourglass
        // ([`crate::launch::run_desktop_program`], which also does the
        // `p_nameit(rlr, fname)` of `:604`). `sh_nextapp` stays `DESKTOP_APP`,
        // so whatever ends it — `Pterm(0)` after a `shel_write`, a crash's
        // `Pterm(-1)` after the report's key (`bios/kprint.c:581-594`), a Rust
        // panic after its key (D6-a) — the next pass starts it again. No loop
        // guard (D6-b), no alert: the arm returns 0 (`:611`).
        DESKTOP_ACTIVE.store(true, Ordering::Release);
        let rc = crate::launch::run_desktop_program(shell::DEF_DESKTOP, &cmd);
        DESKTOP_ACTIVE.store(false, Ordering::Release);
        DESKTOP_UP_ARMED.store(false, Ordering::Release);
        if rc != 0 {
            // "ROM desktop terminated abnormally" (:607-609): any value other
            // than 0 — `Pterm(-1)`'s 65535, any other exit code, a load error.
            tos_api::aes::window::wm_new();
        }
        if rc < 0 {
            // NEWTOS: D6-c/D6-d. The desktop program never ran (a load error
            // — no EmuTOS counterpart: a ROM entry point cannot fail to load).
            // Loading the same blob again would fail again, so it is not
            // retried this boot; the console takes over (see the block
            // comment above).
            DESKTOP_LOAD_ERROR.store(rc, Ordering::Release);
            kprintln!(
                "[shell] desktop {} could not be loaded (rc={}) — not retried; \
                 falling back to the console (EmuCON)",
                as_str(shell::DEF_DESKTOP),
                rc
            );
            return run_console_instead_of_desktop(rc);
        }
        kprintln!(
            "[shell] sh_ldapp: desktop {} (D.s_cmd {}) exited with rc={}",
            as_str(shell::DEF_DESKTOP),
            as_str(&cmd),
            rc
        );
        return 0;
    }

    kprintln!(
        "[shell] sh_ldapp: starting {}, sh_nextapp={}, sh_isgem={}",
        as_str(&cmd),
        nextapp,
        i32::from(shell::sh_isgem()),
    );

    // ── Arm 2 — CONSOLE_APP (`:614-622`, `#if WITH_CLI`) ─────────────────
    if nextapp == CONSOLE_APP {
        return run_console_arm();
    }

    // ── Arm 3 — NORMAL_APP / AUTORUN_APP (`:624-689`) ────────────────────
    //
    // `ret = (psh->sh_nextapp == AUTORUN_APP) ? 1 : sh_find(D.s_cmd);` (:629)
    // — an autorun entry is trusted, a normal application is searched for.
    let found = if nextapp == AUTORUN_APP {
        true
    } else {
        let ok = shell::sh_find(&mut cmd);
        if ok {
            // `sh_find` rewrites `D.s_cmd` in place when it qualifies the name.
            shell::set_s_cmd(&cmd);
        }
        ok
    };

    if !found {
        // `set_default_desktop(psh); return EFILNF;` (:687-688)
        kprintln!("[shell] sh_find: '{}' not found", as_str(&cmd));
        shell::set_default_desktop();
        return tos_types::error::EFILNF;
    }

    let fname = shell::sh_name(&cmd[..clen(&cmd)]);
    shell::sh_show(fname); // (:632)
    p_nameit_and_appdir(&cmd); // (:633-635)

    if nextapp == NORMAL_APP {
        // "by default, run the desktop after a normal application" — set
        // *before* the child runs, so a `shel_write` from inside it wins
        // (`:639-643`).
        shell::set_sh_nextapp(DESKTOP_APP);
        shell::set_sh_isgem(true);
    }

    // `ret = dos_exec(PE_LOADGO, D.s_cmd, ad_stail, ad_envrn);` (:645).
    // `ad_stail` is already the Atari command-tail image (length byte first),
    // which is exactly what the loader copies into the child's `p_cmdlin`.
    // `ret` is `dos_exec`'s value (PROC05): the child's exit code, zero-extended
    // `0 … 65535` (`Pterm(-1)` → 65535), or a negative load error.
    let mut tail = [0u8; MAXPATHLEN];
    let tlen = shell::s_tail_image(&mut tail);
    let ret = crate::launch::run_program_with_tail_rc(&cmd, &tail[..tlen]);

    // `if (rlr->p_flags & AP_OPEN) { … }` (:650-657) — the program called
    // `appl_init` but never `appl_exit`. Done inside
    // `run_program_with_tail_rc`, at the same point in the sequence EmuTOS
    // runs it (right after `dos_exec` returns), because that is where the
    // child's AES id is known.

    // "if the user ran an autorun application and quitted it, return now to
    // the default desktop" (:662-666). EmuTOS reads `psh->sh_nextapp` **live**
    // here, after the child has run: an autorun program that called
    // `shel_write(SHW_EXEC, …)` on its way out has overwritten it, and its
    // choice must win over the return-to-desktop default.
    if shell::sh_nextapp() == AUTORUN_APP {
        shell::set_default_desktop();
    }

    // `if (gl_shgem) wm_new();` (:676-677) — general cleanup after a
    // graphics-mode program; a character-mode one has no windows to sweep.
    if shell::gl_shgem() {
        tos_api::aes::window::wm_new();
    }

    kprintln!("[shell] sh_ldapp: '{}' exited with rc={}", as_str(&cmd), ret);
    crate::print_mem05_stats("after program");

    // `if (ret < 0) return ret; else return 0;` (:681-684) — a load error goes
    // to `sh_main`'s alert, a child's exit code (never negative) never does.
    shell::sh_ldapp_result(ret)
}

/// The desktop the DESKTOP arm of [`sh_ldapp`] is about to run, for its log
/// line: none on a machine without a screen (the arm falls through to the
/// console) or after the desktop program failed to load this boot (D6-c),
/// else the embedded program [`shell::DEF_DESKTOP`].
fn desktop_label() -> &'static str {
    if !shell::gl_shgem() {
        "(none: no screen)"
    } else if DESKTOP_LOAD_ERROR.load(Ordering::Acquire) != 0 {
        "(none: load failed)"
    } else {
        as_str(shell::DEF_DESKTOP)
    }
}

/// NEWTOS: D6-d — the DESKTOP arm's fallback when the desktop program cannot
/// be loaded (`rc`, a negative GEMDOS code): the console arm's body
/// ([`run_console_arm`], `gemshlib.c:614-622`), with the reason on the
/// console's own screen as well as on the serial line.
///
/// EmuCON is a character-mode program, so the screen is first put into the
/// mode `sh_chgrf` (`gemshlib.c:513-523`) puts it in for one (`sh_isgem`
/// FALSE): alpha. The console arm's `set_default_desktop` then makes the
/// desktop the next program again, and the next pass of `sh_main` comes back
/// here through the DESKTOP arm without loading it (D6-c).
fn run_console_instead_of_desktop(rc: i32) -> i32 {
    shell::set_sh_isgem(false);
    sh_chgrf();
    crate::shell::run_console_without_desktop(shell::DEF_DESKTOP, rc);
    shell::set_default_desktop();
    0
}

/// The `CONSOLE_APP` arm body (`gemshlib.c:614-622`):
/// `aes_run_rom_program(coma_start); set_default_desktop(psh); return 0;`
///
/// Note EmuTOS does **not** call `p_nameit`/`p_setappdir` here — EmuCON runs
/// under whatever name the previous occupant left behind
/// (`docs/api/aes/shell-loop.md` §5).
fn run_console_arm() -> i32 {
    crate::shell::run_console();
    shell::set_default_desktop();
    0
}

/// `p_setappdir(rlr, D.s_cmd)` (`gempd.c:90-107`) — record the directory the
/// program was started from, which `sh_find` step (1) searches first.
fn p_setappdir(cmd: &[u8]) {
    tos_api::aes::resource::set_app_dir(cmd);
}

/// `p_nameit(rlr, fname); p_setappdir(rlr, D.s_cmd); rlr->p_flags = 0;`
/// (`gemshlib.c:633-636`).
///
/// All three run, split over two places by *when* each is needed. The
/// app-directory half is set here because `sh_find` searches it before the
/// launch; `p_nameit` **and** `rlr->p_flags = 0` are applied together by
/// [`crate::launch::run_program_with_tail_rc`] at the moment the child gets its
/// process slot ([`tos_api::aes::register_process`] →
/// `tos_api::aes::pd::register`), because NewTOS's AES process descriptors are
/// keyed on that slot (A62: one entry per GEMDOS slot plus one for the
/// kernel context, `tos_api::aes::pd::KERNEL_CONTEXT`), so the descriptor to
/// rename and clear is only identified there. There is exactly **one** launch
/// path — this arm: since DESK43 U3 the desktop asks for a program with
/// `shel_write(SHW_EXEC, …)` and terminates (`deskpro.c:35-45`,
/// `deskmain.c:2166`), as EmuTOS's does.
///
/// Item 10 of the plan's gap list ("AESPD numbering + `p_nameit`/`p_setappdir`
/// on `rlr`") is closed: like EmuTOS's `AESPD`s (`aes/gempd.c:32-35`),
/// NewTOS's exist per index from boot, `appl_init` cannot fail, and the
/// descriptor a program is renamed into is the one its slot has always had
/// (the *identity*, A62); and the **reported value** is EmuTOS's — 0 for the
/// whole application chain, whichever slot (`tos_api::aes::appl::reported_pid`,
/// A56, spec § "AES process identity and pid", contract (b)). The behaviour
/// EmuTOS gets from its model at
/// this boundary — a program never starting on the previous occupant's flags
/// or messages — is in place (A28): the flag reset in `pd::register`, the queue
/// drain in the desktop's own `appl_exit()` at the end of `deskmain`
/// (`deskmain.c:2166`, `desktop::run`), and since A62 the slot's release
/// (`pterm::release_slot` → `pd::release`) for every way a process ends.
fn p_nameit_and_appdir(cmd: &[u8]) {
    p_setappdir(cmd);
}

/// Say what was tried and why the desktop is not there, on the console the
/// user is about to land in. With no raster backend the framebuffer console
/// is inert, so this goes to the same serial device the console REPL's own
/// `Cconout` output reaches — the difference between a fallback and a mystery.
fn report_no_desktop() {
    let mut desk = [0u8; MAXPATHLEN];
    shell::s_cmd(&mut desk);
    kprintln!(
        "[shell] no screen: the desktop ('{}') cannot be started \
         — falling back to the console (EmuCON)",
        as_str(&desk)
    );
}

/// Length of a NUL-terminated byte string.
fn clen(s: &[u8]) -> usize {
    s.iter().position(|&b| b == 0).unwrap_or(s.len())
}

/// A NUL-terminated byte string as `&str` for logging (`"?"` if not UTF-8).
fn as_str(s: &[u8]) -> &str {
    core::str::from_utf8(&s[..clen(s)]).unwrap_or("?")
}
