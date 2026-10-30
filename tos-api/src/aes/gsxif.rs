//! The AES's VDI workstation (`gl_handle`), its keyboard-state source
//! ([`gsx_kstate`], A126) and its one screen-save buffer — the pieces of
//! EmuTOS `aes/gemgsxif.c` NewTOS keeps.
//!
//! # The workstation
//!
//! [`gsx_init`] is EmuTOS `gsx_init`'s workstation half (`:267-270`):
//! `gsx_wsopen` opens the **physical** workstation (`v_opnwk`, `:169-186`)
//! and keeps the handle in `gl_handle`, `gsx_start` (`gemgraf.c:383-428`)
//! sets it up. [`graf_handle`](super::graf::graf_handle) reports that handle
//! (`gemsuper.c:287-293`), so every VDI call an application makes on it is
//! answered (`vq_extnd`'s `DEV_TAB`/`INQ_TAB`), and the first `v_opnvwk`
//! gets handle 2 (`vdi_control.c:420`). [`gsx_wsclose`] is the session end's
//! `v_clswk` (`:191-194`, `geminit.c:749`). Spec:
//! `docs/api/aes/window-graphics.md` § `graf_handle`.
//!
//! # The screen-save buffer
//!
//! EmuTOS `aes/gemgsxif.c`'s `gl_tmp` /
//! `gl_mlen` and the calls around it: `gsx_malloc` (`:108-114`), `gsx_mret`
//! (`:125-129`), `bb_save`/`bb_restore` (`:383-392` → `bb_set`, `:320-380`)
//! — plus the **transient** per-opening block `popup_blit`
//! (`aes/gemmnext.c:136-215`) takes for a pop-up or a nested submenu.
//!
//! EmuTOS keeps exactly **one** buffer for the alert (`fm_alert`,
//! `gemfmalt.c:363,390`) and the drop-down (`menu_sr`, `gemmnlib.c:208-223`),
//! takes it once when the AES goes graphic (`sh_tographic`,
//! `gemshlib.c:236`) from the GEMDOS pool (`dos_alloc_anyram` =
//! `Mxalloc(n, MX_PREFTTRAM)`, `util/gemdos.c:117-120`), and hands its
//! address and length to any program through `wind_get(WF_SCREEN)`. A
//! pop-up or a submenu does **not** use it — the submenu opens *while* the
//! drop-down is down and holds the buffer — but takes a fresh ST-RAM block
//! per opening (`dos_alloc_stram`, `gemmnext.c:155-163`) and frees it on
//! the restore (`:212`).
//!
//! # NewTOS
//!
//! * **Pool.** The buffer is a kernel-owned block of the BDOS ST-/TT-RAM
//!   pool — [`gsx_malloc`] calls the same `Mxalloc(…, MX_PREFTTRAM)` EmuTOS
//!   does, one page over so the page-rounded base is exclusively ours, and
//!   runs before the first process, so the block carries the kernel owner
//!   cookie that every `Pterm` sweep skips (`bdos/mem/traps.rs`,
//!   `KERNEL_PD_SENTINEL`). The kernel reaches it through the high-half
//!   alias `pa + VA_OFFSET`, as `rsrc_load` reaches its blob.
//! * **Size.** One screen in device pixels — `bb_save_words(W, H) * 4`
//!   ([`gsx_mcalc`]), not EmuTOS' `MENU_BUFFER_SIZE * gl_wchar * gl_hchar *
//!   gl_nplanes / 8` (`gemgsxif.c:98-104`). Both users clamp their rectangle
//!   to the screen, so `bb_set`'s "buffer too small, adjust height to fit:
//!   this will leave droppings" path (`:334-343`) is structurally
//!   unreachable; the guard in [`bb_save`] stays as a defence and counts
//!   ([`clipped_saves`]). Named deviation, `docs/COMPATIBILITY.md`.
//! * **Lifetime.** Never freed. EmuTOS frees it in `sh_toalpha`
//!   (`gemshlib.c:258`) and takes it again in the next `sh_tographic`;
//!   between two GEM programs it persists there too. Keeping it across an
//!   alpha interlude is the safe superset and unobservable (a program can
//!   only tell by using a freed block).
//! * **EL0.** `wind_get(WF_SCREEN)` hands an EL0 caller the block's own
//!   address: a pool block, in every process's address space at VA = PA
//!   since ARCH03 stage 1 (`usermem::expose`, `PROT_G`, ADR-023) — see
//!   `window.rs`.
//! * **Without the buffer** (pool exhausted at boot, or a host test without
//!   the [`install_screen_buffer`] fixture) the alert and the drop-down fall
//!   back to a kernel-heap copy per save — the pre-A57 behaviour — and
//!   `WF_SCREEN` answers FALSE (`docs/COMPATIBILITY.md`, A54).

