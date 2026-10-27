# `Super` (GEMDOS 0x20)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bdos/rwa.S:159-346` (`_enter`, `x20_usr`, `x20_sup`,
> `x20_inq`). Written 2026-07-09 following the dispatcher-convention
> re-validation pass (`_dispatcher.md` §2/§6, D-061).

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../../COMPATIBILITY.md).
> **Contract:** trap ID/ABI (single LONG argument), the **toggle-relative-
> to-current-mode** semantics, the three sub-cases (`stack==0`, `stack==1`
> inquiry, explicit address) per calling mode, and the inquiry return values
> (`0`/`-1`). **Reference (NewTOS reimplements freely, and may not be able to
> reimplement at all — see NewTOS notes):** the m68k USP/SSP register
> juggling, the `fix_rte`/exception-frame mechanics, the D0-asymmetry quirk
> in the supervisor→user direction (a byproduct of m68k register scratch
> use, not a documented return value).

## EmuTOS source

**Primary location:** `bdos/rwa.S:159` (`_enter`) — `Super` is **not** a
`funcs[]`/`osif()` entry. It is intercepted directly in the trap-1 assembly
trampoline, before the C dispatcher is ever called
(`docs/api/gemdos/_dispatcher.md` §2, §6). The three sub-handlers are
`bdos/rwa.S:280` (`x20_usr`), `bdos/rwa.S:305` (`x20_sup`), and
`bdos/rwa.S:340` (`x20_inq`, shared by both entry directions).

**Installed handler / vector:** reached via the same `Setexc(0x21, enter)`
install as every other GEMDOS call (`bdos/bdosmain.c:339`,
`docs/api/gemdos/_dispatcher.md` §1) — there is no separate vector for
`Super`; the distinction is made by `_enter` peeking at the caller's
function-ID word before doing anything else.

**Helpers called:** none — the entire implementation is inline assembly;
`ni()`/`osif()`/`funcs[]` are never reached for function ID `0x20`.

**Headers / type definitions used:** none (no C struct is involved; the
"stack" argument is a raw address value).

**System variables / globals read/written:** none of BDOS's own state
(`run`, `pmd`/`pmdalt`, `sft[]`, …). `Super` only manipulates CPU registers
(`SR`, `USP`, `SSP`) — it is the one GEMDOS call that changes **no** BDOS
data structure.

**Build configuration affecting this function:** none — present in every
EmuTOS build (the `#ifdef __mcoldfire__` branches only change which
instruction sequence performs the identical peek/pop, not the behaviour).

---

## Source extraction summary

`Super` toggles the CPU between user and supervisor mode; it does **not**
mean "switch to supervisor" — calling it while already in supervisor mode
switches **back** to user. `_enter` decides which of the two directions
applies by testing the S-bit of the exception frame it just received
(`btst.b #5,(sp)`, `bdos/rwa.S:162`) *before* looking at the function ID, so
the two directions are two genuinely different code paths (`x20_usr` vs.
`x20_sup`), not one shared routine.

The single argument (`stack`, a LONG) has three meanings, identical in both
directions:

- **`stack == 1`** — **inquiry only**: no mode change, no stack change.
  Falls straight through to `x20_inq` (`bdos/rwa.S:340-346`), which reads
  the *original* exception frame's S-bit and returns `0` (was user) or `-1`
  (was supervisor) in D0.
- **`stack == 0`** — switch **using a stack pointer value the trampoline
  derives from the current context** rather than one the caller supplies
  (see per-direction detail below).
- **any other value** — treated as an explicit new supervisor-/user-stack
  address supplied by the caller.

In **both** non-inquiry cases, the direction (user→super or super→user) is
decided purely by which mode the caller was in when it trapped — the
`stack` argument only ever selects *which address becomes the new stack
pointer*, never *which mode to end up in*.

