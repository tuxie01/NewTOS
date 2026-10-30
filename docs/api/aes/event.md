# AES Event Manager (`evnt_*`) — opcodes 20–26

> **PHASE-00b D8, Batch 2 (full-contract refinement).**
> Source: EmuTOS @ `1a7484ab`.
> Primary: `aes/gemevlib.c` (`ev_*`); dispatch arm `aes/gemsuper.c:125-165`
> (`crysbind`); event-flag + slot macros `include/aesdefs.h`; input helpers
> `aes/geminput.c` (`in_mrect`/`downorup`/`b_click`/`bchange`) and the key/fork
> pre-check `aes/gemdisp.c` (`chkkbd`/`forker`); the message pipe is shared with
> the application manager (`aes/gemaplib.c:ap_rdwr`, `aes/gemqueue.c`).
>
> Call convention + AESPB layout: [`_dispatcher.md`](./_dispatcher.md).
> Shared AES globals (message pipe, `rlr`, mouse/button/keyboard state):
> [`_state.md`](./_state.md).
>
> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md).
> **Contract:** opcode numbers (20–26); the `MU_*` event-flag bit values; the
> `int_in`/`int_out`/`addr_in` slot layout of every call (especially the
> `evnt_multi` slot map and the `int_out[1..6]` return block shared by all
> waits); the `MOBLK`/`GRECT` 5-WORD rectangle layout; the enter/leave (`m_out`)
> rectangle semantics; the button clicks/mask/state packing; the 16-byte standard
> message; the double-click-rate speed range (0..4) and its table; the
> `EV_KRET` scancode|ascii key encoding; the multi-event "which fired" bitmask
> return. **Also contract (A112, "Wake conditions and dispatch"):** which event
> wakes a blocked call regardless of which process runs, that every AES call
> yields, and the `evnt_timer(0)` = one tick rule.
> **Reference (NewTOS reimplements freely):** `ev_block`/`iasync`/`mwait`/
> `apret`/`acancel`, the `EVB`/`QPB` event blocks, the fork queue (`forker`/
> `forkq`), the `gl_mowner`/`p_cda` cooperative-scheduler internals, and the
> `xrat`/`yrat`/`button`/`mtrans`/`pr_*` global mirrors — these are the EmuTOS
> cooperative-multitasking mechanics NewTOS drives from its own USB-HID/IKBD
> input path + per-process message pipe.

---

## Event-type flags (`MU_*`, `include/aesdefs.h:256-263`)

The bit mask that selects which events a wait blocks on. Passed in
`int_in[0]` (`MU_FLAGS`) of `evnt_multi`; each single-event call is a fixed
one-flag wait.

| Flag | Value | Event |
|------|------:|-------|
| `MU_KEYBD`    | 0x0001 | a key was pressed |
| `MU_BUTTON`   | 0x0002 | the mouse-button state matched the requested clicks/mask/state |
| `MU_M1`       | 0x0004 | mouse entered/left rectangle 1 |
| `MU_M2`       | 0x0008 | mouse entered/left rectangle 2 |
| `MU_MESAG`    | 0x0010 | a message arrived in the caller's pipe |
| `MU_TIMER`    | 0x0020 | the timer expired |
| `MU_TOSVALID` | 0x003F | mask of all six valid TOS bits (`KEYBD..TIMER`) |
| `MU_M3`       | 0x0100 | **internal only** — rectangle 3, `CONF_WITH_MENU_EXTENSION` (see below) |

> **Correction vs. previous spec:** the old table gave `MU_TOSVALID = 0x3F`
> as "all of the above" and omitted `MU_M3 = 0x0100`. `MU_TOSVALID` is exactly
> **0x003F** (the six standard bits) and is used to **mask off** any extra bits
> a caller sets before `evnt_multi` reaches the library — so a user program
> **cannot** request `MU_M3` (`gemsuper.c:153`, `ev_multi(MU_FLAGS & MU_TOSVALID, …)`).
> `MU_M3` (0x0100) is reserved for the menu-extension code's internal third
> rectangle. Two further internal-use bits exist but are never valid event
> selectors: `MU_SDMSG = 0x0040`, `MU_MUTEX = 0x0080` (`aes/gemlib.h:25-26`) —
> the "send" and "mutex" pseudo-events used by `appl_write`/internal locking,
> not by `evnt_*`.

---

## `MOBLK` — the mouse rectangle (M1/M2, `aes/gemlib.h:28-32`)

`MOBLK { BOOL m_out; GRECT m_gr; }` where `GRECT { WORD g_x, g_y, g_w, g_h; }`
(`include/obdefs.h:155-162`). As it arrives **on the wire** in the AES
parameter block it is **five consecutive WORDs**, cast in place from the
`int_in[]` slots (`(MOBLK *)&MO_FLAGS`, `gemsuper.c:133`):

| Wire WORD | Field | Meaning |
|:---------:|-------|---------|
| 0 | `m_out` (flags) | **0 = fire when the mouse ENTERS** the rect; **1 = fire when it LEAVES** (is *outside*) it |
| 1 | `g_x` | rectangle x |
| 2 | `g_y` | rectangle y |
| 3 | `g_w` | rectangle width |
| 4 | `g_h` | rectangle height |

The condition tested is `in_mrect` (`geminput.c:76-79`):
`m_out != inside(xrat, yrat, &m_gr)` — i.e. the event fires when the mouse's
inside-ness differs from `m_out`. With `m_out = 0` the wait ends when the
cursor is inside the rectangle; with `m_out = 1` when it is outside.

> **Layout note (native vs. wire).** The C `struct moblk` uses `BOOL m_out`
> (`BOOL = int`, 4 bytes on the host — `include/portab.h:120`), so the *native*
> `sizeof(MOBLK)` is 12, not 10. But the AES ABI passes the rectangle as the
> 5-WORD `int_in` slice above (`MO_FLAGS..MO_HEIGHT` / `MMO1_*` / `MMO2_*`,
> `aesdefs.h:182-216`); only the low WORD of `m_out` carries the flag and the
> in-place cast lines the `GRECT` up with `int_in[1..4]`. The **contract is the
> 5-WORD slot layout**; the wider native struct is an EmuTOS host detail. NewTOS
> reads five WORDs per rectangle from the parameter block.

---

## `evnt_multi` (25) — the central multiplexer

**Source:** `gemsuper.c:145-162` (`EVNT_MULTI` arm) → `ev_multi()`
(`gemevlib.c:151-294`). The heart of every GEM event loop: block until **any**
of the requested events fires, return the bitmask of which one(s) did, and
report the mouse / keyboard / button state for all of them in one shot.

### Parameter-block interface

`int_in` layout (`aesdefs.h:193-224`):

| `int_in` | Macro | Meaning |
|:--------:|-------|---------|
| 0 | `MU_FLAGS` | event mask (`MU_*` bits; masked with `MU_TOSVALID` = 0x3F) |
| 1 | `MB_CLICKS` | button: **low byte** = number of clicks to wait for, **high byte** = enter(0)/leave(1) flag (`bclicks = LOBYTE(HIWORD(p))`, `geminput.c:559`; `flag = (buparm>>24)&0xff`, `:348-357`) |
| 2 | `MB_MASK` | button: which buttons to test (bitmask; byte) |
| 3 | `MB_STATE` | button: desired button state (byte) |
| 4 | `MMO1_FLAGS` | rect 1 `m_out` (0 enter / 1 leave) |
| 5..8 | `MMO1_X/Y/WIDTH/HEIGHT` | rect 1 `GRECT` |
| 9 | `MMO2_FLAGS` | rect 2 `m_out` |
| 10..13 | `MMO2_X/Y/WIDTH/HEIGHT` | rect 2 `GRECT` |
| 14 | `MT_LOCOUNT` | timer count, low WORD (milliseconds) |
| 15 | `MT_HICOUNT` | timer count, high WORD |

| `addr_in` | Macro | Meaning |
|:---------:|-------|---------|
| 0 | `MME_PBUFF` | pointer to the caller's 16-byte message buffer (`WORD msg[8]`) |

`int_out` layout (`aesdefs.h:194-199`) — the **shared return block** (also used
by the single-event waits, which fill `int_out[1..4]` via `&EV_MX`):

| `int_out` | Macro | Meaning |
|:---------:|-------|---------|
| 0 | `RET_CODE` | **which events fired** (a `MU_*` bitmask; can be several at once) |
| 1 | `EV_MX` | mouse x |
| 2 | `EV_MY` | mouse y |
| 3 | `EV_MB` | mouse button state (bit set = down) |
| 4 | `EV_KS` | keyboard state `kstate` = `gsx_kstate()` = VDI `vq_key_s` = `Kbshift(-1) & 0x000f`: bits 0-3 only (right/left Shift, Control, Alternate); CapsLock and bits 5-7 never reach it (`aes/gemgsxif.c:432-436`, `vdi/vdi_input.c:110-113,183-187`; A126) |
| 5 | `EV_KRET` | key code: **scancode in the high byte, ASCII in the low byte** (only if `MU_KEYBD` fired and a char was ready) |
| 6 | `EV_BRET` | mouse-button click count (only if `MU_BUTTON` fired) |

> **Correction vs. previous spec.** The old `evnt_multi` `int_in` table was
> **off by one for the rectangles and the timer** (it placed the M1 rect at
> `int_in[4..8]` correctly but listed the M2 rect at `9–13` and the timer at
> `14/15` in a way that conflated `flags,x,y,w,h`, and named `int_in[1..3]` only
> as generic "button clicks/mask/state"). The exact macros are `MB_CLICKS =
> int_in[1]`, `MB_MASK = int_in[2]`, `MB_STATE = int_in[3]`, `MMO1_* =
> int_in[4..8]`, `MMO2_* = int_in[9..13]`, `MT_LOCOUNT = int_in[14]`,
> `MT_HICOUNT = int_in[15]` (`aesdefs.h:202-221`). The **`int_out` return names**
> are `EV_MX/EV_MY/EV_MB/EV_KS/EV_KRET/EV_BRET` = `int_out[1..6]`
> (`aesdefs.h:194-199`) — note the last slot is **`EV_BRET` (button click
> count), not "mouse click count"**, and the requested `EV_MOUT`/`EV_BUTTON`/
> `EV_KRET`/`EV_MMGPBUFF` names from older docs do **not** exist in this EmuTOS
> build (only `EV_KRET` matches). The old table's slot *meanings* for
> `int_out[3..6]` were right; the input-side rectangle/timer/button mapping and
> the `int_out[1..2]` mouse-position slots are the fixed corrections.

### Behaviour