use core::sync::atomic::{AtomicBool, AtomicI16, AtomicU32, AtomicU64, AtomicUsize, Ordering};

use tos_types::{Long, MX_PREFTTRAM, MX_STRAM};

use crate::bdos::process::usermem::PAGE_SIZE;
use crate::vdi::backend::{ClipRect, RasterBackend};

// ── the workstation: `gl_handle` ────────────────────────────────────────

/// EmuTOS `gl_handle` (`gemgraf.c:52`): the physical workstation handle the
/// AES's `v_opnwk` got back. A zeroed global until the first [`gsx_init`],
/// as EmuTOS's BSS one is.
static GL_HANDLE: AtomicI16 = AtomicI16::new(0);

/// `gl_handle` — what `graf_handle` returns in `int_out[0]`
/// (`gemsuper.c:292`).
pub fn gl_handle() -> i16 {
    GL_HANDLE.load(Ordering::Relaxed)
}

// VDI opcodes the AES issues on its workstation (`aes/gemgsxif.h`,
// `vdi/vdi_main.c` jump table).
const OPEN_WORKSTATION: i16 = 1;
const CLOSE_WORKSTATION: i16 = 2;
const ESCAPE_FUNCTION: i16 = 5;
const SET_CHAR_HEIGHT: i16 = 12;
const SET_LINE_TYPE: i16 = 15;
const SET_LINE_WIDTH: i16 = 16;
const INQ_TEXT_ATTRIBUTES: i16 = 38;
const SET_UD_LINE_STYLE: i16 = 113;

/// `FALCON_REZ` (`include/biosdefs.h:127`): `screen_rez()` on a VIDEL
/// machine (`gemgsxif.c:160-166`), the Falcon030/TOS4 target profile.
const FALCON_REZ: i16 = 3;

/// The AES's VDI parameter arrays (EmuTOS's file-scope `contrl`/`intin`/
/// `ptsin`/`intout`/`ptsout`), on the stack of the one caller. `intout` is
/// sized like EmuTOS's `WS` (`gemgsxif.h`): 45 `INTOUT` words, `v_opnwk`'s
/// `PTSOUT` landing in separate `ptsout` here.
struct Gsx {
    contrl: [i16; 12],
    intin: [i16; 16],
    ptsin: [i16; 8],
    intout: [i16; 57],
    ptsout: [i16; 12],
}

impl Gsx {
    fn new() -> Self {
        Self { contrl: [0; 12], intin: [0; 16], ptsin: [0; 8], intout: [0; 57], ptsout: [0; 12] }
    }

    /// `gsx_ncode(code, n, m)` (`gemgsxif.c:133-140`): opcode, `PTSIN` and
    /// `INTIN` counts, `contrl[6] = gl_handle`, then the VDI trap (`gsx2`).
    /// The AES is EL1 code here, so the trap is the VDI dispatcher called
    /// with a kernel-space LP64 parameter block — what the trap handler
    /// itself calls (`kernel/src/arch/aarch64/exceptions.rs`, selector 115).
    fn ncode(&mut self, code: i16, n: i16, m: i16) {
        self.contrl = [0; 12];
        self.contrl[0] = code;
        self.contrl[1] = n;
        self.contrl[3] = m;
        self.contrl[6] = gl_handle();
        self.trap();
    }

