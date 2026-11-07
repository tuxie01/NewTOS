# `Scrdmp` (XBIOS 0x14)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bios/xbios.c:490` (`scrdmp`).

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md).
> Contract: trap ID/ABI, `void` return, the "invoke the installed dump
> routine then reset `dumpflg`" behaviour, the replaceable `dump_vec`.
> Reference: `protect_v`, the default `just_rts`, the Alt-Help trigger.

## EmuTOS source

**Primary location:** `bios/xbios.c:490` — `static void scrdmp(void)`, slot
0x14 of `xbios_vecs[]` (`bios/xbios.c:1310`).

**Installed handler / vector:** `dump_vec` (`include/tosvars.h:92`, sysvar
`0x502`) — the screen-dump routine; default `just_rts` (a no-op), set by
`bios/chardev.c:84`.

**Helpers called:**
- `util/miscasm.S` — `protect_v()` — call `dump_vec` in a `d2`/`a2`-preserving
  supervisor wrapper.

**Headers / type definitions used:**
- `include/tosvars.h:61` — `dumpflg`; `:92` — `dump_vec`.

**System variables / globals read:**
- `dump_vec` (`0x502`) — the routine to invoke.

**System variables / globals written:**
- `dumpflg` (`0x4EE`) — reset to `-1` after the dump (`bios/xbios.c:493`).

**Build configuration affecting this function:** none (always present).

---

## Source extraction summary

`Scrdmp` performs a screen dump by calling whatever routine is installed at
`dump_vec` (via `protect_v`, which preserves `d2`/`a2`), then resets
`dumpflg = -1` to re-arm future dumps. EmuTOS ships **no** built-in dump
routine: `dump_vec` defaults to `just_rts`, so a stock `Scrdmp` does nothing
until a program installs a real dump handler. The Alt-Help key increments
`dumpflg` (`bios/ikbd.c:596`), which the VBL notices to trigger a dump; the
XBIOS call is the programmatic equivalent.

Programs replace `dump_vec` to redirect the dump (to a printer, a file, a
PostScript stream). `Scrdmp` returns nothing.

---

## Trap interface

- **Trap number:** 14 (XBIOS)
- **Function ID:** 0x14
- **Stack frame on entry (m68k):**
  ```
   0: function_id (word) = 0x0014
  ```
  No parameters (`Scrdmp()` = `xbios_v_v(20)`, `include/xbiosbind.h:38`).
- **Total parameter bytes:** 2.
- **Native (AArch64) ABI:** ID `0x14` in `x8`; no argument registers;
  **`void`** — no return value.

  | m68k stack | AArch64 reg | Name | Type / size |
  |------------|-------------|------|-------------|
  | (none)     | —           | —    | —           |

---

## Inputs

None.

---

## Outputs

### Return value

- **Register:** none (`void`).
- **Type:** `void`.
- **Source location:** `bios/xbios.c:490` (declared `void`).

### Side effects on global state

- `dumpflg = -1` (`bios/xbios.c:493`).

### Side effects on hardware

- Whatever `dump_vec` does (nothing by default; a printer/file write if a
  handler is installed).

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| (no error return) | — | — |

---

## Verification criteria

- **Invokes `dump_vec`:** installing a routine at `dump_vec` and calling
  `Scrdmp()` runs that routine exactly once.
- **Resets `dumpflg`:** after `Scrdmp()`, `dumpflg == -1`.
- **Default no-op:** with the default `dump_vec == just_rts`, `Scrdmp()` does
  nothing observable (and does not error).

---

## Edge cases discovered in source

1. **Default is a no-op.** `dump_vec` starts as `just_rts`
   (`bios/chardev.c:84`); EmuTOS has no built-in printer dump. The old spec's
   "default `dmpfunc` dumps to printer" is wrong — corrected (D-046).