The dispatcher arm (`gemsuper.c:146-162`): if `MU_MESAG` is requested it sets
`rlr->p_flags |= AP_MESAG`; if `MU_TIMER` is requested it builds the 32-bit
millisecond count from `MT_HICOUNT`/`MT_LOCOUNT` via `MAKE_ULONG`; it packs the
button spec with `combine_cms(MB_CLICKS, MB_MASK, MB_STATE)`; then calls
`ev_multi(MU_FLAGS & MU_TOSVALID, rect1, rect2, [rect3=NULL,] count, buparm,
MME_PBUFF, &EV_MX)`. After it returns, if the fired set includes `MU_MESAG`
and the delivered `msg[0] == AC_CLOSE` it also sets `AP_ACCLOSE`.

`ev_multi` itself (`gemevlib.c:151-294`):

1. **Pre-check + drain forks.** `chkkbd()` samples the keyboard (queues a
   `kchange` fork if a key or a shift-state change is pending); `forker()`
   flushes the fork queue so any already-pending button/mouse/key changes are
   applied before the quick checks (`:169-170`).
2. **Quick-check keyboard** (`MU_KEYBD`): if a character is queued on the
   caller's console queue (`rlr->p_cda->c_q`), dequeue it into `EV_KRET`
   (`prets[4]`) and mark `MU_KEYBD` fired (`:173-182`).
   Keys reach that queue only while the caller owns the mouse — see
   "Who gets a key" below.