    /// `gsx2()`: the VDI trap on the arrays as they stand.
    fn trap(&mut self) {
        let block: [u64; 5] = [
            self.contrl.as_mut_ptr() as u64,
            self.intin.as_mut_ptr() as u64,
            self.ptsin.as_mut_ptr() as u64,
            self.intout.as_mut_ptr() as u64,
            self.ptsout.as_mut_ptr() as u64,
        ];
        // SAFETY: a kernel-space LP64 parameter block whose five arrays are
        // fields of `self` and outlive the call; every VDI handler bounds
        // its accesses by the counts this block carries (≤ 57/12 words out).
        let _flip_y = unsafe { crate::vdi::dispatch(block.as_ptr() as u64, crate::bdos::process::Abi::Lp64) };
    }

    /// `contrl[5] = sub; gsx_0code(code)` (`gemgsxif.c:305-306,311-312`):
    /// a sub-function opcode with no inputs.
    fn ncode_sub(&mut self, code: i16, sub: i16) {
        self.contrl = [0; 12];
        self.contrl[0] = code;
        self.contrl[5] = sub;
        self.contrl[6] = gl_handle();
        self.trap();
    }

    /// `gsx_1code(code, value)` (`gemgsxif.c:151-155`).
    fn code1(&mut self, code: i16, value: i16) {
        self.intin[0] = value;
        self.ncode(code, 0, 1);
    }

    /// `vst_height(height, …)` (`gemgsxif.c:590-600`).
    fn vst_height(&mut self, height: i16) {
        self.ptsin[0] = 0;
        self.ptsin[1] = height;
        self.ncode(SET_CHAR_HEIGHT, 1, 0);
    }
}

/// `gsx_init`'s workstation half (`gemgsxif.c:267-270`): `gsx_wsopen` then
/// `gsx_start`. Run by the kernel's AES session loop at the start of every
/// session, before the accessories load (`run_accs_and_desktop`,
/// `geminit.c:675`).
///
/// **`gsx_wsopen`** (`:169-186`): `work_in[0] = screen_rez() + 2`
/// (`FALCON_REZ + 2` on the VIDEL profile), `work_in[1..9] = 1`,
/// `work_in[10] = 2` (raster coordinates), `v_opnwk(intin, &gl_handle,
/// &gl_ws)`. EmuTOS also pre-loads `intout[45] = VsetMode(-1)` — the Videl
/// mode `v_opnwk` reads back on the Falcon path (`vdi_control.c:517-525`);
/// NewTOS has no `VsetMode` and its `v_opnwk` switches no mode
/// (`docs/api/vdi/workstation/01-v_opnwk.md`), so nothing is passed there.
///
/// **`gsx_start`** (`gemgraf.c:383-428`), its VDI calls on `gl_handle`:
/// `gsx_textsize` (`vqt_attributes`, `gemgsxif.c:505-512`) for the current
/// character height, `vst_height(ws_chminh)` — `ws_chminh` is `SIZ_TAB[1]`
/// from the open, `gl_ws`'s `PTSOUT[1]` — and `vst_height` back to the
/// current height (`:405-410`), then `vsl_type(7)`, `vsl_width(1)`,
/// `vsl_udsty(0xffff)` (`:420-422`). Its global variables (`gl_width`,
/// `gl_wchar` … `gl_rmenu`, `gl_nplanes`) are derived in this AES from the
/// same backend and font ring the VDI answers from (`graf.rs`, `window.rs`),
/// so they are not read back through the trap here.
///
/// The rest of `gsx_init` is elsewhere: `gsx_setmb` is the VDI mouse feed
/// calling [`super::user_but`]/[`super::user_mot`] directly, `vex_wheelv`
/// is `init::install_wheel_vector`, and `MOUSE_STATE`'s `xrat`/`yrat` seed
/// is the input module's own state.
///
/// Without a raster backend the VDI returns before any handler
/// (`tos-api/src/vdi/mod.rs`), `contrl[6]` stays the input `gl_handle`, and
/// no workstation is open — the same "draw nothing" state such a board is
/// in everywhere else.
pub fn gsx_init() {
    let mut g = Gsx::new();

    // gsx_wsopen
    g.intin[0] = FALCON_REZ + 2;
    for slot in g.intin[1..10].iter_mut() {
        *slot = 1;
    }
    g.intin[10] = 2; // device coordinate space
    g.ncode(OPEN_WORKSTATION, 0, 11);
    GL_HANDLE.store(g.contrl[6], Ordering::Relaxed);
    GL_GRAPHIC.store(true, Ordering::Relaxed); // `gsx_wsopen` (`:186`)
    let ws_chminh = g.ptsout[1]; // gl_ws.ws_chminh = SIZ_TAB[1]

    // gsx_start: the two text-height calls (`gemgraf.c:405-410`)
    g.ncode(INQ_TEXT_ATTRIBUTES, 0, 0); // gsx_textsize
    let ws_chmaxh = g.ptsout[1]; // gl_hptschar, saved as ws_chmaxh
    g.vst_height(ws_chminh);
    g.vst_height(ws_chmaxh); // "must restore the current font height"

    // gsx_start: the line set-up (`gemgraf.c:420-422`)
    g.code1(SET_LINE_TYPE, 7);
    g.ptsin[0] = 1;
    g.ptsin[1] = 0;
    g.ncode(SET_LINE_WIDTH, 1, 0); // vsl_width(1), `gemgsxif.c:653-658`
    g.code1(SET_UD_LINE_STYLE, -1); // 0xffff
}

