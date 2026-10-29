# Super state: `Super` (GEMDOS 0x20) and `Supexec` (XBIOS 0x26) on NewTOS

> **Status: APPROVED (user, 2026-10-27; TASK/MEM12 "Anwenderentscheidungen
> 2026-10-27, zweite Runde").** S1–S12 approved; O1 and O2 resolved (§4).
> Source: EmuTOS @ `1a7484ab`. The EmuTOS side of
> both calls is already extracted in
> [`gemdos/process/20-super.md`](gemdos/process/20-super.md) and
> [`xbios/26-supexec.md`](xbios/26-supexec.md); this file adds the NewTOS
> mapping that the user decision of 2026-10-26/27 asks for
> ([TASK/MEM12](../../TASK/MEM12.md), "Anwenderentscheidungen 2026-10-27").

## 0. The decision this implements (verbatim, 2026-10-27)

> Map the Atari protection model onto page tables instead:
> - page 0: EL0 read-only in user state, read-write while the process is in
>   super state (inside Super()/Supexec), never executable
> - write in user state → bus error, as on Atari
> - cold vs warm start: the Reset stub reads memvalid, as on Atari
> - the kernel treats page 0 as output only; it reads only documented input
>   sysvars (conterm, memvalid …), validated

## 1. Before MEM12 (read at `a90bcfb0`)

- The kernel has no super state. GEMDOS 0x20 answers `EINVFN`, XBIOS 0x26
  answers `38` (TASK/X01; [`xbios/26-supexec.md`](xbios/26-supexec.md)
  "Supexec is deferred").
- The SDK binding replaces both by EL0 code: `Super` is a no-op that hands
  back `SP_EL0` (`sdk/libc/overlay/include/mint/osbind.h` at `a90bcfb0`),
  `Supexec(f)` is a plain call `f()`
  (same file), `SuperToUser` is `Super`. In the built programs the bracket leaves no
  trace: TeraDesk's `force_mediach` has no instruction for `Super(0L)` and
  stores straight to `0x472..0x485`; HypView's `ChangeSysKeyconfig` is
  inlined into `KeyboardOnOff` (`strb` to `0x484`); DOOM's `kbd_install` is
  inlined (TASK/MEM12, "Super/Supexec in den Programmen").
- Page 0 was `UserRwExec` in every process (`tos-api/src/bdos/process/mmu.rs`
  `init_slot_tables` at `a90bcfb0`).

Since MEM12 the SDK macros are traps again
(`sdk/libc/overlay/include/mint/osbind.h:470@Super`,
`:696@Supexec`), and the kernel implements S1–S13
(`tos-api/src/bdos/process/superstate.rs:66@super_call`).

## 2. EmuTOS reference (read, pinned)

| What | Where |
|---|---|
| Trap #1 front end peeks function `0x20` before the BDOS table, in both modes | `bdos/rwa.S:159-184` |
| `x20_usr`: user → supervisor; new SSP = USP (`stack == 0`) or `stack`; returns the old SSP | `bdos/rwa.S:280-301` |
| `x20_sup`: supervisor → user; rebuilds the USP; leaves `d0` untouched | `bdos/rwa.S:305-336` |
| `x20_inq` (`stack == 1`, either mode): `0` user, `-1` supervisor, no change | `bdos/rwa.S:340-346` |
| A GEMDOS call from supervisor mode runs on the caller's stack | `bdos/rwa.S:193` |
| `Supexec`: `movea.l 4(a7),a0; jmp (a0)` — the callback runs in the trap's supervisor mode, its `rts` returns through the dispatcher's `rte`, which restores the caller's SR | `bios/vectors.S:1376-1378` |
| "The executed code cannot perform BIOS or GEMDOS calls" | `bios/xbios.c:767-773` |
| A new process starts in user mode | `bdos/proc.c:549` |
| `Pterm`: `termuser` resumes the parent from the frame saved at its own `Pexec` trap, SR included — the child's mode does not survive it | `bdos/proc.c:601-617`, `bdos/rwa.S:234-267` |
| `kbdvec` is entered from the IKBD interrupt, i.e. in supervisor mode | `bios/aciavecs.S:405-414` |

Not from EmuTOS: that a user-mode access to `$0..$7FF` raises a bus error is
Atari **hardware** (memory controller), not EmuTOS code. It is the premise of
the user decision and can be measured in Hatari; it is not cited here as
source. On the Atari a user-mode **read** of that area faults too; NewTOS
keeps reads open in user state (TASK/MEM12 question 1: native readers exist),
a deliberate superset.

## 3. NewTOS contract (approved 2026-10-27)

**S1 — the state.** Every process has one bit, *super state*, off at load
(`Pexec` 0/3/4/6, accessory load; EmuTOS `bdos/proc.c:549`). It belongs to the
process, not to the machine: other processes, accessories included, keep
their own.

**S2 — what it grants: page-0 write, nothing else.** In super state the
process's page 0 `[0, 0x1000)` is EL0 read-write; in user state EL0
read-only. Page 0 is never executable in either state. Super state grants
**nothing else**: the code still runs at EL0, with the same page tables
apart from that one entry; no EL1 execution, no kernel memory, no MMIO, no
other process's memory; memory-protection rights (`PROT_P/G/S/PR/I`) are
unchanged; the stub page (EL0 execute-only) and the private read-write page
(MEM12 E2) are the same in both states.

**S3 — `Super(1L)`**, either state: no change; returns `0` in user state,
`-1` in super state (`x20_inq`).

**S4 — `Super(stack)`, `stack != 1`, in user state:** enters super state.
Returns a token: the caller's `SP_EL0` at the trap (never 0 or 1 — the stack
is 16-byte aligned and non-null). **Documented deviation (O2 = a):** the
stack does not change, also for an explicit `stack`, which is ignored;
EmuTOS makes `stack` (or the USP) the new SSP and the program continues on
it. NewTOS has one stack per EL0 thread, and switching `SP_EL0` under
compiled AArch64 code (SP-relative locals) would break the caller.

