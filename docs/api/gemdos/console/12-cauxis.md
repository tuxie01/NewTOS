# `Cauxis` (GEMDOS 0x12)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bdos/console.c:186` (`xauxistat`) and `:143` (`constat`).

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../../COMPATIBILITY.md).
> Contract: trap ID/ABI, the truth-valued status return (`-1` available, `0`
> not). Reference: the (unused) AUX typeahead buffer.

## EmuTOS source

**Primary location:** `bdos/console.c:186` — `long xauxistat(void)`

**Installed handler / vector:** (none) — direct `funcs[]` entry `0x12` =
`xauxistat`, `stdio_typ = 0x82` (stdaux), `wparms = 0` (`bdos/bdosmain.c:191`).
When stdaux is redirected to a file (`run->p_uft[2] > 0`) the dispatcher returns
`eof(h) ? 0L : -1L` — see Edge case 2.

**Helpers called:**
- `bdos/console.c:143` — `constat(h)` — typeahead check, else `Bconstat(h)`.
- BIOS `Bconstat(1)` (0x01).

**Headers / type definitions used:**
- `bdos/console.h:24,36` — `H_Aux = -2`, `HXFORM(h) = 3+h` (→ BIOS handle 1).

**System variables / globals read:**
- `run->p_uft[2]` (`bdos/console.c:188`) — stdaux device.
- `aux_typeahead` (via `constat`, through `buffer[1]`).

**System variables / globals written:** none.

**Build configuration affecting this function:** none directly (transitive
serial back-end selection: `CONF_WITH_MFP` / `CONF_WITH_SCC`).

---

## Source extraction summary

`Cauxis` reports whether a character is available on the auxiliary (serial)
device. `xauxistat` resolves stdaux from `run->p_uft[2]` (default `H_Aux = -2` →
BIOS handle 1) and calls the same `constat(h)` used by `Cconis`.

For the default configuration `h == 1`, which `constat` accepts (its guard
rejects only handles outside `1..2`). `constat` first checks the AUX typeahead
buffer — but **no GEMDOS function ever populates `aux_typeahead`** (D-003), so
that branch is dead and `constat` falls through to `Bconstat(1)`, the serial
input-ready flag. The result is truth-valued: non-zero (canonically `-1`) means
a byte is waiting, `0` means none. A redirected stdaux is answered by the
dispatcher's `eof`-based logic.

---

## Trap interface

- **Trap number:** 1 (GEMDOS)
- **Function ID:** 0x12
- **Stack frame on entry (m68k):**
  ```
   0: function_id (word) = 0x0012
  ```
  No parameters.
- **Total parameter bytes:** 2 → `wparms = 0`.
- **Native (AArch64) ABI:** ID `0x12` in `x8`; no argument registers; LONG
  result sign-extended into `x0`.

  | m68k stack | AArch64 reg | Name | Type / size |
  |------------|-------------|------|-------------|
  | (none)     | —           | —    | —           |

---

## Inputs

None. The device is `run->p_uft[2]`, not a parameter.

---

## Outputs

### Return value

- **Register:** D0 / `x0`
- **Type:** LONG
- **Encoding:** `-1L` (or the non-zero `Bconstat(1)` value) if a byte is
  available on AUX, else `0L`. Returns `0L` if `p_uft[2]` resolves to a handle
  outside `1..2`.
- **Source location:** `bdos/console.c:188` → `bdos/console.c:152`.

### Side effects on global state

- None (status probe; does not consume input).

### Side effects on hardware

- Reads the serial input-ready flag (via BIOS `Bconstat`).

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| stdaux resolves to a handle outside `1..2` | `bdos/console.c:147-148` | `0L` |

**Error paths not in Compendium:** the `0L` for an unsupported handle is not an
error code.

---

## Verification criteria

- **Available:** with a byte waiting on AUX, `Cauxis()` returns `-1L`.
- **Not available:** with AUX idle, `Cauxis()` returns `0L`.
- **Non-consuming:** `Cauxis` does not remove the byte; a following `Cauxin`
  still returns it.
- **Redirected stdaux:** with `run->p_uft[2]` forced to a file, `Cauxis()`
  returns `0L` at EOF and `-1L` otherwise (`bdos/bdosmain.c:555-558`).

---

## Edge cases discovered in source

1. **AUX typeahead is dead.** `constat` checks `aux_typeahead`, but nothing ever
   fills it (D-003), so `Cauxis` effectively always reduces to `Bconstat(1)`.

2. **Handle-remap subtlety.** If `p_uft[2]` were set (via `Fforce`) to
   `H_Console` (→ handle 2), `constat` would report **CON** typeahead/status,
   not AUX — a consequence of the standard-handle abstraction, not a bug.

3. **Redirected stdaux uses `eof`.** `case 18` returns `0L` at EOF, `-1L` before
   (`bdos/bdosmain.c:555-558`).

---

## Dependencies

### Requires (must be initialized / functional)

- BIOS serial status `Bconstat(1)`.
- `stdhdl_init()` (sets `p_uft[2] = H_Aux`).

### Used by

- Programs polling RS-232 before reading.

---

## Related data structures

- `docs/api/bios/01-bconstat.md` — the underlying status BIOS call.

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Documents `-1`/`0` status
  — consistent with EmuTOS.)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

---

## Configuration / version notes

- AUX BIOS device selection: `CONF_WITH_MFP`, `CONF_WITH_SCC`.

---

## NewTOS implementation notes

(Empty during Phase 00.)

FS11 / IO01 prep (2026-10-13, user decision 1): see `docs/api/gemdos/file/3f-fread.md` "NewTOS implementation notes" for the device table.
`Cauxis()` = `constat(1)` → `Bconstat(1)` of the AUX entry = **0** (no byte). **Unchanged by the 2026-10-13 option-A revision** (AUX = "port present, no cable", see `docs/api/gemdos/file/3f-fread.md` "NewTOS implementation notes" item 1a): EmuTOS `bconstat1` also returns 0 on an empty iorec (`bios/serport.c:255-258`, `:191-200`). The BDOS typeahead check in `constat` has nothing to find (no cable → no input, so `conbrk(1)` never fills the AUX typeahead; NewTOS has the typeahead since TASK/CON01, 2026-10-18). See the D-003 note in `03-cauxin.md`
(spec said the AUX typeahead is never populated; `conbrk(1)` can populate it in
EmuTOS).

**[XBIOS15] 2026-10-29 (qemu-virt), [BIOS09] 2026-10-30 (real Pi 4):** the AUX
entry is the kernel's serial driver over device 0's input ring on both boards;
`Bconstat(1)`, and so `Cauxis()`, is `-1` while a received byte waits, else `0`
(`docs/api/bios/01-bconstat.md`, XBIOS15 and BIOS09 notes). The **0** above is
the empty-ring answer and the device-table default the host tests see.