/// `gsx_wsclose` (`gemgsxif.c:191-194`): `v_clswk` on `gl_handle`, run by
/// the kernel's AES session loop when the session ends (`geminit.c:749`).
/// The VDI closes every virtual workstation and keeps the physical one
/// (`vdi_control.c:573-590`); `gl_handle` keeps its value, as EmuTOS's
/// does, and the next session's [`gsx_init`] opens it again.
pub fn gsx_wsclose() {
    Gsx::new().ncode(CLOSE_WORKSTATION, 0, 0);
}

/// EmuTOS `gl_graphic` (`gemgsxif.c:84`): whether the screen is in graphics
/// mode. `gsx_wsopen` sets it (`:186`), [`gsx_graphic`] switches it.
static GL_GRAPHIC: AtomicBool = AtomicBool::new(false);

/// `gsx_graphic(tographic)` (`gemgsxif.c:298-318`), called by `sh_tographic`
/// / `sh_toalpha` (`gemshlib.c:234,259`): on a change, the VDI escape
/// `v_exit_cur` (`CONTRL[5] = 2`: home, text cursor off, clear) going to
/// graphics, `v_enter_cur` (`CONTRL[5] = 3`: clear, text cursor on, CR)
/// going to alpha mode — on `gl_handle`, so the VT52 console is the one the
/// character program then writes to (CON04). The `gsx_setmb` half
/// (`:307,313`) has no counterpart: the VDI mouse feed calls the AES
/// handlers directly (see [`gsx_init`]).
pub fn gsx_graphic(tographic: bool) {
    if GL_GRAPHIC.swap(tographic, Ordering::Relaxed) == tographic {
        return;
    }
    let mut g = Gsx::new();
    g.ncode_sub(ESCAPE_FUNCTION, if tographic { 2 } else { 3 });
}

// ── the keyboard state: `gsx_kstate` ─────────────────────────────────────

/// The bits of the BIOS shift byte the AES hands to a program: right Shift,
/// left Shift, Control, Alternate (`K_RSHIFT`/`K_LSHIFT`/`K_CTRL`/`K_ALT`).
/// EmuTOS' `gshift_s` is `Kbshift(-1) & 0x000f` (`vdi/vdi_input.c:183-187`);
/// CapsLock (bit 4), `MODE_HOME`/`MODE_INSERT` (bits 5-6) and a
/// program-stored bit 7 never reach the AES (A126).
const KSTATE_MASK: u8 = 0x0f;