2. **`dumpflg` is the re-arm flag.** `Scrdmp` sets it to `-1` (idle) after the
   dump; Alt-Help does `dumpflg++` (`bios/ikbd.c:596`) to request a dump the
   VBL then performs: the VBL calls `dump_vec` only when `dumpflg == 0`, then
   stores `-1` (`bios/vectors.S:466-472`), so one Alt-Help from the idle `-1`
   triggers exactly one dump. A program can watch/poke `dumpflg` to
   coordinate.
3. **Runs via `protect_v`.** The dump routine executes in a supervisor
   wrapper that preserves `d2`/`a2`, so a C dump handler is register-safe.

---

## Dependencies

### Requires (must be initialized / functional)

- `dump_vec` (installed routine or the default `just_rts`), `dumpflg`.

### Used by

- Screenshot / print-screen tools, the Alt-Help key path.

---

## Related data structures

- `docs/api/xbios/_state.md §5` — `dump_vec`, `dumpflg`.

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Documents "dump screen
  to printer"; EmuTOS ships no default dumper.)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

---

## Configuration / version notes

- No `CONF_WITH_*` gate; the default `dump_vec` is a no-op on every build.

---

## NewTOS implementation notes

(Empty during Phase 00.)

PHASE-07/08: NewTOS may install a default `dump_vec` that writes a PNG of the
framebuffer to a configured path; `Scrdmp` invokes it and resets `dumpflg`,
and programs can still replace `dump_vec`.

**Current NewTOS (TASK/V34, VDI-TEXT-011):**
`tos-api/src/xbios/system.rs:61@scrdmp` returns `void` (→ 0) and stores the
WORD `-1` at `$4EE` of the calling process's page 0 — the cell the program
reads, which the loader seeds with `-1` (`bios/bios.c:357`). With no process
running there is no page 0 and nothing is written. `dump_vec` is **not
called**: it is a hook NewTOS stores but never calls (ADR-031,
`docs/MEMORY.md`); a program that writes `$502` gets the ADR-031 warning at
the page-0 check, not at `Scrdmp`. So the criterion "Invokes `dump_vec`"
does not hold on NewTOS; "Resets `dumpflg`" and "Default no-op" do. Measured
against the pinned oracle by probe T18A (`tosdump/probes/vdprobe.c`):
`dumpflg` poked to 5, then `Scrdmp()` → `-1` on both sides
(`measurements/measurement-2026-11-07-v34/RUNINFO.TXT`). Neither consumer
of `dumpflg` exists on NewTOS: the VBL dump (it would call `dump_vec`,
ADR-031) and Alt-Help's `dumpflg++` (the key is queued as a normal key).

---

## Pi / BCM2711 binding

> Re-points the *hardware reference* (the Centronics printer + `dump_vec`/
> `dumpflg` path) at the NewTOS profile, which has **no printer**. The
> **contract is unchanged** — `void`, "invoke `dump_vec` then reset
> `dumpflg`", the default no-op. Peripheral: see `docs/HARDWARE-PROFILE.md`
> §3.9 (Centronics printer → absent).

- **No printer hardware.** The Pi-4 profile has no Centronics port
  (`docs/HARDWARE-PROFILE.md` §2, §3.9), so the Atari default `dump_vec ==
  just_rts` no-op is the correct, honest behaviour: a stock `Scrdmp` does
  nothing, exactly as on EmuTOS with no dump handler installed.
- **Wired (TASK/V34).** `Scrdmp` (XBIOS `0x14`) is dispatched by
  `tos-api/src/xbios/mod.rs` and re-arms `dumpflg`; see "NewTOS
  implementation notes" above. (An earlier revision said the call was absent
  from the dispatcher; before V34 it was dispatched but did not store
  `dumpflg`.)
- **Future (not required by the hardware).** A `dump_vec` that encodes the
  32 bpp VideoCore framebuffer to a PNG on the SD card is the natural
  NewTOS equivalent of "screen dump to printer" (PHASE-07/08 note); this is
  optional, since the contract's default is already a no-op.
- **1080p later.** A framebuffer dump would use the live geometry; no
  dependency on the fixed 640×480 mode.
