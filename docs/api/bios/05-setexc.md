# `Setexc` (BIOS 0x05)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bios/bios.c:1070` (`setexc`).
> [BIOS12] 2026-10-28: Pi binding, edge case 5 and the hook notes corrected
> from EmuTOS and the audit measurements (`measurements/measurement-2026-10-28-bios-core-audit/RUNINFO.TXT`).

## EmuTOS source

**Primary location:** `bios/bios.c:1070` — `LONG setexc(WORD num, LONG vector)`

**Helpers called:** none (a bare read-modify-write of `*(LONG *)(4L*num)`).

**Headers / type definitions used:**
- `tosvars.ld` — the low-memory vector layout (`etv_timer=0x400`,
  `etv_critic=0x404`, `etv_term=0x408`) that specific `num` values address.

**System variables / globals read/written:**
- The 4-byte LONG at address `4 * num`. For `num` in the standard vector
  range this is a CPU exception vector (`0..0xFF` → `0x000..0x3FC`) or a TOS
  system-vector (`0x100`→`0x400` etv_timer, `0x101`→`0x404` etv_critic,
  `0x102`→`0x408` etv_term).

**Build configuration affecting this function:** none — configuration- and
version-independent.

---

## Source extraction summary

`Setexc` gets/sets a raw 32-bit exception (or system) vector. It computes the
byte address `4 * num`, reads the current LONG there into `oldvector`, and —
only if `vector != -1` — overwrites it with the new value. It always returns
the previous value. `vector == -1` is therefore the "query without changing"
idiom.

There is **no range check** and **no protection**: `num` is a `WORD`, so the
address `4*num` can land anywhere in the low 256 KB (and negative `num` wraps
via unsigned addressing). Writing `num` outside the vector page clobbers
whatever system variable lives at `4*num`. This is intentional 1980s
minimalism; the caller is trusted.

---

## Trap interface

- **Trap number:** 13 (BIOS)
- **Function ID:** 0x05
- **Stack frame on entry (m68k):**
  ```
  +4: vector (LONG)
  +2: num    (word)
   0: function_id (word) = 0x0005
  ```
- **Total parameter bytes:** 8
- **Native (AArch64) ABI:** ID `0x05` in `x8`; `num` in `x0`, `vector` in
  `x1`; previous vector value returned in `x0`.

  | m68k stack | AArch64 reg | Name     | Type / size |
  |------------|-------------|----------|-------------|
  | +2         | `x0`        | `num`    | `WORD` (16) |
  | +4         | `x1`        | `vector` | `LONG` (32) |

---

## Inputs

| Position | Name     | Type   | Size | Source location    | Notes |
|----------|----------|--------|------|--------------------|-------|
| arg0     | `num`    | `WORD` | 2    | `bios/bios.c:1073` | vector number; address is `4*num`. Not range-checked. |
| arg1     | `vector` | `LONG` | 4    | `bios/bios.c:1076` | new handler address, or `-1L` for query-only |

---

## Outputs

### Return value

- **Register:** D0 / `x0`
- **Type:** LONG
- **Encoding:** the previous 32-bit value at `4*num`, returned always
  (whether or not a write happened).
- **Source location:** `bios/bios.c:1074,1079`.

### Side effects on global state

- Writes `*(LONG *)(4*num) = vector` when `vector != -1`
  (`bios/bios.c:1076-1078`). No other state.

### Side effects on hardware

None directly — but installing a CPU-exception vector changes which handler
the CPU dispatches to on the corresponding fault/interrupt.

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| (none — the function cannot fail) | `bios/bios.c:1070` | n/a |

An out-of-range `num` silently writes outside the vector page and returns the
prior contents of that address. There is no error signalling.

**Error paths not in Compendium:** none; Compendium likewise documents no
error.

---

## Verification criteria

- **Nominal set + return old:** `Setexc(0x102, newaddr)` returns the previous
  `etv_term` value (at `0x408`) and stores `newaddr` there.