/// EmuTOS `gsx_kstate` (`aes/gemgsxif.c:432-436`) — the AES's one source of
/// the keyboard state: `gsx_0code(KEY_STATE)` is the VDI's `vq_key_s` on
/// `gl_handle` (`vdi/vdi_input.c:110-113`), whose answer is `gshift_s()` =
/// `Kbshift(-1) & 0x000f` (`:183-187`). `chkkbd` stores it into `kstate`
/// (`aes/gemdisp.c:165,175` → `kchange`, `aes/geminput.c:288-293`), and every
/// value the AES reports reads that global: `ev_rets`' `rets[3]`
/// (`aes/gemevlib.c:56`: `evnt_button`, `evnt_mouse`, `evnt_multi`),
/// `gr_mkstate` (`aes/gemgrlib.c:463`: `graf_mkstate`) and `handle_popup`'s
/// `keystate = rets[3]` (`aes/gemmnext.c:272`: `menu_popup`'s
/// `mn_keystate`).
///
/// NewTOS reads the byte `vq_key_s` reads (`Host::keyboard_shift_state`,
/// the BIOS `Kbshift(-1)` byte) and applies the same mask, without a trap
/// into its own VDI: the VDI answers nothing before a workstation is open
/// or without a raster backend, and the AES's state report must not depend
/// on either. The value equals `vq_key_s`'s (`crate::vdi::inquiry`).
/// 0 without a host, as `keyboard_shift_state`'s keyboard-less default.
pub(super) fn gsx_kstate() -> i16 {
    crate::host::host().map_or(0, kstate_of)
}

/// [`gsx_kstate`] for a caller that already holds the host (`menu_popup`'s
/// tracking loop is handed one).
pub(super) fn kstate_of(h: &dyn crate::host::Host) -> i16 {
    (h.keyboard_shift_state() & KSTATE_MASK) as i16
}

// ── the screen-save buffer ────────────────────────────────────────────────

/// Kernel high-half offset (mirror of `process::{arena,loader,usermem}` and
/// `aes::resource`): a BDOS-pool physical address `pa` is reachable from the
/// kernel at `pa + VA_OFFSET`.
const VA_OFFSET: u64 = 0xFFFF_0000_0000_0000;

/// What `gsx_mret` reports: the buffer's kernel alias, its physical (pool)
/// address and its length in bytes (`gl_tmp.fd_addr` / `gl_mlen`).
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct ScreenBuffer {
    /// Kernel high-half VA of the (page-aligned) buffer.
    pub kva: u64,
    /// Physical address of the buffer — the pool address, what an EL1 caller
    /// of `wind_get(WF_SCREEN)` receives (ADR-021, raw PAs for kernel
    /// callers).
    pub pa: u64,
    /// Length in bytes (`gl_mlen`).
    pub len: usize,
}

// `gl_tmp.fd_addr` / `gl_mlen`. `BUF_LEN != 0` is "the buffer exists"; it is
// stored last (Release) and read first (Acquire), so a reader that sees the
// length sees the two addresses that go with it.
static BUF_KVA: AtomicU64 = AtomicU64::new(0);
static BUF_PA: AtomicU64 = AtomicU64::new(0);
static BUF_LEN: AtomicUsize = AtomicUsize::new(0);

/// How many times [`bb_save`] had to clip its height to the buffer —
/// EmuTOS' `KINFO("Menu/alert buffer too small …")` (`gemgsxif.c:338-342`)
/// as a counter, since `tos-api` has no serial log of its own. Zero by
/// construction at the screen-sized length.
static CLIPPED: AtomicU32 = AtomicU32::new(0);

#[inline]
fn page_round_up(v: u64) -> u64 {
    (v + PAGE_SIZE - 1) & !(PAGE_SIZE - 1)
}

/// `w`/`h` of the backend as the `i16` the raster ABI carries.
#[inline]
fn dim(v: u16) -> i16 {
    v.min(i16::MAX as u16) as i16
}

/// `gsx_mcalc` (`gemgsxif.c:98-104`), at NewTOS' size: the bytes one
/// `bb_save` of the **whole screen** needs — `stride16(W) * H * 2`, i.e.
/// [`crate::vdi::bb_save_words`]`(W, H) * 4` (640×480 → 614 400,
/// 1920×1080 → 4 147 200). Every rectangle either user saves is clamped to
/// the screen, so nothing can exceed it.
pub fn gsx_mcalc(backend: &dyn RasterBackend) -> usize {
    crate::vdi::bb_save_words(dim(backend.width()), dim(backend.height())) * 4
}

