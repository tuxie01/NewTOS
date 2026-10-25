# AES Application Manager (`appl_*`) — opcodes 10–19

> **PHASE-00b D8, Batch 1 (full-contract refinement).**
> Source: EmuTOS @ `1a7484ab`.
> Primary: `aes/gemaplib.c` (`ap_*`); dispatch arm `aes/gemsuper.c:87-123`
> (`crysbind`); message pipe `aes/gemqueue.c`; process descriptors
> `aes/gempd.c`; opcode + slot macros `include/aesdefs.h`.
>
> Call convention + AESPB layout: [`_dispatcher.md`](./_dispatcher.md).
> Shared AES globals (ap ids, the message pipe, `rlr`/`D`):
> [`_state.md`](./_state.md).
>
> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md).
> **Contract:** opcode numbers (10–19); the `int_in`/`addr_in` slot layout of
> each call; `int_out[0]` = return code; the `global[]`/`AESGLOBAL` field
> layout and the exact values `appl_init` writes (`ap_version`, `ap_count`,
> `ap_id`, `ap_private`, `ap_ptree`); the 16-byte standard message; the
> 8-char space-padded application-name format used by `appl_find`; the -1
> "not found" / "unsupported opcode" returns. **Reference:** `rlr`/`AESPD`,
> the `p_queue`/`p_qindex` pipe internals, the `QPB`/`ev_block`/`iasync`
> scheduler machinery, the fork/`EVNTREC` tape internals — NewTOS reimplements
> these on its own process/IPC model.

---

## AES process identity and pid

> **2026-09-20, A62 + A56.** Source: `aes/gempd.c` (`pd_index`, `fpdnm`,
> `getpd`, `iprocess`), `aes/geminit.c:236-257,850-888` (accessory/SCRENMGR
> allocation, the AESPD init loop), `aes/gemshlib.c:572,592-689` (`sh_ldapp`,
> the "not the AESPD" comment), `aes/gemwmlib.c:851-863,904,1255,1290,1377,
> 1579-1580` (`ap_sendmsg`, `w_redraw`, `w_setup`, `WF_OWNER`), `aes/gemctrl.c`
> (control-manager message posters), `aes/gemevlib.c:100-117` (`ev_mesag`'s
> fast path), `aes/gemaplib.c:54-63,251-261` (`ap_init`/`ap_exit`),
> `aes/struct.h:36`, `include/sysconf.h:37`, `include/aesdefs.h:509` (`WM_OX`).
> Full derivation: `docs/analysis/aes-process-table-and-apid-2026-09-20.md`.

### EmuTOS's model

EmuTOS keeps a **fixed** table of `NUM_PDS = NUM_ACCS + 2` process descriptors
(`aes/struct.h:36`; `NUM_ACCS = 6`, `include/sysconf.h:37` → 8 `AESPD`s at
most). The **live** count is `totpds = num_accs + 2` (`geminit.c:819`) — with
zero accessories loaded, exactly **2**. The table is two arrays,
`D.g_int[2]` (indices 0-1) and `D.g_acc[num_accs]` (indices 2+);
`pd_index(i)` maps an index to its descriptor (`gempd.c:32-35`).

| pid | Descriptor | Who runs on it | Assigned |
|----:|-----------|-----------------|----------|
| 0 | `D.g_int[0]` | the **entire application chain**: the desktop, every program the shell loop launches via `sh_ldapp`, and every child that program starts with `Pexec`/`dos_exec` — all of them, because `rlr` is set once at `gem_main` init and is **never reassigned** by any nested `Pexec`/`dos_exec` (`geminit.c:850-877`; only `p_name`/`p_appdir` are overwritten in place by `sh_ldapp`, `gemshlib.c:604-605,634-636`, each new launch) | init loop, `geminit.c:851-877` |
| 1 | `D.g_int[1]` | the **Screen Manager** (`ctlmgr`) — a genuine second cooperative process that owns window-furniture clicks and menu-bar tracking: `gl_mowner = ctl_pd = iprocess("SCRENMGR", ctlmgr)` | `geminit.c:888` → `getpd()`, `gempd.c:60-70` |
| 2… | `D.g_acc[i-2]` | desk accessories, one per **successfully loaded** `.ACC` file (a failed load takes no pid, see below) | `load_one_acc`, `geminit.c:236-257` → `getpd()` |

`getpd()` (`gempd.c:60-70`) has exactly **two callers** in the whole AES:
`iprocess` (used once, for SCRENMGR) and `load_one_acc`. No other code path
ever allocates an `AESPD`. A GEMDOS-level `Pexec` is entirely invisible to
this table — it is a BDOS call, not an AES one; `aes_run_rom_program`'s own
comment says so explicitly (`gemshlib.c:572`, `/* this is the BDOS PD
structure, not the AESPD */`). So "the application" in AES terms is not one
program but the whole nested `Pexec` chain running on the one persistent
descriptor 0 — already established for the desktop/first-launched-app case in
[`shell-loop.md`](./shell-loop.md) §4-§5 (2026-09-01); this section extends
it to arbitrary `Pexec` nesting depth and adds the Screen-Manager/`msg[1]`
half of the picture.

`fpdnm` (`gempd.c:38-57`) — the lookup behind both `appl_find` (name → pid)
and `appl_write`/`appl_read`'s target resolution (`fpdnm(NULL, AP_RWID)`,
`gemsuper.c:102-105`) — walks all `totpds` entries with **no liveness test**,
only `p_name`/`p_pid` comparison. Consequences:

- `ap_init` (`gemaplib.c:54-63`) is three unconditional statements — **it
  cannot fail**; there is no capacity check, because the descriptor was
  allocated once at boot, before any application could call it.
- `ap_exit` (`gemaplib.c:251-261`) frees nothing: it drains the queue and
  clears `AP_OPEN` only; `p_pid`/`p_name`/the rest of the flags survive — a
  static array slot, not a heap allocation, has nothing to release (already
  spec'd under `appl_exit` below; NewTOS's A29 fix reproduces exactly this).
- A target of pid 0 for `appl_write`/`appl_read` always resolves to
  `D.g_int[0]` (`fpdnm`, `gempd.c:43-52`) — **independent of who the caller
  is and of which nested chain member is "logically" running**. The whole
  chain shares that one descriptor and therefore **one message queue**
  (`p_queue`, `struct.h:152`). (Corrected 2026-10-17, A113: this bullet used
  to say pid 0 "unambiguously means whichever chain member currently holds the
  CPU" and that EmuTOS's AES is "strictly single-threaded". It is not: the AES
  is cooperatively multi-process — `dsptch()` runs `SCRENMGR` and every
  accessory on their own `AESPD`s — so the *caller* of `appl_write(0, …)` may
  be an accessory, and pid 0 is then **not** the caller. Only a *chain member*
  calling `appl_write(0, …)` writes into its own, shared queue.)
- `appl_find(name)` for the chain can only ever report the **current**
  occupant's name, because `p_nameit` overwrites `p_name` in place on every
  launch (`gemshlib.c:604-605,634-635`) — a previously-launched program's name
  is gone, not a second table entry (measured: `docs/analysis/
  aes-process-table-and-apid-2026-09-20.md` §1a, "nine different programs" run
  — `appl_find(previous) = -1` every time). Accessory names are set once at
  load (`pstart` → `p_nameit`, `gempd.c:110-121`) and stay for the session, so
  `appl_find("ACCNAME ")` returns the accessory's pid (2…) for as long as the
  AES runs, whether or not it is still alive/blocked.

### Pid assignment, accessories, and a failed load (A113, 2026-10-17)

Source-read this session: `geminit.c:236-257,805-894`, `gempd.c:26-121`,
`util/gemdos.c:29-45` (`pgmld`).

1. **Order in `gem_main`.** `count_accs()` fills `acc[].name` in directory
   order (`dos_sfirst`/`dos_snext` over `\*.ACC`, at most `NUM_ACCS = 6`;
   `0` if `BOOTFLAG_SKIP_AUTO_ACC`, `geminit.c:264-282`); `num_accs` descriptors
   are `dos_alloc_anyram`'d and zeroed — **if that allocation fails, `num_accs`
   is reset to 0** (`:810-817`); `totpds = num_accs + 2` (`:819`); the init loop
   sets `p_pid = i` for **every** descriptor, top-down (`:851-877`);
   `curpid = 0; rlr->p_pid = curpid++` leaves `curpid == 1` (`:875-876`);
   `iprocess("SCRENMGR", ctlmgr)` → `pstart` → `getpd()` takes `curpid == 1`
   (`:888`, `gempd.c:60-70`); only **after** that does `aes_run_rom_program(
   accdesk_start)` (`:894`) run `run_accs_and_desktop`, whose `load_accs`
   (`geminit.c:687`) calls `load_one_acc` per name, in `acc[]` order.
2. **Accessory pid = `2 + number of successful loads before it`**, not
   `2 + directory index`. `getpd()` is called from `pstart` only
   (`gempd.c:110-114`), and `load_one_acc` reaches `pstart` only when
   `dos_open` succeeded (`ret >= 0`) **and** `pgmld` did not return `-1`
   (`geminit.c:246-255`). `pgmld` returns `-1` when `Pexec(PE_LOAD, …)` fails
   or `Mshrink` fails (`util/gemdos.c:33-43`). Same name order = load order =
   pid order for the survivors.
3. **A failed load consumes no pid and leaves no trace but a dead
   descriptor.** `curpid` is not advanced, so the next successful accessory
   takes the pid the failed one would have had. The trailing descriptor(s)
   that are never handed out keep the state of the init loop: `p_pid = i`
   (their index), `p_name` = 8 blanks (`geminit.c:866`), empty queue,
   `p_flags = 0` (`bzero`, `:816`), never put on `drl`, no `psetup`. Consequences, all from
   `fpdnm`'s lack of a liveness test (`gempd.c:38-57`): such a pid still
   resolves for `appl_write`/`appl_read` (the write succeeds into a queue
   nobody reads; the 9th 16-byte message blocks the writer for good), the
   all-blank name `"        "` resolves in `appl_find`, and `wait_for_accs`
   (`geminit.c:599-628`) finds a descriptor without `AP_MESAG`/`AP_ACCLOSE`
   ever set and therefore runs its full `WAIT_TIMEOUT` loop before giving up
   (inference from reading the loop; not measured). Whether the pgmld-failed
   case that has *already* loaded memory (`Mshrink` failure after `Pexec`)
   leaks that memory is not analysed here.