**S5 — `Super(stack)`, `stack != 1`, in super state:** leaves super state;
`stack` is not used as a stack (S4). Returns the caller's `SP_EL0` (EmuTOS
leaves `d0` undefined; NewTOS defines it). `SuperToUser(x)` is this case.

**S6 — `Supexec(codeptr)`:** runs `codeptr()` at EL0 in the calling process
with super state **on**; when the callback returns, the state in force at
the `Supexec` call is restored and `Supexec` returns the callback's `x0`
(64 bits, as the callback left it). The callback gets no arguments (EmuTOS
`jmp (a0)`). Different from EmuTOS, the callback **may** make system calls
(the OSHEADER reset stub, which TeraDesk's "Reboot" reaches through
`Supexec(rv)`, is one); nested `Supexec` is allowed. A `Super` toggle inside
the callback holds until the callback returns, then the entry state is
restored (EmuTOS: a callback that leaves supervisor mode crashes in the
dispatcher's `rte`; NewTOS defines the outcome). A fault in the callback ends
the program like any EL0 fault.

**S7 — end of the process:** `Pterm`, `Pterm0`, `Ptermres` and a crash end
the state with the process. The parent continues in its own state, which the
child cannot change (EmuTOS `termuser`). A `Pexec` child starts in user state
even when its parent is in super state (S1).

**S8 — system calls in super state** behave as in user state; a call that
writes a caller buffer checks it with the caller's current rights
(`tos-api/src/uaccess.rs:453@user_kptr`), so a buffer in page 0 is writable
exactly in super state.

**S9 — a page-0 write in user state** is a permission fault at EL0: crash
report with "Bus Error", key, `Pterm(-1)` — EmuTOS's bus-error path
(`any_vec` → `dopanic` → `kill_program`), the kernel survives
(`kernel/src/arch/aarch64/exceptions.rs:1454@kill_crashed_program`). The
report names the area "system variables (page 0)".

**S10 — the kernel reads page 0 only as input it documents, validated.**
Everything else there is output (written by the loader or the kernel, never
read back): `conterm` (`0x484`, bits 0-3 only, the rest ignored), and for the
reset entry `memvalid`/`memval2`/`memval3` (`0x420`/`0x43A`/`0x51A`, exact
32-bit magic compare). The `kbdvec`/`ikbdsys` slots and the `G_USERDEF`
result move off page 0 (MEM12 E2). `Setexc(n < 0x200, v)` keeps reading and
writing the cell through the kernel, in either state (MEM12 E5: the TOS API
path, EmuTOS allows it).

**S11 — upcalls.** The `kbdvec` hook upcall runs in **super state**, as
EmuTOS calls `kbdvec` from the IKBD interrupt in supervisor mode
(`bios/aciavecs.S:405-414`); the state in force before the upcall is
restored at its `Psigreturn` (O1 = yes). Other upcalls (`G_USERDEF`, …) run
in the process's current state and leave it unchanged.

**S13 — whose `Kbdvbase` table gets a key** (D9, `xbios/22-kbdvbase.md`
Teil D): the innermost started process (`process::emutos_run()`), whichever
process has the CPU. An accessory's table is never consulted (deviation:
on EmuTOS an accessory's hook is global). When an accessory installs a
`kbdvec` hook — its `kbdvec` slot no longer holds the default — the kernel
logs a warning once per accessory, so the deviation never bites silently.

**S12 — ABI.** `Super`: GEMDOS `x8 = 0x20`, `x0` = `stack` (64-bit pointer or
`0`/`1`), result in `x0`. `Supexec`: XBIOS `x8 = 0x26`, `x0` = `codeptr`,
result in `x0`. The SDK macros become traps again (`Super` →
`trap_1_wl(0x20, …)`, `Supexec` → `trap_14_wl(0x26, …)`, `SuperToUser` →
`Super`); programs built with the old macros keep their reads, and their
page-0 writes fault (S9) until rebuilt. APITEST's `SX03` ("inside the
bracket still says 0") changes to `-1`, which is the EmuTOS value.

## 4. Resolved points (user, 2026-10-27)

- **O1 — kbdvec hook in super state: yes** (S11). `G_USERDEF` keeps the
  current state (not checked how the EmuTOS AES calls `ub_code`).
- **O2 — `Super(stack)` with an explicit `stack` from user state: (a)**,
  `stack` is ignored, a documented deviation (S4). No ported program passes
  anything but `0L`/`1L`/a token (TASK/X01 inventory).
- **D9 — yes**, with the accessory-hook warning (S13).

## 5. Measurement (user, 2026-10-27)

S3–S6 get oracle cases (EmuTOS in Hatari vs NewTOS): `Super(1L)` in user and
super state, the `Super(0L)`/`SuperToUser` returns as an address class (not an
exact value), and `Supexec` passing the callback's return value through.