/// `gsx_malloc` (`gemgsxif.c:108-114`): take the buffer from the GEMDOS pool
/// (`dos_alloc_anyram` = `Mxalloc(len, MX_PREFTTRAM)`), once. Called from
/// `sh_tographic` (`gemshlib.c:236`); a second call returns the buffer
/// already taken (EmuTOS would take another, but only after `gsx_mfree`,
/// which NewTOS never does — module docs).
///
/// Must run **outside any process**: the block has to carry the kernel
/// owner cookie so no `Pterm` reclaims it (`bdos/mem/traps.rs`
/// `current_owner`). With a process running the call takes nothing and
/// answers `None` rather than mint a buffer that dies with that process.
///
/// `None` when there is no raster backend, no pool, or the pool is short
/// (`ENSMEM`); the AES then runs on the per-save fallback (module docs) and
/// `WF_SCREEN` answers FALSE. The kernel prints the outcome on its boot
/// log.
pub fn gsx_malloc() -> Option<ScreenBuffer> {
    if let Some(b) = gsx_mret() {
        return Some(b);
    }
    let backend = crate::host::host()?.raster()?;
    let len = gsx_mcalc(backend);
    if len == 0 {
        return None;
    }
    let running = crate::bdos::process::current();
    debug_assert!(running.is_null(), "gsx_malloc must run before the first process (kernel-owned block)");
    if !running.is_null() || !crate::bdos::mem::init::is_initialised() {
        return None;
    }
    // Page-round and over-allocate one page so the page-aligned base fits
    // inside the block (`rsrc_load`, `arena::alloc_tpa`): the pages the
    // buffer occupies then belong to nothing else, which an EL0 mapping
    // needs (ADR-023).
    let pages = page_round_up(len as u64);
    let want = pages + PAGE_SIZE;
    if want > Long::MAX as u64 {
        return None;
    }
    // SAFETY: boot CPU between the AUTO folder and the first GEM program —
    // no trap is in flight, the implicit BDOS lock is ours.
    let raw = unsafe { crate::bdos::mem::traps::mxalloc(want as Long, MX_PREFTTRAM) };
    if raw == 0 {
        return None;
    }
    let pa = page_round_up(raw as u64);
    let kva = pa.wrapping_add(VA_OFFSET);
    BUF_KVA.store(kva, Ordering::Relaxed);
    BUF_PA.store(pa, Ordering::Relaxed);
    BUF_LEN.store(len, Ordering::Release);
    Some(ScreenBuffer { kva, pa, len })
}

/// `gsx_mret` (`gemgsxif.c:125-129`): the buffer, or `None` while there is
/// none.
pub fn gsx_mret() -> Option<ScreenBuffer> {
    let len = BUF_LEN.load(Ordering::Acquire);
    if len == 0 {
        return None;
    }
    Some(ScreenBuffer {
        kva: BUF_KVA.load(Ordering::Relaxed),
        pa: BUF_PA.load(Ordering::Relaxed),
        len,
    })
}

/// Host-test fixture — the counterpart of `host::install_host`: hand the
/// AES a buffer that did not come from the pool (a host process has no BDOS
/// pool, and a host `Vec` does not lie under 2 GiB, so `gsx_malloc` cannot
/// run there). `pa` is what an EL1 `wind_get(WF_SCREEN)` will report.
///
/// # Safety
///
/// `ptr .. ptr + len` must stay valid and writable for the rest of the
/// process; `len` must be at least [`gsx_mcalc`] of the installed backend if
/// the [`bb_save`] guard is not the thing under test.
pub unsafe fn install_screen_buffer(ptr: *mut u8, pa: u64, len: usize) {
    BUF_KVA.store(ptr as u64, Ordering::Relaxed);
    BUF_PA.store(pa, Ordering::Relaxed);
    BUF_LEN.store(len, Ordering::Release);
}

/// How often [`bb_save`] clipped a rectangle's height to the buffer
/// (`gemgsxif.c:334-343`). Diagnostics only.
pub fn clipped_saves() -> u32 {
    CLIPPED.load(Ordering::Relaxed)
}