- **Query only:** `Setexc(0x102, -1)` returns the current `etv_term` value and
  leaves it unchanged.
- **`-1` is never installed:** even if a caller passes `vector == -1`, no
  write occurs; the vector cannot be *set* to `-1` via this call.

---

## Edge cases discovered in source

1. **`vector == -1` is a magic "don't write" sentinel**, not a real address.
   The install idiom is: `old = Setexc(num, -1)` to read, then
   `Setexc(num, myhandler)` to install and chain via `old`.

2. **No range check on `num`.** `num > 0xFF` addresses TOS system-vectors
   (`0x100`→etv_timer, `0x101`→etv_critic, `0x102`→etv_term); larger values
   clobber arbitrary low memory. `num` is a signed `WORD` but is used as
   `4L*num`, so negative values compute negative addresses.

3. **Read-modify-write is a single LONG store.** On m68k the store is atomic
   against equal-or-lower-priority interrupts; there is no explicit locking.

4. **The standard TOS system-vector numbers** (via `4*num`): `0x100`=etv_timer
   (`0x400`), `0x101`=etv_critic (`0x404`), `0x102`=etv_term (`0x408`). AES
   installs its own via these. (`tosvars.ld:33-35`.)

5. **The `etv_*` hooks have real, non-zero default handlers, not NULL.**
   `bios_init()` sets `etv_timer = just_rts`, `etv_critic =
   default_etv_critic`, `etv_term = just_rts` (`bios/bios.c:360-362`).
   `default_etv_critic` (`bios/vectors.S:823-828`) loads the error code from
   `4(sp)`, sign-extends it and returns it in D0.
   **[BIOS12] correction:** `etv_timer` does **not** keep `just_rts`. GEMDOS
   `time_init()` (`bdos/bdosmain.c:343`, from `osinit_after_xmaddalt`)
   overwrites it with `tikfrk` (`bdos/time.c:167`), the static BDOS clock
   ticker (`bdos/time.c:175`). So after boot `Setexc(0x100, -1)` returns the
   address of `tikfrk`, not of `just_rts`; `etv_critic` and `etv_term` keep
   `default_etv_critic` / `just_rts`. (Read from code; that `bdosmain` runs
   after `bios_init` is inferred from the boot order, not traced. The
   measurement only shows the three values are TOS addresses, not which
   ones.) The earlier text here (and `vectors.rs` comments) said `etv_timer`
   is `just_rts`: wrong for a running system. A conforming implementation
   must not treat "unset" as `0`; the query-before-install idiom always
   yields a callable address to chain to.

6. **[BIOS12] Who calls the hooks in EmuTOS (contract-relevant callers).**
   - `etv_timer`: the Timer-C handler calls it every 20 ms with the tick
     length in ms (`timer_ms`, a WORD) on the stack, all registers saved
     (`bios/vectors.S:540-544`). Measured: a hook counts 100 calls over 400
     `_hz_200` ticks on EmuTOS TT and Falcon (Sonde T01).
   - `etv_critic`: `call_etv_critic(err_dev)` pushes the LONG `error code ||
     device`, preloads D0 = -1, calls the hook and restores D2/A2
     (`bios/vectors.S:807-821`); callers `bios/blkdev.c:521,628,808`
     (logical requests only, `bios/blkdev.c:520`).
   - `etv_term`: `xterm()` itself reads it with `Setexc(0x102, -1)` and calls
     it through `protect_v` before any teardown, no arguments, D2/A2 kept
     (`bdos/proc.c:606-607`, `util/miscasm.S:213-220`).

