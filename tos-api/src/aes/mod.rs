//! AES — Application Environment Services (GEM trap, selector 200).
//!
//! Spec: [`docs/api/aes/_dispatcher.md`] and the per-manager files under
//! `docs/api/aes/`. The AES shares the GEM trap (`svc #2`) with the VDI;
//! the kernel's exception handler routes selector 200 (`x0`) here with
//! the AES parameter block (`AESPB`) pointer in `x1`.
//!
//! Like the VDI, the AES takes a **parameter block of array pointers**
//! (six of them) rather than a register ABI. [`dispatch`] is EmuTOS's `xif`
//! (`aes/gemsuper.c:428-448`): it parses the block, runs the [`crysbind`]
//! opcode switch against a **local** `int_out[O_SIZE]` — whose slot 0 is
//! `RET_CODE` — and then copies exactly `control[OUT_LEN]` words of it into
//! the caller's array. A caller that declares `OUT_LEN 0` gets nothing
//! written, not even the return code.

pub mod acc;
mod appl;
mod event;
pub mod form;
pub mod fsel;
mod graf;
pub mod gsxif;
pub mod init;
mod input;
pub mod menu;
pub mod object;
pub mod resource;
pub mod shell;
pub mod thread;
pub mod trace;
pub mod window;
mod pd;

pub use graf::hbox_of;

use core::sync::atomic::{AtomicU32, Ordering};

use crate::bdos::process::Abi;
use crate::uaccess::{Space, UserFault};

/// GEMDOS slots whose tenant has ended and whose Window-Manager property has
/// **not** been swept yet — one bit per slot (`MAX_PROCESSES` = 16, so a
/// `u32` covers it with room to spare). Set by [`release_client`] on the
/// `Pterm` path, drained by [`sweep_dead_clients`] at the next AES dispatch
/// (A107).
static PENDING_SWEEP: AtomicU32 = AtomicU32::new(0);

/// Mark `slot`'s Window-Manager property for the next dispatch's sweep.
fn mark_for_sweep(slot: usize) {
    if slot < crate::bdos::process::MAX_PROCESSES {
        PENDING_SWEEP.fetch_or(1u32 << slot, Ordering::AcqRel);
    }
}

/// Run [`window::wm_new_for_slot`] for every slot [`release_client`] marked
/// — EmuTOS's `if (gl_shgem) wm_new();` after a program ends
/// (`aes/gemshlib.c:676-677`), for the one program EmuTOS never has: an EL0
/// `Pexec` child of a **living** GEM parent (A107).
///
/// **Why here and not at the `Pterm`.** The sweep draws: `wm_close` runs
/// `draw_change` (`gemwmlib.c:1453`), which repaints the background over the
/// vacated rectangle and posts `WM_REDRAW` to every window underneath
/// (`:1216-1222`). The only hook on the child's death is
/// `aes::release_client(slot)` from `bdos::process::pterm::release_slot`,
/// which the kernel calls from the `Pterm` SVC handler
/// (`kernel/src/arch/aarch64/exceptions.rs`) — on the **exception stack**,
/// with the dying process's mapping still installed and the parent's AES
/// fiber possibly parked mid-call (`aes::thread`). Neither the raster nor the
/// parent's message queue may be touched from there.
///
/// This function runs at the entry of [`dispatch`], **before** the opcode:
/// the same context as the opcode itself and as the inline Control Manager
/// (`menu::track`, `window::intercept`, reached from `evnt_multi` on this
/// very path), i.e. the context in which every other AES draw happens — an
/// EL0 caller's dispatch is on the AES fiber (`thread::call`), an in-kernel
/// caller's on the boot stack, and in both the fiber is idle at entry (a
/// parked `G_USERDEF` stall resumes through `thread::resume`, never through
/// `dispatch`). `RUN` is already the parent at that point (`xterm`'s `run =
/// run->p_parent`, `bdos/proc.c:609`; our `pterm::terminate`), so the sweep's
/// repaint and its `WM_REDRAW` fan-out reach the living client, which is
/// exactly what `sh_ldapp`'s sweep does for the shell loop's next program.
///
/// The mark can only be consumed by the **first** AES call after the child
/// died, and a program that takes the freed slot next cannot own an AES
/// window before its own first AES call — so a recycled slot is never swept
/// out from under its new tenant.
fn sweep_dead_clients() {
    let mut pending = PENDING_SWEEP.swap(0, Ordering::AcqRel);
    while pending != 0 {
        let slot = pending.trailing_zeros() as usize;
        pending &= !(1u32 << slot);
        window::wm_new_for_slot(slot);
    }
}

/// Test hook (not wired to boot): is `slot` still waiting for its post-`Pterm`
/// sweep? Lets a host test state that the mark is set at `release_slot` and
/// cleared by the parent's next AES call (A107).
pub fn test_sweep_pending(slot: usize) -> bool {
    PENDING_SWEEP.load(Ordering::Acquire) & (1u32 << slot) != 0
}