3. **Quick-check button / rectangles** (only if the caller owns the mouse,
   `rlr == gl_mowner`): `MU_BUTTON` uses `downorup(button, buparm)` on the
   current (or, if a transition is pending, the transition-time `pr_button`)
   state, and reports the click count in `EV_BRET` (`prets[5]`); `MU_M1`/`MU_M2`
   use `in_mrect` (`:184-213`).

   > **This gate is quick-check-only, and `gl_mowner` itself is frozen during
   > a control-manager track.** `rlr == gl_mowner` above decides only whether
   > this *non-blocking* check runs at all — it says nothing about the later
   > blocking arm (see the note after point 6). Separately, ownership does not
   > move by mouse-down while the control manager is tracking the menu bar:
   > `bchange`'s reassignment of `gl_mowner` is wrapped in `if (!gl_ctmown &&
   > (new == MB_DOWN) && (button == 0)) gl_mowner = chkown();`
   > (`aes/geminput.c:334-335`), and `gl_ctmown` is set for the whole of
   > `mn_do`'s tracking loop (`ct_mouse(TRUE)`/`ct_mouse(FALSE)`,
   > `aes/gemmnlib.c:277,513`; `aes/gemctrl.c:400-417`). A fresh button-down
   > landing anywhere on screen during that bracket does **not** hand the
   > mouse to the process under the cursor — it stays with `ctl_pd`. Likewise
   > `set_mown` (called from `w_setactive`'s `ct_chgown`, `aes/gemctrl.c:392-397`
   > at the head of the control-manager loop, `aes/gemctrl.c:441-442`) is a
   > no-op while a button is physically held (`if (!button) { … }`,
   > `aes/geminput.c:222-235`), so ownership cannot pass to an application
   > mid-click either. See [`menu.md`](./menu.md) "The drop-down tracking
   > loop" for the resulting **contract**: the click that selects a bar item
   > is never delivered to the application as `MU_BUTTON`.
4. **Quick-check timer:** if `tmcount == 0` the timer is already "expired" →
   `MU_TIMER` is set in the result without arming anything (`:216-220`), so
   this call does not *block* on the timer. **This is not "the call does not
   yield"** — the trap ends in `dsptch()` regardless (see "Wake conditions and
   dispatch" below) — **and it holds only for `tmcount == 0` exactly**: a
   non-zero count below `gl_ticktime` divides to 0 ticks and `adelay` turns that
   into **1** tick (`gemevlib.c:259`, `geminput.c:500-501`). The same 0 as
   `evnt_timer(0)` is **not** a poll (see `evnt_timer`).
5. **Quick-check message:** if the caller's pipe is non-empty (`p_qindex > 0`)
   or a control-manager fast-path message is pending (`p_msg.action >= 0`),
   deliver it with `ev_mesag(mebuff)` and mark `MU_MESAG` (`:222-229`).
6. **If nothing fired yet**, arm an async wait (`iasync`) for each requested
   event and block on `mwait(flags)` until one signals, then `acancel`s the
   others (`:231-265`).

   > **The blocking `MU_BUTTON` arm is not gated on ownership at all.**
   > `iasync(MU_BUTTON, buparm)` (`:238-239`) calls `abutton`
   > (`aes/gemasync.c:154-156` → `aes/geminput.c:545-565`), whose entry check
   > is an unconditional level test of the live `button` global: `if
   > (downorup(button, p)) azombie(e, 1);` (`geminput.c:549-551`) — no
   > `rlr == gl_mowner` test, unlike the quick-check in point 3. So a caller
   > that (re-)arms this wait while `button` already matches its `buparm`
   > gets an immediate `MU_BUTTON`, **regardless of who owns the mouse** —
   > including during a control-manager track where `gl_mowner == ctl_pd`
   > (see the note after point 3, and [`menu.md`](./menu.md) "The drop-down
   > tracking loop" for the one documented case this produces: a long press
   > held across the app's next `evnt_multi` call). If instead the level does
   > *not* match, the wait truly sleeps on the caller's own
   > `p_cda->c_bsleep` list, and is satisfied only by `post_button(gl_mowner,
   > …)` (`geminput.c:463-483`) — i.e. only while the caller *is*
   > `gl_mowner` — or discarded unsatisfied by `acancel` when some other
   > requested event fires first.

   > **The blocking `MU_M1`/`MU_M2` arm is tested once, at arm time, then
   > only through ownership (A73, 2026-09-20).** `iasync(MU_M1, (LONG)pmo1)` /
   > `iasync(MU_M2, (LONG)pmo2)` (`:242,244`) both route through `amouse`
   > (`aes/gemasync.c:157-162` → `aes/geminput.c:568-587`): it checks
   > `in_mrect` exactly **once**, against the pointer position *at the
   > instant the wait is armed* (`geminput.c:575-576`) — ownership-blind,
   > like `abutton`'s level test above. If the rectangle already matches at
   > that instant the event fires immediately; otherwise the request is
   > queued on the caller's own `p_cda->c_msleep` list and from then on
   > satisfied **only** by `post_mouse(gl_mowner, …)`
   > (`geminput.c:446-456`) — i.e. only while the caller owns the mouse, or
   > from the moment ownership returns to it. Two sites drive `post_mouse`:
   > `mchange` (`geminput.c:388-390`), on every raw mouse move, and
   > `set_mown` (`geminput.c:220-235`), once when the mouse changes hands.
   > A rectangle the pointer crosses *while the control manager owns the
   > mouse* (a bar/drop-down track, `gl_mowner == ctl_pd`) therefore never
   > reaches a parked M1/M2 arm — `post_mouse(ctl_pd, …)` only walks
   > `ctl_pd`'s own `c_msleep`, never the application's. If some other
   > requested event (typically the track's own `MN_SELECTED` message)
   > wakes the caller first, `acancel` discards the unsatisfied M1/M2 arm
   > without re-testing it (`aes/gemevlib.c:264`; `acancel` itself
   > `aes/gemasync.c:217-243`) — the rectangle condition is re-tested only
   > by the *next* call's fresh arm.
   >
   > Traced against a bar-title-select track ([`menu.md`](./menu.md) "Mouse
   > ownership over the track"): **selection on press** — the button is
   > still down when `ct_msgup` busy-waits (`aes/gemctrl.c:100-101`), so
   > `w_setactive` (and the `set_mown` inside it) does not run until *after*
   > the application has already consumed the `MN_SELECTED` message and
   > returned from `evnt_multi` — the app gets `MU_MESAG` alone, and any
   > armed `MU_M1`/`MU_M2` only on its *next* call, where the now-live
   > pointer position satisfies the fresh arm at once. **Selection on
   > release** — `button == 0` when `mn_do` returns, so `ctlmgr`'s next pass
   > reaches `w_setactive` (`aes/gemctrl.c:442`) → `set_mown(app)`
   > (`aes/gemwmlib.c:321-334` → `aes/geminput.c:220-231`) →
   > `post_mouse(app, xrat, yrat)` against the *live* position, before the
   > application is dispatched — so that same call returns `MU_MESAG |
   > MU_M1` (or `M2`) together.
7. **Fill the return block** with `ev_rets` (mouse x/y/button + shift state,
   `:42-58,268`), then, if the wait actually blocked, collect each fired
   event's result via `apret` — `EV_KRET` for `MU_KEYBD`, `EV_BRET` for
   `MU_BUTTON` (`:270-290`). Return the "which fired" bitmask as `RET_CODE`.

`ev_rets` reports the transition-time snapshot (`pr_xrat`/`pr_yrat`/`pr_button`)
when a button transition is pending (`mtrans > 1`), otherwise the live
`xrat`/`yrat`/`button`; `EV_KS` is always the live `kstate` (`gemevlib.c:42-58`).

**`kstate` carries bits 0-3 only (A126).** The AES's one source of the keyboard
state is `gsx_kstate` (`aes/gemgsxif.c:432-436`): `gsx_0code(KEY_STATE)`, the
VDI's `vq_key_s` (`vdi/vdi_input.c:110-113`), whose answer is `gshift_s()` =
`Kbshift(-1) & 0x000f` (`:183-187`). `chkkbd` stores it into `kstate` whenever
it changed (`aes/gemdisp.c:165,172-175` → `kchange`, `aes/geminput.c:288-293`);
`chkkbd` runs in every `schedule`, i.e. in the dispatch after every AES call
(`aes/gemsuper.c:455-466`). So `EV_KS` (and `graf_mkstate`'s `kstate`,
`menu_popup`'s `mn_keystate`) is `K_RSHIFT|K_LSHIFT|K_CTRL|K_ALT` at most: with
CapsLock on and Alt held it is `K_ALT` (0x08), not 0x18; `MODE_HOME`/
`MODE_INSERT` (0x20/0x40) and a program-stored bit 7, which BIOS `Kbshift`
keeps, never appear. Measured on the pinned oracle, tt and falcontc:
`tosdump/probes/akprobe.c`
(`measurements/measurement-2026-10-30-a126/RUNINFO.TXT`).
NewTOS: every AES site that reports the state reads it through one helper,
`tos-api/src/aes/gsxif.rs:286@gsx_kstate` (`tos-api/src/aes/gsxif.rs:292@kstate_of` for the pop-up's host),
which reads the BIOS shift byte `vq_key_s` reads and applies the same mask.

### Wake conditions and dispatch — contract (A112, 2026-10-17; re-extracted from source)

Which event wakes a blocked call, **independent of which process is running at
that moment**, and the rule that every AES call yields, are *observable* — a
program sees another program's timer/message arrive while it is itself
running — so they are contract, not scheduler internals. The mechanisms
(`EVB`, `nrl`/`drl`/`rlr`, fork ring) stay reference.

**1. Every wait arms one `EVB` per requested event and sleeps on the union.**
`ev_multi` calls `iasync()` once per set flag (`gemevlib.c:235-259`), each
chaining an `EVB` with `e_mask = flag` onto `rlr->p_evlist`
(`gemasync.c:130-148`), then `mwait(flags)` sets `p_evwait = flags` and — only
if none of those bits is already in `p_evflg` — marks the process `WAITIN` and
calls `dsptch()` (`gemasync.c:117-127`; it lands on `nrl` via `mwait_act`,
`gemdisp.c:82-93`). The single waits are the same with one flag:
`ev_block` = `mwait(iasync(code, arg))` + `apret` (`gemevlib.c:64-69`).
There is **no** single "input" wake key: each event has its own source (table).

**2. The wake itself is `signal()`** (`gemasync.c:35-61`), called by
`azombie()` (`:64-78`) when an `EVB` completes: it ORs the event's mask into
the owner's `p_evflg`, and if the owner is **not** `rlr` and
`p_evflg & p_evwait != 0` it moves the owner from `nrl` to `drl`
(`:46-59`). `schedule()` then appends everything on `drl` to `rlr`
(`gemdisp.c:193-197`). Any process can be the one that completes the `EVB`
(`signal` names `e->e_pd`, not the caller), and `forker()` runs with
`rlr = -1` (`gemdisp.c:107-108`), so a completion made from the fork ring
wakes its owner whenever an awaited bit is set.

| Call (flag) | Armed by | Completed at arm time (tested once) | Wake source while blocked |
|---|---|---|---|
| `evnt_keybd` / `MU_KEYBD` | `akbin` (`geminput.c:486-493`) | a key is already in the caller's own `p_cda->c_q` | `kchange` → `post_keybd(gl_mowner->p_cda, ch)` (`geminput.c:288-308`): **only if the caller is the key/mouse owner**. `kchange` is a fork queued by `chkkbd` (`gemdisp.c:158-178`), which runs in `schedule()` (`:191`) and at the head of `ev_multi` (`gemevlib.c:169`) |
| `evnt_button` / `MU_BUTTON` | `abutton` (`geminput.c:545-565`) | `downorup(button, parm)` on the live level — ownership-blind | `post_button(gl_mowner, …)` (`:463-483`) from `bchange` (`:344`), `b_delay` and `set_mown` (`:233`): owner only |
| `evnt_mouse` / `MU_M1`, `MU_M2` | `amouse` (`geminput.c:568-587`) | `in_mrect` at the instant of arming — ownership-blind | `post_mouse(gl_mowner, …)` (`:446-456`) from `mchange` (`:390`) and `set_mown` (`:231`): owner only |
| `evnt_mesag` / `MU_MESAG` | `ev_mesag` → `ap_rdwr` → `aqueue(FALSE,…)` (`gemevlib.c:100-117`, `gemaplib.c:69-89`, `gemqueue.c:85-125`) | the caller's pipe is non-empty (`qready = p_qindex > 0`, `gemqueue.c:99`); or the `p_msg.action >= 0` `WM_ARROWED` shortcut (`gemevlib.c:102-114`) | a successful **write into the caller's own pipe by any process** — `appl_write`/`ap_sendmsg` → `aqueue(TRUE,…)` services the parked reader on `p_qdq` (`gemqueue.c:104-118`). Not input, not the owner |
| `evnt_timer` / `MU_TIMER` | `adelay` (`geminput.c:496-542`) | **never** (see "0-ms timers") | the tick handler `tikcod` counts `CMP_TICK` down (`gemdosif.S:389-396`); at 0 it `forkq`s `tchange` (`:398-401`), which pulls expired `EVB`s off `dlr` via `evremove` → `azombie` → `signal` (`gemflag.c:32-68`). Independent of owner and of the running process — it needs only a `forker()` pass (see point 4) |

Notes on the table:

* **Only keyboard, button and mouse are owner-gated.** A blocked `MU_MESAG` or
  `MU_TIMER` is woken whoever owns the mouse and whoever runs; a process that
  waits on `MU_MESAG | MU_TIMER` is therefore never "waiting for input".
* The `p_msg.action` shortcut needs no wake: `handle_arrow_msg` sets it only
  when the target is **not** `WAITIN`, and sends a real `ap_sendmsg` message
  when it is (`gemctrl.c:120-132`).
* A write that finds the pipe full parks on `p_qnq` (`MU_SDMSG`) and is woken
  by the next successful read of that pipe (`gemqueue.c:96-118`); the details
  are in [`application.md`](./application.md).
* On wake, `ev_multi` does **not** re-test any condition. `which = mwait(flags)`
  is simply `p_evflg` — every `EVB` that completed before the process actually
  resumed — then `acancel(flags)` drops the still-pending ones and ORs in the
  already-`COMPLETE` ones (`gemevlib.c:261-264`, `gemasync.c:217-243`); results
  come from each `EVB`'s `e_return` via `apret` (`:270-290`). So several bits can
  be set in one return even from a blocking call.

**3. Every AES call gives up the CPU.** The trap enters `super()`
(`gemdosif.S:258`); for selector 200 it runs `xif()` and **falls through** to
`dsptch()`, for 201 it runs only `dsptch()` (`gemsuper.c:454-466`). So a call
that returned without blocking — `appl_read` on a non-empty pipe, an
`evnt_multi` whose quick checks fired, `evnt_timer`'s own completion, `graf_mkstate` —
still ends in `dsptch()`. (`mwait`'s own `dsptch()` is skipped when the event is
already complete, `gemasync.c:120-124`; `super()`'s is not skippable.)
`dsptch()` is a no-op only when `indisp` is already set
(`gemasm.S:129-132`, i.e. when called from fork-ring code).

`disp()` (`gemdisp.c:221-253`): takes the running process off the head of `rlr`
(`:226-227`); if it is `WAITIN` it goes to `nrl` unless an awaited bit is already
set (`mwait_act`, `:82-93`), otherwise `disp_act` appends it to the **end** of
`rlr` (`insert_process`: "find the end", `gempd.c:133-145`); then it drains the
fork ring (`forker`) and runs `schedule()`, which polls the keyboard and appends
`drl` to `rlr`, spinning until `rlr` or the fork ring is non-empty
(`gemdisp.c:181-205`); finally `switchto(rlr->p_uda)` resumes the **head** of `rlr`
(`:252`).

* **Order = round robin over `rlr`.** The caller of `dsptch()` goes to the back,
  processes already on `rlr` run first in list order, processes woken during
  this pass (`drl`) are appended **behind the caller** (the caller is inserted by
  `disp_act` before `schedule()` runs, `gemdisp.c:231-243`). If the caller is
  alone on `rlr`, it is resumed at once and the woken process runs at the caller's
  *next* `dsptch()`. (Inferred from the list handling above: `drl` is a push-front
  stack — `signal`, `gemasync.c:56-57`; `iprocess`, `gempd.c:125-128` — so several
  processes woken in one pass join `rlr` in reverse wake order.)
* **NewTOS binding (A115, 2026-10-25).** The order above is reproduced as
  contract, not left to the scheduler: a ready list with `rlr`/`drl`
  semantics (`tos-api/src/bdos/process/sched.rs`, "The ready list"). Every
  place that makes a process ready goes through it — `pstart` = `admit` (push
  onto `drl`, `gempd.c:125-128`), `signal` = `wake`/`wake_timers` (push onto
  `drl`, `gemasync.c:56-57`), `disp_act` = `ready` (end of `rlr`),
  `mwait_act` = `unready` (`nrl`). Consequences: an `appl_write` reader
  joins `rlr` behind the writer; timer completions of one pass are pushed in
  `dlr` order — deadline ascending, a later-armed delay before an equal
  earlier one (`adelay`, `geminput.c:520-540`) — and so run in reverse; desk
  accessories, `pstart`ed in load order (`geminit.c:309-315`), are first
  run in **reverse** load order. NewTOS's EL1 kernel client (the boot path's
  accessory barrier, EmuTOS pid 0) is one entry of `rlr` while its frame is
  parked; its waits are polls, so it is appended like a `dsptch()` caller,
  never put on `nrl`. NewTOS-only broadcast wakes (one key waking every
  matching waiter, e.g. the window-tree lock hand-over or input) push in slot
  order; only the waiter whose predicate holds proceeds.
* **A process that never calls the AES never dispatches.** There is no
  preemption: the fork ring and the tick work (`tchange`, mouse/key forks) only
  run inside `disp()` or at the head of `ev_multi`. A program that polls with
  AES calls therefore feeds the others; one that polls without any does not.
* **`appl_yield` (17, `CONF_WITH_PCGEM`)** is `dsptch()` and nothing else
  (`gemsuper.c:115-119`); since `super()` dispatches again afterwards the call
  yields twice — observable only as extra latency. Returns the default `TRUE`.

**4. Who runs `forker()`** (and so delivers input and timer expiry): `disp()`
(`gemdisp.c:239-244`) on every dispatch, and the head of `ev_multi`
(`gemevlib.c:169-170`). `ev_button`/`ev_mouse`/`ev_block` have no pre-drain
(`gemevlib.c:84-94,123-128`) and rely on the dispatch.

**5. When the immediate conditions are tested.** *Once, at arm time — never
again after a resume.* `ev_multi` runs its quick checks (`gemevlib.c:172-229`)
and, only if they found nothing, arms each event, and each `iasync` arm tests its
own condition once more as the table shows (`gemasync.c:149-176`). From then on
a condition is satisfied only by a `post_*`/`aqueue`/`tchange` completion; after
`mwait` returns, no button level, rectangle, pipe or key queue is looked at
again (`gemevlib.c:261-290`). Differences between the two arm-time tests:
the quick check of button/rectangles is owner-gated (`rlr == gl_mowner`,
`:185`), the `abutton`/`amouse` arm is not; message and key tests exist in both.
Only a *new* AES call arms and tests again.

**6. 0-ms timers.** The two forms differ:

* `evnt_timer(0)` → `ev_timer` → `ev_block(MU_TIMER, 0/gl_ticktime)`
  (`gemevlib.c:134-137`) → `adelay`, whose first statement is `if (c == 0L) c = 1L`
  (`geminput.c:500-501`). It always arms a real one-tick delay and sleeps until
  the next tick completes it through `tchange`. Not a poll; elapsed time is the
  next tick boundary (0..1 tick periods — inferred: `tikcod` decrements on each
  tick, `gemdosif.S:392-396`).
* `evnt_multi(MU_TIMER, …, 0 ms)`: `tmcount == 0` completes `MU_TIMER` in the quick
  check, nothing is armed, **no blocking** (`gemevlib.c:216-220`). Still ends in
  `dsptch()` via `super()`. Other requested events that are already satisfied
  are reported in the same return; none that are not are armed.
* Any non-zero `tmcount` below one tick (`ms/gl_ticktime == 0`) is armed as
  **one tick** (`gemevlib.c:259` + `geminput.c:500-501`).

### The button state an event reports (`mtrans` / `pr_*`) — contract (LDG-M3 B1, 2026-10-06)

The *values* `EV_MX`/`EV_MY`/`EV_MB` (and `EV_BRET` of the quick check) are
contract; the globals that produce them are reference. How EmuTOS produces them:

* **`bchange(new, clicks)` publishes one button transition**
  (`aes/geminput.c:329-345`): `mtrans++; pr_button = button; pr_mclick =
  mclick; pr_xrat = xrat; pr_yrat = yrat; button = new; mclick = clicks;` —
  the state *before* the transition is recorded, `mtrans` counts transitions.
* **`ev_rets` clears the count** (`mtrans = 0`, `gemevlib.c:57`). It runs at
  the end of **every** `ev_button` (`:91`), `ev_mouse` (`:126`) and `ev_multi`
  (`:268`) — also the AES-internal ones: `fm_do` (`gemfmlib.c:256,312-317`),
  `gr_stilldn` (`gemgrlib.c:62-66`, i.e. `gr_watchbox`/`gr_dragbox`/
  `gr_rubwind`/`gr_slidebox`), `mn_do` (`gemmnlib.c:597`), the pop-up
  (`gemmnext.c:268,314,351`) and the Control Manager's own loop
  (`gemctrl.c:456-462`). A click the AES consumed itself is therefore not
  reported again by the application's next event.
* **When the transitions are published.** `b_click` (`geminput.c:110-147`)
  publishes a change at once (`forkq(bchange, MAKE_ULONG(state, 1))`) unless a
  waiter asks for more than one click and the change is a press
  (`gl_bpend && state`): then it opens the button-delay window
  (`gl_bdesired = state; gl_bclick = 1; gl_bdely = gl_dclick`), counts
  further entries into `gl_bdesired` while it is open, and publishes nothing.
  When the window closes — by the tick countdown or by a move of more than ±2
  pixels (`mchange` → `b_delay(gl_bdely)`, `:365-368`) — `b_delay` publishes
  `bchange(gl_bdesired, gl_bclick)` and, if the button is no longer in that
  state, `bchange(gl_btrue, 1)` (`:155-171`).
* **Consequence — the contract.** A click (press + release) whose release falls
  inside the double-click window is published as two transitions at the
  window's close, so the event that reports it has `mtrans == 2` and returns
  **`EV_MB` = the pressed button** (`pr_button`), with `EV_MX/EV_MY` = the
  pointer at the release. A press held past the window is one transition: the
  event reports the live state, button still down. A double click reports
  `EV_BRET = 2` and `EV_MB` = down. `evnt_mouse` alone overwrites `EV_MB`
  with the live `button` (`gemevlib.c:127`); its `EV_MX/EV_MY` follow
  `ev_rets` like the others.
* **The quick check uses the record first.** `ev_multi`'s owner-only quick
  check tries `(mtrans > 1) && downorup(pr_button, buparm)` before
  `downorup(button, buparm)` (`gemevlib.c:188-198`) and reports `pr_mclick`
  for it: a click made while the application was not waiting (both
  transitions published between two calls) still satisfies its next wait for
  the press, with the recorded click count and no new click window.

WinDom handles a click only when `evnt.mbut & 1`
(`sdk/windom/overlay/src/evnt_windom.c:334`); with the live state (0 after the
window) every normal click on a WinDom form was dropped.

**NewTOS mapping** (`tos-api/src/aes/input.rs`, `event.rs`). `b_click`,
`bchange`, `b_delay`, `post_button`, `abutton`, `ev_rets` are reproduced with
EmuTOS's state (`gl_btrue`, `gl_bdesired`, `gl_bclick`, `gl_bdely`,
`gl_bpend`, `mclick`, `mtrans`, `pr_*`, one parked button wait per AES
identity); see "The double-click window" below for how the window is timed
(ZVIEW02, 2026-10-07 — until then the window was opened by the waiting call
after the press, the press published at once, and the count cut short by a
`MU_TIMER`). The AES-internal waits listed above consume the transitions at
the point their EmuTOS counterpart's `ev_*` returns (the press or key in
`fm_do`/fsel/`form_alert`, the release in the `gr_stilldn` loops and
`ev_button` up-waits, the end of the menu track, the press on a frame gadget
or an untopped window in the Control Manager); where they return on a
*button*, a window still open is closed first (`input::ev_consume`), because
their EmuTOS counterparts block on the published `button` and cannot return
before the window has closed.

### The double-click window (`gl_bdely` / `gl_bpend`) — contract (ZVIEW02 / A111, 2026-10-07)

The *values* are contract (which call returns `MU_BUTTON`, its `EV_BRET` and
`EV_MB`, and when); the globals are reference. How EmuTOS produces them:

* **Who opens a window.** `abutton` (`aes/geminput.c:545-565`), the arm of
  every `MU_BUTTON` wait (`ev_button`, `ev_multi` when its quick checks found
  nothing), answers at once with **one** click if `downorup(button, buparm)`
  already holds (`azombie(e, 1)`, ownership-blind); otherwise it parks the
  wait on the caller's `c_bsleep` list and, for a request of more than one
  click (`LOBYTE(HIWORD(buparm)) > 1`), increments `gl_bpend`.
  `post_button` decrements it when it completes such a wait (`:477-479`);
  **`acancel` does not** — `takeoff` only unlinks the block
  (`aes/gemasync.c:96-114,217-243`). An application whose multi-click wait is
  ever ended by another event (a `MU_TIMER`, a message, a key) therefore
  leaves `gl_bpend > 0` for the rest of the session, and from then on *every*
  press opens a window. `gl_bpend` is not reset by an AES restart
  (`aes/geminit.c:661-664` resets only `gl_btrue`/`gl_bdesired`/`gl_bdely`/
  `gl_bclick`).
* **Opening and counting.** `b_click` (`:110-147`): with no window open, a
  press while `gl_bpend != 0` sets `gl_bclick = 1; gl_bdesired = state;
  gl_bdely = gl_dclick` and publishes **nothing**; with a window open, a change
  *into* `gl_bdesired` does `gl_bclick++; gl_bdely += 3` — every further click
  extends the window by **three ticks**, it does not get a window of its own.
  Any other change is published at once (`forkq(bchange, (state, 1))`).
* **Timing.** `gl_dclick = gl_dcrates[gl_dcindex] / gl_ticktime`
  (`ev_dclick`, `aes/gemevlib.c:300-309`): ticks of `gl_ticktime` = the
  VDI's `Tickcal`, 20 ms — 11 ticks at the default index 3 (220 ms), 22/16/13/8
  for indices 0/1/2/4. The AES timer handler calls `b_delay(1)` on every tick
  (`aes/gemdosif.S:389-411`), **independently of any application call**.
* **Closing.** When `gl_bdely` reaches 0 — by the countdown, or at once by a
  pointer move of more than ±2 (`mchange` → `b_delay(gl_bdely)`, `:365-368`)
  — `b_delay` publishes `bchange(gl_bdesired, gl_bclick)` and, if the button
  has left that state since, `bchange(gl_btrue, 1)` (`:155-171`).
  `bchange` → `post_button(gl_mowner, new, clicks)` completes every parked
  wait of the mouse owner that `new` satisfies, with `min(clicks of the
  transition, clicks requested)` (`:463-483`).
* **Consequences — the contract.**
  1. A double click is reported by the wait that is parked **when the window
     closes**, with `EV_BRET = 2`. A `MU_TIMER` that ends a call while the
     window is open does not end the window: that call returns `MU_TIMER`
     alone, with `EV_MB = 0` (the press is not published yet), and a later
     call gets the gesture. (zView polls with a 20-ms timer and opens a folder
     only on `nb_click == 2 && evnt.mbut == 1`,
     `Applications/z-tools/zview/catalog/catalog_mouse.c:132`.)

  2. `EV_MB` of that return is `pr_button` = down when the second release fell
     inside the window (two transitions), the live state — down — when the
     second press is held past it (one transition).
  3. A third press after the extended window (`gl_dclick + 3` ticks after the
     first press for a double click) starts a new gesture.
  4. A press while `gl_bpend > 0` is invisible to `button` (and so to
     `graf_mkstate`, `gr_mkstate`, `aes/gemgrlib.c:458-464`) until its window
     closes: every click, also a single one and also one on a window frame or
     for a one-click waiter, is delivered one window late.
  5. The quick check reports `mclick`/`pr_mclick` unbounded by the request
     (`aes/gemevlib.c:190-198`); only `post_button` bounds the count.

**NewTOS mapping.** `input::b_click`/`b_delay`/`post_button`/`abutton`/
`acancel` are the EmuTOS functions; there is no timer interrupt in the AES,
so the countdown is evaluated lazily against the host clock in whole 20-ms
ticks (`input::b_delay_poll`) at every mouse packet (`b_click`, `mchange`)
and at the head of every Event Manager pass — the same transitions in the same
order. `post_button` walks the parked waits of the **mouse owner only** —
`post_button(gl_mowner, …)` (`aes/geminput.c:344,463-483`): the identities
`input::same_pd` folds into the owner's `AESPD` (the whole application chain,
or one desk accessory), none while `ctl_pd` owns the mouse — and wakes them
through the scheduler's input class. *(Corrected 2026-10-21, GH1: until then
it walked the waits of every application identity while any application
owned the mouse, so a press in a desk accessory's dialog also completed the
desktop's parked `MU_BUTTON`, `TASK/GH1.md`.)* `ev_multi`'s quick check
(`if (rlr == gl_mowner)`, `aes/gemevlib.c:185`) asks whether **the caller's**
`AESPD` owns the mouse (`input::owns_mouse`, the same fold as `post_button`'s).
*(Corrected 2026-10-25, A118: until then it asked "an application owns the
mouse" (`input::own_mouse`), so a desk accessory's quick check fired on the
desktop's press — e.g. `evnt_multi(MU_BUTTON | MU_TIMER, 0 ms)` returned
`MU_BUTTON | MU_TIMER` instead of `MU_TIMER` — and a parked rectangle wait of
a process that does not own the mouse was completed against the live
position, where only `post_mouse(gl_mowner, …)` may complete it,
`aes/geminput.c:390,446-456`; `tos-api/tests/aes_a118_lock_takers.rs`.)* The Control Manager runs inline
(below); its loop wait (`ctlmgr`'s `ev_multi(…, 0x0001ff01L, …)`,
`aes/gemctrl.c:456-462`) is a latch that `post_button` sets when it completes
a left press for `ctl_pd` outside a menu track, so a short click on a frame
gadget whose press and release a window published together is still handled
(`window::intercept`). Not reproduced (pre-existing, outside ZVIEW02):
`graf_mkstate` reports the raw VDI button byte, not `button`, so during an
open window it shows the press that consequence 4 hides; and the internal
`fm_do`/fsel/menu loops act on the raw press at once instead of one window
later (they close the window themselves, see `ev_consume` above).

### NewTOS deviation — the application does not run while a drop-down is open (A91, 2026-09-23)

**EmuTOS.** The Control Manager is a *process*:
`gl_mowner = ctl_pd = iprocess("SCRENMGR", ctlmgr)` (`aes/geminit.c:888`), and
`ctlmgr` is an endless `w_setactive(); ev_multi(…); wm_update(BEG_UPDATE); …`
loop of its own (`aes/gemctrl.c:427-486`). A menu track therefore runs in
`ctl_pd`'s context — `mn_do`'s state machine (`aes/gemmnlib.c:530-690`) blocks
`ctl_pd`, not the application. The application stays on the AES's ready/delay
lists throughout: a `MU_TIMER` it asked for is posted by `tchange`
(`aes/gemflag.c:33-64`) and dispatched to it while the drop-down is on the
screen, and so is anything else that is not gated on mouse ownership.

**NewTOS.** There is no AES scheduler and no `ctl_pd` process. The Control
Manager runs **inline**, inside the application's own `evnt_multi` trap:
`menu_intercept` / `window::intercept` at the head of the wait loop
(`tos-api/src/aes/event.rs`), and `menu::track` (`tos-api/src/aes/menu.rs`)
drives the whole `mn_do` state machine there and returns only with the
selection or the cancel. While it runs, the application's `evnt_multi` has not
returned: **its `MU_TIMER` does not fire, and no other arm of that call is
evaluated.** Since A73 the track also stays open in `OUTSIDE_STATE` — the
pointer between the bar and the drop-down box, which no event leaves until it
re-enters or a button transition cancels (`gemmnlib.c:415-443,559-585`) — so
"while it runs" can be arbitrarily long: a drop-down left open holds the
application for as long as the user leaves the pointer there.

"The application's own trap" is whichever client is waiting with `MU_MESAG`
when the pointer reaches the bar or a frame is pressed — with accessories that
is as often a desk accessory idling in `evnt_multi(MU_MESAG)` as the
application. The waiter is never the recipient: a menu pick goes to the bar
owner `gl_mnppd` (`gemctrl.c:353`, [`menu.md`](./menu.md) "`MN_SELECTED`
message", A117), a frame message to the window owner `w_owner`
(`gemctrl.c:314`), an `AC_OPEN` to the accessory (`gemctrl.c:337-340`).

This is a property of the inline model, not of A73; A73 only made the window
long enough to notice. Nothing else is affected: the messages, their word
contents and the order they arrive in are unchanged, and no application that
does not rely on timer ticks *during* a menu track can observe it. What can:
a clock accessory, a progress animation, anything driven from `MU_TIMER`
alone — it stalls while a menu is open and resumes afterwards.

**Where it disappears.** The cooperative kernel (milestone 2b,
`docs/analysis/aes-audit/00-MASTER.md`) gives the AES real processes; the
Screen Manager becomes one, the track moves into it, and the application keeps
being dispatched during a track exactly as in EmuTOS. The deviation ends there
by construction — there is no separate fix to make. `TASK/ACC03.md` is where it
is decided: an accessory is a second `AESPD` parked in `evnt_multi`
(`aes/geminit.c:654-745`), the ticket lists A91 as a dependency, and its third
work item is exactly "must the inline Control Manager go for this, or can
accessories live without events during a track".

### Who gets a key — the mouse owner (`gl_mowner`) — contract (KEYOWNER, 2026-10-08)

**EmuTOS.** A key belongs to the process that owns the mouse at the moment the
AES picks it up. `gl_mowner` is declared *"current mouse/keybd owner"*
(`aes/geminput.c:52`); `chkkbd` reads a character only while that owner's
keyboard queue has room and forks `kchange` with it (`aes/gemdisp.c:158-177`);
`kchange` hands it to exactly that process — `post_keybd(gl_mowner->p_cda, ch)`
(`aes/geminput.c:288-296`) — which wakes the owner's parked `MU_KEYBD` wait or
queues the key in the owner's `c_q` (`:299-308`). A wait reads only its own
queue: `ev_multi`'s quick check `pc = &rlr->p_cda->c_q` (`aes/gemevlib.c:
173-182`), `akbin` for the blocking arm and for `evnt_keybd`
(`aes/geminput.c:486-493`, `gemasync.c:151-152`, `gemsuper.c:127`). A process
that does not own the mouse never sees a key, however long it waits.

`gl_mowner` moves at four places, and the keyboard moves with it:

| Where | Rule | Source |
|-------|------|--------|
| the Control Manager's loop head | `w_setactive()`: the owner of the top window (`w_top()`, the desktop window when none is open) gets the mouse — `ct_chgown(D.w_win[wh].w_owner, &work)`, skipped for a NULL owner | `aes/gemctrl.c:439-442`, `aes/gemwmlib.c:322-334` |
| `wind_update(BEG_MCTRL)` / `form_do` | `fm_own(TRUE)` on the 0 → 1 edge of `ml_ocnt`: remember the owner (`get_mown(&ml_pmown)`), give the mouse to the caller (`ct_chgown(rlr, &gl_rscreen)`); `fm_own(FALSE)` on the 1 → 0 edge gives it back to `ml_pmown` | `aes/gemfmlib.c:82-107`, `aes/gemwmlib.c:1778-1797` |
| a left-button press | `if (!gl_ctmown && new == MB_DOWN && button == 0) gl_mowner = chkown()`: inside the control rectangle → `gl_cowner`, a window frame or the bar → `ctl_pd`, the desktop → the desktop window's owner | `aes/geminput.c:310-335` |
| the pointer entering the menu titles, buttons up | `gl_mowner = ctl_pd` | `aes/geminput.c:388-389` |

Two guards apply to every hand-over through `set_mown`: nothing changes while
a button is held (`if (!button)`, `aes/geminput.c:220-235`), and nothing
changes while a menu track holds the mouse (`ct_chgown`'s `if (!gl_ctmown)`,
`aes/gemctrl.c:392-397`).

**Consequences an application can observe** (the contract):

1. **Normal typing goes to the top window's owner** — with no window open, to
   the owner of the desktop window, i.e. the application chain (`wm_init`
   records `rlr`, `aes/gemwmlib.c:1254-1290`).
2. **During `wind_update(BEG_MCTRL)` the keys go to the caller.** The Control
   Manager cannot take them back while `ml_ocnt > 0`: its own `ev_multi`
   (`MU_KEYBD | MU_BUTTON [| MU_M1 | MU_MESAG]`, `aes/gemctrl.c:445-462`) is
   fired only by what `ctl_pd` owns, and during the bracket it owns nothing —
   keys and presses go to the holder (the control rectangle is the whole screen,
   so `chkown` answers `gl_cowner`), and `gl_mntree` is held (`NULL`), so there
   is no bar to track. A desk accessory tracking its own drop-down
   (`mn_popup`'s model, `aes/gemmnext.c:312-353`) therefore receives Esc,
   Return and the cursor keys; the application waiting in its own `evnt_multi`
   at the same time does not.
3. **A desk accessory gets keys only while it owns the mouse:** its window is
   topped, or it is inside `BEG_MCTRL` / `form_do` / `form_alert`. An idle
   accessory waiting for `MU_MESAG` (or even `MU_KEYBD`) takes nothing from
   the application.
4. **The application chain is one owner.** EmuTOS runs the desktop, every
   launched program and every `Pexec` child on one `AESPD`, `D.g_int[0]`
   (`aes/geminit.c:851-877`, `gemshlib.c:572`), and each accessory on its own
   (`D.g_acc[]`, `geminit.c:819`). A key owned by the chain is the chain's,
   whichever of its members is waiting.

**NewTOS (tos-api `aes::input`, `aes::event`).** `gl_mowner` is held as the
existing Control-Manager flag (`ctl_pd` or "an application", A73/H02) plus the
**AES identity** the application half names, maintained at the four places of
the table above with the two guards. Identities map onto EmuTOS's process
model as point 4 says: a registered accessory (`aes::acc`) is its own owner;
the kernel desktop and every GEMDOS slot that is not an accessory are the one
chain owner. A desktop window without a recorded owner is the chain's
(`wm_init`'s `rlr`). `evnt_multi`'s `MU_KEYBD` arm and `evnt_keybd` take a key
only when the caller is the current key owner.

**Where NewTOS differs, and why** (named, not rationalised — each is a
property of the inline Control Manager and the shared key ring):

* **The key is assigned when it is read, not when `chkkbd` forks it.** The
  IKBD ring is also the GEMDOS/BIOS console's (`Cconin`, `Bconin`), and with
  the cooperative kernel desk accessories keep running their `evnt_*` loops
  while a TOS program reads the console. Moving keys into a per-process AES
  queue from an accessory's wait would take them away from that program. So
  the key stays in the ring until its owner reads it; a key that arrives while
  A owns the mouse but is read only after ownership moved to B goes to B.
* **Keys typed while `ctl_pd` owns the mouse are not eaten.** EmuTOS's
  `ctlmgr` swallows them (its `MU_KEYBD`); here they wait in the ring and go
  to the next application owner. This happens only with a button held on a
  window frame or the bar, or during a menu track.
* **`w_setactive` runs at every pass of every wait** (the inline Control
  Manager, see A91 above), not only after a Control-Manager event — except
  while `ml_ocnt > 0`, where it keeps the owner, because `ctlmgr` is dormant
  then (point 2). After an accessory's `form_do`/`form_alert` ends, EmuTOS
  leaves the accessory owner until `ctlmgr` next loops; NewTOS hands the
  keys back to the top window's owner at the next wait pass of any client.
* **The kernel's own modal loops** (`form_do`, `form_alert`, `fsel_input` —
  busy loops that never yield) read the ring directly: they run under
  `fm_own(TRUE)`, so the caller is the owner (with the button up on entry), and
  no other client runs until they return.
* `evnt_keybd` runs the Control Manager's loop head (`w_setactive`) on each
  pass as `evnt_mouse` does, so a key owner taken by a hover over the bar
  comes back to the waiter (EmuTOS's independent `ctlmgr` would loop there).

Tests: `tos-api/tests/aes_key_owner.rs`.

---

## The single-event waits (opcodes 20–24, 26)

Each parks on one fixed event; the dispatch arms live at `gemsuper.c:125-165`.
`ev_block(code, arg)` (`gemevlib.c:64-69`) is the AES scheduler primitive:
`mwait(iasync(code, arg))` then `apret(code)` — arm the async event, yield to
other AES processes, and return that event's result. Its mechanics are
**reference**; the per-call contract is below.

### `evnt_keybd` (20)

**Source:** `gemsuper.c:126-128` → `ev_block(MU_KEYBD, 0)`.

| Slot | Meaning |
|------|---------|
| `int_in` / `addr_in` | (none) |
| `int_out[0]` | **return: the key code** — scancode in high byte, ASCII in low byte |

Blocks until a key is pressed and returns its packed code. **Note the return
slot:** unlike `evnt_multi` (where the key goes in `EV_KRET` = `int_out[5]`),
`evnt_keybd` returns the key in **`int_out[0]` (`RET_CODE`)** directly. The
encoding is the raw AES/VDI keyboard word (`gsx_char` → VDI string input,
`gemgsxif.c:459-471`): high byte = IKBD scancode, low byte = ASCII (0 for
non-ASCII keys such as the function/cursor keys).

The wait is `akbin` on the caller's own queue (`aes/geminput.c:486-493`): only
a key that arrives while the caller owns the mouse completes it ("Who gets a
key", above).

### `evnt_button` (21)

**Source:** `gemsuper.c:129-131` → `ev_button(B_CLICKS, B_MASK, B_STATE,
&EV_MX)` (`gemevlib.c:84-94`).

| Slot | Macro | Meaning |
|------|-------|---------|
| `int_in[0]` | `B_CLICKS` | **low byte** = number of clicks to wait for; **high byte** (not bit!) = enter(0)/leave(1) flag. `combine_cms` packs the whole WORD into bits 31..16 of the LONG (`gemevlib.h:34-48`), so `downorup` reads the flag as `(buparm>>24)&0xff` and `abutton` reads the count as `LOBYTE(HIWORD(p))` |
| `int_in[1]` | `B_MASK` | button bitmask (which buttons to test; byte) |
| `int_in[2]` | `B_STATE` | desired button state (byte) |
| `int_out[0]` | `RET_CODE` | **return: the actual click count reached** |
| `int_out[1..4]` | `EV_MX/MY/MB/KS` | mouse x, y, button state, shift state (filled via `&EV_MX`) |

Blocks until the button state satisfies the `downorup` predicate the requested
number of times (or the multi-click delay window closes), and returns how many
clicks actually occurred. The clicks/mask/state are packed by
`combine_cms(clicks, mask, state)` into a LONG: **clicks in the high WORD, mask
in byte 2, state in byte 3** (`gemevlib.h:34-50`), and decoded by
> **Corrected 2026-09-01.** Two entries in this file previously said "high **bit**"
> where the source says "high **byte**". They are not the same: TeraDesk asks with
> `bclicks = 0x102`, whose high *bit* is 0 but whose high *byte* is 1. Implementing
> the "high bit" reading leaves the button condition inverted — which is exactly the
> defect measured on hardware and fixed in `2d3865b`. The spec carried the bug.

`downorup(new, buparm)` as `flag = (buparm>>24)&0xff` (the enter/leave bit, from
the top byte of the clicks word), `mask = (buparm>>8)&0xff`, `val = buparm&0xff`
(`geminput.c:348-357`). The predicate is
`((mask & (state ^ new)) == 0) != flag`.

### `evnt_mouse` (22)

**Source:** `gemsuper.c:132-134` → `ev_mouse((MOBLK *)&MO_FLAGS, &EV_MX)`
(`gemevlib.c:123-128`).

| Slot | Macro | Meaning |
|------|-------|---------|
| `int_in[0]` | `MO_FLAGS` | rectangle `m_out`: 0 = wait to ENTER, 1 = wait to LEAVE |
| `int_in[1..4]` | `MO_X/Y/WIDTH/HEIGHT` | the `GRECT` |
| `int_out[0]` | `RET_CODE` | return: 1 (`ev_mouse` is `void`; `crysbind`'s `ret` stays `TRUE`) |
| `int_out[1..4]` | `EV_MX/MY/MB/KS` | mouse x, y, **live** button state, shift state |

Blocks until the mouse crosses the rectangle boundary in the requested
direction (`ev_block(MU_M1, &MOBLK)`), then fills the mouse position/state.
`ev_mouse` overwrites `rets[2]` with the **current** `button` after the wait,
so the reported button is always live (`gemevlib.c:127`).

> **The wait is `amouse`'s, and there is no pre-drain (A90/A92,
> 2026-09-21).** `ev_block(MU_M1, pmo)` is `mwait(iasync(MU_M1, pmo))`
> (`gemevlib.c:64-69`) → `amouse` (`aes/gemasync.c:157-162` →
> `aes/geminput.c:568-587`): the rectangle is tested **once, at arm time**,
> ownership-blind (`in_mrect` → `azombie`, `:575-576`), and otherwise parked on
> the caller's `c_msleep`, which only `post_mouse(gl_mowner, …)` walks
> (`:446-456`) — the same rule as `evnt_multi`'s `MU_M1`/`MU_M2` arm (point 6
> note above). Unlike `ev_multi`, `ev_mouse` runs **no** `chkkbd(); forker();`
> before arming (`:123-128` vs `:169-170`): its arm-time position is the one
> the previous dispatch left (`super()` → `dsptch()`, `gemsuper.c:452-464`);
> a packet queued since is processed inside the wait (`mwait` → `dsptch` →
> `forker`) and delivered under ownership. So a rectangle crossed while the
> Control Manager holds the mouse — e.g. during `ct_msgup`'s wait for the
> selecting press to be released (`aes/gemctrl.c:100-101`) — is delivered only
> once ownership returns (`w_setactive` → `set_mown(app)` → `post_mouse(app,
> xrat, yrat)`, `:441-442`, `aes/geminput.c:225-231`), against the position
> at that moment.

### `evnt_mesag` (23)

**Source:** `gemsuper.c:135-141` → `ev_mesag((WORD *)ME_PBUFF)`
(`gemevlib.c:100-117`).

| Slot | Macro | Meaning |
|------|-------|---------|
| `addr_in[0]` | `ME_PBUFF` | pointer to the 16-byte message buffer to fill |
| `int_out[0]` | `RET_CODE` | return: 1 (`void` handler) |

Sets `rlr->p_flags |= AP_MESAG` (the arm, `gemsuper.c:137`), then blocks until a
16-byte message is available and copies it into the caller's buffer. Two
sources feed it:

- The **normal pipe** (`ap_rdwr(MU_MESAG, rlr, 16, mebuff)`, `gemevlib.c:116`) —
  the same 16-byte queue that `appl_write`/`appl_read` use (see
  [`application.md`](./application.md), [`_state.md`](./_state.md) §3).
- A **control-manager fast-path** `WM_ARROWED`: when the pipe is empty
  (`p_qindex == 0`) and a pending scroll message is set (`p_msg.action >= 0`,
  posted by `gemctrl.c:129-133` while an arrow gadget is held), `ev_mesag`
  synthesizes a full `WM_ARROWED` message in place (`msg[0]=WM_ARROWED`,
  `msg[1]=p_pid`, `msg[3]=p_msg.wh`, `msg[4]=p_msg.action`) and clears
  `p_msg.action = -1` (`gemevlib.c:102-114`). **`p_pid` here is the reader's
  own pid — 0 for the application chain, ≥ 2 for an accessory** — unlike the normal *queued*
  `WM_ARROWED`, which arrives via `ap_sendmsg` from the Screen Manager with
  `msg[1] = 1` (`gemctrl.c:125`; see [`application.md`](./application.md)
  §"AES process identity and pid", 2026-09-20/A62+A56).

After the arm returns, if `msg[0] == AC_CLOSE` the dispatcher sets
`rlr->p_flags |= AP_ACCLOSE` (`gemsuper.c:139-140`) — the desk-accessory close
handshake `appl_exit` waits on.

### `evnt_timer` (24)

**Source:** `gemsuper.c:142-144` → `ev_timer(MAKE_ULONG(T_HICOUNT, T_LOCOUNT))`
(`gemevlib.c:134-137`).

| Slot | Macro | Meaning |
|------|-------|---------|
| `int_in[0]` | `T_LOCOUNT` | delay, low WORD (milliseconds) |
| `int_in[1]` | `T_HICOUNT` | delay, high WORD |
| `int_out[0]` | `RET_CODE` | return: 1 (`void` handler) |

Waits the given number of **milliseconds** (a 32-bit count assembled hi:lo).
Internally the ms count is divided by `gl_ticktime` (ms per system tick,
`geminit.c:704`) to get a tick count for `ev_block(MU_TIMER, …)`. **A count of 0 ms does
not return immediately** (corrected A112, 2026-10-17): `adelay` raises a tick
count of 0 to 1 (`if (c == 0L) c = 1L`, `geminput.c:500-501`), so
`evnt_timer(0)` — like any count below one `gl_ticktime` — arms a real
one-tick delay and sleeps until the next tick completes it
(`gemflag.c:32-68`). Only `evnt_multi` with `MU_TIMER` and `tmcount == 0`
exactly is a poll (quick check, `gemevlib.c:216-220`). See "Wake conditions
and dispatch", point 6.

### `evnt_dclick` (26)

**Source:** `gemsuper.c:163-165` → `ev_dclick(EV_DCRATE, EV_DCSETIT)`
(`gemevlib.c:300-309`).

| Slot | Macro | Meaning |
|------|-------|---------|
| `int_in[0]` | `EV_DCRATE` | new speed index **0..4** (used only when setting) |
| `int_in[1]` | `EV_DCSETIT` | 0 = **get** current speed, non-zero = **set** to `EV_DCRATE` |
| `int_out[0]` | `RET_CODE` | **return: the (new or current) speed index** `gl_dcindex` |

Get or set the double-click speed. When `EV_DCSETIT != 0` it stores
`gl_dcindex = rate` and recomputes the click-window `gl_dclick =
gl_dcrates[rate] / gl_ticktime` (ticks). It **always** returns the current
`gl_dcindex`, so `evnt_dclick(x, 0)` reads the setting without changing it.

**The rate table** `gl_dcrates[5] = {450, 330, 275, 220, 165}`
(`gemevlib.c:33`) — the click-window in **milliseconds** for indices 0
(slowest) … 4 (fastest). The AES initialises the speed to **index 3** (220 ms)
at start-up (`ev_dclick(3, TRUE)`, `geminit.c:708`).

> **Convention note.** Compendium documents `evnt_dclick(rate, setit)` with
> `rate = -1` sometimes cited for "query"; in **this EmuTOS build the query is
> selected by `setit == 0`**, and the `rate` argument is ignored when querying.
> A caller wanting the current speed passes `setit = 0` (any `rate`).

---

## Trap interface (parameter-block form)

All `evnt_*` calls enter via the GEM trap (`svc #2`) with **selector 200 in
`x0`** and an **AESPB pointer in `x1`** — see [`_dispatcher.md`](./_dispatcher.md).
Per call the dispatcher reads `control[0]` = opcode, `control[1..3]` = #int_in /
#int_out / #addr_in, and returns via `int_out[0]` = `RET_CODE`.