**User → supervisor (`x20_usr`, `bdos/rwa.S:280-301`):** unconditionally
sets the SR's supervisor bit (`ori.w #0x2000,d0`, `:295`) before the
eventual `rte`. For `stack == 0` the new supervisor stack is the address
the CPU's SSP register already held going into this trap (an always-valid,
reusable value — on m68k *every* exception, including this one, is taken
onto the current SSP register regardless of any software convention); for
an explicit `stack`, that address becomes the new SSP. **D0 is set
explicitly** to the *previous* value of that stack pointer
(`move.l sp,d0 // set return value (old SSP)`, `:298`, comment in source) —
this is the classic `old = Super(0L); … ; Super(old);` round-trip idiom.

**Supervisor → user (`x20_sup`, `bdos/rwa.S:305-336`):** unconditionally
clears the SR's supervisor bit (`bclr.b #5,(sp) // set user mode`, `:334`)
before the `rte`. For `stack == 0` the new USP is reconstructed as
`(original sp) + 6` (68000-specific short-frame arithmetic, `:329-330`);
for an explicit `stack`, that address is used both as the new USP source
bookkeeping *and* pre-loaded with a ready-to-`rte` return frame at the
supplied address, so a **later** `Super()` call passing that same address
back (from user mode) resumes exactly here. **D0 is never explicitly set
in this direction** (see Edge case 1 — a genuine, source-verified
asymmetry, not an omission in this spec).

---

## Trap interface

- **Trap number:** 1 (GEMDOS)
- **Function ID:** 0x20
- **Stack frame on entry (m68k):**
  ```
  +2: stack (LONG) — 0 = derived stack, 1 = inquiry, else = explicit address
   0: function_id (word) = 0x0020
  ```
- **Total parameter bytes:** 4 (LONG). **Not** expressed as a `funcs[]`
  `wparms` entry — the argument layout is hard-coded in `bdos/rwa.S`, not
  looked up from a table (`docs/api/gemdos/_dispatcher.md` §6).
- **Native (AArch64) ABI:** **open** — see NewTOS implementation notes.
  On m68k: ID `0x20` in the caller's stack-frame function-ID word; `stack`
  as the LONG argument; result (when the call returns to the caller at all —
  see Edge case 4) in D0.

  | m68k stack | Name    | Type / size |
  |------------|---------|-------------|
  | +2         | `stack` | `LONG` (32) — `0`/`1`/explicit address |

---

## Inputs

| Position | Name    | Type   | Size | Source location | Notes |
|----------|---------|--------|------|------------------|-------|
| arg0     | `stack` | `LONG` | 4    | `bdos/rwa.S:281` (user entry), `:306` (supervisor entry) | `0` = derived stack pointer; `1` = inquiry (no switch); other = explicit new stack address |

`stack` is read directly off the caller's own stack by the trampoline (via
`a0` = USP for the user-mode entry, or stack-relative addressing for the
supervisor-mode entry) — **not** through the `pw[]` copy every other
GEMDOS call goes through, because `Super` never reaches `osif()`.

---

## Outputs

### Return value

- **Register:** D0 (m68k)
- **Type:** LONG
- **Encoding — four distinct, source-verified cases:**

  | Calling mode | `stack` | Result | Source |
  |--------------|--------:|--------|--------|
  | user | `1` | `0` (was user) in D0; no mode change | `bdos/rwa.S:340-346` |
  | user | `0` or explicit | switches to supervisor; D0 = the **previous** stack-pointer value (explicitly assigned) | `bdos/rwa.S:298` |
  | supervisor | `1` | `-1` (was supervisor) in D0; no mode change | `bdos/rwa.S:340-346` |
  | supervisor | `0` or explicit | switches to user; **D0 is not assigned by this path** — see Edge case 1 | `bdos/rwa.S:305-336` (no `move.l …,d0` anywhere in this branch) |

### Side effects on global state

None on BDOS state (`run`, pools, `sft[]`, `dirtbl[]` are untouched). The
side effect **is** the CPU mode/stack-pointer change itself: SR S-bit,
USP, and (for the user→super direction) the active SSP.

### Side effects on hardware

None beyond the CPU-internal privilege/stack-pointer registers already
described — `Super` touches no MMIO, no device.

---

## Error conditions

`Super` has **no error return** — every input value (0, 1, or any other
LONG) is accepted and produces one of the four cases above. An
out-of-range or garbage `stack` value used as an explicit address is not
validated; it becomes the new stack pointer verbatim, and a bad address
only faults later, when something is actually pushed to or popped from it.