7. **[BIOS12] Page-0 cells are plain memory, not "hooks".** `0x103` is
   `etv_xtra` at `$40C` (`tosvars.ld:36`, "unused"); `$420` is `memvalid`
   (`tosvars.ld:38`). There are no "GEM hooks to `$047F`" in EmuTOS: `$40C-$47F`
   are ordinary system variables. `Setexc(0x108, -1)` returns the LONG
   `memvalid`, measured `0x752019F3` on EmuTOS (`bios-core-r1` case 59,
   `bios-core-r1.tsv`, row at file line 59; Sonde S08). `$5A0/$5A4/$5A8`
   (`0x168-0x16A`: `_p_cookies`, `ramtop`, `ramvalid`) are read the same way
   (cases 60-62).

8. **[BIOS12] Return width.** `setexc` returns `LONG` (32 bit) read from a
   32-bit cell (`bios/bios.c:1070-1080`). A 64-bit return of such a value is a
   NewTOS choice, see "Open decisions".

---

## Dependencies

### Requires (must be initialized / functional)

- A populated low-memory vector page (set up by `vecs_init()`,
  `bios/bios.c:118`, and the boot ROM).

### Used by

- AES/VDI install hooks; `Pterm` cleanup via `etv_term`; custom interrupt
  handlers; debuggers; terminate-stay-resident programs.

---

## Related data structures

- `docs/api/bios/_state.md` — low-memory vector / etv_* layout
- `docs/api/structures/osheader.md` — related boot vectors

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Historically: agrees on
  `-1` query semantics and always-return-old.)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

### Atari ST Internals

- Not present in `references/`. (Would be the reference for the full vector
  number → address table.)

---

## Configuration / version notes

None. Identical across all EmuTOS builds and TOS versions.

---

## NewTOS implementation notes

(Empty during Phase 00.)

[BIOS12] stale; see "Pi / BCM2711 binding" for the current state.
PHASE-06 (history): NewTOS keeps a software "m68k vector shadow" at virtual `0..0x400`
for m68k binaries; native AArch64 handlers consult the shadow for
m68k-emulated hooks (etv_term etc.). For native source ports the etv_* hooks
are reached by name, not by poking `4*num`.

---

## Pi / BCM2711 binding

> Re-points the hardware reference from the **68000 low-memory vector table**
> (`4*num` in physical RAM) and the MFP/GLUE interrupt sources to the **AArch64
> exception model** (`VBAR_EL1` + GIC-400) plus a **software vector shadow**.
> The `Setexc` contract (get/set an entry, `-1` = query, always return the
> previous value) is unchanged; only the backing store is re-pointed here. See
> `docs/HARDWARE-PROFILE.md` §3.8, §3.9.

### There is no 68k vector page; `Setexc` is a software shadow plus page 0

AArch64 has **no flat `4*num` vector page**. Exceptions go through `VBAR_EL1`
(`kernel/src/arch/aarch64/exceptions.rs`) and interrupts come from the
GIC-400, not from the MFP/GLUE. [BIOS12] corrected: the code today
(`tos-api/src/bios/vectors.rs:98@setexc`) has **three regimes**, not two:

| `num` (as `usize` of the `WORD`) | Behaviour today | Source |
|---|---|---|
| `< NVEC` (`0x120`) | software shadow `VECTORS[num]`, `AtomicI64`; `-1` queries, any other value stored; old value returned in full 64 bit | `tos-api/src/bios/vectors.rs:21@NVEC`, `tos-api/src/bios/vectors.rs:56@VECTORS`, `tos-api/src/bios/vectors.rs:98@setexc` |
| `NVEC <= num < 0x200` | peek/poke of the 4-byte cell `4*num` in the calling process's page 0 (kernel alias; without a process, a faulting-safe user access); old value returned **sign-extended** `as i32 as i64` | `tos-api/src/bios/vectors.rs:73@SYSVAR_NUM_LIMIT`, `tos-api/src/bios/vectors.rs:124@old`, `tos-api/src/bios/vectors.rs:131@read_unaligned` |
| `>= 0x200` | returns `0`, no access (EmuTOS reads/writes RAM there) | `tos-api/src/bios/vectors.rs:98@setexc` (final `0`) |