4. **`pid >= totpds`** (e.g. `appl_write(4, …)` with two accessories loaded):
   `fpdnm` returns `NULL`, `aqueue` then dereferences `p->p_qindex`
   (`gemqueue.c:94,97,99`) — undefined behaviour, **not** a `0` return (see
   "What `appl_write` returns" below). Any `0` a NewTOS gives here is a
   NewTOS safety choice, not EmuTOS behaviour.
5. **`menu_register` reports the menu id, not the pid.** `mn_register(pid,
   pstr)` (`gemmnlib.c:789-816`) uses `pid` only for the test `pid >= 0`; the
   owner stored is `desk_ppd[openda] = rlr` (`:807`) and the return value is
   `openda`, the slot index `0..NUM_ACCS-1` (`:805-811`), or `-1`. It does not
   resolve or report the AES pid.
6. **`msg[1]` is the sender's `rlr->p_pid` at send time** (`ap_sendmsg`,
   `gemwmlib.c:851-862`): 1 for messages posted from `ctlmgr`
   (`gemctrl.c:84,125`), the *caller's* pid for messages posted inside the
   caller's own trap — `WM_REDRAW` from its own `wind_open`/`wind_close`/
   `wind_set` (`gemwmlib.c:904`), `AC_CLOSE` from `mn_cleanup` inside the
   exiting program's `appl_exit` (`gemmnlib.c:747`) = pid 0 for a chain
   member, ≥ 2 if an accessory itself triggers the redraw. The non-queued
   `WM_ARROWED` synthesised by `ev_mesag` carries the **reader's** pid
   (`gemevlib.c:105`).
7. **`WF_OWNER`** returns `pwin->w_owner->p_pid` in `int_out[1]` and the
   is-open flag in `int_out[2]` (`gemwmlib.c:1579-1582`, `poutwds` =
   `&WM_OX`, `gemsuper.c:343`). `w_owner` is `rlr` at `wind_create`
   (`w_setup(rlr, i, kind)`, `gemwmlib.c:1377`), so a window created by an
   accessory reports **that accessory's pid (≥ 2)**; the desktop window
   (`DESKWH`) is owned by whoever ran `wm_init` (`ppd = rlr`,
   `gemwmlib.c:1250-1255,1290`), i.e. pid 0 in the normal boot/launch path.

### NewTOS contract, in two layers

**(a) Identity (internal — not observed by a program).** A NewTOS AES
client's *identity* — message queue, `AP_*` flags, name, wait-key, resource
list (A39, `resource.rs` `RscEntry`), and window ownership (`Window.owner`) —
is keyed by its **GEMDOS process slot**, plus one reserved index for the
kernel-context desktop (table size `MAX_PROCESSES + 1`). Because every GEMDOS
slot gets its own descriptor by construction, opening one can never fail on
capacity grounds: **`appl_init` never returns `-1`** for "table full" —
matching EmuTOS's "cannot fail" contract exactly, and for the same
structural reason (no capacity check exists), not because NewTOS happens to
provision enough slots.

**(b) Reported value (external — what a program reads back).** The *value*
`appl_init`/`ap_id` (`global[2]`), `appl_find`, `wind_get(WF_OWNER)`
(`int_out[1]`, `gemwmlib.c:1580`), the `appl_write`/`appl_read` target, and
`msg[1]` of the 16-byte standard message report is **computed from the
identity**, not equal to it, and follows EmuTOS's numbers (rewritten
2026-10-17, A113: the previous text knew no accessories and gave target 0 the
meaning "the currently running client"; both were wrong against
`geminit.c:236-257,851-877`, `gempd.c:38-70`):

- **0** for every member of the application chain — the kernel-context
  desktop, the program it launches, and any `Pexec` child at any nesting
  depth. `ap_id == 0` is already contract, established in
  [`shell-loop.md`](./shell-loop.md) §4-§5 and
  [`scrap-shell.md`](./scrap-shell.md) (2026-09-01) for desktop/first-app;
  this extends it to the full nested-`Pexec` chain (`geminit.c:850-877`,
  `gemshlib.c:572`).
- **1** is the Screen Manager, `SCRENMGR` (`geminit.c:888`, `getpd` →
  `curpid == 1`, `gempd.c:60-70`). In EmuTOS a genuine `AESPD` that is
  addressable: `appl_find("SCRENMGR")` = 1 and `appl_write(1, …)` queue into
  it. In NewTOS no schedulable process sits behind it (H02, `ctl_pd` as a
  flag); it is the **sender stamp** on `msg[1]`, written by `ap_sendmsg`
  (`gemwmlib.c:851-862`, `ap_msg[1] = rlr->p_pid`) while `rlr == ctl_pd`, plus
  the name `appl_find` answers and the `appl_write` target that is not an
  application. Exact call sites, all inside `gemctrl.c`'s `ctlmgr()` context
  via `ct_msgup`/`handle_arrow_msg`: `WM_TOPPED` (`:178`),
  `WM_CLOSED`/`WM_FULLED` (`:210-220`), `WM_SIZED` (`:259`), `WM_MOVED`
  (`:238`), `WM_HSLID`/`WM_VSLID` (`:300-314`), `WM_UNTOPPED` (`:151`),
  `AC_OPEN`/`MN_SELECTED` (`:346-367`), and the **queued** case of
  `WM_ARROWED` (`:125`, taken when the target is `WAITIN`).
- **2, 3, … = desk accessories**, in successful-load order (**not** directory
  index — a failed load frees its pid for the next one; see "Pid assignment,
  accessories, and a failed load" above). `appl_init` of an accessory returns
  and stores its own pid in `global[2]`; `appl_find("ACCNAME ")` returns it
  from any caller; `appl_write(pid, …)` from any caller (chain, accessory,
  another accessory) delivers into **that accessory's** queue and no other;
  `WF_OWNER` of a window the accessory created is its pid; `msg[1]` of a
  message posted inside the accessory's own trap (e.g. its `WM_REDRAW`) is its
  pid.
- **`msg[1]` of `WM_REDRAW` is the pid of whoever's trap posts it**:
  `w_redraw`'s `ap_sendmsg` (`gemwmlib.c:904`) runs from `draw_change`, called
  synchronously from `wm_open`/`wm_close`/`wm_set(WF_CURRXYWH)`
  (`gemwmlib.c:1426,1453,1612`) — i.e. from *inside* the caller's own
  `wind_open`/`wind_close`/`wind_set` trap, so `rlr` is the caller (0 for a
  chain member, ≥ 2 for an accessory), not the Screen Manager. Likewise
  `AC_CLOSE` from `mn_cleanup` carries the pid of the program running
  `appl_exit` (`gemmnlib.c:747`).
- **Exceptionally the reader's own pid** for a **non-queued** `WM_ARROWED`:
  when `handle_arrow_msg` finds the target not currently `WAITIN`, it stashes
  the action directly on the target's `p_msg` instead of sending
  (`gemctrl.c:129-133`); the target's own later `evnt_mesag` call then
  synthesizes the message itself via `ev_mesag`, with `msg[1] = rlr->p_pid`
  (`gemevlib.c:100-117`; see [`event.md`](./event.md) §`evnt_mesag`). The same
  message id therefore carries **1** or the reader's pid (0 for the chain)
  depending on whether the target was already blocked waiting when the arrow
  was clicked.
- **`appl_write`/`appl_read` target** = `fpdnm(NULL, AP_RWID)`
  (`gemsuper.c:102-105`, `gempd.c:38-57`): the descriptor whose `p_pid` equals
  the id, with **no liveness test**. Target **0** is `D.g_int[0]`, the chain's
  one shared queue, **whoever the caller is** — an accessory writing to 0
  reaches the chain; the chain writing to 0 reaches itself. It is *not* "the
  currently running client". (EmuTOS has a single queue for the whole chain;
  how NewTOS maps 0 onto its per-slot identities is an implementation choice,
  see the NewTOS note below.)
- **`WF_OWNER`** (`wind_get`, `int_out[1]`): the owner's pid —
  `w_owner->p_pid` (`gemwmlib.c:1580`), `w_owner` = `rlr` at `wind_create`
  time (`w_setup`, `:1377`): **0** for every window created by a chain
  member, **the accessory's pid** for one created by an accessory; the desktop
  window is owned by whoever ran `wm_init` (`:1255,1290`), pid 0 in the
  normal path.

> **NewTOS state vs. this contract (A113, 2026-10-17).** Before the A113 fix
> `tos-api/src/aes/appl.rs` reported 0 for every client including accessories
> and resolved target 0 to the *caller* (measured:
> `measurements/measurement-2026-10-17-a113-a114/serial/idprobe1.log`). Since
> the fix (`measurements/measurement-2026-10-17-a113-fix/`) `reported_pid` =
> 2 + the accessory's fixed position among the successfully loaded
> accessories (a forgotten accessory leaves a hole, nothing shifts), else 0;
> 1 only as the SCRENMGR stamp. `identity_of(0)` = the caller when it is a
> chain member, otherwise the innermost live chain member that is not an
> accessory; `identity_of(2+n)` = accessory n (or the innermost process it
> `Pexec`ed); a hole, 1, negative values and `pid >= totpds` resolve to
> nothing (`appl_write` returns 0, NewTOS choice, see point 4 above).
> **Inference, not EmuTOS source:** a process `Pexec`ed by an accessory
> reports the accessory's pid, because EmuTOS does not change `rlr` across
> `dos_exec`.

### Consequences for A39 / A28 / H02 / H05 (reasoned, not re-derived here)