**Error paths not in Compendium:** none — Compendium documents the same
three-way `stack` convention with no error return.

---

## Verification criteria

- **Inquiry, user mode:** calling `Super(1)` while in user mode returns `0`
  and performs no mode/stack change.
- **Inquiry, supervisor mode:** calling `Super(1)` while in supervisor mode
  returns `-1` and performs no mode/stack change.
- **User → supervisor, `stack==0`:** from user mode, `Super(0)` switches to
  supervisor mode; the returned D0 is a valid stack-pointer value that, when
  passed back to a later `Super(that_value)` call made from supervisor
  mode, returns to user mode.
- **User → supervisor, explicit address:** from user mode, `Super(addr)`
  with a valid `addr` switches to supervisor mode using `addr` as the new
  supervisor stack.
- **Supervisor → user round trip:** `old = Super(0)` (from user mode)
  followed later by `Super(old)` (from supervisor mode) returns the CPU to
  user mode at the point after the second call.
- **No error path:** no input value to `Super` produces a negative
  GEMDOS error code.

---

## Edge cases discovered in source

1. **D0 asymmetry between directions (source-verified quirk, not a
   documentation gap).** The user→supervisor path explicitly sets D0 to the
   old stack-pointer value (commented `// set return value (old SSP)` in the
   source, `:298`). The supervisor→user path has **no equivalent
   instruction** anywhere in `x20_sup`/`xs_0`/`xs_usp`/`xs_exit`/`xs_scram`
   (`bdos/rwa.S:305-336`) — D0 is left holding whatever a preceding scratch
   use left in it: for `stack==0` (`xs_0`, `:328-330`) D0 is **not touched at
   all** in this branch (the caller's own pre-call D0 survives unchanged);
   for the general branch, only the low 16 bits get overwritten as a
   byproduct of popping the SR word into D0 as scratch (`move.w (sp)+,d0`,
   `:313`) before that same value is written on to the new stack frame
   (`:315`) — the high word is whatever D0 held before the trap. **A program
   calling `Super()` to leave supervisor mode gets no meaningful return
   value in D0** — this matches the classic idiom, where only the
   *entering* call's return value (`old_ssp = Super(0)`) is ever used; the
   *restoring* call's return value is conventionally discarded.

2. **Toggle, not "set".** `Super`'s destination mode is always "the
   opposite of whatever mode the caller was in", never selectable by the
   `stack` argument — the argument only ever picks the stack pointer to
   use for the destination mode. A caller cannot use `Super` to
   idempotently "ensure supervisor mode"; it must track its own state (or
   use the inquiry form first).

3. **Bypasses the entire normal dispatcher.** No standard-handle
   redirection, no special-name interception, no hard-error `setjmp`
   recovery, no `funcs[]`/`wparms` lookup — none of `_dispatcher.md` §4-§8
   applies to `Super`. It also bypasses the `P_REGSAVE`/`P_A7SAVE` bookkeeping
   `enter_2` performs for every ordinary call, so the basepage's saved
   register area is **not** touched by a `Super` call.

4. **Register scratch differs from the ordinary GEMDOS convention.**
   `_dispatcher.md` §9 states the general rule "D1, D2, A0, A1, A2 are not
   preserved" for a returning GEMDOS call (i.e. no guarantee, but nothing
   specifically claims they change). For `Super` specifically, only `D0`,
   `D1`, `A0`, `A1` are ever written by the assembly (`D1`/`A0`/`A1` as
   address/parameter scratch); `D2-D7`/`A2-A6` are never touched by any of
   `x20_usr`/`x20_sup`/`x20_inq`.

5. **68000-specific reconstruction in `xs_0`.** `move.l sp,a0; addq.l
   #6,a0` (`bdos/rwa.S:329-330`) assumes the short (6-byte, no format word)
   68000 exception frame to derive the "as if from user mode" USP value for
   `stack==0` in the supervisor→user direction. The comment marks this
   line as 68000-specific; longer-frame CPUs are handled by the
   `fix_trap`/`fix_rte` normalization used throughout `rwa.S`, not by this
   line itself.