/// **Start a new AES session** — everything `gem_main` and
/// `run_accs_and_desktop` do between one `sh_main` returning and the next one
/// being entered (A106; EmuTOS `aes/geminit.c:753-899` under the
/// `gemstart.S:86-124` `aes_restart` loop).
///
/// The shell loop ends a session when a program terminated while a
/// `shel_write(SHW_RESCHNG, …)` was pending (`gemshlib.c:727`). EmuTOS then
/// unwinds to assembly, re-zeroes the whole `theglo` global area
/// (`gemstart.S:101-110`) and re-enters `gem_main`, which rebuilds every AES
/// structure from scratch and re-runs the accessories + desktop. NewTOS's AES
/// state is a set of module statics rather than one `D` struct, so "re-zero
/// `theglo`" becomes this function: each module puts its own share back to
/// the state it holds at power-up, in EmuTOS's order.
///
/// Called from exactly one place, the kernel's session loop around `sh_main`
/// (`kernel/src/lib.rs`), which is NewTOS's `aes_restart`.
///
/// **The resolution switch itself is deliberately not here.** `gem_main`'s
/// `Setscreen`/`Vsetmode` dispatch (`geminit.c:778-793`) is the only
/// hardware-gated step of the sequence, and the board has exactly one mode,
/// negotiated once at boot (ADR-017 rev 2, D-101): the request is recorded
/// (`gl_changerez`/`gl_nextrez`), reported, and never handed to a mode
/// switch. `gsx_wsclear()` (`:795`) has already happened by the time we get
/// here — `sh_main`'s own tail runs `sh_toalpha()` (`gemshlib.c:733`), which
/// clears the screen and homes the cursor (`v_enter_cur`), and the next
/// session's `gsx_init` → `v_opnwk` → `esc_init` hides the text cursor and
/// clears again: the screen EmuTOS's `\033f\033E` (`geminit.c:763`) leaves.
///
/// What is **not** reset, and why:
///
/// * [`gsxif`]'s screen buffer (`gl_tmp`/`gl_mlen`) — a kernel-owned pool
///   block taken once and never freed (module docs, `docs/COMPATIBILITY.md`,
///   A54/A57). EmuTOS frees it in `sh_toalpha` and takes a fresh one in the
///   next `sh_tographic` (`gemshlib.c:236,258`); NewTOS has no `gsx_mfree`,
///   so re-allocating would leak one screen-sized block per restart. The
///   address is stable and kernel-owned, so nothing can dangle: the next
///   session's `sh_tographic` finds the same buffer and `WF_SCREEN` keeps
///   reporting it. `clipped_saves()` counts for the whole boot and is
///   diagnostics, not session state.
/// * [`window`]'s `WM_READY` — a lazy-init latch for the desktop window, not
///   session state; what it guards ([`window::wm_init`]'s `init_desk_window`)
///   runs unconditionally below.
/// * [`thread`]'s fiber context — idle by construction at this point (the
///   session ends with the running program terminated and `sh_main` returning
///   on the boot stack), and not AES session state.
/// * [`trace`]'s ring buffer — a diagnostic instrument (feature-gated) whose
///   value is that it spans the boot.
/// * `fsel`'s relocated `GEM.RSC` copy — EmuTOS re-runs `gem_rsc_init`/
///   `fs_start` per session on a fresh ROM copy (`geminit.c:658,724`); ours
///   is a kernel-BSS blob relocated once and cached, with no pointer outside
///   the kernel. Its per-dialog scratch (path, selection, listing) is
///   rewritten by every `fsel_input`.
pub fn reinit() {
    // `gem_main`: re-read the `.INF` file and rebuild the AES's own resource
    // (`geminit.c:766-773`, `gem_rsc_init`/`gem_rsc_fixit`), skipping
    // `process_inf1` as `:775` does on this path. Also clears `gl_changerez`
    // (`:806`) via `shell::reinit` below.
    init::reinit();
    // `mn_init()` (`:808`) + `mn_cleanup`'s submenu free + `mnext_init()`
    // (`:672`).
    menu::reinit();
    // The desk-accessory process list. EmuTOS reaches the same state by
    // re-running `count_accs`/`load_accs` for the new session
    // (`geminit.c:812,687`); the kernel's `free_accs` has already disposed of
    // the processes by the time a restart gets here.
    acc::reinit();
    // The Window Manager: the `theglo` blank of `D.w_win[]` plus `wm_start()`
    // (`:723`). Before the process table, because neither draws or posts here
    // — but the order that would matter if it ever did is this one: no window
    // may be left that could post into a queue the next step empties.
    window::reinit();
    // The process table and its message queues (`:846-880`).
    // SAFETY: single-CPU restart path, no AES call in flight and no EL0
    // process alive (see the `pd::reinit` docs).
    unsafe { pd::reinit() };
    // The button variables (`:661-664`) and the mouse owner (`:888`).
    input::reinit();
    // `init_3d()` (`:675`).
    object::init_3d();
    // `ev_dclick(3, TRUE)` (`:701`) — the power-up click rate, before
    // `process_inf2` re-applies the `.INF` `#E` line (`:734`).
    event::ev_dclick(3, true);
    // No wait is armed and no `appl_yield` is half done across a restart
    // (A112).
    event::reinit_waits();
    appl::reinit_yields();
    // Per-process resource lists and the application directory.
    resource::reinit();
    // The Shell Manager, `gl_changerez = FALSE` included (`:806`).
    shell::reinit();
    // No dead client is left to sweep: every window is gone (A107).
    PENDING_SWEEP.store(0, Ordering::Release);
}

/// The AES parameter block: six pointers (four to WORD arrays, two to
/// LONG arrays). The block pointers — and the `addrin`/`addrout` LONG
/// elements — are the caller's pointer width: 8-byte for an LP64 caller,
/// 4-byte for an ILP32 one (ADR-020). `x1` points at the six consecutive
/// pointers; [`Aespb::from_raw`] is told the caller's [`Abi`] so it reads
/// them at the right stride. The WORD arrays (`control`/`global`/`intin`/
/// `intout`) are 16-bit either way, so those stay plain `*mut i16`.
///
/// **Caller space (ARCH03 stage 3a).** The block and every array it names
/// are in the caller's address space ([`Space`]): an EL0 program's memory is
/// read and written with its rights through [`crate::uaccess`] — or checked
/// with its rights and then reached through its kernel alias
/// ([`Space::kptr`], plan §3.6), so the AES never dereferences a user address
/// through whatever `TTBR0` happens to be live — and a bad pointer ends the
/// call and, through the syscall tail, the program (EmuTOS: bus error in
/// `xif`/the manager, `docs/api/bios/_exception-crash.md`). An in-kernel
/// caller's (the desktop's `svc`, the boot selftests) pointers are kernel
/// addresses and are used as they are.
pub(crate) struct Aespb {
    /// The caller's `control` array — its **kernel** address (checked
    /// readable for the five WORDs of fixed meaning in [`Self::from_raw`]).
    control: *mut i16,
    /// The caller's `global` array, a **caller** address: checked and
    /// translated only by the managers that use it ([`Self::global_kptr`]) —
    /// most calls never touch it, as in EmuTOS.
    global: u64,
    /// The **dispatch-local** `int_in[I_SIZE]` every manager reads —
    /// EmuTOS `xif`'s own stack array (`gemsuper.c:431`). The caller's array
    /// is read exactly once, by [`Aespb::from_raw`], and only the
    /// `min(control[IN_LEN], I_SIZE)` words the call declared.
    intin: *mut i16,
    /// The **caller's** `intin` array, kept for the same reason
    /// [`Self::caller_intout`] is: so the trap tracer can still report a block
    /// whose `intin` pointer is null (`trace::flags::NO_INTIN`). No manager
    /// ever sees it, and without the `aes-trace` feature nothing reads it —
    /// the tracer is the field's only consumer.
    #[cfg_attr(not(feature = "aes-trace"), allow(dead_code))]
    caller_intin: *const i16,
    /// The **dispatch-local** `int_out[O_SIZE]` every manager writes —
    /// EmuTOS `xif`'s own stack array (`gemsuper.c:432`). Nothing a manager
    /// stores here reaches the caller until [`Aespb::copy_intout_back`] runs.
    intout: *mut i16,
    /// The **caller's** `intout` array (a caller address). Written by exactly
    /// one place, [`Aespb::copy_intout_back`], and only `control[OUT_LEN]`
    /// words of it, in the caller's space.
    caller_intout: u64,
    /// The **dispatch-local** `addr_in[AI_SIZE]` every manager reads —
    /// EmuTOS `xif`'s own stack array (`gemsuper.c:433`). Filled by
    /// [`Aespb::from_raw`] with the `min(control[AIN_LEN], AI_SIZE)` slots
    /// the call declared, already widened from the caller's pointer width to
    /// a native address, so the accessor needs no ABI knowledge.
    addrin: *mut u64,
    /// Base address of the caller's `addrout` LONG array (a caller address)
    /// — written by the managers that return addresses (`rsrc_gaddr`, …).
    /// Element width tracks [`Self::abi`], so this stays a raw base indexed
    /// at write time, in the caller's space.
    addrout: u64,
    /// Caller pointer width, threaded from the running process.
    abi: Abi,
    /// Whose address space the block and its arrays are in.
    space: Space,
}

// CONTRL indices with fixed meaning (`include/aesdefs.h`).
const C_OPCODE: usize = 0;
const C_IN_LEN: usize = 1;
const C_OUT_LEN: usize = 2;
const C_AIN_LEN: usize = 3;

/// `I_SIZE` — the size of the AES's `int_in` array, 16 WORDs
/// (`include/aesdefs.h:122`). `xif` copies the inputs into a local
/// `WORD int_in[I_SIZE]` (`gemsuper.c:431`); so do we. 16 is not a round
/// number picked for comfort: `evnt_multi` is the one function with sixteen
/// input words (`aesdefs.h:193-221`), which is why EmuTOS' own binding has to
/// lay `control[1] = 16` by hand instead of through its packed control code
/// (`desk/gembind.c:268-269`).
const I_SIZE: usize = 16;

