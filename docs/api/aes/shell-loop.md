# AES Shell Loop and Process Identity (`gem_main` → `sh_main` → `sh_ldapp`)

> **Stage S0 of `docs/analysis/desktop-userspace-plan.md`.** Closes the spec
> gap that document names: the process-identity semantics of `sh_ldapp` and
> the shell loop it runs in were nowhere in `docs/api/` before this file.
> Source: EmuTOS @ `1a7484ab` (`references/emutos/`, pinned commit verified
> at analysis time). Primary files: `aes/geminit.c`, `aes/gemshlib.c`,
> `aes/gempd.c`, `aes/gemsuper.c`, `aes/gem_rsc.c`, `aes/struct.h`.
> Cross-reference: `docs/api/emutos-boot-flow.md` §4 (the same loop, at
> boot-flow granularity, without the process-identity detail this file adds);
> `docs/api/aes/scrap-shell.md` (`shel_*`/`scrp_*` **opcode** contract — this
> file documents what happens on the far side of `shel_write`, not the
> opcode ABI itself, which is unchanged).
>
> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md)
.
> Each section below is marked. Short version: the *numbers and values* an
> application can read back (`ap_id`, `ap_private`, the effect of
> `shel_write`/`shel_find`, the window/alpha-graphic state a launched program
> observes at startup) are **contract**. The *mechanism EmuTOS uses to
> produce them* (a nested `Pexec`/`dos_exec` running a ROM function pointer,
> the `SHELL sh` struct, the specific control-flow shape of `sh_main`) is
> **reference** — NewTOS may implement the mechanism differently as long as
> the observable numbers/behaviour match.
>
> **Correction to the previous framing.** `docs/api/aes/scrap-shell.md:48-53`
> and `:494-503` state that "the `SHELL sh` struct, `sh_main`'s
> desktop/console/app launch loop, … `aes_run_rom_program` … are EmuTOS's
> cooperative-shell internals NewTOS rebuilds … freely." That is too broad
> and has been corrected in place (see the note at the bottom of this file
> and the edit in `scrap-shell.md` itself): the loop's *internal shape* is
> free, but the **process-identity numbering it produces is not** —
> `ap_id == 0` for the desktop/launched-app slot and `ap_private == 0x1143`
> are measured, reproducible facts about a real machine, not implementation
> choices.

---

## 0. The two process concepts, disambiguated first

EmuTOS overlays two *different* "process" concepts on this code path, and
conflating them is the easiest way to misread it:

- **BDOS `PD`** (`bdos.h`) — a GEMDOS basepage: memory allocation, `Pexec`,
  `p_tbase`/`p_hitpa`/etc. Every `Pexec`/`dos_exec` call creates or consumes
  one of these. This is what `kernel/`'s `Pexec(PE_LOADGO)` already
  implements.
- **AES `AESPD`** (`aes/struct.h:119-154`) — an AES-internal bookkeeping
  record: name, application directory, event/message queue, flags. There
  are exactly `NUM_PDS = NUM_ACCS + 2` of these (`struct.h:36`), allocated
  **once**, statically, at `gem_main()` startup (§1). They are **not**
  created or destroyed by `Pexec`/`dos_exec` — `aes_run_rom_program`'s own
  comment says so explicitly: `PD *pd; /* this is the BDOS PD structure, not
  the AESPD */` (`gemshlib.c:572`).

The global `rlr` ("running process") is a pointer to the **currently active
AESPD**. It is set once, permanently, in `gem_main()`'s init loop (§1), and
**never reassigned** by any of the nested `Pexec`/`dos_exec` calls that run
the desktop, the console, or a launched application. What *does* change
`rlr`'s AESPD's identity is `p_nameit`/`p_setappdir` **overwriting its
fields in place** (§5) — the same AESPD slot is renamed, not replaced.

---

## 1. `gem_main` — the full sequence

**Source:** `aes/geminit.c:753-903`.

> **Citation correction.** The plan's citation `geminit.c:753-800` covers
> only the boot-time resolution-change branch. The function's full body,
> needed to answer "the `gem_main` sequence" as asked, runs to line **903**
> — it also contains the AESPD-table init loop (`:850-877`), the SCRENMGR
> creation (`:888`), and the `aes_run_rom_program(accdesk_start)` call
> (`:894`) that starts everything below. Citing only `:753-800` would miss
> all three.

In order:

1. `dos_conws("\033f\033E")` (`:763`) — cursor off, clear screen.
2. Read `EMUDESK.INF` into `infbuf` (`:766-773`), unless
   `BOOTFLAG_SKIP_AUTO_ACC`.
3. `process_inf1()` (`:776`, only if not already mid-rez-change) — see §2.
4. If a resolution change was requested, apply it via `Setscreen` and
   `gsx_wsclear()`, then reset the boot drive (`:778-802`).
5. `mn_init()` (`:808`), `count_accs()` (`:810`) — scan `\*.ACC`.
6. Allocate + zero the `AESPROCESS` array for accessories (`:812-817`).
7. `totpds = num_accs + 2` (`:819`).
8. `set_aestrap()` (`:822`) — installs trap #2 → `aestrap`, **while
   interrupts are disabled** (`:821,823`).
9. Init event-block free list (`:829-834`).
10. Init sync block `wind_spb` (`:837-839`).
11. **AESPD table init loop**, `i = totpds-1` down to `0`
    (`:851-877`) — see §4.
12. `enable_interrupts()` (`:882`).
13. `gl_mowner = ctl_pd = iprocess("SCRENMGR", ctlmgr)` (`:888`) — creates
    AESPD 1 and starts `ctlmgr` running under it.
14. **`aes_run_rom_program(accdesk_start)`** (`:894`) — this is the call
    that does not return until shutdown or resolution change; everything
    from §3 onward (accessory loading, `sh_main`'s loop, and every
    application ever launched from the desktop) happens *inside* it.
15. On return: `unset_aestrap()` (`:898`), free the accessory array
    (`:901-902`).

**Contract:** the *order* of 3 vs. 11 vs. 13 vs. 14 relative to each other
is not itself observable by an application (it all happens before any
application can run) — **reference**. What *is* contract is everything each
step produces that a later application can read: the AESPD numbering (§4),
`ap_private` (§11), the trap-#2 installation (an application's `appl_init`
trap must work — trivially contract).

`accdesk_start` (`aes/gemstart.S:139`, called only from assembly) calls
`run_accs_and_desktop()` (`geminit.c:654-750`), **not** `deskmain()`
directly — this is a second, outer level distinct from `sh_ldapp`'s own
`aes_run_rom_program(deskstart)` call (§6, §8). `run_accs_and_desktop()`
does the one-time-per-boot AES setup (open the GEM resource, open the VDI
workstation, seed the shell buffer with `CP_SHELL_INIT`, load accessories,
take the tick/error-handler vectors, enter graphics mode) and *then* calls
`sh_main(isauto, isgem)` (`:739`) — the loop documented in §7. When
`sh_main` finally returns (shutdown or rez-change), `run_accs_and_desktop`
tears the same things back down (`:741-749`) and returns to `accdesk_start`,
which is what makes `aes_run_rom_program(accdesk_start)` in `gem_main`
finally return.

**So there are two nested "ROM program" launches, not one:**

```
gem_main()
  └─ aes_run_rom_program(accdesk_start)        [outer, once per boot]
       └─ run_accs_and_desktop()
            └─ sh_main(isauto, isgem)           [the loop, §7]
                 └─ sh_ldapp()                  [each iteration, §8]
                      └─ aes_run_rom_program(deskstart)   [inner, once per desktop instance]
                           └─ deskmain()         (desk/deskmain.c)
```

The desktop (`deskstart`/`deskmain`) is launched and terminates repeatedly,
*inside* the single outer `accdesk_start` call that lives for the whole AES
session. This is why `rlr` never needs reassigning across desktop restarts:
it was set up once at the `gem_main` level (§4), long before either nested
launch begins.

---

## 2. `process_inf1` — `#Q`, `#E` resolution

**Source:** `aes/geminit.c:343-392`. **[CONTRACT]** for the resolution
decision it makes (an application booted after a rez-change observes the new
mode); **[REFERENCE]** for the string-scanning mechanics.

Called from `gem_main:776`, before any AESPD/rez-change/desktop code runs.
Scans `infbuf` (the raw `EMUDESK.INF` bytes) for `#E` and `#Q` lines:

- **`#E` (`:361-382`)** — skips 6 bytes of non-video preference fields; if
  the line ends there (`\r`), no video info was saved and nothing happens.
  Otherwise reads two hex bytes (`env1`, `env2` via `scan_2`), combines them
  into `mode`, and calls `check_moderez(mode)`. If `mode == 0`: no change.
  If `mode > 0`: `gl_changerez = 2` (Falcon-family), `gl_nextrez = mode`. If
  `mode < 0`: `gl_changerez = 1` (ST/TT-family), `gl_nextrez = (mode &
  0x00ff) + 2` — **note the `+2` offset baked in here**, the same offset
  `shel_write(SHW_RESCHNG)` documents for its caller (`scrap-shell.md`
  discrepancy #5); this is `process_inf1` being a *second* producer of the
  same `gl_nextrez` convention.
- **`#Q` (`:384-388`, `#if CONF_WITH_BACKGROUNDS`)** — reads three
  desktop-background byte-pairs into `bg[0..2]` via `scan_2`, sets
  `bgfound = TRUE`. This is the **AES's own** read of the `#Q` line, used
  later by `set_aes_background()` (§11) — entirely separate from the
  desktop's own `#Q` reader in `desk/deskapp.c:953` (confirmed: `deskapp.c`
  has its own `case 'Q':` in `app_start`'s line loop, reading into
  `G.g_patcol[]`, via its own `read_inf_file()`, `deskapp.c:680-692`). **Two
  independent readers of the same `EMUDESK.INF` line** — this is 1:1 by
  design, not an accident to fix.

`process_inf1` runs **before** `process_inf2` (§3) and before the AESPD
table exists; it only sets globals (`gl_changerez`, `gl_nextrez`, `bg[]`,
`bgfound`) consumed later.

---

## 3. `process_inf2` — `#E` double-click, `#Z` autorun

**Source:** `aes/geminit.c:407-464`. **[CONTRACT]** for the autorun
selection, the double-click speed, and the initial alpha/graphic mode;
**[REFERENCE]** for the parsing mechanics.

Called from `gem_main:734`, **after** the AESPD table and accessories are
set up but **before** `dsptch()`/`sh_main`. Signature:
`static BOOL process_inf2(BOOL *isauto)`.

- **`#E` (`:421-436`)** — `ev_dclick(env & 0x07, TRUE)` sets the
  double-click speed from the low 3 bits of the first `#E` byte; optionally
  (gated `CONF_WITH_BLITTER`/`CONF_WITH_CACHE_CONTROL`) sets blitter and
  cache state from later bytes on the same line.
- **`#Z` (`:438-460`)**, format `#Z <isgem-byte> <path>@`:
  1. `scan_2(pcurr, &isgem)` — `00` = not GEM (character mode), else GEM.
     **This `isgem` value is a local variable in `process_inf2`, not stored
     via `sh_wdef`.** It only affects `process_inf2`'s own return value
     (below) — nothing else reads it.
  2. The path string up to `@` is captured (`tmpptr1`); `sh_name(tmpptr1)`
     finds the filename portion (`tmpptr2`) and `*(tmpptr2-1) = 0` **splits
     the single string in place** — a NUL is written right after the last
     path separator, turning one buffer into two NUL-terminated strings: a
     directory prefix (`tmpptr1`) and a bare filename (`tmpptr2`).
  3. Unless `BOOTFLAG_SKIP_AUTO_ACC`: **`sh_wdef(tmpptr2, tmpptr1)`**
     (`:455`) stores *only* the split (program-name, directory) pair into
     the `SHELL` struct's `sh_desk`/`sh_cdir` fields (`gemshlib.c:504-510`);
     `*isauto = TRUE` (`:456`).

**Return value** (`:463`): `(*isauto && !isgem) ? FALSE : TRUE`. This
becomes `gem_main`'s local `isgem` (`:734`), passed straight into
`sh_main(isauto, isgem)` (`:739`) as the **initial** `SHELL.sh_isgem` seed
(`gemshlib.c:699`) — i.e. whether the very first thing `sh_main`'s loop
runs (the autorun program, or the desktop) starts in graphic or character
mode. **The `isgem` flag from `#Z` is threaded out-of-band through this
return value, not through `sh_wdef`.** `sh_wdef` carries only the
program/directory split.

---

## 4. AESPD numbering: `0` = dos-appl, `1` = SCRENMGR, `2+` = ACC

**Source:** `aes/geminit.c:844-877` (init loop), `:888` (SCRENMGR);
`aes/struct.h:36` (`NUM_PDS`). **[CONTRACT]** — `ap_id` (§11 sibling) is a
documented `appl_init()` out-value.

```c
/* geminit.c:851-877 */
for(i=totpds-1; i>=0; i--)
{
    rlr = pd_index(i);
    ...
    memset(rlr->p_name, ' ', AP_NAMELEN);
    rlr->p_appdir[0] = '\0';
    ...
    rlr->p_pid = i;
    rlr->p_stat = 0;
    rlr->p_msg.action = -1;
}
curpid = 0;
rlr->p_pid = curpid++;   /* → pid 0, and rlr now stays pointed at pd_index(0) */
rlr->p_link = NULL;
```

