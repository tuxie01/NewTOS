# `Supexec` (XBIOS 0x26)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bios/vectors.S:1376` (`_supexec`), slot 0x26 of
> `xbios_vecs[]` (`bios/xbios.c:1341`).

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md).
> Contract: trap ID/ABI, the "run the callback in supervisor mode, return its
> D0" behaviour, no argument passing. Reference: the m68k `jmp` tail-call and
> supervisor-stack mechanics — which do **not** map to AArch64 (see notes).

## NewTOS status: **decision deferred** (needs ADR-Supexec)

`Supexec` is present in the table (slot 0x26) with a real handler on EmuTOS,
so it is **not** the dispatcher's func-number fallback. On NewTOS it needs the
**ADR-Supexec** security/ABI decision before it can run a caller's code with
kernel privilege (AArch64 has no in-process privilege flip — see notes).
**Deferred until a program actually needs it.** (The old spec's opening line
"Returns `EINVFN` today (the dispatcher catch-all)" is wrong on two counts:
the dispatcher returns the *function number*, not `EINVFN`, and `Supexec` has
its own table slot so it never hits the catch-all. Corrected here; see D-039.)

## EmuTOS source

**Primary location:** `bios/vectors.S:1376` — `_supexec` (assembly). The
non-debug handler is two instructions: load the code pointer from `4(a7)` and
`jmp` to it. A debug-only C version (`bios/xbios.c:786-806`, `DBG_XBIOS`)
brackets the call with a full register save/restore.

**Installed handler / vector:** (none) — direct table entry.

**Helpers called:** none.

**Headers / type definitions used:** none.

**System variables / globals read/written:** none (the callback may touch
anything).

**Build configuration affecting this function:**
- `DBG_XBIOS` — swaps the bare asm handler for the register-preserving C
  wrapper. Production builds use the bare asm.

---

## Source extraction summary

`Supexec` runs a caller-provided routine in supervisor mode and returns
whatever that routine leaves in D0. The XBIOS trap already entered supervisor
mode, so the production handler simply tail-`jmp`s to the callback
(`movea.l 4(a7),a0; jmp (a0)`) — the callback's terminating `rts` returns
straight through the XBIOS dispatcher's `rte`, which restores the caller's
prior mode.

**Register preservation:** the production (`jmp`) handler does **not** save or
restore any registers — it is a bare tail jump. (Only the `DBG_XBIOS` C
version saves `d2-d7/a2-a6`.) The XBIOS dispatcher's own save area preserves
`d3-d7/a3-a7` across the whole trap, but a Supexec callback that clobbers
`d0-d2/a0-a2` follows normal m68k caller-saved conventions. The callback
receives **no arguments** (`jmp (a0)`); to pass data it uses globals or
PC-relative constants. EmuTOS documents that the callback must not make BIOS
or GEMDOS calls.

---

## Trap interface

- **Trap number:** 14 (XBIOS)
- **Function ID:** 0x26
- **Stack frame on entry (m68k):** `Supexec(codeptr)` = `xbios_l_l(38, a)`
  (`include/xbiosbind.h:56`):
  ```
  +2: codeptr (LONG, function pointer)
   0: function_id (word) = 0x0026
  ```
- **Total parameter bytes:** 6 (L).
- **Native (AArch64) ABI:** ID `0x26` in `x8`; arg in `x0`; the LONG result
  (the callback's return) **sign-extended** into `x0`.

  | m68k stack | AArch64 reg | Name      | Type / size |
  |------------|-------------|-----------|-------------|
  | +2         | `x0`        | `codeptr` | `LONG (*)(void)` (function ptr) |

---

## Inputs

| Position | Name      | Type   | Size | Source location     | Notes |
|----------|-----------|--------|------|---------------------|-------|
| arg0     | `codeptr` | `PFLONG` | 4  | `bios/vectors.S:1377` | routine to run in supervisor mode; called with no args |

---

## Outputs

### Return value

- **Register:** D0 / `x0`
- **Type:** LONG
- **Encoding:** whatever the callback returns in D0.
- **Source location:** the callback's own `rts`; `_supexec` does not modify D0.

### Side effects on global state / hardware

- Whatever the callback does (arbitrary — this is the point of `Supexec`).

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| (no error return at the XBIOS level) | — | — |

`Supexec` itself has no error path. A faulting callback raises a
bus/address/illegal-instruction exception in supervisor mode, handled by the
generic exception handlers.

---

## Verification criteria

- **Runs the callback:** `Supexec(fn)` executes `fn` once and returns its D0.
- **Supervisor privilege:** a callback that reads a supervisor-only location
  (or the m68k SR) succeeds where the same code would fault in user mode.
- **No arguments:** `fn` is entered with no parameters; passing data requires
  globals (a callback expecting args in registers gets whatever the dispatcher
  left).
- **Return propagation:** `fn`'s D0 is the value `Supexec` returns.

---

## Edge cases discovered in source

1. **Production handler is a bare `jmp` — no register save.** `_supexec` does
   `jmp (a0)`, not `jsr` (`bios/vectors.S:1376-1378`); the callback's `rts`
   returns through the dispatcher. Only the `DBG_XBIOS` C version saves
   `d2-d7/a2-a6`. The old spec claimed the convention is "all registers
   preserved by caller" — accurate for the *dispatcher's* `d3-d7/a3-a7`, but
   the handler itself preserves nothing.
2. **No argument passing.** `jmp (a0)` gives the callback no parameters; use
   globals / PC-relative data.
3. **Callback must not call BIOS/GEMDOS.** Documented restriction
   (`bios/xbios.c:769-773`) — re-entering the trap machinery from inside a
   Supexec callback is unsafe.
4. **Supervisor stack is small.** Deep recursion / large locals in the
   callback can overflow the supervisor stack; EmuTOS does not check.

---

## Dependencies

### Requires (must be initialized / functional)

- The XBIOS trap path (already in supervisor mode) and a valid `codeptr`.

### Used by

- Any program peeking/poking hardware, vector tables, MMU state; monitors,
  benchmarks, palette flippers, copy-protection.

---

## Related data structures

- `docs/api/xbios/_dispatcher.md` — the trap entry and the save area.

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Documents the no-args,
  run-in-supervisor-mode convention.)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