/// `O_SIZE` — the size of the AES's `int_out` array, 7 WORDs
/// (`include/aesdefs.h:123`). `xif` builds its results in a local
/// `WORD int_out[O_SIZE]` (`gemsuper.c:432`); so do we.
const O_SIZE: usize = 7;

/// `AI_SIZE` — the size of the AES's `addr_in` array, 3 LONGs
/// (`include/aesdefs.h:124`). `xif` copies the address inputs into a local
/// `LONG addr_in[AI_SIZE]` (`gemsuper.c:433`); so do we. Three is
/// `fsel_exinput`'s slot count, the AES maximum (`desk/gembind.c:707`).
const AI_SIZE: usize = 3;

impl Aespb {
    /// Build from the raw `x1` pointer to the six-pointer block, reading
    /// the block pointers at the caller's pointer width (`abi`): 8-byte
    /// LP64 pointers, or 4-byte (zero-extended) ILP32 pointers.
    ///
    /// This is `xif`'s marshalling step (`aes/gemsuper.c:435-439`):
    ///
    /// ```c
    /// if (IN_LEN)
    ///     memcpy(int_in, pcrys_blk->intin, min(IN_LEN,I_SIZE)*sizeof(WORD));
    /// if (AIN_LEN)
    ///     memcpy(addr_in, pcrys_blk->addrin, min(AIN_LEN,AI_SIZE)*sizeof(LONG));
    /// ```
    ///
    /// The caller's `intin` / `addrin` arrays are touched **here and nowhere
    /// else**, and only for the `min(declared, size)` elements the call
    /// declared; every manager afterwards reads `local_intin` /
    /// `local_addrin`. On EmuTOS that clamp is a stack-buffer bound; with two
    /// address spaces it is the boundary between the words an application
    /// handed over and the rest of its memory.
    ///
    /// `local_intout` is the dispatch-local `int_out[O_SIZE]` the managers
    /// write into (EmuTOS `gemsuper.c:432`); the caller's own `intout`
    /// pointer is kept aside for [`Self::copy_intout_back`].
    ///
    /// A **negative** length copies nothing. EmuTOS' `if (IN_LEN)` is true for
    /// it and `min(IN_LEN,I_SIZE)*sizeof(WORD)` then widens to a huge
    /// `size_t` — a `memcpy` of about four gigabytes, not a behaviour with a
    /// value to reproduce. [`Self::copy_intout_back`] reads a negative
    /// `OUT_LEN` the same way.
    ///
    /// The block and the declared parts of `control`/`intin`/`addrin` are
    /// read in the caller's `space` (see [`Aespb`]); `Err` when the program
    /// may not read one of them — the fault is recorded and the call must
    /// stop (the syscall tail ends the program).
    ///
    /// # Safety
    /// For [`Space::Kernel`]: `ptr` must point at six consecutive valid array
    /// pointers of the caller's width; the caller's `intin` must hold at
    /// least `min(control[IN_LEN], I_SIZE)` WORDs and its `addrin` at least
    /// `min(control[AIN_LEN], AI_SIZE)` LONGs — its own declaration. For
    /// both spaces: `local_intin` / `local_intout` / `local_addrin` must be
    /// writable `[i16; I_SIZE]` / `[i16; O_SIZE]` / `[u64; AI_SIZE]` that
    /// outlive this `Aespb`.
    pub(crate) unsafe fn from_raw(
        ptr: u64,
        abi: Abi,
        space: Space,
        local_intin: *mut i16,
        local_intout: *mut i16,
        local_addrin: *mut u64,
    ) -> Result<Self, UserFault> {
        // The six block pointers, at the caller's width, in its space.
        let mut blk = [0u64; 6];
        for (i, slot) in blk.iter_mut().enumerate() {
            *slot = abi.read_ptr_in(space, ptr, i)?;
        }
        let [control_va, global, caller_intin, caller_intout, caller_addrin, addrout] = blk;
        // `control`: the five WORDs of fixed meaning (`OPCODE`, `IN_LEN`,
        // `OUT_LEN`, `AIN_LEN`, `AOUT_LEN`), read on every call.
        let control = space.kptr(control_va, 2 * 5, false)? as *mut i16;

        // SAFETY: `control` is the checked kernel address of 5 WORDs; the
        // locals are the caller's (`dispatch`'s) `[_; I_SIZE/AI_SIZE]`.
        unsafe {
            // `if (IN_LEN) memcpy(int_in, …, min(IN_LEN,I_SIZE)*sizeof(WORD))`
            let declared_in = control.add(C_IN_LEN).read();
            let n = if declared_in <= 0 { 0 } else { (declared_in as usize).min(I_SIZE) };
            for i in 0..n {
                // `i < I_SIZE` bounds the local write; the read is inside the
                // `IN_LEN` words the caller declared, in its space.
                local_intin.add(i).write(space.get_u16(caller_intin + 2 * i as u64)? as i16);
            }

            // `if (AIN_LEN) memcpy(addr_in, …, min(AIN_LEN,AI_SIZE)*sizeof(LONG))`
            let declared_ain = control.add(C_AIN_LEN).read();
            let m = if declared_ain <= 0 { 0 } else { (declared_ain as usize).min(AI_SIZE) };
            for i in 0..m {
                // `i < AI_SIZE` bounds the local write; one caller-width LONG
                // inside the declared `AIN_LEN`, in the caller's space.
                local_addrin.add(i).write(abi.read_ptr_in(space, caller_addrin, i)?);
            }
        }

        Ok(Aespb {
            control,
            global,
            intin: local_intin,
            caller_intin: caller_intin as *const i16,
            intout: local_intout,
            caller_intout,
            addrin: local_addrin,
            addrout,
            abi,
            space,
        })
    }

    /// `xif`'s copy-back rule (`aes/gemsuper.c:443-444`):
    ///
    /// ```c
    /// if (OUT_LEN)
    ///     memcpy(pcrys_blk->intout, int_out, OUT_LEN*sizeof(WORD));
    /// ```
    ///
    /// Exactly `control[OUT_LEN]` WORDs reach the caller — **including**
    /// `RET_CODE`, which is just the local `int_out[0]`
    /// (`include/aesdefs.h:140`). With `OUT_LEN == 0` nothing is written at
    /// all, not even the return code; slots past `OUT_LEN` keep whatever the
    /// caller had there.
    ///
    /// **Boundary, and it is ours, not EmuTOS's.** EmuTOS does not clamp
    /// `OUT_LEN` against `O_SIZE`, so an `OUT_LEN > 7` copies whatever
    /// follows its 7-WORD stack array — an indeterminate value even on
    /// EmuTOS, not a behaviour with a value to reproduce, and an
    /// out-of-bounds read here. The defined range (`OUT_LEN <= O_SIZE`) is
    /// reproduced exactly; beyond it we copy the 7 words that exist and stop.
    /// A negative `OUT_LEN` (EmuTOS: `OUT_LEN*sizeof(WORD)` widening to a
    /// huge `size_t`) copies nothing.
    ///
    /// The words are written in the caller's space; a word the program may
    /// not write stops the copy (the fault is recorded, the syscall tail ends
    /// the program).
    ///
    /// # Safety
    /// For [`Space::Kernel`]: the caller's `intout` must hold at least
    /// `min(control[OUT_LEN], O_SIZE)` WORDs — the caller's own declaration.
    unsafe fn copy_intout_back(&self) {
        let declared = self.control(C_OUT_LEN);
        let n = if declared <= 0 { 0 } else { (declared as usize).min(O_SIZE) };
        for i in 0..n {
            // SAFETY: `i < O_SIZE` bounds the local read.
            let w = unsafe { self.intout.add(i).read() };
            if self.space.put_u16(self.caller_intout + 2 * i as u64, w as u16).is_err() {
                return;
            }
        }
    }