| Opcode | Function | `int_in` | `addr_in` | `int_out[0]` return | Also fills |
|-------:|----------|----------|-----------|---------------------|-----------|
| 20 | `evnt_keybd`  | — | — | key code (scancode<<8 \| ascii) | — |
| 21 | `evnt_button` | `[0]`=clicks(+flag), `[1]`=mask, `[2]`=state | — | actual click count | `int_out[1..4]` mouse x/y/btn/shift |
| 22 | `evnt_mouse`  | `[0]`=m_out, `[1..4]`=GRECT | — | 1 | `int_out[1..4]` mouse x/y/btn/shift |
| 23 | `evnt_mesag`  | — | `[0]`=16-byte buffer | 1 | buffer ← message; `AP_MESAG`/`AP_ACCLOSE` |
| 24 | `evnt_timer`  | `[0]`=lo ms, `[1]`=hi ms | — | 1 | — |
| 25 | `evnt_multi`  | `[0]`=flags, `[1..3]`=button, `[4..8]`=rect1, `[9..13]`=rect2, `[14..15]`=timer | `[0]`=16-byte buffer | which events fired (`MU_*` mask) | `int_out[1..6]` mouse/key/button block |
| 26 | `evnt_dclick` | `[0]`=rate(0..4), `[1]`=setit | — | current/new speed index | — |