[BIOS12] The earlier row "out-of-range `num` (`>= NVEC`) returns 0" was stale
since the page-0 branch (B1) and is replaced by the table. Contract status:

- `NVEC` has no basis in EmuTOS. The justification "etv_* / GEM hooks up to
  `$047F`" is wrong (item 7 above): `$40C-$47F` are system variables, and the
  shadow makes `Setexc(0x103..0x11F, -1)` return 0 instead of the variable
  (BIOS-CORE-006, case 59: EmuTOS `0x752019F3`, NewTOS `0`, measured on both
  profiles; fix is [BIOS07](../../../TASK/BIOS07.md)). Case 58 (`$40C`) is
  0 on both.
- `etv_*` shadow values today are the opaque 64-bit tokens
  `0x4E75_0000_0000_0000` and `0x4372_6974_0000_0000`
  (`tos-api/src/bios/vectors.rs:45@ETV_DEFAULT_JUST_RTS`,
  `tos-api/src/bios/vectors.rs:54@ETV_DEFAULT_CRITIC`), not TOS addresses
  (measured, Sonde S01-S03; `bios-core-r1` 55-57 PTR_RAM DIFF). CPU vectors
  that nobody installed read 0 (cases 46-54 DIFF; `bios-r1` 53/54 NONZERO
  DIFF). EmuTOS returns real addresses for all of them (`bios/bios.c:1073-1074`).
  **Target** (EmuTOS, not yet implemented): non-zero TOS address < 2 GiB
  (MEM11 invariant) for every queried vector.
- Install/restore cycle is equal on both systems (Sonde S04/S05: new value
  arrives whole, restore returns the previous value).

### Canonical hook vector numbers (unchanged from the contract)

| `Setexc` num | m68k addr | Symbol | Meaning | EmuTOS default after boot |
|-------------:|-----------|--------|---------|---------------------------|
| `0x100` | `$0400` | `etv_timer` | 20 ms system tick hook | `tikfrk` (`bdos/time.c:167`) |
| `0x101` | `$0404` | `etv_critic` | critical-error handler | `default_etv_critic` (`bios/bios.c:361`) |
| `0x102` | `$0408` | `etv_term` | process-terminate hook | `just_rts` (`bios/bios.c:362`) |

### What NewTOS serves (ADR-031, `docs/MEMORY.md`, "ADR-031: OS hooks installed by programs")

Decision text, not code reading: hooks NewTOS does not serve are reported by
one warning per hook and process; never silent.

| Hook | NewTOS | Note |
|---|---|---|
| `etv_term` (`0x102`) | **served** | Pterm reads the shadow and delivers the handler as an EL0 upcall unless it is `0` or the non-callable boot token (`kernel/src/arch/aarch64/exceptions.rs:1393@etv`, `:1390@etv`). Corrects the earlier "Pterm does not run etv_term" |
| `etv_timer` (`0x100`) | stored, **not served**, warning | BIOS-CORE-002 is an ACCEPTED deviation by ADR-031; measured T01: calls 100 (EmuTOS) / 0 (NewTOS) |
| `etv_critic` (`0x101`) | stored, **not served**, warning | `Rwabs` has no critical-error retry path |
| CPU/trap vectors `0..0xFF` | stored in the shadow, never dispatched (reserved for the m68k userland) | the EL1 handlers do not consult them |
| page-0 hook cells written directly (`hdv_*`, `swv_vec`, `exec_os`, `dump_vec`, `resvector`) | not served; page 0 is only read to warn | ADR-031; they are not `Setexc` vectors |

The accessor `vectors::vector(num)` is no longer `#[allow(dead_code)]`: it is
read for `etv_term` (`tos-api/src/bios/vectors.rs:149@vector`).

### Open decisions (NewTOS choices; no decision taken here)