    #[inline]
    fn control(&self, i: usize) -> i16 {
        // SAFETY: callers index within the control array (≥ 5 WORDs).
        unsafe { self.control.add(i).read() }
    }
    /// Read from the dispatch-local `int_in[I_SIZE]` (EmuTOS
    /// `gemsuper.c:431`). What a manager finds past `control[IN_LEN]` is a
    /// cleared word, not the caller's memory — the clamp is in
    /// [`Self::from_raw`], the one place that ever touches the caller's array.
    /// Indices past `I_SIZE` would run off the local buffer and can never hold
    /// a declared word (the copy is capped at `I_SIZE`), so they read 0 —
    /// `evnt_multi`'s sixteen words are the AES maximum.
    #[inline]
    fn intin(&self, i: usize) -> i16 {
        debug_assert!(i < I_SIZE, "AES int_in index past I_SIZE");
        if i >= I_SIZE {
            return 0;
        }
        // SAFETY: `intin` is the dispatch-local `[i16; I_SIZE]` and
        // `i < I_SIZE` was just checked.
        unsafe { self.intin.add(i).read() }
    }
    /// Store into the dispatch-local `int_out[O_SIZE]` (EmuTOS
    /// `gemsuper.c:432`). A manager writes freely here; how much of it the
    /// caller ever sees is decided by [`Self::copy_intout_back`] alone.
    /// Indices past `O_SIZE` could never reach the caller (the copy-back is
    /// capped at `O_SIZE`), so they are dropped rather than running off the
    /// local array — `evnt_multi`'s seven words are the AES maximum.
    #[inline]
    fn set_intout(&self, i: usize, v: i16) {
        debug_assert!(i < O_SIZE, "AES int_out index past O_SIZE");
        if i < O_SIZE {
            // SAFETY: `intout` is the dispatch-local `[i16; O_SIZE]` and
            // `i < O_SIZE` was just checked.
            unsafe { self.intout.add(i).write(v) }
        }
    }
    /// The function return code (`RET_CODE = int_out[0]`).
    #[inline]
    fn set_ret(&self, v: i16) {
        self.set_intout(0, v);
    }
    /// Read from the dispatch-local `addr_in[AI_SIZE]` (EmuTOS
    /// `gemsuper.c:433`). The caller's slots were widened to native addresses
    /// in [`Self::from_raw`], which is also where the `min(AIN_LEN, AI_SIZE)`
    /// clamp lives; an ILP32 pointer (e.g. an OBJECT tree in the low-VA TPA)
    /// zero-extended to a valid VA there. Past the declaration a manager gets
    /// a null pointer, not an address the call never handed over; past
    /// `AI_SIZE` it gets one too, for the same reason [`Self::intin`] does —
    /// three slots (`fsel_exinput`) are the AES maximum.
    #[inline]
    fn addrin(&self, i: usize) -> u64 {
        debug_assert!(i < AI_SIZE, "AES addr_in index past AI_SIZE");
        if i >= AI_SIZE {
            return 0;
        }
        // SAFETY: `addrin` is the dispatch-local `[u64; AI_SIZE]` and
        // `i < AI_SIZE` was just checked.
        unsafe { self.addrin.add(i).read() }
    }
    /// The declared `addrin` length, `control[AIN_LEN]`. Used by
    /// `wind_set` `WF_NAME`/`WF_INFO`/`WF_NEWDESK` to tell the native
    /// convention (string pointer in `addr_in[0]`, `AIN_LEN >= 1`) from
    /// the classic-AES gemlib convention (pointer packed in the `int_in`
    /// WORDs, `AIN_LEN == 0`).
    #[inline]
    fn ain_len(&self) -> i16 {
        self.control(C_AIN_LEN)
    }
    /// How many `int_in` WORDs a pointer packed at the caller's own width
    /// occupies (the packed form [`Self::intin_ptr`] reads): four for an LP64
    /// caller (8 bytes), two for an ILP32 one (4 bytes). The one place that
    /// width is decided — a word that a caller lays *after* such a pointer
    /// (`wind_set(WF_NEWDESK)`'s root, `int_in[2 + ptr_words]`: `int_in[4]`
    /// for ILP32, bit-exact with EmuTOS's `pinwds[2]`, `aes/gemwmlib.c:1688`;
    /// `int_in[6]` for LP64 — A81) is found through this, not a fixed slot.
    #[inline]
    fn ptr_words(&self) -> usize {
        match self.abi {
            Abi::Lp64 => 4,
            Abi::Ilp32 => 2,
        }
    }
    /// Read a caller-width pointer packed into the `int_in` WORD array
    /// starting at WORD index `word`. This is the classic AES convention
    /// for the `wind_set` `WF_NAME`/`WF_INFO` string pointer: gemlib writes
    /// `*(char**)(aes_intin + word) = ptr`
    /// (`a_wind_set_str.c:25`), i.e. 4 pointer bytes at
    /// ILP32 (`int_in[word..word+2]`) or 8 at LP64 (`int_in[word..word+4]`),
    /// stored at `&int_in[word]`. That address is only WORD-aligned, so the
    /// load must be unaligned.
    ///
    /// The bytes come out of the dispatch-local `int_in[I_SIZE]`, so a caller
    /// whose `IN_LEN` does not cover the whole packed pointer hands over a
    /// **truncated** one — the same words the manager would have got on
    /// EmuTOS, where `wm_set` reads `*(char **)pinwds` out of `xif`'s local
    /// buffer (`aes/gemwmlib.c:1661`). A pointer that would run off the end of
    /// that buffer reads as null rather than off the array; gemlib declares
    /// `num_intin = 6` for the one opcode that uses this convention
    /// (`a_wind_set_str.c:21`), which holds the four LP64 words at index 2.
    #[inline]
    fn intin_ptr(&self, word: usize) -> u64 {
        let words = self.ptr_words();
        debug_assert!(word + words <= I_SIZE, "packed AES int_in pointer past I_SIZE");
        if word + words > I_SIZE {
            return 0;
        }
        // &int_in[word] — the pointer bytes start `word` WORDs (2 bytes
        // each) into the `int_in` array.
        let base = (self.intin as u64).wrapping_add((word as u64) * 2);
        // SAFETY: `intin` is the dispatch-local `[i16; I_SIZE]` and the bound
        // above keeps all `words` WORDs of the pointer inside it; the load
        // width tracks the caller ABI and tolerates the slot's 2-byte
        // alignment.
        unsafe {
            match self.abi {
                Abi::Lp64 => (base as *const u64).read_unaligned(),
                Abi::Ilp32 => (base as *const u32).read_unaligned() as u64,
            }
        }
    }
    /// Store a 32-bit `LONG` over two consecutive `int_out` WORDs starting
    /// at `word` — the way `wm_get(WF_SCREEN)` hands out the screen buffer:
    /// `gsx_mret((LONG *)poutwds, (LONG *)(poutwds + 2))`
    /// (`aes/gemwmlib.c:1564`, `gemgsxif.c:125-129`) executes `*(LONG *)
    /// &int_out[1] = addr; *(LONG *)&int_out[3] = len;` — a native 32-bit
    /// store at a WORD-aligned address. This is that store, native to
    /// AArch64: little-endian, so `int_out[word]` holds bits 0–15 and
    /// `int_out[word + 1]` bits 16–31, and unaligned because `&int_out[word]`
    /// is only 2-byte aligned. The width is fixed at 32 bits regardless of
    /// the caller's ABI because EmuTOS writes 32 bits and the values (a user
    /// VA under 32 MiB, a length under 4 GiB) fit — the T04 contract
    /// (`docs/COMPATIBILITY.md`, "A `LONG` in `int_in`/`int_out`/`ptsin`/
    /// `ptsout`"): a LONG is 32 bits over two WORDs at every caller width,
    /// `WF_SCREEN` stays LONG-class (A57), it is not widened to a pointer.
    ///
    /// What a reader gets back therefore depends on the reader's width, not
    /// on the store: gemlib's `mt_wind_get` copies the four WORDs and
    /// TeraDesk reassembles the length from `int_out[3..4]` through its
    /// `union { long size; struct { _WORD high, low; } words; }`
    /// (`sdk/apps/teradesk/resource.c:233-241`) — low 32 bits right, the
    /// upper 32 bits of its 8-byte `long` are its own uninitialised stack.
    /// An **8-byte** read of one of these slots on LP64 is wrong: cflib's
    /// `*((void **)&aes_intout[1])` / `*((long *)&aes_intout[3])`
    /// (`sdk/cflib/cflib-21/purec.c:166,171`, ILP32-correct, `__PUREC__`-only
    /// and not compiled on NewTOS) and gemlib's pristine
    /// `aes_intout_ptr(1, void *)` (`a_wind_get_ptr.c:51`) would take the
    /// length as the address's upper half, `addr | len << 32` — pinned by
    /// `tos-api/tests/aes_t04_intin_long_contract.rs`; the gemlib overlay
    /// `sdk/gemlib/overlay/a_wind_get_ptr.c` reads 32 bits there.
    ///
    /// Like [`Self::set_intout`], a store that would run off the local
    /// `int_out[O_SIZE]` is dropped rather than made.
    #[inline]
    fn set_intout_long(&self, word: usize, v: u32) {
        debug_assert!(word + 2 <= O_SIZE, "AES int_out LONG past O_SIZE");
        if word + 2 > O_SIZE {
            return;
        }
        // &int_out[word] — `word` WORDs (2 bytes each) into the local array.
        let base = (self.intout as u64).wrapping_add((word as u64) * 2);
        // SAFETY: `intout` is the dispatch-local `[i16; O_SIZE]` and the
        // bound above keeps both WORDs of the LONG inside it; the store
        // tolerates the slot's 2-byte alignment.
        unsafe { (base as *mut u32).write_unaligned(v) }
    }
    /// Store `v` into slot `i` of the caller's `addrout` array, at its
    /// pointer width (ILP32 stores the low 4 bytes), in its space. A slot
    /// the program may not write records the fault (the syscall tail then
    /// ends the program).
    #[inline]
    #[allow(dead_code)] // used by the address-returning managers (later sub-phases)
    fn set_addrout(&self, i: usize, v: u64) {
        let _ = self.abi.write_ptr_in(self.space, self.addrout, i, v);
    }
    /// The caller's `global` array (the 15-WORD / 30-byte `AESGLOBAL`,
    /// `aes/gemrslib.h:15-27`) as a **kernel** address the manager may read
    /// and write, after checking that the caller may write all 30 bytes
    /// ([`Space::kptr`]). `None` — the fault recorded, the call must stop —
    /// when it may not. A null `global` stays null (some managers accept
    /// it, `rsrc_*` test for it).
    #[inline]
    fn global_kptr(&self) -> Option<*mut i16> {
        self.space.kptr_or_null(self.global, 30, true).ok().map(|p| p as *mut i16)
    }
    /// The caller's space (see [`Aespb`]).
    #[inline]
    fn space(&self) -> Space {
        self.space
    }
    /// A caller buffer of `len` bytes at `va` as a kernel address the
    /// manager may use with ordinary loads/stores, after checking the
    /// caller's rights ([`Space::kptr_or_null`]; null stays null). `None`
    /// — fault recorded — when the program may not use it: the manager
    /// must stop.
    #[inline]
    fn ubuf(&self, va: u64, len: u64, write: bool) -> Option<u64> {
        self.space.kptr_or_null(va, len, write).ok()
    }
}

