# `deskpro.c` — program-launch wrapper (`pro_run`/`pro_exec`)

> Phase-00 extraction, Track A. **New 2026-07-21** (Researcher task: chunk
> re-validation of `deskdir.md`/`deskfun.md`/`deskact.md`, which asked to
> check `deskpro.c`/`.h` "program launch / `do_open`" for coverage).
> Source: EmuTOS @ `1a7484ab`, `desk/deskpro.c` (65 lines) +
> `desk/deskpro.h` (16 lines) — the **entire** file, quoted in full below
> since it is short. Previously listed as "out of scope" in
> [`_overview.md`](_overview.md) with no extraction; this closes that gap
> for the small, self-contained part of the launch chain that lives in
> `deskpro.c` itself. **Does not** close the larger gap: `do_aopen`/
> `do_open` (`desksupp.c`), the actual caller, remain unextracted — see
> [`deskfun.md`](deskfun.md#dependencies) "Missing coverage" note.
>
> **Contract vs. internal** (`docs/COMPATIBILITY.md`): `pro_run`/
> `pro_exec` are desktop-application-internal glue — no GEMDOS/AES/VDI
> trap takes them as a parameter. What *is* contract is the single AES
> call they make: `shel_write(SHW_EXEC, isgraf, isover, pcmd, ptail)`,
> already contract-documented in `docs/api/aes/scrap-shell.md` (`Shel_write`
> (121), `SHW_EXEC` mode, `sh_nextapp`/`D.s_cmd`/`D.s_tail` semantics). A
> NewTOS reimplementation of `dir.rs`'s launch path must call the
> equivalent of `shel_write(SHW_EXEC,…)` with the **same argument
> preparation** documented below (the command-tail length-prefix byte,
> the trailing `0x0D`), not just "start the program somehow."

---

## EmuTOS source

**Primary locations (public, `deskpro.h`):**
- `deskpro.c:52-64` `pro_run(isgraf, cmd, tail, wh, curr)` — the only
  exported function

**Helpers called (this file, `static`):**
- `deskpro.c:35-45` `pro_exec(isgraf, isover, pcmd, ptail)` — thin
  `desk_busy_on()` + `shel_write(SHW_EXEC,…)` wrapper; turns the busy
  cursor back **off** only if the call failed (`!ret`) — on success the
  busy cursor is left **on**, since a successful `SHW_EXEC` means the
  desktop process is about to terminate and hand control to the AES to
  start the next program (the busy cursor state is irrelevant at that
  point, the desktop doesn't get to draw again)

**Helpers called (other files, cited — not re-derived here):**
- `aesbind.c`/`gembind.c` `shel_write()` — the AES trap wrapper; contract
  documented in `docs/api/aes/scrap-shell.md`
- `desksupp.c` `desk_busy_on()`/`do_wopen()` — `desk_busy_on` is cited
  only (not re-derived); `do_wopen(FALSE, wh, curr, &G.g_desk)` **is**
  traced below (it is the one piece of `desksupp.c` this function calls
  directly)

**System variables / globals read:** none directly (all state comes in
via parameters)

**System variables / globals written:** none (no `G.` field is touched
by this file)

**Build configuration affecting this function:** none — no
`#if CONF_WITH_*` gating anywhere in `deskpro.c`.

---

## Source extraction summary

`deskpro.c` is the smallest file in `desk/` (65 lines) and sits at the
very bottom of every launch call chain in `deskfun.c`/`deskmain.c`: every
place that "opens" (runs) a program — `do_aopen`'s several call sites in
`desksupp.c` (not itself extracted, see [`deskfun.md`](deskfun.md)'s
"Missing coverage" note), and `deskmain.c:753`'s direct call to launch the
console (`DEF_CONSOLE`) — ultimately calls `pro_run()`, which does exactly
two things: (1) format the command-tail buffer into the shape
`shel_write`'s `SHW_EXEC` mode expects, and (2) call `pro_exec()`, which
issues the `shel_write` trap itself and manages the busy cursor around it.

```c
static WORD pro_exec(WORD isgraf, WORD isover, char *pcmd, char *ptail)
{
    WORD ret;

    desk_busy_on();

    ret = shel_write(SHW_EXEC, isgraf, isover, pcmd, ptail);
    if (!ret)
        desk_busy_off();
    return ret;
}


WORD pro_run(WORD isgraf, char *cmd, char *tail, WORD wh, WORD curr)
{
    WORD ret, len;

    tail[0] = len = strlen(tail+1);
    tail[len+2] = 0x0D;     /* follows the nul byte, just like Atari TOS */
    ret = pro_exec(isgraf, 1, cmd, tail);

    if (wh != -1)
        do_wopen(FALSE, wh, curr, &G.g_desk);

    return ret;
}
```
(`deskpro.c:35-64`, quoted verbatim — the whole non-comment body of the
file.)

**`pro_run`'s tail-buffer convention** (`:56-57`): the caller
(`do_aopen`, always) has already written the actual command-line text
starting at `tail[1]` (a NUL-terminated C string) — `pro_run` itself:
1. Computes its length (`strlen(tail+1)`) and writes it as a **length-
   prefix byte** into `tail[0]` — this is the classic GEMDOS/TOS
   "counted string" convention for a command tail (as used by
   `Pexec`/`BASEPAGE.p_cmdlin` and `Fsetdta`-adjacent APIs elsewhere in
   TOS): byte 0 = length, bytes 1.. = the text, no built-in NUL
   requirement for the *consumer* of this convention.
2. Writes `0x0D` (carriage return) at `tail[len+2]` — **one byte past**
   the string's own NUL terminator (`tail[1+len]` is the NUL from the
   caller's `strcpy`, so `tail[len+2]` is the byte after that) — the
   source comment explicitly notes this mirrors **real Atari TOS**
   behaviour ("follows the nul byte, just like Atari TOS"), i.e. this is
   not an EmuTOS invention but a documented compatibility requirement:
   the command-tail buffer handed to the next program via `Pexec`'s
   `p_cmdlin` must have this exact 3-part shape (length byte, text+NUL,
   trailing CR) for full Atari-TOS-alike compatibility, even though the
   length-byte + NUL already fully describe the string for any
   `strlen`-based consumer.
3. Calls `pro_exec(isgraf, 1, cmd, tail)` — note **`isover` is
   hardcoded to `1`** at this call site; `pro_exec`'s `isover` parameter
   (`SHW_EXEC`'s third `int_in` word, `iscr` in `shel_write`'s own
   signature) is **never** passed through as a variable from any
   `pro_run` caller — every launch from the desktop always requests
   "overwrite" mode (per `docs/api/aes/scrap-shell.md`'s own
   `shel_write` parameter table, this is the `isover`/`iscr` slot; its
   exact semantic meaning is documented there, not re-derived here).
4. **After** the exec call (regardless of success/failure — `ret` is not
   checked here), if `wh != -1`, calls `do_wopen(FALSE, wh, curr,
   &G.g_desk)` — re-opens/redisplays the window `wh` at its full-desktop
   size (`&G.g_desk`) with `full=FALSE`. This runs on the assumption that
   `shel_write(SHW_EXEC,…)`'s *return value* only reports whether the
   request was *accepted* (queued for the AES to act on when the desktop
   process next terminates), not whether the launched program has
   already run — i.e. `pro_run` does not distinguish "exec accepted, will
   run after desktop exits" from "exec rejected" for the purpose of this
   post-call `do_wopen`; per `docs/api/aes/scrap-shell.md`'s note that
   "`shel_write` always returns `TRUE`" (D-054), this call is effectively
   unconditional in practice.

**Callers pass `wh=-1` to skip the `do_wopen` step** — e.g.
`desksupp.c:951`/`:986`/`:1001`'s `do_aopen` call sites all pass
`G.g_cwin` (the current window handle, could legitimately be `-1`? not
verified here — `G.g_cwin`'s value space is `_data-model.md`'s concern,
not re-derived) and `deskmain.c:753`'s console launch passes `wh=-1`
explicitly, skipping the re-open entirely (makes sense: launching the
console via a menu/hotkey, not a window-drag, has no "window to restore"
context).

---

## Edge cases discovered in source

- **The busy cursor is only turned off on the *failure* path**
  (`pro_exec`, `:42-43`) — on success, `desk_busy_on()`'s effect is left
  in place, on the (implicit) assumption that the desktop process is
  about to exit anyway (a successful `SHW_EXEC` means AES will terminate
  the desktop and start the next program once this call returns and the
  desktop's main loop notices `exit_desktop`/the equivalent exit
  condition upstream in `deskfun.c`/`deskmain.c` — not re-derived here).
  If a caller somehow keeps running the desktop after a "successful"
  `pro_run` without itself calling `desk_busy_off()`, the hourglass
  cursor would remain stuck on — worth a regression check for whichever
  NewTOS caller ends up owning this state transition.
- **The tail-buffer's trailing `0x0D` write has no bounds check against
  the buffer's actual allocated size** — `tail[len+2]` is written
  unconditionally; the caller (`do_aopen`, `desksupp.c`, out of scope for
  this task) is responsible for ensuring `G.g_work` (the buffer passed as
  `tail` from every call site, per `deskfun.c`'s citations of
  `do_aopen(..., G.g_work, ...)`... actually `desksupp.c`'s own `do_aopen`
  builds `G.g_work` itself; not independently re-verified here that its
  size always accommodates `len+3` bytes) is large enough. Flagged as
  **needs deeper analysis** if a NewTOS reimplementation's tail buffer is
  ever sized differently from EmuTOS's `G.g_work` (`_data-model.md`'s
  `GLOBES` section, if it documents `g_work`'s size — not checked in this
  task).

---

## Error conditions

`pro_run`/`pro_exec` propagate `shel_write`'s return value verbatim; per
`docs/api/aes/scrap-shell.md` (D-054), `shel_write` **always returns
`TRUE`** in EmuTOS's own implementation, so the `if (!ret)` failure branch
in `pro_exec` (`:42-43`) is — per that citation — dead code in practice
(never taken), though it is not removed from the source and a
reimplementation is free to keep or drop the corresponding defensive
branch.

| Condition | Source location | Result |
|---|---|---|
| `shel_write` returns falsy (per `scrap-shell.md`, does not happen in EmuTOS itself) | `deskpro.c:42-43` | `desk_busy_off()`, `ret` (falsy) propagated to `pro_run`'s caller |

---

## Verification criteria

- **Command-tail shape**: given a command-line string of length `len` at
  `tail[1]`, after `pro_run` returns, `tail[0]==len`, `tail[1..1+len]` is
  the unchanged string, `tail[1+len]==0` (the caller's own NUL, untouched
  by `pro_run`), and `tail[len+2]==0x0D`.
- **`isover` is always `1`** for every desktop-initiated launch (no code
  path in `deskpro.c` ever passes a different value) — a NewTOS launch
  path that needs a different `Shel_write` "overwrite" mode for some
  case would be diverging from EmuTOS, not matching it.
- **`wh==-1` skips the post-launch `do_wopen`** entirely — verify no
  window-restore call happens for the console-launch path
  (`deskmain.c:753`).

---

## Dependencies

### Requires (must be initialized / functional)
- `docs/api/aes/scrap-shell.md` — `Shel_write`/`SHW_EXEC` (the AES trap
  contract)
- `desksupp.c` — `desk_busy_on()`/`desk_busy_off()`/`do_wopen()` (cited,
  not re-derived; `do_wopen` is out of scope, part of the
  `desksupp-redraw.md`-adjacent "rest of `desksupp.c`" gap)

### Used by
- [`deskfun.md`](deskfun.md) — `fun_win2win`, `fun_file2desk` (both via
  `do_aopen`, `desksupp.c`, not `pro_run` directly — see that doc's
  "Missing coverage" note for the un-extracted link in between)
- `deskmain.c:753` — direct call to launch the console (`DEF_CONSOLE`),
  not via `do_aopen` at all (see `deskmain.md`)

## Related data structures

- `docs/api/desktop/_data-model.md` — `GLOBES.g_work` (the tail buffer
  every call site passes as `tail`) — **not independently re-verified in
  this task** whether `_data-model.md` documents `g_work`'s size/lifetime
  in enough detail to confirm the bounds-safety of `pro_run`'s
  `tail[len+2]` write; flagged as **needs deeper analysis** if the 1:1
  rebuild's buffer differs in size from EmuTOS's.

## Cross-references

### Atari Compendium / Profibuch
Not consulted for this entry — `pro_run`/`pro_exec` are desktop-internal
glue around the already Compendium-documented `Shel_write` GEMDOS/AES
call; the compatibility-relevant surface (the tail-buffer shape) is a
**documented Atari TOS convention** per the source's own comment
(`deskpro.c:57`, "just like Atari TOS"), not independently cross-checked
against Compendium/Profibuch text in this task.

## Configuration / version notes

No `#if CONF_WITH_*` gating anywhere in `deskpro.c`. No TOS-version-
specific behaviour identified (EmuDesk-specific file, but the tail-buffer
convention it implements is asserted by the source comment to match
original Atari TOS `Pexec`/`Shel_write` behaviour).

---

## NewTOS implementation notes

**Implemented 2026-09-23 (DESK06)** as
[`programs/desktop/src/pro.rs`](../../../programs/desktop/src/pro.rs) — `pro_exec`,
`pro_run`, plus `desk_busy_on`/`desk_busy_off` (`desksupp.c:1840-1852`),
which had no home in the tree before and whose first caller is `pro_exec`.
The one AES call is a real trap: the new `shel_write` aesbind
wrapper (since DESK43 U6b in `programs/newtos-gem/src/aes.rs`; `svc #2`, opcode 121,
 `AES_CTRL_CODE(SHEL_WRITE, 3, 1, 2)` —
`desk/gembind.c:907-915`), like every other AES call the desktop makes.
The board run's AES trace shows it as
`[aes] pid=0 op 121 in=(1,0,1,0,0,0) out=(1,…)` =
`shel_write(SHW_EXEC, isgem=0, isover=1, "EMUCON", tail)`
(`measurements/measurement-2026-09-23-desk06/serial/cli-serial.log:98`).

The argument preparation this document calls out as the compatibility-
relevant part is reproduced literally: `tail[0] = strlen(tail+1)`,
`tail[len+2] = 0x0D` "just like Atari TOS" (`deskpro.c:56-57`), and the
busy cursor is turned **off only on failure** (`:41-43`) — on success the
hourglass stays up until the desktop is ready for input again.

Two named differences, both from the `tail` buffer this doc's
"Related data structures" note asks about:

- **The tail buffer is a `[u8; CMDTAILSIZE]` local, not `G.g_work`.**
  `CMDTAILSIZE` (128, `aes/gemlib.h:97`) is what `shel_write` copies into
  `ad_stail`, so the buffer the AES reads is exactly as big as the image it
  takes. The length is **clamped** to `CMDTAILSIZE - 3` so the length byte,
  the text, its NUL and the CR all fit; EmuTOS has no such bound (`G.g_work`
  is the same 128 bytes and `tail[len+2]` writes past on a maximal tail).
  For every tail that fits — which is every tail a caller can legally pass —
  the two are identical. This closes the "needs deeper analysis" flag above:
  the bound is in the reimplementation, not inherited.
- **`pro_run` does not carry `(wh, curr)`.** EmuTOS's second half is
  `do_wopen(FALSE, wh, curr, &G.g_desk)` (`:59-60`), the un-zoom of the icon
  a double-click opened. NewTOS's double-click does not go through `pro_run`
  yet (it calls the loader directly — `main::launch_native`, AES gap G1) and
  `do_wopen` has no counterpart in the tree, so the only caller is `CLIITEM`,
  which passes `wh = -1` and skips the step (`deskmain.c:753`). Carrying two
  dead parameters and an unreachable branch would be worse than naming the
  gap; they come back with `do_wopen` when the double-click path is routed
  through here.

### The current-directory step (DESK18, 2026-09-23)

`set_default_path` (`desksupp.c:1785-1801`) joined `desk_busy_on`/
`desk_busy_off` in [`programs/desktop/src/pro.rs`](../../../programs/desktop/src/pro.rs)
for the same reason they are there: it belongs to `desksupp.c`, it has no home
of its own in the tree, and the module that starts programs is where its
caller lives.

```c
desk_busy_on();
dos_sdrv(path[0]-'A');
rc = (WORD)dos_chdir(path);
desk_busy_off();
return rc;
```

Both calls are real GEMDOS traps through
`desk::dosbind` (since DESK43 U6b [`programs/newtos-gem/src/gemdos.rs`](../../../programs/newtos-gem/src/gemdos.rs)): `dos_sdrv`
 was already
there (the desktop reads the drive map with it, `deskapp.c:714`), `dos_chdir`
= `Dsetpath` (0x3B, `gemdos.h:95-98`) is new. `path[0]-'A'` is EmuTOS's own
drive derivation, taken verbatim — which is why the argument must carry its
`X:` prefix. An unmounted drive and a missing directory both come back as
`EPTHNF` (`docs/api/gemdos/directory/3b-dsetpath.md`, which folds `ckdrv`'s
failure into `findit`'s), and that is the one value the caller checks.

**The caller is `do_aopen`'s path choice**, `desksupp.c:893-916`, reimplemented
as `desk::main::set_launch_dir(app_dir, own_path)`. NewTOS splits `do_aopen`
over two functions — `main::open_item`'s `OpenTarget::App` arm (a program
selected directly, EmuTOS's `else` branch, switch = `G.g_appdir`) and
`main::open_document` (a data file with an installed application, EmuTOS's
`is_installed(pa)` branch, switch = the association's `AF_APPDIR`) — so the
shared body takes the switch as a parameter. On failure it raises `STDEFDIR`
(`desk_rsc.c:2558-2559`) through `dir::fun_alert` and the caller does **not**
launch, exactly as `do_aopen` returns `FALSE` there. Note the order, which the
alert's identity depends on: the directory is set *before* the
`file_exists(pa->a_pappl)` check (`:933`), so an association pointing at a
missing drive reports "failed to set default directory" and not "file not
found".

One named simplification: for the non-installed arm EmuTOS passes the
**window's search spec** (`C:\SUB\*.*`) as `pathname` and NewTOS passes the
program's own full path (`C:\SUB\PROG.NCP`). `filename_start`
(`util/optimize.c:226-235`) cuts both back to the identical `C:\SUB\`, and
that is the only thing the function does with the string.

**What this does not cover.** `pro_run` still does not set a directory (it
does not in EmuTOS either) and `crate::launch::run_program` still does not —
the step is `do_aopen`'s, and `\AUTO\` programs and the `#Z` autorun reach
their directory by other routes (`sh_chdef`'s `set_drvdir`,
`aes/gemshlib.c:526-566`, which NewTOS's `tos-api/src/aes/shell.rs` already
has). The `\AUTO\` folder run has no `set_drvdir` equivalent at all; it is
out of this ticket's scope and named in `TASK/DESK18-STATUS-DONE.MD`.