---

## Inputs / Outputs summary

Inputs arrive in the AESPB arrays (no register/stack params). Every wait that
reports mouse/keyboard state shares the **`int_out[1..6]` return block**:
`[1]`=mouse x (`EV_MX`), `[2]`=mouse y (`EV_MY`), `[3]`=button state (`EV_MB`),
`[4]`=shift state (`EV_KS`), `[5]`=key code (`EV_KRET`), `[6]`=button click
count (`EV_BRET`). The single-event waits fill only `[1..4]` (via `&EV_MX`);
`evnt_multi` also fills `[5]`/`[6]` when the corresponding event fired.
`int_out[0]` (`RET_CODE`) is the per-call return (event mask for `evnt_multi`,
key code for `evnt_keybd`, click count for `evnt_button`, speed index for
`evnt_dclick`, `1` for the `void` waits `evnt_mouse`/`evnt_mesag`/`evnt_timer`).

---

## Error conditions

The event manager has **no negative-error surface** — there are no `E*` TOS
error codes and no documented failure returns. Every `evnt_*` call is a
successful wait/query:

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| `evnt_keybd` — always returns a key code | `gemsuper.c:127` | key code (never negative) |
| `evnt_button` — returns click count (may be 0 if the delay window closes) | `gemevlib.c:84-94` | click count ≥ 0 |
| `evnt_multi` — returns the fired-event mask (never 0 in practice: it blocks until something fires, or a `tmcount == 0` timer completes it in the quick check) | `gemevlib.c:293` | `MU_*` mask |
| `void` waits (`evnt_mouse`/`evnt_mesag`/`evnt_timer`) | `gemsuper.c:85` | 1 (`crysbind` default `ret`) |
| any opcode not compiled (n/a here — all 20–26 present) | `gemsuper.c:406-417` | -1 |