- **A39** (resource list keyed by caller) keeps its key *kind* unchanged —
  still "which client owns this entry" — but the key type moves from the
  *reported* pid to the *identity* slot. This is required: giving parent and
  child the same reported value (0) as their resource-list key would make
  `on_process_exit` at the child's `Pterm` delete the parent's entries too
  (the H05 regression). Identity keeps them separate; only the value
  reported back to `appl_init`/`WF_OWNER` collapses to 0.
- **A28** (launch-boundary reset) is unaffected in substance —
  `register`/`flags = 0`/queue-drain still happen at the identity (slot)
  boundary; only the numbers a test compares change, from "the slot" to "the
  reported value."
- **H02** (`ctl_pd` built as a flag, not a process) is exactly the shape this
  section requires: pid 1 is a **stamp**, not a process descriptor —
  `msg[1] = 1` must be written at the poster (the control-manager-equivalent
  code paths above), not derived from "the current process's identity",
  because there is no NewTOS process behind it.
- **H05** (child's return not corrupting the parent) is preserved by the same
  identity/value split as A39: parent and child never share an identity
  slot, so nothing about their resource lists or queues collides, even
  though both report `ap_id == 0` while running.
- **`on_process_exit`** stays keyed by the GEMDOS slot (identity), unchanged
  by this section.

---

## `AESGLOBAL` — the application's AES state (`global[]`)

`aes/gemrslib.h:15-27` (struct) + `include/aesdefs.h:147-157` (the `AP_*`
`global[]` slot macros used by the dispatcher). The `global` pointer of the
AESPB points at this array; `appl_init` writes the six fields marked below.
Byte offsets are the m68k layout (`WORD` = 2 bytes; `LONG`/pointer = 4 bytes,
2-byte-aligned — no padding):

| Byte | `global[]` word | Field (struct) | Type | Filled by `appl_init` | Meaning |
|-----:|:---------------:|----------------|------|:---------------------:|---------|
| 0x00 | `[0]` `AP_VERSION` | `ap_version` | WORD | yes = `AES_VERSION` | AES version (see below) |
| 0x02 | `[1]` `AP_COUNT`   | `ap_count`   | WORD | yes = 1 | max concurrent applications |
| 0x04 | `[2]` `AP_ID`      | `ap_id`      | WORD | yes = `rlr->p_pid` | this application's id (= AES pid) |
| 0x06 | `[3]` `AP_PRIVATE` | `ap_private` | LONG | yes (see note) | free for the application |
| 0x0A | `[5]` `AP_PTREE`   | `ap_ptree`   | LONG (ptr) | no† | ptr to array of resource-tree addresses |
| 0x0E | `[7]` `AP_1RESV`   | `ap_rscmem`  | LONG (ptr) | no | loaded `.RSC` file in memory (internal) |
| 0x12 | `[9]` `AP_2RESV0`  | `ap_rsclen`  | UWORD | no | `.RSC` length (internal) |
| 0x14 | `[10]` `AP_2RESV1` | `ap_planes`  | WORD | yes = `gl_nplanes` | screen colour planes |
| 0x16 | `[11]` `AP_3RESV`  | `ap_3resv`   | LONG (ptr) | yes = `&D` | ptr to the AES global area `THEGLO` (internal) |
| 0x1A | `[13]` `AP_4RESV`  | `ap_4resv`   | LONG | no | reserved (AES 4.00; unused here) |

> **Correction vs. the previous spec:** the fields at and below `ap_rscmem`
> are *internal* (`/* the following are not advertised to applications */`,
> `gemrslib.h:21`). `appl_init` **does** write `ap_planes` (`AP_2RESV1`,
> `global[10]`, byte 0x14) and `ap_3resv` (`global[11]`, byte 0x16); it does
> **not** write `ap_rscmem`/`ap_rsclen`/`ap_4resv` — those are set later by
> `rsrc_load`. See the exact assignment list under `appl_init`.

> **†`ap_private` / `ap_ptree`:** `appl_init` writes the **`ap_private`** slot
> (`global[3]`) with `rs_trees[DESKTOP][ROOT].ob_spec` (`gemsuper.c:95-96`).
> It does **not** initialise `ap_ptree` (`global[5]`) — that field is populated
> by `rsrc_load` when the application loads its resource file. (The previous
> spec wrongly attributed the DESKTOP `ob_spec` to `ap_ptree`; the source
> writes it to `ap_private`.) NewTOS's `rsrc_load` fills `ap_ptree`,
> `ap_rscmem` and `ap_rsclen` since 2026-09-14 (A35) — at the TOS widths
> below, and only when the loaded blob is addressable in four bytes; see
> [`resource-fileselector.md`](./resource-fileselector.md) §`rsrc_load (110)`.
>
> **NewTOS `ap_private` (D-099, closed 2026-09-01, stage S1b).** NewTOS fills
> `ap_private` from the **same object** EmuTOS does: the ROOT of the AES's own
> built-in `DESKTOP` tree, transcribed from `aes/gem_rsc.c:550-570` into
> `tos-api::aes::init` (`desktop_root_obspec()`), read live by `appl_init`.
> Its compiled-in `ob_spec` is the literal `4419L` = **`0x1143`** — the value a
> real TT running EmuTOS reports. At AES boot the `.INF` file's `#Q` line is
> scanned (`process_inf1`, `geminit.c:343-392`) and, if present, its
> plane-appropriate desktop byte replaces the **low byte only**
> (`set_aes_background`, `geminit.c:635-641`), so `ap_private` is
> `0x1100 | patcol`.
>
> An earlier NewTOS revision sourced this from the **WF_NEWDESK-registered**
> desktop-background tree instead. That was wrong: `appl_init` indexes
> `rs_trees[DESKTOP]`, the AES's own compiled-in `gem.rsc` ensemble, while a
> `wind_set(WF_NEWDESK)` tree lands in `gl_newdesk` (`gemwmlib.c:1687`) and is
> read only by `w_drawdesk`/`w_redraw_desktop` — it never reaches `ap_private`.
> The desktop reads `#Q` a **second**, independent time for its own `g_patcol`
> (`desk/deskapp.c:953-960`); two readers of one line is the 1:1 design, not a
> duplication to collapse.

> **NewTOS layout decision (2026-09-13).** `AESGLOBAL` keeps the m68k widths
> above on **every** ABI, including native 64-bit AArch64: 30 bytes, every
> `LONG`/pointer field 4 bytes. It is the one GEM boundary structure that is
> addressed two ways at once — by byte offset (the table above) *and* by WORD
> index (`AP_VERSION`..`AP_4RESV`, `include/aesdefs.h:147-157`; every client
> declares it `short aes_global[]`). Widening the pointer fields to 8 bytes
> would move `ap_rsclen` off `global[9]` and `ap_planes` off `global[10]`,
> i.e. satisfy one half of the contract by breaking the other; the two halves
> hold together only at the m68k widths. This is therefore the named exception
> to [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md) §"Native vs m68k struct
> variants" (`OBJECT` has no index contract and does follow the caller's
> width).
>
> The consequence is carried by the **toolchain**, not by the structure:
> pristine gemlib's accessors have no `__LP64__` arm (`gem.h:41,49,57`), so for
> a 64-bit caller `_AESappglobal`, `_AESrscfile` and `_AESrscmem` read eight
> bytes each — bytes 6..13, 10..17 and 14..21, three pointers inside fourteen
> bytes. `_AESrscmem` in particular reaches `ap_planes`, so on a zeroed array
> it returns `planes << 48`: a non-NULL wild pointer manufactured by the
> reader. NewTOS narrows those three readers to the TOS field widths in
> `sdk/gemlib/overlay/{gem.h,mt_gem.h.in}`; `_AESrsclen` (`global[9]`) and
> `_AESmaxchar`/`_AESminchar` (`[13]`/`[14]`) are WORD readers at the right
> offsets already and are left as gemlib has them. `_AESrscfile`/`_AESrscmem`
> stop being assignable lvalues there (C has no 32-bit pointer type on LP64);
> the overlay adds `_AESrscfile_set(p)` / `_AESrscmem_set(p)` for the write
> side, so an unconverted assignment is a compile error rather than a silent
> 8-byte store across three fields. The m68k build compiles the pristine tree,
> where `long` is already 4 bytes.
>
> Byte-for-byte coverage of what `appl_init` writes and leaves alone:
> `tos-api/tests/aes_global_layout.rs`.

The application's identity is its **AES process id** (`AESPD.p_pid`,
`aes/struct.h:129`). The AES keeps a per-application process descriptor
(`AESPD`, `aes/struct.h:119-155`) holding the 8-char name (`p_name`,
offset 0x0C), the message queue (`p_qaddr`/`p_qindex`/`p_queue[128]`,
`:150-152`), the process link (`p_link`), and flags (`p_flags`, `:136`):
`AP_OPEN = 0x0001` ("between `appl_init` and `appl_exit`"),
`AP_MESAG = 0x0002`, `AP_ACCLOSE = 0x0004` (`aes/struct.h:115-117`).

---

## `AES_VERSION` (the value `appl_init` reports)

`include/config.h:1345-1355`. `AES_VERSION` is **build-configuration
dependent**:

| Value | AES | Selected when |
|------:|-----|---------------|
| `0x0340` | 3.40 | `CONF_WITH_3D_OBJECTS` **and** `CONF_WITH_MENU_EXTENSION` **and** `CONF_WITH_WINDOW_COLOURS` **and** `CONF_WITH_GRAF_MOUSE_EXTENSION` |
| `0x0330` | 3.30 | the last three of the above (no 3D objects) |
| `0x0320` | 3.20 | only `CONF_WITH_GRAF_MOUSE_EXTENSION` |
| `0x0140` | 1.40 | none of the above |

In the **default EmuTOS build** all four flags are `1`
(`include/config.h:1259,1283,1305,1329`), so **`AES_VERSION = 0x0340`**. This
is the value a conforming NewTOS should report from `appl_init` unless a
deliberate ADR pins a different AES level. (Contract note: the *value* is
observable via `global[0]`; the exact figure is a NewTOS decision recorded in
the config notes below, but 0x0340 matches the default reference build.)