/// AES dispatch entry — EmuTOS's `xif` (`aes/gemsuper.c:428-448`).
///
/// `aespb_ptr` is the `x1` value at the GEM trap (selector 200 in `x0`);
/// `abi` is the calling process's pointer width (the kernel passes
/// [`crate::bdos::process::current_abi`]).
///
/// Like `xif`, the opcode switch ([`crysbind`]) runs against a **local**
/// `int_out[O_SIZE]`; only `control[OUT_LEN]` words of it are copied into the
/// caller's array afterwards ([`Aespb::copy_intout_back`]). A caller that
/// declares `OUT_LEN 0` therefore gets nothing written — not even `RET_CODE`
/// — and slots past its declaration are never touched.
///
/// A blocking `evnt_*` wait that found a runnable peer returns the [`Switch`]
/// to yield to (S2 restartable block, design §4.3 + §8.2 sub-step 3); the
/// GEM-trap handler applies it. Every other opcode returns `None`.
///
/// That `Switch` is the one exit that does **not** copy back, and the reason
/// is EmuTOS's own control flow: a blocking wait there does not return from
/// `crysbind` at all, it calls `dsptch()` and suspends `xif`'s frame *inside*
/// the wait, so `gemsuper.c:443` has not run yet and only runs once the wait
/// really completes. Our S2 equivalent aborts the trap and re-executes it on
/// resume; no result exists yet, and the caller's `int_out` must be left as it
/// was until the completing pass produces one.
///
/// [`Switch`]: crate::bdos::process::sched::Switch
///
/// # Safety
/// `aespb_ptr` must be a valid AES parameter block of `abi`'s pointer
/// width (see [`Aespb`]).
pub unsafe fn dispatch(
    aespb_ptr: u64,
    abi: Abi,
) -> Option<crate::bdos::process::sched::Switch> {
    // SAFETY: forwarded — a kernel-address parameter block.
    unsafe { dispatch_in(aespb_ptr, abi, Space::Kernel) }
}