/// Where a [`Saved`] rectangle's pixels are.
enum Backing {
    /// The one screen buffer (`gl_tmp`): shared, not owned — a later
    /// [`bb_save`] overwrites it, as on EmuTOS.
    Screen,
    /// A transient pool block (`popup_blit`'s `dos_alloc_stram`), owned by
    /// the process that opened the pop-up / submenu and freed with the
    /// restore (`gemmnext.c:212`) — or with the drop, if the restore never
    /// comes.
    Pool { raw: Long, kva: u64 },
    /// No screen buffer exists: a kernel-heap copy per save, the pre-A57
    /// alert path, so the alert and the drop-down still put the screen back.
    Heap(alloc::vec::Vec<u32>),
}

/// A screen rectangle [`bb_save`] or [`popup_blit_save`] stored, to be put
/// back by [`Saved::restore`]. Carries the (possibly height-clipped)
/// rectangle the pixels belong to.
pub struct Saved {
    x: i16,
    y: i16,
    w: i16,
    h: i16,
    backing: Backing,
}

impl Saved {
    /// The rectangle whose pixels are held: `(x, y, w, h)`.
    pub fn rect(&self) -> (i16, i16, i16, i16) {
        (self.x, self.y, self.w, self.h)
    }

    fn mem(&self) -> *const u32 {
        match &self.backing {
            Backing::Screen => BUF_KVA.load(Ordering::Relaxed) as *const u32,
            Backing::Pool { kva, .. } => *kva as *const u32,
            Backing::Heap(v) => v.as_ptr(),
        }
    }

    /// `bb_restore` (`gemgsxif.c:389-392`) / `popup_blit(tree, buf)`
    /// (`gemmnext.c:183-214`): copy the saved pixels back to their
    /// rectangle, cursor hidden for the copy (`bb_set`,
    /// `gemgsxif.c:367,378`). A pool block is released afterwards
    /// (`dos_free`, `:212`).
    pub fn restore(self, backend: &dyn RasterBackend) {
        let clip = ClipRect::full(backend.width(), backend.height());
        // SAFETY: `mem()` addresses `bb_save_words(w, h)` `u32` words the
        // matching save filled — the screen buffer (guard in `bb_save`), a
        // pool block sized for exactly this rectangle, or the heap `Vec`.
        unsafe {
            crate::vdi::bb_restore(backend, self.mem(), self.x, self.y, self.w, self.h, clip);
        }
        // `self` drops here: a pool block is freed by `Drop`.
    }
}

impl Drop for Saved {
    fn drop(&mut self) {
        if let Backing::Pool { raw, .. } = self.backing {
            // SAFETY: `raw` is the live pool block `popup_blit_save` took and
            // nothing else references it once the `Saved` goes.
            let _ = unsafe { crate::bdos::mem::traps::mfree(raw) };
        }
    }
}

/// `bb_save` (`gemgsxif.c:383-386` → `bb_set(TRUE, r)`): copy the screen
/// rectangle `(x, y, w, h)` — already clamped to the screen by the caller,
/// as `fm_alert`'s `rc_intersect(&gl_rscreen, &d)` (`gemfmalt.c:358`) and
/// `menu_sr`'s on-screen box do — into **the** screen buffer. `None` for an
/// empty rectangle.
///
/// If the rectangle does not fit the buffer, the height is clipped to what
/// fits, `sh = gl_mlen * sh / size` (`:336`), and the event counted
/// ([`clipped_saves`]); at the screen-sized length that cannot happen. With
/// no screen buffer at all the pixels go to a kernel-heap copy instead
/// (module docs).
pub fn bb_save(backend: &dyn RasterBackend, x: i16, y: i16, w: i16, h: i16) -> Option<Saved> {
    if w <= 0 || h <= 0 {
        return None;
    }
    let mut h = h;
    let backing = match gsx_mret() {
        Some(buf) => {
            let need = crate::vdi::bb_save_words(w, h) * 4;
            if need > buf.len {
                // `bb_set`: "adjust height to fit buffer: this will leave
                // droppings!" (`gemgsxif.c:334-337`).
                h = ((buf.len as u64 * h as u64) / need as u64) as i16;
                CLIPPED.fetch_add(1, Ordering::Relaxed);
                if h <= 0 {
                    return None;
                }
            }
            // SAFETY: `kva` addresses `len >= bb_save_words(w, h) * 4` bytes
            // of the buffer, which lives for the rest of the boot.
            unsafe { crate::vdi::bb_save(backend, buf.kva as *mut u32, x, y, w, h) };
            Backing::Screen
        }
        None => {
            let words = crate::vdi::bb_save_words(w, h);
            let mut v = alloc::vec![0u32; words];
            // SAFETY: `v` holds `bb_save_words(w, h)` `u32` words.
            unsafe { crate::vdi::bb_save(backend, v.as_mut_ptr(), x, y, w, h) };
            Backing::Heap(v)
        }
    };
    Some(Saved { x, y, w, h, backing })
}