All seven event opcodes (20–26) have a dispatch `case`, so none of them hit the
dispatcher's unsupported-opcode → `-1` path (see [`_dispatcher.md`](./_dispatcher.md)).

---

## Verification criteria

- **`evnt_multi(MU_KEYBD)` key:** with `int_in[0] = MU_KEYBD` and a key ready,
  the call returns `MU_KEYBD` in `int_out[0]` and the packed key code
  (scancode high byte, ASCII low byte) in `int_out[5]` (`EV_KRET`)
  (`gemevlib.c:179-181,275`).
- **`evnt_keybd` return slot:** `evnt_keybd` returns the key code in
  **`int_out[0]`** (not `int_out[5]`); high byte = scancode, low byte = ASCII
  (`gemsuper.c:127`, `gemgsxif.c:459-471`).
- **`evnt_multi` mouse block:** on any fired event, `int_out[1]`/`int_out[2]`
  hold the current (or transition-time) mouse x/y, `int_out[3]` the button
  state, `int_out[4]` the shift state (`gemevlib.c:42-58,268`).
- **Click reported with the button down:** `evnt_multi(MU_BUTTON, 0x0102, 3,
  0)` (or the desktop's `2, 1, 1`) with one press + release inside the click
  window returns `MU_BUTTON`, `EV_BRET = 1`, **`EV_MB = 1`** and the click
  position; an immediately following 0-ms poll returns `MU_TIMER` only and the
  live `EV_MB = 0` (`gemevlib.c:42-58`, `geminput.c:155-171,329-345`).
- **Click between two calls:** press + release published while no wait is armed;
  the next `evnt_multi(MU_BUTTON|MU_TIMER, 0x0102, 3, 0)` with a 0-ms timer
  reports `MU_BUTTON` with `EV_BRET = pr_mclick = 1` and `EV_MB = 1`
  (`gemevlib.c:190-194`).
- **Double click across a timer poll (ZVIEW02):** a loop of
  `evnt_multi(MU_MESAG|MU_TIMER|MU_KEYBD|MU_BUTTON, 0x0102, 3, 0, 20 ms)`
  over press 15 ms / release 25 ms / press 15 ms returns `MU_TIMER` alone with
  `EV_MB = 0` while the window is open, then once `MU_BUTTON` with
  `EV_BRET = 2`, `EV_MB = 1`; with the second press held 360 ms the same
  return comes while the button is still down (`geminput.c:110-171,463-483`).
- **Window extension (A111):** a 3-click wait (`0x0003, 1, 1`) with presses
  at t0, t0+100 ms and t0+300 ms returns `EV_BRET = 2` at t0+260..280 ms —
  11 + 3 ticks of 20 ms (`geminput.c:121-122`, `gemevlib.c:300-309`).
- **`evnt_button` double-click:** with `int_in[0] = 2` (clicks=2, enter),
  `int_in[1] = 1` (mask=left), `int_in[2] = 1` (state=down), two presses inside
  the current click window (`gl_dcrates[gl_dcindex]` ms) return `2` in
  `int_out[0]` when the window closes; a second press *after* the window
  returns `1`
  (`geminput.c:114-147,340-357`).
- **`evnt_mouse` enter vs leave:** `int_in[0]=0` over a rect fires when the
  cursor moves **inside** it; `int_in[0]=1` fires when it moves **outside** —
  `in_mrect` = `m_out != inside(x,y,rect)` (`geminput.c:76-79`).
- **`evnt_timer(0)` is one tick, not a poll** (corrected A112): it arms a
  one-tick delay and returns after the next tick (`gemevlib.c:136`,
  `geminput.c:500-501`). `evnt_multi` with `MU_TIMER` and `tmcount == 0` fires
  `MU_TIMER` from the quick check without blocking (`gemevlib.c:216-220`), but
  the trap still ends in `dsptch()` (`gemsuper.c:458-462`).
- **Wake independent of the running process:** a process blocked in
  `evnt_multi(MU_TIMER)` or `evnt_multi(MU_MESAG)` while another AES process
  runs (and calls the AES) is woken by its tick expiry / by the message another
  process wrote, not by input (`gemflag.c:32-68`, `gemqueue.c:104-118`,
  `gemasync.c:35-61`); `MU_KEYBD`/`MU_BUTTON`/`MU_M1/M2` only while it owns the
  mouse (`geminput.c:288-308,446-483`).
- **`evnt_mesag` delivery + AC_CLOSE:** after a message is delivered, the
  16-byte buffer holds it and `AP_MESAG` is set; if `msg[0] == AC_CLOSE`,
  `AP_ACCLOSE` is also set (`gemsuper.c:137-140`).
- **`evnt_dclick` get:** `evnt_dclick(rate, 0)` returns the current
  `gl_dcindex` unchanged (default **3** after boot); `evnt_dclick(1, 1)` sets
  index 1 (330 ms window) and returns `1` (`gemevlib.c:300-309`).

---

## Edge cases discovered in source

1. **`evnt_keybd` returns the key in `int_out[0]`, not `int_out[5]`.**
   The single-event keyboard wait puts the code in `RET_CODE`; only
   `evnt_multi` uses the `EV_KRET` (`int_out[5]`) slot (`gemsuper.c:127` vs
   `gemevlib.c:179`). Programs must read the right slot per call.
2. **A 0-count timer is a poll only in `evnt_multi`, and even then it yields.**
   *(Corrected A112, 2026-10-17: the earlier text said `evnt_timer(0)` and
   `evnt_multi` with a 0-ms `MU_TIMER` "fire immediately without yielding";
   both halves were wrong.)* `evnt_multi` with `tmcount == 0` completes
   `MU_TIMER` in its quick check and does not block (`gemevlib.c:216-220`) —
   the idiom for "check for events now" — but every AES call, this one
   included, ends in `dsptch()` (`gemsuper.c:458-462`), so other ready
   processes run before the caller resumes. `evnt_timer(0)` is **not** a poll:
   `ev_timer` → `ev_block(MU_TIMER, 0)` → `adelay` raises 0 to 1
   (`gemevlib.c:134-137`, `geminput.c:500-501`) and the caller sleeps until the
   next tick. A non-zero `evnt_multi` count under one `gl_ticktime` is likewise
   armed as one tick (`gemevlib.c:259`).
3. **`MU_M3` (0x100) cannot be requested by an application.** The dispatcher
   masks `MU_FLAGS & MU_TOSVALID` (0x3F) before calling `ev_multi`, so the
   third rectangle is internal to the menu-extension code only
   (`gemsuper.c:153`, `aesdefs.h:262-263`).
4. **The enter/leave flag for buttons lives in the high bit of the clicks
   word.** `combine_cms` puts clicks in the high WORD of the packed LONG, and
   `downorup` reads the enter/leave `flag` from bit 24 (the top byte of the
   clicks word) — so `int_in[0]` / `MB_CLICKS` with its top bit set means "wait
   to LEAVE the state" (`gemevlib.h:34-50`, `geminput.c:348-357`).
5. **`evnt_mouse` reports the *live* button, not the snapshot.** After the rect
   wait it overwrites `rets[2]` with the current `button`, unlike the other
   waits that may report the transition-time `pr_button` (`gemevlib.c:127`).
6. **The click count can be 0 or grow beyond the request.** `ev_button`
   documents that it returns *how many times the state was actually reached*
   within the delay window — this may be fewer than requested (window closed) or
   the accumulated `mclick`/`gl_bclick` count (`gemevlib.c:72-94`,
   `geminput.c:114-147`).
7. **`evnt_mesag` has a queue-bypass fast path.** A held-down window arrow
   posts `p_msg.action`/`p_msg.wh` directly on the process descriptor; when the
   real pipe is empty, `ev_mesag` synthesizes the `WM_ARROWED` message from
   those fields instead of reading the queue (`gemevlib.c:100-114`,
   `gemctrl.c:129-133`) — auto-repeat scrolling without flooding the 128-byte
   queue. The synthesized message's `msg[1]` is the reader's own pid (0),
   whereas the normal queued `WM_ARROWED` carries `msg[1] = 1` (Screen
   Manager) — see [`application.md`](./application.md) §"AES process
   identity and pid" (2026-09-20/A62+A56).
8. **`evnt_multi` pre-drains the fork queue.** It calls `chkkbd()` + `forker()`
   before the quick checks so a keystroke or button/mouse change that arrived
   just before the call is seen on this pass, not the next (`gemevlib.c:169-170`).
9. **The double-click *speed index* is the return, not the ms value.**
   `evnt_dclick` returns `gl_dcindex` (0..4), never the `gl_dcrates[]`
   millisecond figure (`gemevlib.c:308`).
10. **Multiple bits can be set in the `evnt_multi` return.** If several armed
    events are already satisfied on entry (e.g. a message *and* a matching
    button), the quick-check phase can OR several `MU_*` bits into `RET_CODE`
    (`gemevlib.c:165-229`).

---

## Dependencies

### Requires (must be initialized / functional)

- The **AES scheduler** primitives (`ev_block`/`iasync`/`mwait`/`apret`/
  `acancel`/`forker`) and the run lists (`rlr`, `gl_mowner`) — the cooperative
  multitasking core (`gemevlib.c`, `gemasync.c`, `gemdisp.c`;
  [`_state.md`](./_state.md)). **Reference** — NewTOS reimplements the wait on
  its own process model.
- The **message pipe** (`ap_rdwr`/`gemqueue.c`) for `evnt_mesag`/`evnt_multi`
  `MU_MESAG` — the same 16-byte queue the application manager fills with
  `appl_write` (see [`application.md`](./application.md)).
- **Mouse/keyboard input feeding the AES state** (`xrat`/`yrat`/`button`/
  `kstate`/`mclick`/`gl_mowner`, plus the `bchange`/`mchange`/`kchange` fork
  handlers). In EmuTOS this comes from the IKBD via the VDI `user_but`/
  `user_mot` vectors and `gsx_char`/`gsx_kstate`; in NewTOS from USB-HID/IKBD
  (see the VDI mouse cross-reference below).
- **`gl_ticktime`** (ms per system tick, `geminit.c:704`) for the timer and
  double-click conversions, and the **system tick** driving `b_delay` (Phase-07
  timer / `Vsync`/`CNTPCT` clock).

### Used by

- **Every GEM application's main loop** is `evnt_multi` (or one of the
  single-event waits) plus the window/menu message handlers.