/// [`dispatch`] for a caller whose parameter block lives in `space`
/// (ARCH03 stage 3a): the GEM trap passes the caller's [`Space`] — an EL0
/// program's block and arrays are then read and written with its rights
/// ([`Aespb`]). [`dispatch`] itself is this with [`Space::Kernel`], for the
/// kernel's own callers and the host tests.
///
/// # Safety
/// As [`dispatch`] for [`Space::Kernel`]; for [`Space::User`] nothing — a
/// bad user pointer is a recorded fault, not undefined behaviour.
pub unsafe fn dispatch_in(
    aespb_ptr: u64,
    abi: Abi,
    space: Space,
) -> Option<crate::bdos::process::sched::Switch> {
    // An EL0 `Pexec` child that ended while this caller was alive left its
    // windows / background tree / menu bar in the AES; sweep them before the
    // opcode, so this call already sees the table `sh_ldapp`'s `wm_new()`
    // would have left it (`aes/gemshlib.c:676-677`; A107). Drawing context:
    // see [`sweep_dead_clients`].
    sweep_dead_clients();
    // `xif`'s four stack arrays (`gemsuper.c:430-433`). EmuTOS leaves all of
    // them uninitialised — indeterminate C auto storage, i.e. no value to
    // reproduce; zeroed here so that neither an under-writing handler can leak
    // stack bytes into a caller that declared more `int_out` words than it
    // filled, nor an over-reading one see anything but a defined 0 past the
    // `IN_LEN` / `AIN_LEN` the caller declared.
    //
    // The `int_out` half of that is an observable difference and a **named
    // deviation**: a handler that writes fewer than `OUT_LEN` words delivers 0
    // in the rest, where EmuTOS delivers whatever its supervisor stack held
    // (`wind_get(WF_COLOR)` never writes `int_out[1]`, the slider fields and
    // `WF_TOP` write `[1]` only, an exhausted rectangle list writes `[3..4]`
    // only, …). Decision and the three reasons — no application effect,
    // measurement noise against the TT dumps, no kernel-stack leak to EL0 —
    // are in `docs/COMPATIBILITY.md` ("An AES handler that writes fewer than
    // `OUT_LEN` words delivers 0", A27); the per-opcode inventory is
    // `docs/analysis/aes-underwriting-handlers-2026-09-19.md`; the regression
    // test is `tos-api/tests/aes_intout_underwrite.rs`.
    let mut local_intin = [0i16; I_SIZE];
    let mut local_intout = [0i16; O_SIZE];
    let mut local_addrin = [0u64; AI_SIZE];
    // SAFETY: caller guarantees a valid parameter block of `abi`'s width
    // (checked in its space for an EL0 caller); the three locals live until
    // this function returns, outliving `pb`.
    let pb = unsafe {
        Aespb::from_raw(
            aespb_ptr,
            abi,
            space,
            local_intin.as_mut_ptr(),
            local_intout.as_mut_ptr(),
            local_addrin.as_mut_ptr(),
        )
    };
    // A block the program may not read: EmuTOS takes the bus error in `xif`
    // before any manager runs. The fault is recorded; the syscall tail ends
    // the program. Nothing else happens for this call.
    let Ok(pb) = pb else {
        return None;
    };

    // AES trap tracer (`trace`): compiled out entirely without the `aes-trace`
    // feature, and even with it the capture is a timestamp plus a fixed-size
    // struct store into a RAM ring — no I/O and no formatting on this path.
    let t0 = trace::call_begin();

    let sw = crysbind(&pb);

    if sw.is_none() {
        // The call completed — `xif`'s copy-back (`gemsuper.c:443-444`).
        // SAFETY: the caller's `intout` holds the `OUT_LEN` words it declared.
        unsafe { pb.copy_intout_back() };
    }
    trace::call_end(&pb, t0, sw.is_some());
    if sw.is_none() {
        // `super()` runs `xif` and then **falls through** to `dsptch()`
        // (`aes/gemsuper.c:452-464`) — every completed AES call ends there.
        // The `Switch` exit is the one case that has not completed: the trap is
        // aborted and re-executed on resume, and the yield itself *is* the
        // dispatch (it came out of `sched::sleep`, which already pumped).
        dsptch();
        // …and that `dsptch()` gives up the CPU (A112 F3, `docs/api/aes/
        // event.md` "Wake conditions and dispatch" point 3): `disp()` puts
        // the caller at the end of the ready list and resumes its head, so a
        // ready peer runs before the caller continues. The returned `Switch`
        // is marked `completed`: the trap handler finishes this call first
        // and then yields as a leaf.
        return crate::bdos::process::sched::dispatch_after_call();
    }
    sw
}

/// EmuTOS `dsptch()` — the dispatch every AES call ends with.
///
/// `super()`, the AES trap's supervisor entry, runs the opcode switch and then
/// falls through to `dsptch()` for **every** AES call (`aes/gemsuper.c:452-464`);
/// `dsptch` saves the machine state and jumps to `disp` (`aes/gemasm.S:129-167`),
/// and `disp` ends with
///
/// ```text
/// do {
///     if (fpcnt)
///         forker();
///     schedule();
/// } while (fpcnt);
/// ```
///
/// (`aes/gemdisp.c:236-242`). `forker()` drains the fork ring — the FPDs the
/// mouse/keyboard interrupt posted (`aes/gemdisp.c:101-155`) — so `bchange` and
/// `mchange` run *there* and publish the new `button` / `mclick` / `xrat` /
/// `yrat` (`aes/geminput.c:333-343`, `:360-373`), and `schedule()` additionally
/// polls the keyboard through `chkkbd()` (`aes/gemdisp.c:158-177,180-206`).
///
/// **Why this is load-bearing and not bookkeeping.** It is the only reason the
/// input state an application reads is not frozen for the duration of a polling
/// loop. `gr_mkstate` just hands out the globals — `*pmstat = button`
/// (`aes/gemgrlib.c:458-464`) — so an app that does nothing but call
/// `graf_mkstate` in a loop still observes the button change, because each of
/// its traps ends in this dispatch and the drain updates `button` on the way
/// out. TeraDesk depends on exactly that: after a **double** click it spins in
/// `while (xe_button_state()) ;` (`sdk/apps/teradesk/window.c:4406-4412`), and
/// `xe_button_state` is a bare `graf_mkstate`
/// (`sdk/apps/teradesk/library/xdialog/xdevent.c:251-258`).
///
/// **Our equivalent of the drain.** This kernel has no interrupt-driven
/// pointer: the IKBD / USB-HID packets are collected by `Host::pump_input`,
/// which reaches [`crate::vdi::feed_relative`] — our `mouse_int` — and from
/// there [`user_but`] / [`user_mot`], i.e. the `b_click` + `mchange` work
/// EmuTOS does in `forker()`, plus the keyboard poll `schedule()` does in
/// `chkkbd()`. One call is `disp`'s one pass through that loop. It is the same
/// call the blocking `evnt_*` waits already make; what was missing is that the
/// *non-blocking* opcodes made it too, which is what `super()`'s fall-through
/// guarantees in EmuTOS.
fn dsptch() {
    // `schedule()`'s keyboard poll, `chkkbd()` — first thing in every pass of
    // the scheduler loop (`aes/gemdisp.c:180-184`), and it precedes the read
    // because that is where `gsx_char` puts it.
    chkkbd();
    if let Some(h) = crate::host::host() {
        h.pump_input();
    }
}

