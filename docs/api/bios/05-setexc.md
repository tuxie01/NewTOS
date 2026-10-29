# `Setexc` (BIOS 0x05)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bios/bios.c:1070` (`setexc`).
> [BIOS12] 2026-10-28: Pi binding, edge case 5 and the hook notes corrected
> from EmuTOS and the audit measurements (`measurements/measurement-2026-10-28-bios-core-audit/RUNINFO.TXT`).
> [BIOS07] 2026-10-29: Pi binding rewritten for page 0 (D1-D3, P3 built).

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
   NewTOS choice, see "Decisions" (D1; built by BIOS07: the cell sign-extended).

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

[BIOS07] Implemented by TASK/BIOS07 (2026-10-29): see "Pi / BCM2711
binding" below. PHASE-06 (history) planned a software "m68k vector shadow"
at virtual `0..0x400`; it existed until BIOS07 and is replaced by page 0.

---

## Pi / BCM2711 binding

> Re-points the hardware reference from the **68000 low-memory vector table**
> (`4*num` in physical RAM) and the MFP/GLUE interrupt sources to the **AArch64
> exception model** (`VBAR_EL1` + GIC-400) plus **page 0 of every process**.
> The `Setexc` contract (get/set an entry, `-1` = query, always return the
> previous value) is unchanged; only the backing store is re-pointed here. See
> `docs/HARDWARE-PROFILE.md` §3.8, §3.9.

### There is no 68k vector page; `Setexc` answers from page 0

AArch64 has **no flat `4*num` vector page**. Exceptions go through `VBAR_EL1`
(`kernel/src/arch/aarch64/exceptions.rs`) and interrupts come from the
GIC-400, not from the MFP/GLUE. Every process has a private page 0 with the
classic low-memory system area (`docs/COMPATIBILITY.md`, page 0 per process).
[BIOS07] Since TASK/BIOS07 (decision D3) page 0 carries **every**
`num < 0x200`, in three regimes (`tos-api/src/bios/vectors.rs:114@setexc`):

| `num` (the `WORD` as `usize`) | Cell | Source |
|---|---|---|
| `0 … 0x103` (vectors, `etv_timer`/`etv_critic`/`etv_term`/`etv_xtra`) | **one cell for every process**, 32 bit, as EmuTOS has one vector table; every loaded process's page 0 carries a copy at `4*num`, written at load and again by every write of the cell (`Setexc`, the owner's `Pterm` restore). A query answers the cell | `tos-api/src/bios/vectors.rs:90@VECTORS`, `tos-api/src/bios/vectors.rs:166@store_shared`, `tos-api/src/bios/vectors.rs:181@write_page0_vectors` (called from `tos-api/src/bdos/process/loader.rs:1304@write_page0_vectors`) |
| `0x104 … 0x1FF` (the system variables `$410`–`$7FC`) | the calling process's own page-0 cell, through the page's kernel alias; with no process running a fault-recovered user access at VA `4*num` | `tos-api/src/bios/vectors.rs:114@setexc` |
| `>= 0x200`, and a negative `num` | answers `0`, touches nothing (D2) | `tos-api/src/bios/vectors.rs:114@setexc` (final `0`) |

All regimes: the answer is the 32-bit cell **sign-extended** into `x0` (D1,
`tos-api/src/bios/vectors.rs:101@long`); a write stores the **low 32 bits**
of `x1`; the query test compares the **low 32 bits** with `-1` (P3, as
`bios/bios.c:1074` compares the `LONG`), so `0x0000_0000_FFFF_FFFF` queries as
`-1` does. P3 was measured before it was changed: `tosdump/probes/sxprobe.c`
Q03/Q04, EmuTOS (tt, falcontc) `Q03 ret=80000001 hi=ffffffff`, `Q04` the
same (a query); NewTOS before BIOS07 `Q04 ret=ffffffff` (a write)
(`measurements/measurement-2026-10-29-bios07/RUNINFO.TXT`).

### Boot values of the shared cells

| `num` | NewTOS | EmuTOS | Source / status |
|---|---|---|---|
| `0` (`$000`) | the OSHEADER's first LONG: `os_entry` (0) and `os_version` | `os_entry`/`os_version` of the ROM | EmuTOS copies the OSHEADER's first two LONGs to address 0 where the hardware does not route the ROM there (`bios/bios.c:120-136`); `tos-api/src/bdos/process/loader.rs:1089@OSHEADER_LONG0` |
| `1` (`$004`) | `reseth`, the reset stub | `_main` | as above; `tos-api/src/bios/reset.rs:108@RESET_STUB_VA` |
| `2 … 0xFF` | the shared `ret` stub, EmuTOS' `just_rts` (`JUST_RTS`) | the ROM's exception/interrupt handlers | D3: vectors NewTOS does not serve point to the `just_rts` stub (`tos-api/src/bios/vectors.rs:67@JUST_RTS`) |
| `0x100` `etv_timer` | `JUST_RTS` | `tikfrk` (`bdos/time.c:167`) | not served, ACCEPTED deviation BIOS-CORE-002 (ADR-031) |
| `0x101` `etv_critic` | `JUST_RTS` | `default_etv_critic` (`bios/bios.c:361`) | not served (ADR-031) |
| `0x102` `etv_term` | `JUST_RTS` | `just_rts` (`bios/bios.c:362`) | served (below) |
| `0x103` `etv_xtra` | `0` | `0` | `bios-core-r1` case 58 |