**Decisions (user 2026-10-28):** D1 = **1**, keep sign extension (the rule
of `tos-api/src/trap.rs:6@TrapArgs`); the deviations from the rule are
fixed: the `Setexc(0x100…0x102, -1)` placeholders (cases 55–57) return real
handler addresses with D3. The query test `v == -1` checks the **low 32
bits** like EmuTOS (`references/emutos/bios/bios.c:1074`) — probe first,
P3 (no caller affected today). D2 = **1** (return 0, documented
deviation). D3 = **1** (page 0 backs every `num < 0x200`); vectors NewTOS
does not serve default to the `just_rts` stub. The options stay below as
rationale.

**D1. Width of the `Setexc` return value.** EmuTOS returns a 32-bit LONG in
D0 (`bios/bios.c:1070-1080`). NewTOS returns `x0` (64 bit). Today: page-0
cells are sign-extended (`tos-api/src/bios/vectors.rs:131@read_unaligned`),
shadow entries are full 64 bit. For a page-0 cell holding a value
`>= 0x80000000` the caller sees `0xFFFF_FFFF_8xxx_xxxx` (prediction from
code, per `docs/analysis/lp64-tos-struct-survey-2026-10-28.md` section 2.2 and
M3; not measured). The standard cells (`_p_cookies` etc.) are < 2 GiB, so
nothing observable differs today.
1. Keep sign extension. Matches m68k `D0` read as a signed LONG and a
   `long` caller on LP64 that compares with `-1`/negative error codes;
   a cell value >= 2 GiB looks negative.
2. Zero-extend the 32-bit value (`as u32 as i64`). Pointers/addresses never
   look negative; `-1` stored by a cell no longer compares equal to `-1` on
   LP64 callers (an all-ones LONG would read `0xFFFF_FFFF`). A TOS error-style
   use of the result is not defined for `Setexc`, so this breaks no
   documented contract; costs a divergence from what the 68k D0 shows to a
   `long` read.
3. Return only the low 32 bit in `w0`, the upper half specified as 0 (same
   as option 2 for the register, but states the width as contract). Makes
   BIOS07's "every address < 2 GiB" the rule instead of a test.
Consequence for BIOS07 (fix ticket): once all values are TOS addresses
< 2 GiB the three options coincide on every queried vector, so D1 only matters
for page-0 cells a program wrote itself.

**D2. Cells at `num >= 0x200`.** EmuTOS reads/writes RAM at `4*num`
(unchecked). Today NewTOS returns 0 and does nothing.
1. Keep (safe; documented deviation; the audit excludes these from tables,
   `docs/audits/2026-10-28-bios-core.md` "Setexc(n >= 0x200)").
2. Read-only peek of the process's mapped memory, write refused.
3. Full unchecked access: a corruption hazard on a 64-bit kernel; not
   advisable, listed for completeness.

**D3. Range of the shadow vs page 0 (BIOS07).** EmuTOS makes no distinction
between CPU vectors, `etv_*` and variables (one memory cell per `num`).
1. Page 0 backs all `num < 0x200` (shadow only as a cache for the
   dispatched hooks). Matches `bios-core-r1` 46-62 and the cell semantics of
   `TASK/MEM12.md`; needs page 0 initialised with real handler addresses.
2. Keep the shadow for `0..0xFF` and `0x100-0x102`, route `0x103..0x1FF` to
   page 0 (shrink `NVEC` to `0x103`). Smaller change; the shadow stays
   64-bit for the CPU vectors, so D1 applies to them.
3. Keep `NVEC = 0x120` (today). Contradicts case 59; listed as the status quo.

### Gaps vs. the contract (open, fix tickets named)

- BIOS07: NULL / token / shadow return values (above).
- BIOS-CORE-002: `etv_timer` not dispatched (ACCEPTED, ADR-031).
- CPU-exception shadow entries are inert: they do not redirect the AArch64
  handlers.