- The **desktop** (`desk/`) and the AES itself call `evnt_multi` to drive
  redraws, menu selection, and window control.
- `evnt_dclick` is called by control panels / the desktop preferences to expose
  the double-click speed slider.

---

## Related data structures

- **`MU_*` event-flag table** — values above (`include/aesdefs.h:256-263`);
  contract.
- **`MOBLK` / `GRECT` rectangle** — the 5-WORD wire layout above
  (`aes/gemlib.h:28-32`, `include/obdefs.h:155-162`).
- **`evnt_multi` slot map** — the `int_in[0..15]` + `addr_in[0]` +
  `int_out[0..6]` tables above (`aesdefs.h:178-224`).
- **The 16-byte standard message** — `WORD msg[8]`; `msg[0]` = message id
  (`MN_SELECTED` 10, `WM_*` 20–31, `AC_OPEN` 40 / `AC_CLOSE` 41,
  `aesdefs.h:227-243`), `msg[1]` = sender ap_id (**0** = application chain,
  **1** = Screen-Manager sender stamp only — see
  [`application.md`](./application.md) §"AES process identity and pid"),
  `msg[2]` = extra-byte count, `msg[3..7]` message-specific. Shared with the
  application manager ([`application.md`](./application.md),
  [`_state.md`](./_state.md) §3).