Every default address is non-null, even, `>= 0x400` and below 2 GiB (the
MEM11 invariant; the class `PTR_RAM` of `bios-core-r1` 46–57), host test
`tos-api/tests/bios07_setexc.rs`. The value of a vector NewTOS does not
serve cannot equal EmuTOS's ROM address; the tables compare it as `PTR_RAM`.

**Page 0 written directly.** A program in the super state may write the copy
of a shared cell in its own page 0. NewTOS does not act on that copy
(ADR-031 rule 4): `Setexc(num, -1)` keeps answering the shared cell, `Pterm`
calls the `etv_term` of the shared cell, and the page-0 check warns about the
copy (`tos-api/src/bdos/process/hooks.rs:311@check_page0`, compared with the
shared cell). EmuTOS, with one memory cell, would answer and call the written
value — a documented deviation for that case only; the next write of the
cell through `Setexc` overwrites every copy.

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
| `etv_term` (`0x102`) | **served** | Pterm reads the shared cell and delivers the handler as an EL0 upcall unless it is `0` or the boot value `JUST_RTS`, whose call would do nothing (`kernel/src/arch/aarch64/exceptions.rs:1391@etv`, `:1396@JUST_RTS`) |
| `etv_timer` (`0x100`) | stored, **not served**, warning | BIOS-CORE-002 is an ACCEPTED deviation by ADR-031; measured T01: calls 100 (EmuTOS) / 0 (NewTOS) |
| `etv_critic` (`0x101`) | stored, **not served**, warning | `Rwabs` has no critical-error retry path |
| CPU/trap vectors `0..0xFF` | stored, never dispatched (reserved for the m68k userland) | the EL1 handlers do not consult them |
| page-0 hook cells written directly (`hdv_*`, `swv_vec`, `exec_os`, `dump_vec`, `resvector`; the copies of the vector cells) | not served; page 0 is only read to warn | ADR-031 |

The accessor `vectors::vector(num)` answers a shared cell as `Setexc` does;
it is read for `etv_term` (`tos-api/src/bios/vectors.rs:191@vector`).

[BIOS15] Built (`tos-api/src/bios/vectors.rs:237@note_write`, the shared
mechanism `tos-api/src/bdos/process/hooks.rs:518@note`): a `Setexc` write of
`etv_timer`, `etv_critic` or a vector `< 0x100` other than its boot value
logs one `[hook] warning` per hook and process; `etv_term` (served),
`etv_xtra` and the system variables do not warn. A write into a shared cell
(`<= 0x103`) makes the calling process its owner: its `Pterm` puts the
previous value back while the cell still holds its hook, otherwise the cell
stays and the log says so; after `Ptermres` it stays (ADR-031 rule 3). A
write of a system variable (`0x104 <= num < 0x200`, [BIOS07]: before it
`0x120 <= num`) runs the page-0 hook check (`bell_hook` `0x16B`, `kcl_hook`
`0x16C`).

### Decisions (user 2026-10-28), implemented by BIOS07

D1 = **1**, keep sign extension (the rule of `tos-api/src/trap.rs:6@TrapArgs`);
the `Setexc(0x100…0x102, -1)` placeholders (cases 55–57) are gone, the cells
hold real handler addresses. The query test `v == -1` checks the **low 32
bits** like EmuTOS (`references/emutos/bios/bios.c:1074`) — P3, probe first:
done (above). D2 = **1** (return 0, documented deviation). D3 = **1** (page 0
backs every `num < 0x200`); vectors NewTOS does not serve default to the
`just_rts` stub. All four are built
(`measurements/measurement-2026-10-29-bios07/RUNINFO.TXT`). The options
stay below as rationale.

**D1. Width of the `Setexc` return value.** EmuTOS returns a 32-bit LONG in
D0 (`bios/bios.c:1070-1080`). NewTOS returns `x0` (64 bit), the 32-bit cell
sign-extended. For a cell holding a value `>= 0x80000000` the caller sees
`0xFFFF_FFFF_8xxx_xxxx` (measured: `sxprobe` Q02, both sides). The default
cells are < 2 GiB, so nothing observable differs for them.
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

**D2. Cells at `num >= 0x200`.** EmuTOS reads/writes RAM at `4*num`
(unchecked). NewTOS returns 0 and does nothing.
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
   (Built: the vector cells stay one cell for every process — ADR-031 rule 3
   treats them as shared — and page 0 carries their copy.)
2. Keep the shadow for `0..0xFF` and `0x100-0x102`, route `0x103..0x1FF` to
   page 0 (shrink `NVEC` to `0x103`). Smaller change; the shadow stays
   64-bit for the CPU vectors, so D1 applies to them.
3. Keep `NVEC = 0x120` (the state before BIOS07). Contradicts case 59.

### Gaps vs. the contract

- BIOS-CORE-002: `etv_timer` not dispatched (ACCEPTED, ADR-031).
- The vector cells are inert: they do not redirect the AArch64 handlers.
- A shared cell's page-0 copy written directly is not what `Setexc` answers
  (above, "Page 0 written directly").
- `num >= 0x200` answers 0 (D2).