/// EmuTOS `chkkbd()` (`aes/gemdisp.c:157-177`) — the keyboard poll the
/// scheduler runs on every dispatcher pass.
///
/// EmuTOS reads the key with `gsx_char()`, and `gsx_char` opens with
///
/// ```text
/// intin[0] = 4;
/// intin[1] = 2;
/// gsx_ncode(SET_INPUT_MODE, 0, 2);    /* vsin_mode(4, 2): string, SAMPLE */
/// ```
///
/// (`aes/gemgsxif.c:459-463`) before issuing the `STRING_INPUT`, because a
/// polling AES must not block inside the VDI: REQUEST mode makes `v_string`
/// wait for a key (`vdi/vdi_input.c:88-97`), SAMPLE makes it drain what is
/// pending and return (`:98-105`).
///
/// **Why the mode write belongs here and not only in the VDI.** The four
/// device modes are file-scope VDI globals, not per-workstation fields
/// (`vdi/vdi_defs.h:258`), so the AES's `vsin_mode(4, 2)` is visible to every
/// application through `vqin_mode(4)` — and nothing ever writes it back:
/// `vsin_mode` is the only writer (`vdi_input.c:118-142`), the AES's one
/// other call site sets the *locator* (`gsx_setmousexy`,
/// `aes/gemgsxif.c:484-488`, reached from `appl_tplay` alone), and the only
/// reset is `vdimouse_init` at `v_opnwk` (`vdi/vdi_control.c:563`,
/// `vdi/vdi_mouse.c:575-578`). On any EmuTOS machine `vqin_mode(4)` therefore
/// reports **1** from the first dispatcher pass onward, while devices 1, 2
/// and 3 stay at their default 0. Measured on the reference TT (EmuTOS 1.4):
/// `tosdump/reference/EMUTOS14-VDI-r2.BIN` record 16 and
/// `EMUTOS14-VDI-all.BIN` record 40 both read `INTOUT[0] = 1` for
/// `vqin_mode(4)`; records 6, 38 and 39 of the latter read 0 for devices 1,
/// 2 and 3.
///
/// **What of `chkkbd` this is, and what it is not.** The mode write is
/// reproduced; the key *read* is not routed through `v_string`. EmuTOS'
/// `chkkbd` pulls the key out of the VDI and forks it into the mouse owner's
/// per-process key queue (`forkq(kchange, …)`, `:172-176`); NewTOS' AES has
/// no such queue and leaves the key in the IKBD ring until an `evnt_keybd` /
/// form / dialog consumer takes it — the same delivery difference
/// [`input`](self::input) already documents for the button side. EmuTOS'
/// two guards on the read (`gl_play`, and the owner's queue being full) have
/// no counterpart for that reason; neither ever *clears* the mode, so they
/// can at most delay the first write, and on a normal boot both are false at
/// the first pass.
fn chkkbd() {
    crate::vdi::set_input_mode(crate::vdi::DEV_STRING, crate::vdi::MODE_SAMPLE);
}

/// The opcode switch — EmuTOS's `crysbind` (`aes/gemsuper.c:77-421`).
/// Results go into the dispatch-local `int_out`; the caller's array is
/// untouched until [`dispatch`] runs the copy-back.
fn crysbind(pb: &Aespb) -> Option<crate::bdos::process::sched::Switch> {
    let opcode = pb.control(C_OPCODE);

    // The opcodes whose body opens with `fm_own(TRUE)` → `wm_update(BEG_UPDATE)`
    // (`fm_do`, `aes/gemfmlib.c:287`; `fm_alert`, `aes/gemfmalt.c:361`, also
    // behind `fm_error` for a code it has an alert for, `gemfmlib.c:436-472`;
    // `mn_popup`'s `wm_update(BEG_MCTRL)`, `aes/gemmnext.c:312`) block there
    // while another process holds the window-tree lock (`ev_block(MU_MUTEX,
    // …)`, `aes/gemwmlib.c:1785-1786`). The wait comes before the opcode has
    // committed anything, so a yield re-executes the whole call (GH1).
    //
    // The same wait for the AES's other takers (A118): `ap_exit`'s own
    // `wm_update(BEG_UPDATE)` (`aes/gemaplib.c:253`); `gr_rubwind`/
    // `gr_dragbox` behind `graf_rubberbox`/`graf_dragbox`
    // (`aes/gemgrlib.c:205,250`); `fs_input`'s `fm_do` (`aes/gemfslib.c:718`,
    // after the NULL-path refusal, `:630-631`); and `wm_open`/`wm_close`/
    // `wm_delete`/`wm_set` for a call they do not refuse first
    // ([`window::opcode_takes_lock`]).
    let takes_screen = match opcode {
        19 | 36 | 50 | 52 | 70 | 71 => true,
        53 => pb.intin(0) <= 63,
        90 | 91 => pb.addrin(0) != 0,
        101 | 102 | 103 | 105 => window::opcode_takes_lock(opcode, pb),
        _ => false,
    };
    if takes_screen {
        if let Some(sw) = window::update_block() {
            return Some(sw);
        }
    }

    match opcode {
        10 => appl::appl_init(pb),
        19 => return appl::appl_exit(pb),
        // `appl_read` blocks on an empty queue like `evnt_mesag` (A75) — it
        // may yield to a peer too; propagate the `Switch`.
        11 => return appl::appl_read(pb),
        // `appl_write` blocks on a full queue until a read makes room (A116).
        12 => return appl::appl_write(pb),
        13 => appl::appl_find(pb),
        14 => appl::appl_tplay(pb),
        15 => appl::appl_trecord(pb),
        // `appl_yield` is a `dsptch()` of its own before `super()`'s (A112) —
        // it may yield to a peer mid-call.
        17 => return appl::appl_yield(pb),
        // The six blocking waits may yield to a peer — propagate the `Switch`.
        20 => return event::evnt_keybd(pb),
        21 => return event::evnt_button(pb),
        22 => return event::evnt_mouse(pb),
        23 => return event::evnt_mesag(pb),
        24 => return event::evnt_timer(pb),
        25 => return event::evnt_multi(pb),
        26 => event::evnt_dclick(pb),
        70 => graf::graf_rubberbox(pb),
        71 => graf::graf_dragbox(pb),
        72 => graf::graf_movebox(pb),
        73 => graf::graf_growbox(pb),
        74 => graf::graf_shrinkbox(pb),
        75 => graf::graf_watchbox(pb),
        76 => graf::graf_slidebox(pb),
        77 => graf::graf_handle(pb),
        78 => graf::graf_mouse(pb),
        79 => graf::graf_mkstate(pb),
        30 => menu::menu_bar(pb),
        31 => menu::menu_icheck(pb),
        32 => menu::menu_ienable(pb),
        33 => menu::menu_tnormal(pb),
        34 => menu::menu_text(pb),
        35 => menu::menu_register(pb),
        36 => menu::menu_popup(pb),
        37 => menu::menu_attach(pb),
        38 => menu::menu_istart(pb),
        39 => menu::menu_settings(pb),
        40 => object::objc_add(pb),
        41 => object::objc_delete(pb),
        42 => object::objc_draw(pb),
        43 => object::objc_find(pb),
        44 => object::objc_offset(pb),
        45 => object::objc_order(pb),
        46 => object::objc_edit(pb),
        47 => object::objc_change(pb),
        48 => object::objc_sysvar(pb),
        50 => form::form_do(pb),
        51 => form::form_dial(pb),
        52 => form::form_alert(pb),
        53 => form::form_error(pb),
        54 => form::form_center(pb),
        55 => form::form_keybd(pb),
        56 => form::form_button(pb),
        80 => shell::scrp_read(pb),
        81 => shell::scrp_write(pb),
        82 => shell::scrp_clear(pb),
        90 => fsel::fsel_input(pb),
        91 => fsel::fsel_exinput(pb),
        100 => window::wind_create(pb),
        101 => window::wind_open(pb),
        102 => window::wind_close(pb),
        103 => window::wind_delete(pb),
        104 => window::wind_get(pb),
        105 => window::wind_set(pb),
        106 => window::wind_find(pb),
        // `wind_update(BEG_UPDATE/BEG_MCTRL)` blocks while another process
        // holds the window-tree lock (GH1) — it may yield to a peer.
        107 => return window::wind_update(pb),
        108 => window::wind_calc(pb),
        109 => window::wind_new(pb),
        110 => resource::rsrc_load(pb),
        111 => resource::rsrc_free(pb),
        112 => resource::rsrc_gaddr(pb),
        113 => resource::rsrc_saddr(pb),
        114 => resource::rsrc_obfix(pb),
        120 => shell::shel_read(pb),
        121 => shell::shel_write(pb),
        122 => shell::shel_get(pb),
        123 => shell::shel_put(pb),
        124 => shell::shel_find(pb),
        125 => shell::shel_envrn(pb),
        126 => shell::shel_rdef(pb),
        127 => shell::shel_wdef(pb),
        // Any opcode not matched above — out of range, negative, or a
        // not-compiled-in function — returns the AES unsupported-opcode
        // sentinel `-1` in `int_out[0]` (`RET_CODE`). This mirrors EmuTOS
        // `crysbind`'s `default:` arm (`gemsuper.c:406-417`), which sets
        // `unsupported = TRUE` and then `ret = -1` unconditionally. Opcode 0
        // is included: EmuTOS returns `-1` for it too, only suppressing the
        // on-screen `fm_show(ALNOFUNC)` alert ("since some PRGs make this
        // call"). The alert is reference-level (spec `_dispatcher.md` §2/§3a,
        // D-084); the `-1` return is the contract, so we set it here.
        _ => pb.set_ret(-1),
    }
    // Non-blocking opcodes (and the six waits' non-yield exits) reach here.
    None
}