- **The double-click rate table** — `gl_dcrates[5] = {450, 330, 275, 220, 165}`
  ms, indices 0..4 (`gemevlib.c:33`); default index 3.
- **`combine_cms` / `downorup` button packing** — clicks:WORD ∥ mask:byte ∥
  state:byte, enter/leave in the clicks-word high bit (`gemevlib.h:34-50`,
  `geminput.c:348-357`); reference-level, but the `int_in` slot values are
  contract.
- **`EVB`/`QPB`/`CQUEUE` event blocks + `p_cda`/`p_msg`** — cooperative
  scheduler internals (`aes/struct.h`); **reference** — see
  [`_state.md`](./_state.md).

---

## Cross-references

### Atari Compendium / Profibuch

- **Not present in `references/`** (no PDFs). Cross-check of the `evnt_multi`
  `int_in`/`int_out` slot map, the `MU_*` bit values, the `MOBLK` enter/leave
  semantics, the `evnt_dclick` speed range (0..4) and rate table, and the
  `EV_KRET` scancode|ascii encoding is **deferred** until the documentation PDFs
  are available. (Notably, Compendium sometimes documents `evnt_dclick`'s query
  as `rate = -1`; this build queries via `setit == 0` — see the convention note
  and D-050.)

### AES managers

- [`application.md`](./application.md) — the message pipe (`appl_write` posts
  the messages `evnt_mesag`/`evnt_multi` deliver; both share `ap_rdwr` and the
  16-byte format); `AP_MESAG`/`AP_ACCLOSE` flags.
- [`_state.md`](./_state.md) — the shared identity/queue model, `p_flags`, the
  16-byte message, the mouse/keyboard state globals.
- [`_dispatcher.md`](./_dispatcher.md) — selector-200 call model, `control[]`
  counts, `int_out[0]` return, the unsupported-opcode `-1` path.

### VDI

- [`../vdi/input/_mouse.md`](../vdi/input/_mouse.md) — the mouse position/button
  source. The AES installs `user_but`/`user_mot` via VDI `vex_butv`/`vex_motv`
  to drive `evnt_button`/`evnt_mouse`; `vq_mouse` and the AES `xrat`/`yrat`/
  `button` share the same IKBD/HID packet path. (Do not edit the VDI docs.)

---

## Configuration / version notes

- **`CONF_WITH_MENU_EXTENSION`** (default **1**, `include/config.h:1305`) adds a
  third mouse rectangle (`MU_M3`, a `pmo3` argument to `ev_multi`). It is
  **internal**: `MU_M3` (0x100) is masked out of the application-supplied flags
  by `MU_TOSVALID`, so `evnt_multi`'s ABI is unchanged for callers regardless of
  this flag (`gemsuper.c:152-159`). With the flag off, `ev_multi` takes two
  rectangles instead of three; the contract slot map (M1 at `int_in[4..8]`, M2
  at `int_in[9..13]`) is identical either way.
- The **default double-click speed is index 3** (220 ms window),
  `ev_dclick(3, TRUE)` at AES start-up (`geminit.c:708`). This is a
  boot-time default, not a compile option.
- `gl_ticktime` (ms per tick) is read from the VDI at init (`gsx_tick`,
  `geminit.c:704`) and used to convert the ms-based timer/dclick values into
  tick counts — a **reference** detail (NewTOS uses its own tick source; the
  observable contract is that timers and click windows are specified in ms).
- No TOS-version conditionals affect the `evnt_*` opcode set: all seven
  (20–26) are present in every EmuTOS build.

---

## NewTOS implementation notes

(Empty during Phase 00.)

- **`evnt_multi` is the EC1-critical call.** A window demo's event loop is
  `evnt_multi` + the window/menu message handlers; bring-up can wire a single
  application plus the timer/keyboard/mouse event sources and add cooperative
  AES multitasking (DAs) later without changing the `evnt_*` ABI.
- **Event sources map to NewTOS input.** Keyboard/mouse come from USB-HID / the
  native IKBD ([[keyboard-via-atari-ikbd]] decoder feeds key events; the mouse
  packet feeds `evnt_mouse`/`evnt_button`); the timer from the frame/`CNTPCT`
  clock (cf. XBIOS `Vsync`). Messages come from the per-process 16-byte pipe
  shared with `appl_write` (see [`application.md`](./application.md)).
- **The wait mechanics are free; the flag model + arrays are contract — and so
  is the wake set** (A112): a blocked `evnt_multi` must be woken by *each* armed
  event's own source (timer expiry, a message another process wrote, owner-gated
  input), not by one shared input key, and each AES call must yield afterwards.
  See "Wake conditions and dispatch".
  `ev_block`/`iasync`/`mwait`/`apret`/`acancel` and the fork queue are EmuTOS
  cooperative-scheduler internals — NewTOS parks the calling process on its own
  scheduler. What must match exactly: the `MU_*` bit values, the `int_in`/
  `int_out`/`addr_in` slot layout (especially the `evnt_multi` map and the
  `int_out[1..6]` block), the 16-byte message format, the double-click speed
  range/table and its ms semantics, the enter/leave rectangle rule, and the
  `EV_KRET` scancode|ascii encoding.
- **Pointers in `addr_in[0]` (message buffer) and the cast `MOBLK`/`GRECT`
  slices are user (application) pointers** — apply the same boundary handling as
  other AES `addr*`/parameter reads (see [`_dispatcher.md`](./_dispatcher.md)
  §NewTOS notes). The m68k-vs-native width follows the caller ABI; the WORD
  slot arrays are identical in both worlds.