6. **Only function *not* subject to D-061's `EINVFN` convention.** Every
   other unknown/unimplemented GEMDOS function ID returns `EINVFN` (D-061);
   `Super` is the sole function-ID `0x20` and is intercepted before that
   check (`fn > MAX_FNCALL`, `bdos/bdosmain.c:444`) is ever reached, so the
   `EINVFN` convention is simply not applicable to it.

---

## Dependencies

### Requires (must be initialized / functional)

- Nothing BDOS-side — `Super` needs only a valid trap-1 vector install
  (`docs/api/gemdos/_dispatcher.md` §1) and a functioning m68k exception
  frame. It runs before `run`, the memory pools, or any file table are
  consulted.

### Used by

- Any program or library routine that needs to touch supervisor-only state
  (hardware registers, system variables) via the classic
  `old = Super(0L); … ; Super(old);` idiom.
- Conceptually paired with `Supexec` (XBIOS 0x26,
  [`../../xbios/26-supexec.md`](../../xbios/26-supexec.md)) — `Supexec` is
  "run this one routine with supervisor privilege for its duration";
  `Super` is "flip my own privilege bit now, I'll flip it back myself".

---

## Related data structures

- `docs/api/gemdos/_dispatcher.md` §2 ("Detect undocumented function 0x20"),
  §6 ("`Super` (0x20) is not in `funcs[]`"), §10 (source-location summary
  row `Super`/`x20` short-circuit) — the dispatcher-level bypass this spec
  fills in.
- `docs/api/xbios/26-supexec.md` — the sibling "run with supervisor
  privilege" mechanism; shares the open ADR-Supexec blocker with this spec
  (see NewTOS implementation notes).

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. Historically
  documents the same three-way `stack` convention (`0`/`1`/explicit
  address) and the toggle-not-set semantics — consistent with EmuTOS.

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

---

## Configuration / version notes

- Present, byte-identical in behaviour, across every EmuTOS build; the only
  variation is the `__mcoldfire__` instruction selection for the initial
  `0x20` peek, which does not change observable behaviour.

---

## NewTOS implementation notes

> **MEM12 draft (2026-10-27, awaiting approval):** the NewTOS mapping as a
> per-process *super state* that makes page 0 EL0-writable and grants nothing
> else — [`_super-state.md`](../../_super-state.md). It supersedes the deferral below once
> approved.

(Empty during Phase 00.)

**Deferred — grouped with ADR-Supexec.** AArch64 has no m68k-style
in-process supervisor-mode bit that a user program can flip on itself: EL0
(user) and EL1 (kernel) are separate exception levels with no
`SR`-equivalent a userspace instruction can write to raise its own
privilege. `Super`'s entire contract — "flip my own privilege now, using
this stack, and give me back a token that undoes it" — presupposes exactly
the in-process privilege flip that
[`../../xbios/26-supexec.md`](../../xbios/26-supexec.md) already identifies
as the open **ADR-Supexec** question (EL1 callback / pseudo-supervise at
EL0 with a curated MMIO map / kernel-side pattern translation — the three
candidates sketched there). `Super` cannot be resolved independently of
that decision: whichever mechanism ADR-Supexec picks for "run this code
with elevated privilege" determines what, if anything, `Super`'s "stay
elevated until I call it again" variant can mean on NewTOS. See D-062
(`docs/api/_discrepancies.md`).

Until ADR-Supexec lands, `Super` is **not silently stubbed** — like
`Supexec`, it should surface as `EINVFN` only because it has no dispatcher
slot yet (an honest "not wired", not a stub pretending success), never as a
fabricated stack-pointer value.

---

## Pi / BCM2711 binding

> No hardware binding applies — `Super` touches only CPU privilege/stack
> registers, not a peripheral. The relevant re-pointing is architectural
> (m68k SR S-bit / USP / SSP → AArch64 EL0/EL1 + `SP_EL0`/`SP_EL1`), and
> that re-pointing is exactly the open ADR-Supexec question, not a
> Pi-specific detail. See `docs/HARDWARE-PROFILE.md` — no `Super`/`Supexec`
> row exists there yet because the mechanism is undecided; add one when
> ADR-Supexec resolves.