/// `sh_ldapp`'s `AP_OPEN` cleanup (`aes/gemshlib.c:650-657`) for the program
/// that has just terminated in GEMDOS slot `slot` — see `pd::close_if_open`.
/// Called from the program launcher at the point EmuTOS calls it: immediately
/// after `dos_exec(PE_LOADGO, …)` returns.
///
/// It drains the descriptor's message queue and clears `AP_OPEN`; it does
/// **not** free the descriptor. EmuTOS cannot free one (its `AESPD`s are fixed
/// array entries, `aes/gempd.c:32-35`), and NewTOS's are fixed entries per
/// slot likewise (A62). The slot's own release ([`release_client`]) has
/// already run by the time the launcher gets here, so on that path the
/// `AP_OPEN` guard finds nothing to do.
pub fn close_client_if_open(slot: usize) {
    // SAFETY: single-CPU launcher context; the AES pd table follows the
    // single-trap-path discipline and the launcher runs on that CPU with no
    // concurrent GEM trap in flight.
    unsafe { pd::close_if_open(slot) };
}

/// The AES side of a GEMDOS slot's release: the tenant of `slot` is gone, its
/// descriptor's flags are cleared and its queue drained (`pd::release`) so the
/// slot's next occupant starts clean — for every way a process ends,
/// including an EL0 `Pexec` child that died with `appl_init` unmatched
/// (A62 report §8). Called from `bdos::process::pterm::release_slot`, next to
/// the slot's other per-process side tables.
pub fn release_client(slot: usize) {
    // A desk accessory that ends (EmuTOS's never do, `docs/api/aes/
    // accessories.md` §4 point 6) leaves the accessory list here, so the
    // barriers stop waiting for it and its pid (`2 + position`) stops
    // resolving — the slot's next tenant is no accessory and must not inherit
    // that pid (A113). Its entry is emptied in place: the others keep their
    // pids. Its name goes too: left in the descriptor it would make
    // `appl_find` answer the chain's 0 for a dead accessory (or for whatever
    // `Pexec` child takes the slot next without being named).
    if acc::is_process(slot) {
        acc::forget_process(slot);
        // SAFETY: as `pd::release` below.
        unsafe { pd::clear_name(slot) };
    }
    // SAFETY: single-CPU process-release context under the BDOS giant lock;
    // no GEM trap is in flight for the slot being released.
    unsafe { pd::release(slot) };
    // A `wait_for_accs` the tenant was in the middle of dies with it: drop the
    // pass budget so the slot's next tenant gets the full `WAIT_TIMEOUT` and
    // performs the `AC_CLOSE` handshake instead of skipping it (ACC10).
    acc::forget_wait(slot);
    // Likewise an `evnt_multi`/`evnt_mouse` it had armed and an `appl_yield`
    // it was in the middle of (A112): the next tenant starts with neither.
    event::forget_wait(slot);
    appl::forget_yield(slot);
    // The Window-Manager half of the same release — `sh_ldapp`'s `wm_new()`
    // (`aes/gemshlib.c:676-677`) for a client the shell loop never ran. Only
    // *marked* here: this runs on the `Pterm` exception stack and the sweep
    // draws (A107). [`sweep_dead_clients`] executes it at the next dispatch.
    mark_for_sweep(slot);
}

/// The AES pid NewTOS **reports** for the program in GEMDOS process-table slot
/// `slot` — the value that program gets from `appl_init`
/// (`appl::reported_pid`): 0, the application chain's pid, for every chain
/// slot, and 2, 3, … for a desk accessory (and its `Pexec` descendants) in
/// load order (spec § "AES process identity and pid", contract (b), A56/A113).
/// A value, not an identity: the descriptor is keyed by `slot` itself (A62).
pub fn pid_of_slot(slot: usize) -> i16 {
    appl::reported_pid(slot)
}

/// Name a freshly-launched EL0 program's AES process descriptor from its launch
/// `path`, so a later `appl_find(name)` can locate it — the reimplementation of
/// EmuTOS naming a process at start (`pstart` → `p_nameit`, `aes/gempd.c:110-131`).
/// `slot` is the GEMDOS process-table slot the loader placed the program in —
/// the identity its descriptor is keyed by; the pid it reports is
/// [`pid_of_slot`]. Called by the kernel launcher before the program enters
/// EL0, alongside [`resource::set_app_dir`].
pub fn register_process(slot: usize, path: &[u8]) {
    // SAFETY: single-CPU launcher context; the AES pd table follows the
    // single-trap-path discipline and the launcher runs on that CPU with no
    // concurrent GEM trap in flight.
    unsafe { pd::register(slot, path) };
}

/// The AES's `user_but` vector — call it from the VDI mouse interrupt when the
/// button state changed. EmuTOS installs `far_bcha` here with
/// `gsx_setmb(far_bcha, far_mcha, &drwaddr)` (`aes/gemgsxif.c:271`, i.e.
/// `vex_butv`/`vex_motv`), and `far_bcha` calls `b_click`
/// (`aes/gemdosif.S:306-325`). `buttons` is the new mask in VDI convention
/// (bit 0 = left), the `d0` the VDI passes (`vdi/vdi_asm.S:270-284`).
pub fn user_but(buttons: u8) {
    input::b_click(buttons);
}

/// The AES's `user_mot` vector — call it from the VDI mouse interrupt for a
/// packet that carried movement, with the **clipped** new position
/// (`vdi/vdi_asm.S:355-357`). EmuTOS's `far_mcha` forks `mchange`
/// (`aes/gemdosif.S:327-347`).
pub fn user_mot(x: i16, y: i16) {
    input::mchange(x, y);
}

/// The AES's `user_wheel` vector — EmuTOS `aes_wheel`
/// (`aes/gemdosif.S:352-374`), installed by `gsx_init` with
/// `vex_wheelv(aes_wheel, &old_wheelv)` (`aes/gemgsxif.c:288-292`) and called
/// by the VDI whenever the mouse wheel moves, with the wheel number in `d0.w`
/// and the signed detent count in `d1.w`.
///
/// EmuTOS's `aes_wheel` is pure m68k glue: switch to the AES's interrupt
/// stack, push the two WORDs, `forkq(_wheel_change, …)` so the work happens
/// on the AES side of the interrupt, restore. The fork queue is an m68k-ISR
/// deferral with no counterpart here — NewTOS reaches this function from the
/// cooperative `pump_input`, already on the AES's own side — so what remains
/// of `aes_wheel` is the call it defers: [`input::wheel_change`]. (INPUT01's
/// timer-interrupt pump does not decode wheel detents; they are held for the
/// trap-side pump — `kernel::input_gate`.)
pub fn aes_wheel(wheel_number: i16, wheel_amount: i16) {
    input::wheel_change(wheel_number, wheel_amount);
}