---

## `appl_init` (10)

**Source:** `aes/gemsuper.c:90-100` (the `APPL_INIT` arm fills `global[]`
inline) + `aes/gemaplib.c:54-63` (`ap_init` sets the flag and returns the pid).

| Slot | Meaning |
|------|---------|
| `int_in` | (none) |
| `addr_in` | (none) |
| `int_out[0]` | return: application id (`ap_id`), or `-1` on failure |
| `global[]` | filled — see below |

**Behaviour.** The `APPL_INIT` arm writes, in order (`gemsuper.c:92-98`):

- `AP_VERSION` (`global[0]`) = `AES_VERSION` (0x0340 default);
- `AP_COUNT` (`global[1]`) = `1` (the reported max concurrent-application
  count — see the single-app note);
- `AP_ID` (`global[2]`) = `rlr->p_pid` (the running process's pid);
- `AP_PRIVATE` (`global[3]`) = `rs_trees[DESKTOP][ROOT].ob_spec` (the DESKTOP
  resource tree's root `ob_spec`, the desktop background pattern/colour word) —
  **NewTOS:** the same object, from the built-in `DESKTOP` tree in
  `tos-api::aes::init` (`desktop_root_obspec()`): `0x1143`, or `0x1100 | patcol`
  after a `#Q` line (see the `ap_private` callout above and D-099);
- `AP_2RESV1`/`ap_planes` (`global[10]`) = `gl_nplanes` (screen colour planes) —
  **NewTOS:** the raster backend's `planes()` (`screen_planes()`), matching
  VDI `INQ_TAB[4]`. This is not `log2(num_colors)`: the current truecolor
  backend has 16 planes but reports 256 logical colour pens.
- `AP_3RESV`/`ap_3resv` (`global[11]`) = `&D` (address of the AES global area
  `THEGLO`, an internal pointer) — **NewTOS:** the AES process-descriptor table
  address (`tos-api::aes::pd::table_addr()`), the process-management core of our
  AES state. Written as a **4-byte** field at byte 0x16 (no gemlib reader; kept
  4 bytes so the app-readable `ap_maxchar`/`ap_minchar` at `global[13]`/`[14]`
  stay 0 as at EmuTOS `appl_init`; on LP64 the 64-bit pointer is truncated —
  harmless, nothing reads it back).

Then `ap_init()` (`gemaplib.c:54`) sets `rlr->p_flags |= AP_OPEN` and returns
`rlr->p_pid`. That pid becomes `RET_CODE` (`int_out[0]`).

**Repeated calls:** `ap_init` only ORs `AP_OPEN` into the existing descriptor.
It does not clear other flags, the application name, or queued messages
(`gemaplib.c:54-63`). A repeated `appl_init` therefore returns the same id
and refreshes the six global fields while preserving pending messages.
NewTOS must initialise a newly allocated descriptor separately from reopening
an existing one; reopening is not a queue reset.

`rlr` = "run-list root" = the currently running AES process descriptor
(`aes/aesvars.h`; see [`_state.md`](./_state.md)).

> `appl_init` never fails in EmuTOS — `ap_init` (`gemaplib.c:54-63`) is three
> unconditional statements with no failure path at all; there is no capacity
> check because the descriptor was allocated once at AES start-up, before any
> application could call it (see "AES process identity and pid" above).
>
> **Correction (2026-09-20, A62/A56).** The previous wording here — "the
> documented `-1` failure return is the contract for a NewTOS that *can* fail
> to allocate an application slot" — **licensed a divergence** from EmuTOS
> instead of naming it. EmuTOS's contract is "never `-1`", full stop. A
> NewTOS `appl_init` that can return `-1` under input a real program can
> produce (e.g. more than `MAX_APPS` concurrently open descriptors) is not
> EmuTOS-faithful; per the identity model above (table sized
> `MAX_PROCESSES + 1`, one entry per GEMDOS slot by construction), `appl_init`
> should have no capacity-`-1` path either. Measured: `appl_init` returns
> `-1` today at `Pexec` nesting depth 9
> (`docs/analysis/aes-process-table-and-apid-2026-09-20.md` §1b) — this is
> the defect A62 names, not a documented allowance.

## `appl_read` (11) / `appl_write` (12)

**Source:** `aes/gemsuper.c:101-105` → `aes/gemaplib.c:69-89` (`ap_rdwr`);
pipe mechanics `aes/gemqueue.c` (`aqueue`/`doq`). The inter-application
**message pipe**.

| Slot | Name | Meaning |
|------|------|---------|
| `int_in[0]` | `AP_RWID` | target/own application id (the *pid*) |
| `int_in[1]` | `AP_LENGTH` | byte count |
| `addr_in[0]` | `AP_PBUFF` | message buffer (`WORD *`) |
| `int_out[0]` | — | return: non-zero (1) on success |

**Behaviour.** The dispatcher resolves the id to an `AESPD` via
`fpdnm(NULL, AP_RWID)` (find process descriptor by pid, `gempd.c:38-57`), then
calls `ap_rdwr(code, p, AP_LENGTH, AP_PBUFF)` with `code = MU_MESAG` for
`appl_read` and `code = MU_SDMSG` for `appl_write` (`gemsuper.c:103-104`).

- **`MU_MESAG` (0x0010, `include/aesdefs.h:260`)** = read from *its own* pipe.
- **`MU_SDMSG` (0x0040, `aes/gemlib.h:25`)** = send to *another* app's pipe.

`ap_rdwr` (`gemaplib.c:69`) has a **fast path**: when `code == MU_MESAG`, the
pipe currently holds exactly `p->p_qindex == length == 16` bytes, it `memcpy`s
16 bytes to the buffer, clears `p_qindex`, and returns `1` (`gemaplib.c:77-82`).
Otherwise it builds a `QPB` (`qpb_ppd`/`qpb_cnt`/`qpb_buf`, `aes/struct.h:158`)
and **blocks** via `ev_block(code, &m)` (`gemaplib.c:84-88`) until the queue
operation completes (`aqueue`/`doq` in `gemqueue.c`). The **16-byte** message
is the GEM standard — the `msg[8]` `WORD` array delivered by `evnt_mesag`
(which itself calls `ap_rdwr(MU_MESAG, rlr, 16, mebuff)`, `gemevlib.c:116`).

> **Redraw coalescing (source detail):** on a *write* of a `WM_REDRAW`/
> `WM_ARROWED`/`WM_HSLID`/`WM_VSLID` message, `doq` unions the redraw rectangle
> with a pending matching-handle `WM_REDRAW` already in the queue, or overwrites
> a pending `WM_ARROWED` (`gemqueue.c:44-72`). This is a reference-level
> optimisation of the pipe, not a slot-level contract.

### Blocking behaviour (A75, 2026-09-20)

**The two blocking conditions, precisely.** `ap_rdwr`'s slow path builds a
`QPB` and calls `ev_block(code, &m)` (`gemaplib.c:84-88`) → `mwait(iasync(code,
&m))` → `apret(code)` (`ev_block`, `gemevlib.c:64-69`). `iasync` routes
`MU_MESAG` (read) and `MU_SDMSG` (write) both into `aqueue`
(`gemasync.c:164-166,170-172`), which computes readiness once
(`gemqueue.c:96-99`): `qready = isqwrite ? (qpb_cnt <= QUEUE_SIZE - p_qindex) :
(p_qindex > 0)`.

- **Read blocks iff the queue is empty** (`p_qindex == 0`, `gemqueue.c:98-99`)
  — not "fewer than `length` bytes buffered"; any non-empty queue is ready.
  The 16-byte fast path in `ap_rdwr` (`gemaplib.c:77-82`) only *skips* the
  block machinery for the common 16-in/16-out case; it does not change the
  readiness rule.
- **Write blocks iff the message would not fit** (`qpb_cnt > QUEUE_SIZE -
  p_qindex`, `gemqueue.c:97`). `QUEUE_SIZE = 128` bytes (`aes/struct.h:56`) = 8
  standard 16-byte messages. For the standard message (`qpb_cnt == 16`) the
  write of the *n*-th 16-byte message succeeds for `n <= 8`
  (`p_qindex <= 112` going in) and blocks from the 9th on (`p_qindex == 128`
  going in, i.e. **more than 7** messages already queued) — the arithmetic
  behind the ticket's ">7 messages", verified rather than assumed.

When `qready` is false, `aqueue` inserts the event onto one of two per-process
wait lists (`gemqueue.c:120-124`, `evinsert`) — reads onto `p->p_qdq`, writes
onto `p->p_qnq` (`gemqueue.c:101`: `ppe = (isqwrite ^ qready) ? &p_qnq :
&p_qdq`) — and `mwait` parks the caller in `dsptch()`
(`gemasync.c:117-127`, `WAITIN`): a genuine cooperative block, the caller's
stack frame stays suspended in `ev_block` until some *other* code path
resumes it.

**When `qready` is true, the call returns immediately — the "sofortiger
Rückweg" the source shows.** `aqueue` services the request at once (`doq` +
`azombie(e, 1)`, `gemqueue.c:104-107`); `azombie` sets `e_return` and calls
`signal(e)` (`gemasync.c:64-69,77`), which ORs the event's mask into
`p_evflg` (`gemasync.c:35-40`) *before* `mwait` runs its
`!(mask & p_evflg)` test (`gemasync.c:120`) — so that test is already false
and `mwait` does not call `dsptch()`. (That is `mwait`'s own dispatch only:
every AES call still ends in `super()`'s `dsptch()`, `gemsuper.c:458-462` —
see [`event.md`](./event.md) "Wake conditions and dispatch", point 3. Corrected
A112, 2026-10-17: this paragraph used to say the call never touches `dsptch()`.)
`ev_block` then returns `apret(code)`'s
extracted `e_return` (`gemasync.c:182-214`, extraction at `:208`) — **1**.
So an `appl_read` on a non-empty queue, and an `appl_write` on a queue with
room, both return with `1` without blocking in `mwait` (the trap's closing
`dsptch()` still runs).

**Who wakes a blocked call.** After servicing the *current* request, `aqueue`
also checks the *complementary* wait list and — unconditionally, without
re-testing readiness — services **one** entry from it (`gemqueue.c:108-118`):
a successful read (`qready` true) checks `p_qnq` (pending blocked *writes*)
and, if one is there, does `doq(write)` + `azombie(e, 1)` for it; a
successful write checks `p_qdq` (pending blocked *reads*) the same way.
Three details of that service (A116, 2026-10-22, read from the source):

- **The entry served is the list head, and the head is the most recently
  parked one.** `evinsert` links a new `EVB` in *front* of the current first
  entry (`aes/gemasync.c:81-93`: `e->e_link = p; q->e_link = e` with `q` the
  list root), and the service takes `*ppe`, the first entry
  (`gemqueue.c:108-111`). Several writers parked on one queue are therefore
  served **last-in, first-out**, one per successful read.
- **`ap_rdwr`'s fast path services nobody.** A 16-byte read from a queue that
  holds exactly 16 bytes is completed by `ap_rdwr` itself (`gemaplib.c:77-82`)
  without entering `aqueue`, so `p_qnq` is not looked at. With standard
  16-byte writers this cannot strand anyone (a parked 16-byte writer implies a
  full 128-byte queue); a writer of more than 112 bytes parked behind one
  16-byte message stays parked after that message is read, although the queue
  is then empty — until a later read takes the slow path.
- **The service does not re-test room.** `doq(!isqwrite, …)` copies the head
  writer's `qpb_cnt` bytes whether or not they fit (`gemqueue.c:116`); with
  mixed message sizes a head longer than the message just read would be
  copied past the end of `p_queue[QUEUE_SIZE]` (`aes/struct.h:152`) —
  undefined behaviour, not a contract.

So:

- **A blocked `appl_read` is woken only by a subsequent successful write into
  the *same* queue** — `appl_write`/`ap_sendmsg`, or any of EmuTOS's own
  posters, landing in that process's `p_queue` (`gemqueue.c:104-118`).
- **A blocked `appl_write` is woken only by a subsequent successful read of
  the *same target's own* queue.** Every internal EmuTOS call site that
  performs that read targets `rlr` — the target process reading *its own*
  queue, never a foreign one: `ev_mesag` → `ap_rdwr(MU_MESAG, rlr, 16,
  mebuff)` (`gemevlib.c:116`), `ap_exit`'s drain → `ap_rdwr(MU_MESAG, rlr,
  rlr->p_qindex, …)` (`gemaplib.c:256-257`), and `sh_ldapp`'s "no
  `appl_exit`" cleanup drain, same shape (`gemshlib.c:655`). The raw
  `APPL_READ` trap (`gemsuper.c:101-105`) *can* target any pid via
  `AP_RWID`, but no EmuTOS source file exercises that on a foreign id: every
  poster in `aes/*.c` (`ap_sendmsg`, `gemctrl.c:84,125`; `geminput.c:419`;
  `gemmnlib.c:747`; `gemwmlib.c:904`) only ever **writes** to another
  process, never reads its queue — confirmed by an exhaustive
  `/usr/bin/grep -rn "ap_rdwr\|MU_MESAG\|MU_SDMSG" aes/*.c` over the whole
  AES tree: no read call site names a target other than `rlr`.

**Consequence — a genuine EmuTOS self-deadlock, not a NewTOS artifact.** If a
process writes to its *own* id (`appl_write(id = its own ap_id, …)`, legal —
edge case 7 above) while its own queue already holds 8 messages, the write
blocks (`gemqueue.c:120-124`) and parks the caller in `dsptch()`. `aqueue`
makes no exception for a self-targeted write. Because no *internal* EmuTOS
reader ever reads a queue other than its owner's own, and the owner *is* the
blocked caller, only another **application** reading that queue by id
(`appl_read(the writer's id, …)`, the raw `APPL_READ` arm, `gemsuper.c:
101-105`) can wake it; without one the process hangs forever. This is
reachable only through a self-targeted `appl_write` at depth ≥ 8; it is a
property of EmuTOS's own message pipe, confirmed by reading every internal
reader above, not inferred.

For a write to a **different** process's queue, EmuTOS's block is safe under
its own cooperative multi-process AES: `dsptch()` hands the CPU to other
runnable `AESPD`s (the Screen Manager `ctlmgr`, desk accessories), and the
target process eventually runs its own event loop, calls `evnt_mesag`, and —
as the side effect described above — drains the blocked write. NewTOS
reproduces both halves since A116 (2026-10-22); see the "NewTOS
implementation notes" below for the mapping and what still differs.

**What `appl_write` returns.** There is no EmuTOS code path that returns `0`
for `appl_write`: `ap_rdwr`'s only return statements are the fast path's
`return 1` (`gemaplib.c:81`, read-only) and `ev_block`'s `apret(code)`, which
always extracts the `1` a successful `azombie(e, 1)` stored
(`gemqueue.c:107,117`). So a completed `appl_write` returns exactly `1`; a
queue-full write either completes later with `1` (once drained) or never
returns (self-deadlock above). The one way `ap_rdwr` can misbehave without a
defined error code is an unresolved `AP_RWID`: `fpdnm(NULL, AP_RWID)` returns
`NULL` for a pid no descriptor holds (`gempd.c:38-57`), and `aqueue` then
dereferences `m->qpb_ppd->p_qindex` through that `NULL`
(`gemqueue.c:94,97-99`) — undefined behaviour (a supervisor-mode crash on
real hardware), not a `0` return. Out of this ticket's scope; already
flagged as N1 in
`docs/analysis/a30-appl-init-recall-measurement-design-2026-09-19.md` §7.

**What `appl_read` returns.** Symmetrically: `1` with the message copied to
`addr_in[0]` on completion (fast path `gemaplib.c:81`, or after the block
resolves), never `0` for "queue was empty" — an empty queue blocks, it does
not fail.

**Caller survey (A75).** None of the surveyed applications call `appl_read`
directly:
`/usr/bin/grep -rn "appl_read\|appl_write" sdk/apps/teradesk/*.c
sdk/apps/qed/src/*.c sdk/cflib/cflib-21/*.c` finds **zero** `appl_read(` call
sites. Every message these programs *receive* comes back through
`evnt_multi`'s `MU_MESAG` bit or `evnt_mesag`, matching gemlib's own binding
comment: "Normally this call is not used. `mt_evnt_multi()` or
`mt_evnt_mesag()` is used instead for standard message reception. …required
for reading messages that are long and/or of variable length"
(`sdk/gemlib/gemlib-0.44.0/a_appl_read.c:27-30`). Every call found is
`appl_write` (send), on a queue the caller does not control the depth of:

| Call site | Target | Purpose |
|---|---|---|
| `sdk/apps/teradesk/dragdrop.c:119` | `dpid` (drop target's ap_id) | drag-and-drop notify; checks `== 0` for "appl_write error" |
| `sdk/apps/teradesk/font.c:631`, `:692` | `fnt_dial->ap_id` / `cl_ap_id` | font-selector reply to caller |
| `sdk/apps/teradesk/va.c:474,498,533,700,1413` | `dest_ap_id` / iterated `avclients` list / `av_current` | AV protocol (inter-app data exchange); `va_send_all` (`:498`) broadcasts to every registered AV client in a loop |
| `sdk/apps/qed/src/comm.c:38` | `id` (clipboard peer) | `(ret > 0)` gates success |
| `sdk/apps/qed/src/event.c:619` | `gl_apid` — **QED's own id** | self-send `AP_TERM` on `abort_prog`, so QED's own next `evnt_multi` pass sees the quit request |
| `sdk/cflib/cflib-21/ddcreate.c:78` | `apid` | drag-and-drop, same protocol as TeraDesk's |
| `sdk/cflib/cflib-21/fontsel.c:240` | `ap_id` | font-selector reply |
| `sdk/cflib/cflib-21/sendspec.c:47` | `appl_find("SCRENMGR")`'s result | MagiC extension message to the Screen Manager |
| `sdk/cflib/cflib-21/sendstrt.c:46` | `id` | generic message send |

Two things worth naming: `event.c:619` is the one surveyed **self-write** —
QED sends `AP_TERM` to its own `gl_apid` while shutting down; its queue at
that point is not plausibly at depth 8, so this does not exercise the
self-deadlock case above, but it does confirm self-targeted `appl_write` is a
real, if rare, usage pattern, not a theoretical one. `sendspec.c:47` targets
the Screen Manager directly by name — the one write target that is not an
application (see "AES process identity and pid" above, `SCRENMGR_PID`).
None of the surveyed call sites
checks for a *blocked-forever* outcome — consistent with the finding above
that EmuTOS's `appl_write` has no error return to check for in the common
(non-self, non-full) case.

> **NewTOS implementation notes (A75, 2026-09-20).** The bring-up code in
> `tos-api/src/aes/appl.rs` (`appl_read` `:434-469`, `appl_write` `:381-424`)
> and `pd.rs` (`MsgQueue::enqueue` `:168-201`, the full-queue `false` return
> documented in place at `:160-167`) predates the cooperative sleep/wake
> substrate that now exists and is already wired for the sibling functions on
> the *same* message pipe: `evnt_mesag` (`event.rs:729-769`) and
> `evnt_multi`'s `MU_MESAG` arm (`event.rs:638-658`, inside the wait loop
> `event.rs:500-681`) both pop-if-present, else
> `sched::sleep(sched::wait_mesg(idx))` (`event.rs:765`, wake key
> `sched.rs:61-69`), and propagate the resulting `Option<sched::Switch>` so
> the trap dispatcher can switch to a peer; the poster side already wakes
> that key (`pd::post_msg`/`post_words` → `wake_mesg_waiter`,
> `pd.rs:383-397,399-422,424-434`).
>
> **`appl_read` (11) is now a real, fixable gap, not a structural one.**
> Nothing prevents it from looping the way `evnt_mesag` does: pop the front
> message if present, else `sched::sleep(wait_mesg(idx))` and retry on
> resume, returning `Some(Switch)` up through the dispatcher on a peer yield.
> `appl_read` currently does not (`appl.rs:434-469`: a single `pop_into`, `0`
> on empty, no wait) — the asymmetry both the A30 design note (§7, "N4") and
> this ticket name; it is now a divergence from the function's own sibling,
> not only from EmuTOS. Fixing it is a `tos-api` code change — Developer
> work, not this analysis.
>
> **`appl_write` (12) blocks on a full queue (A116, 2026-10-22).** User
> decision 2026-10-22 (`TASK/A116.md`): as EmuTOS. The A75 return-`0` and its
> rationale ("at most one genuinely-scheduled AES client"; revised once on
> 2026-10-17 when desk accessories ran as scheduled AES clients) are
> withdrawn. The mapping (`tos-api/src/aes/appl.rs` `appl_write`,
> `tos-api/src/aes/pd.rs` "parked writers"):
>
> - **Readiness** is `aqueue`'s byte rule `qpb_cnt <= QUEUE_SIZE - p_qindex`,
>   tested before any coalescing (`MsgQueue::fits`). NewTOS's queue is a slot
>   FIFO of at most 8 messages; for messages shorter than 16 bytes that slot
>   bound is reached before the byte bound (the slot model's own limit, not
>   part of A116).
> - **Park.** A write that is not ready is recorded for the writer's identity
>   with its target, its message and a park sequence (`pd::park_write`, the
>   `evinsert(e, &p_qnq)` of `gemqueue.c:120-124`); the writer sleeps on
>   `sched::wait_qnq(writer)` (`mwait`). A yield aborts the trap (S2); the
>   re-executed `appl_write` finds the write still in flight and sleeps again.
>   The message is copied at park time rather than at service time —
>   NewTOS's writer and reader live in different address spaces, and the
>   writer is blocked in between.
> - **Service.** Every completed read of a queue — `appl_read`, `evnt_mesag`,
>   `evnt_multi`'s `MU_MESAG`, the `appl_exit` and `sh_ldapp` drains
>   (`pd::close`/`close_if_open`) — calls `pd::after_read`: unless the read was
>   `ap_rdwr`'s 16-from-16 fast path, the most recently parked writer on that
>   queue has its message enqueued (with `doq`'s coalescing) and is woken; its
>   call returns 1. A head that still does not fit (EmuTOS's overrun case) is
>   left parked.
> - **Self-writes are not special**, as in EmuTOS: a write to the caller's own
>   full queue parks until another process reads that queue.
> - **pid 1 (`SCRENMGR`) never blocks**: NewTOS has no Screen Manager queue;
>   the write returns 1 at once and the message goes nowhere (as before).
>   EmuTOS's `ctlmgr` reads its queue on every pass (`ev_multi(… | MU_MESAG)`
>   under `CONF_WITH_3D_OBJECTS`, `gemctrl.c:445-448`, `hctl_msg` `:480-483`),
>   so a writer there waits at most for its next turn and also gets 1.
>
> What still differs is named in [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md)
> "Kernel-side AES message posts into a full queue are dropped" — the
> kernel-side posters, which in EmuTOS block through `ap_sendmsg` too, and
> the NewTOS-only release of a writer whose target identity ends.

## `appl_find` (13)

**Source:** `aes/gemsuper.c:106-108` → `aes/gemaplib.c:95-108` (`ap_find`);
lookup `aes/gempd.c:38-57` (`fpdnm`).

| Slot | Name | Meaning |
|------|------|---------|
| `addr_in[0]` | `AP_PNAME` | 8-char application name (`char *`) |
| `int_out[0]` | — | return: the app's pid, or `-1` if not found / `pname == NULL` |

**Behaviour.** `ap_find` first rejects a `NULL` name pointer with `-1`
(`gemaplib.c:103-104`) — an explicit guard, because `fpdnm(NULL, …)` means
"search by pid" instead of by name. Otherwise it calls `fpdnm(pname, 0)`, which
walks all `totpds` process descriptors (`gempd.c:43`) and compares the name
with `strncmp(pname, p->p_name, AP_NAMELEN)` over **exactly 8 characters**
(`gempd.c:48`, `AP_NAMELEN = 8`, `aes/struct.h:111`). It returns the matched
`p->p_pid`, or `-1` if no descriptor matches.

**Name format (contract).** Names in the descriptor are set by `p_nameit`
(`gempd.c:77-86`): copy up to 8 characters of the filename, **stopping at the
first `.`** (extension stripped), then **space-pad to 8** with `' '` (0x20).
So `appl_find` must be passed the 8-char, space-padded, extension-less name
(e.g. `"CONTROL "` for `CONTROL.ACC`); a shorter string only matches if the
caller space-pads it, because `strncmp` compares the full 8 bytes.

## `appl_yield` (17)

**Source:** `aes/gemsuper.c:115-119` → `dsptch()` (the AES scheduler). A
**PC-GEM extension**, compiled only `#if CONF_WITH_PCGEM`
(`gemsuper.c:115`). `CONF_WITH_PCGEM` defaults to **1**
(`include/config.h:1319-1321`), so `appl_yield` is present in the default
build.

| Slot | Meaning |
|------|---------|
| `int_in` / `addr_in` | (none) |
| `int_out[0]` | return: `TRUE` (1) — `ret` is left at its initial `TRUE` (`gemsuper.c:85`) |

Cooperatively yields the CPU to any other ready AES process by calling
`dsptch()` (the run-list dispatcher). This is not special to `appl_yield`: every AES call
ends in `dsptch()` (`gemsuper.c:458-462`), so `appl_yield` dispatches twice per
call (the arm at `:116-118`, then the fall-through); round-robin order in
[`event.md`](./event.md) "Wake conditions and dispatch". If `CONF_WITH_PCGEM` is 0, opcode 17
falls to the `default` arm and returns `-1` (see [`_dispatcher.md`](./_dispatcher.md)).

## `appl_exit` (19)

**Source:** `aes/gemsuper.c:120-123` → `aes/gemaplib.c:251-261` (`ap_exit`).

| Slot | Meaning |
|------|---------|
| `int_in` / `addr_in` | (none) |
| `int_out[0]` | return: `TRUE` (1) — `ap_exit` is `void`; `ret` stays `TRUE` |

**Behaviour.** `ap_exit` tears the application down (`gemaplib.c:252-260`):

1. `wm_update(BEG_UPDATE)` — lock the screen against redraws.
2. `mn_cleanup()` (`gemmnlib.c:740-758`) — **does not touch the menu bar**
   (corrected 2026-10-17, A113; the old wording "drop the app's menu bar if it
   owns one" was wrong: `gl_mntree`/`gl_mnppd` are written only by `mn_bar` and
   `fm_own`, see [`menu.md`](./menu.md) "A dead client's menu bar"). It does
   two things: (a) for each `i < NUM_ACCS` with `desk_ppd[i] != NULL` — every
   accessory slot filled by `menu_register` — it posts `AC_CLOSE` to that
   accessory's `AESPD` via `ap_sendmsg(appl_msg, AC_CLOSE, desk_ppd[i], i, 0,
   0, 0, 0)` (`:745-748`), i.e. `msg[1]` = the pid of the program running
   `appl_exit`, `msg[2]` = 0, `msg[3]` = the accessory's **menu id** `i` (the
   `menu_register` return value, not its pid); an accessory that never called
   `menu_register` gets no `AC_CLOSE` here; (b) with `CONF_WITH_MENU_EXTENSION`,
   frees `gl_submenu` (`:750-756`). `ap_sendmsg` → `ap_rdwr(MU_SDMSG, …)` can
   itself block on a full accessory queue (see "Blocking behaviour").
3. `wait_for_accs(AP_ACCLOSE)` — block until every desk accessory has seen the
   `AC_CLOSE` message (`geminit.c:599-628`; polls the DA descriptors' flags
   with a `WAIT_TIMEOUT = 500` loop cap). **"Block" is a round-robin yield, not
   a wait on an event**: each pass is `all_run()` (`geminit.c:554-566`), i.e.
   `dsptch()` once per accessory, and `dsptch` from a process that has not set
   `WAITIN` puts it straight back on the ready list (`gemdisp.c:74-79,230-231`).
   The barrier therefore makes progress with no input of any kind, and it is
   bounded. Extracted in full in
   [`accessories.md`](./accessories.md) §3a — the distinction is load-bearing
   (ACC10).
4. If the app still has a queued message (`rlr->p_qindex != 0`), drain it with
   `ap_rdwr(MU_MESAG, rlr, rlr->p_qindex, D.g_valstr)` (`gemaplib.c:256-257`;
   `D.g_valstr` is a scratch buffer in `THEGLO`, `aes/gemlib.h:133`).
5. `wm_update(END_UPDATE)` — unlock the screen.
6. `all_run()` — re-schedule the other processes (`geminit.c:554`).
7. `rlr->p_flags &= ~AP_OPEN` — mark `appl_exit` done.

> **NewTOS (A118, 2026-10-25).** Steps 1 and 5 are reproduced: the lock is
> taken before `mn_cleanup` — the call waits there while another process
> holds it — and held across the `wait_for_accs` barrier, so an accessory
> that answers `AC_CLOSE` with a `BEG_UPDATE` (or a window call, a dialog)
> runs on only after the barrier, when step 5 hands the lock to it. The
> call is re-executed while it yields to the accessories (the S2 restartable
> block); the lock is taken once per `AC_CLOSE` round. Step 6 (`all_run()`
> after the unlock) is **not** reproduced — open, outside A118. Receipts:
> `tos-api/tests/aes_a118_lock_takers.rs` (`appl_exit_*`).

> `appl_exit` always returns `1`. The DA-close handshake (steps 2-3) presumes
> concurrent AES processes; with no `.NAC` loaded `wait_for_accs` returns
> immediately, which is what NewTOS's single-app bring-up saw until ACC02-ACC05
> gave it real accessory processes (2026-09-25) — from then on the handshake is
> live and step 3's *yield* semantics matter (ACC10, 2026-09-26).

**The descriptor is not freed.** The list above is exhaustive: `ap_exit` drains
the queue and clears one flag bit. `p_pid`, `p_name` and the remaining flags
survive, and the `AESPD` itself cannot be released — EmuTOS's descriptors are
fixed entries of `D.g_int[]`/`D.g_acc[]` addressed by index (`pd_index`,
`aes/gempd.c:32-35`), and `fpdnm` walks that array with no liveness test
(`:38-57`). So after `appl_exit`, `appl_find(name)` still resolves the name and
`appl_write(id, …)` still queues into the descriptor. The same two steps — and
only those two — are what `sh_ldapp` performs on the program's behalf when it
skipped `appl_exit` (`gemshlib.c:650-657`, see
[`shell-loop.md`](./shell-loop.md) §8).

> **NewTOS (A29, 2026-09-14).** Both end-of-program paths used to destroy the
> descriptor instead. That was observable, not academic: NewTOS's desktop runs
> in the kernel boot context and has pid 1, the first program it launches gets
> GEMDOS slot 0 and therefore pid 1 too, so the two share one descriptor exactly
> as EmuTOS's `sh_ldapp` shares one renamed `rlr`. Destroying it at program end
> left `pd::by_pid(1)` empty for the rest of the session, and every message the
> AES posted to the desktop afterwards — `WM_REDRAW`, `WM_CLOSED`, `WM_MOVED`,
> `WM_SIZED`, `WM_ARROWED`, `MN_SELECTED` — was dropped without a trace.
> Regression test: `tos-api/tests/aes_app_return_descriptor.rs`.

## `appl_tplay` (14) / `appl_trecord` (15)

**Source:** `aes/gemsuper.c:109-114` → `aes/gemaplib.c:119-184` (`ap_tplay`) /
`:189-246` (`ap_trecd`). Event-"tape" play/record: replay or capture a stream
of `EVNTREC` records (timer/button/mouse/keyboard events) for macros/testing.

| Slot | Name | Meaning |
|------|------|---------|
| `addr_in[0]` | `AP_TBUFFER` | `EVNTREC *` — the event tape |
| `int_in[0]` | `AP_TLENGTH` | record count |
| `int_in[1]` | `AP_TSCALE` | playback time-scale (`ap_tplay` only) |
| `int_out[0]` | — | `appl_tplay`: `TRUE` (1), it is `void`; `appl_trecord`: actual #records captured |

**`EVNTREC`** (`aes/gemaplib.h:17-28`) is two LONGs: `ap_event` (the event
kind — `TCHNG`=0 timer, `BCHNG`=1 button, `MCHNG`=2 mouse, `KCHNG`=3 keyboard)
and `ap_value` (packed event data; encoding per kind documented at
`gemaplib.h:24-28`).

**`ap_tplay`** iterates the buffer, converting each record into a fork-queued
call (`bchange`/`mchange`/`kchange`) or an `ev_timer` delay scaled by
`AP_TSCALE` (`gemaplib.c:127-168`), and `dsptch()`es between records. Mouse
playback temporarily disconnects the VDI cursor and redraws it itself
(`gemaplib.c:140-153,173-181`).

**`ap_trecd`** sets `gl_recd`/`gl_rbuf`/`gl_rlen` and lets the fork dispatcher
(`gemdisp.c`) record raw `FPD`s into the buffer, polling every 0.1 s until the
buffer fills, then converts the captured `FPD`s back to `EVNTREC` format and
returns the actual count (`gemaplib.c:209-245`). Relies on
`_Static_assert(sizeof(EVNTREC) == sizeof(FPD))` (`gemaplib.c:206`).

> **NewTOS note (honest-minimal).** These two need the fork-queue + input-event
> recorder infrastructure (`gemdisp.c`, `forkq`, `gl_recd`). NewTOS has no such
> recorder at bring-up. Per CLAUDE.md Rule 2 (no stubs), `appl_tplay`/
> `appl_trecord` are **honest-unsupported** until the input subsystem exists:
> the dispatch arms may return `-1` (or the documented `TRUE`/count once the
> recorder is built), but must not silently pretend to replay/capture. The slot
> ABI above is the contract for when they are implemented.

---

## Trap interface (parameter-block form)

All `appl_*` calls enter via the GEM trap (`svc #2`) with **selector 200 in
`x0`** and an **AESPB pointer in `x1`** — see [`_dispatcher.md`](./_dispatcher.md).
Per call, the dispatcher reads `control[0]` = opcode, `control[1]` = #int_in,
`control[2]` = #int_out, `control[3]` = #addr_in, and returns via
`int_out[0]` = `RET_CODE`.

| Opcode | Function | `int_in` | `addr_in` | `int_out[0]` return |
|-------:|----------|----------|-----------|---------------------|
| 10 | `appl_init`   | — | — | `ap_id` (pid) / -1; fills `global[]` |
| 11 | `appl_read`   | `[0]`=id, `[1]`=len | `[0]`=buf | 1 on success |
| 12 | `appl_write`  | `[0]`=id, `[1]`=len | `[0]`=buf | 1 on success |
| 13 | `appl_find`   | — | `[0]`=name | pid / -1 |
| 14 | `appl_tplay`  | `[0]`=len, `[1]`=scale | `[0]`=`EVNTREC*` | 1 (void) |
| 15 | `appl_trecord`| `[0]`=len | `[0]`=`EVNTREC*` | #records captured |
| 17 | `appl_yield`  | — | — | 1 *(CONF_WITH_PCGEM; else -1)* |
| 19 | `appl_exit`   | — | — | 1 |

> **Not present in this build:** `appl_getinfo` (130), `appl_search` (18),
> `appl_control`. No opcode macro and no dispatch arm exists for any of them in
> EmuTOS `1a7484ab` (`include/aesdefs.h:16-24`, `aes/gemsuper.c:88-123`). Calls
> to those opcodes hit the dispatcher `default` arm → `-1` (see
> [`_dispatcher.md`](./_dispatcher.md)). If NewTOS later adds `appl_getinfo`
> for MagiC/newer-GEM compatibility that is a documented extension, not a
> reference-parity item.

---

## Inputs / Outputs summary

All inputs arrive in the AESPB arrays (no register/stack params). Outputs are
`int_out[0]` (the return code) plus, for `appl_init`, the `global[]` fields
listed above. `appl_read`/`appl_write` also transfer the 16-byte message
through `addr_in[0]`.

---

## Error conditions

The application manager has almost no error surface — it is "trusted" AES
code. The only defined negative returns:

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| `appl_find` with `pname == NULL` | `gemaplib.c:103-104` | `-1` |
| `appl_find` name not in any descriptor | `gemaplib.c:106-107` | `-1` |
| `appl_init` cannot allocate an app slot | (contract; never happens here) | `-1` |
| `appl_yield` when `CONF_WITH_PCGEM == 0` | `gemsuper.c:406-417` | `-1` (unsupported opcode) |

There are **no** TOS negative error codes (`E*`) in the application manager;
returns are pid / 0 / 1 / -1 only.

---

## Verification criteria

- **`appl_init` id + version:** `appl_init` returns a **non-negative** value
  (the pid), sets `global[0]` = `AES_VERSION` (0x0340 in the default build),
  `global[1]` = 1, and `global[2]` = the same pid it returned
  (`gemsuper.c:92-99`).
- **`appl_init` private:** after `appl_init`, `global[3..4]` (`ap_private`,
  read via `ULONG_AT(&global[3])`) equals the DESKTOP tree root's `ob_spec`
  (`gemsuper.c:95-96`). **Satisfied on NewTOS** since stage S1b: `0x1143`
  with no `#Q` line in the `.INF` file, `0x1100 | patcol` with one — the high
  24 bits are the resource constant and never change (D-099, closed).
- **`appl_find` unknown:** `appl_find` of a name that no application registered
  returns `-1` (`gemaplib.c:106-107`).
- **`appl_find` NULL:** `appl_find` with `addr_in[0] == NULL` returns `-1`
  without searching (`gemaplib.c:103-104`).
- **`appl_find` self:** `appl_find` of the caller's own 8-char space-padded
  name returns the caller's pid (`gempd.c:48`).
- **16-byte read fast path:** with the pipe holding exactly a 16-byte message,
  `appl_read(id=self, len=16, buf)` returns `1` and copies the 16 bytes
  (`gemaplib.c:77-82`).
- **`appl_exit` return:** `appl_exit` returns `1` and clears `AP_OPEN`
  (`gemaplib.c:260`).
- **`appl_yield` present:** in the default build (`CONF_WITH_PCGEM=1`) opcode
  17 returns `1`; if that flag is 0, opcode 17 returns `-1`.

---

## Edge cases discovered in source

1. **`ap_id` is the AES pid, not a separate handle.** `appl_init` returns
   `rlr->p_pid` and stores the same value in `global[2]`
   (`gemsuper.c:94,99`). **Correction (2026-09-20, A62/A56):** the previous
   wording here — "the AES pid is small (0/1 = system processes, 2+ =
   accessories/applications)" — was wrong. Per `geminit.c:850-888` and
   `gempd.c:60-70`: pid **0** is the *entire* application chain (desktop,
   every launched program, every `Pexec` child of it — not a "system
   process" distinct from the application); pid **1** is the Screen Manager
   (`SCRENMGR`/`ctlmgr`), a genuine second AES process, not a flag; pid
   **2+** is desk accessories only. See "AES process identity and pid" above.
2. **`ap_private` — not `ap_ptree` — carries the DESKTOP `ob_spec`.**
   `appl_init` writes `global[3]` (`AP_PRIVATE`), *not* `global[5]`
   (`AP_PTREE`). `ap_ptree` stays unset until `rsrc_load` (`gemsuper.c:95-96`).
3. **`appl_init` also sets `ap_planes` and `ap_3resv`** (`global[10]` and
   `global[11]`) even though those are "not advertised" fields — a program that
   reads them sees the screen plane count and the `THEGLO` pointer
   (`gemsuper.c:97-98`).
4. **`appl_find` compares exactly 8 bytes.** A caller passing a name shorter
   than 8 chars must space-pad it, or `strncmp` over 8 bytes will mismatch
   against the space-padded descriptor name (`gempd.c:48,77-86`).
5. **`appl_find(NULL)` is a defined `-1`, not a crash**, because a NULL name has
   the special "search by pid" meaning inside `fpdnm` and is explicitly guarded
   (`gemaplib.c:99-104`).
6. **`appl_read`/`appl_write` block the AES process** unless the 16-byte fast
   path applies; blocking is via the cooperative AES scheduler (`ev_block` →
   `mwait`/`dsptch`), not a preemptive wait (`gemaplib.c:84-88`,
   `gemevlib.c:64-70`). Precise readiness conditions, who wakes whom, and the
   self-targeted-write deadlock this implies: "Blocking behaviour (A75)"
   above.
7. **`appl_write` to self is legal** — send id == own id delivers into the
   caller's own pipe (the id is resolved with `fpdnm(NULL, id)`, no self-check).
8. **The dispatcher's `default` arm returns `-1` and shows an alert.** Opcodes
   16, 18, and (without PC-GEM) 17 within the 10–19 range are not handled and
   fall through to `fm_show(ALNOFUNC, …)` + `ret = -1`, except opcode 0 which is
   silently ignored (`gemsuper.c:406-417`). See [`_dispatcher.md`](./_dispatcher.md).
9. **`appl_exit` may drain a leftover message** into `D.g_valstr` before
   returning, so a queued-but-unread message at exit is consumed, not leaked
   (`gemaplib.c:256-257`).

---

## Dependencies

### Requires (must be initialized / functional)

- The AES process model: `rlr`, the `AESPD` descriptors, `totpds`/`curpid`
  (`geminit.c` start-up; [`_state.md`](./_state.md)). `appl_init` needs a
  descriptor already allocated.
- `rs_trees[DESKTOP]` (the built-in DESKTOP resource) for `appl_init`'s
  `ap_private` value (`gemsuper.c:95`).
- The AES scheduler (`ev_block`/`mwait`/`dsptch`) and the message pipe
  (`gemqueue.c`) for `appl_read`/`appl_write`.
- `wm_update`, `mn_cleanup`, `wait_for_accs`, `all_run` for `appl_exit`.
- The fork/input-recorder subsystem (`gemdisp.c`) for `appl_tplay`/
  `appl_trecord` — **absent at NewTOS bring-up**.

### Used by

- **Every GEM application** brackets its life with `appl_init` … `appl_exit`.
- `evnt_mesag` / `evnt_multi` (event manager) sit on the same 16-byte message
  pipe as `appl_read`/`appl_write` (`gemevlib.c:116`) — see
  [`event.md`](./event.md).
- `mn_register` and the desk-accessory machinery use `appl_find`/the pid model.

---

## Related data structures

- **`AESGLOBAL`** — the `global[]` array, layout table above
  (`gemrslib.h:15-27`).
- **16-byte standard message** — `WORD msg[8]`; `msg[0]` = message id
  (`MN_SELECTED`/`WM_*`/`AC_*`, `include/aesdefs.h:227-243`), `msg[1]` = sender
  ap_id (**0** = application chain, **1** = Screen-Manager sender stamp only —
  see "AES process identity and pid" above), `msg[2]` = extra-byte count,
  `msg[3..7]` = message-specific. Shared with the event manager
  ([`event.md`](./event.md)).
- **`EVNTREC`** (tape record) — `gemaplib.h:17-28`.
- **`AESPD` / `QPB`** — process descriptor + queue parameter block
  (`aes/struct.h:119-163`); reference-level (NewTOS reimplements). See
  [`_state.md`](./_state.md).
- **`appl_find` name format** — 8 chars, extension stripped at first `.`,
  space-padded (`gempd.c:77-86`).

---

## Cross-references

### Atari Compendium / Profibuch

- **Not present in `references/`** (no PDFs). Cross-check of `appl_init`'s
  reported `ap_version`/`ap_count`, the `appl_find` 8-char/space-pad format and
  the `-1` conventions, the `appl_read`/`appl_write` 16-byte pipe, and the
  absence of `appl_getinfo`/`appl_search` in this build is **deferred** until
  the documentation PDFs are available.

### AES dispatcher

- [`_dispatcher.md`](./_dispatcher.md) — the selector-200 call model, `control[]`
  counts, `int_out[0]` return convention, opcode→manager map, and the
  `default` → `-1` unsupported-opcode path.

---

## Configuration / version notes

- **`AES_VERSION`** reported by `appl_init` is build-dependent (table above);
  **0x0340** in the default reference build. NewTOS should pin the reported
  value by ADR; matching the default (0x0340) keeps parity with EmuTOS's
  standard build.
- **`CONF_WITH_PCGEM`** (default **1**) gates `appl_yield` (17). With it off,
  opcode 17 is an unsupported opcode (`-1`).
- **`ap_count = 1`** is hard-coded (`gemsuper.c:93`) — EmuTOS reports a single
  concurrent application even though it runs desk accessories as additional AES
  processes; NewTOS reports the same constant 1 (accessories are *not*
  reflected in `ap_count`, `gemsuper.c:93`).
- The tape functions (`appl_tplay`/`appl_trecord`) are configuration-
  independent in EmuTOS but depend on the input/fork subsystem NewTOS has not
  built.

---

## NewTOS implementation notes

(Empty during Phase 00.)

- **`ap_private` — the AES's own built-in `DESKTOP` tree (D-099, closed
  2026-09-01, stage S1b).** `tos-api::aes::init` carries `rs_trees[DESKTOP]`
  as a static three-object tree transcribed from `aes/gem_rsc.c:550-570` (ROOT
  `ob_spec = 4419`, the background box, and the `DTNAME` `G_TEXT` with its
  `rs_tedinfo[12]`). AES boot (`aes::init::init()`, called from the kernel boot
  path before anything can `appl_init`) does the three `geminit.c` steps in
  EmuTOS's order: `process_inf1`'s `#Q` scan of the first `INF_SIZE` = 300 bytes
  of `C:\NEWDESK.INF` (`geminit.c:343-392`, `:99`), the `gem_rsc_fixit`
  char-grid→pixel conversion plus `tree[ROOT].ob_height = screen height`
  (`gem_rsc.c:636-641`, `geminit.c:715-717`), and `set_aes_background`
  (`geminit.c:635-641`) when a `#Q` line was found. `appl_init` then reads
  `rs_trees[DESKTOP][ROOT].ob_spec` verbatim (`gemsuper.c:95-96`).
  The earlier "no DESKTOP tree exists at the AES layer, so report 0" position
  is withdrawn: the tree is the AES's own resource, not the desktop's, so
  nothing about it needed the desktop client.
- **Single-app vs. multi-app (the contract split).** EmuTOS runs the desktop
  plus up to `NUM_ACCS` desk accessories as concurrent AES processes, but
  reports `ap_count = 1`. The **reported value 1** and the **pid-as-ap_id**
  model are *contract*; the concurrent-DA scheduling is *reference*.
  **Corrected 2026-10-17 (A113):** the earlier wording — "NewTOS's cooperative
  single-AES-app bring-up satisfies the contract with one process; DA
  concurrency can be added later *without changing what `appl_init` reports*" —
  is false. Accessories now run (SYSACC), and what `appl_init` reports **is**
  the visible half of that concurrency: an accessory must report its own pid
  (2, 3, … in successful-load order) and be addressable by `appl_find`/
  `appl_write`/`WF_OWNER`/`msg[1]` (`geminit.c:236-257`, `gempd.c:60-70`; see
  "Pid assignment, accessories, and a failed load"). The scheduling is
  reference; the pid space is contract.
- **`ap_id` ≠ the Phase-06 process id. Correction (2026-09-20, A62/A56):**
  this line previously equated the *reported* `ap_id` with the GEMDOS
  process id (i.e. "Slot + 1"), which is not what EmuTOS reports — EmuTOS's
  `ap_id` is **0** for the whole application chain (Fall 644,
  `measurements/measurement-2026-09-12-aes-r4/aes-r4-subfunctions.tsv`, CRC `227422B2`).
  What *is* true, and what this line was reaching for, is only that the AES
  application's *life* brackets a GEMDOS process's life
  (`appl_init`/`appl_exit` inside `Pexec`/`Pterm`) — not that its reported
  numeric id equals the GEMDOS one. See "AES process identity and pid" above
  for the identity-vs-value split this requires.
- **The message pipe** maps onto NewTOS's process/IPC model; the 16-byte
  standard message is what `evnt_mesag` delivers — the two are designed
  together (see [`event.md`](./event.md)).
- **`appl_read`/`appl_write` blocking (A75, 2026-09-20; A116, 2026-10-22).**
  Full contract (readiness conditions, who wakes whom, LIFO service, the fast
  path, the self-targeted-write deadlock EmuTOS itself has) and the NewTOS
  mapping are under "Blocking behaviour (A75)" above, not repeated here.
  Short version: `appl_read` blocks on an empty queue on the sleep/wake
  substrate `evnt_mesag` uses; `appl_write` blocks on a full queue until a
  read of that queue services it (A116). The remaining difference — kernel-side
  posters drop instead of blocking — is named in
  [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md).
- **`global[]` is an application-supplied array** (in the app's memory) — a
  **user pointer**; apply the same boundary handling as other AES `addr*`
  pointers (see [`_dispatcher.md`](./_dispatcher.md) §NewTOS notes). Its
  pointer fields do **not** follow the caller's ABI: `AESGLOBAL` keeps the
  m68k widths on every ABI (the §`AESGLOBAL` decision of 2026-09-13 above),
  the **named exception** to
  [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md) "Native vs m68k struct
  variants". What does follow the caller is the *array* `ap_ptree` points at:
  its slots are native `OBJECT *` (8 bytes LP64, 4 ILP32), because that is how
  `_AESrscfile[i]` is dereferenced.
- **`appl_tplay`/`appl_trecord`** are honest-unsupported until the input
  recorder exists (Rule 2), not stubbed.
