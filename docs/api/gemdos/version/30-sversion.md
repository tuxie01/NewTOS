# `Sversion` (GEMDOS 0x30)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bdos/bdosmain.c:298` (`xgetver`) and
> `include/config.h:1575-1591` (`GEMDOS_VERSION`).

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../../COMPATIBILITY.md).
> Contract: trap ID/ABI, the returned WORD value and its **byte order**
> (minor in the high byte, major in the low byte), the zero-extend-to-LONG
> shape. Reference: the `GEMDOS_VERSION` build constant.

## EmuTOS source

**Primary location:** `bdos/bdosmain.c:298` — `static long xgetver(void)`. The
body is `return (long)GEMDOS_VERSION;` — a pure return of the build-time
version constant, zero-extended to LONG. No inputs, no state.

**Installed handler / vector:** (none) — direct `funcs[]` entry `0x30` =
`xgetver`, `stdio_typ = 0`, `wparms = 0` (`bdos/bdosmain.c:240`).

**Helpers called:** none.

**Headers / type definitions used:**
- `include/config.h:1589-1591` — `GEMDOS_VERSION` (`#ifndef` default
  `0x2000`).

**System variables / globals read:** none — the value is a compile-time
constant, **not** read from the OS header. (`Sversion` reports the *GEMDOS*
version; the OS-header `os_version` at `0x4F2` is a separate TOS version and is
read by other code, not by `xgetver`.)

**System variables / globals written:** none.

**Build configuration affecting this function:** `GEMDOS_VERSION` — set
per-build in `config.h` (default `0x2000`). No other `CONF_*` flag alters the
returned value.

> **NewTOS choice:** NewTOS targets the Falcon030 / TOS 4.04 profile
> ([[emutos-target-profile-falcon-tos4]]), so it reports `0x3000` — the value
> `include/config.h:1585` documents as "used by TOS v4.0x" — rather than EmuTOS' own
> conservative `0x2000` default. Implemented in `tos-api/src/gemdos/version.rs`.
> The separate OS-header TOS version (`os_version` at `0x4F2`, see below) will
> be `0x0404` once the BIOS OS header is built; the AES version is `0x0340`
> (Falcon AES 3.40).

---

## Source extraction summary

`Sversion` returns the GEMDOS version as a 16-bit value, zero-extended into the
LONG result. EmuTOS returns the compile-time constant `GEMDOS_VERSION`, which
defaults to `0x2000` (`include/config.h:1590`).

The byte order is the historical Atari convention, spelled out in the config.h
comment (`include/config.h:1577-1578`): **"the minor version number in the
high-order byte, and the major version number in the low-order byte."** So
`0x2000` decodes as major = low byte = `0x00`, minor = high byte = `0x20` (32
decimal) — i.e. GEMDOS 0.32, the version reported by TOS v2.06 / v3.06. A
program that wants a human "minor.major" string reads `(D0 >> 8) & 0xFF` (=
minor) and `D0 & 0xFF` (= major).

The constant "does not have a well-defined purpose, although it could be
checked by programs to determine presence or absence of certain GEMDOS
functions" (`include/config.h:1586-1587`) — it is not a reliable feature bitmap;
the version-to-feature mapping is unwritten Atari lore.

---

## Trap interface

- **Trap number:** 1 (GEMDOS)
- **Function ID:** 0x30
- **Stack frame on entry (m68k):**
  ```
   0: function_id (word) = 0x0030
  ```
- **Total parameter bytes:** 2 (just the ID) → `wparms = 0`.
- **Native (AArch64) ABI:** ID `0x30` in `x8`; no arguments; the version WORD
  returned **zero-extended** into `x0` (bits 16-63 = 0).

  | m68k stack | AArch64 reg | Name | Type / size |
  |------------|-------------|------|-------------|
  | (none)     | —           | —    | —           |

---

## Inputs

None.

---

## Outputs

### Return value

- **Register:** D0 / `x0`
- **Type:** LONG (a zero-extended version WORD)
- **Encoding:** version WORD, **minor in the high byte, major in the low byte**
  (Atari byte-swapped convention); high 16 bits of the LONG are zero.