The loop counts **down** from `totpds-1` to `0` and assigns `p_pid = i`
directly for every slot — **including** slot 0, which gets `p_pid = 0` twice
(once in the loop body, once redundantly by the `rlr->p_pid = curpid++`
line after the loop, since `rlr` is left pointing at `pd_index(0)` on the
loop's last iteration). `pd_index(i)` (`gempd.c:32-34`) maps `i<2` to
`D.g_int[i].a_pd` (the two "internal" AESPDs: 0 = dos-appl, 1 = SCRENMGR)
and `i>=2` to `D.g_acc[i-2].a_pd` (accessories).

`curpid` is a separate global cursor used only by `getpd()`
(`gempd.c:60-70`, `p = pd_index(curpid); p->p_pid = curpid++;`), which is
called from `iprocess`/`pstart` — **not** from the init loop above. After
the init loop leaves `curpid = 1`:

- `iprocess("SCRENMGR", ctlmgr)` (`geminit.c:888`) → `pstart` → `getpd()` →
  consumes `curpid == 1` → **AESPD 1 = SCRENMGR**, `curpid` becomes 2.
- Each accessory, loaded via `load_one_acc` → `pstart` → `getpd()`
  (`geminit.c:236-257`), consumes `curpid == 2, 3, …` → **AESPDs 2+ = ACCs**.

`NUM_PDS = NUM_ACCS + 2` (`struct.h:36`, comment: `/* acc's + ctrlpd + dos
appl. */`) — note the comment lists them in the opposite order to the
numeric assignment (accs first in the comment, but numerically **last**).

**AESPD 0 (the "dos appl" slot) is never passed through `getpd()`/`curpid`
again after this one-time init.** It is the slot `rlr` permanently points
at, and it is what `sh_ldapp` renames for every desktop instance and every
launched application (§5, §8) — one physical AESPD struct, reused
sequentially, never reallocated.

---

## 5. `p_nameit` / `p_setappdir` applied to `rlr`

**Source:** `aes/gempd.c:77-107` (definitions); `aes/gemshlib.c:604-605`
(desktop), `:634-635` (normal/autorun app). **[CONTRACT]**: `p_name` is what
`appl_find` (opcode) matches against and what a real control panel /
accessory would use to identify the foreground app; `p_appdir` is what
`shel_find`'s app-directory search step reads (`scrap-shell.md` item 11).

```c
/* gempd.c:77-86 */
void p_nameit(AESPD *p, char *pname)
{
    /* copies up to AP_NAMELEN=8 chars of pname into p->p_name,
       stopping at '.', space-padding the rest */
}

/* gempd.c:90-107 */
void p_setappdir(AESPD *pd, char *pfilespec)
{
    /* finds the position after the last PATHSEP in pfilespec,
       copies everything up to and including it into pd->p_appdir */
}
```

Both are called with **`rlr`** as the target — the single persistent AESPD
0 — in exactly two places in `sh_ldapp`:

```c
/* gemshlib.c:602-605, DESKTOP_APP arm */
sh_show("");
p_nameit(rlr, fname);        /* fname = filename portion of D.s_cmd = "EMUDESK" */
p_setappdir(rlr, D.s_cmd);

/* gemshlib.c:632-636, NORMAL_APP/AUTORUN_APP arm */
sh_show(fname);
p_nameit(rlr, fname);        /* fname = filename portion of the app being launched */
p_setappdir(rlr, D.s_cmd);
rlr->p_flags = 0;
```

This is the mechanism by which "AESPD 0 is renamed" (plan §2.1) actually
happens: the same struct's `p_name`/`p_appdir` fields are overwritten each
time a new program is about to run. There is no third call site for these
two functions outside `sh_ldapp` and `pstart` (used for SCRENMGR/ACCs, §4) —
confirmed by reading every call site in the tree
(`grep p_nameit\|p_setappdir` across `aes/` and `desk/` returns only these).

**`CONSOLE_APP` is the one dispatch arm that does *not* call
`p_nameit`/`p_setappdir` on `rlr`** (`gemshlib.c:614-622`) — EmuCON runs
under whatever name/appdir the previous occupant of AESPD 0 left behind.
This is a genuine EmuTOS behaviour, not an omission in this document: it
follows directly from reading `sh_ldapp`'s three arms side by side.

---

## 6. `aes_run_rom_program`

**Source:** `aes/gemshlib.c:570-580`. **[REFERENCE]** — internal mechanism
for running a ROM-resident function as a full BDOS process; NewTOS's
`Pexec`-based launch of a real `.NAP` ELF is a legitimate 1:1-behaviour
reimplementation of what this achieves, not of how.

```c
LONG aes_run_rom_program(PRG_ENTRY *entry)
{
    PD *pd;     /* this is the BDOS PD structure, not the AESPD */

    /* Create a basepage with the standard Pexec() */
    pd = (PD *) Pexec(PE_BASEPAGEFLAGS, (char*)PF_STANDARD, "", NULL);
    pd->p_tbase = (UBYTE *) entry;

    /* Run the program with dos_exec() for AES reentrancy issues */
    return dos_exec(PE_GOTHENFREE, NULL, (const char *)pd, NULL);
}
```

Two calls: `gem_main:894` (`accdesk_start`, the outer per-boot wrapper, §1)
and `sh_ldapp:606` (`deskstart`, once per desktop instance, §8). Both create
a **fresh BDOS basepage** (new GEMDOS-level memory allocation) whose "text"
is simply the function pointer — there is no ELF/PRG file involved, the
"load address" is the function's own machine code address in ROM. The
comment `/* for AES reentrancy issues */` documents *why* `dos_exec` is used
instead of directly calling the function: it goes through the full
BDOS process-switch machinery (stack, SSP) rather than a bare `jsr`, which
matters because these calls nest (§1's diagram) inside a system that is
otherwise a flat, non-preemptive single stack per active BDOS process.

---

## 7. The `sh_main` loop

**Source:** `aes/gemshlib.c:692-731`. **[REFERENCE]** for the loop shape;
**[CONTRACT]** for what each iteration does to shared window/screen state
that a launched program can observe on entry.

```c
void sh_main(BOOL isauto, BOOL isgem)
{
    WORD rc = 0;
    SHELL *psh = &sh;

    psh->sh_doexec = SHW_EXEC;
    psh->sh_nextapp = isauto ? AUTORUN_APP : DESKTOP_APP;
    psh->sh_isgem = isgem;
    strcpy(sh_apdir, D.s_cdir);
    gl_shgem = TRUE;

    if (psh->sh_nextapp == DESKTOP_APP)
        set_default_desktop(psh);

    do
    {
        sh_chdef(psh);                   /* :712 — set drive/dir for next app */
        sh_chgrf(psh);                   /* :713 — switch alpha/graphic mode  */

        if (gl_shgem)
        {
            wm_init();                   /* :717 */
            ratinit();                   /* :718 */
            sh_draw(D.s_cmd, TRUE);      /* :719 — CLEAR_SCREEN */
        }

        if (rc && !IS_BIOS_ERROR(rc))
            fm_error(-rc-31);

        rc = sh_ldapp(psh);              /* :725 */

    } while((psh->sh_doexec != SHW_SHUTDOWN) && !gl_changerez);

    if (gl_shgem)
        sh_toalpha();
}
```

Per-iteration order, **before** the next program runs:
`sh_chdef` → `sh_chgrf` → (if graphic) `wm_init`/`ratinit`/`sh_draw(clear)`
→ (error alert from the *previous* iteration's return code, if any) →
`sh_ldapp` (which actually runs the program and blocks until it exits).

`IS_BIOS_ERROR(n)` = `(n < E_OK) && (n > EINVFN)` (`gemerror.h:64`) —
used to decide whether `sh_ldapp`'s return code from the *previous*
iteration should pop a `fm_error` alert (BIOS-level errors, e.g. from a
failed drive access during the *next* app's `sh_chdef`, are suppressed
here; GEMDOS-level errors like `EFILNF` from a failed `sh_find` are shown).

The alert is an ordinary taker of the window-tree lock: `fm_error` →
`fm_show` → `fm_alert`'s `wm_update(BEG_UPDATE)` (`aes/gemfmalt.c:361`)
waits while a desk accessory holds it. NewTOS (A118, 2026-10-25): the shell
loop issues the alert as the AES call `form_error` through the GEM trap from
its EL1 context (`kernel/src/shell_loop.rs` `fm_error`), so the dispatcher's
gate parks it like any other caller (the EL1 kernel park, the `svc`
re-executed when the lock is free); a direct call could not wait.

`wait_for_accs(AP_MESAG)` (`geminit.c:737`, called once, **before**
`sh_main`, not inside its loop) is separate and out of `sh_main`'s scope —
it blocks until all desk accessories have done their first `appl_init`.

**The exit condition on NewTOS — both terms, and the restart around them
(A48 tier 1 + A106, 2026-09-22).** `kernel/src/shell_loop.rs`'s `sh_main`
implements this loop with **both** terms of `:727`, through
`tos_api::aes::shell::sh_main_continue()` (`sh_doexec != SHW_SHUTDOWN &&
gl_changerez == 0`), and reports which one failed as a `ShExit`
(`Shutdown` / `RezChange`) — the value `gemstart.S:122-124` re-reads from
`gl_changerez` after `gem_main` returns.

`gl_changerez` has two producers and only one of them is hardware-gated
here. `process_inf1`'s boot-time `#E` parse (§2 above) goes through
`check_moderez`, which genuinely reports no change on a board with exactly
one fixed mode (ADR-017 rev 2, D-101). `sh_write`'s `SHW_RESCHNG` arm
(`gemshlib.c:187-191`) is pure AES software state, set unconditionally,
independent of what real hardware can do — recorded since A48 tier 1
(`tos-api/src/aes/shell.rs` `gl_changerez()`/`gl_nextrez()`) and **consumed
here** since A106.

The outer loop is `kernel/src/lib.rs`'s session loop, NewTOS's
`gemstart.S:86-124` `aes_restart`: one pass is one AES session — the tail
of `run_accs_and_desktop` (`sh_tographic` → `process_inf2` → `sh_main`) —
and a `ShExit::RezChange` runs `tos_api::aes::reinit()` (the `theglo`
re-zero plus `gem_main`'s and `run_accs_and_desktop`'s init, module by
module), `kernel::desk::reset()` (the desktop is a program that restarts,
`deskmain`) and `dos_sdrv(bootdev)` (`geminit.c:796-803`), then enters
`sh_main` again. `ShExit::Shutdown` falls through to `halt()` and is
unreachable on this profile for the same reason it is on a Falcon
(`can_shutdown()` FALSE, `bios/bios.c:754-770`).

Each session opens the AES's VDI workstation first and closes it last, as
`run_accs_and_desktop` does: `gsx_init()` (`geminit.c:675`, `v_opnwk` →
`gl_handle`, then `gsx_start`'s line/text set-up) before `load_accs`, and
`gsx_wsclose()` (`:749`, `v_clswk`) after `free_accs` — on the shutdown
path too, since EmuTOS closes before returning out of
`run_accs_and_desktop`. NewTOS: `tos_api::aes::gsxif::{gsx_init,
gsx_wsclose}` in `kernel/src/lib.rs`'s session loop; contract and
consequences in [`window-graphics.md` § `graf_handle`](./window-graphics.md)
(K1, 2026-10-07).

The one step of `gem_main` that stays absent is the physical mode switch
(`Setscreen`/`Vsetmode`, `geminit.c:778-793`): the framebuffer is
negotiated once at boot (ADR-017 rev 2, D-101), so `gl_nextrez` is
recorded, reported and never handed to a mode switch. `gsx_wsclear()`
(`:795`) and `dos_conws("\033f\033E")` (`:763`) need no separate step —
`sh_main`'s own closing `sh_toalpha()` (`:733`) clears the screen and homes
the cursor. Designed in
[`docs/analysis/a48-shw-reschng-design-2026-09-21.md`](../../analysis/a48-shw-reschng-design-2026-09-21.md)
§5; host tests `tos-api/tests/aes_a106_restart.rs`, QEMU evidence in
`measurements/measurement-2026-09-22-a106/`.

---

## 8. `sh_ldapp` dispatch — all four logical arms

**Source:** `aes/gemshlib.c:592-689`. **[CONTRACT]** for the observable
effect of each arm (what runs, what state the newly-started program sees);
**[REFERENCE]** for how the dispatch is coded.

`sh_nextapp` has four values (`NORMAL_APP=0`, `CONSOLE_APP=1`,
`AUTORUN_APP=2`, `DESKTOP_APP=3`, `gemshlib.c:70-73`), but the C code has
only **three** top-level branches — `AUTORUN_APP` and `NORMAL_APP` share one
branch, distinguished internally by two `if`s (`:629`, `:639`, `:662`).

**Arm 1 — `DESKTOP_APP`** (`:600-612`):
```c
sh_show("");                      /* :603 — no name shown for the desktop itself */
p_nameit(rlr, fname);             /* fname = "EMUDESK" */
p_setappdir(rlr, D.s_cmd);
if (aes_run_rom_program(deskstart))
{
    wm_new();                     /* :609 — only on ABNORMAL desktop exit */
}
return 0;
```
Note `wm_new()` here only runs if `aes_run_rom_program` returns **non-zero**
(abnormal termination) — the desktop's own normal exit path
(`deskmain.c:2154-2166`, per the plan's §2.2) already does its own cleanup
(`wind_set(DESKWH, WF_NEWDESK, NULL, 0, 0)`, `appl_exit()`) before
`Pterm`ing, so the "clean" path does not need `wm_new` again.

**Arm 2 — `CONSOLE_APP`** (`:614-622`, `#if WITH_CLI`):
```c
aes_run_rom_program(coma_start);
set_default_desktop(psh);          /* always return to desktop after EmuCON */
return 0;
```
No `p_nameit`/`p_setappdir` call (§5), no `wm_new()` call at all in this
arm.

**Arm 3 — `NORMAL_APP` / `AUTORUN_APP`** (`:624-689`), the combined arm:
```c
ret = (psh->sh_nextapp == AUTORUN_APP) ? 1 : sh_find(D.s_cmd);   /* :629 */
if (ret)
{
    sh_show(fname);
    p_nameit(rlr, fname);
    p_setappdir(rlr, D.s_cmd);
    rlr->p_flags = 0;

    if (psh->sh_nextapp == NORMAL_APP)        /* :639 */
    {
        psh->sh_nextapp = DESKTOP_APP;        /* preset BEFORE the app runs: */
        psh->sh_isgem = TRUE;                 /*  "return to desktop by default" */
    }

    ret = dos_exec(PE_LOADGO, D.s_cmd, ad_stail, ad_envrn);   /* :645 — the actual launch */

    if (rlr->p_flags & AP_OPEN)               /* :650-657 — appl_init w/o appl_exit cleanup */
    {
        mn_cleanup();
        if (rlr->p_qindex)
            ap_rdwr(MU_MESAG, rlr, rlr->p_qindex, (WORD *)D.g_valstr);
        rlr->p_flags &= ~AP_OPEN;
    }

    if (psh->sh_nextapp == AUTORUN_APP)       /* :662-666 */
        set_default_desktop(psh);

    if (wind_spb.sy_owner == rlr)             /* :668-669 */
        unsync(&wind_spb);

    if (gl_shgem)                             /* :676-677 */
        wm_new();

    return (ret < 0) ? ret : 0;
}
set_default_desktop(psh);                     /* :687 — sh_find failed */
return EFILNF;
```

Key points not obvious from a skim:
- The "return to desktop after this app" decision (`:639-643`) is made
  **before** `dos_exec` runs the child — a well-behaved app that calls
  `shel_write` itself while running overrides this preset.
- The `AP_OPEN` cleanup (`:650-657`) fires whenever the *just-terminated*
  program called `appl_init()` — which sets `AP_OPEN` on `rlr`
  (`rlr->p_flags |= AP_OPEN;`, `gemaplib.c:60`, inside `ap_init()`, called
  from the `APPL_INIT` dispatch arm at `gemsuper.c:99`) — but never called
  `appl_exit()` before
  `Pterm`ing — a crashed or sloppy GEM app. It runs `mn_cleanup()` and
  drains any pending message queue on `rlr` via `ap_rdwr`, then clears the
  flag. This is the AES doing on the app's behalf exactly what
  `appl_exit()` would have done — necessary because `rlr` (AESPD 0) is
  about to be reused by the *next* program, and a stale `AP_OPEN`/queue
  would corrupt that next program's state.
- `wm_new()` (`:677`) only runs `if (gl_shgem)` — a character-mode
  (autorun TTP) app that never touched graphics skips window cleanup
  entirely, since there is nothing to clean up. **NewTOS (A96,
  2026-09-21):** `kernel/src/shell_loop.rs` runs this sweep on the shell-loop
  path; the built-in desktop's double-click launch, which bypasses the shell
  loop (gap G1), runs the same step as `wm_new_keep_desktop()` at
  `desktop_resume` — every window but the desktop's own, which this
  desktop keeps because it resumes in place instead of restarting through
  `sh_main`/`cnx_get` (gap G2). (That construct ended with DESK43 U3;
  `wm_new_keep_desktop` was removed in U10 — the desktop restarts through
  `sh_main` like EmuTOS's.) **A107 (2026-09-22):** a GEM program that
  `Pexec`s another one *without* going through this loop (TeraDesk → QED)
  gets the same step for the child's slot alone, at the parent's next AES
  dispatch — `wm_new_for_slot()`. Rule and rationale:
  [`window-graphics.md`](window-graphics.md) § `wind_new (109)`.
- `AUTORUN_APP` skips `sh_find` (line `:629`'s ternary) — the autorun
  entry's path was already validated at INF-parse time (implicitly, by
  virtue of having been written into `EMUDESK.INF` by a previous
  `app_save`); it is trusted rather than re-searched.

### NewTOS implementation notes — the child's exit code (PROC05, DESK43 U9-0b, 2026-10-14)

**[CONTRACT]** `dos_exec(PE_LOADGO, …)`'s value, as `sh_ldapp` sees it
(`docs/analysis/desk43-u9-prep-2026-10-14.md` §2):

1. **Load error** — the child never ran: the negative `xexec` code
   (`EFILNF`, `EPLFMT`, `ENSMEM`, …), unchanged.
2. **The child ran** — its exit code as a **`UWORD`, zero-extended into a
   non-negative LONG `0 … 65535`** (`xterm(UWORD rc)`, `run->p_dreg[0] = rc`,
   `bdos/proc.c:601,616`). A child that ends with `Pterm(-1)` (a crash report's
   `kill_program`, a Rust panic) delivers **65535**, not −1; the console
   Ctrl-C's `xterm(-32)` (`bdos/console.c:101`) delivers 65504.

Consequences, all EmuTOS's:

* **NORMAL/AUTORUN arm:** `(ret < 0) ? ret : 0` (`:681-684`) — a child's
  exit code, whatever it is, **never** raises an alert; `sh_main`'s
  `fm_error(-rc-31)` (`:722-723`) is reached only by load errors and by
  `sh_find`'s `EFILNF` (`:687-688`). There is no alert for `Pterm(-1)`.
* **DESKTOP arm:** `if (aes_run_rom_program(deskstart)) wm_new();` — any value
  other than 0 (`:606-609`); the arm returns 0 in every case (`:611`).

NewTOS: `kernel/src/launch.rs` `run_program_with_tail_rc` returns that value.
The `Pterm` trap already masks the code to 16 bits (D-005); for a program the
kernel launched there is no `Pexec` caller whose `x0` could take it, so the
trap's terminate tail (`finish_terminate`, kernel-launch branch) leaves it for
the launcher (`launch::record_kernel_child_exit`, zero-extended through
`tos_api::bdos::process::pterm::parent_exit_code`) — **how** it gets there is
internal. The shell loop's decision is `tos_api::aes::shell::sh_ldapp_result`
/ `sh_main_alert` (host tests `tos-api/tests/proc05_shell_exit_code.rs`). The
serial line `[shell] sh_ldapp: '<cmd>' exited with rc=N` shows the real value
(until U9-0b always 0); a program that ends cleanly still shows `rc=0`. The
DESKTOP arm's `wm_new()`-on-abnormal-exit is reachable since the desktop is a
launched program (DESK43 U9-4, next section).

### NewTOS implementation notes — the DESKTOP arm starts `NEWDESK` (DESK43 U9-3/U9-4, 2026-10-15)

Decisions D5/D6 (`TASK/DESK43.md`, "U9-Vorbereitung entschieden";
`docs/analysis/desk43-u9-prep-2026-10-14.md` §1). `kernel/src/shell_loop.rs`
`sh_ldapp`, DESKTOP arm:

1. `sh_show("")`, `p_setappdir(rlr, D.s_cmd)` as above (`:603-605`).
2. **`aes_run_rom_program(deskstart)`** (`:606`) = `launch::run_desktop_program`:
   the desktop program embedded in the kernel image under the registry name
   `DEF_DESKTOP` (`NEWDESK`, `kernel/embed.rs`) is loaded and run at EL0 as a
   fresh process, every time. `p_nameit(rlr, fname)` (`:604`) names its AES
   descriptor from `D.s_cmd`. Like EmuTOS's ROM-program start, and unlike a
   `dos_exec(PE_LOADGO)` of the NORMAL arm: **no hourglass**, an **empty**
   command tail (`Pexec(PE_BASEPAGEFLAGS, PF_STANDARD, "", NULL)`, `:575`), and
   `D.s_cmd`/`ad_stail` are **not** rewritten (`shel_read` answers during the
   desktop what it answered before). The name in `D.s_cmd` is not used to find
   the desktop (EmuTOS starts `deskstart` whatever `sh_desk` says).
3. `if (rc != 0) wm_new();` (`:606-609`) — any value other than 0: `Pterm(-1)`'s
   65535 after a crash report's key or a panic's key (D6-a), any other exit
   code, a load error. The arm returns 0 (`:611`); `sh_nextapp` stays
   `DESKTOP_APP`, so the next pass starts the desktop again. **No loop guard**
   (D6-b), **no alert** — EmuTOS's only brake is the crash report's key, and
   NewTOS's desktop panic waits for a key too (`newtos-gem` feature
   `panic-wait-key`, D6-a).
4. **NEWTOS (no EmuTOS counterpart): a load error is not retried** (D6-c). A ROM
   entry point cannot fail to load; an embedded program can (`EPLFMT`,
   `ENSMEM`, no slot, `EFILNF` if the registry has no `NEWDESK`). Loading the
   same blob again would fail again, so the arm logs
   `[shell] desktop NEWDESK could not be loaded (rc=N) — not retried; falling
   back to the console (EmuCON)` and, for the rest of the boot, runs the
   console arm's body in its place (D6-d): `sh_isgem` FALSE + `sh_chgrf`
   (alpha mode, as for any character program, `:513-523`), EmuCON with the
   line `No desktop: NEWDESK could not be loaded (GEMDOS error N).` under its
   banner (screen and serial line), then `set_default_desktop` (`:614-622`).
   Each later pass through the DESKTOP arm logs `[shell] desktop NEWDESK not
   started: its load failed earlier this boot (rc=N)` and takes the same way
   without loading. (From U9-4 to U10 the fallback was the built-in kernel
   desktop, and a boot argument `desk=kernel` chose it outright, D5; DESK43
   U10 removed both together with the kernel's copy of the desktop.)

The serial line of a desktop end is `[shell] sh_ldapp: desktop '<cmd>' exited
with rc=N`. The kernel's "desktop up" observation keys on the launched child's
slot (U9-0c) — `DESKTOP_ACTIVE` is set around the program's start.

**`wm_new()` itself** (`aes/gemwmlib.c:1875-1893`, cited by the plan and
re-verified here): drains outstanding mouse-control and sync-block waiters,
then closes and deletes every window still `VF_ISOPEN`/`VF_INUSE`. This is
what makes "one AESPD, sequential reuse" safe: whatever windows the
previous occupant of `rlr` left open are unconditionally swept before the
next occupant starts.

---

## 9. `sh_chdef` / `sh_chgrf` / `sh_toalpha` / `sh_tographic`

**Source:** `aes/gemshlib.c:223-260` (mode switch), `:513-566` (dir switch +
mode-switch trigger). **[CONTRACT]** for the resulting mode/directory a
launched program observes at startup; **[REFERENCE]** for the vector/mouse
mechanics.

`sh_chdef` (`:537-566`) sets the current drive+directory **before** the next
program runs, dispatching on `sh_nextapp`:

```c
switch(psh->sh_nextapp) {
case NORMAL_APP:
    set_drvdir(sh_apdir);              /* the dir saved by shel_write's SHW_EXEC */
    break;
case CONSOLE_APP:
#if WITH_CLI
    set_drvdir(psh->sh_cdir);          /* set by EmuDesk via shel_wdef() */
#endif
    break;
case AUTORUN_APP:
case DESKTOP_APP:
    set_drvdir(psh->sh_cdir);
    /* build fully-qualified D.s_cmd, splitting only for AUTORUN_APP */
    n = 0;
    if (psh->sh_nextapp == AUTORUN_APP) {
        strcpy(D.s_cmd, psh->sh_cdir);
        n = strlen(D.s_cmd);
        if (n) D.s_cmd[n++] = PATHSEP;
    }
    strcpy(D.s_cmd+n, psh->sh_desk);
    break;
}
```
`set_drvdir` (`:526-534`) is `dos_sdrv`+`dos_chdir` — an ordinary directory
change, contract-observable by the launched program (e.g. via `Dgetcwd`).
**This is where `AUTORUN_APP`'s `sh_wdef`-split (program-name, directory)
is recombined into one fully-qualified `D.s_cmd`** for the actual launch —
confirming the plan's prediction that the split exists specifically for
this reconstruction, not for storage.

`sh_chgrf` (`:513-523`) only acts if the *desired* mode
(`psh->sh_isgem`) differs from the *current* mode (`gl_shgem`):
```c
if (psh->sh_isgem != gl_shgem) {
    gl_shgem = psh->sh_isgem;
    if (gl_shgem) sh_tographic(); else sh_toalpha();
}
```

`sh_tographic` (`:223-239`): retakes the critical-error-handler and AES-trap
vectors (`retake()`, in case a character app stepped on them), converts the
workstation to graphics, sets the initial clip rect, allocates screen
space, starts the mouse (`ratinit()`), sets the hourglass cursor.

`sh_toalpha` (`:246-260`): mirror image — arrow cursor, gives back the
error handler (`giveerr()`), turns off the mouse (`ratexit()`), frees
screen space, closes the graphics workstation.

**Contract-observable consequence:** a GEM application's `appl_init()`
always returns into an already-graphic, already-mouse-active screen (mode
switch happens in `sh_main`'s loop *before* `sh_ldapp` runs it) — an
application never has to perform its own mode transition on entry.

### NewTOS implementation notes — the mode switch for a character program launched by the built-in desktop (DESK22, 2026-09-24)

The mirror-image consequence is the one NewTOS was missing: a **character**
program's screen has no mouse pointer on it, because `sh_toalpha`'s
`ratexit()` (`:256`) takes the cursor off before the text screen appears.

NewTOS's built-in desktop cannot return into `sh_main` (AES gap G1 — it is an
in-kernel call, see §7), so it runs the launch nested. DESK06 built that for
`CONSOLE_APP` (`crate::shell_loop::sh_run_pending_console`), and its EmuCON
screen was pointer-free from the start. The `.NCP` path
(`desk::main::launch_native`, `ftype::Flavor::Console`) predated it and only
cleared the screen (`fbcon::clear_and_home`), so the pointer stayed on the
text as a black smudge
(`measurements/measurement-2026-09-23-desk06/screendumps/redraw-C-program.png`).

The pair `crate::shell_loop::sh_alpha_enter` / `sh_alpha_leave` is that same
pass for a character `NORMAL_APP`: each sets `sh_isgem` the way
`shel_write(SHW_EXEC, isgraf, …)` would (`:173`) and then calls the real
`sh_chgrf`, so `gl_shgem` stays the state machine EmuTOS's next `sh_chgrf`
compares against — the switch is not performed behind the AES's back. The
launch arm's order is `sh_main`'s: mode switch, `fm_error`-equivalent alert
(`:722-723`), desktop rebuild, `desk_busy_off` (`deskmain.c:2071`) undoing
`sh_tographic`'s hourglass.

Before/after screendumps of the text screen:
`measurements/measurement-2026-09-24-desk22-desk23-desk28-desk29/screendumps/d22-{before,after}-C-program.png`.

---

### NewTOS implementation notes — the `gl_shgem` block for a GEM program launched by the built-in desktop (DESK35, 2026-09-24)

`sh_main`'s body has three statements between `sh_chgrf` and `sh_ldapp`, and
they are **inside** `if (gl_shgem)` (`gemshlib.c:715-720`):

```c
sh_chdef(psh);                  /* :712 */
sh_chgrf(psh);                  /* :713 */
if (gl_shgem) {
    wm_init();                  /* :717 -- every window closed and deleted  */
    ratinit();                  /* :718 */
    sh_draw(D.s_cmd, TRUE);     /* :719 -- clear the screen                 */
}
rc = sh_ldapp(psh);             /* :725 */
```

**DESK06 left all three out of the nested run, and that was right for exactly
one of the two cases.** `sh_run_pending_console`'s pass has `sh_isgem == FALSE`,
so `sh_chgrf` has already taken `gl_shgem` to FALSE and *EmuTOS itself skips the
block*: a character program gets no `wm_init` and no `sh_draw`. It can afford
to — the text screen covers the whole display, and the windows the desktop left
in the AES table are torn down by the **next** graphics pass's `wm_init`, not by
this one. A GEM program keeps `gl_shgem` TRUE and EmuTOS takes all three steps,
because that program draws only into its own rectangles: without the clear the
desktop's windows stay on the screen as painted pixels with no event loop behind
them, and without `wm_init` they stay in the AES window table and clip and
composite against the program's own. That was the defect
(`d35q-before-E-qed.png`: QED's menu bar over the frozen desktop).

| EmuTOS (`gemshlib.c`) | NewTOS |
|---|---|
| `wm_init(); ratinit(); sh_draw(D.s_cmd, TRUE)` (`:715-720`) | `crate::shell_loop::sh_gem_enter()`, called from `desk::main::desktop_relinquish` |
| `unsync(&wind_spb); if (gl_shgem) wm_new()` (`:668-669,676-677`) | `crate::shell_loop::sh_gem_leave()`, called from `desk::main::desktop_resume` |
| the restarted `deskmain`: `win_start()`, `desk_verify(DESKWH, FALSE)`, `wind_set(WF_NEWDESK)`, `cnx_get()`, `men_update()`, `menu_bar(menutree, 1)` (`deskmain.c:2028-2073`) | `desktop_resume`: `win_free_all()` (in `desktop_relinquish`, see below), `appl_init`, `desk_verify(DESKWH)`, `wind_set_newdesk`, `cnx_get()`, `men_update()`, `redraw::redraw_desktop()` |

**`cnx_put`/`cnx_get` are not an alternative here, they are the requirement.**
`wm_init` closes and deletes *every* AES window, so after it the desktop's
`WNODE`s hold dead handles and the AES table carries none of the state the
returning desktop needs. That is exactly EmuTOS's situation too, and exactly why
its restarted `deskmain` re-opens the windows from the context save area
(`cnx_get`, `deskmain.c:2055`) — which `desktop_relinquish` has filled with
`cnx_put()` (`deskmain.c:2154`) since DESK09. AES gap G2, named in DESK06 and in
`wm_new_keep_desktop`'s doc comment, is closed by that pair.

### Named deviations

1. **Order: the block runs *before* `appl_exit()`, not after it.** EmuTOS's
   order is the desktop epilog (`deskmain.c:2166`) and *then* `gemshlib.c:717`,
   because there the desktop is a process that has ended. Here the two share one
   AES descriptor across the boundary, and `wm_new`'s `wm_close` fans `WM_REDRAW`
   out to the owners of the windows still open below each one it closes
   (`gemwmlib.c:1216-1222,1453`) — that owner being this desktop. `ap_exit`'s
   queue drain (`gemaplib.c:251-261`) is what keeps the child from inheriting
   those (A28), and it can only do that if it comes last. Same steps, same pass,
   drain last.
2. **A desktop-side `win_free_all()` with no EmuTOS counterpart.** EmuTOS's
   `WNODE` array belongs to the desktop *process*, which ends; the restarted
   desktop allocates a fresh one in `win_start()` (`deskmain.c:2028`). NewTOS's
   desktop resumes in place, so the array it keeps is emptied by hand — through
   `win_free`, so each window's item `OBJECT`s go back to the `g_screen` free
   chain (`obj_wfree`, `deskwin.c:148`) instead of leaking one window's worth per
   launch. It makes no AES call: the windows are closed and deleted by `wm_init`,
   as in EmuTOS.
3. **`sh_gem_leave` is the plain `wm_new`**, not the `wm_new_keep_desktop`
   variant the launch path used before: `sh_gem_enter`'s `wm_init` has already
   taken the desktop's own windows, so there is nothing left to keep and the
   sweep is exactly EmuTOS's.
4. `wind_update(BEG_UPDATE/END_UPDATE)` around `cnx_get` (`deskmain.c:2054,2056`)
   is not taken: the desktop is the only AES client alive at that moment, so the
   bracket has nothing to keep out, and the whole-screen `redraw_desktop()` that
   follows covers what the individual `wind_open`s drew.

### Measured

`measurements/measurement-2026-09-24-desk34-desk35/`, §3 and §4 — QEMU raspi4b, 640×480, two
windows open on `C:\` before the launch.

* `d35-after-D-app.png`: the launched `.NAP`'s window on an otherwise empty
  screen — no menu bar, no desktop windows, no icons. `d35-before-D-app.png` is
  the same instant on the HEAD build: both desktop windows, both desktop icons
  and the trash still standing, the program's window composited *inside* them.
* `d35q-after-E-qed.png` / `d35q-before-E-qed.png`: the same for QED, the
  ticket's own application.
* `d35-after-E-back.png`: after the program terminated, the desktop is back with
  **both** windows at their saved rectangles and in their saved z-order — that is
  `cnx_get` — and the menu bar re-installed.

---

### NewTOS implementation notes — `sh_show(fname)`, the screen behind a launched GEM program (DESK37, 2026-09-24)

The `gl_shgem` block above (DESK35) is only the **first** of two paint steps,
and taking it alone leaves the screen white. EmuTOS's pair is:

```c
sh_draw(D.s_cmd, TRUE);   /* :719 -- ob_spec = CLEAR_SCREEN, ROOT only  */
...
sh_show(fname);           /* :632 -- the whole tree, real ob_spec       */
```

`sh_draw(…, TRUE)` swaps the `DESKTOP` tree ROOT's `ob_spec` for
`CLEAR_SCREEN` — `(WHITE<<12)|(WHITE<<8)|(IP_HOLLOW<<4)|WHITE` = `0x0000`,
`gemshlib.c:64` — draws that one object over the whole screen and puts the
saved `ob_spec` straight back (`:277-282`). Its comment "white desktop screen"
is true and still misleading: the white is a state that lasts milliseconds.
`sh_ldapp` then calls `sh_show(fname)` (`:632`), which is `sh_draw(lcmd,
FALSE)` — `ted->te_ptext = lcmd; ob_draw(tree, ROOT, MAX_DEPTH)` (`:284-286`) —
i.e. the **whole** tree with the root's real `ob_spec`: the desktop background
pattern, the menu-bar strip (object 1), and `DTNAME`'s text (object 2), which
now carries the name of the program being started.

**The background is the AES's, not the desktop's.** It is
`rs_trees[DESKTOP][ROOT].ob_spec`, the resource constant `4419` = `0x1143`
(`gem_rsc.c:551-556`), whose low byte `set_aes_background` replaces with the
`.INF` `#Q` desktop byte at AES start-up (`geminit.c:635-641,719-728`). In
NewTOS that tree is `tos_api::aes::init`'s `DESKTOP` static and the `#Q` read
is `process_inf1_q`. The desktop's own `#Q` copy (`G.g_patcol`, DESK09) plays
no part: by this point `desktop_relinquish` has already un-registered the
desktop's background tree with `wind_set(DESKWH, WF_NEWDESK, NULL)`, so what
the screen shows behind a launched program comes from the AES side alone.

NewTOS's step is `crate::shell_loop::sh_gem_show`, called from
`desk::main::run_gem_app` between `desktop_relinquish()` and the loader —
exactly `sh_ldapp`'s position between the `sh_main` block (`:715-720`) and
`dos_exec` (`:645`).

**Measured** (`measurements/measurement-2026-09-24-desk37-desk36-desk30/`, QEMU raspi4b,
640×480):

* `d37g-after-C-app.png` vs. the EmuTOS reference
  `measurements/measurement-2026-09-24-hatari-appbg/screendumps/emutos-bgwin-falcon-tc.png`
  (Hatari 2.6.1, Falcon/VGA, EmuTOS 1.4.0+): rows 0..17 white with the program
  name in black, row 18 solid black, rows 19..479 a 50 % green/white
  checkerboard — 320 green + 320 white per row in **both**, and the same
  parity phase pixel for pixel (`WGWGWG…` on row 19, `GWGWGW…` on row 20).
  Only the two colour words differ, which is Hatari's Falcon VGA ramp:
  `(0,251,0)`/`(255,251,255)` there against `(0,255,0)`/`(255,255,255)` here.
  The menu bar carries the program name as EmuTOS's does (`GEMWIN.NAP` /
  `BGWIN.PRG`).
* `d37q-after-D-qed.png` vs. `emutos-qed-falcon-tc.png`: QED's own menu bar
  across the top and the same green dither behind it.
* `d37g-before-C-app.png` / `d37q-before-D-qed.png` are the defect: every row
  below the menu strip is solid white, and there is no menu strip at all in
  the `GEMWIN` case.

---

## 10. The state handover channel: `app_save → shel_put` / `shel_get → app_start`

**Source:** `desk/deskapp.c:136` (`CPDATA_LEN`), `:781-840` (`app_start`),
`:1106-1299` (`app_save`); `aes/gemshlib.c:203-216` (`sh_get`/`sh_put`
implementation, opcodes `shel_get`/`shel_put` — already contract-documented
in `scrap-shell.md`); `aes/geminit.c:97,685` (`CP_SHELL_INIT` boot seed).

**Layering, precisely:** `shel_get`/`shel_put` (the AES opcode pair, 122/
123) are **opaque, length-bounded `memcpy`s to/from `D.g_shelbuf`** — the
AES server has **no knowledge of `CPDATA_LEN`** at all. `CPDATA_LEN = 128`
(`deskapp.c:136`) is defined **in `desk/`**, not `aes/`: it is a
**desktop-side convention layered on top of the opaque buffer**, not an AES
contract. The comment at its definition (`deskapp.c:131-135`) says exactly
this: *"the following defines the number of bytes reserved for control
panel use at the start of the shell buffer. these bytes are not modified by
EmuDesk, and are (currently) not saved to the EMUDESK.INF file."*

**Read side — `app_start`** (`deskapp.c:781-840`):
```c
buf = must_alloc_buf(SIZE_SHELBUF);          /* :808 — 4192-byte local buffer */
shel_get(buf, SIZE_SHELBUF);                 /* :832 — whole buffer, incl. the 128-byte prefix */
inf_data = buf + CPDATA_LEN;                 /* :833 — INF text starts after the prefix */
if (!(bootflags & BOOTFLAG_SKIP_AUTO_ACC)
 && (inf_data[0] != '#'))                    /* :834-835 — no valid signature in memory */
    read_inf_file(inf_data);                 /* :836 — fall back to disk */
if (inf_data[0] != '#')                      /* :839 — still nothing valid */
    build_inf(inf_data, xcnt, ycnt);          /* :840 — synthesize hardcoded defaults */
```
Three-level fallback: **shell-buffer memory → disk file → hardcoded
defaults.** The 128-byte prefix (`buf[0..128]`) is read into `buf` but
**never inspected** by `app_start` — it is treated as opaque bytes that
happen to precede the INF text in the same buffer.

**Write side — `app_save`** (`deskapp.c:1106-1299`):
```c
outbuf = must_alloc_buf(SIZE_SHELBUF+MAX_SIZE_INF_LINE);
shel_get(outbuf, CPDATA_LEN);                /* :1123 — round-trip ONLY the 128-byte prefix */
inf_data = outbuf + CPDATA_LEN;              /* :1124 — new INF text written starting here */
... (build INF text into inf_data via sprintf, :1128-1284) ...
len = pcurr - outbuf;                        /* :1290 — total length incl. prefix */
shel_put(outbuf, len+1);                     /* :1293 — write prefix+text+NUL back */
if (todisk)
    save_to_disk(inf_data, len-CPDATA_LEN);  /* :1297 — disk write EXCLUDES the prefix */
```
`app_save` pulls the **current** 128-byte prefix out of the shared buffer
first (preserving whatever a real control-panel accessory left there),
writes fresh INF text after it, then puts the **whole thing** (prefix +
text) back — the prefix passes through unmodified on every save. The disk
file never contains the prefix at all (`save_to_disk` is called with
`inf_data`, which already starts past it).

**Boot-time seed:** `run_accs_and_desktop()` calls
`sh_put(CP_SHELL_INIT, sizeof(CP_SHELL_INIT)-1)` (`geminit.c:685`) exactly
**once per AES boot** (not once per desktop restart), where
`CP_SHELL_INIT = "#a000000\r\n"` (`geminit.c:97`) — 9 bytes. This writes
into the **start** of `D.g_shelbuf`, i.e. into the CPDATA prefix region
(bytes 0-8 of it), **before** accessories load and **before** `sh_main`'s
loop (and thus the desktop) ever runs. On a genuinely cold boot this is the
only content in the shel buffer's prefix; bytes at offset 128+ (where
`app_start` looks for the `#` signature) are still zero, so the very first
`app_start()` call correctly falls through to the disk-file (or built-in
default) path. After the first `app_save()`, the prefix contains whatever
that save round-tripped in (still `CP_SHELL_INIT`'s bytes, since nothing
in this codebase ever writes new content into the first 128 bytes) and the
signature at offset 128 is `#` (a real INF line), so subsequent
`app_start()` calls (e.g. after returning from a launched application, each
time `sh_ldapp`'s `DESKTOP_APP` arm restarts `deskmain`) succeed from
memory without touching disk.

**Contract framing:** the `shel_get`/`shel_put` opcode ABI (length + memcpy,
`SIZE_SHELBUF=4192`) is **already contract**, documented in
`scrap-shell.md`. The **128-byte split and its three-level fallback order**
is **not** AES-server contract (the server has no idea it exists) — it is
**desktop-program behaviour**. Under the project's standing "1:1 EmuTOS,
no deviations" rule, a faithful NewTOS desktop client must still reproduce
it (reserve the first 128 bytes, round-trip them unmodified through every
`app_save`, use the memory→disk→defaults fallback order in `app_start`) —
but this is a fidelity requirement on *our desktop program*, not a
requirement on the AES server's opcode behaviour.

---

## 11. The `ap_private` chain, and `rs_trees[DESKTOP]`

**Source:** `aes/geminit.c:635-641` (`set_aes_background`), `:724-727`
(caller, in `run_accs_and_desktop`); `aes/gemsuper.c:94-96` (`appl_init`);
`aes/gem_rsc.c:550-578`; `aes/gem_rsc.h:48-49`. **[CONTRACT]** — `ap_private`
is a documented `appl_init()` out-value; this is the value D-R1-6 measures.

```c
/* gem_rsc.h:48-49 */
#define DESKTOP  2      /* rs_trees[] index */
#define DTNAME   2      /* object index *within* that tree */

/* gem_rsc.c:550-570 — TREE 2 ("DESKTOP"), 3 objects */
{ -1, 1, 2, G_BOX,  NONE, NORMAL, (LONG) 4419L,      0,0,80,25  },  /* [0] ROOT */
{  2,-1,-1, G_BOX,  NONE, NORMAL, (LONG) 16716032L,  0,0,80,513 },  /* [1] background box */
{  0,-1,-1, G_TEXT, LASTOB, NORMAL, (LONG)&rs_tedinfo[12], 0,0,80,769 },  /* [2] DTNAME */

/* gem_rsc.c:574-578 */
OBJECT * const rs_trees[] = {
    &rs_obj[TR0],   /* FSELECTR */
    &rs_obj[TR1],   /* DIALERT  */
    &rs_obj[TR2],   /* DESKTOP  */
};
```

`rs_trees[DESKTOP][ROOT].ob_spec` (object 0, `ROOT` = `0` per
`include/obdefs.h:20`) is the **literal decimal constant `4419`** in the
built-in `gem.rsc` resource — `4419 == 0x1143`, exactly the value measured
on the real TT (plan §1.2 point 4). This confirms the plan's correction of
the earlier premise ("the desktop's own resource" was the wrong tree):
**this is the AES's own compiled-in `gem.rsc`, tree index 2, not
`desk/desktop.rsc`.**

Chain, in call order:
```c
/* run_accs_and_desktop, geminit.c:713-717 — every boot */
tree = rs_trees[DESKTOP];
tree[ROOT].ob_height = gl_rscreen.g_h;      /* screen height, once per boot */

/* geminit.c:719-728, CONF_WITH_BACKGROUNDS — only if #Q was found (process_inf1, §2) */
if (bgfound) {
    WORD n = (gl_nplanes > 2) ? 2 : gl_nplanes-1;
    set_aes_background(bg[n] & 0xff);
}

/* set_aes_background, geminit.c:635-641 */
void set_aes_background(UBYTE patcol) {
    OBJECT *tree = rs_trees[DESKTOP];
    tree[ROOT].ob_spec &= 0xffffff00L;      /* clear low byte only */
    tree[ROOT].ob_spec |= patcol;           /* OR in the new pattern/colour byte */
}

/* appl_init, gemsuper.c:94-96 — every APPL_INIT call, by any process */
pglobal->ap_id = rlr->p_pid;
tree = rs_trees[DESKTOP];
pglobal->ap_private = tree[ROOT].ob_spec;   /* whatever the current low byte is */
```

So: `ap_private`'s low byte is `4419 & 0xff = 0x43` **by default** (no `#Q`
line found), or the `#Q`-supplied `patcol` byte if `process_inf1` found one
— `set_aes_background` only ever touches the low 8 bits, the high 24 bits
(`0x1143 & 0xffffff00 = 0x1100`, i.e. the `0x11` byte at bit 8) are the
resource's compiled-in constant and never change. **This is every process's
`ap_private`, not just the desktop's** — any accessory or launched
application calling `appl_init()` gets the same value, because it reads a
single shared static object tree, not anything specific to the caller's
identity.

**What `ap_private` is *not*, corrected per the plan's own §1.2 finding, and
one citation of the plan's tightened here:** the tree the desktop registers
via `wind_set(WF_NEWDESK)` lands in `gl_newdesk`
(`aes/gemwmlib.c:54,1687`) and is read by **two** functions — `w_drawdesk`
(`:340-350`) and, separately, `w_redraw_desktop`
(`:1902-1919`, `#if CONF_WITH_3D_OBJECTS`) — not "only `w_drawdesk`" as the
plan states; both are unrelated to `ap_private`. It is also **reset to
`NULL` on every `wm_init()` call** (`:1300`), which `sh_main`'s loop invokes
before every single program launch when in graphics mode
(`gemshlib.c:717`, §7 above) — so a previous program's `WF_NEWDESK`
registration cannot leak into the next one either. Neither function nor the
reset touches `rs_trees[DESKTOP]`/`ap_private`. This document re-confirms
the plan's core finding by tracing `appl_init`'s `tree = rs_trees[DESKTOP]`
line directly: it unambiguously indexes the compiled-in `gem.rsc` array,
not any `wind_set`-registered tree.

---

## 12. `DEF_DESKTOP` / `DEF_CONSOLE` — magic names

**Source:** `aes/geminit.h:13` (`DEF_DESKTOP`), `include/aesdefs.h:627`
(`DEF_CONSOLE`), `aes/gemshlib.c:172` (classification use).

```c
/* geminit.h:13 */
#define DEF_DESKTOP "EMUDESK"   /* default desktop */

/* aesdefs.h:627 */
#define DEF_CONSOLE     "EMUCON"
```

`DEF_DESKTOP` is used in two places: `sh_write`'s `SHW_NOEXEC` case
(`gemshlib.c:163`, `strcpy(D.s_cmd, DEF_DESKTOP)`) and
`set_default_desktop` (`:587`, `strcpy(psh->sh_desk, DEF_DESKTOP)`) — both
purely internal (`D.s_cmd`/`sh_desk` are never inspected by an application;
only the *effect*, "the desktop runs next", is observable). **Reference.**

`DEF_CONSOLE` is compared directly against the caller-supplied `pcmd`
string inside `sh_write` itself:
```c
/* gemshlib.c:172 */
psh->sh_nextapp = (strcmp(pcmd, DEF_CONSOLE) == 0) ? CONSOLE_APP : NORMAL_APP;
```
This means **an application that calls `shel_write(SHW_EXEC, 0/1, isover,
"EMUCON", tail)` directly** — not just the desktop's own "Execute EmuCON"
menu item (`desk/deskmain.c:751-753`, `pro_run(FALSE, DEF_CONSOLE, …)`) —
triggers the same classification. The literal string `"EMUCON"` is
therefore **contract** (already listed as such in `scrap-shell.md` item 8);
`"EMUDESK"` is not directly typed by any caller and stays reference.

---

## Predictions — confirmed / refuted

The plan predicted two mismatches between EmuTOS and our
`programs/desktop/src/inf.rs`. Checked directly against the current file (read
only, no code touched):

**(a) The 128-byte `CPDATA` prefix.** **Confirmed, and stronger than
predicted.** `programs/desktop/src/inf.rs` has **no `shel_get`/`shel_put` call
at all** (`grep -n "shel_get\|shel_put\|CPDATA" programs/desktop/src/inf.rs` —
zero hits). It talks to `NEWDESK.INF` exclusively via direct GEMDOS
`fopen`/`fcreate`/`fread`/`fwrite` (`read_inf_file`/`write_inf_file`,
`inf.rs:71-101`), unconditionally hitting disk on every load/save. The AES
server side of `shel_get`/`shel_put` (opcodes 122/123) already exists and
is spec'd (`tos-api/src/aes/shell.rs:336-357`, per `scrap-shell.md`) — the
gap is entirely that the desktop client never calls it. So the prediction
undersold the finding: it is not "the prefix is handled wrong", it is "the
whole memory-buffer channel §10 describes is unused; every load/save goes
straight to disk, skipping the memory-first fallback order `app_start`
implements (`deskapp.c:832-836`) entirely."

**(b) `#Z` with its `isgem` flag and the `sh_wdef` program/directory
split.** **Confirmed.** `programs/desktop/src/inf.rs`'s `#Z` arm
(`parse_inf`, `:366-369`) reads one byte and **discards** it (comment:
`"AF_ISCRYS byte — write-only, discarded"`), then stores the *rest of the
line as a single unsplit string* (`d.autorun = scan_str(&mut c)`). Compared
to §3/§9 above: EmuTOS's `process_inf2` reads the analogous byte as
`isgem` and threads it (not discarded) into `sh_main`'s initial mode; and
`sh_wdef` stores an explicit (program-name, directory) **pair**, later
recombined by `sh_chdef`'s `AUTORUN_APP` arm (§9). Neither the isgem
threading nor the two-field split exist in `inf.rs` today. **Root cause,
found while confirming:** `inf.rs`'s `#Z` handling is the **desktop-level**
`app_start`/`app_save` `#Z` line (`deskapp.c`'s own `case 'Z'`, which stores
one combined `a_pappl` path per ANODE — that format genuinely *is* a single
field, no split) — not an implementation of the **AES-level**
`process_inf2` (§3), which does not exist anywhere in this codebase yet
(the plan's own gap list already says so, §3.2 row 8: "`process_inf2` …
fehlt"). So the file is faithfully missing an entire function, not
misimplementing one that exists — a sharper finding than "doesn't match."

---

## Corrections applied to other documents

**`docs/api/aes/scrap-shell.md`** — the header block (`:48-53`) and the
closing "AES↔shell launch loop (reference)" section (`:490-503`) claimed
the shell loop, `sh_chgrf`/`sh_tographic`/`sh_toalpha`, `aes_run_rom_program`
and the launch-loop internals are "reference … NewTOS reimplements freely."
**Corrected in place**: the sentence now points at this document and
narrows the "free" claim to the *mechanism* only, explicitly excluding the
process-identity numbering (`ap_id`, `ap_private`, `p_name`/`p_appdir`) that
this file establishes as contract. See the edit at `scrap-shell.md:48-56`
and `:494-505`.

**`docs/analysis/aes-audit/p1-appl-pd-dispatch.md`**, finding **AUD-P1-004**
(`:78-93`) — its "1:1-Fix" line recommends *"getrennte Räume für Desktop und
EL0-App"* (separate AESPD spaces for the desktop and the first EL0
application) as the correct fix for the pid-1 collision it found. **This is
EmuTOS-contrary for the launched-application case**, per §5/§8 above:
`sh_ldapp` deliberately **renames the one persistent AESPD 0** for both the
desktop and every subsequently launched application via
`p_nameit(rlr,…)`/`p_setappdir(rlr,…)` — it never allocates a second AESPD
for the launched app. A correct 1:1 fix is "one shared descriptor, renamed
sequentially, never concurrently occupied" (true in EmuTOS because the
desktop *terminates* before the app starts, plan §2.2) — not "separate
descriptors." A correction note has been added directly under
AUD-P1-004 in `p1-appl-pd-dispatch.md` citing this file.

---

## Open questions / genuinely ambiguous points

- **§10, the fate of the CPDATA prefix if NewTOS never ships a control
  panel.** EmuTOS itself has no consumer for these 128 bytes either — they
  exist purely for compatibility with a hypothetical real TOS control
  panel. Whether NewTOS's desktop must actually reserve/round-trip them (as
  opposed to treating `CPDATA_LEN` as vestigial and starting INF text at
  offset 0) is a **product decision**, not something the source resolves —
  I have documented the EmuTOS behaviour precisely; whether to replicate a
  compatibility provision for software that doesn't exist in this project's
  scope is for the lead, not for this document, to decide.
- **§4, the redundant `rlr->p_pid = curpid++` line** (`geminit.c:876`)
  writes `p_pid = 0` to a slot that the loop immediately above it already
  set to `0` on its last iteration (`i == 0`). This looks like dead/vestige
  code (the `curpid` cursor is otherwise unused until `iprocess` three
  lines later) rather than a behaviourally significant statement — noted,
  not resolved, since it produces no observable difference either way.