---

## Configuration / version notes

- `DBG_XBIOS` swaps in the register-preserving C wrapper; production uses the
  bare asm.

---

## NewTOS implementation notes

> **MEM12 (approved 2026-10-27):** the NewTOS mapping as a
> per-process *super state* that makes page 0 EL0-writable and grants nothing
> else — [`_super-state.md`](../_super-state.md). It supersedes the deferral below once
> implemented.

(Empty during Phase 00.)

**PHASE-06 design driver — needs ADR-Supexec.** AArch64 has no m68k-style
in-process supervisor flag: user code runs at EL0, kernel at EL1, with no
cheap privilege flip. The TOS `Supexec` semantics must resolve to one of:

1. **Run the callback at EL1** (trap into the kernel and jump to the user
   pointer) — full-memory-access security hole; rejected unless sandboxed.
2. **Pseudo-supervise**: run the callback at EL0 with a curated set of MMIO
   regions (palette, Line-A shadows, hardware we already emulate) mapped
   read/write into the process. Most real `Supexec` uses fall here — the
   working assumption.
3. **Kernel-side pattern translation**: recognise common callback signatures
   (palette swap, screen flip) and perform the intended effect natively;
   foreign callbacks fault gracefully.

ADR to be filed in `docs/MEMORY.md` once PHASE-06 prototyping picks an
approach. Until then `Supexec` is deferred; it is **not** silently stubbed.

---

## Pi / BCM2711 binding

> Re-points the privilege model from the **m68k in-process supervisor flag**
> (SR S-bit; the production handler is a bare `jmp` that stays in the mode the
> XBIOS trap already entered) to the **AArch64 EL0/EL1 split**, where there is
> no cheap in-process privilege flip. The trap ID/ABI contract is unchanged;
> the mechanism is an **open decision** (ADR-Supexec). See
> `docs/HARDWARE-PROFILE.md` §3.7-§3.8.

### `Supexec` is deferred — not in the dispatcher today

`Supexec` (XBIOS 0x26) has **no entry** in the NewTOS XBIOS dispatcher, so a
call falls through to its catch-all, which returns **the function number
itself**, zero-extended — `other => other as i64` in `xbios::dispatch`. A
caller therefore gets `38` (`0x26`), and a caller testing `result < 0` sees
"not an error".

> **This section used to claim `EINVFN`** and contradicted the head of this
> file, which had it right (DESK27, 2026-09-23). The XBIOS dispatcher's
> catch-all has never been `EINVFN`: returning the function number is the
> documented Atari-TOS behaviour for both the `xbios_unimpl` padded slot and
> the trampoline's range check (`docs/api/xbios/_dispatcher.md` §3, D-008), and
> the BIOS trampoline does the same. `Supexec` is deferred **by absence** from
> the table, pending the ADR — not stubbed, and not an error return either.

**What this means for a caller that needs the callback to run:** it does not
run, and the return value does not say so. That is the honest state of a
deferred slot. Where the desktop needed one of `Supexec`'s EmuTOS uses — the
`_IDT` cookie behind `$5A0` — it reads the cookie through
`Ssystem(S_GETCOOKIE)` instead (DESK13), which needs no supervisor mode
because the low-memory area is already in the process's address space.

### Why the m68k mechanism does not port

| Atari reference | Pi-4 / AArch64 reality |
|-----------------|------------------------|
| XBIOS trap already in supervisor mode; `jmp (a0)` tail-calls the callback in the same mode | XBIOS `svc #14` runs at **EL1**; the caller runs at **EL0**. Jumping to a user pointer at EL1 = full-memory-access hole |
| Callback's `rts` returns through the dispatcher's `rte`, restoring the caller's mode | AArch64 has no in-process privilege flip; returning to EL0 is an `eret`, not a mode-restore |
| Callback reads supervisor-only locations / the SR | most real uses read MMIO/vector shadows NewTOS could map read-only into EL0 (pseudo-supervise) |

### Decision recommended (ADR-Supexec)

The three candidate mechanisms are already sketched in the NewTOS notes above
(EL1 callback / pseudo-supervise at EL0 / kernel-side pattern translation).
**Recommend option 2 (pseudo-supervise)** as the working assumption: run the
callback at EL0 with a curated read/write map of the MMIO/shadows NewTOS already
models (palette, Line-A shadows, cookie jar). This unblocks the two known
consumers on the Pi:

- the **cookie-jar** app-access pattern (`Supexec` + scan `$5A0`) — already
  routed around via `Ssystem` (see `_cookie-jar.md`), so it does **not** need
  `Supexec`;
- generic hardware-poke callbacks (palette flips, benchmarks) — the pseudo-
  supervise map covers the common cases; foreign callbacks fault gracefully.

File the ADR before wiring `0x26`; until then it stays out of the table
(deferred), which means the dispatcher answers with the function number.

### Gaps vs. the contract (Step-2 candidates)

- **Supexec callback-ABI unresolved (ADR-Supexec).** The whole call is deferred;
  no callback runs today. This blocks any m68k `Supexec` idiom (including the
  classic cookie-jar scan, though NewTOS provides `Ssystem` instead).
- **Function number by catch-all** — once the ADR lands, `0x26` gets a real
  slot; the catch-all's function-number answer is what an absent slot yields,
  not the intended contract answer.