/// `popup_blit(tree, NULL)` (`gemmnext.c:136-215`, the save direction):
/// copy the screen rectangle `(x, y, w, h)` — the caller's `popup_blit`
/// rectangle, clamped to the screen — into a **fresh ST-RAM pool block**
/// (`dos_alloc_stram(bufsize)`, `:155-163`; `bufsize = ((w + 15) / 16) *
/// 2 * h * gl_nplanes` = [`crate::vdi::bb_save_words`]`(w, h) * 4`). The
/// block belongs to the running process and goes with the restore.
///
/// `None` when the rectangle is empty or the pool cannot supply the block
/// (`buf == NULL`): the pop-up is then not shown at all (`mn_popup`,
/// `:329-334`, D-086 in `docs/api/aes/menu.md`), the submenu not displayed
/// (`display_submenu`, `:668-671`).
///
/// With **no pool at all** — a host test binary; on the board the BDOS pools
/// are up before the first AES call (`kernel/src/lib.rs` `bring_up_bdos_pools`
/// precedes the AUTO folder and the desktop) — the pixels go to a kernel-heap
/// copy, exactly as [`bb_save`] does without a screen buffer (`Backing::Heap`,
/// module docs). That keeps the D-086 "pool short → no pop-up/submenu"
/// contract intact where a pool exists, and lets the drop-down tracker's
/// submenu states (`mn_do` `SUBMENU_STATE`/`OUTSIDE_STATE`, A89) run in a
/// host test. A host `Vec` cannot stand in for a pool block: `mxalloc` hands
/// out `Long` pool keys the AES re-derives through the kernel high-half alias.
pub fn popup_blit_save(backend: &dyn RasterBackend, x: i16, y: i16, w: i16, h: i16) -> Option<Saved> {
    if w <= 0 || h <= 0 {
        return None;
    }
    if !crate::bdos::mem::init::is_initialised() {
        let words = crate::vdi::bb_save_words(w, h);
        let mut v = alloc::vec![0u32; words];
        // SAFETY: `v` holds `bb_save_words(w, h)` `u32` words.
        unsafe { crate::vdi::bb_save(backend, v.as_mut_ptr(), x, y, w, h) };
        return Some(Saved { x, y, w, h, backing: Backing::Heap(v) });
    }
    let bytes = crate::vdi::bb_save_words(w, h) * 4;
    if bytes == 0 || bytes > Long::MAX as usize {
        return None;
    }
    // SAFETY: single-CPU AES path (trap context or the shell loop's
    // `sh_tographic`), the
    // implicit BDOS lock is held.
    let raw = unsafe { crate::bdos::mem::traps::mxalloc(bytes as Long, MX_STRAM) };
    if raw == 0 {
        return None;
    }
    let kva = (raw as u64).wrapping_add(VA_OFFSET);
    // SAFETY: `kva` aliases `bytes` live RW pool bytes of the block just
    // taken, exactly `bb_save_words(w, h)` `u32` words.
    unsafe { crate::vdi::bb_save(backend, kva as *mut u32, x, y, w, h) };
    Some(Saved { x, y, w, h, backing: Backing::Pool { raw, kva } })
}