- **Source location:** `bdos/bdosmain.c:300`.

### Version WORD values (from `include/config.h:1580-1585`)

| Value    | Major.Minor (decoded) | Atari TOS era                     |
|----------|-----------------------|-----------------------------------|
| `0x1300` | 0.19                  | TOS v1.00 & v1.02                 |
| `0x1500` | 0.21                  | TOS v1.04 & v1.06                 |
| `0x1700` | 0.23                  | TOS v1.62                         |
| `0x1900` | 0.25                  | TOS v2.01, v2.05, v3.01, v3.05    |
| `0x2000` | 0.32                  | TOS v2.06 & v3.06 (EmuTOS default) |
| `0x3000` | 0.48                  | TOS v4.0x (Falcon)                |

(The decoded major is the low byte, always `0x00` for these; the "minor" is the
high byte read as decimal. The list is exactly the set enumerated in
`config.h`; earlier drafts included a `0x0F00`/"TOS 1.00" row that is **not** in
this source — corrected, see D-037.)

### Side effects on global state

None.

### Side effects on hardware

None.

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| (none) | `bdos/bdosmain.c:298` | always `GEMDOS_VERSION` (`0x2000` default) |

`Sversion` never fails.

---

## Verification criteria

- **Nominal:** `Sversion()` returns `0x00002000` in `x0` (high 16 bits zero)
  for the default build. Decoded: major `0x2000 & 0xFF = 0`, minor
  `(0x2000 >> 8) & 0xFF = 0x20 = 32` → "GEMDOS 0.32".
- **Byte order:** the low byte of the returned WORD is the major (`0x00`), the
  high byte is the minor (`0x20`). A conforming implementation must **not**
  return `0x0020` (that would be the un-swapped form).
- **Build-configurable:** if `GEMDOS_VERSION` is overridden (e.g. `0x1900`),
  `Sversion()` returns that exact value.

---

## Edge cases discovered in source

1. **Byte-swapped by convention, not accident.** The value is stored already
   byte-swapped (minor high, major low). Programs and this spec must not
   "correct" it — the swapped word *is* the ABI value (`include/config.h:
   1577-1578`).

2. **Not the OS-header version.** `Sversion` returns the GEMDOS constant, which
   is independent of the TOS `os_version` field in the OS header (`0x4F2`). The
   two can differ; do not conflate them.

3. **Not a reliable feature probe.** `config.h` itself notes the value "does
   not have a well-defined purpose." Some programs instead read the GEMDOS
   trap-1 vector via `Setexc(33, -1)` and inspect the dispatcher — outside the
   API contract.

4. **High word guaranteed zero.** `UWORD`→`long` return zero-extends; `x0` bits
   16-63 = 0 (not sign-extended).

---

## Dependencies

### Requires (must be initialized / functional)

- `GEMDOS_VERSION` build constant. Nothing else.

### Used by

- Program startup / AES / CPX code that checks the GEMDOS version before using
  version-gated calls (Mxalloc, Maddalt, Pexec mode 6, …).

---

## Related data structures

- `docs/api/gemdos/_state.md` §9 — OS header (distinct `os_version`).

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Historically: "returns
  the GEMDOS version; low byte = major, high byte = minor" — agrees with the
  EmuTOS byte order.)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

### Atari ST Internals

- Version-to-era table above matches the `config.h` enumeration.

---

## Configuration / version notes

- `GEMDOS_VERSION` defaults to `0x2000` (`include/config.h:1590`). NewTOS
  reports `0x2000` to match the EmuTOS feature baseline (Mxalloc/Maddalt/Pexec
  mode 6 all present). Reporting an older version makes programs refuse newer
  GEMDOS calls; reporting a newer one invites calls that do not exist.

---

## NewTOS implementation notes

(Empty during Phase 00.)

PHASE-02 decision: report `0x2000`. Rationale: matches the implemented feature
set. If NewTOS later advertises a distinct "NewTOS" identity, that goes in a
cookie (see `154-ssystem.md` / `docs/api/bios/_cookie-jar.md`), **not** by
mutating `Sversion` — programs key the version word to Atari feature
generations.
